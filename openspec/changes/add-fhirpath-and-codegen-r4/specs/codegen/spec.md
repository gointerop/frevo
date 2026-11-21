# codegen spec delta

## ADDED Requirements

### Requirement: Pinned, verified generator inputs

`frevo-codegen` SHALL generate from a FHIR definition package identified by
version and content digest, and SHALL refuse to generate from anything else, so
that the generated model is reproducible from the repository alone.

#### Scenario: [codegen.inputs.001] generating from the pinned R4 package succeeds

- **WHEN** `frevo-codegen generate --version r4` runs with the pinned
  `hl7.fhir.r4.core#4.0.1` package present and matching its recorded SHA-256
- **THEN** the command exits `0`
- **AND** it reports the package name, the version `4.0.1` and the digest it
  verified

#### Scenario: [codegen.inputs.002] a digest mismatch refuses to generate

- **WHEN** the input package's SHA-256 does not match the recorded digest
- **THEN** the command exits non-zero without writing any output file
- **AND** the error names the expected and the actual digest

#### Scenario: [codegen.inputs.003] a missing package refuses to generate

- **WHEN** the input package is absent
- **THEN** the command exits non-zero, names the package and version it needs
  and the command that fetches it, and writes no output file

#### Scenario: [codegen.inputs.004] the generator does not read `reference/`

- **WHEN** `frevo-codegen generate --version r4` runs with `reference/` absent
- **THEN** generation succeeds and produces output identical to a run with
  `reference/` present

#### Scenario: [codegen.inputs.005] the pinned package and the HAPI copy agree

- **WHEN** the pinned package's `StructureDefinition` set is compared with
  `reference/hapi-fhir/hapi-fhir-validation-resources-r4/src/main/resources/org/hl7/fhir/r4/model/profile/*.xml`
  at SHA `7ab4c7c7c49281496faa1b3c2498a652457669ae`
- **THEN** both declare `fhirVersion` `4.0.1`
- **AND** the set of StructureDefinition canonical URLs is identical in both
- **AND** for every URL, the element names, types and cardinalities of the
  snapshot agree

#### Scenario: [codegen.inputs.006] the input inventory is reported and asserted

- **WHEN** generation completes for R4
- **THEN** the report states 63 datatype StructureDefinitions, 149 resource
  StructureDefinitions (146 concrete, 2 abstract, 1 logical) and 1,375
  SearchParameter resources
- **AND** a count that differs from those numbers fails the run rather than
  silently generating a different model

### Requirement: Generated datatypes

`frevo-codegen` SHALL emit a Rust type for every R4 datatype StructureDefinition,
primitive and complex, with the element set the snapshot declares.

#### Scenario: [codegen.types.001] every primitive datatype is generated

- **WHEN** generation completes
- **THEN** a type exists for each of the 20 R4 primitive datatypes —
  `base64Binary`, `boolean`, `canonical`, `code`, `date`, `dateTime`, `decimal`,
  `id`, `instant`, `integer`, `markdown`, `oid`, `positiveInt`, `string`,
  `time`, `unsignedInt`, `uri`, `url`, `uuid`, `xhtml`
- **AND** each carries the `id` and `extension` properties that `Element`
  declares

#### Scenario: [codegen.types.002] every complex datatype is generated

- **WHEN** generation completes
- **THEN** a type exists for each complex datatype StructureDefinition in
  `profiles-types.xml`, including `Address`, `Age`, `Annotation`, `Attachment`,
  `CodeableConcept`, `Coding`, `ContactPoint`, `Count`, `Distance`, `Dosage`,
  `Duration`, `HumanName`, `Identifier`, `Money`, `Period`, `Quantity`,
  `Range`, `Ratio`, `Reference`, `SampledData`, `Signature`, `Timing`

#### Scenario: [codegen.types.003] metadata types are generated

- **WHEN** generation completes
- **THEN** types exist for `ElementDefinition`, `Extension`, `Meta`,
  `Narrative`, `ParameterDefinition`, `DataRequirement`, `Expression`,
  `TriggerDefinition`, `UsageContext`, `RelatedArtifact`, `ContactDetail`,
  `Contributor`

#### Scenario: [codegen.types.004] abstract types are generated as shared element sets, not instantiable resources

- **WHEN** generation completes
- **THEN** the elements `Resource` declares (`id`, `meta`, `implicitRules`,
  `language`) appear on every generated resource type
- **AND** the elements `DomainResource` adds (`text`, `contained`, `extension`,
  `modifierExtension`) appear on every generated resource type except
  `Bundle`, `Binary` and `Parameters`
- **AND** no value of type `Resource` or `DomainResource` can be constructed

### Requirement: Generated resources

`frevo-codegen` SHALL emit a Rust type for every concrete R4 resource
StructureDefinition, with its backbone elements.

#### Scenario: [codegen.types.005] all 146 concrete resource types are generated

- **WHEN** generation completes
- **THEN** a resource type exists for every StructureDefinition in
  `profiles-resources.xml` with `kind` `resource`, `abstract` `false` and
  `derivation` `specialization`, and there are 146 of them
- **AND** the generated resource-type registry maps each FHIR type name to its
  type and back

#### Scenario: [codegen.types.006] backbone elements become named nested types

- **WHEN** `Patient` is generated
- **THEN** nested types exist for `Patient.contact`, `Patient.communication`
  and `Patient.link`
- **AND** each nested type's element set matches the snapshot path prefix it was
  generated from

#### Scenario: [codegen.types.007] deeply nested backbone elements are generated at every level

- **WHEN** `Questionnaire` is generated
- **THEN** a nested type exists for `Questionnaire.item` and for
  `Questionnaire.item.enableWhen`, `Questionnaire.item.answerOption` and
  `Questionnaire.item.initial`

#### Scenario: [codegen.types.008] recursive backbone elements terminate

- **WHEN** `Questionnaire.item` is generated, whose `item` child is of type
  `Questionnaire.item`
- **THEN** generation terminates
- **AND** the recursive child is reachable to unbounded depth at runtime

#### Scenario: [codegen.types.009] type cycles across datatypes terminate

- **WHEN** generation processes `Reference`, which contains `Identifier`, which
  contains `Reference`
- **THEN** generation terminates and both types are complete

### Requirement: Choice elements

`frevo-codegen` SHALL generate `value[x]`-style choice elements so that the
element is addressed by its base name, which is the name FHIRPath navigates by.

#### Scenario: [codegen.choice.001] a choice element is generated under its base name

- **WHEN** `Observation.value[x]` is generated
- **THEN** the element is named `value`
- **AND** its value carries which of the declared types it holds
- **AND** the JSON property names `valueQuantity`, `valueCodeableConcept`,
  `valueString`, `valueBoolean`, `valueInteger`, `valueRange`, `valueRatio`,
  `valueSampledData`, `valueTime`, `valueDateTime` and `valuePeriod` all map to
  it

#### Scenario: [codegen.choice.002] every declared choice type is representable

- **WHEN** `Extension.value[x]` is generated, which permits every open type
- **THEN** each of the declared types is representable, including `Age`,
  `Quantity`, `Duration` and `Reference`

#### Scenario: [codegen.choice.003] a choice element with a single permitted type is still a choice

- **WHEN** a StructureDefinition declares an element whose path ends in `[x]`
  with exactly one `type` entry
- **THEN** it is generated as a choice element, addressed by its base name

#### Scenario: [codegen.choice.004] a second value for the same choice element is a repeated non-repeatable element

- **WHEN** a JSON document carries `valueString` and then `valueQuantity` for
  `Observation.value[x]`, decoded in strict mode
- **THEN** decoding fails at the **second** property with HAPI's
  repeated-element error, message code `1823`:
  `HAPI-1823: Multiple repetitions of non-repeatable element 'value' found during parse`.
  HAPI's `ParserState.enteringNewElement` records each `0..1` child it enters
  under the choice's field name, `value` for every `value[x]` property
  (`RuntimeChildChoiceDefinition.getField().getName()`), so the second property
  is a repetition (`hapi-fhir-base/src/main/java/ca/uhn/fhir/parser/ParserState.java:660-672`), and
  `StrictErrorHandler.unexpectedRepeatingElement` throws with no location
  (`StrictErrorHandler.java:76-79`; `describeLocation(null)` is empty,
  `ParseErrorHandler.java:24-30`). The message names `value`, not either JSON
  property
- **WHEN** the same document is decoded in lenient mode, the default
- **THEN** decoding **succeeds**: `LenientErrorHandler.unexpectedRepeatingElement`
  only logs (`LenientErrorHandler.java:183-190`) and the parser swallows the
  second property and everything under it (`SwallowChildrenWholeState`,
  `ParserState.java:670`), so `Observation.value` holds the **first** property
  in document order, the `string` `6.3`, and the `Quantity` is not part of the
  decoded resource; a repeated-element diagnostic naming `value` is returned
  where HAPI logs
- **AND** a canonical encode of the decoded resource emits only `valueString`
- **AND** a document with only one `value[x]` property decodes cleanly in both
  modes

### Requirement: Cardinality

`frevo-codegen` SHALL carry each element's declared cardinality into the
generated type so that a single-valued element cannot hold a list and a
repeating element preserves order.

#### Scenario: [codegen.cardinality.001] `0..1` is optional and single

- **WHEN** `Patient.birthDate` (`0..1`) is generated
- **THEN** it is absent or holds exactly one value

#### Scenario: [codegen.cardinality.002] `1..1` is required and single, and its absence is not checked on decode

- **WHEN** `Observation.status` (`1..1`) is generated
- **THEN** the element holds at most one value and records that its minimum
  cardinality is 1
- **WHEN** an `Observation` document with no `status` is decoded, in lenient or
  strict mode
- **THEN** decoding **succeeds** in both modes with the element absent and **no**
  diagnostic: HAPI's parser checks no minimum cardinality while decoding.
  `ParserState` never consults a child's `getMin()`, and the four call sites of
  `missingRequiredElement` in the JSON parser concern only an extension's `url`
  and `value` (`hapi-fhir-base/src/main/java/ca/uhn/fhir/parser/JsonParser.java:1503`, `:1514` on decode,
  `:1995`, `:2004` on encode), so neither error handler is reached. A server
  asked to store that resource answers `2xx` unless a validator rejects it,
  which is not this change's
- **AND** in neither mode is the element given a default value

#### Scenario: [codegen.cardinality.003] `0..*` is an ordered list

- **WHEN** `Patient.name` (`0..*`) is generated and decoded from a document with
  three `name` entries
- **THEN** the three values are returned in document order
- **AND** re-serialising preserves that order

#### Scenario: [codegen.cardinality.005] a required element present but blank has no value, and strict mode rejects it

- **WHEN** an `Observation` document carries `"status": ""` for the `1..1`
  element `Observation.status`, decoded in lenient mode
- **THEN** decoding **succeeds**, `Observation.status` has no value, and an
  invalid-value diagnostic is returned where HAPI logs: a `value` of `""` is
  reported through `invalidValue` with the error `Attribute value must not be
  empty ("")` before any value is set (`hapi-fhir-base/src/main/java/ca/uhn/fhir/parser/ParserState.java:1454-1457`),
  and `LenientErrorHandler.invalidValue` logs a blank value and returns
  (`LenientErrorHandler.java:100-108`). No missing-required-element diagnostic is
  returned (`codegen.cardinality.002`)
- **WHEN** the same document is decoded in strict mode
- **THEN** decoding fails with message code `1821`:
  `HAPI-1821: [element="status"] Invalid attribute value "": Attribute value must not be empty ("")`
  (`StrictErrorHandler.java:56-59`; the location renders as
  `[element="<name>"] `, `ParseLocation.java:55-57`). The same holds for every
  blank primitive value, a blank `Reference.reference` included
  (`model.json.023`, `model.json.025`)

#### Scenario: [codegen.cardinality.004] `1..*` is an ordered list whose minimum is recorded

- **WHEN** a `1..*` element, `Appointment.participant`, is generated
- **THEN** it records that its minimum cardinality is 1 and preserves order
- **WHEN** a document gives it an empty array, `"participant": []`, decoded in
  lenient or strict mode
- **THEN** decoding **succeeds** in both modes with the element absent and no
  diagnostic: the JSON parser iterates the array's entries and an empty array
  has none (`hapi-fhir-base/src/main/java/ca/uhn/fhir/parser/JsonParser.java:1439-1457`), and no minimum
  cardinality is checked (`codegen.cardinality.002`)

### Requirement: Value-set bindings

`frevo-codegen` SHALL generate a closed enumeration only where R4 binds a
required value set whose codes are known from the input bundle, and SHALL leave
every other coded element open.

#### Scenario: [codegen.binding.001] a required binding generates a closed enumeration

- **WHEN** `Patient.gender`, bound `required` to
  `http://hl7.org/fhir/ValueSet/administrative-gender|4.0.1`, is generated
- **THEN** an enumeration exists with the codes `male`, `female`, `other`,
  `unknown`
- **AND** the enumeration round-trips to and from exactly those code strings

#### Scenario: [codegen.binding.002] a code outside a required binding is rejected on decode in both modes

- **WHEN** a document carries `"gender": "M"` for `Patient.gender`, decoded in
  lenient mode (the default) or strict mode
- **THEN** decoding **fails** in both modes with message code `1821`:
  `HAPI-1821: [element="gender"] Invalid attribute value "M": Unknown AdministrativeGender code 'M'`.
  HAPI's model holds `Patient.gender` as an `Enumeration`, whose factory throws
  `IllegalArgumentException("Unknown AdministrativeGender code 'M'")`
  (`org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/model/Enumerations.java:256-269`);
  `ParserState` reports it through `invalidValue` with that message
  (`hapi-fhir-base/src/main/java/ca/uhn/fhir/parser/ParserState.java:1482-1514`), and the default
  `LenientErrorHandler` hands a non-blank invalid value to the strict handler
  (`LenientErrorHandler.java:100-108`), which throws
  (`StrictErrorHandler.java:56-59`). A server asked to store that resource
  answers `400`
- **AND** a code that matches a binding code ignoring case is corrected **and**
  still reported first: `"gender": "Male"` calls `invalidValue` with
  `Case discrepancy detected in code value 'Male' (should be 'male')`
  (`ParserState.java:1488-1503`), so it fails the same way in both default
  modes, with that text after `Invalid attribute value "Male": `
- **AND** a blank code is a blank value: `"gender": ""` follows
  `codegen.cardinality.005` (lenient: no value and a diagnostic; strict: `1821`)
- **AND** the check applies to exactly the `code` elements HAPI's R4 model holds
  as an `Enumeration`, which is the set `path.api.005` describes and the
  generated class table of `codegen.ancestry.003` and `.004` lists (323
  elements in the 4.0.1 package); an element
  HAPI holds as a `CodeType` accepts any code, even under a `required` binding
  (`Attachment.contentType`, `Attachment.java:58`). Frevo's generated
  enumerations must cover that set, and the decode checks a code against them

#### Scenario: [codegen.binding.003] extensible, preferred and example bindings generate no enumeration

- **WHEN** `Observation.code`, bound `example`, is generated
- **THEN** no enumeration is generated for it and any code string is accepted

#### Scenario: [codegen.binding.004] a required binding whose value set is not in the input bundle generates no enumeration

- **WHEN** an element binds `required` to a value set that is not resolvable
  from the pinned package
- **THEN** no enumeration is generated, the element stays open, and the
  generator reports the element path and the unresolved value-set URL

### Requirement: Rust naming

`frevo-codegen` SHALL map FHIR names to Rust names by a rule that is total,
collision-free and reversible, so the JSON property name is always recoverable.

#### Scenario: [codegen.naming.001] element names become snake_case fields

- **WHEN** `Patient.multipleBirthBoolean`, `Patient.deceasedDateTime` and
  `Observation.effectiveTiming` are generated
- **THEN** the field names are `multiple_birth`, `deceased` and `effective`
  (choice base names), and each records its FHIR property name

#### Scenario: [codegen.naming.002] a FHIR name that is a Rust keyword is escaped deterministically

- **WHEN** `ElementDefinition.type`, `Identifier.use`, `Claim.use` and
  `Encounter.class` are generated
- **THEN** a snake_case name that is a Rust 2024 strict or reserved keyword gets
  one `_` appended, and every other name is left as it is: `type` becomes
  `type_`, `use` becomes `use_`, and `class` stays `class`, because it is a
  keyword in Java but not in Rust
- **AND** the keyword list is exactly the Rust 2024 strict keywords `as`,
  `async`, `await`, `break`, `const`, `continue`, `crate`, `dyn`, `else`,
  `enum`, `extern`, `false`, `fn`, `for`, `if`, `impl`, `in`, `let`, `loop`,
  `match`, `mod`, `move`, `mut`, `pub`, `ref`, `return`, `self`, `Self`,
  `static`, `struct`, `super`, `trait`, `true`, `type`, `unsafe`, `use`,
  `where`, `while`, and the reserved keywords `abstract`, `become`, `box`, `do`,
  `final`, `gen`, `macro`, `override`, `priv`, `try`, `typeof`, `unsized`,
  `virtual`, `yield`: 52 names. Weak keywords (`union`, `raw`, `safe`,
  `macro_rules`) are valid identifiers and are not escaped; a raw identifier
  (`r#type`) is never emitted
- **AND** the escape is deterministic: the same FHIR name always yields the same
  Rust name
- **AND** the FHIR property names `class`, `type` and `use` are still what is
  read and written

#### Scenario: [codegen.naming.003] two FHIR names that map to the same Rust name are disambiguated and reported

- **WHEN** two elements of the same type would map to the same Rust name
- **THEN** generation fails with an error naming both FHIR paths, rather than
  emitting a type that silently loses one

#### Scenario: [codegen.naming.004] resource and type names are used unchanged

- **WHEN** `ExplanationOfBenefit`, `ChargeItemDefinition` and `MedicinalProduct`
  are generated
- **THEN** their Rust type names are the FHIR names verbatim

#### Scenario: [codegen.naming.005] enumeration variants are derived from codes and the code is recoverable

- **WHEN** a code system contains codes `entered-in-error`, `_special` and `>=`
- **THEN** each becomes a distinct variant whose original code string is
  returned on serialisation

### Requirement: Generated search-parameter tables

`frevo-codegen` SHALL emit the R4 search-parameter table from the same input
bundle as the model, so the model and the index cannot drift.

#### Scenario: [codegen.searchparam.001] every R4 SearchParameter is in the table

- **WHEN** generation completes
- **THEN** the table holds one row per SearchParameter resource in
  `search-parameters.json`, 1,375 in total
- **AND** each row carries the parameter's `code`, canonical `url`, `type`,
  `base` list and `expression`

#### Scenario: [codegen.searchparam.002] a parameter with several bases appears once per base

- **WHEN** the table is queried for `Patient` and for `Practitioner`
- **THEN** the `_id`, `_lastUpdated`, `_profile`, `_security`, `_source` and
  `_tag` parameters appear for both, each with the expression its
  SearchParameter declares

#### Scenario: [codegen.searchparam.003] composite parameters carry their components

- **WHEN** the table row for `Observation-code-value-quantity` is read
- **THEN** its type is `composite`
- **AND** it names its component parameter definitions and each component's
  expression

#### Scenario: [codegen.searchparam.004] reference parameters carry their targets

- **WHEN** the table row for `Observation-subject` is read
- **THEN** its type is `reference` and its target list is the resource types the
  SearchParameter declares

#### Scenario: [codegen.searchparam.005] every table expression parses

- **WHEN** every `expression` in the generated table is handed to
  `frevo-path`'s parser
- **THEN** each one parses without error
- **AND** a parameter whose expression is absent is recorded as having none,
  distinctly from having an empty one

#### Scenario: [codegen.searchparam.006] parameter modifiers and flags are carried

- **WHEN** a table row is read
- **THEN** it carries `multipleOr`, `multipleAnd`, `comparator`, `modifier` and
  `chain` as the SearchParameter declares them, with absent meaning "not
  stated" rather than `false`

### Requirement: Generator output discipline

`frevo-codegen`'s output SHALL be deterministic, marked as generated, and
accepted by the project's lint and format gates without hand editing.

#### Scenario: [codegen.output.001] regeneration is byte-identical

- **WHEN** generation runs twice from the same input package on the same commit
- **THEN** every output file is byte-identical between the two runs

#### Scenario: [codegen.output.002] regeneration is byte-identical across platforms

- **WHEN** generation runs on Linux and on Windows from the same input package
- **THEN** every output file is byte-identical, including line endings

#### Scenario: [codegen.output.003] every output file carries the `@generated` marker and its inputs

- **WHEN** any generated file is read
- **THEN** its first lines contain `@generated`, the generator name and version,
  the input package name, version and SHA-256, and a statement that edits are
  overwritten

#### Scenario: [codegen.output.004] generated code passes the lint and format gates

- **WHEN** `cargo fmt --all --check` and
  `cargo clippy --workspace --all-targets -- -D warnings` run after generation
- **THEN** both pass with no `#[allow]` added to accommodate the generator's
  output style

#### Scenario: [codegen.output.005] a drift check fails when checked-in output does not match a fresh run

- **WHEN** the checked-in `frevo-model` sources differ from a fresh generation
- **THEN** the check fails and names the differing files
- **AND** the message says to regenerate, not to edit

#### Scenario: [codegen.output.006] generated code contains no `unsafe`

- **WHEN** any generated file is read
- **THEN** it contains no `unsafe` block, and `frevo-model` declares
  `#![forbid(unsafe_code)]`

### Requirement: Generated model and codec completeness

The generated output SHALL be sufficient for `frevo-model` to decode, navigate
and canonically re-encode every R4 resource, which is what the round-trip oracle
measures.

#### Scenario: [codegen.roundtrip.001] every R4 specification example round-trips byte-identically

- **WHEN** every example in the R4 example package is decoded and canonically
  re-encoded, and both the source and the result are canonicalised the same way
- **THEN** the two byte sequences are identical for every example
- **AND** the report names each example by file name, so a failure identifies a
  case rather than a count

#### Scenario: [codegen.roundtrip.002] the round-trip covers every resource type that has an example

- **WHEN** the round-trip run finishes
- **THEN** the report lists, for each of the 146 concrete resource types, how
  many examples exercised it
- **AND** a type with zero examples is reported as uncovered rather than omitted

#### Scenario: [codegen.roundtrip.003] round-tripping an XML example through the JSON codec is not claimed

- **WHEN** the example set contains XML files
- **THEN** they are reported as `not-applicable-xml`, a status distinct from
  `passed`

#### Scenario: [codegen.roundtrip.004] the element-order table is generated

- **WHEN** the generated output is read
- **THEN** it contains, for every generated type, the element order the
  StructureDefinition snapshot declares
- **AND** that order is what `frevo-model` emits when it serialises a resource
  it constructed, as distinct from the lexical key order the round-trip
  comparison canonicalises to (`model.canonical.001`)

### Requirement: R4 model class table

`frevo-codegen` SHALL generate, from the R4 StructureDefinitions and a reviewed exception file only, a table of the HAPI R4 model class of every value and its assignability (`path.api.005`), reading no Java source (constitution section 4; owner decision 2026-11-21).

#### Scenario: [codegen.ancestry.001] the class table is generated from the StructureDefinitions and the exception file only

- **WHEN** `frevo-codegen` runs over the pinned `hl7.fhir.r4.core#4.0.1`
  package (`codegen.inputs.001`) and the exception file
  `crates/frevo-codegen/data/r4-class-table-exceptions.toml`
- **THEN** it emits one `@generated` table, regenerated byte-identically from
  the same inputs (`codegen.output.*`), whose rows are every R4 model class a
  navigated or engine-built value can have:
  - the table is written by `frevo-codegen class-table --version r4` as a
    tab-separated file at `crates/frevo-path/src/eval/r4_classes.tsv`, in
    `frevo-path` and not in `frevo-model`: `frevo-path` is the crate that
    evaluates against it, and `path.api.008` forbids `frevo-path` from
    depending on `frevo-model`. No normative statement places it in
    `frevo-model`; constitution section 4 requires only that it is generated
    and never hand-edited;
  - its header carries the `@generated` marker, the generator name and
    version, the package name, version and SHA-256, the exception file's path
    and SHA-256, and a statement that edits are overwritten
    (`codegen.output.003`);
  - the drift check of `codegen.output.005` covers this file as it covers
    `frevo-model`'s generated sources: a checked-in `r4_classes.tsv` that
    differs from a fresh run fails the check, naming the file;
  - one row per FHIR type of kind `primitive-type`, `complex-type` and
    `resource` defined by specialization in the package, including the
    abstract `Element`, `BackboneElement`, `Resource` and `DomainResource`;
  - one row per nested component, that is per element of type
    `BackboneElement` (or `Element`, inside a datatype) that is not a
    `contentReference`;
  - one row for HAPI's `Enumeration` class
- **AND** each row carries: the class's Java simple name and fully qualified
  name (`org.hl7.fhir.r4.model.<Class>`, with `$` before a nested class's
  name); the set of FHIR type names the class is assignable to; and which of
  the interface kinds `IBase`, `IPrimitiveType`, `ICompositeType` and
  `IBaseReference` it implements
- **AND** each element of a generated type carries the row of the class HAPI
  holds its value in, which is what `path.api.005` checks and what
  `codegen.binding.002` uses as the set of `Enumeration` elements
- **AND** the generator reads no `.java` file: a test runs it with
  `reference/` absent and gets the same bytes
- **AND** the oracle is the generator's own output (regeneration determinism),
  the spec-reviewer's check of every exception entry against the cited line,
  and the `path.api.005` tests; the live differential grades the rest
  (`DEBT-0021`)

#### Scenario: [codegen.ancestry.002] class names and the FHIR type hierarchy follow fixed rules

- **WHEN** a row is derived from a StructureDefinition
- **THEN** its class name is: for a primitive type, the type name with its
  first letter upper-cased and `Type` appended (`string` is `StringType`,
  `dateTime` is `DateTimeType`, `positiveInt` is `PositiveIntType`); for a
  complex type or resource, the type name, except `List`, which is
  `ListResource`; for a nested component, the element's
  `structuredefinition-explicit-type-name` value with its first letter
  upper-cased and `Component` appended when the extension is present
  (`Patient.contact` is `ContactComponent`, `Consent.provision` is
  `ProvisionComponent`), and otherwise the element path's segments, each with
  its first letter upper-cased, concatenated, with `Component` appended
  (`Patient.communication` is `PatientCommunicationComponent`,
  `Observation.component` is `ObservationComponentComponent`); and, for a
  `contentReference` element, the class of the element it references
- **AND** a class is assignable to its own FHIR type and to every type on the
  `baseDefinition` chain of its StructureDefinition (`string` to `string` and
  `Element`; `Patient` to `Patient`, `DomainResource` and `Resource`;
  `code` to `code`, `string` and `Element`), and a nested component to
  `BackboneElement` and `Element`, or to `Element` inside a datatype; the
  classes HAPI adds that have no FHIR type (`Base`, `Type`, `PrimitiveType`,
  `BaseDateTimeType`, `BackboneType`, `MetadataResource`, `BaseReference`,
  `BaseNarrative`, `BaseExtension`, `BaseResource`, `BaseBinary`) are not rows and no FHIR
  type name stands for them, except as the exception file says
- **AND** the interface kinds are: `IBase` for every row; `IPrimitiveType` for
  every primitive-type row and `Enumeration`
  (`PrimitiveType.java:46`, `Enumeration.java:76`); `ICompositeType` for every
  concrete `complex-type` row, directly or, for `Reference`, `Meta`,
  `Narrative` and `Extension`, through `IBaseReference`, `IBaseMetaType`,
  `INarrative` and `IBaseExtension` as `path.api.005` states, and not for
  `Element`, `BackboneElement` or any nested component
  (`Patient.java:230`, `BackboneElement.java:49`); `IBaseReference` for
  `Reference` (`Reference.java:50`)
- **AND** an element's value class is the class of its declared type; a
  type `http://hl7.org/fhirpath/System.*` (the 4.0.1 package's typing of every
  `id` element and of `Extension.url`) is read through its
  `structuredefinition-fhir-type` extension, so `Element.id` is `StringType`
  (`Element.java:56`) and `Extension.url` is `UriType` (`Extension.java:55`),
  while `Resource.id`, typed the same way, is `codegen.ancestry.004`'s
  exception. A `System.*` element with **no** `structuredefinition-fhir-type`
  extension takes the class of the field HAPI inherits for it: the only such
  element in the 4.0.1 package is `xhtml.id` (typed `System.String` with no
  extension), and `XhtmlType` extends `PrimitiveType`, `Type` and `Element`
  (`XhtmlType.java:41`, `PrimitiveType.java:46`, `Type.java:40`), so its `id`
  is `Element`'s `StringType` field (`Element.java:56`); the generator maps
  it to `StringType` and refuses any other `System.*` element without the
  extension, so a new one is a reviewed decision; a `Quantity` element profiled `SimpleQuantity` or `MoneyQuantity` is still a
  `Quantity` (the pinned model declares no field of either profile class); a
  choice element's value class is that of the chosen type
- **AND** the rules were checked against the pinned Java model by the
  proposer (2026-11-21): every nested component class name of the 4.0.1
  package follows them except the thirteen names in `codegen.ancestry.004`

#### Scenario: [codegen.ancestry.003] a `code` element is an `Enumeration` exactly when its required binding is closed and not a type list

- **WHEN** an element's single type is `code`
- **THEN** its value class is `Enumeration` exactly when all of these hold,
  and `CodeType` otherwise:
  - its binding strength is `required`;
  - its value set resolves in the package, and every `compose.include` either
    lists its concepts, or names a code system in the package whose `content`
    is `complete`, with no `filter`; an included value set must satisfy the
    same rule;
  - its binding's `elementdefinition-bindingName` is not `ResourceType`,
    `FHIRAllTypes` or `FHIRDefinedType` (HAPI holds the resource-type and
    data-type lists as `CodeType`, for example `SearchParameter.base` and
    `DataRequirement.type`, `DataRequirement.java:1538`)
- **AND** an element HAPI holds in an inherited field follows the same rule,
  because the R4 snapshot carries the same binding: the canonical resources'
  `status`, declared on `MetadataResource` (`MetadataResource.java:98`), and
  `Age`, `Count`, `Distance` and `Duration`'s `comparator`, declared on
  `Quantity` (`Quantity.java:231`)
- **AND** `Patient.gender`, `Observation.status` and `Address.use` are
  `Enumeration`s; `Attachment.contentType` (bound to a value set over
  `urn:ietf:bcp:13`, not in the package) and `Questionnaire.subjectType`
  (`ResourceType`) are `CodeType`s
- **AND** the rule was checked against the pinned Java model (proposer and
  fifty-fourth review, 2026-11-21): it predicts 324 `Enumeration` elements,
  and the model holds 323 of them as `Enumeration` (291 declared fields, 29
  canonical resources' inherited `MetadataResource.status` and 4 inherited
  `Quantity.comparator`) and no other element. The one exception is
  `ImplementationGuide.definition.parameter.code`, which is
  `codegen.ancestry.004`'s. Elements HAPI reuses a class for through
  `contentReference` take the referenced element's value class

#### Scenario: [codegen.ancestry.004] the exception file lists every place HAPI's class tree differs, each with its citation

- **WHEN** the exception file
  `crates/frevo-codegen/data/r4-class-table-exceptions.toml` is read
- **THEN** each entry names its kind (`superclass`, `element-class`,
  `runtime-class` or `class-name`), its target (a FHIR type, an element path
  or a component path), the effect on the table, and a `source` citing
  `<file>.java:<line>` under `reference/org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/model/` at `73c432b5`; the
  generator refuses an entry without a `source`, an entry whose target is not
  in the package, and an entry that changes nothing
- **AND** the file is versioned with the change and edited only through a
  reviewed change: a spec-reviewer checks each cited line. Its entries at this
  change are exactly:
  - `superclass` `id`: `IdType` extends `UriType` (`IdType.java:120`), so an
    `id` value is assignable to `id`, `uri` and `Element`, and **not** to
    `string`
  - `superclass` for `Dosage`, `ElementDefinition`, `MarketingStatus`,
    `Population`, `ProdCharacteristic`, `ProductShelfLife`,
    `SubstanceAmount` and `Timing`: each extends `BackboneType`, which extends
    `Type` and `Element` (`Dosage.java:50`, `ElementDefinition.java:57`,
    `MarketingStatus.java:48`, `Population.java:46`,
    `ProdCharacteristic.java:49`, `ProductShelfLife.java:48`,
    `SubstanceAmount.java:55`, `Timing.java:56`; `BackboneType.java:49`), so
    each is assignable to its own type and `Element` and **not** to
    `BackboneElement`, although R4 bases them on `BackboneElement`; each
    still implements `ICompositeType`
  - `element-class` `Resource.id` (and so every resource's `id`): typed
    `System.String` with fhir-type `string` in the package, but declared
    `IdType` (`Resource.java:53`), so a resource's `id` value is an `IdType`,
    assignable to `id`, `uri` and `Element` by the `id` entry above, and not
    to `string`; `Element.id` stays `StringType` (`Element.java:56`)
  - `element-class` `ImplementationGuide.definition.parameter.code`: declared
    `StringType` (`ImplementationGuide.java:14580-14582`), so it is assignable
    to `string` and not to `code`, and is not an `Enumeration` although its
    binding satisfies `codegen.ancestry.003`
  - `runtime-class` `Narrative.div`: declared `XhtmlType`
    (`Narrative.java:232`), but navigating by name yields a `StringType`
    (`Narrative.java:364-366`) and navigating with `*` an `XhtmlType`
    (`Narrative.java:343`), as `path.api.005` and `path.navigation.025` state
  - `class-name`, the thirteen nested components whose HAPI class name does
    not follow `codegen.ancestry.002`: `CodeSystem.concept.designation` is
    `ConceptDefinitionDesignationComponent` (`CodeSystem.java:2590`);
    `Consent.provision.actor` and `Consent.provision.data` keep their
    explicit type names' lower-case initial, `provisionActorComponent` and
    `provisionDataComponent` (`Consent.java:2287`, `:2572`), while
    `Consent.provision` itself is `ProvisionComponent` (`:1224`);
    `InsurancePlan.coverage.benefit.limit` is `CoverageBenefitLimitComponent`
    (`InsurancePlan.java:1159`); `InsurancePlan.plan.specificCost.benefit.cost`
    is `PlanBenefitCostComponent` (`InsurancePlan.java:2978`);
    `MeasureReport.group.stratifier.stratum.component` is
    `StratifierGroupComponentComponent` (`MeasureReport.java:1867`);
    `TestReport.setup.action.operation` and `.assert` are
    `SetupActionOperationComponent` and `SetupActionAssertComponent`
    (`TestReport.java:1506`, `:1878`); `TestScript.setup.action.operation`,
    `.operation.requestHeader` and `.assert` are
    `SetupActionOperationComponent`,
    `SetupActionOperationRequestHeaderComponent` and
    `SetupActionAssertComponent` (`TestScript.java:4619`, `:6199`, `:6484`);
    `ValueSet.compose.include.concept.designation` and
    `ValueSet.compose.include.filter` are
    `ConceptReferenceDesignationComponent` and `ConceptSetFilterComponent`
    (`ValueSet.java:1842`, `:2183`)
- **AND** a difference found later, by the differential or a review, is added
  here with its citation, never patched in generated output

#### Scenario: [codegen.ancestry.005] the generated table gives `path.api.005`'s answers

- **WHEN** the generated table is queried for the classes `path.api.005` names
- **THEN** `StringType`, `CodeType`, `MarkdownType`, `UriType`, `UrlType`,
  `CanonicalType`, `OidType`, `UuidType`, `IntegerType`, `PositiveIntType`,
  `UnsignedIntType`, `DateType`, `DateTimeType`, `InstantType`,
  `BooleanType`, `DecimalType`, `Base64BinaryType`, `TimeType`, `XhtmlType`,
  `IdType` and `Enumeration` are `IPrimitiveType`; `IdType` is assignable to
  `uri` and not to `string`; `Enumeration` is assignable to `Element` only;
  `HumanName`, `CodeableConcept`, `Reference`, `Meta`, `Narrative` and
  `Extension` are `ICompositeType`, `Reference` also `IBaseReference`; `Age`,
  `Count`, `Distance`, `Duration` are assignable to `Quantity`;
  `ContactComponent` (`Patient$ContactComponent`) is assignable to
  `BackboneElement` and `Element` and is not `ICompositeType`; `Patient` is
  assignable to `Patient`, `DomainResource` and `Resource`; `Dosage` is not
  assignable to `BackboneElement`
- **AND** `Patient.gender` and `Observation.status` map to `Enumeration`,
  `Patient.active` to `BooleanType`, `Patient.birthDate` to `DateType`,
  `Patient.name` to `HumanName`, `Patient.contact` to `ContactComponent`,
  `Observation.code` to `CodeableConcept`, `Observation.subject` to
  `Reference`, `Patient.id` to `IdType`, and `Narrative.div` to `StringType`
  by name and `XhtmlType` through `*`
- **AND** graded by the generator's in-process tests, and through
  `path.api.005`'s tests
