# model spec delta

## ADDED Requirements

### Requirement: A read that returns a resource unchanged does not parse it

`frevo-model` SHALL return an unmodified resource as the bytes it was given, without parsing or re-serialising it.

This is constitution rule 6.1 and it is the reason `frevo-model` exists rather
than a `serde` derive. It is observable, so it is specified as behaviour and
measured, not asserted in a comment.

#### Scenario: [model.bytes.001] stored bytes are returned verbatim

- **WHEN** a resource is loaded from its stored canonical JSON bytes and
  serialised again with no element read and no element changed
- **THEN** the output byte sequence is identical to the input byte sequence

#### Scenario: [model.bytes.002] a pass-through performs no JSON parse

- **WHEN** a resource is loaded from stored bytes and serialised again without
  any element being read
- **THEN** the number of JSON tokens scanned is zero, as reported by the codec's
  instrumentation counter
- **AND** the number of heap allocations proportional to the document size is
  zero

#### Scenario: [model.bytes.003] reading one element does not force a full parse

- **WHEN** `Patient.birthDate` is read from a 4 KiB `Patient` document
- **THEN** the value is returned
- **AND** the codec's token counter reports fewer tokens scanned than a full
  parse of the same document reports

#### Scenario: [model.bytes.004] reading an element does not disturb pass-through

- **WHEN** `Patient.birthDate` is read and the resource is then serialised with
  nothing changed
- **THEN** the output byte sequence is identical to the input byte sequence

#### Scenario: [model.bytes.005] a resource that was never bytes serialises from its element order

- **WHEN** a `Patient` is constructed in memory rather than decoded
- **THEN** serialising it emits properties in the generated element order
  (`codegen.roundtrip.004`)

#### Scenario: [model.bytes.006] the byte-backed and constructed forms are indistinguishable to a reader

- **WHEN** the same logical resource is obtained once by decoding bytes and once
  by construction, and the same element is read from each
- **THEN** both reads return equal values, with the same type

### Requirement: Lazy materialisation

Parsing SHALL materialise only the elements a caller actually asks for
(constitution rule 6.2).

#### Scenario: [model.lazy.001] an unread sibling is never materialised

- **WHEN** `Bundle.entry[0].resource.id` is read from a 100-entry `Bundle`
- **THEN** the value is returned
- **AND** entries 1 through 99 report as not materialised

#### Scenario: [model.lazy.002] a repeating element can be indexed without materialising the rest

- **WHEN** `Patient.name[2].family` is read from a document with ten `name`
  entries
- **THEN** the value is returned and `name[0]`, `name[1]` and `name[3..9]`
  report as not materialised

#### Scenario: [model.lazy.003] a count does not materialise the values

- **WHEN** the number of `Patient.name` entries is requested
- **THEN** the count is correct and no `HumanName` is materialised

#### Scenario: [model.lazy.004] materialisation is idempotent and cached

- **WHEN** `Patient.birthDate` is read twice
- **THEN** both reads return equal values
- **AND** the second read scans zero additional JSON tokens

#### Scenario: [model.lazy.005] a contained resource is materialised only when entered

- **WHEN** `Patient.name.family` is read from a document whose `contained` array
  holds three resources
- **THEN** none of the three contained resources is materialised

#### Scenario: [model.lazy.006] narrative is not materialised by an unrelated read

- **WHEN** `Patient.gender` is read from a document with a 50 KiB
  `text.div`
- **THEN** `text` reports as not materialised and the XHTML is not validated

### Requirement: FHIR JSON decoding

`frevo-model` SHALL decode the R4 JSON representation as the parser the starter
actually uses does — `ca.uhn.fhir.parser.JsonParser` with the error handler
`FhirContext` installs by default — and SHALL offer the two parse modes HAPI
offers: lenient by default, strict on request. Which upstream file governs which
part is recorded in [`../../design.md`](../../design.md) section 3.

#### Scenario: [model.json.001] the resource type comes from `resourceType`

- **WHEN** a document whose first property is `"resourceType": "Observation"` is
  decoded without the caller stating a type
- **THEN** it decodes as `Observation`

#### Scenario: [model.json.002] a missing `resourceType` is an error

- **WHEN** a document has no `resourceType` property
- **THEN** decoding fails with an error saying `resourceType` is required

#### Scenario: [model.json.003] an unknown `resourceType` is an error naming the value

- **WHEN** `"resourceType": "Patinet"` is decoded
- **THEN** decoding fails and the error contains `Patinet`

#### Scenario: [model.json.004] an unknown property is accepted in lenient mode and reported as a diagnostic

- **WHEN** a `Patient` document carrying `"nmae": []` is decoded in lenient
  mode, which is the default
- **THEN** decoding **succeeds**, the property is dropped from the resource, and
  an unrecognised-property diagnostic naming path `Patient.nmae` is returned
  alongside the resource
- **AND** a server that stores this resource answers `2xx`, which is what HAPI
  8.12.0 does: `LenientErrorHandler.unknownElement` logs a warning and returns
- **WHEN** the same document is decoded in strict mode
- **THEN** decoding fails with an error naming path `Patient.nmae`

#### Scenario: [model.json.005] a choice element decodes from its typed property name

- **WHEN** an `Observation` carries `"valueQuantity": {"value": 6.3, "unit": "mmol/L"}`
- **THEN** `Observation.value` holds a `Quantity`
- **AND** its FHIR type name is `Quantity`

#### Scenario: [model.json.006] contained resources decode as resources

- **WHEN** `patient-container-example.json` is decoded
- **THEN** `Patient.contained` holds one resource whose `id` is `1`

#### Scenario: [model.json.007] a Bundle's entry resources decode as their own types

- **WHEN** a `Bundle` with entries of differing resource types is decoded
- **THEN** each `Bundle.entry.resource` decodes as the type its own
  `resourceType` names

#### Scenario: [model.json.008] `text.div` is carried as an opaque XHTML string

- **WHEN** a `Patient` with a `text.div` containing nested markup and entity
  references is decoded and re-encoded
- **THEN** the `div` string is byte-identical to the input

#### Scenario: [model.json.009] a decimal's lexical form is preserved

- **WHEN** `parameters-example-decimals.xml`'s JSON equivalent, carrying
  `1.58700`, `1.00e3` and `0.0014`, is decoded and re-encoded
- **THEN** each number is emitted with the same digits, the same trailing zeros
  and the same exponent form as the input

#### Scenario: [model.json.010] a decimal is never converted through a binary float

- **WHEN** `0.1`, `1e308` and a 40-digit decimal are decoded, read as values and
  re-encoded
- **THEN** each is emitted unchanged
- **AND** reading the value yields a decimal with the input's precision, not a
  rounded binary approximation

#### Scenario: [model.json.011] `integer` bounds are enforced

- **WHEN** an `integer` element carries `2147483648`
- **THEN** decoding reports the value as out of range for `integer`, naming the
  element path

#### Scenario: [model.json.012] `positiveInt` and `unsignedInt` bounds are enforced

- **WHEN** a `positiveInt` element carries `0`, or an `unsignedInt` element
  carries `-1`
- **THEN** decoding reports the value as out of range, naming the element path
  and the declared type

#### Scenario: [model.json.013] a JSON type mismatch is accepted in lenient mode and reported as a diagnostic

- **WHEN** `"active": "true"` is decoded for `Patient.active`, which is
  `boolean`, in lenient mode
- **THEN** decoding **succeeds** and returns a diagnostic naming
  `Patient.active`, the expected type `boolean` and the found JSON type
  `string`, because `LenientErrorHandler.incorrectJsonType` logs and returns
- **AND** the element's value is the scalar's string form, which is what HAPI's
  parser passes to `attributeValue` regardless of the JSON type
- **WHEN** the same document is decoded in strict mode
- **THEN** decoding fails with that same information as an error

#### Scenario: [model.json.014] a repeating element given a single object is accepted in lenient mode

- **WHEN** `"name": {"family": "Chalmers"}` is decoded for `Patient.name`, which
  is `0..*`, in lenient mode
- **THEN** decoding **succeeds** with one `name` entry and returns an
  incorrect-JSON-type diagnostic naming `Patient.name`
- **WHEN** the same document is decoded in strict mode
- **THEN** decoding fails, naming `Patient.name` and that an array was
  expected

#### Scenario: [model.json.015] a single element given an array is accepted in lenient mode, keeping the first value

- **WHEN** `"birthDate": ["1974-12-25", "1980-01-01"]` is decoded for
  `Patient.birthDate`, which is `0..1`, in lenient mode
- **THEN** decoding **succeeds**, `Patient.birthDate` holds `1974-12-25`, and an
  unexpected-repeating-element diagnostic naming `Patient.birthDate` is
  returned, because `LenientErrorHandler.unexpectedRepeatingElement` logs and
  returns and the parser keeps the first repetition
- **WHEN** the same document is decoded in strict mode
- **THEN** decoding fails, naming `Patient.birthDate`

#### Scenario: [model.json.016] a duplicate JSON key takes the last occurrence in lenient mode

- **WHEN** a document carries `"gender": "male"` and then `"gender": "female"`,
  decoded in lenient mode
- **THEN** decoding **succeeds** and `Patient.gender` is `female`, because HAPI
  reads JSON through a Jackson `ObjectMapper` configured without
  `FAIL_ON_READING_DUP_TREE_KEY` or `STRICT_DUPLICATE_DETECTION`, so a repeated
  field name silently overwrites
- **AND** a duplicate-property diagnostic naming the property is returned, which
  is more than HAPI reports; HAPI has no error-handler callback for this case at
  all, so there is nothing to match and reporting it costs nothing observable
- **WHEN** the same document is decoded in strict mode
- **THEN** decoding fails with an error naming the duplicated property

#### Scenario: [model.json.017] `base64Binary` is carried as its encoded text

- **WHEN** an `Attachment.data` value is decoded and re-encoded
- **THEN** the base64 text is byte-identical to the input, with the input's
  padding and without re-encoding

#### Scenario: [model.json.018] `null` in a non-primitive array yields a value-less element in lenient mode

- **WHEN** `"name": [null]` is decoded for `Patient.name` in lenient mode
- **THEN** decoding **succeeds** with one `name` entry that has no value and no
  children, because HAPI's parser has an explicit null branch that enters the
  element, parses any `_name` sibling at the same index, and ends it — it does
  not reject
- **AND** re-serialising the resource unchanged emits `"name": [null]` again
- **WHEN** the same document is decoded in strict mode
- **THEN** decoding fails, naming `Patient.name` and that `null` is meaningful
  only as a hole in a primitive array paired with `_name`

#### Scenario: [model.json.019] decoding is encoding-strict

- **WHEN** input bytes are not valid UTF-8
- **THEN** decoding fails with an error naming the byte offset

#### Scenario: [model.json.021] the parse mode is selectable and lenient is the default

- **WHEN** a decoder is created with no explicit mode
- **THEN** it is lenient, matching `FhirContext`'s default error handler, so a
  request body HAPI 8.12.0 accepts is accepted
- **WHEN** a decoder is created in strict mode
- **THEN** every diagnostic the lenient mode returns beside the resource becomes
  an error instead, and nothing else about decoding changes
- **AND** the diagnostics a lenient decode returns are the same set, with the
  same paths and offsets, as the errors a strict decode raises, so the two modes
  cannot drift apart

#### Scenario: [model.json.022] an extension carrying both a value and nested extensions is fatal in both modes

- **WHEN** an `Extension` that has both a `value[x]` and nested `extension`
  children is decoded in lenient mode
- **THEN** decoding **fails**, because this is one of only two conditions
  `LenientErrorHandler` treats as fatal by default
- **AND** a server asked to store that resource answers `400 Bad Request` with
  an `OperationOutcome`, rather than storing a resource that cannot later be
  read, updated or deleted
- **AND** the error names the extension's element path and its URL

#### Scenario: [model.json.023] an invalid primitive value is fatal in both modes

- **WHEN** a primitive element carries a value that does not match its type's
  value rule — `"birthDate": "not-a-date"`, say — and is decoded in lenient mode
- **THEN** decoding **fails**, because an invalid value is the other condition
  `LenientErrorHandler` treats as fatal by default, on the stated grounds that
  ignoring it loses data silently
- **AND** a blank value is **not** fatal: it is logged and parsing continues
- **AND** a server asked to store a resource with an invalid primitive value
  answers `400 Bad Request`

#### Scenario: [model.json.020] element `id` and `extension` decode on any element

- **WHEN** a `HumanName` carries `"id": "n1"` and an `extension` array
- **THEN** both are readable on that element and round-trip unchanged

### Requirement: Primitive extensions — the `_field` form

A primitive element's `id` and `extension` live in a sibling property named
`_` plus the element name. `frevo-model` SHALL represent this without forcing
either side to be materialised, and SHALL distinguish "no value" from "no
element".

#### Scenario: [model.primext.001] a singleton primitive's `_field` sibling is readable

- **WHEN** a document carries `"birthDate": "1974-12-25"` and
  `"_birthDate": {"extension": [...]}`
- **THEN** `Patient.birthDate`'s value is `1974-12-25` and its extensions are
  the ones in `_birthDate`

#### Scenario: [model.primext.002] a repeating primitive's `_field` array aligns by index

- **WHEN** `patient-name-extensions.json` is decoded, which carries a `given`
  array and a `_given` array
- **THEN** index `i` of `given` and index `i` of `_given` describe the same
  element

#### Scenario: [model.primext.003] a `null` hole in the value array means the element has extensions but no value

- **WHEN** `given` is `[null, "Peter"]` and `_given` is `[{...}, null]`
- **THEN** `name.given` yields two elements
- **AND** the first has extensions and no value, the second has a value and no
  extensions

#### Scenario: [model.primext.004] `hasValue` is false for an extension-only primitive

- **WHEN** the expression `Patient.name.given.select($this.hasValue())` is
  evaluated over `patient-name-extensions.json`
- **THEN** the result is the two booleans `false`, `true` in that order
- **AND** this is oracle case `miscEngineTests/testPrimitiveExtensions`

#### Scenario: [model.primext.005] a `_field` with no value sibling at all is still an element

- **WHEN** a document carries `"_birthDate": {"extension": [...]}` and no
  `birthDate`
- **THEN** `Patient.birthDate` yields one element with extensions and no value

#### Scenario: [model.primext.006] `_field` arrays of unequal length are reported

- **WHEN** `given` has three entries and `_given` has two
- **THEN** decoding reports the mismatch at path `Patient.name.given`

#### Scenario: [model.primext.007] a `_field` sibling round-trips in place

- **WHEN** a document with `_field` siblings is decoded and re-encoded unchanged
- **THEN** the output is byte-identical, with the `null` holes in the same
  positions

#### Scenario: [model.primext.008] reading a primitive's value does not materialise its extensions

- **WHEN** `Patient.birthDate`'s value is read from a document whose
  `_birthDate` carries a large extension tree
- **THEN** the value is returned and the extension tree reports as not
  materialised

#### Scenario: [model.primext.009] `_field` is rejected on a non-primitive element

- **WHEN** a document carries `"_name": {...}` for `Patient.name`, which is a
  complex type
- **THEN** decoding reports an unexpected `_` property at `Patient.name`

### Requirement: Canonical serialisation

`frevo-model` SHALL produce the canonical JSON form that
`org.hl7.fhir.r4.formats.JsonCreatorCanonical` produces, so the round-trip
oracle compares like with like.

#### Scenario: [model.canonical.001] canonical form sorts property names and emits no insignificant whitespace

- **WHEN** a resource is serialised in canonical form
- **THEN** every object's property names are in ascending lexicographic order
- **AND** there is no whitespace between tokens

#### Scenario: [model.canonical.002] canonical form preserves array order

- **WHEN** a resource with repeating elements is serialised in canonical form
- **THEN** array element order is unchanged, because arrays are ordered in FHIR
  and only object properties are sorted

#### Scenario: [model.canonical.003] canonical form preserves a decimal's presented form

- **WHEN** a decimal of `1.58700` is serialised in canonical form
- **THEN** the output is `1.58700`, not `1.587`

#### Scenario: [model.canonical.004] canonical form is idempotent

- **WHEN** canonical output is decoded and canonically serialised again
- **THEN** the second output is byte-identical to the first

#### Scenario: [model.canonical.005] canonicalising a pretty-printed example yields the same bytes as canonicalising the round-trip

- **WHEN** an R4 example file is canonicalised directly, and the same file is
  decoded, re-encoded and canonicalised
- **THEN** both byte sequences are identical
- **AND** this is the comparison the `r4-example-roundtrip` oracle performs

#### Scenario: [model.canonical.006] canonical and pretty forms differ only in whitespace and key order

- **WHEN** the same resource is serialised canonically and with indentation
- **THEN** decoding both yields resources whose every element is equal

### Requirement: Copy-on-write mutation with byte splicing

A change to one element SHALL not cost a re-serialisation of the whole
document, and SHALL not change the bytes of any subtree that was not touched.

#### Scenario: [model.mutate.001] changing one element preserves the bytes of untouched subtrees

- **WHEN** `Patient.active` is set to `false` on a decoded document and the
  resource is serialised
- **THEN** the byte ranges that encoded `Patient.name`, `Patient.contained` and
  `Patient.text` in the input appear unchanged in the output

#### Scenario: [model.mutate.002] adding a repeating element keeps the existing entries' bytes

- **WHEN** a fourth `Patient.name` is appended and the resource is serialised
- **THEN** the first three names' byte ranges are unchanged and the fourth is
  newly encoded

#### Scenario: [model.mutate.003] removing an element removes only its bytes

- **WHEN** `Patient.name[1]` is removed and the resource is serialised
- **THEN** `name[0]` and the former `name[2]` appear with their original bytes
  and in that order

#### Scenario: [model.mutate.004] a mutated resource no longer claims byte-identity

- **WHEN** any element has been changed
- **THEN** serialising does not return the original bytes, and the resource
  reports itself as modified

#### Scenario: [model.mutate.005] setting an element to the value it already has still marks the resource modified

- **WHEN** `Patient.gender` is set to the value it already holds
- **THEN** the resource reports as modified
- **AND** serialising produces a document that decodes equal to the original

#### Scenario: [model.mutate.006] a mutation inside a contained resource splices at that resource's boundary

- **WHEN** an element of `Patient.contained[0]` is changed
- **THEN** the serialised output re-encodes only `contained[0]` and splices the
  original bytes for `contained[1..]` and for the enclosing `Patient`

#### Scenario: [model.mutate.007] `Meta` updates are the common case and are cheap

- **WHEN** `Resource.meta.versionId` and `Resource.meta.lastUpdated` are set on
  a 64 KiB resource and it is serialised
- **THEN** every byte outside `meta` is spliced from the original
- **AND** the operation is measured by the `model-meta-update` workload in
  `bench/micro/`

### Requirement: The navigation surface `frevo-path` evaluates over

`frevo-model` SHALL implement the node-navigation and model-information traits
that `frevo-path` defines, returning **owned** values, so that the engine holds
no borrow into the model and no knowledge of its concrete types (ADR 0004
decision 5).

#### Scenario: [model.nav.001] a node reports its FHIR type name

- **WHEN** the type name of a decoded `Patient`, of its `name` child and of its
  `name.family` child is requested
- **THEN** the answers are `Patient`, `HumanName` and `string`

#### Scenario: [model.nav.002] children are returned by element name in document order

- **WHEN** the children named `name` of a decoded `Patient` are requested
- **THEN** they are returned in document order

#### Scenario: [model.nav.003] a choice element is reached by its base name

- **WHEN** the children named `value` of a decoded `Observation` carrying
  `valueQuantity` are requested
- **THEN** one child is returned, whose type name is `Quantity`

#### Scenario: [model.nav.004] the typed property name yields no children

- **WHEN** the children named `valueQuantity` of that same `Observation` are
  requested
- **THEN** no children are returned, which is what makes oracle case
  `polymorphics/testPolymorphicsB` an empty result at runtime

#### Scenario: [model.nav.005] an absent element yields an empty child list, not an error

- **WHEN** the children named `birthDate` of a `Patient` without one are
  requested
- **THEN** an empty list is returned

#### Scenario: [model.nav.006] all children can be enumerated with their names

- **WHEN** every child of a decoded `Patient` is enumerated
- **THEN** each comes back paired with its element name, in document order
- **AND** `extension` and `modifierExtension` appear as ordinary named children

#### Scenario: [model.nav.007] returned values are owned and outlive the node they came from

- **WHEN** a child value is obtained and the node it was obtained from is
  dropped
- **THEN** the value is still readable

#### Scenario: [model.nav.008] a node reports whether it is a resource

- **WHEN** a decoded `Patient`, its `contained[0]` and its `name` are each asked
- **THEN** the first two report as resources and the third does not

#### Scenario: [model.nav.009] a primitive node reports its value, or reports having none

- **WHEN** `Patient.birthDate` with a value, and an extension-only
  `Patient.birthDate`, are each asked for a primitive value
- **THEN** the first returns `1974-12-25` and the second returns none

#### Scenario: [model.nav.010] the model answers type-hierarchy questions

- **WHEN** the model is asked whether `code` derives from `string`, and whether
  `Age` derives from `Quantity`
- **THEN** both are yes, which is what makes `Patient.gender.is(string)` and
  `...value is Quantity` true in oracle group `testInheritance`

#### Scenario: [model.nav.011] the model answers element-type questions for the static check

- **WHEN** the model is asked for the declared types of `Observation.value[x]`
- **THEN** it returns the full declared choice list

#### Scenario: [model.nav.012] navigation works over a node that is not `frevo-model`

- **WHEN** the same expression is evaluated over a `frevo-model` node and over
  the harness's test-only node built from an XML oracle input
- **THEN** the results are equal
- **AND** `frevo-path` contains no reference to any `frevo-model` type

### Requirement: Error reporting

A decode failure SHALL identify the case well enough to act on without reading
the codec.

#### Scenario: [model.errors.001] every decode error carries an element path

- **WHEN** any decode error from `model.json.*` or `model.primext.*` is reported
- **THEN** it names the FHIR element path, not a JSON pointer alone

#### Scenario: [model.errors.002] every decode error carries a byte offset

- **WHEN** any decode error is reported
- **THEN** it names the byte offset in the input at which the problem was found

#### Scenario: [model.errors.003] a strict decode reports the first error; a lenient decode reports every diagnostic

- **WHEN** a document containing several independent problems is decoded in
  strict mode
- **THEN** decoding returns an error for the first one in document order and
  does not panic
- **WHEN** the same document is decoded in lenient mode
- **THEN** decoding succeeds and returns every diagnostic, in document order,
  because HAPI logs each one and continues

#### Scenario: [model.errors.004] truncated input is an error, not a panic

- **WHEN** a valid document is truncated at every byte offset in turn and each
  prefix is decoded
- **THEN** every prefix either decodes or returns an error, and none panics

#### Scenario: [model.errors.005] adversarially nested input is bounded

- **WHEN** a document nests arrays and objects 10,000 deep
- **THEN** decoding returns a depth-limit error rather than overflowing the
  stack

### Requirement: Model performance budgets

`frevo-model` SHALL meet the budgets below, which are gate conditions, and SHALL report measured before and after figures for any change that touches them.

Budgets, their derivation and the workloads that measure them are in
[`../../design.md`](../../design.md). They are gate conditions.

#### Scenario: [model.perf.001] pass-through is a byte copy

- **WHEN** the `model-passthrough` workload loads a 4 KiB `Patient` from bytes
  and serialises it unchanged
- **THEN** the median time is at most 1 µs per operation
- **AND** no JSON token is scanned

#### Scenario: [model.perf.002] a single-element read is far cheaper than a full parse

- **WHEN** the `model-lazy-read` and `model-full-parse` workloads run over the
  same 4 KiB `Patient`
- **THEN** `model-lazy-read` is at most one tenth of `model-full-parse`'s median
  time
- **AND** `model-lazy-read`'s median is at most 2 µs

#### Scenario: [model.perf.003] full materialisation stays within budget

- **WHEN** the `model-full-parse` workload decodes a 4 KiB `Patient` completely
- **THEN** the median time is at most 20 µs

#### Scenario: [model.perf.004] a metadata update is proportional to `meta`, not to the document

- **WHEN** the `model-meta-update` workload runs over a 4 KiB and a 64 KiB
  resource
- **THEN** the median times differ by at most 25%

#### Scenario: [model.perf.005] canonical serialisation of a constructed resource stays within budget

- **WHEN** the `model-canonical-encode` workload canonically serialises a
  constructed 4 KiB `Patient`
- **THEN** the median time is at most 25 µs

#### Scenario: [model.perf.006] no regression against the recorded baseline

- **WHEN** any `bench/micro/` model workload is compared with the baseline
  recorded for the previous commit on `main`
- **THEN** no workload is more than 10% slower, per the regression policy in
  `bench/targets.md`
