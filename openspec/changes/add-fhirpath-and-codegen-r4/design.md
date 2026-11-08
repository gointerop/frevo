# Design — `add-fhirpath-and-codegen-r4`

Rules this file answers to, from `openspec/config.yaml` `rules.design`:
state the performance budget with the benchmark that measures it; justify any
new dependency against the PostgreSQL-only rule; say which parts are generated
and which generator produces them; record the PostgreSQL schema changes.

## 1. What is generated, and by what

| Artifact | Produced by | From | Marked |
|---|---|---|---|
| `crates/frevo-model/src/r4/**` — resource and datatype types, their JSON property maps, element-order tables, required-binding enumerations | `frevo-codegen` | `hl7.fhir.r4.core#4.0.1` StructureDefinitions | `@generated` |
| `crates/frevo-model/src/r4/searchparam.rs` — the R4 search-parameter table | `frevo-codegen` | the same package's `SearchParameter` resources | `@generated` |
| `crates/frevo-model/src/codec/**` — the lazy, byte-preserving codec | hand-written | — | not generated |
| `crates/frevo-path/**` | hand-written | derived from `FHIRPathEngine.java`; see [`proposal.md`](proposal.md) for the provenance table | not generated |

Generated output is **checked in**, not produced by `build.rs`. Three reasons:
`openspec-guard` looks for the `@generated` marker in committed files;
constitution rule 4 says a wrong output is fixed in the generator and
regenerated, which needs the output to be diffable; and a `build.rs` that reads
a 10 MB definition package would be paid on every clean build by every crate
downstream. A CI drift check (`codegen.output.005`) is what keeps the checked-in
output honest.

## 2. How the generator is driven

### Input, and why it is not `reference/`

`reference/` is cloned read-only, is git-ignored, and the constitution says it
is never compiled into the product. A generator that read it would make the
model unreproducible for anyone who had not run `reference/clone.sh`.

So the authoritative input is the **published FHIR package**
`hl7.fhir.r4.core#4.0.1`, fetched once by a script into a git-ignored directory
and verified against a SHA-256 recorded in the repository. That is the same
discipline `oracles/fetch.sh` already uses for suites too large to vendor.

`frevo-codegen` reads, from the package:

| File in the package | What the generator takes |
|---|---|
| `StructureDefinition-*.json`, `kind` = `primitive-type` | the 20 primitive datatypes and their value regexes |
| `StructureDefinition-*.json`, `kind` = `complex-type` | the complex datatypes and `Extension` |
| `StructureDefinition-*.json`, `kind` = `resource` | the 146 concrete resources, plus `Resource` and `DomainResource` as shared element sets |
| each definition's `snapshot.element` | element name, path, `min`, `max`, `type[]`, `contentReference`, `binding` |
| `ValueSet-*.json` + `CodeSystem-*.json` | the codes behind a `required` binding |
| `SearchParameter-*.json` | the 1,375 search-parameter rows |

The HAPI copy under
`reference/hapi-fhir/hapi-fhir-validation-resources-r4/src/main/resources/org/hl7/fhir/r4/model/`
at SHA `7ab4c7c7c49281496faa1b3c2498a652457669ae` is the **cross-check**, not the
input: a test that runs only when `reference/` is present asserts the two agree
on the StructureDefinition set and on every element's name, type and cardinality
(`codegen.inputs.005`). That test is how we find out if the published package
and the one HAPI actually validates against have drifted, which would silently
make the model disagree with the reference implementation.

### Driving it

```
frevo-codegen generate --version r4 --package <dir> --out crates/frevo-model/src/r4
```

The generator is a workspace binary, not a library consumed at build time. It is
not a runtime dependency of anything.

### Extensions: left as raw elements

`docs/capabilities/codegen.md` asks whether extensions are modelled as typed
structures or left raw. **Left raw.** `Extension` is generated as the ordinary
complex type it is, with a `url` and a `value[x]`; the standard extension
definitions in the package are read only to check that the generator can
*parse* them, and no Rust type is generated per extension. Profile-aware typed
extension access is a validator concern and belongs with `add-validator-profiles`.
Generating 500-odd extension structs now would be volume with no consumer.

## 3. The lazy, byte-preserving codec

### Representation

A resource is one of two things, and callers cannot tell which
(`model.bytes.006`):

```
Resource
├── Stored   { bytes: Arc<[u8]>, root: Span, index: OnceCell<Index>, dirty: bool }
└── Built    { ... owned element tree ... }
```

`Span` is a byte range into `bytes`. `Index` is built lazily and only as deep as
navigation has gone: for each object that has been entered, a small map from
property name to the span of its value. Entering an object is a single scan of
*that* object's braces, not of its children.

An element handle, which is what navigation hands out, is:

```
Node { bytes: Arc<[u8]>, value: Option<Span>, sibling: Option<Span>, def: &'static ElementDef }
```

- `value` is the span of the element's own JSON value, absent when the element
  exists only as a `_field` sibling.
- `sibling` is the span of the matching `_name` value, absent when there is
  none.
- `def` is the generated static element definition: FHIR type name, cardinality,
  the choice set when the element is `value[x]`, and the element-order table for
  serialisation.

`Arc<[u8]>` is why values can be **owned** and still cheap: cloning a `Node` is
an atomic increment plus two ranges, and the node outlives the node it came from
(`model.nav.007`), which is exactly what ADR 0004 says `haste-fhirpath`'s
borrowed `&'a dyn MetaValue` made impossible.

### The `_field` primitive-extension form

FHIR JSON puts a primitive's `id` and `extension` in a **sibling property**
named `_` plus the element name. Three cases, and the representation handles all
three without materialising either side:

**Singleton.** `"birthDate": "1974-12-25"` with `"_birthDate": {...}` becomes one
`Node` with `value` = the span of `"1974-12-25"` and `sibling` = the span of the
`_birthDate` object.

**Repeating, aligned by index.** `"given": ["Peter", "James"]` with
`"_given": [{...}, null]` becomes two `Node`s: index `i` takes `value` from
`given[i]` and `sibling` from `_given[i]`. A `null` in either array means
"nothing at this index" for that side.

**Value-less.** `"given": [null, "Peter"]` with `"_given": [{...}, null]` gives a
first node with `value: None, sibling: Some(..)` and a second with
`value: Some(..), sibling: None`. This is what makes
`Patient.name.given.select($this.hasValue())` return `false, true` — oracle case
`miscEngineTests/testPrimitiveExtensions`, specified as `model.primext.004`.

`hasValue()` is therefore `value.is_some()` and nothing more. That is the whole
reason to keep the two spans separate instead of merging them into a struct: a
merged struct cannot represent "element present, value absent" without an extra
flag, and getting that flag wrong is a silent wrong answer in a validation
invariant.

`ElementDef` carries whether the element is primitive, so a `_name` property on
a complex element is rejected rather than quietly accepted
(`model.primext.009`).

### Mutation and splicing

`Stored` starts `dirty: false`. A write replaces the written subtree with an
owned value and records the span it replaced. Serialisation walks the original
bytes and, at each recorded span, emits the new encoding instead of the original
bytes. Everything else is `copy_from_slice`. So:

- no write at all — output is the input, by `Arc` clone (`model.bytes.001`);
- a `meta` update on a 64 KiB resource — one small encode plus two memcpys, which
  is why `model.perf.004` can require that the 4 KiB and 64 KiB cases be within
  25% of each other;
- a write inside `contained[0]` — splices at that resource's boundary
  (`model.mutate.006`).

### Which upstream parser governs which part, and the two parse modes

Two different Java parsers are in play, and conflating them was the single
biggest error in the first draft of this design.

| Concern | Upstream authority | Pinned at |
|---|---|---|
| The **shape** of the representation: the `_field` sibling form, choice property names, `resourceType` dispatch, contained resources | `org.hl7.fhir.r4.formats.JsonParser` | `73c432b5` |
| The **canonical** form the round-trip oracle compares | `org.hl7.fhir.r4.formats.JsonCreatorCanonical` | `73c432b5` |
| What the server **accepts** over HTTP | `ca.uhn.fhir.parser.JsonParser` plus the handler `FhirContext` installs | `7ab4c7c7` |

The third row is the one that decides observable behaviour, and it is lenient.
`FhirContext` declares
`private volatile IParserErrorHandler myParserErrorHandler = new LenientErrorHandler();`,
and `LenientErrorHandler`'s own class documentation states that it "logs issues
but does not abort parsing, with only two exceptions". Those two are
`invalidValue` and `extensionContainsValueAndNestedExtensions`. Everything else
— `unknownElement`, `unknownAttribute`, `incorrectJsonType`,
`unexpectedRepeatingElement`, `missingRequiredElement`,
`containedResourceWithNoId`, `unknownReference`, `invalidInternalReference` —
logs a warning and returns.

So HAPI 8.12.0 answers `2xx` to all of these, and `frevo-model` must too:

| Body | HAPI | Scenario |
|---|---|---|
| `Patient` with `"nmae": []` | accepted, warning logged | `model.json.004` |
| `"active": "true"` where `boolean` belongs | accepted, warning logged | `model.json.013` |
| `"name": {...}` where a `0..*` array belongs | accepted, warning logged | `model.json.014` |
| `"birthDate": [...]` where a `0..1` value belongs | accepted, first repetition kept | `model.json.015` |
| `Observation` with no `status` | accepted, warning logged | `codegen.cardinality.002` |
| `"gender"` twice | accepted, last wins | `model.json.016` |
| `"name": [null]` | accepted as a value-less element | `model.json.018` |
| `Extension` with both a value and nested extensions | **rejected**, `DataFormatException` | `model.json.022` |
| a primitive with a non-blank invalid value | **rejected**, `DataFormatException` | `model.json.023` |

Two of those nine have no error-handler callback at all, so they are read from
the parser rather than from the handler. HAPI reads JSON through a Jackson
`ObjectMapper` built without `FAIL_ON_READING_DUP_TREE_KEY` and without
`STRICT_DUPLICATE_DETECTION`, so a repeated field name silently overwrites —
hence last-wins for a duplicate key. And `JsonParser.parseChildren` has an
explicit `theJsonVal.isNull()` branch that enters the element, parses its
`_name` sibling and ends it, so a `null` in an array is accepted rather than
rejected.

The same `ObjectMapper` enables `USE_BIG_DECIMAL_FOR_FLOATS` and uses a
`JsonNodeFactory(true)`, which is independent corroboration that decimals must
not go through a binary float — the point the next subsection makes.

`frevo-model` therefore offers **both** modes, as HAPI does:

- **lenient**, the default. Decoding succeeds and returns the resource plus a
  list of diagnostics. This is what the HTTP surface uses, and it is why a body
  HAPI accepts is accepted.
- **strict**, on request. Every diagnostic becomes an error instead. This is
  HAPI's `StrictErrorHandler`, selected the same way HAPI selects it
  (`FhirContext.setParserErrorHandler`), and it is what the validator will want.

`model.json.021` requires the two sets to be identical — same paths, same
offsets — so they cannot drift apart. **This is a match with HAPI, not a
deviation**, which is why `docs/deviations.md` records it under "Considered and
not recorded as deviations" rather than as a `DEV-nnnn`.

### Two serialisation orders, deliberately

| Form | Property order | Used for |
|---|---|---|
| **stored / normal** | the generated element order from the StructureDefinition snapshot | what the server stores and returns |
| **canonical** | property names sorted lexicographically, no whitespace | the round-trip oracle's comparison |

The canonical form is not invented here: it is what
`org.hl7.fhir.r4.formats.JsonCreatorCanonical` produces — it collects each
object's property names and sorts them, and refuses to pretty-print. Using
HAPI's own canonical form is what makes `model.canonical.005` a meaningful
comparison rather than a comparison against our own convention.

Decimals are held as their **lexical form** plus a parsed arbitrary-precision
value, never as `f64`. `1.58700` must re-serialise as `1.58700`
(`model.canonical.003`) and `1.58700.precision()` must be `5`
(`path.literal.006`); both are lost the moment the number goes through a binary
float.

### Answering the capability's open questions

`docs/capabilities/model.md` asks two questions. The answers are above and are
recorded here so the stub can be retired:

1. *How is the original byte range of an unmodified element retained?* By
   `Arc<[u8]>` plus a `Span` per element handle, with a lazily built per-object
   index; the resource keeps the buffer alive and the spans are the retention.
2. *What is the representation for primitive extensions?* Two independent spans
   per node — value and `_field` sibling — never merged, with index alignment and
   `null` holes for the repeating case.

## 4. The evaluation seam: three traits, owned values

ADR 0004 decision 5 requires `frevo-path` to evaluate over a trait, not over
`frevo-model`. The lesson it draws from `haste-fhirpath` is specific: 77
`downcast_ref` calls to one model's concrete types, and FHIRPath semantics turn
a failed downcast into an empty collection rather than an error.

The seam is split in three, mirroring how `FHIRPathEngine` is split between
`Base`, `IWorkerContext` and `IHostApplicationServices` — because that split is
what makes the engine testable without a model.

### 4.1 `Value` — the owned result type

```rust
pub enum Value {
    Boolean(bool),
    Integer(i64),
    Decimal(Decimal),          // lexical form + arbitrary precision
    String(String),
    Date(PartialDate),         // precision-carrying
    DateTime(PartialDateTime), // precision- and offset-carrying
    Time(PartialTime),
    Quantity(Quantity),
    Node(Arc<dyn Node>),       // a FHIR element or resource
}
```

Owned, as ADR 0004 requires. `Arc<dyn Node>` is the one indirection, and it is
there so a lazy model can hand out a handle rather than a materialised tree.
Each primitive variant also carries the optional `_field` sibling, so a value
that came from a primitive element keeps its `id` and `extension` reachable —
this is the same thing `helios-fhirpath-support`'s `PrimitiveElement` does, and
ADR 0004 cites it approvingly for exactly this reason.

### 4.2 `Node` — navigation

```rust
pub trait Node: Send + Sync {
    /// FHIR type name of this node: "Patient", "HumanName", "code".
    fn type_name(&self) -> &str;

    /// Is this node a resource? `%resource` tracking needs this.
    fn is_resource(&self) -> bool;

    /// Children under `name`, in document order, owned.
    /// `name` is an element name with no `[x]` suffix: a choice element is
    /// reached by its base name, and the typed spelling yields nothing.
    fn children(&self, name: &str) -> Result<Vec<Value>, NodeError>;

    /// Every child, paired with its element name, in document order.
    /// Backs children() and descendants().
    fn all_children(&self) -> Result<Vec<(String, Value)>, NodeError>;

    /// This node's primitive value, or None when it has no value —
    /// which is exactly what hasValue() reports.
    fn primitive(&self) -> Option<Primitive>;
}
```

Four properties of this signature are load-bearing:

- **Owned returns.** `Vec<Value>`, not `&dyn`. A borrow with the receiver's
  lifetime would force the model to hold a materialised value to lend a
  reference to, which is the specific thing ADR 0004 says would have ended the
  lazy codec.
- **`Result`, not `Option`.** A decode error must propagate as an error. An
  `Option` return would make a malformed document indistinguishable from an
  absent element, and FHIRPath would turn that into an empty collection — the
  silent-wrong-answer mode (`path.errors.005`).
- **`children(name)` takes a base name.** The engine never constructs
  `valueQuantity`; the polymorphic rule lives in the model's element
  definitions, so `Observation.valueQuantity` yields nothing at runtime and the
  static check is what rejects it (`path.navigation.007`, `model.nav.004`).
- **No `resource()` / `root_resource()`.** `%resource` and `%rootResource` are
  evaluation-context state, updated when traversal enters a node whose
  `is_resource()` is true. Putting them on the node would require every node to
  know its ancestors, which a span-based handle does not.

### 4.3 `Model` — type information

```rust
pub trait Model: Send + Sync {
    fn is_type(&self, type_name: &str) -> bool;
    fn base_type(&self, type_name: &str) -> Option<&str>;      // code -> string
    fn element_types(&self, type_name: &str, element: &str)
        -> Option<Vec<String>>;                                 // value[x] -> the choice set
    fn is_primitive(&self, type_name: &str) -> bool;
    fn is_resource_type(&self, type_name: &str) -> bool;
}
```

This is what makes `Patient.gender.is(string)` true (`code` derives from
`string`) and `Observation...value is Age` and `is Quantity` both true while
`is Duration` is false — oracle group `testInheritance`, specified as
`path.types.004` and `path.types.006`. It is also the whole input to the static
check pass, which is why the check pass can be tested with a stub `Model` and no
resource at all.

### 4.4 `HostServices` — what the engine cannot know

```rust
pub enum ConstantMode { Explicit, NoValue, ImplicitBefore, ImplicitAfter }

pub trait HostServices: Send + Sync {
    // --- constants -------------------------------------------------------
    fn resolve_constant(&self, name: &str, mode: ConstantMode)
        -> Result<Vec<Value>, HostError>;
    fn resolve_constant_type(&self, name: &str, mode: ConstantMode)
        -> Result<Option<TypeSet>, HostError>;

    // --- resolution ------------------------------------------------------
    fn resolve_reference(&self, url: &str, identifier: Option<&Value>,
                         context: Option<&Value>)
        -> Result<Option<Value>, HostError>;
    fn conforms_to_profile(&self, item: &Value, url: &str) -> Result<bool, HostError>;
    fn resolve_value_set(&self, url: &str) -> Result<Option<Value>, HostError>;

    // --- host-supplied functions -----------------------------------------
    fn resolve_function(&self, name: &str) -> Option<FunctionDetails>;
    fn check_function(&self, name: &str, focus: &TypeSet, params: &[TypeSet])
        -> Result<TypeSet, HostError>;
    fn execute_function(&self, name: &str, focus: &[Value],
                        params: &[Vec<Value>]) -> Result<Vec<Value>, HostError>;
    fn param_is_type(&self, name: &str, index: usize) -> bool;

    // --- diagnostics -----------------------------------------------------
    fn log(&self, name: &str, focus: &[Value]);
}

pub struct FunctionDetails { pub name: String, pub min_params: usize, pub max_params: usize }
```

Four things in that signature are not cosmetic.

**`resolve_constant` returns a `Vec`.** HAPI's
`FHIRPathTests.resolveConstantReturnsList` asserts that a host service may
return two values for one `%name`, so a single-value signature would be wrong
(`path.env.012`).

**`ConstantMode` has exactly four values.** They are the four
`FHIRPathConstantEvaluationMode` carries, and `FhirPathR4.setEvaluationContext`
maps them one-for-one onto `IFhirPathEvaluationContext.ConstantEvaluationMode`.
A host service written against HAPI's wrapper therefore sees the same value
Frevo passes. The explicit mode is in force for a `%name` the expression writes;
the no-value mode is in force during the check pass, which wants a type and not
a value (`path.env.015`).

**The four function methods exist because `Function::Custom` does.** The parser
takes a custom function's arity from the host service's `FunctionDetails`
(`min_params`/`max_params`), the check pass asks `check_function` for the result
type, evaluation calls `execute_function`, and `param_is_type` says which
argument is a type name to be passed unevaluated rather than evaluated. Omitting
them — as the first draft of this design did — would have left the whole
`IHostApplicationServices` extension point unimplementable, and the validator
and CQL both use it (`path.utility.027`).

**`resolve_reference` takes an `identifier`.** `funcResolve` reads both
`Reference.reference` and `Reference.identifier` and passes both on, so a host
service that can resolve by business identifier is able to
(`path.utility.017`).

Three implementations exist by the end of this change:

| Implementation | `conformsTo` | Used by |
|---|---|---|
| **none** (unset) | **raises** a no-host-services error (`path.utility.022`) | most oracle cases |
| the oracle harness's | `true` for the `Patient` profile, `false` for `Person`, error otherwise — exactly `FHIRPathTestEvaluationServices` | the suite |
| the server-facing one | `false` unconditionally — `FhirPathR4.setEvaluationContext`'s anonymous implementation (`path.utility.014`) | `frevo-server` later |

The first row is the one the first draft of this design missed: "no host
service" is not the same state as "a host service that says no", and the engine
distinguishes them.

### 4.5 Enforcing the separation

`path.api.008` makes it a test: `frevo-path`'s manifest must not name
`frevo-model`, and the crate must compile and pass its unit tests without it.
The dependency edge runs the other way — `frevo-model` depends on `frevo-path`
for the trait definitions. That is also what lets `model.nav.012` compare
evaluation over `frevo-model` with evaluation over the harness's XML-backed test
node and require equal results.

### Answering the capability's open questions

`docs/capabilities/path.md` asks two:

1. *Which FHIRPath version does HAPI 8.12.0 implement, and which non-standard
   functions does it add?* The suite declares
   `reference="http://hl7.org/fhirpath|2.0.0"` and 31 cases carry
   `version="2.1.0"`. The R4 `ExpressionNode.Function` enum adds, beyond the
   base specification: `matchesFull`, `encode`, `decode`, `escape`, `unescape`,
   `trim`, `split`, `join`, `lowBoundary`, `highBoundary`, `precision`,
   `comparable`, `memberOf`, `conformsTo`, `defineVariable`, `check`, `sort`,
   `htmlChecks`, `htmlChecks2`, `hasTemplateIdOf`, `type`, `hasValue`,
   `children`, `descendants`, `extension` and `resolve`. It does **not** add
   `section`, `entry`, `entryIn`, `resources`, `resourcesIn`, `sameSubject` or
   `hasCoding` — the R5 engine adds only the first two — which is why the
   `documentFunctions` group — 115 of the 1,191 cases — is out of scope
   (`path.oracle.014`).
2. *Is evaluation needed over the lazy model without full materialisation, and
   at what cost?* Yes, and the cost is the `Arc<dyn Node>` indirection plus a
   `Vec` allocation per `children()` call. `path.perf.003` and `path.perf.004`
   are the budget and the check.

## 5. UCUM

Quantity comparison, `comparable()`, and quantity multiplication and division
need UCUM canonical form. `FHIRPathEngine` delegates to
`worker.getUcumService()` (`org.fhir:ucum`), and where that service is absent it
takes a different branch — so "no UCUM" is an observable behaviour, not a
degradation we may choose.

Decision: implement the unit algebra inside `frevo-path` as a private module,
driven by `ucum-essence.xml`.

- **Not a new runtime dependency** in the sense the constitution restricts: it
  is our own code plus a data file, not a service. No crate is added.
- No Rust UCUM crate is on `deny.toml`'s allow-list today, and adding one would
  need its own ADR; the oracle needs only canonical-form conversion,
  commensurability and unit multiplication/division, which is a bounded piece of
  work.
- `ucum-essence.xml` is **fetched**, not vendored
  (`oracles/fhir-test-cases/fetched/ucum/ucum-essence.xml`), and needs a
  `NOTICE` entry for Regenstrief's UCUM copyright. That is a task here and a
  `chore/oracles-*` change, per [`proposal.md`](proposal.md).
- If `frevo-terminology` later needs the same algebra for `$validate-code`
  against UCUM, extracting it is a follow-up change, not a reason to put it in
  the wrong crate now.

`path.quantity.016` makes the file's presence and version a reported, failing
condition rather than a silent fallback.

## 6. Dependencies

Against the PostgreSQL-only rule and `deny.toml`:

| Dependency | Where | Justification |
|---|---|---|
| none new, runtime | — | `frevo-path` needs no crate beyond the workspace's existing `thiserror`; `frevo-model` needs none beyond an arbitrary-precision decimal, below |
| arbitrary-precision decimal | `frevo-model`, `frevo-path` | required by `model.json.009`, `model.canonical.003` and `path.literal.006`: a decimal must keep its lexical precision, which `f64` cannot. The crate chosen must be on `deny.toml`'s allow-list; if none is, the fallback is a lexical-form-plus-integer-mantissa representation written here, and that choice is recorded in `tasks.md` |
| `helios-fhirpath` 0.2.7 (MIT) | **dev-dependency of the comparison harness only** | ADR 0004 decision 3. Never in a shipped crate's graph (`path.oracle.013`) |
| `haste-fhirpath` 0.129.3 | **not added to the workspace at all** | it declares wildcard versions (`0.*`) for three dependencies, and `deny.toml` sets `wildcards = "deny"`. Measuring it (task 2) happens in `harness/fhirpath-compare/`, a crate listed in `[workspace] exclude`, so the product's dependency graph and the `cargo deny` gate are untouched. This is the only way to get ADR 0004's missing number without relaxing a rule |
| criterion or divan | dev-dependency of the benched crates | needed for `bench/micro/`; dev-only, not in the shipped binary |

No broker, no cache, no search engine, no second database. Nothing here touches
that rule.

## 7. PostgreSQL

**No schema change, no migration, no effect on existing rows.** This change
produces no SQL and opens no connection. `frevo-store` does not exist yet, and
`add-store-schema` is the change that decides how the byte-preserved resource is
stored.

What this change does is **constrain** that later decision, and it is worth
recording now so it is not rediscovered:

- the stored column holds the canonical JSON **bytes**, so it is `bytea` (or
  `text`), never `json` or `jsonb` — `jsonb` normalises key order and drops
  duplicate keys, which destroys byte identity and therefore destroys
  `model.bytes.001` and the entire `read-by-id` target;
- whatever compression `add-store-schema` chooses must be decompress-to-bytes,
  not decompress-and-parse, or the pass-through becomes a parse;
- the generated search-parameter table is the input to
  `add-search-params`' index design, and it is generated from the same package
  as the model so the two cannot disagree.

## 8. Performance budgets

Context: `bench/targets.md` is **proposed, awaiting owner approval**, and its
own "Decisions I need from the owner" section names *this change* as the one
that will show whether its ratios are too soft or too hard. The budgets below
are derived from its numbers; they are not independent targets.

### Why micro-benchmarks here

`bench/workloads.yaml` is fourteen HTTP workloads against a running server.
There is no server in this change, so none of them can run. Rather than defer
measurement until `add-server-skeleton` — by which time the codec's shape is
fixed — this change adds `bench/micro/`, in-process benchmarks whose numbers are
the **leading indicators** for the HTTP targets.

### The derivation

Two targets in `bench/targets.md` reach down into this change:

**`read-by-id`: p99 ≤ 5 ms, ≥ 5,000 rps, 14× HAPI.** That table's own
explanation of the ratio is "Frevo stores canonical JSON as bytes and returns it
without parsing, so a read is one indexed lookup plus a socket write. The
remaining cost is almost entirely syscalls." For that to be true, the model's
contribution to a read must be negligible against a database round trip and a
socket write — call it under 1% of a 200 µs budget at 5,000 rps, so **≤ 1 µs**.
That is a byte copy and an `Arc` clone, which is `model.perf.001`.

**Bulk ingest: ≥ 5,000 resources/s, 5× HAPI.** On the reference environment's 12
CPUs, 5,000 resources/s leaves 2.4 ms of core time per resource. Search-parameter
extraction is one of several stages on that path (parse, validate, extract,
`COPY`), so giving it 12.5% of the budget is 300 µs per resource — which is
`path.perf.003`. A `Patient` has roughly 25 enabled search parameters, so that
is about 12 µs per expression including navigation, against a per-expression
evaluation budget of 5 µs for a simple path (`path.perf.002`) — the margin is
for the compound expressions, and it is deliberately not generous.

### The workloads

New file `bench/micro/` with these, each a criterion/divan benchmark in the
crate it measures:

| Workload | Crate | Measures | Budget | Scenario |
|---|---|---|---|---|
| `model-passthrough` | `frevo-model` | load from bytes, serialise unchanged, 4 KiB `Patient` | median ≤ 1 µs, zero tokens scanned | `model.perf.001` |
| `model-lazy-read` | `frevo-model` | read `Patient.birthDate` from 4 KiB bytes | median ≤ 2 µs, ≤ 10% of `model-full-parse` | `model.perf.002` |
| `model-full-parse` | `frevo-model` | decode a 4 KiB `Patient` completely | median ≤ 20 µs | `model.perf.003` |
| `model-meta-update` | `frevo-model` | set `meta.versionId` and `meta.lastUpdated`, serialise, at 4 KiB and 64 KiB | the two medians within 25% | `model.perf.004` |
| `model-canonical-encode` | `frevo-model` | canonically serialise a constructed 4 KiB `Patient` | median ≤ 25 µs | `model.perf.005` |
| `path-compile` | `frevo-path` | parse and check a typical search-parameter expression | median ≤ 20 µs | `path.perf.001` |
| `path-eval-compiled` | `frevo-path` | evaluate a pre-parsed simple path over a 4 KiB `Patient`, 1,000 times | median ≤ 5 µs per evaluation, no reparse | `path.perf.002` |
| `path-extract-patient` | `frevo-path` | evaluate every enabled `Patient` search-parameter expression over one 4 KiB `Patient` | median total ≤ 300 µs | `path.perf.003` |
| `path-suite-wallclock` | harness | the full 1,191-case oracle run | ≤ 10 s | `path.perf.005` |

The 4 KiB `Patient` is a fixed fixture, committed with the benchmark, so the
numbers are comparable across commits. The reference environment is the one
`bench/targets.md` states: Docker Desktop on Windows, a Linux VM with 12 CPUs and
8 GiB RAM.

### Regression policy

`bench/targets.md`'s policy applies unchanged: a regression of more than 10% on
any workload fails the performance gate (`model.perf.006`, `path.perf.006`), and
a regression that cannot be resolved is an owner checkpoint, not something to
absorb.

### One budget that is not about requests

`cargo build -p frevo-model` must stay usable, because every other crate depends
on it. 146 resource types with a full codec each is a lot of generated code.
Budget: **a cold `cargo build -p frevo-model` in release mode at most 120 s on
the reference environment**, and the generated source at most 40 MB. If the
generator cannot meet that, the lever is to generate the codec as data tables
interpreted at runtime rather than as per-type code — which is a design change,
so it goes in `docs/adr/` rather than being improvised. This is stated here
because discovering it after `frevo-store`, `frevo-search` and `frevo-validator`
all depend on the crate would be expensive.

## 9. Where things live

```
crates/frevo-codegen/            generator binary
crates/frevo-model/src/r4/       @generated output
crates/frevo-model/src/codec/    hand-written lazy codec
crates/frevo-path/src/           lexer, parser, check pass, evaluator, ucum
crates/frevo-path/tests/         the oracle harness and its per-case report
harness/fhirpath-compare/        NOT a workspace member; helios + haste measurement
bench/micro/                     the workloads in section 8
```

`harness/fhirpath-compare/` being outside the workspace is the point: it is the
only place `haste-fhirpath`'s wildcard dependencies can be resolved without
failing `cargo deny`, and it ships in nothing.

## 10. What would make this design wrong

Recorded so the next session does not have to infer it:

- **`model.perf.001` missed.** If pass-through cannot be made a byte copy, the
  `read-by-id` premise in `bench/targets.md` is wrong and the whole ratio table
  needs revisiting. Owner checkpoint.
- **ADR 0004 reversed by task 2.** If `helios-fhirpath` measures at or above
  HAPI's rate *and* its `IntoEvaluationResult` can be implemented over the lazy
  model, sections 4.1 to 4.5 are replaced by an adapter and most of
  `specs/path/spec.md` becomes a conformance check on a dependency rather than a
  specification of our own engine. The ADR says so; this is where it lands.
- **The compile-time budget missed.** Pushes the codec from generated code to
  interpreted tables, which changes section 3's representation. New ADR.
