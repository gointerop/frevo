#!/usr/bin/env python3
"""Response normalisation for differential testing.

Two conformant FHIR servers answering identical requests differ in ways that
carry no meaning: they assign different resource ids, stamp different
timestamps, mint different paging tokens, and may order a Bundle's entries
differently where the specification leaves the order undefined. Comparing raw
responses would therefore report a diff on nearly every case.

Normalisation removes exactly those differences -- and nothing else.

**Every rule you apply is a field you have stopped checking.** A too-broad rule
turns a parity test into a test that passes whatever the server does, which is
worse than having no test, because it reports confidence that was never earned.
Rules are therefore opt-in per case, named in the corpus entry, and each one is
documented below with what it stops checking.

Never add a rule to make a diff go away. A real difference is either a bug or a
recorded deviation in docs/deviations.md.
"""
from __future__ import annotations

import copy
import json
import re
from typing import Any

# --------------------------------------------------------------------------
# Rule catalogue
# --------------------------------------------------------------------------
# name -> what it does, and what it stops checking.
RULES: dict[str, str] = {
    "ids": (
        "Replaces server-assigned resource ids with stable placeholders, "
        "consistently within one response (first-seen order). "
        "STOPS CHECKING: the literal id values. Still checks that the same id "
        "is used in the same places, so broken internal references are caught."
    ),
    "meta_lastupdated": (
        "Replaces meta.lastUpdated with a placeholder. "
        "STOPS CHECKING: the instant a resource was written."
    ),
    "meta_versionid": (
        "Replaces meta.versionId with a placeholder. "
        "STOPS CHECKING: version numbering. Use only where the case does not "
        "concern versioning -- history and optimistic-locking cases must not "
        "use this rule."
    ),
    "timestamps": (
        "Replaces server-generated instants outside meta (Bundle.timestamp, "
        "OperationOutcome issue timings, job timestamps). "
        "STOPS CHECKING: those instants. Does NOT touch clinical dates, which "
        "come from the request and must match exactly."
    ),
    "paging_links": (
        "Normalises Bundle.link urls: strips the server base and replaces "
        "opaque continuation tokens. "
        "STOPS CHECKING: the token format. Still checks which relations are "
        "present (self/next/previous/first/last), which is the behaviour that "
        "matters."
    ),
    "bundle_entry_order": (
        "Sorts Bundle.entry by a content hash. "
        "STOPS CHECKING: entry order. Only legitimate where the specification "
        "leaves order undefined -- NOT for a sorted search (_sort), a "
        "transaction response (which must mirror request order), or history "
        "(which is ordered by time)."
    ),
    "base_url": (
        "Replaces the server's own base url wherever it appears. "
        "STOPS CHECKING: absolute url construction beyond the base."
    ),
    "narrative": (
        "Removes generated narrative (text.div). "
        "STOPS CHECKING: narrative content entirely. HAPI templates with "
        "Thymeleaf and Frevo with minijinja, so identical narrative is not "
        "achievable; narrative parity is tracked separately."
    ),
    "outcome_text": (
        "Removes OperationOutcome issue.diagnostics and text, keeping "
        "severity, code, and expression/location. "
        "STOPS CHECKING: human-readable message wording. Message-text parity "
        "is far more expensive than severity parity; where a case needs exact "
        "wording, do not use this rule."
    ),
    "etag": (
        "Normalises ETag and Content-Location headers. "
        "STOPS CHECKING: the literal version token in those headers."
    ),
    "_last_modified_note": (
        "Not a rule you select. The Last-Modified header's value is "
        "placeholdered whenever meta_lastupdated or timestamps is applied, "
        "because it carries the same server-generated instant. "
        "STOPS CHECKING: that instant. The header's presence is still checked."
    ),
    "volatile_headers": (
        "Drops Date, Server, X-Powered-By, Content-Length, Keep-Alive and "
        "similar transport headers. "
        "STOPS CHECKING: nothing of FHIR significance."
    ),
}

PLACEHOLDER_ID = "<id:{}>"
PLACEHOLDER_TS = "<timestamp>"
PLACEHOLDER_VER = "<version>"
PLACEHOLDER_TOKEN = "<token>"

INSTANT_RE = re.compile(
    r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2})$"
)
UUID_RE = re.compile(
    r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}"
)

# Instants that a server generates rather than echoes. Clinical dates are
# deliberately absent: they come from the request and a difference in them is a
# real bug.
GENERATED_INSTANT_KEYS = {
    "timestamp",      # Bundle.timestamp
    "lastUpdated",
    "lastModified",   # Bundle.entry.response.lastModified
    "issued",         # only inside a Bundle wrapper; see _walk
    "startTime", "endTime", "createTime", "transactionTime",
    "expiry", "authored",
}

VOLATILE_HEADERS = {
    "date", "server", "x-powered-by", "content-length", "keep-alive",
    "connection", "transfer-encoding", "x-request-id", "x-correlation-id",
    "vary", "set-cookie", "strict-transport-security",
}


class Normalizer:
    """Applies a named set of rules to one response.

    Id mapping is per-response and order-based, so two servers that assign
    different ids still normalise to the same placeholders as long as they use
    them in the same structural positions.
    """

    def __init__(self, rules: list[str], base_urls: list[str] | None = None,
                 tokens: dict[str, str] | None = None):
        # Entries starting with "_" are documentation of implicit behaviour,
        # not selectable rules; selecting one would do nothing and imply it had.
        selectable = {k for k in RULES if not k.startswith("_")}
        unknown = [r for r in rules if r not in selectable]
        if unknown:
            raise ValueError(
                f"unknown normalisation rule(s): {unknown}. "
                f"Known rules: {sorted(RULES)}. "
                "A rule must be defined and documented before it can be used -- "
                "an undefined rule would silently normalise nothing."
            )
        self.rules = set(rules)
        self.base_urls = [u.rstrip("/") for u in (base_urls or []) if u]
        # Harness-level substitutions, applied unconditionally and independently
        # of the named rules. These are values that differ between the two runs
        # *by construction* rather than because the servers disagree: above all
        # the per-run isolation tag, which each run generates for itself and
        # then writes into the resources it creates. Without this the tag shows
        # up as a diff on every case that uses it.
        self.tokens = dict(tokens or {})
        # Longest first, so one token cannot partially shadow another.
        self._token_order = sorted(self.tokens, key=len, reverse=True)
        self._id_map: dict[str, str] = {}

    # ---- helpers ---------------------------------------------------------
    def _placeholder_for(self, raw: str) -> str:
        if raw not in self._id_map:
            self._id_map[raw] = PLACEHOLDER_ID.format(len(self._id_map))
        return self._id_map[raw]

    def _strip_base(self, s: str) -> str:
        for b in self.base_urls:
            s = s.replace(b, "<base>")
        return s

    # ---- body ------------------------------------------------------------
    def body(self, obj: Any) -> Any:
        obj = copy.deepcopy(obj)
        obj = self._walk(obj, path=())
        if "bundle_entry_order" in self.rules:
            obj = self._sort_bundles(obj)
        return obj

    def _walk(self, node: Any, path: tuple[str, ...]) -> Any:
        if isinstance(node, dict):
            out: dict[str, Any] = {}
            rt = node.get("resourceType")

            for k, v in node.items():
                # --- narrative
                if "narrative" in self.rules and k == "text" and isinstance(v, dict) \
                        and "div" in v:
                    out[k] = {"status": v.get("status"), "div": "<narrative>"}
                    continue

                # --- OperationOutcome wording
                if "outcome_text" in self.rules and rt == "OperationOutcome" \
                        and k == "issue" and isinstance(v, list):
                    out[k] = [
                        {
                            ik: ("<diagnostics>" if ik == "diagnostics" else
                                 self._walk(iv, path + (k, ik)))
                            for ik, iv in issue.items()
                            if ik != "details"
                        }
                        for issue in v if isinstance(issue, dict)
                    ]
                    continue

                # --- meta
                if k == "meta" and isinstance(v, dict):
                    m = dict(v)
                    if "meta_lastupdated" in self.rules and "lastUpdated" in m:
                        m["lastUpdated"] = PLACEHOLDER_TS
                    if "meta_versionid" in self.rules and "versionId" in m:
                        m["versionId"] = PLACEHOLDER_VER
                    out[k] = self._walk(m, path + (k,))
                    continue

                # --- resource id
                if "ids" in self.rules and k == "id" and isinstance(v, str) and rt:
                    out[k] = self._placeholder_for(v)
                    continue

                # --- Bundle.link
                if "paging_links" in self.rules and k == "link" and isinstance(v, list) \
                        and rt == "Bundle":
                    out[k] = [self._normalise_link(l) for l in v]
                    continue

                # --- generated instants
                if "timestamps" in self.rules and k in GENERATED_INSTANT_KEYS \
                        and isinstance(v, str) and INSTANT_RE.match(v):
                    # Bundle.timestamp and job timestamps are generated. An
                    # Observation.issued echoed from the request is not, so only
                    # normalise `issued` inside a Bundle wrapper, not on a
                    # stored clinical resource.
                    if k != "issued" or rt == "Bundle":
                        out[k] = PLACEHOLDER_TS
                        continue

                out[k] = self._walk(v, path + (k,))

            # Reference / fullUrl rewriting happens after the dict is built so
            # the id map is populated by the owning resource first.
            return self._rewrite_refs(out)

        if isinstance(node, list):
            return [self._walk(v, path) for v in node]

        if isinstance(node, str):
            return self._scalar(node)

        return node

    def _apply_tokens(self, s: str) -> str:
        for t in self._token_order:
            if t and t in s:
                s = s.replace(t, self.tokens[t])
        return s

    def _scalar(self, s: str) -> str:
        if "base_url" in self.rules or "paging_links" in self.rules:
            s = self._strip_base(s)
        return self._apply_tokens(s)

    def _rewrite_refs(self, node: dict[str, Any]) -> dict[str, Any]:
        if "ids" not in self.rules:
            return node
        for key in ("reference", "fullUrl", "url", "location"):
            v = node.get(key)
            if not isinstance(v, str):
                continue
            node[key] = self._rewrite_ref_string(v)
        return node

    def _rewrite_ref_string(self, v: str) -> str:
        v = self._apply_tokens(self._strip_base(v))
        # Type/id, Type/id/_history/vid, urn:uuid:...
        m = re.match(r"^(?:<base>/)?([A-Z][A-Za-z]+)/([A-Za-z0-9._-]{1,64})(/_history/.+)?$", v)
        if m:
            typ, rid, hist = m.groups()
            new = f"{typ}/{self._placeholder_for(rid)}"
            if hist:
                if "meta_versionid" in self.rules:
                    new += "/_history/" + PLACEHOLDER_VER
                else:
                    new += hist
            return new
        if v.startswith("urn:uuid:"):
            return "urn:uuid:" + self._placeholder_for(v[len("urn:uuid:"):])
        if UUID_RE.fullmatch(v):
            return self._placeholder_for(v)
        return v

    def _normalise_link(self, link: Any) -> Any:
        if not isinstance(link, dict):
            return link
        out = dict(link)
        url = out.get("url")
        if isinstance(url, str):
            url = self._apply_tokens(self._strip_base(url))
            # A link url embeds the resource id for instance-level queries
            # (<base>/Patient/1024/_history). Rewrite those through the same id
            # map as the bodies, so the link is still checked for pointing at
            # the same resource -- rather than tokenising the id, which would
            # stop checking that. `_history` is lower-case so the version
            # segment is left alone.
            if "ids" in self.rules:
                url = re.sub(
                    r"\b([A-Z][A-Za-z]+)/([A-Za-z0-9._-]{1,64})",
                    lambda m: f"{m.group(1)}/{self._placeholder_for(m.group(2))}",
                    url,
                )
            # HAPI's continuation tokens look like
            # ?_getpages=<uuid>&_getpagesoffset=20&...
            url = re.sub(r"(_getpages=)[^&]+", r"\1" + PLACEHOLDER_TOKEN, url)
            url = re.sub(r"(_searchId=)[^&]+", r"\1" + PLACEHOLDER_TOKEN, url)
            url = re.sub(r"(_pageId=)[^&]+", r"\1" + PLACEHOLDER_TOKEN, url)
            url = UUID_RE.sub(PLACEHOLDER_TOKEN, url)
            # Query parameter order is not significant.
            if "?" in url:
                head, _, q = url.partition("?")
                parts = sorted(p for p in q.split("&") if p)
                url = head + "?" + "&".join(parts)
            out["url"] = url
        return out

    def _sort_bundles(self, node: Any) -> Any:
        if isinstance(node, dict):
            out = {k: self._sort_bundles(v) for k, v in node.items()}
            if out.get("resourceType") == "Bundle" and isinstance(out.get("entry"), list):
                out["entry"] = sorted(
                    out["entry"],
                    key=lambda e: json.dumps(e, sort_keys=True, ensure_ascii=False),
                )
            return out
        if isinstance(node, list):
            return [self._sort_bundles(v) for v in node]
        return node

    # ---- headers ---------------------------------------------------------
    def headers(self, hdrs: dict[str, str]) -> dict[str, str]:
        out: dict[str, str] = {}
        for k, v in hdrs.items():
            lk = k.lower()
            if "volatile_headers" in self.rules and lk in VOLATILE_HEADERS:
                continue
            if lk == "etag" and "etag" in self.rules:
                out[lk] = re.sub(r'W?/?"[^"]*"', '"<version>"', v)
                continue
            # Last-Modified carries the resource's lastUpdated instant, so its
            # VALUE is server-generated while its PRESENCE is meaningful. It gets
            # the same treatment as meta.lastUpdated: placeholder the value, keep
            # the header. Without this, two runs that straddle a second boundary
            # differ -- which is a flake, not a parity failure, and it only
            # showed up once the database was large enough to slow the runs
            # down. The earlier all-green result was partly luck.
            if lk == "last-modified" and (
                "meta_lastupdated" in self.rules or "timestamps" in self.rules
            ):
                out[lk] = PLACEHOLDER_TS
                continue
            if lk in ("content-location", "location"):
                out[lk] = self._rewrite_ref_string(v) if "ids" in self.rules \
                    else self._strip_base(v)
                continue
            out[lk] = self._apply_tokens(v)
        return out


def describe_rules() -> str:
    lines = ["Normalisation rules", "==================="]
    for name in sorted(RULES):
        lines.append("")
        lines.append(f"{name}")
        lines.append("  " + RULES[name].replace(". ", ".\n  "))
    return "\n".join(lines)


if __name__ == "__main__":
    print(describe_rules())
