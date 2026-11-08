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

#### Scenario: [codegen.choice.004] two values for the same choice element are rejected on decode

- **WHEN** a JSON document carries both `valueString` and `valueQuantity` for
  `Observation.value[x]`
- **THEN** decoding fails with an error naming the element path and both
  properties

### Requirement: Cardinality

`frevo-codegen` SHALL carry each element's declared cardinality into the
generated type so that a single-valued element cannot hold a list and a
repeating element preserves order.

#### Scenario: [codegen.cardinality.001] `0..1` is optional and single

- **WHEN** `Patient.birthDate` (`0..1`) is generated
- **THEN** it is absent or holds exactly one value

#### Scenario: [codegen.cardinality.002] `1..1` is required and single

- **WHEN** `Observation.status` (`1..1`) is generated
- **THEN** it holds exactly one value, and a document lacking it is reported as
  a missing required element rather than silently defaulted

#### Scenario: [codegen.cardinality.003] `0..*` is an ordered list

- **WHEN** `Patient.name` (`0..*`) is generated and decoded from a document with
  three `name` entries
- **THEN** the three values are returned in document order
- **AND** re-serialising preserves that order

#### Scenario: [codegen.cardinality.004] `1..*` is a non-empty ordered list

- **WHEN** `Bundle.entry.request` ... `Questionnaire.item.answerOption`-style
  `1..*` elements are generated
- **THEN** an empty list is reported as a cardinality violation on decode, not
  accepted

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

#### Scenario: [codegen.binding.002] a code outside a required binding is preserved, not dropped

- **WHEN** a document carries `"gender": "M"` for `Patient.gender`
- **THEN** decoding reports the code as not in the required binding
- **AND** re-serialising the resource unchanged still emits `"gender": "M"`

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

- **WHEN** `Encounter.class`, `ElementDefinition.type` and
  `OperationDefinition.use`-style elements are generated
- **THEN** each gets a deterministic non-keyword Rust name
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
