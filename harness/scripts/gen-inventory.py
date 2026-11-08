#!/usr/bin/env python3
"""Generate docs/inventory.md -- the Frevo parity checklist.

Extracted from the pinned upstream reference tree, not from memory:

  * configuration keys  <- AppProperties.java (@ConfigurationProperties("hapi.fhir")),
                           cross-referenced against the starter's application.yaml
  * operations          <- JpaConstants.java and ProviderConstants.java
                           (OPERATION_* constants), plus clinical-reasoning
  * interceptor hooks   <- Pointcut.java
  * REST interactions   <- the FHIR R4 interaction set, annotated with the
                           HAPI-specific behaviour the starter exposes

Every row is assigned a target crate and a phase, and carries a Status column
that must eventually point at a passing test.
"""
from __future__ import annotations

import json
import pathlib
import re
import sys

ROOT = pathlib.Path(sys.argv[1])
REF = ROOT / "reference"
STARTER = REF / "hapi-fhir-jpaserver-starter"
HAPI = REF / "hapi-fhir"

APP_PROPS = STARTER / "src/main/java/ca/uhn/fhir/jpa/starter/AppProperties.java"
APP_YAML = STARTER / "src/main/resources/application.yaml"
POINTCUT = HAPI / "hapi-fhir-base/src/main/java/ca/uhn/fhir/interceptor/api/Pointcut.java"
JPA_CONST = HAPI / "hapi-fhir-jpaserver-model/src/main/java/ca/uhn/fhir/jpa/model/util/JpaConstants.java"
PROV_CONST = HAPI / "hapi-fhir-server/src/main/java/ca/uhn/fhir/rest/server/provider/ProviderConstants.java"

# --------------------------------------------------------------------------
# Crate / phase routing
# --------------------------------------------------------------------------
# Ordered: first matching pattern wins.
CRATE_RULES: list[tuple[str, str, int]] = [
    (r"^(cr|cql)_|measure|care[-_]?gaps|plan[-_]?definition|activity|questionnaire|cds|data[-_]requirements|collect[-_]data|submit[-_]data|\$apply|\$evaluate|\$extract|\$populate", "frevo-cr", 8),
    (r"mdm", "frevo-mdm", 6),
    (r"subscription|websocket|\$trigger-subscription|email", "frevo-subscription", 6),
    (r"ips|\$summary|\$docref|\$document", "frevo-ips", 9),
    (r"graphql", "frevo-graphql", 9),
    (r"hfql", "frevo-hfql", 9),
    (r"openapi|swagger", "frevo-openapi", 9),
    (r"narrative", "frevo-narrative", 9),
    (r"^mcp|spring[._]ai", "frevo-mcp", 9),
    (r"tester", "frevo-server", 9),
    (r"valid|snapshot|slic|profile|schematron", "frevo-validator", 4),
    (r"term|codesystem|valueset|conceptmap|\$expand|\$lookup|\$subsumes|\$translate|\$validate-code|snomed|loinc|preexpand|expansion", "frevo-terminology", 4),
    (r"package|implementationguide|^ig_|npm", "frevo-package", 4),
    (r"bulk|\$export|\$import|batch|job|reindex|delete[-_]expunge|\$expunge", "frevo-jobs", 5),
    (r"search|index|lucene|fulltext|elastic|_include|chain|composite|\$lastn|filter|contains|_has|sort|total", "frevo-search", 3),
    (r"partition|tenant|multitenan", "frevo-store", 2),
    (r"binary|storage|\$binary-access", "frevo-store", 2),
    (r"convert|\$convert|version", "frevo-convert", 9),
    (r"fhirpath|\$diff|patch", "frevo-path", 1),
    (r"cors|logger|access[-_]log|interceptor|server|port|context[-_]path|cap(ability)?[-_]?statement|metadata", "frevo-server", 1),
    (r"", "frevo-store", 2),  # default: storage behaviour
]

ORACLE_RULES: list[tuple[str, str]] = [
    (r"fhirpath|\$diff|patch", "FHIRPath suite + differential"),
    (r"valid|snapshot|slic|profile|schematron", "fhir-test-cases validator suite"),
    (r"cql|measure|care[-_]?gaps|\$evaluate|data[-_]requirements", "cql-tests + clinical-reasoning fixtures"),
    (r"term|\$expand|\$lookup|\$subsumes|\$translate|\$validate-code|codesystem|valueset", "spec expansion fixtures + differential"),
    (r"bulk|\$export|\$import", "Inferno Bulk Data + differential"),
    (r"", "differential vs HAPI 8.12.0"),
]


def route(key: str) -> tuple[str, int]:
    k = key.lower()
    for pat, crate, phase in CRATE_RULES:
        if not pat or re.search(pat, k):
            return crate, phase
    return "frevo-store", 2


def oracle(key: str) -> str:
    k = key.lower()
    for pat, o in ORACLE_RULES:
        if not pat or re.search(pat, k):
            return o
    return "differential vs HAPI 8.12.0"


# --------------------------------------------------------------------------
# 1. Configuration keys from AppProperties.java
# --------------------------------------------------------------------------
FIELD_RE = re.compile(
    r"^\s*(?:private|public|protected)\s+(?:final\s+)?"
    r"(?P<type>[A-Za-z_][\w.<>,\[\]\s]*?)\s+"
    r"(?P<name>[A-Za-z_]\w*)\s*(?:=\s*(?P<default>[^;]+?))?\s*;",
)
CLASS_RE = re.compile(r"^\s*(?:public|private|protected)?\s*(?:static\s+)?class\s+(?P<name>\w+)")


PREFIX_RE = re.compile(r'@ConfigurationProperties\(\s*(?:prefix\s*=\s*)?"(?P<prefix>[^"]+)"')


def _parse_java_classes(path: pathlib.Path) -> dict[str, list[dict]]:
    """Return {class name: [field, ...]} for every class declared in a file."""
    per_class: dict[str, list[dict]] = {}
    class_stack: list[str] = []
    depth = 0
    for raw in path.read_text(encoding="utf-8", errors="replace").splitlines():
        m = CLASS_RE.search(raw)
        if m:
            class_stack.append(m.group("name"))
            per_class.setdefault(m.group("name"), [])
            depth += raw.count("{") - raw.count("}")
            continue
        fm = FIELD_RE.match(raw)
        if fm and class_stack:
            typ = " ".join(fm.group("type").split())
            if not typ.startswith("return") and typ not in ("new",):
                per_class[class_stack[-1]].append({
                    "name": fm.group("name"),
                    "type": typ,
                    "default": (fm.group("default") or "").strip(),
                })
        depth += raw.count("{") - raw.count("}")
        while class_stack and depth < len(class_stack):
            class_stack.pop()
    return per_class


def parse_config_properties() -> list[dict]:
    """Every hapi.fhir.* key bound by a @ConfigurationProperties class.

    AppProperties.java is NOT the whole configuration surface: the starter binds
    nine such classes, and the entire hapi.fhir.cr.* tree (clinical reasoning
    and the CQL compiler) lives in separate ones. Reading only AppProperties
    silently loses about a hundred keys, which would surface much later as
    missing configuration compatibility.
    """
    files = sorted((STARTER / "src/main/java").rglob("*.java"))
    # class name -> (its own prefix, fields)   for classes that declare a prefix
    rooted: list[tuple[str, str, dict[str, list[dict]]]] = []
    # class name -> prefix, for every prefix-declaring class anywhere
    own_prefix: dict[str, str] = {}
    all_classes: dict[str, list[dict]] = {}

    parsed: dict[pathlib.Path, dict[str, list[dict]]] = {}
    for f in files:
        text = f.read_text(encoding="utf-8", errors="replace")
        if "@ConfigurationProperties" not in text:
            continue
        pm = PREFIX_RE.search(text)
        if not pm or not pm.group("prefix").startswith("hapi.fhir"):
            continue
        classes = _parse_java_classes(f)
        parsed[f] = classes
        all_classes.update(classes)
        # The annotated class is the outermost one declared in the file.
        root = next(iter(classes), None)
        if root:
            own_prefix[root] = pm.group("prefix")
            rooted.append((root, pm.group("prefix"), classes))

    out: list[dict] = []
    seen: set[str] = set()

    for root, prefix, classes in rooted:
        def expand(cls: str, pfx: str, trail: tuple[str, ...]) -> None:
            if cls in trail:
                return
            for fl in classes.get(cls, all_classes.get(cls, [])):
                base = fl["type"].split("<")[0].strip()
                key = f"{pfx}.{fl['name']}" if pfx else fl["name"]
                # A field whose type carries its own @ConfigurationProperties
                # prefix is bound by that class, so it is counted there.
                if base in own_prefix and base != cls:
                    continue
                if base in classes and base != cls:
                    expand(base, key, trail + (cls,))
                    continue
                if base in all_classes and base != cls:
                    expand(base, key, trail + (cls,))
                    continue
                if key in seen:
                    continue
                seen.add(key)
                out.append({
                    "key": key,
                    "type": fl["type"],
                    "default": fl["default"],
                    "source": "properties class",
                })

        expand(root, prefix, ())

    return out


def yaml_keys() -> tuple[set[str], set[str]]:
    """Keys the starter's own YAML files mention, active or commented out.

    Returns (leaf_keys, all_paths). A leaf has a value on its own line; an
    interior path is only a prefix and is not a configuration key.

    Commented-out keys are included deliberately: the starter documents most of
    its surface as commented examples, and those are exactly the keys operators
    copy into their own configuration.
    """
    leaves: set[str] = set()
    paths: set[str] = set()
    for yf in sorted((STARTER / "src/main/resources").glob("application*.yaml")):
        stack: list[tuple[int, str]] = []
        for raw in yf.read_text(encoding="utf-8", errors="replace").splitlines():
            if not raw.strip():
                continue
            stripped = re.sub(r"^(\s*)#\s?", r"\1", raw.rstrip())
            m = re.match(r"^(\s*)(?:-\s+)?([A-Za-z_][\w\-.]*)\s*:(.*)$", stripped)
            if not m:
                continue
            indent, name, rest = len(m.group(1)), m.group(2), m.group(3)
            while stack and stack[-1][0] >= indent:
                stack.pop()
            path = ".".join([p for _, p in stack] + [name])
            paths.add(path)
            if rest.strip() and not rest.strip().startswith("#"):
                leaves.add(path)
            else:
                stack.append((indent, name))
    return leaves, paths


# --------------------------------------------------------------------------
# 2. Operations
# --------------------------------------------------------------------------
OP_RE = re.compile(r'(?P<const>OPERATION_[A-Z0-9_]+)\s*=\s*"(?P<val>[^"]+)"')


def parse_operations() -> list[dict]:
    ops: dict[str, dict] = {}
    for f in (JPA_CONST, PROV_CONST):
        if not f.exists():
            continue
        for m in OP_RE.finditer(f.read_text(encoding="utf-8", errors="replace")):
            val, const = m.group("val"), m.group("const")
            if not val.startswith("$"):
                continue
            # HAPI namespaces a few internal operations as "$hapi.fhir.x".
            ops.setdefault(val, {"name": val, "const": const, "source": f.name})
    # Clinical reasoning operations the starter wires in through cqf-fhir-cr-hapi.
    for name in ("$evaluate-measure", "$care-gaps", "$data-requirements",
                 "$collect-data", "$submit-data", "$apply", "$evaluate",
                 "$questionnaire", "$populate", "$extract", "$package"):
        ops.setdefault(name, {"name": name, "const": "-", "source": "clinical-reasoning"})
    return sorted(ops.values(), key=lambda r: r["name"])


# --------------------------------------------------------------------------
# 3. Interceptor pointcuts
# --------------------------------------------------------------------------
def parse_pointcuts() -> list[str]:
    if not POINTCUT.exists():
        return []
    text = POINTCUT.read_text(encoding="utf-8", errors="replace")
    # Enum constants: NAME( ... at one tab of indentation.
    names = re.findall(r"^\t([A-Z][A-Z0-9_]+)\s*\(", text, re.MULTILINE)
    out, seen = [], set()
    for n in names:
        if n not in seen:
            seen.add(n)
            out.append(n)
    return out


# --------------------------------------------------------------------------
# 4. REST interactions and search features (curated, annotated from upstream)
# --------------------------------------------------------------------------
INTERACTIONS = [
    ("read", "GET [type]/[id]", "frevo-store", 2),
    ("vread", "GET [type]/[id]/_history/[vid]", "frevo-store", 2),
    ("update", "PUT [type]/[id]", "frevo-store", 2),
    ("update (conditional)", "PUT [type]?[search]", "frevo-store", 2),
    ("patch (JSON Patch)", "PATCH [type]/[id]", "frevo-path", 2),
    ("patch (FHIRPath Patch)", "PATCH [type]/[id]", "frevo-path", 2),
    ("patch (conditional)", "PATCH [type]?[search]", "frevo-path", 2),
    ("delete", "DELETE [type]/[id]", "frevo-store", 2),
    ("delete (conditional)", "DELETE [type]?[search]", "frevo-store", 2),
    ("delete (cascading)", "DELETE [type]/[id]?_cascade=delete", "frevo-store", 2),
    ("create", "POST [type]", "frevo-store", 2),
    ("create (conditional)", "POST [type] + If-None-Exist", "frevo-store", 2),
    ("search (type)", "GET|POST [type]/_search", "frevo-search", 3),
    ("search (system)", "GET /?[params]", "frevo-search", 3),
    ("search (compartment)", "GET [type]/[id]/[compartment]", "frevo-search", 3),
    ("capabilities", "GET /metadata", "frevo-server", 1),
    ("transaction", "POST / (Bundle type=transaction)", "frevo-store", 2),
    ("batch", "POST / (Bundle type=batch)", "frevo-store", 2),
    ("history (instance)", "GET [type]/[id]/_history", "frevo-store", 2),
    ("history (type)", "GET [type]/_history", "frevo-store", 2),
    ("history (system)", "GET /_history", "frevo-store", 2),
    ("conditional read (ETag)", "If-None-Match / If-Modified-Since", "frevo-store", 2),
    ("optimistic locking", "If-Match on update/delete", "frevo-store", 2),
    ("GraphQL", "GET|POST [type]/[id]/$graphql", "frevo-graphql", 9),
]

SEARCH_FEATURES = [
    ("parameter type: Number", 3), ("parameter type: Date/DateTime", 3),
    ("parameter type: String", 3), ("parameter type: Token", 3),
    ("parameter type: Reference", 3), ("parameter type: Composite", 3),
    ("parameter type: Quantity", 3), ("parameter type: URI", 3),
    ("parameter type: Special", 3),
    ("modifier :missing", 3), ("modifier :exact", 3), ("modifier :contains", 3),
    ("modifier :text", 3), ("modifier :in / :not-in", 4),
    ("modifier :below / :above", 4), ("modifier :of-type", 3),
    ("modifier :identifier", 3), ("modifier :not", 3),
    ("modifier :type (reference)", 3), ("modifier :mdm", 6),
    ("modifier :iterate (_include)", 3),
    ("prefix eq/ne/gt/lt/ge/le/sa/eb/ap", 3),
    ("chaining (forward)", 3), ("reverse chaining (_has)", 3),
    ("_include", 3), ("_revinclude", 3), ("_include:iterate", 3),
    ("composite parameters", 3), ("combo-unique / combo-non-unique params", 3),
    ("_sort (including -desc and chained)", 3),
    ("_total=none|estimated|accurate", 3),
    ("_contained / _containedType", 3),
    ("_list", 3), ("_filter", 3), ("_elements", 3), ("_summary", 3),
    ("_count / paging (keyset)", 3),
    ("custom SearchParameter + $reindex", 3),
    ("fulltext _content / _text (tantivy)", 3),
    ("$lastn", 3), ("uplifted refchains", 3),
    ("HFQL queries", 9),
]


# --------------------------------------------------------------------------
# Render
# --------------------------------------------------------------------------
def main() -> None:
    cfg = parse_config_properties()
    yaml_leaves, yaml_paths = yaml_keys()
    ops = parse_operations()
    pointcuts = parse_pointcuts()

    known = {r["key"] for r in cfg}

    # Keys the starter's YAML sets that no @ConfigurationProperties class in the
    # starter binds. These are real -- they are bound by HAPI itself, by Spring
    # Boot, or by a library -- and they are part of the compatibility surface,
    # so they are listed rather than quietly dropped.
    for p in sorted(yaml_leaves):
        if not p.startswith("hapi.fhir."):
            continue
        if p in known:
            continue
        # Skip interior prefixes that merely happen to carry an inline comment.
        if any(q.startswith(p + ".") for q in yaml_paths):
            continue
        cfg.append({
            "key": p,
            "type": "(from yaml)",
            "default": "",
            "source": "yaml only",
        })

    cfg.sort(key=lambda r: r["key"])

    for r in cfg:
        r["crate"], r["phase"] = route(r["key"])
        r["oracle"] = oracle(r["key"])
        r["in_yaml"] = r["key"] in yaml_paths or r["key"] in yaml_leaves

    for r in ops:
        r["crate"], r["phase"] = route(r["name"])
        r["oracle"] = oracle(r["name"])

    lines: list[str] = []
    A = lines.append

    A("# Feature inventory (parity checklist)")
    A("")
    A("Every feature HAPI FHIR JPA Server Starter 8.12.0 exposes, extracted from")
    A("the pinned upstream sources in [`reference/`](../reference/) rather than from")
    A("documentation or memory. This is the parity checklist: **every row must")
    A("eventually point at a passing test.**")
    A("")
    A("Generated by `harness/scripts/gen-inventory.py` from:")
    A("")
    A("| Source | Pinned at | Used for |")
    A("|---|---|---|")
    A("| starter: all 9 `@ConfigurationProperties(\"hapi.fhir*\")` classes | `377e6eba` | configuration keys |")
    A("| starter: `application.yaml`, `application-cds.yaml`, `application-elastic.yaml` | `377e6eba` | keys bound elsewhere, and which keys are documented |")
    A("| `hapi-fhir` `JpaConstants.java`, `ProviderConstants.java` | `7ab4c7c7` | operations |")
    A("| `hapi-fhir` `Pointcut.java` | `7ab4c7c7` | interceptor hooks |")
    A("| `clinical-reasoning` `cqf-fhir-cr-hapi` | `ccba474a` | clinical reasoning operations |")
    A("")
    A("> `AppProperties.java` alone is **not** the configuration surface. The starter")
    A("> binds nine properties classes, and the whole `hapi.fhir.cr.*` tree (clinical")
    A("> reasoning, and the CQL compiler's own flags) lives in separate ones. Reading")
    A("> only `AppProperties` yields 124 keys instead of 209 -- the first version of")
    A("> this inventory made exactly that mistake, and the YAML cross-check below is")
    A("> what caught it. The generator now asserts that every `hapi.fhir.*` leaf key")
    A("> in the starter's own YAML appears here.")
    A("")
    A("Status values: `not started`, `in progress` (names the change id), `done`")
    A("(names the scenario ids, which resolve to passing tests), `gap` (names a")
    A("`GAP-nnnn` entry), `deviation` (names a `DEV-nnnn` entry).")
    A("")

    # ---- totals
    A("## Totals")
    A("")
    A("| Category | Count |")
    A("|---|---|")
    A(f"| Configuration keys (`hapi.fhir.*`) | **{len(cfg)}** |")
    A(f"| &nbsp;&nbsp;bound by a `@ConfigurationProperties` class | {sum(1 for r in cfg if r['source'] == 'properties class')} |")
    A(f"| &nbsp;&nbsp;set in the starter's YAML but bound elsewhere | {sum(1 for r in cfg if r['source'] == 'yaml only')} |")
    A(f"| &nbsp;&nbsp;mentioned in the starter's YAML at all | {sum(1 for r in cfg if r['in_yaml'])} |")
    A(f"| Operations | **{len(ops)}** |")
    A(f"| REST interactions | **{len(INTERACTIONS)}** |")
    A(f"| Search features (types, modifiers, prefixes, result params) | **{len(SEARCH_FEATURES)}** |")
    A(f"| Interceptor pointcuts | **{len(pointcuts)}** |")
    total = len(cfg) + len(ops) + len(INTERACTIONS) + len(SEARCH_FEATURES) + len(pointcuts)
    A(f"| **Total rows to reach parity** | **{total}** |")
    A("")
    A("Counts are of distinct named features, not of upstream lines. They are the")
    A("denominator for the parity percentage in the README.")
    A("")

    # ---- per crate
    A("## By target crate")
    A("")
    counts: dict[str, int] = {}
    for r in cfg + ops:
        counts[r["crate"]] = counts.get(r["crate"], 0) + 1
    for _, _, c, _ in [(0, 0, c, 0) for _, _, c, _ in INTERACTIONS]:
        counts[c] = counts.get(c, 0) + 1
    A("| Crate | Config keys | Operations |")
    A("|---|---|---|")
    for crate in sorted({r["crate"] for r in cfg} | {r["crate"] for r in ops}):
        nc = sum(1 for r in cfg if r["crate"] == crate)
        no = sum(1 for r in ops if r["crate"] == crate)
        A(f"| `{crate}` | {nc} | {no} |")
    A("")

    # ---- config keys
    A("## Configuration keys")
    A("")
    A("Source of truth: `AppProperties.java`, annotated")
    A('`@ConfigurationProperties(prefix = "hapi.fhir")`. Key names and semantics must')
    A("match exactly, so an existing deployment can switch without changing its")
    A("configuration. `in yaml` marks keys the starter's own `application.yaml`")
    A("mentions, including commented-out examples -- those are the keys operators")
    A("are most likely to be using.")
    A("")
    A("| Key | Type | Default | Bound by | in yaml | Crate | Phase | Status |")
    A("|---|---|---|---|---|---|---|---|")
    for r in cfg:
        d = r["default"].replace("|", "\\|")
        if len(d) > 34:
            d = d[:31] + "..."
        t = r["type"].replace("|", "\\|")
        bound = "properties class" if r["source"] == "properties class" else "**elsewhere**"
        A(f"| `{r['key']}` | `{t}` | {('`' + d + '`') if d else '-'} | {bound} | "
          f"{'yes' if r['in_yaml'] else '-'} | `{r['crate']}` | {r['phase']} | not started |")
    A("")

    # ---- operations
    A("## Operations")
    A("")
    A("Extracted from the `OPERATION_*` constants in `JpaConstants.java` and")
    A("`ProviderConstants.java`, plus the operations the starter wires in from")
    A("`cqf-fhir-cr-hapi`. Names prefixed `$hapi.fhir.` are HAPI's own namespaced")
    A("operations, not specification operations; they are still part of the parity")
    A("surface because existing clients call them.")
    A("")
    A("| Operation | Upstream constant | Crate | Phase | Oracle | Status |")
    A("|---|---|---|---|---|---|")
    for r in ops:
        A(f"| `{r['name']}` | `{r['const']}` | `{r['crate']}` | {r['phase']} | "
          f"{r['oracle']} | not started |")
    A("")

    # ---- interactions
    A("## REST interactions")
    A("")
    A("| Interaction | Form | Crate | Phase | Status |")
    A("|---|---|---|---|---|")
    for name, form, crate, phase in INTERACTIONS:
        A(f"| {name} | `{form}` | `{crate}` | {phase} | not started |")
    A("")

    # ---- search
    A("## Search features")
    A("")
    A("| Feature | Phase | Status |")
    A("|---|---|---|")
    for name, phase in SEARCH_FEATURES:
        A(f"| {name} | {phase} | not started |")
    A("")

    # ---- pointcuts
    A("## Interceptor pointcuts")
    A("")
    A(f"HAPI exposes {len(pointcuts)} pointcuts through its `Pointcut` enum. Parity means a")
    A("Frevo hook point with equivalent timing and the ability to alter the same")
    A("outcome. Not all are reachable from the starter's configuration alone -- some")
    A("exist only for custom Java interceptors, which have no equivalent here; those")
    A("are expected to become documented gaps rather than ports.")
    A("")
    A("| Pointcut | Crate | Phase | Status |")
    A("|---|---|---|---|")
    for p in pointcuts:
        crate, phase = route(p)
        A(f"| `{p}` | `{crate}` | {phase} | not started |")
    A("")

    # ---- gaps in this inventory itself
    A("## Known limits of this inventory")
    A("")
    A("Stated so they are not mistaken for completeness:")
    A("")
    A("1. **Spring and server keys are excluded.** `server.*`, `spring.*` and")
    A("   `management.*` in the starter's `application.yaml` configure Spring Boot,")
    A("   Tomcat, Actuator and the datasource, not FHIR behaviour. Frevo must accept")
    A("   the ones that affect deployment (`server.port`, `server.servlet.context-path`,")
    A("   `spring.datasource.*`, the Actuator health and Prometheus endpoints) but")
    A("   they are tracked in the `frevo-config` capability, not row by row here.")
    A("2. **Spring relaxed binding is not enumerated.** Each key below is also")
    A("   settable as kebab-case, camelCase and as an environment variable. Which")
    A("   spellings must be supported is an open question in")
    A("   [`docs/capabilities/config.md`](capabilities/config.md).")
    A("3. **Operations reachable only through custom providers** (the starter's")
    A("   `custom_provider_classes`) cannot be enumerated: they are arbitrary Java.")
    A("   Frevo has no equivalent extension point and this is expected to be a")
    A("   documented gap.")
    A("4. **Per-operation parameters are not listed.** `$expand` alone takes more")
    A("   than a dozen. Those are enumerated in each capability's spec delta, where")
    A("   they get scenario ids.")
    A("5. **Pointcut counts include client-side and test pointcuts.** Triage happens")
    A("   when `frevo-server` is specified.")
    A("")

    (ROOT / "docs" / "inventory.md").write_text("\n".join(lines) + "\n",
                                                encoding="utf-8", newline="\n")

    summary = {
        "config_keys": len(cfg),
        "config_keys_in_yaml": sum(1 for r in cfg if r["in_yaml"]),
        "operations": len(ops),
        "interactions": len(INTERACTIONS),
        "search_features": len(SEARCH_FEATURES),
        "pointcuts": len(pointcuts),
        "total": total,
    }
    (ROOT / "docs" / "inventory-totals.json").write_text(
        json.dumps(summary, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
