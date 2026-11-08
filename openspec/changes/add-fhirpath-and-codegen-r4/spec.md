# Spec delta — `add-fhirpath-and-codegen-r4`

This is the index to the spec delta. The **normative text** — every
`#### Scenario:` with its `WHEN`/`THEN` body — lives in three files, one per
capability, because that is the layout `openspec validate` parses and
`openspec archive` promotes into `openspec/specs/<capability>/spec.md`:

| Capability | Delta file | Scenarios |
|---|---|---:|
| `codegen` | [`specs/codegen/spec.md`](specs/codegen/spec.md) | 49 |
| `model` | [`specs/model/spec.md`](specs/model/spec.md) | 83 |
| `path` | [`specs/path/spec.md`](specs/path/spec.md) | 361 |
| **Total** | | **493** |

This file carries no normative text of its own. It carries the ID registry,
which is the thing a later change must not collide with.

## ID allocation

Format is `<capability>.<area>.<nnn>`, written in a delta as
`#### Scenario: [<id>] <description>`. IDs are allocated once and never
renumbered or reused, because tests reference them by ID forever.

No `codegen.*`, `model.*` or `path.*` ID was allocated before this change:
`openspec/specs/` holds only `server`, whose IDs are `server.metadata.001` and
`server.metadata.002`, and `openspec/changes/archive/` is empty. Every area
below therefore starts at `001` and is contiguous.

Revision after review: 32 IDs were **added** at the end of their areas to cover
behaviour the first draft missed, and 41 existing scenarios were **rewritten**
in place. None was removed. No ID was renumbered, reused or repurposed. The rewritten ones keep
their numbers because tests reference them by ID even when the behaviour they
describe is corrected; `path.subset.003`, `path.compare.007`,
`path.quantity.015`, `path.string.020`, `path.datetime.003`,
`path.utility.018`, `path.strictness.006`, `model.json.004`, `model.json.013`,
`model.json.014`, `model.json.015`, `model.json.016`, `model.json.018` and
`codegen.cardinality.002` all now state the opposite of what they first said,
because the pinned source says the opposite.

Revision after the second review: 7 further IDs were **added** at the end of
their areas — `model.json.024`, `model.json.025`, `model.canonical.007`,
`path.env.016`, `path.env.017`, `path.oracle.017` and `path.utility.028` — and 6
existing scenarios were **rewritten** in place: `path.oracle.002`,
`path.oracle.007`, `path.oracle.008`, `path.env.015`, `path.utility.001` and
`path.check.015`. Again none was removed, renumbered, reused or repurposed; each
rewritten ID still covers the same harness attribute inventory, the same
comparison branch, the same constant-evaluation surface, the same `trace`
function and the same check-pass warning channel it covered before, stated
correctly.

| Area | IDs allocated | Range |
|---|---:|---|
| `codegen.inputs.*` | 6 | `001`-`006` |
| `codegen.types.*` | 9 | `001`-`009` |
| `codegen.choice.*` | 4 | `001`-`004` |
| `codegen.cardinality.*` | 5 | `001`-`005` |
| `codegen.binding.*` | 4 | `001`-`004` |
| `codegen.naming.*` | 5 | `001`-`005` |
| `codegen.searchparam.*` | 6 | `001`-`006` |
| `codegen.output.*` | 6 | `001`-`006` |
| `codegen.roundtrip.*` | 4 | `001`-`004` |
| `model.bytes.*` | 6 | `001`-`006` |
| `model.lazy.*` | 6 | `001`-`006` |
| `model.json.*` | 25 | `001`-`025` |
| `model.primext.*` | 9 | `001`-`009` |
| `model.canonical.*` | 7 | `001`-`007` |
| `model.mutate.*` | 7 | `001`-`007` |
| `model.nav.*` | 12 | `001`-`012` |
| `model.errors.*` | 5 | `001`-`005` |
| `model.perf.*` | 6 | `001`-`006` |
| `path.lexer.*` | 16 | `001`-`016` |
| `path.literal.*` | 15 | `001`-`015` |
| `path.parse.*` | 8 | `001`-`008` |
| `path.navigation.*` | 24 | `001`-`024` |
| `path.collection.*` | 15 | `001`-`015` |
| `path.filter.*` | 9 | `001`-`009` |
| `path.subset.*` | 13 | `001`-`013` |
| `path.combine.*` | 6 | `001`-`006` |
| `path.equality.*` | 12 | `001`-`012` |
| `path.compare.*` | 12 | `001`-`012` |
| `path.logic.*` | 10 | `001`-`010` |
| `path.math.*` | 14 | `001`-`014` |
| `path.mathfn.*` | 11 | `001`-`011` |
| `path.string.*` | 24 | `001`-`024` |
| `path.convert.*` | 14 | `001`-`014` |
| `path.datetime.*` | 7 | `001`-`007` |
| `path.quantity.*` | 18 | `001`-`018` |
| `path.boundary.*` | 10 | `001`-`010` |
| `path.types.*` | 13 | `001`-`013` |
| `path.env.*` | 17 | `001`-`017` |
| `path.utility.*` | 28 | `001`-`028` |
| `path.check.*` | 19 | `001`-`019` |
| `path.strictness.*` | 10 | `001`-`010` |
| `path.errors.*` | 5 | `001`-`005` |
| `path.api.*` | 8 | `001`-`008` |
| `path.oracle.*` | 17 | `001`-`017` |
| `path.perf.*` | 6 | `001`-`006` |

## Oracles by scenario group

| Scenario group | Oracle | Case selector |
|---|---|---|
| `path.*` except `path.perf.*` | `oracles/fhirpath` — `tests-fhir-r4.xml`, pinned `c95adbee099c875de97d25ed7616aae7d66dd85b` | 1,191 `<test>` elements in 103 `<group>`s, addressed `<group name>/<test name>`; the gate is pass rate at or above HAPI 8.12.0's measured rate on the same cases |
| `path.check.*` except `015`-`019` | same suite | the 29 `invalid="semantic"` cases plus the `isSemanticChecks` group |
| `path.check.015`-`019` | `path-check-warnings` — an in-process unit oracle over the generated R4 model | the nine check-pass warning message ids, asserted per expression: choice-no-type-specifier, choice-spurious-type-specifier, collection-status-operation-left, collection-status-operation-right, collection-status-parameter, collection-status-context, not-a-collection, unknown-extension, of-type-impossible. The suite selector above grades only whether the check **fails**, and a live HAPI server over HTTP never exposes the warning list, so neither can reach this channel |
| `model.json.024`, `model.json.025`, `model.canonical.007` | `model-parser-errors` — an in-process unit oracle over hand-built documents | one document per condition, asserted in both parse modes; the lenient diagnostic set and the strict error, by HAPI message code. The `r4-example-roundtrip` selector cannot reach these because no example in `hl7.fhir.r4.examples#4.0.1` carries a contained resource without an `id` or a dangling `#` reference |
| `path.errors.*` | same suite | the 27 `invalid="execution"` cases |
| `path.lexer.*`, `path.parse.*` | same suite | the 7 `invalid="syntax"` cases, plus the `comments` group |
| `model.*`, `codegen.roundtrip.*` | `r4-example-roundtrip` | every example in `hl7.fhir.r4.examples#4.0.1`, canonicalised both sides; interim selector is the 103 examples in `oracles/fhir-test-cases/fetched/r4/examples/` |
| `codegen.*` other than `roundtrip` | the generator's own output | regeneration determinism and input-inventory assertions; no external suite |
| `*.perf.*` | `bench/micro/` | the named workloads in [`design.md`](design.md) |

**The pinned suite is 1,191 cases in 103 groups, counted by XML parse.** A text
match gives 104 and 1,193 because the `testDollarResource` group at
`tests-fhir-r4.xml:1576-1582` sits inside an XML comment whose bodies still
contain `<test ...>` text — and are leftover Java, not well-formed test
elements. `documentFunctions` is 115 cases, not 114. The `invalid` counts are
unaffected: 7 syntax, 29 semantic, 27 execution.

**Nineteen scenarios have no live case in the pinned suite to grade them and
say so in their own text**: `path.subset.003`, `path.subset.013`,
`path.compare.007`, `path.compare.011`, `path.convert.012`,
`path.datetime.003`, `path.quantity.015`, `path.quantity.018`, `path.env.015`,
`path.env.016`, `path.env.017`, `path.utility.017`, `path.utility.022`,
`path.utility.023`, `path.utility.024`, `path.utility.027`,
`path.utility.028`, `path.strictness.005` and `path.strictness.006`. The pinned
suite exercises none of `resolve()`, `memberOf`, `defineVariable`,
`%rootResource`, `%context`, `modifierExtension` or `toDate`, installs no host
service and passes no application context, and no case sets
`allowPolymorphicNames`, `allowDoubleQuotes`, `doImplicitStringConversion` or
`emitSQLonFHIRWarning`, so those scenarios are graded by the differential
harness against live HAPI 8.12.0. A scenario that names no oracle at all would
be ungradeable; "differential-only" is an oracle, and naming it is what stops a
behaviour from quietly going unchecked.

Five further scenarios — `path.check.015` through `path.check.019` — are graded
by neither the suite nor the differential harness but by the in-process
`path-check-warnings` oracle named above, and three — `model.json.024`,
`model.json.025` and `model.canonical.007` — by the in-process
`model-parser-errors` oracle. Those two oracles exist precisely because the
behaviour they grade is invisible to both the vendored suite and an HTTP
differential.

Input files the FHIRPath suite references all live in
`oracles/fhir-test-cases/fetched/r4/` — the directory
`TestingUtilities.loadTestResource("r4", input)` resolves to in
`org.hl7.fhir.r4.test.FHIRPathTests`. The per-file counts are in
[`proposal.md`](proposal.md).

## Registry

Every allocated ID, grouped as it is grouped in the delta files.

### `codegen.*` — [`specs/codegen/spec.md`](specs/codegen/spec.md)

**Requirement: Pinned, verified generator inputs**

- `codegen.inputs.001` — generating from the pinned R4 package succeeds
- `codegen.inputs.002` — a digest mismatch refuses to generate
- `codegen.inputs.003` — a missing package refuses to generate
- `codegen.inputs.004` — the generator does not read `reference/`
- `codegen.inputs.005` — the pinned package and the HAPI copy agree
- `codegen.inputs.006` — the input inventory is reported and asserted

**Requirement: Generated datatypes**

- `codegen.types.001` — every primitive datatype is generated
- `codegen.types.002` — every complex datatype is generated
- `codegen.types.003` — metadata types are generated
- `codegen.types.004` — abstract types are generated as shared element sets, not instantiable resources

**Requirement: Generated resources**

- `codegen.types.005` — all 146 concrete resource types are generated
- `codegen.types.006` — backbone elements become named nested types
- `codegen.types.007` — deeply nested backbone elements are generated at every level
- `codegen.types.008` — recursive backbone elements terminate
- `codegen.types.009` — type cycles across datatypes terminate

**Requirement: Choice elements**

- `codegen.choice.001` — a choice element is generated under its base name
- `codegen.choice.002` — every declared choice type is representable
- `codegen.choice.003` — a choice element with a single permitted type is still a choice
- `codegen.choice.004` — two values for the same choice element are rejected on decode

**Requirement: Cardinality**

- `codegen.cardinality.001` — `0..1` is optional and single
- `codegen.cardinality.002` — `1..1` is required and single, and its absence is a diagnostic rather than a rejection
- `codegen.cardinality.003` — `0..*` is an ordered list
- `codegen.cardinality.005` — a required element present but empty is treated as absent
- `codegen.cardinality.004` — `1..*` is an ordered list whose minimum is recorded

**Requirement: Value-set bindings**

- `codegen.binding.001` — a required binding generates a closed enumeration
- `codegen.binding.002` — a code outside a required binding is preserved, not dropped
- `codegen.binding.003` — extensible, preferred and example bindings generate no enumeration
- `codegen.binding.004` — a required binding whose value set is not in the input bundle generates no enumeration

**Requirement: Rust naming**

- `codegen.naming.001` — element names become snake_case fields
- `codegen.naming.002` — a FHIR name that is a Rust keyword is escaped deterministically
- `codegen.naming.003` — two FHIR names that map to the same Rust name are disambiguated and reported
- `codegen.naming.004` — resource and type names are used unchanged
- `codegen.naming.005` — enumeration variants are derived from codes and the code is recoverable

**Requirement: Generated search-parameter tables**

- `codegen.searchparam.001` — every R4 SearchParameter is in the table
- `codegen.searchparam.002` — a parameter with several bases appears once per base
- `codegen.searchparam.003` — composite parameters carry their components
- `codegen.searchparam.004` — reference parameters carry their targets
- `codegen.searchparam.005` — every table expression parses
- `codegen.searchparam.006` — parameter modifiers and flags are carried

**Requirement: Generator output discipline**

- `codegen.output.001` — regeneration is byte-identical
- `codegen.output.002` — regeneration is byte-identical across platforms
- `codegen.output.003` — every output file carries the `@generated` marker and its inputs
- `codegen.output.004` — generated code passes the lint and format gates
- `codegen.output.005` — a drift check fails when checked-in output does not match a fresh run
- `codegen.output.006` — generated code contains no `unsafe`

**Requirement: Generated model and codec completeness**

- `codegen.roundtrip.001` — every R4 specification example round-trips byte-identically
- `codegen.roundtrip.002` — the round-trip covers every resource type that has an example
- `codegen.roundtrip.003` — round-tripping an XML example through the JSON codec is not claimed
- `codegen.roundtrip.004` — the element-order table is generated

### `model.*` — [`specs/model/spec.md`](specs/model/spec.md)

**Requirement: A read that returns a resource unchanged does not parse it**

- `model.bytes.001` — stored bytes are returned verbatim
- `model.bytes.002` — a pass-through performs no JSON parse
- `model.bytes.003` — reading one element does not force a full parse
- `model.bytes.004` — reading an element does not disturb pass-through
- `model.bytes.005` — a resource that was never bytes serialises from its element order
- `model.bytes.006` — the byte-backed and constructed forms are indistinguishable to a reader

**Requirement: Lazy materialisation**

- `model.lazy.001` — an unread sibling is never materialised
- `model.lazy.002` — a repeating element can be indexed without materialising the rest
- `model.lazy.003` — a count does not materialise the values
- `model.lazy.004` — materialisation is idempotent and cached
- `model.lazy.005` — a contained resource is materialised only when entered
- `model.lazy.006` — narrative is not materialised by an unrelated read

**Requirement: FHIR JSON decoding**

- `model.json.001` — the resource type comes from `resourceType`
- `model.json.002` — a missing `resourceType` is an error
- `model.json.003` — an unknown `resourceType` is an error naming the value
- `model.json.004` — an unknown property is accepted in lenient mode and reported as a diagnostic
- `model.json.005` — a choice element decodes from its typed property name
- `model.json.006` — contained resources decode as resources
- `model.json.007` — a Bundle's entry resources decode as their own types
- `model.json.008` — `text.div` is carried as an opaque XHTML string
- `model.json.009` — a decimal's lexical form is preserved
- `model.json.010` — a decimal is never converted through a binary float
- `model.json.011` — `integer` bounds are enforced
- `model.json.012` — `positiveInt` and `unsignedInt` bounds are enforced
- `model.json.013` — a JSON type mismatch is accepted in lenient mode and reported as a diagnostic
- `model.json.014` — a repeating element given a single object is accepted in lenient mode
- `model.json.015` — a single element given an array is accepted in lenient mode, keeping the first value
- `model.json.016` — a duplicate JSON key takes the last occurrence in lenient mode
- `model.json.017` — `base64Binary` is carried as its encoded text
- `model.json.018` — `null` in a non-primitive array yields a value-less element in lenient mode
- `model.json.019` — decoding is encoding-strict
- `model.json.021` — the parse mode is selectable and lenient is the default
- `model.json.022` — an extension carrying both a value and nested extensions is fatal in both modes
- `model.json.023` — an invalid primitive value is fatal in both modes
- `model.json.024` — a contained resource with no `id` survives a lenient decode but cannot be referenced
- `model.json.025` — a `#` reference with no matching contained resource is a diagnostic, not a resolution
- `model.json.020` — element `id` and `extension` decode on any element

**Requirement: Primitive extensions — the `_field` form**

- `model.primext.001` — a singleton primitive's `_field` sibling is readable
- `model.primext.002` — a repeating primitive's `_field` array aligns by index
- `model.primext.003` — a `null` hole in the value array means the element has extensions but no value
- `model.primext.004` — `hasValue` is false for an extension-only primitive
- `model.primext.005` — a `_field` with no value sibling at all is still an element
- `model.primext.006` — `_field` arrays of unequal length are reported
- `model.primext.007` — a `_field` sibling round-trips in place
- `model.primext.008` — reading a primitive's value does not materialise its extensions
- `model.primext.009` — `_field` is rejected on a non-primitive element

**Requirement: Canonical serialisation**

- `model.canonical.001` — canonical form sorts property names and emits no insignificant whitespace
- `model.canonical.002` — canonical form preserves array order
- `model.canonical.003` — canonical form preserves a decimal's presented form
- `model.canonical.004` — canonical form is idempotent
- `model.canonical.005` — canonicalising a pretty-printed example yields the same bytes as canonicalising the round-trip
- `model.canonical.006` — canonical and pretty forms differ only in whitespace and key order
- `model.canonical.007` — encoding a `#` reference with no matching contained resource is diagnosed on the encode side

**Requirement: Copy-on-write mutation with byte splicing**

- `model.mutate.001` — changing one element preserves the bytes of untouched subtrees
- `model.mutate.002` — adding a repeating element keeps the existing entries' bytes
- `model.mutate.003` — removing an element removes only its bytes
- `model.mutate.004` — a mutated resource no longer claims byte-identity
- `model.mutate.005` — setting an element to the value it already has still marks the resource modified
- `model.mutate.006` — a mutation inside a contained resource splices at that resource's boundary
- `model.mutate.007` — `Meta` updates are the common case and are cheap

**Requirement: The navigation surface `frevo-path` evaluates over**

- `model.nav.001` — a node reports its FHIR type name
- `model.nav.002` — children are returned by element name in document order
- `model.nav.003` — a choice element is reached by its base name
- `model.nav.004` — the typed property name yields no children
- `model.nav.005` — an absent element yields an empty child list, not an error
- `model.nav.006` — all children can be enumerated with their names
- `model.nav.007` — returned values are owned and outlive the node they came from
- `model.nav.008` — a node reports whether it is a resource
- `model.nav.009` — a primitive node reports its value, or reports having none
- `model.nav.010` — the model answers type-hierarchy questions
- `model.nav.011` — the model answers element-type questions for the static check
- `model.nav.012` — navigation works over a node that is not `frevo-model`

**Requirement: Error reporting**

- `model.errors.001` — every decode error carries an element path
- `model.errors.002` — every decode error carries a byte offset
- `model.errors.003` — a strict decode reports the first error; a lenient decode reports every diagnostic
- `model.errors.004` — truncated input is an error, not a panic
- `model.errors.005` — adversarially nested input is bounded

**Requirement: Model performance budgets**

- `model.perf.001` — pass-through is a byte copy
- `model.perf.002` — a single-element read is far cheaper than a full parse
- `model.perf.003` — full materialisation stays within budget
- `model.perf.004` — a metadata update is proportional to `meta`, not to the document
- `model.perf.005` — canonical serialisation of a constructed resource stays within budget
- `model.perf.006` — no regression against the recorded baseline

### `path.*` — [`specs/path/spec.md`](specs/path/spec.md)

**Requirement: Lexing**

- `path.lexer.001` — a `//` comment runs to end of line
- `path.lexer.002` — several `//` comment lines are ignored
- `path.lexer.003` — a block comment after the expression is ignored
- `path.lexer.004` — text after a closing block comment is part of the expression
- `path.lexer.005` — a leading block comment is ignored
- `path.lexer.006` — a comment can separate an operator from its operand
- `path.lexer.007` — an inline block comment is ignored mid-expression
- `path.lexer.008` — a trailing operator is a syntax error
- `path.lexer.009` — an unterminated block comment is a syntax error
- `path.lexer.010` — a backtick-delimited identifier is accepted
- `path.lexer.011` — string escape sequences are decoded
- `path.lexer.015` — a double-quoted string is a syntax error by default
- `path.lexer.016` — double quotes are accepted when the engine is told to allow them
- `path.lexer.012` — an unknown escape sequence is a syntax error
- `path.lexer.013` — a unicode escape with fewer than four hex digits is a syntax error
- `path.lexer.014` — an unterminated string literal is a syntax error

**Requirement: Literals**

- `path.literal.001` — `true` and `false` are boolean literals
- `path.literal.002` — a single-quoted string is a string literal
- `path.literal.003` — an integer literal is an integer
- `path.literal.004` — a negative integer needs parentheses before a function call
- `path.literal.005` — an unparenthesised negative integer with a function call is an execution error
- `path.literal.006` — a decimal literal keeps its precision
- `path.literal.007` — date literals are accepted at each precision
- `path.literal.008` — a year-precision date reports precision 4
- `path.literal.009` — dateTime literals are accepted at each precision including offsets
- `path.literal.010` — a millisecond dateTime reports precision 17
- `path.literal.011` — time literals are accepted at each precision
- `path.literal.012` — a quantity literal with a UCUM unit is a Quantity
- `path.literal.013` — calendar-duration keywords are quantity literals
- `path.literal.014` — an empty collection literal is empty
- `path.literal.015` — an integer literal beyond the 32-bit range behaves as the pinned engine does

**Requirement: Parsing and operator precedence**

- `path.parse.001` — an expression parses once and evaluates many times
- `path.parse.002` — precedence follows the engine's table
- `path.parse.003` — parentheses override precedence
- `path.parse.004` — an unbalanced parenthesis is a syntax error
- `path.parse.005` — an unknown function name is a syntax error at parse time
- `path.parse.006` — the wrong number of arguments is reported at parse time
- `path.parse.007` — a parse error names the offset
- `path.parse.008` — the seven syntax-invalid oracle cases all fail to parse

**Requirement: Navigation**

- `path.navigation.001` — a leading resource type name matches the focus resource
- `path.navigation.002` — the resource type name may be omitted
- `path.navigation.003` — a leading type name that is not the focus type yields empty
- `path.navigation.004` — navigation flattens across a repeating element
- `path.navigation.005` — an absent element yields empty, not an error
- `path.navigation.006` — a polymorphic element is navigated by its base name
- `path.navigation.007` — a typed polymorphic property name is a semantic error
- `path.navigation.008` — the indexer selects by zero-based position
- `path.navigation.009` — an out-of-range index yields empty
- `path.navigation.010` — a negative index yields empty
- `path.navigation.011` — a non-integer index is an execution error
- `path.navigation.012` — a contained resource's elements are navigable
- `path.navigation.013` — `Resource.id` yields the logical id, not the full URL
- `path.navigation.014` — navigation into a primitive's extension works
- `path.navigation.015` — `$this` is the current item
- `path.navigation.016` — `$this` with no match yields empty
- `path.navigation.017` — `$index` is the current item's position
- `path.navigation.018` — `$total` is only defined inside `aggregate`
- `path.navigation.019` — navigation over an empty focus yields empty
- `path.navigation.020` — `children()` returns the immediate children
- `path.navigation.021` — `descendants()` returns the transitive children
- `path.navigation.022` — `children()` produces an unordered collection
- `path.navigation.023` — navigation preserves document order across repeating elements
- `path.navigation.024` — the miscellaneous accessor cases all hold

**Requirement: Collection predicates**

- `path.collection.001` — `empty()` is true for an empty collection
- `path.collection.002` — `exists()` is true for a non-empty collection
- `path.collection.003` — `exists(criteria)` filters before testing
- `path.collection.004` — `not()` inverts a singleton boolean
- `path.collection.005` — `not()` on an empty collection yields empty
- `path.collection.006` — `all(criteria)` is true when every item satisfies it
- `path.collection.007` — `all(criteria)` is true for an empty collection
- `path.collection.008` — the boolean folds reduce a boolean collection
- `path.collection.009` — a boolean fold over a non-boolean item is an execution error
- `path.collection.010` — `subsetOf` compares by equality
- `path.collection.011` — `supersetOf` compares by equality
- `path.collection.012` — `isDistinct` detects duplicates
- `path.collection.013` — `distinct` removes duplicates and keeps first-occurrence order
- `path.collection.014` — `count` returns the number of items
- `path.collection.015` — `count` of an empty collection is zero

**Requirement: Filtering and projection**

- `path.filter.001` — `where` keeps items whose criteria is true
- `path.filter.002` — `where` drops items whose criteria is empty
- `path.filter.003` — `where` with a non-boolean criteria is an execution error
- `path.filter.004` — `select` projects each item and flattens the result
- `path.filter.005` — `select` returning empty for an item contributes nothing
- `path.filter.006` — `repeat` applies the projection transitively until nothing new appears
- `path.filter.007` — `repeat` terminates on a cyclic projection
- `path.filter.008` — `ofType` keeps only items of the named type
- `path.filter.009` — `ofType` accepts a namespace-qualified type name

**Requirement: Subsetting**

- `path.subset.001` — `single` returns the only item
- `path.subset.002` — `single` on a collection of more than one is an execution error
- `path.subset.003` — `single` on an empty collection is an execution error
- `path.subset.004` — `first` and `last` select the ends
- `path.subset.005` — `tail` drops the first item
- `path.subset.006` — `skip` drops the first n items
- `path.subset.007` — `skip` past the end yields empty
- `path.subset.008` — `take` keeps the first n items
- `path.subset.009` — `take(0)` and `take(-1)` yield empty
- `path.subset.010` — `intersect` keeps items present in both, without duplicates
- `path.subset.011` — `exclude` removes items present in the argument
- `path.subset.013` — an unordered focus is rejected for all six position-dependent functions
- `path.subset.012` — `exclude` keeps duplicates that are not excluded

**Requirement: Combining**

- `path.combine.001` — the union operator unions and removes duplicates
- `path.combine.002` — `union()` behaves as the union operator
- `path.combine.003` — `combine` concatenates and keeps duplicates
- `path.combine.004` — `in` tests membership
- `path.combine.005` — `contains` tests membership the other way round
- `path.combine.006` — `in` with an empty left operand yields empty

**Requirement: Equality and equivalence**

- `path.equality.001` — the equality operator compares values of the same type
- `path.equality.002` — the inequality operator is the negation of equality
- `path.equality.003` — equality with an empty operand yields empty
- `path.equality.004` — the equivalence operator ignores case and whitespace for strings
- `path.equality.005` — the non-equivalence operator is the negation of equivalence
- `path.equality.006` — equivalence with an empty operand yields a boolean, never empty
- `path.equality.007` — equality on collections compares item by item in order
- `path.equality.008` — equality on decimals compares by value, not by lexical form
- `path.equality.009` — equality on complex elements compares every child
- `path.equality.010` — equality on dates of differing precision yields empty where undecidable
- `path.equality.011` — equivalence on dates of differing precision compares at the lower precision
- `path.equality.012` — `hasValue()` distinguishes a primitive with a value from one without

**Requirement: Ordered comparison**

- `path.compare.001` — the less-than operator accepts a narrower numeric type set than the other three
- `path.compare.002` — the less-or-equal operator accepts the extended numeric type set
- `path.compare.003` — the greater-than operator accepts the extended numeric type set
- `path.compare.004` — the greater-or-equal operator accepts the extended numeric type set
- `path.compare.005` — comparing different types is an execution error
- `path.compare.006` — comparison with an empty operand yields empty
- `path.compare.007` — comparison of a collection of more than one yields empty
- `path.compare.008` — strings compare by code point
- `path.compare.009` — dates of insufficient precision compare to empty
- `path.compare.011` — a `positiveInt` or `unsignedInt` operand compares under three operators and fails under `<`
- `path.compare.012` — numeric comparison goes through binary floating point
- `path.compare.010` — a Period invariant written with raw comparison and one written with boundaries differ

**Requirement: Boolean logic**

- `path.logic.001` — `and` follows the engine's truth table including empty
- `path.logic.002` — `or` follows the engine's truth table including empty
- `path.logic.003` — `xor` follows the engine's truth table including empty
- `path.logic.004` — `implies` follows the engine's truth table including empty
- `path.logic.005` — a singleton non-boolean operand of `and` is treated as true
- `path.logic.006` — `and` short-circuits on a false left operand
- `path.logic.007` — `or` and `implies` short-circuit, and `xor` does not
- `path.logic.008` — `iif` selects a branch and does not evaluate the other
- `path.logic.009` — `iif` with an empty condition yields the else branch
- `path.logic.010` — `iif` with no else branch and a false condition yields empty

**Requirement: Arithmetic**

- `path.math.001` — addition works on integers, decimals, strings and quantities
- `path.math.002` — the concatenation operator joins strings and treats empty as the empty string
- `path.math.003` — subtraction works on integers, decimals and quantities
- `path.math.004` — multiplication works on integers, decimals and quantities
- `path.math.005` — division always yields a decimal
- `path.math.006` — `div` truncates toward zero
- `path.math.007` — `mod` returns the remainder
- `path.math.008` — division by zero yields empty
- `path.math.009` — arithmetic with an empty operand yields empty
- `path.math.010` — arithmetic on incompatible types is an execution error
- `path.math.011` — integer arithmetic that overflows behaves as the pinned engine does
- `path.math.012` — date arithmetic adds and subtracts calendar durations
- `path.math.013` — date arithmetic preserves the operand's precision
- `path.math.014` — subtracting two dateTimes is not addition of a duration

**Requirement: Math functions**

- `path.mathfn.001` — `abs` works on integers, decimals and quantities
- `path.mathfn.002` — `ceiling` rounds up
- `path.mathfn.003` — `floor` rounds down
- `path.mathfn.004` — `round` rounds to the requested number of digits
- `path.mathfn.005` — `sqrt` of a negative number yields empty
- `path.mathfn.006` — `exp` and `ln` are inverses within the engine's precision
- `path.mathfn.007` — `log` takes the base as its argument
- `path.mathfn.008` — `power` of a negative base with a fractional exponent yields empty
- `path.mathfn.009` — `truncate` discards the fractional part
- `path.mathfn.010` — a math function on an empty collection yields empty
- `path.mathfn.011` — a math function on a collection of more than one is an execution error

**Requirement: String functions**

- `path.string.001` — `upper` and `lower` change case
- `path.string.002` — `toChars` splits a string into single characters
- `path.string.003` — `indexOf` returns a zero-based position or -1
- `path.string.004` — `substring` takes a start and an optional length
- `path.string.005` — `substring` with an out-of-range start yields empty
- `path.string.006` — `startsWith` and `endsWith` test affixes
- `path.string.007` — `contains` tests substring presence
- `path.string.008` — `matches` tests a regular expression unanchored
- `path.string.009` — `matchesFull` requires the whole string to match
- `path.string.010` — `replaceMatches` substitutes regular-expression matches
- `path.string.011` — `replace` substitutes literal substrings
- `path.string.012` — `length` counts characters
- `path.string.013` — `trim` removes leading and trailing whitespace
- `path.string.014` — `split` keeps empty segments
- `path.string.015` — `join` concatenates with a separator and yields empty for an empty collection
- `path.string.016` — `encode` supports base64, urlbase64 and hex
- `path.string.017` — `decode` reverses `encode` for each supported form
- `path.string.018` — `encode` and `decode` with an unsupported format yield empty or fail as the pinned engine does
- `path.string.019` — `escape` and `unescape` support `html` and `json`
- `path.string.020` — a string function on a non-string yields empty
- `path.string.021` — a string function on an empty collection yields empty
- `path.string.023` — the functions that do reject a bad focus reject it with an error
- `path.string.024` — implicit string conversion changes the outcome of every string function
- `path.string.022` — string functions operate on unicode scalar values, not bytes

**Requirement: Type conversion**

- `path.convert.001` — `toInteger` converts strings, booleans and integers
- `path.convert.002` — `toDecimal` converts strings, booleans, integers and decimals
- `path.convert.003` — `toString` converts every convertible type
- `path.convert.004` — `convertsToDate` accepts year, month and day precision strings
- `path.convert.005` — `convertsToDateTime` accepts every precision and offset form
- `path.convert.006` — `convertsToTime` accepts hour, minute, second and millisecond precision
- `path.convert.007` — `convertsToBoolean` accepts the engine's accepted spellings
- `path.convert.008` — `convertsToQuantity` and `toQuantity` convert strings and numbers
- `path.convert.009` — a failed conversion yields empty while its `convertsTo` yields false
- `path.convert.010` — a conversion function on a collection of more than one is an execution error
- `path.convert.012` — `toDate()` does not exist and is a parse failure
- `path.convert.013` — `toTime` converts and yields empty on failure
- `path.convert.014` — `toBoolean` converts the engine's accepted spellings and yields empty on failure
- `path.convert.011` — `toDateTime` on a date widens precision without inventing components

**Requirement: Date and time functions**

- `path.datetime.001` — `today()` returns the current date
- `path.datetime.002` — `now()` returns the current dateTime
- `path.datetime.003` — `now()` is read afresh on every call
- `path.datetime.004` — `today()` and `now()` read a configured time zone, not the machine's
- `path.datetime.005` — `precision()` reports the component count of a date, dateTime or time
- `path.datetime.007` — `today()` has day precision
- `path.datetime.006` — a dateTime with an offset and one in UTC compare correctly

**Requirement: Quantities and UCUM**

- `path.quantity.001` — quantities in commensurable UCUM units compare equal after conversion
- `path.quantity.002` — equivalence of quantities rounds to the lower precision
- `path.quantity.003` — equality of quantities does not round
- `path.quantity.004` — calendar durations and UCUM units are commensurable where the engine says so
- `path.quantity.005` — quantities in commensurable units compare by magnitude
- `path.quantity.006` — multiplying quantities multiplies their units
- `path.quantity.007` — dividing quantities divides their units
- `path.quantity.008` — dividing a unit by itself yields the dimensionless unit
- `path.quantity.009` — two unitless quantities compare equal and equivalent
- `path.quantity.010` — two quantities with the same unit text and no code compare equal
- `path.quantity.011` — a quantity with a unit compared to one without yields empty for equality
- `path.quantity.012` — a quantity with a unit compared to one without yields false for equivalence
- `path.quantity.013` — unit text without a code is handled per the suite's dedicated input
- `path.quantity.014` — `comparable()` reports whether two quantities are commensurable
- `path.quantity.015` — comparing incommensurable units returns a boolean, not an error
- `path.quantity.016` — UCUM data comes from the pinned `ucum-essence.xml`
- `path.quantity.018` — a quantity whose system is not UCUM is not canonicalised
- `path.quantity.017` — a quantity with an unknown unit code does not crash

**Requirement: Boundary functions**

- `path.boundary.001` — `lowBoundary` on a decimal widens to the implied precision
- `path.boundary.002` — `lowBoundary(n)` with an out-of-range precision yields empty
- `path.boundary.003` — `lowBoundary` on a negative decimal moves away from zero
- `path.boundary.004` — `lowBoundary` carries and borrows across digit positions
- `path.boundary.005` — `lowBoundary` reads precision from a decimal's stored lexical form
- `path.boundary.006` — `highBoundary` on a decimal widens to the implied precision
- `path.boundary.007` — `highBoundary` carries across digit positions
- `path.boundary.008` — boundary functions work on quantities
- `path.boundary.009` — boundary functions work on dates, dateTimes and times
- `path.boundary.010` — an integer's boundaries are decimals half a unit away

**Requirement: Type operators and reflection**

- `path.types.001` — `is` tests the System namespace for a literal
- `path.types.002` — `is` tests the FHIR namespace for a model element
- `path.types.003` — `is` on a resource tests the resource type
- `path.types.004` — `is` follows the FHIR type hierarchy
- `path.types.005` — `is` distinguishes `uri` from `url`
- `path.types.006` — `is` on an extension's value resolves the concrete choice type
- `path.types.007` — `as` yields the value when the type matches and empty when it does not
- `path.types.008` — `as` on a `string`-typed element does not match `code`
- `path.types.009` — `type()` reports a namespace and a name
- `path.types.010` — the `is` operator and the `is()` function agree
- `path.types.011` — `is` with an unknown type name is a semantic error
- `path.types.012` — `is` on an empty collection yields empty
- `path.types.013` — the whole `testTypes`, `testType` and `testInheritance` surface holds

**Requirement: Environment variables**

- `path.env.001` — `%sct`, `%loinc` and `%ucum` are the canonical system URLs
- `path.env.002` — a value-set shorthand expands to an HL7 ValueSet URL
- `path.env.003` — a code-system shorthand expands to an HL7 base URL
- `path.env.004` — an extension shorthand expands to an HL7 StructureDefinition URL
- `path.env.005` — `%us-zip` is the engine's built-in ZIP pattern
- `path.env.006` — `%resource` is the resource the focus is inside
- `path.env.007` — `%resource` with no focus resource is an execution error
- `path.env.008` — `%rootResource` is the outermost resource
- `path.env.009` — `%rootResource` with no root resource is an execution error
- `path.env.010` — `%context` is the node evaluation started from
- `path.env.011` — an unknown environment variable with no host service is an execution error
- `path.env.012` — a host service may supply an environment variable, including a collection
- `path.env.015` — the host service is told which constant-evaluation mode is in force
- `path.env.016` — a host constant can shadow a real property
- `path.env.017` — the implicit-after mode is a fall-through, and both implicit modes need an application context
- `path.env.013` — `defineVariable` introduces a variable for the rest of the expression
- `path.env.014` — a variable defined by `defineVariable` shadows nothing and does not leak

**Requirement: Utility and FHIR-specific functions**

- `path.utility.001` — `trace` passes its input through and records a log line
- `path.utility.002` — `trace` with a projection logs the projection, not the input
- `path.utility.003` — `check` passes its input through when the condition holds
- `path.utility.004` — `check` fails when the condition does not hold
- `path.utility.005` — `aggregate` folds with `$this` and `$total`
- `path.utility.006` — `aggregate` without an initial value starts with an empty `$total`
- `path.utility.007` — `sort` orders a collection, with no argument and with a key
- `path.utility.008` — `sort` accepts several keys and a descending key
- `path.utility.009` — `extension(url)` selects extensions by URL
- `path.utility.010` — `extension(url)` with no match yields empty
- `path.utility.011` — `extension` reaches a primitive's extensions
- `path.utility.012` — `conformsTo` asks the host service
- `path.utility.013` — `conformsTo` with a profile the host service does not know is an execution error
- `path.utility.014` — `conformsTo` through the server-facing host service returns false for every URL
- `path.utility.015` — `memberOf` asks the host service for the value set
- `path.utility.016` — `memberOf` with no resolvable value set yields empty
- `path.utility.017` — `resolve()` resolves a local `#id` reference against the root resource's `contained`
- `path.utility.018` — `resolve()` has no Bundle `fullUrl` resolution of its own
- `path.utility.019` — `resolve()` on an unresolvable reference yields empty
- `path.utility.020` — `resolve()` delegates an external reference to the host service and swallows its errors
- `path.utility.022` — `conformsTo` with no host service at all is an execution error
- `path.utility.023` — `extension(url)` also returns modifier extensions
- `path.utility.024` — `memberOf` as a binary operator folds the whole left collection
- `path.utility.025` — `memberOf` falls back to the worker context when no host service is set
- `path.utility.026` — `memberOf` returns empty for a focus whose type it cannot validate
- `path.utility.027` — a host service may supply a custom function
- `path.utility.028` — a host service may intercept `trace` and suppress the engine's log
- `path.utility.021` — the appointment and explanation-of-benefit cases hold

**Requirement: The static type-check pass**

- `path.check.001` — the check pass is separate from evaluation
- `path.check.002` — the 29 semantic-invalid oracle cases all fail the check
- `path.check.003` — navigating a non-existent element is a semantic error
- `path.check.004` — the check infers the type of a choice element as the declared set
- `path.check.005` — the check narrows the type after an `is` guard in `implies`
- `path.check.006` — the same expression without the guard fails the check
- `path.check.007` — the `is()` function narrows the type as the `is` operator does
- `path.check.008` — `and` narrows the right operand assuming the left is true
- `path.check.009` — `or` narrows the right operand assuming the left is false
- `path.check.010` — `iif` narrows its branches by the condition's polarity
- `path.check.011` — a guard that narrows to the empty type set makes the guarded operand unreachable, not an error
- `path.check.012` — a context type naming several types checks against all of them
- `path.check.013` — the check pass can be skipped
- `path.check.015` — the check pass returns a warning list separate from its failure
- `path.check.016` — comparing a collection with a singleton produces a warning, not a failure
- `path.check.017` — navigating a choice element without `ofType()` produces a warning only when the SQL-on-FHIR check is enabled
- `path.check.018` — an unknown extension URL and an impossible `ofType` produce warnings
- `path.check.019` — the simple check surface discards warnings
- `path.check.014` — the harness records which cases the check pass is exercised for

**Requirement: Strictness configuration**

- `path.strictness.001` — the default configuration is non-strict for every pre-R5 version
- `path.strictness.002` — the oracle harness runs the check pass non-strict and the evaluation strict
- `path.strictness.003` — the as-singleton rule rejects a multi-item focus when enforced
- `path.strictness.004` — the as-singleton rule allows a multi-item focus when not enforced
- `path.strictness.005` — a mis-cased type name resolves only when case is not enforced
- `path.strictness.006` — lenient polymorphic names are supported by the engine and left unset by HAPI
- `path.strictness.008` — the SQL-on-FHIR warning is off by default
- `path.strictness.009` — implicit string conversion is off by default
- `path.strictness.010` — liquid mode is out of scope and must be off
- `path.strictness.007` — legacy mode is not enabled for R4

**Requirement: Execution errors**

- `path.errors.001` — the 27 execution-invalid oracle cases all fail at evaluation
- `path.errors.002` — an error names the expression and the position
- `path.errors.003` — evaluation never panics
- `path.errors.004` — evaluation is bounded for an adversarial expression
- `path.errors.005` — an error from `frevo-model` is surfaced, not swallowed into empty

**Requirement: The public evaluation surface**

- `path.api.001` — an expression can be evaluated from its text
- `path.api.002` — a parsed expression can be evaluated repeatedly
- `path.api.003` — a first-match operation returns at most one value
- `path.api.004` — a first-match operation on no match returns nothing
- `path.api.005` — a result of an unexpected type is reported, not silently dropped
- `path.api.006` — evaluation is synchronous
- `path.api.007` — a host service is optional and replaceable
- `path.api.008` — `frevo-path` does not depend on `frevo-model`

**Requirement: The oracle harness and the gate**

- `path.oracle.001` — the harness parses every case in the suite
- `path.oracle.002` — the harness reads every attribute the case carries
- `path.oracle.017` — the expression-level `mode` attribute changes no outcome
- `path.oracle.003` — the harness loads each referenced input file
- `path.oracle.004` — the harness handles both XML and JSON inputs
- `path.oracle.005` — the harness evaluates in three stages and attributes the failure to the right one
- `path.oracle.006` — the harness applies `predicate="true"` as a boolean conversion
- `path.oracle.007` — the harness applies `ordered="false"` as an unordered comparison
- `path.oracle.008` — the harness compares count, and compares type and value only when the expectation states them
- `path.oracle.009` — the harness picks the Quantity comparison from the result item's type, not from the expectation
- `path.oracle.010` — the harness reports per-case status with four distinct values
- `path.oracle.011` — the harness records HAPI's result for the same case
- `path.oracle.012` — the gate is pass rate at or above HAPI's on the same cases
- `path.oracle.016` — a case HAPI structurally cannot pass is recorded as such, and Frevo is graded on its own run
- `path.oracle.013` — the harness can run the same cases through `helios-fhirpath`
- `path.oracle.014` — the `documentFunctions` group is reported as out of scope, not as passing
- `path.oracle.015` — the harness does not modify the suite

**Requirement: FHIRPath performance budgets**

- `path.perf.001` — compiling an expression stays within budget
- `path.perf.002` — evaluating a compiled expression does not re-parse it
- `path.perf.003` — extracting a resource's whole search-parameter set stays within budget
- `path.perf.004` — evaluation over the lazy model does not force full materialisation
- `path.perf.005` — the whole suite runs fast enough to be a per-pull-request gate
- `path.perf.006` — no regression against the recorded baseline

