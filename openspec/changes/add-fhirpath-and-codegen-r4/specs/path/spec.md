# path spec delta

## ADDED Requirements

### Requirement: Lexing

The lexer SHALL accept the token set `FHIRLexer` accepts and reject what it
rejects.

#### Scenario: [path.lexer.001] a `//` comment runs to end of line

- **WHEN** `2 + 2 // This is a single-line comment + 4` is evaluated
- **THEN** the result is the single integer `4` (`comments/testComment1`)

#### Scenario: [path.lexer.002] several `//` comment lines are ignored

- **WHEN** an expression whose first two lines are `//` comments and whose third
  line is `2+2` is evaluated
- **THEN** the result is the single integer `4` (`comments/testComment2`)

#### Scenario: [path.lexer.003] a block comment after the expression is ignored

- **WHEN** `2 + 2` followed by a block comment containing `+2` is evaluated
- **THEN** the result is the single integer `4` (`comments/testComment3`)

#### Scenario: [path.lexer.004] text after a closing block comment is part of the expression

- **WHEN** `2 + 2`, then a block comment, then `+2` is evaluated
- **THEN** the result is the single integer `6` (`comments/testComment4`)

#### Scenario: [path.lexer.005] a leading block comment is ignored

- **WHEN** a block comment followed by `2+2` is evaluated
- **THEN** the result is the single integer `4` (`comments/testComment5`)

#### Scenario: [path.lexer.006] a comment can separate an operator from its operand

- **WHEN** `2 // comment` on one line and `/ 2` on the next is evaluated
- **THEN** the result is the single decimal `1` (`comments/testComment6`)

#### Scenario: [path.lexer.007] an inline block comment is ignored mid-expression

- **WHEN** `2 + /* inline $@%^+ * */ 2 = 4` is evaluated
- **THEN** the result is `true` (`comments/testComment9`)

#### Scenario: [path.lexer.008] a trailing operator is a syntax error

- **WHEN** `2 + 2 /` is parsed
- **THEN** parsing fails as a syntax error (`comments/testComment7`)

#### Scenario: [path.lexer.009] an unterminated block comment is a syntax error

- **WHEN** `2 + 2 /* not finished` is parsed
- **THEN** parsing fails as a syntax error (`comments/testComment8`)

#### Scenario: [path.lexer.010] a backtick-delimited identifier is accepted

- **WHEN** `Patient.is(FHIR.` with the type name `Patient` wrapped in backticks
  is evaluated over `patient-example.xml`
- **THEN** the result is `true` (`testType/testType19`)

#### Scenario: [path.lexer.011] string escape sequences are decoded

- **WHEN** a string literal containing the escapes for backslash, slash, form
  feed, carriage return, newline, tab, double quote, backtick, single quote and
  `*` is evaluated with `convertsToString()`
- **THEN** the result is `true` (`testLiterals/testLiteralStringEscapes`)

#### Scenario: [path.lexer.015] a double-quoted string is a syntax error by default

- **WHEN** `"foo" = 'foo'` is parsed on an engine with double quotes not
  allowed, which is the default and what both HAPI wrappers use
- **THEN** parsing fails as a syntax error

#### Scenario: [path.lexer.016] double quotes are accepted when the engine is told to allow them

- **WHEN** the same expression is parsed on an engine configured to allow double
  quotes
- **THEN** parsing succeeds and the result is `true`, because the lexer then
  treats a double-quoted run as a string literal
- **AND** the engine exposes this as a public setting
  (`setAllowDoubleQuotes`), threaded into the lexer at construction

#### Scenario: [path.lexer.012] an unknown escape sequence is a syntax error

- **WHEN** a string literal contains `\q`
- **THEN** parsing fails as a syntax error

#### Scenario: [path.lexer.013] a unicode escape with fewer than four hex digits is a syntax error

- **WHEN** a string literal contains `\u00`
- **THEN** parsing fails as a syntax error

#### Scenario: [path.lexer.014] an unterminated string literal is a syntax error

- **WHEN** a string literal is opened and never closed
- **THEN** parsing fails as a syntax error

### Requirement: Literals

The engine SHALL recognise the literal forms `FHIRPathEngine` recognises, and SHALL preserve a decimal literal's stated precision rather than normalising it.

#### Scenario: [path.literal.001] `true` and `false` are boolean literals

- **WHEN** `Patient.name.exists() = true` and `Patient.name.empty() = false` are
  evaluated over `patient-example.xml`
- **THEN** both yield `true` (`testLiterals/testLiteralTrue`,
  `testLiterals/testLiteralFalse`)

#### Scenario: [path.literal.002] a single-quoted string is a string literal

- **WHEN** `Patient.name.given.first() = 'Peter'` is evaluated over
  `patient-example.xml`
- **THEN** the result is `true` (`testLiterals/testLiteralString1`)

#### Scenario: [path.literal.003] an integer literal is an integer

- **WHEN** `1.convertsToInteger()`, `0.convertsToInteger()` and
  `2147483647.convertsToInteger()` are evaluated
- **THEN** each yields `true` (`testLiterals/testLiteralInteger1`,
  `testLiteralInteger0`, `testLiteralIntegerMax`)

#### Scenario: [path.literal.004] a negative integer needs parentheses before a function call

- **WHEN** `(-1).convertsToInteger()` is evaluated
- **THEN** the result is `true` (`testLiterals/testLiteralIntegerNegative1`)

#### Scenario: [path.literal.005] an unparenthesised negative integer with a function call is an execution error

- **WHEN** `-1.convertsToInteger()` is evaluated
- **THEN** evaluation fails, because the unary minus applies to the result of
  `1.convertsToInteger()` and cannot negate a boolean
  (`testLiterals/testLiteralIntegerNegative1Invalid`)

#### Scenario: [path.literal.006] a decimal literal keeps its precision

- **WHEN** `1.58700.precision()` is evaluated
- **THEN** the result is the integer `5` (`Precision/PrecisionDecimal`)

#### Scenario: [path.literal.007] date literals are accepted at each precision

- **WHEN** `@2014`, `@2014-01` and `@2014-01-05` are evaluated
- **THEN** each yields a single `date` whose precision is the number of
  components given

#### Scenario: [path.literal.008] a year-precision date reports precision 4

- **WHEN** `@2014.precision()` is evaluated
- **THEN** the result is the integer `4` (`Precision/PrecisionYear`)

#### Scenario: [path.literal.009] dateTime literals are accepted at each precision including offsets

- **WHEN** `@2014-01-05T10:30`, `@2014-01-05T10:30:00`,
  `@2014-01-05T10:30:00.000`, `@2014-01-05T10:30:00Z` and
  `@2014-01-05T10:30:00+10:00` are evaluated
- **THEN** each yields a single `dateTime`

#### Scenario: [path.literal.010] a millisecond dateTime reports precision 17

- **WHEN** `@2014-01-05T10:30:00.000.precision()` is evaluated
- **THEN** the result is the integer `17`
  (`Precision/PrecisionDateTimeMilliseconds`)

#### Scenario: [path.literal.011] time literals are accepted at each precision

- **WHEN** `@T10:30.precision()` and `@T10:30:00.000.precision()` are evaluated
- **THEN** the results are `4` and `9` (`Precision/PrecisionTimeMinutes`,
  `Precision/PrecisionTimeMilliseconds`)

#### Scenario: [path.literal.012] a quantity literal with a UCUM unit is a Quantity

- **WHEN** `1 'cm'` is evaluated
- **THEN** the result is a single `Quantity` with value `1`, code `cm` and
  system `http://unitsofmeasure.org`

#### Scenario: [path.literal.013] calendar-duration keywords are quantity literals

- **WHEN** `7 days`, `1 week`, `1 year`, `1 month`, `1 day`, `1 hour`,
  `1 minute`, `1 second` and `1 millisecond` are evaluated
- **THEN** each yields a single `Quantity`, with the plural and singular forms
  treated alike

#### Scenario: [path.literal.014] an empty collection literal is empty

- **WHEN** `{}` is evaluated
- **THEN** the result is an empty collection

#### Scenario: [path.literal.015] an integer literal beyond the 32-bit range behaves as the pinned engine does

- **WHEN** an integer literal larger than `2147483647` is evaluated
- **THEN** the engine behaves as `FHIRPathEngine` at the pinned SHA does for
  that literal, and the harness records which behaviour that is

### Requirement: Parsing and operator precedence

The parser SHALL accept an expression once and produce a reusable parsed form, and SHALL apply the operator precedence `FHIRPathEngine` applies.

#### Scenario: [path.parse.001] an expression parses once and evaluates many times

- **WHEN** an expression is parsed and then evaluated against three different
  resources
- **THEN** all three evaluations succeed, and the expression is parsed once, as
  `IFhirPath.parse` plus `evaluate(IBase, IParsedExpression, Class)` allows

#### Scenario: [path.parse.002] precedence follows the engine's table

- **WHEN** the `testPrecedence` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.parse.003] parentheses override precedence

- **WHEN** `(1 + 2) * 3 = 9` and `1 + 2 * 3 = 7` are evaluated
- **THEN** both yield `true`

#### Scenario: [path.parse.004] an unbalanced parenthesis is a syntax error

- **WHEN** an expression opens a parenthesis and never closes it
- **THEN** parsing fails as a syntax error

#### Scenario: [path.parse.005] an unknown function name is a syntax error at parse time

- **WHEN** `Patient.nosuchfunction()` is parsed
- **THEN** parsing fails, naming the function, before any evaluation happens

#### Scenario: [path.parse.006] the wrong number of arguments is reported at parse time

- **WHEN** `Patient.name.substring()` or `Patient.name.first(1)` is parsed
- **THEN** parsing fails, naming the function and its expected arity

#### Scenario: [path.parse.007] a parse error names the offset

- **WHEN** any syntax error is reported
- **THEN** the error names the character offset in the expression at which it
  was found

#### Scenario: [path.parse.008] the seven syntax-invalid oracle cases all fail to parse

- **WHEN** each of the suite's seven `invalid="syntax"` cases is parsed
- **THEN** each one fails to parse

### Requirement: Navigation

The engine SHALL navigate a resource through the node trait, addressing a polymorphic element by its base name, and SHALL yield an empty collection rather than an error for an absent element.

#### Scenario: [path.navigation.001] a leading resource type name matches the focus resource

- **WHEN** `Patient.name.given.first()` is evaluated over `patient-example.xml`
- **THEN** the result is `Peter`
- **AND** `testBasics/testEscapedIdentifierUnicode`
  (``Patient.`\u006eame`.given.first()``, expecting `Peter`) is **not** a pass,
  and is parity: HAPI does decode the escape, but in **base 32**
  (`FHIRLexer.java:511`, `Integer.parseInt(s.substring(i, i+4), 32)`), so
  `006e` is U+00CE and the name looked up is `Îame`. The case fails **at the
  check stage** with "The name 'Îame' is not valid for any of the possible
  types" (`oracles/fhirpath/hapi-8.12.0-results.json:20`, status `failed`).
  Frevo's lexer reproduces the decoder, so the case is `failed` on both sides
  (`path.oracle.012`); it is recorded under "Considered and not recorded as
  deviations" in `docs/deviations.md`, not as an entry

#### Scenario: [path.navigation.002] the resource type name may be omitted

- **WHEN** `name.given.first()` is evaluated over `patient-example.xml`
- **THEN** the result is `Peter`

#### Scenario: [path.navigation.003] a leading type name that is not the focus type yields empty

- **WHEN** `Observation.value` is evaluated over `patient-example.xml`
- **THEN** the result is an empty collection, when the check is skipped as
  `FhirPathR4` does (`path.check.013`): the leading-name special case adds the
  focus only when a type in its hierarchy matches
- **AND** the **check** rejects it first: the leading `Observation` does not
  match the `Patient` focus (`executeType`, `FHIRPathEngine.java:3231-3238`),
  so it is looked up as a child name, resolves to no type, and raises
  `FHIRPATH_UNKNOWN_NAME` naming `Observation` (`:1591-1593`). HAPI records the equivalent suite case
  `testBasics/testSimpleWithWrongContext` as `passed`
  (`oracles/fhirpath/hapi-8.12.0-results.json:24`). A test must therefore
  grade the empty answer with the check skipped, and the check failure
  separately

#### Scenario: [path.navigation.004] navigation flattens across a repeating element

- **WHEN** `Patient.name.given` is evaluated over `patient-example.xml`
- **THEN** the result is the five strings `Peter`, `James`, `Jim`, `Peter`,
  `James` in document order (`testJoin/testJoin`)

#### Scenario: [path.navigation.005] an absent element yields empty, not an error

- **WHEN** `Patient.name.prefix` is evaluated over `patient-example.xml`, which
  has none
- **THEN** the result is an empty collection
- **AND** the suite grades the same absence with `testBasics/testSimpleNone`
  (`name.suffix`, expecting no output), which HAPI passes
  (`oracles/fhirpath/hapi-8.12.0-results.json:18`) and Frevo must pass
- **AND** `testJoin/testJoinEmpty` (`name.prefix.join(',')`) is **not** a
  grading case for this scenario: navigation yields empty, but `join` then adds
  one empty string (`funcJoin`, `FHIRPathEngine.java:4547-4567`), so HAPI's
  recorded status is `failed` (`hapi-8.12.0-results.json:457`) and Frevo's
  required status is `failed` too, under `path.string.015` and
  `path.oracle.012`

#### Scenario: [path.navigation.006] a polymorphic element is navigated by its base name

- **WHEN** `Observation.value.exists()` is evaluated over
  `observation-example.xml`
- **THEN** the result is `true` (`polymorphics/testPolymorphicsA`)

#### Scenario: [path.navigation.007] a typed polymorphic property name is a semantic error

- **WHEN** `Observation.valueQuantity.exists()` is checked over
  `observation-example.xml`
- **THEN** the static check fails as a semantic error
  (`polymorphics/testPolymorphicsB`)

#### Scenario: [path.navigation.008] the indexer selects by zero-based position

- **WHEN** `Patient.name[0].given[0]` is evaluated over `patient-example.xml`,
  and the `testIndexer` group's cases are evaluated
- **THEN** the first result is `Peter`, and `testIndexer1` and `testIndexer2`
  (the group has three cases) yield their stated output
- **AND** `testIndexer3` (`Patient.name[-1]`) is graded `failed` for HAPI
  (`oracles/fhirpath/hapi-8.12.0-results.json`) and for Frevo alike, per
  `path.navigation.010` and `DEV-0012`

#### Scenario: [path.navigation.009] an out-of-range index yields empty

- **WHEN** `Patient.name[99]` is evaluated over `patient-example.xml`
- **THEN** the result is an empty collection

#### Scenario: [path.navigation.010] a negative index raises

- **WHEN** `Patient.name[-1]` is evaluated over `patient-example.xml`
- **THEN** evaluation fails with an execution error, as `skip(-1)` does
  (`path.subset.006`): `funcItem` (`FHIRPathEngine.java:6277-6283`) guards only
  the upper bound, `Utilities.isInteger("-1")` is true and `-1 < 3`, so
  `focus.get(-1)` is reached and HAPI ends the call with an unchecked
  `IndexOutOfBoundsException`
- **AND** the failure is a returned error, never a panic (`path.errors.003`)
- **AND** the pinned suite's `testIndexer/testIndexer3` expects empty and
  carries no `invalid` attribute, so HAPI fails its own case and Frevo matches:
  `failed` on both sides, parity under `path.oracle.012`. This is `DEV-0012`,
  ruled for HAPI by the owner on 2026-11-17

#### Scenario: [path.navigation.011] a non-integer index is an execution error

- **WHEN** `Patient.name['x']` is evaluated
- **THEN** evaluation fails
- **AND** the static check rejects it before evaluation is reached: `case Item`
  (`FHIRPathEngine.java:3363-3367`) types the argument against a singleton
  `integer` and `checkParamTypes` (`:3830`) raises `FHIRPATH_WRONG_PARAM_TYPE`
  for `string`. A test of the execution error must skip the check

#### Scenario: [path.navigation.012] a contained resource's elements are navigable

- **WHEN** `contained.id` is evaluated over `patient-container-example.json`
- **THEN** the result is the single `id` `1`
  (`miscEngineTests/testContainedId`)

#### Scenario: [path.navigation.013] `Resource.id` yields the logical id, not the full URL

- **WHEN** `Patient.id` is evaluated over a `Patient` whose id was set from
  `http://base/Patient/123/_history/222`
- **THEN** the result is the single value `123`, which is
  `FHIRPathTests.testEvaluate_Id`'s assertion

#### Scenario: [path.navigation.014] navigation into a primitive's extension works

- **WHEN** `Patient.birthDate.extension.url` is evaluated over a document whose
  `_birthDate` carries one extension
- **THEN** the extension's URL is returned

#### Scenario: [path.navigation.015] `$this` is the current item

- **WHEN** `Patient.name.given.where(substring($this.length()-3) = 'ter')` is
  evaluated over `patient-example.xml`
- **THEN** the result is the two strings `Peter`, `Peter`
  (`testDollar/testDollarThis2`)

#### Scenario: [path.navigation.016] `$this` with no match yields empty

- **WHEN** `Patient.name.given.where(substring($this.length()-3) = 'out')` is
  evaluated over `patient-example.xml`
- **THEN** the result is an empty collection (`testDollar/testDollarThis1`)

#### Scenario: [path.navigation.017] `$index` is the current item's position

- **WHEN** `Patient.telecom.select(iif(value='(03) 3410 5613', $index, {} ))` is
  evaluated over `patient-example.xml`
- **THEN** the result is the single integer `2` (`index-part/testIndex`)

#### Scenario: [path.navigation.018] `$total` is only defined inside `aggregate`

- **WHEN** `$total` is used outside an `aggregate()` argument
- **THEN** evaluation fails

#### Scenario: [path.navigation.019] navigation over an empty focus yields empty

- **WHEN** `{}.name.given` is evaluated
- **THEN** the result is an empty collection, when the check is skipped
- **AND** the **check** rejects it: the empty literal has no type, so `name`
  resolves against no types and `executeType` raises `FHIRPATH_UNKNOWN_NAME`
  (`FHIRPathEngine.java:1590-1593`, with `canBeNone` false). A test must skip
  the check to grade the empty answer

#### Scenario: [path.navigation.020] `children()` returns the immediate children

- **WHEN** `Patient.children().count()` is evaluated over `patient-example.xml`
- **THEN** the count equals the number of child elements present on the resource

#### Scenario: [path.navigation.021] `descendants()` returns the transitive children

- **WHEN** `Questionnaire.descendants().linkId.isDistinct()`,
  `Questionnaire.descendants().linkId.distinct().count()`,
  `Questionnaire.descendants().code.count() = 23` and
  `concept.code.combine($this.descendants().concept.code).isDistinct()` are
  evaluated over their stated inputs
- **THEN** each yields its stated output (`testDistinct/testDistinct2`,
  `testDistinct3`, `testDistinct5`, `testDistinct6`, `testRepeat/testRepeat3`,
  `testCombine()/testCombine1`)
- **AND** the grading selector is those six cases; the `testDollarResource`
  group this scenario previously named is commented out of the pinned suite

#### Scenario: [path.navigation.022] `children()` produces an unordered collection

- **WHEN** `Patient.children().skip(1)` is checked
- **THEN** the static check fails as a semantic error, unconditionally — the
  rule is not gated on any engine setting, and the `mode="strict"` and
  `checkOrderedFunctions="true"` attributes the suite case carries are read by
  no R4 harness (`testDollar/testDollarOrderNotAllowed`)

#### Scenario: [path.navigation.023] navigation preserves document order across repeating elements

- **WHEN** `Patient.name.skip(1).given` is evaluated over `patient-example.xml`
- **THEN** the result is `Jim`, `Peter`, `James` in that order
  (`testDollar/testDollarOrderAllowed`)

#### Scenario: [path.navigation.024] the miscellaneous accessor cases all hold

- **WHEN** the `testMiscellaneousAccessorTests` (3 cases), `testBasics` (8) and
  `testObservations` (10) groups' cases are evaluated
- **THEN** every case HAPI passes yields its stated output: all 3, 7 of the 8
  and all 10
- **AND** the one exception is `testBasics/testEscapedIdentifierUnicode`, which
  HAPI fails at the check stage (`oracles/fhirpath/hapi-8.12.0-results.json:20`)
  and Frevo must fail the same way, as `path.navigation.001` states: its
  required status is `failed` (parity under `path.oracle.012`), and a Frevo pass
  would be listed `ahead-of-hapi`, not accepted as a pass of this scenario

#### Scenario: [path.navigation.025] `Narrative.div` navigates to a string holding the composed XHTML, and `children()` to an `xhtml`

- **WHEN** `Patient.text.div` is evaluated over a `Patient` whose `text.div` is
  `<div xmlns="http://www.w3.org/1999/xhtml">hi</div>`
- **THEN** the result is a single **`string`** (not `xhtml`) whose value is
  `<div xmlns="http://www.w3.org/1999/xhtml">hi</div>`: `Narrative.getProperty`
  answers `div` with `new StringType(new XhtmlComposer(true).composeEx(div))`
  (`Narrative.java:364-366`), the parsed node re-serialised in XML mode, not
  pretty (`XhtmlComposer.java:59-68`, `:105-116`, `:486-488`)
- **AND** with the static check skipped, as `FhirPathR4` runs it:
  - `Patient.text.div is string` is `true` and `Patient.text.div is xhtml` is
    `false`: the `is` operator reads its right operand as plain text
    (`executeTypeName`, `FHIRPathEngine.java:1531-1539`) and `opIs` compares it
    with `fhirType()` and then with `sd.getType()` along the base chain
    (`:1940-1965`);
  - so `Patient.text.div is FHIR.string` is **`false`**: the text
    `FHIR.string` equals neither `string` nor any `getType()` in the chain, and
    the item, built by `getProperty`, is not `disallowExtensions`, so the
    System branch does not apply;
  - the function form reads a namespace (`funcIs`, `:5095-5145`), so
    `Patient.text.div.is(string)` and `Patient.text.div.is(FHIR.string)` are
    `true` and `Patient.text.div.is(xhtml)` is `false`;
  - `Patient.text.div.ofType(string)` returns the item,
    `Patient.text.div.toString()` and
    `Patient.text.div = '<div xmlns="http://www.w3.org/1999/xhtml">hi</div>'`
    give that string and `true`, and `Patient.text.div.contains('>hi<')` is
    `true`, because at evaluation `contains` accepts the item's `string` type
- **AND** the static check rejects the string functions on `div`: it types
  `div` as `xhtml` from the element definition, and `checkContextString`
  (`:3496-3502`, `:3865-3869`) accepts only a focus type reached by
  `TypeDetails.hasType` from `string`, `code`, `uri`, `url`, `canonical` or `id`,
  which `xhtml` is not; the 12-type string-like set (`:2407`) is used only at
  evaluation. So checking `Patient.text.div.contains('>hi<')` against
  `Patient` fails with
  `Error evaluating FHIRPath expression: The function contains can only be used on string, uri, url, code, id but found [http://hl7.org/fhir/StructureDefinition/xhtml] (@char 18)`
  (`FHIRPATH_STRING_SING_ONLY`, `Messages.properties:363`; the holder is the
  `contains` node, column 18, byte offset 17; `TypeDetails.describe`,
  `TypeDetails.java:450-452`)
- **AND** the stored narrative is the `div` element's **markup**: in a JSON
  document the `div` string, and in an XML document the serialised `div`
  element with its descendants, not its text content
- **AND** the value is the composition, not the stored text: in XML mode an
  element with no children and no content is written self-closed and a `"` in
  text is written `&quot;` (`XhtmlComposer.java:176-230`, `:313-350`), so a
  stored `<div xmlns="http://www.w3.org/1999/xhtml">say "a"<br></br>b</div>`
  navigates to `<div xmlns="http://www.w3.org/1999/xhtml">say &quot;a&quot;<br/>b</div>`.
  Attributes are written in the iteration order of the node's
  `java.util.HashMap` (`XhtmlNode.java:89`, `:142`; `attributes`,
  `XhtmlComposer.java:306-311`), and the text and attributes are those
  HAPI's `XhtmlParser` keeps. No line break is inserted, because
  `breakBlocksWithLines` runs only in HTML mode (`XhtmlComposer.java:105-108`):
  a stored `<div xmlns="http://www.w3.org/1999/xhtml">x<ul><li>a</li><li>b</li></ul></div>`
  navigates to exactly that text, and reading `text.div` never edits the
  stored narrative
- **WHEN** `Patient.text.children()` is evaluated over the same `Patient`
- **THEN** the result is the `status` code and one item of type **`xhtml`**:
  `*` navigation collects `listChildren`'s property values, which hold
  `new XhtmlType(this)` for `div` (`Narrative.java:343`, `Base.java:226-236`,
  `FHIRPathEngine.java:4833`); `Patient.text.children().count()` is `2` and
  `Patient.text.children().ofType(xhtml).count()` is `1`. The same holds for
  `descendants()` (`:4819`)
- **AND** this scenario grades that item's **type and count only**. Its string
  value in Frevo is the same XML-mode composition `text.div` gives, and
  reading it never edits the stored narrative. HAPI differs:
  `XhtmlType.primitiveValue` composes in HTML mode (`XhtmlType.java:107-108`,
  `XhtmlComposer(false)`), which first edits the narrative's own node by
  inserting CR LF text nodes (`breakBlocksWithLines`, `XhtmlComposer.java:85-104`),
  and only when the item's string is read (`toString()`, a comparison, a string
  function, rendering the result), not when `children()` navigates
  (`XhtmlType.java:77-78`, `:107-108`). That difference is `DEV-0028`, and
  the HTML-mode value and the edit are owed as `DEBT-0018`
- **WHEN** the narrative has a `status` and **no** `div`
- **THEN** `Patient.text.div` is empty (`getProperty` returns no value for a
  null `div`, `Narrative.java:364-366`), but `Patient.text.children()` still
  yields the `status` code and one `xhtml` item, so `.count()` is `2`:
  `listChildren` adds `new XhtmlType(this)` unconditionally (`:343`)
- **AND** in HAPI, reading that item's string calls `getDiv()`, which creates
  an empty `div` on the narrative (`Narrative.java:316-323`, auto-create being
  `Configuration`'s default, `Configuration.java:85-98`) and composes it in HTML
  mode as `<div></div>`; a later `text.div` over that resource object is then
  `<div/>`. In Frevo the item's string is `<div/>`, the XML-mode composition of
  that empty `div`, and nothing is created, so `text.div` stays empty. Both are
  part of `DEV-0028` and `DEBT-0018`
- **WHEN** the stored narrative is markup the XHTML parser rejects
- **THEN** `Patient.text.div` fails with the model's error, surfaced as
  `path.errors.005` requires, and the `xhtml` item from `children()` reports
  the stored text unchanged. HAPI has no counterpart: its parser rejects such a
  resource when it is read, so no FHIRPath evaluation over it exists; whether
  Frevo admits such a resource at all is decided where resources are ingested
  (`add-store-crud`)
- **AND** no live case grades it, so it is differential-only

### Requirement: Collection predicates

The engine SHALL implement the collection predicates `FHIRPathEngine` implements, and SHALL treat an empty collection as the specification's identity for each of them.

#### Scenario: [path.collection.001] `empty()` is true for an empty collection

- **WHEN** `Patient.name.prefix.empty()` is evaluated over `patient-example.xml`
- **THEN** the result is `true`

#### Scenario: [path.collection.002] `exists()` is true for a non-empty collection

- **WHEN** the `testExists` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.collection.003] `exists(criteria)` filters before testing

- **WHEN** `Patient.name.exists(given = 'Jim')` is evaluated over
  `patient-example.xml`
- **THEN** the result is `true`

#### Scenario: [path.collection.004] `not()` inverts a singleton boolean

- **WHEN** `Patient.name.exists().not()` is evaluated over `patient-example.xml`
- **THEN** the result is `false`

#### Scenario: [path.collection.005] `not()` on an empty collection yields empty

- **WHEN** `{}.not()` is evaluated
- **THEN** the result is an empty collection

#### Scenario: [path.collection.006] `all(criteria)` is true when every item satisfies it

- **WHEN** the `testAll` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.collection.007] `all(criteria)` is true for an empty collection

- **WHEN** `Patient.name.prefix.all($this = 'x')` is evaluated over
  `patient-example.xml`
- **THEN** the result is `true`

#### Scenario: [path.collection.008] the boolean folds reduce a boolean collection

- **WHEN** the `testCollectionBoolean` group's cases, which exercise
  `allTrue()`, `anyTrue()`, `allFalse()` and `anyFalse()`, are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.collection.009] a boolean fold over a non-boolean item is an execution error

- **WHEN** `(true | 'foo').allTrue()` is evaluated
- **THEN** evaluation fails (`from-Zulip/from-zulip-2`)

#### Scenario: [path.collection.010] `subsetOf` compares by equality

- **WHEN** the `testSubSetOf` group's cases are evaluated
- **THEN** every case yields its stated output
- **AND** `{}.subsetOf(1 | 2)` **fails the static check**, naming `subsetOf`
  parameter 1: `case SubsetOf` (`FHIRPathEngine.java:3316`) types the argument
  against the focus's type set, an empty focus has no types, and
  `checkParamTypes` (`:3830`) raises `FHIRPATH_WRONG_PARAM_TYPE` for `integer`.
  Evaluation alone, with the check skipped, answers `true` (`funcSubsetOf`,
  `:5360`: an empty focus is a subset of anything). No suite case grades it, so
  it is differential-only

#### Scenario: [path.collection.011] `supersetOf` compares by equality

- **WHEN** the `testSuperSetOf` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.collection.012] `isDistinct` detects duplicates

- **WHEN** `Patient.name.given.isDistinct()` is evaluated over
  `patient-example.xml`, whose given names repeat
- **THEN** the result is `false`

#### Scenario: [path.collection.013] `distinct` removes duplicates and keeps the last occurrence of each

- **WHEN** the `testDistinct` group's cases are evaluated
- **THEN** every case yields its stated output
- **AND** `Patient.name.given.distinct()` over `patient-example.xml`, whose given
  names in document order are `Peter, James, Jim, Peter, James`, yields
  `Jim, Peter, James`: `funcDistinct` (`FHIRPathEngine.java:5659-5680`) drops
  item `i` when an equal item follows it (`for j = i+1`), so it keeps the
  **last** occurrence and the survivors stay in their original relative order
- **AND** when any two items cannot be compared (`doEquals` answers null),
  `distinct()` returns an **empty** collection rather than the items
  (`FHIRPathEngine.java:5668-5670`)
- **AND** no suite case grades the order or the null case (`testDistinct4` is
  `(1 | 2 | 3)`), so both are differential-only

#### Scenario: [path.collection.014] `count` returns the number of items

- **WHEN** the `testCount` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.collection.015] `count` of an empty collection is zero

- **WHEN** `Patient.name.prefix.count()` is evaluated over `patient-example.xml`
- **THEN** the result is the integer `0`

### Requirement: Filtering and projection

The engine SHALL implement `where`, `select`, `repeat` and `ofType`, flattening projections and dropping items whose criteria does not evaluate to true.

#### Scenario: [path.filter.001] `where` keeps items whose criteria is true

- **WHEN** the `testWhere` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.filter.002] `where` drops items whose criteria is empty

- **WHEN** `Patient.name.where(prefix = 'Dr')` is evaluated over
  `patient-example.xml`, whose names have no prefix
- **THEN** the result is an empty collection
- **AND** an operator that follows an **indexer on the first term** of an
  expression, argument or group is dropped by the pinned parser, so
  `Patient.name.where(given[1] = 'James')` is not a comparison: only a
  proximal parse reads trailing operators (`FHIRPathEngine.java:1199`), and it
  attaches them to `focus`, which the indexer has moved to the synthetic item
  node (`:1188-1197`, `:1203`, `:1206-1213`); `execute` applies an operation
  only on a proximal node (`:1508`). With the check skipped the criteria
  evaluates as `given[1]`, and `asBool` (`:6891-6892`) answers `True` for a
  single non-boolean item, so `name[0]` and `name[2]` (each `Peter, James`,
  second given `James`) are kept and `name[1]` (`Jim`, no second given, empty
  so `Null`) is dropped: the result is **two** names, as for `path.filter.003`.
  The check pass fails the expression, because `where` parameter 1 is a
  `string`, not a boolean
- **AND** the first-term cases are each observable: `name[0].given = 'Peter'`
  with `Patient` as the context yields the given names `Peter, James`, not a
  boolean (first term of the expression), and `(1 | 2)[0] = 5` yields the
  integer `1`, not `false` (the group's content and the indexed group are both
  proximal first terms, `:1135-1138`)
- **AND** `Patient.name.where($this.given[1] = 'James')` is a real comparison,
  because `$this` is the first term and the indexer is on a later one
- **AND** an indexer on a later term is not affected: `Patient.name[0].given =
  'Peter' | 'James'` (`testIndexer1`), `testIndexer2`, `testSort9` and
  `testQuantityUnitTextEqual` evaluate the operator and pass, as
  `path.navigation.008` relies on
- **AND** this is `DEV-0015`; no suite case grades the dropped form

#### Scenario: [path.filter.003] `where` with a non-boolean criteria is rejected by the check, before evaluation

- **WHEN** `Patient.name.where('x')` is checked and then evaluated
- **THEN** the **check** fails: `case Where` (`FHIRPathEngine.java:3330`) calls
  `checkParamTypes` against a singleton boolean and raises
  `FHIRPATH_WRONG_PARAM_TYPE` for a `string`
- **AND** the expression is not evaluated; evaluated with the check skipped, as
  `FhirPathR4` does (`path.check.013`), it does **not** fail: `asBool`
  (`:6887-6894`) answers `True` for any single non-boolean item, so all three
  names are kept
- **AND** no suite case grades either answer, so it is differential-only

#### Scenario: [path.filter.004] `select` projects each item and flattens the result

- **WHEN** the `testSelect` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.filter.005] `select` returning empty for an item contributes nothing

- **WHEN** `Patient.name.select(prefix)` is evaluated over `patient-example.xml`
- **THEN** the result is an empty collection

#### Scenario: [path.filter.006] `repeat` applies the projection transitively until nothing new appears

- **WHEN** the `testRepeat` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.filter.007] `repeat` terminates on a cyclic projection

- **WHEN** `Questionnaire.repeat(item)` is evaluated over
  `questionnaire-example.xml`, whose `item` elements nest
- **THEN** evaluation terminates and returns each distinct item, at every depth,
  once
- **AND** `repeat` does not seed its result with the focus (`funcRepeat`,
  `FHIRPathEngine.java:5261-5290`, starts with an empty `result`), so
  `Questionnaire.repeat(item).repeat(item)` returns only the items at depth two
  or deeper and its `count()` is **strictly smaller** than that of
  `Questionnaire.repeat(item)`; the expression
  `Questionnaire.repeat(item).count() = Questionnaire.repeat(item).repeat(item).count()`
  is therefore `false`

#### Scenario: [path.filter.008] `ofType` keeps only items of the named type

- **WHEN** `Patient.gender.ofType(code)` and `Patient.gender.ofType(string)` are
  evaluated over `patient-example.xml`
- **THEN** the first yields the `code` `male` and the second yields empty
  (`testInheritance/testFHIRPathAsFunction17`,
  `testInheritance/testFHIRPathAsFunction16`)

#### Scenario: [path.filter.009] `ofType` accepts a namespace-qualified type name

- **WHEN** `Patient.ofType(FHIR.Patient).type().name` is evaluated over
  `patient-example.xml`
- **THEN** the result is the string `Patient` (`testType` group)
- **AND** `Patient.ofType(System.Patient)` **raises** `FHIRPATH_INVALID_TYPE`
  rather than yielding empty: `funcOfType` (`FHIRPathEngine.java:5198-5207`)
  rejects a name for which `isKnownType` (`:1904`) is false, and the `System`
  namespace admits only `String`, `Boolean`, `Integer`, `Decimal`, `Quantity`,
  `DateTime`, `Time`, `SimpleTypeInfo` and `ClassInfo`. No suite case grades it,
  so it is differential-only

### Requirement: Subsetting

The engine SHALL implement the subsetting functions, and SHALL yield an empty collection rather than an error for a count beyond the end of the collection, while raising, as HAPI does, for a `skip` or `take` whose count is empty and for a `skip` whose count is negative (`DEV-0014`).

#### Scenario: [path.subset.001] `single` returns the only item

- **WHEN** the `testSingle` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.subset.002] `single` on a collection of more than one is an execution error

- **WHEN** `Patient.name.single()` is evaluated over `patient-example.xml`,
  which has three names
- **THEN** evaluation fails

#### Scenario: [path.subset.003] `single` on an empty collection is an execution error

- **WHEN** `Patient.name.prefix.single()` is evaluated over
  `patient-example.xml`, whose `name.prefix` is absent
- **THEN** evaluation fails, because `funcSingle` returns the focus only when
  its size is exactly 1 and throws `FHIRPATH_NO_COLLECTION` otherwise — size 0
  takes the throw
- **AND** this differs from the FHIRPath specification, which returns empty for
  an empty focus; the divergence is recorded as `DEV-0001` and resolved as
  HAPI's behaviour
- **AND** no live case grades it (`testSingle/testSingle2` covers only the
  more-than-one case), so it is differential-only

#### Scenario: [path.subset.004] `first` and `last` select the ends

- **WHEN** the `testFirstLast` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.subset.005] `tail` drops the first item

- **WHEN** the `testTail` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.subset.006] `skip` drops the first n items

- **WHEN** the `testSkip` group's cases are evaluated
- **THEN** every case yields its stated output, **except** `testSkip5`
  (`skip(-1)`), `testSkip7` (`(0 | 1 | 2).skip({})`) and `testSkip8`
  (`{}.skip({})`), which **raise**: `funcSkip` (`FHIRPathEngine.java:6101-6109`)
  reads `n1.get(0)` with no emptiness guard and starts its loop at `i1`, so an
  empty count and a negative count each reach an out-of-range `List.get`
- **AND** HAPI fails those three cases on its own suite
  (`oracles/fhirpath/hapi-8.12.0-results.json`, status `failed`); Frevo
  matches, so they are `failed` on both sides and are parity, not a gap. This is
  `DEV-0014`

#### Scenario: [path.subset.007] `skip` past the end yields empty

- **WHEN** `Patient.name.skip(3).given` is evaluated over `patient-example.xml`
- **THEN** the result is an empty collection
  (`testDollar/testDollarOrderAllowedA`)

#### Scenario: [path.subset.008] `take` keeps the first n items

- **WHEN** the `testTake` group's cases are evaluated
- **THEN** every case yields its stated output, **except** `testTake8`
  (`(0 | 1 | 2).take({})`) and `testTake9` (`{}.take({})`), which **raise**:
  `funcTake` (`FHIRPathEngine.java:5024-5032`) reads `n1.get(0)` with no
  emptiness guard
- **AND** HAPI fails those two cases on its own suite, Frevo matches, and they
  are parity (`DEV-0014`). A negative count does not raise: `take(-1)` is empty
  (`path.subset.009`)

#### Scenario: [path.subset.009] `take(0)` and `take(-1)` yield empty

- **WHEN** `Patient.name.take(0)` and `Patient.name.take(-1)` are evaluated
- **THEN** both yield an empty collection

#### Scenario: [path.subset.010] `intersect` keeps items present in both, without duplicates

- **WHEN** the `testIntersect` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.subset.011] `exclude` removes items present in the argument

- **WHEN** the `testExclude` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.subset.013] an unordered focus is rejected for all six position-dependent functions

- **WHEN** each of `item` (the indexer), `first`, `last`, `tail`, `skip` and
  `take` is checked against a focus whose collection status is unordered — the
  result of `children()`, say
- **THEN** every one of the six fails the static check with an ordered-only
  error naming the function
- **AND** the rule is unconditional: no engine setting disables it
- **AND** only `skip` is graded by a live case
  (`testDollar/testDollarOrderNotAllowed`); the other five are
  differential-only

#### Scenario: [path.subset.012] `exclude` keeps duplicates that are not excluded

- **WHEN** `1.combine(1).exclude(2).count()` is evaluated
- **THEN** the result is the integer `2` (`funcExclude`,
  `FHIRPathEngine.java:5074`, filters and never deduplicates)
- **AND** `(1 | 1 | 2).count()` is `2` and `(1 | 1 | 2).exclude(2).count()` is
  `1`, not `2`: the union operator removes the duplicate (`opUnion`, `:2831`)
  before `exclude` runs, as `testUnion/testUnion3` grades

### Requirement: Combining

The engine SHALL implement the union operator, `union`, `combine`, `in` and `contains` with the duplicate handling `FHIRPathEngine` applies.

#### Scenario: [path.combine.001] the union operator unions and removes duplicates

- **WHEN** the `testUnion` group's cases are evaluated
- **THEN** every case HAPI passes yields its stated output
- **AND** Frevo's required status per case follows HAPI's own recorded run (`oracles/fhirpath/hapi-8.12.0-results.json`): `passed` for every case HAPI passes, and `failed` (parity under `path.oracle.012`) for the cases named below, which HAPI fails. A Frevo pass on one of them would be listed as `ahead-of-hapi`, not accepted as a pass of this scenario
- **AND** the failed case is `testUnion12` (`(@2015-02-04T10:00 |
  @2015-02-04T10:00).count()`), for the minute-precision reason in
  `path.equality.001`

#### Scenario: [path.combine.002] `union()` behaves as the union operator

- **WHEN** `(1 | 2).union(2 | 3).count()` is evaluated
- **THEN** the result is the integer `3`

#### Scenario: [path.combine.003] `combine` concatenates and keeps duplicates

- **WHEN** the `testCombine()` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.combine.004] `in` tests membership

- **WHEN** the `testIn` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.combine.005] `contains` tests membership the other way round

- **WHEN** the `testContainsCollection` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.combine.006] `in` with an empty left operand yields empty, and with an empty right operand yields false

- **WHEN** `{} in (1 | 2)` is evaluated
- **THEN** the result is an empty collection
- **AND** `1 in {}` is the boolean `false`, not empty: `opIn`
  (`FHIRPathEngine.java:2630-2636`) tests the left operand's size first and
  the right operand's second. `contains` is empty for an empty operand on
  either side

### Requirement: Equality and equivalence

The engine SHALL implement equality, inequality, equivalence and non-equivalence as `FHIRPathEngine` does, including the cases where equality yields an empty collection and equivalence yields a boolean.

#### Scenario: [path.equality.001] the equality operator compares values of the same type

- **WHEN** the `testEquality` group's cases are evaluated
- **THEN** every case HAPI passes yields its stated output
- **AND** Frevo's required status per case follows HAPI's own recorded run (`oracles/fhirpath/hapi-8.12.0-results.json`): `passed` for every case HAPI passes, and `failed` (parity under `path.oracle.012`) for the cases named below, which HAPI fails. A Frevo pass on one of them would be listed as `ahead-of-hapi`, not accepted as a pass of this scenario
- **AND** the failed cases are `testEquality31`, `32`, `33` and `35` (`100 = 1.0`,
  `10 = 1.0`, `10 = 10.0`, `100.toDecimal() = 1.0`): `removeTrailingZeros`
  (`FHIRPathEngine.java:2161-2182`) strips zeros from a number written with no
  decimal point, so `100 = 1.0` is `true` and `10 = 10.0` is `false`; and
  `testEquality37`, `38`, `39` and `40`, where a minute-precision `dateTime`
  is rejected by `DateTimeType.copy()` (`DateTimeType.java:164-165`, called at
  `BaseDateTimeType.java:924`) and evaluation fails. With no offset the message
  is `Invalid date/time format` (`BaseDateTimeType.java:797`); with an offset
  (`testEquality40`) parsing succeeds and the precision check raises `Invalid
  date/time string (datatype DateTimeType does not support MINUTE precision)`
  (`BaseDateTimeType.java:103-106`)
- **AND** the FHIRPath specification requires `100 = 1.0` to be `false` and
  `10 = 10.0` to be `true`; this is recorded as `DEV-0017` and resolved as
  HAPI's behaviour. Equality of a minute-precision `dateTime` is recorded in the
  same entry

#### Scenario: [path.equality.002] the inequality operator is the negation of equality

- **WHEN** the `testNEquality` group's cases are evaluated
- **THEN** every case HAPI passes yields its stated output
- **AND** Frevo's required status per case follows HAPI's own recorded run (`oracles/fhirpath/hapi-8.12.0-results.json`): `passed` for every case HAPI passes, and `failed` (parity under `path.oracle.012`) for the cases named below, which HAPI fails. A Frevo pass on one of them would be listed as `ahead-of-hapi`, not accepted as a pass of this scenario
- **AND** the failed case is `testNEquality27` (`@2015-02-04T10:00 !=
  @2015-02-04T11:00`), for the minute-precision reason in `path.equality.001`

#### Scenario: [path.equality.003] equality with an empty operand yields empty

- **WHEN** `{} = 1` and `1 = {}` are evaluated
- **THEN** both yield an empty collection

#### Scenario: [path.equality.004] the equivalence operator ignores case and whitespace for strings

- **WHEN** the `testEquivalent` group's cases are evaluated
- **THEN** every case HAPI passes yields its stated output
- **AND** Frevo's required status per case follows HAPI's own recorded run (`oracles/fhirpath/hapi-8.12.0-results.json`): `passed` for every case HAPI passes, and `failed` (parity under `path.oracle.012`) for the cases named below, which HAPI fails. A Frevo pass on one of them would be listed as `ahead-of-hapi`, not accepted as a pass of this scenario
- **AND** the failed cases are `testEquivalent24` to `29` (`1 year ~ 1 'a'`,
  `2 years ~ 2 'a'`, `1 month ~ 1 'mo'`, `1 year ~ 1 day`, `1 'x' ~ 1 year`,
  `1 year ~ 1 'kg'`): `qtyEquivalent` returns `null` and `doEquivalent` unboxes
  it (`doEquivalent`, `FHIRPathEngine.java:2204-2206`), so HAPI raises a `NullPointerException` and
  evaluation fails; and `testEquivalent30` (`@2015-02-04T10:00 ~
  @2015-02-04T10:00`), for the minute-precision reason in `path.equality.001`.
  Frevo raises an execution error in each (an error, not a panic:
  `path.errors.003`)
- **AND** the specification makes these `true` or `false`; recorded as `DEV-0017`

#### Scenario: [path.equality.005] the non-equivalence operator is the negation of equivalence

- **WHEN** the `testNotEquivalent` group's cases are evaluated
- **THEN** every case HAPI passes yields its stated output
- **AND** Frevo's required status per case follows HAPI's own recorded run (`oracles/fhirpath/hapi-8.12.0-results.json`): `passed` for every case HAPI passes, and `failed` (parity under `path.oracle.012`) for the cases named below, which HAPI fails. A Frevo pass on one of them would be listed as `ahead-of-hapi`, not accepted as a pass of this scenario
- **AND** the failed cases are `testNotEquivalent23` (`1 year !~ 1 day`) and
  `24` (`1 month !~ 1 'mo'`), for the `qtyEquivalent` reason in `path.equality.004`

#### Scenario: [path.equality.006] equivalence with an empty operand yields a boolean, never empty

- **WHEN** `{} ~ {}` and `{} ~ 1` are evaluated
- **THEN** the results are `true` and `false`

#### Scenario: [path.equality.007] equality on collections compares item by item in order

- **WHEN** `(1 | 2 | 3) = (1 | 2 | 3)` and `(1 | 2 | 3) = (3 | 2 | 1)` are
  evaluated
- **THEN** the results are `true` and `false`

#### Scenario: [path.equality.008] equality on decimals compares by value, not by lexical form

- **WHEN** `1.0 = 1` and `1.10 = 1.1` are evaluated
- **THEN** both yield `true`

#### Scenario: [path.equality.009] equality on complex elements compares every child

- **WHEN** `Patient.name[0] = Patient.name[0]` and
  `Patient.name[0] = Patient.name[1]` are evaluated over `patient-example.xml`
- **THEN** the results are `true` and `false`

#### Scenario: [path.equality.010] equality on dates of differing precision yields empty where undecidable

- **WHEN** `@2014 = @2014-01` is evaluated
- **THEN** the result is an empty collection

#### Scenario: [path.equality.011] equivalence on dates of differing precision compares at the lower precision

- **WHEN** `@2014 ~ @2014-01` is evaluated
- **THEN** the result is the boolean the engine at the pinned SHA computes

#### Scenario: [path.equality.012] `hasValue()` distinguishes a primitive with a value from one without

- **WHEN** the `testHasValue` group's cases are evaluated
- **THEN** every case HAPI passes yields its stated output
- **AND** Frevo's required status per case follows HAPI's own recorded run (`oracles/fhirpath/hapi-8.12.0-results.json`): `passed` for every case HAPI passes, and `failed` (parity under `path.oracle.012`) for the cases named below, which HAPI fails. A Frevo pass on one of them would be listed as `ahead-of-hapi`, not accepted as a pass of this scenario
- **AND** the failed case is `testHasCodingValue2` (`name.first().hasValue()`,
  expected `false`): `funcHasValue` is `!noString(convertToString(item))` and a
  `HumanName`'s string is its `toString()`, so HAPI answers `true`. Frevo
  answers `true`; the specification says a complex element has no value;
  recorded as `DEV-0017`

### Requirement: Ordered comparison

The engine SHALL implement the four ordered comparison operators as `FHIRPathEngine` does, yielding an empty collection where the comparison is undecidable and failing where the operands are not comparable.

#### Scenario: [path.compare.001] the less-than operator accepts a narrower numeric type set than the other three

- **WHEN** the `testLessThan` group's cases are evaluated
- **THEN** every case HAPI passes yields its stated output
- **AND** Frevo's required status per case follows HAPI's own recorded run (`oracles/fhirpath/hapi-8.12.0-results.json`): `passed` for every case HAPI passes, and `failed` (parity under `path.oracle.012`) for the cases named below, which HAPI fails. A Frevo pass on one of them would be listed as `ahead-of-hapi`, not accepted as a pass of this scenario
- **AND** the failed cases, each with its outcome, are:
  - `testLessThan28`: the comparison is `true` but the second conjunct, the
    operand's `toString()`, is `false` because the comparison has rewritten the
    operand's stored string to its Zulu form (`path.compare.013`); the suite
    expects `true`
  - `testLessThan29` (`1 year < 2 months`): a calendar-duration literal sets
    `unit` and no `code` (`FHIRPathEngine.java:1131`), so both `code` lists are
    empty and compare equal (`:2438-2440`) and the raw values are compared:
    `1 < 2` is `true`, one item where the suite expects none
  - `testLessThan30` (`1 'kg' < 2 'm'`): the `code` lists differ, so the
    canonical decimals are compared (`:2447-2450`): one item, `false`, where the
    suite expects none
  - `testLessThan33`: the quantity with only unit text has no code and the other
    has one, so the lists differ; `qtyToCanonicalDecimal` returns `null` for the
    quantity with no UCUM system (`:2301-2304`) and the recursive `opLessThan`
    calls `isPrimitive()` on it (`:2414`): a `NullPointerException`, so
    evaluation fails
- **AND** the operator compares the 12 string-like types
  (`string`, `uri`, `code`, `oid`, `id`, `uuid`, `sid`, `markdown`,
  `base64Binary`, `canonical`, `url`, `xhtml`), `integer` and `decimal` only,
  `date`/`dateTime`/`instant`, `time`, and `Quantity` — and **not**
  `unsignedInt` or `positiveInt`

#### Scenario: [path.compare.002] the less-or-equal operator accepts the extended numeric type set

- **WHEN** the `testLessOrEqual` group's cases are evaluated
- **THEN** every case HAPI passes yields its stated output
- **AND** Frevo's required status per case follows HAPI's own recorded run (`oracles/fhirpath/hapi-8.12.0-results.json`): `passed` for every case HAPI passes, and `failed` (parity under `path.oracle.012`) for the cases named below, which HAPI fails. A Frevo pass on one of them would be listed as `ahead-of-hapi`, not accepted as a pass of this scenario
- **AND** the failed cases are `testLessOrEqual28` (`1 year <= 2 days`), which
  fails with a `NullPointerException`: the units differ, so
  `qtyToCanonicalDecimal` returns `null` for the calendar duration
  (`FHIRPathEngine.java:2301-2304`) and `isPrimitive()` is called on it
  (`:2544-2546`); and `testLessOrEqual29` (`1 'kg' <= 999 'g'`), which yields one
  item, `true`, where the suite expects `false`: a UCUM-quoted literal sets
  `code` and `system` and leaves `unit` null (`:1131`), `<=` compares `unit`
  (`:2533-2537`), null against null is equal, and the raw values are compared
- **AND** `<=` with a null left `unit` and a non-null right `unit`
  (`1 'kg' <= 1 year`) fails with a `NullPointerException` on `lunit.equals`
  (`:2537`); see `path.compare.014`
- **AND** the operator's numeric branch accepts `integer`, `decimal`,
  `unsignedInt` and `positiveInt`

#### Scenario: [path.compare.003] the greater-than operator accepts the extended numeric type set

- **WHEN** the `testGreaterThan` group's cases are evaluated
- **THEN** every case HAPI passes yields its stated output
- **AND** Frevo's required status per case follows HAPI's own recorded run (`oracles/fhirpath/hapi-8.12.0-results.json`): `passed` for every case HAPI passes, and `failed` (parity under `path.oracle.012`) for the cases named below, which HAPI fails. A Frevo pass on one of them would be listed as `ahead-of-hapi`, not accepted as a pass of this scenario
- **AND** the failed cases are `testGreaterThan28` (`1 year > 1 'a'`) and `29`
  (`1 year > 2 months`), both of which fail with a `NullPointerException`: `>`
  compares `unit` (`FHIRPathEngine.java:2485-2487`), the units differ,
  `qtyToCanonicalDecimal` returns `null` for the calendar duration
  (`:2301-2304`) and `isPrimitive()` is called on it (`:2494-2496`)
- **AND** the operator's numeric branch accepts `integer`, `decimal`,
  `unsignedInt` and `positiveInt`

#### Scenario: [path.compare.004] the greater-or-equal operator accepts the extended numeric type set

- **WHEN** the `testGreatorOrEqual` group's cases are evaluated
- **THEN** every case HAPI passes yields its stated output
- **AND** Frevo's required status per case follows HAPI's own recorded run (`oracles/fhirpath/hapi-8.12.0-results.json`): `passed` for every case HAPI passes, and `failed` (parity under `path.oracle.012`) for the cases named below, which HAPI fails. A Frevo pass on one of them would be listed as `ahead-of-hapi`, not accepted as a pass of this scenario
- **AND** the failed cases are `testGreatorOrEqual28` (`2 years >= 2 year`),
  which fails with a `NullPointerException` (the units differ and the calendar
  duration has no UCUM system, `FHIRPathEngine.java:2301-2304`, `2583-2594`); and
  `testGreatorOrEqual29` (`2 'kg' >= 1 'm'`), which yields one item, `true`,
  where the suite expects none: both `unit`s are null, so the raw values are
  compared (`:1131`, `:2583-2585`)
- **AND** the operator's numeric branch accepts `integer`, `decimal`,
  `unsignedInt` and `positiveInt`

#### Scenario: [path.compare.005] comparing different types is an execution error

- **WHEN** `1 < 'a'` is evaluated
- **THEN** evaluation fails

#### Scenario: [path.compare.006] comparison with an empty operand yields empty

- **WHEN** `{} < 1` is evaluated
- **THEN** the result is an empty collection

#### Scenario: [path.compare.007] comparison of a collection of more than one yields empty

- **WHEN** `(1 | 2) < 3`, `(1 | 2) <= 3`, `(1 | 2) > 3` and `(1 | 2) >= 3` are
  evaluated
- **THEN** each yields an empty collection, because the operator handles only
  the empty-operand case and the singleton-and-singleton cases and otherwise
  returns an empty list
- **AND** this differs from the FHIRPath specification, which makes it an
  error; the divergence is recorded as `DEV-0002` and resolved as HAPI's
  behaviour
- **AND** no live case in the pinned suite grades it, so it is
  differential-only

#### Scenario: [path.compare.008] strings compare by code point

- **WHEN** `'a' < 'b'` and `'A' < 'a'` are evaluated
- **THEN** both yield `true`

#### Scenario: [path.compare.009] dates of insufficient precision compare to empty

- **WHEN** `@2014 < @2014-01-05` is evaluated
- **THEN** the result is an empty collection

#### Scenario: [path.compare.011] a `positiveInt` or `unsignedInt` operand compares under three operators and fails under `<`

- **WHEN** two `positiveInt` values — for example `Timing.repeat.count` — are
  compared with `<`, and then with `<=`, `>` and `>=`
- **THEN** `<` fails with a cannot-compare error naming both FHIR types, while
  the other three return a boolean
- **AND** the same holds for `unsignedInt`, for example `Media.height`
- **AND** this asymmetry is upstream's, not a specification requirement; it is
  recorded as `DEV-0003` and resolved as HAPI's behaviour
- **AND** no live case grades it, so it is differential-only

#### Scenario: [path.compare.012] numeric comparison goes through binary floating point

- **WHEN** two decimals that differ only beyond the precision an IEEE-754
  double can represent are compared with `<`, `<=`, `>` or `>=`
- **THEN** the result is what the comparison of their `double` values gives,
  because all four operators compare
  `Double.valueOf(left) OP Double.valueOf(right)`
- **AND** the same applies to `sqrt`, `abs`, `ceiling`, `floor`, `exp`, `ln`,
  `log` and `power`, which parse the focus as a `double`, and to `sort`'s
  numeric comparator
- **AND** it does **not** apply to `+`, `-`, `*`, `/`, `div`, `mod`, `round` or
  `truncate`, which use exact decimal arithmetic
- **AND** this differs from the FHIRPath specification, which requires decimal
  semantics throughout; the divergence is recorded as `DEV-0004` and resolved as
  HAPI's behaviour

#### Scenario: [path.compare.013] an ordered comparison of a dateTime rewrites the operand's stored string to Zulu

- **WHEN** `Patient.birthDate.extension('http://hl7.org/fhir/StructureDefinition/patient-birthTime').value < @1975-01-01T00:00:00Z`
  is evaluated and the same element's string is read afterwards in the same
  evaluation (`testLessThan28`)
- **THEN** the comparison is `true`, and the element's `toString()` is then its
  Zulu form rather than `1974-12-25T14:35:45-05:00`
- **AND** the mechanism is `compareDateTimeElements` calling
  `setTimeZoneZulu(true)` on the operand itself when it is already a
  `DateTimeType` and its precision is finer than a day
  (`FHIRPathEngine.java:636-648`), which rewrites the stored string
  (`BaseDateTimeType.java:690-694`); `<`, `>`, `<=` and `>=` all reach it
- **AND** the specification does not describe the mutation; HAPI's recorded run
  is `failed` for `testLessThan28`, and Frevo matches it as parity
  (`path.oracle.012`)
- **AND** the evaluation's **result** carries the rewrite when the compared
  element is what it returns:
  `Patient.birthDate.extension('http://hl7.org/fhir/StructureDefinition/patient-birthTime').value.where($this < @1975-01-01T00:00:00Z)`
  over `patient-example.xml` returns one `dateTime` whose value is
  `1974-12-25T19:35:45Z`, not `1974-12-25T14:35:45-05:00`. `funcWhere` adds
  the same `item` object it compared (`FHIRPathEngine.java:6249-6260`), which
  `compareDateTimeElements` has already set to Zulu; `select($this)` after a
  comparison returns the rewritten object the same way (`funcSelect`,
  `:6263-6273`), so
  `...value.where($this < @1975-01-01T00:00:00Z).select($this)` returns
  `1974-12-25T19:35:45Z` too. The returned items are part of the evaluation,
  so a rewrite that is dropped before the result is produced does not meet
  this scenario
- **AND** a `dateTime` **literal** operand is rewritten the same way, within
  the evaluation that compared it; `path.compare.015` states what that is
  observable through and why it does not outlive the evaluation
- **AND** this scenario grades the rewrite **within one evaluation** only.
  Upstream the operand is the caller's own in-memory element object, so the
  Zulu string also remains on that resource after the evaluation returns;
  whether Frevo must reproduce that past the end of an evaluation is not decided
  by this scenario (`tasks/blockers.md`, 2026-11-20, "element rewrite after the
  evaluation"), and no scenario grades it until the owner rules
- **AND** returning the Zulu string of a value only because it was compared is a
  wrong answer, not an error; the flag that would restore the operand's own
  string is `DEBT-0017` (origin `DEV-0017`)

#### Scenario: [path.compare.015] an ordered comparison rewrites a `dateTime` literal for the rest of that evaluation, including its result, and only for the value it produced

- **WHEN** `(@2012-04-15T10:00:00+02:00).select($this < @2013-01-01T00:00:00Z and $this.toString() = '2012-04-15T08:00:00Z')`
  is evaluated
- **THEN** the result is the single boolean `true`: the literal is the focus of
  `select`, `$this` is that same value (`funcSelect`, `changeThis`,
  `FHIRPathEngine.java:6263-6273`, `:4726-4734`), the comparison sets it to Zulu
  (`compareDateTimeElements`, `:635-648`; `setTimeZoneZulu`,
  `BaseDateTimeType.java:690-694`), and `and` evaluates its right operand after
  its left (`:1508-1525`; `preOperate`, `:1542`, short-circuits only a `false` left), so `toString()` reads `2012-04-15T08:00:00Z`
- **AND** `(@2012-04-15T10:00:00+02:00).select(($this < @2013-01-01T00:00:00Z) and $this.toString() = '2012-04-15T10:00:00+02:00')`
  is the single boolean `false`
- **AND** the evaluation's **result** carries the rewrite when the compared
  literal's value is what it returns:
  `(@2012-04-15T10:00:00+02:00).where($this < @2013-01-01T00:00:00Z)` returns
  one `dateTime` whose value is `2012-04-15T08:00:00Z`, because `funcWhere`
  returns the object it compared (`:6249-6260`), and
  `(@2012-04-15T10:00:00+02:00).where($this < @2013-01-01T00:00:00Z).select($this)`
  returns the same rewritten value. An uncompared literal is returned as
  written: `(@2012-04-15T10:00:00+02:00).select($this)` returns
  `2012-04-15T10:00:00+02:00`
- **AND** the rewrite is not limited to elements and `@` literals: it applies to
  **any** `DateTimeType` object finer than a day that reaches an ordered
  comparison, including the values the evaluation builds itself, because
  `compareDateTimeElements` writes to whichever object it is given. So
  `now().where($this > @2000-01-01T00:00:00Z)` returns the current instant
  rendered in Zulu (`DateTimeType.now()`, `:4772-4776`, which otherwise renders
  in the JVM zone);
  `(@2012-04-15T10:00:00+02:00 + 1 day).where($this < @2013-01-01T00:00:00Z)`
  returns `2012-04-16T08:00:00Z` (`dateAdd` returns a `DateTimeType` copy,
  `:2706-2708`, `:2715-2716`); and
  `@2012-04-15T10:00:00+02:00.lowBoundary().where($this < @2013-01-01T00:00:00Z)`
  returns `2012-04-15T08:00:00.000Z`
- **AND** for a boundary, what decides is the precision of the **built** value,
  not the type of the focus. `lowBoundary` and `highBoundary` build a
  `DateTimeType` for a `date` focus as well as a `dateTime` one (`:4317-4320`,
  `:4361-4364`), and `DateTimeUtil.lowBoundaryForDate` /
  `highBoundaryForDate` (`DateTimeUtil.java:65-90`, `applyDatePrecision`
  `:130-144`) keep 4, 7 or 10 characters with no offset at precision 4, 6/7 and
  8/10, which is year, month or day precision and is never rewritten, and at
  precision 14 or 17 a time component with an offset (`+14:00` for the low
  boundary and `-12:00` for the high one when the focus has none), which is
  finer than a day and is rewritten. So
  `@2014-01-01.lowBoundary(17).where($this < @2015-01-01T00:00:00Z)` returns
  `2013-12-31T10:00:00.000Z` (from `2014-01-01T00:00:00.000+14:00`), while
  `@2014-01-01T08.lowBoundary(8).where($this < @2015-01-01T00:00:00Z)` returns
  `2014-01-01`, unchanged
- **AND** a boundary at precision 14 is the malformed text of
  `path.boundary.015` (17 characters, then the offset), which
  `BaseDateTimeType.parse` reads with the offset found at index 17
  (`getOffsetIndex`, `BaseDateTimeType.java:330-342`; `:470-480`), the
  seconds taken from the first two characters of the offset
  (`substring(17, 19)`, `:491`) at SECOND precision, and the zone set from the
  offset (`:479`); the comparison then rewrites that value to Zulu. So, each
  followed by `.where($this < @2015-01-01T00:00:00Z)`:
  - `@2014-01-01T08.lowBoundary(14)` (text `2014-01-01T08:00:+14:00`, seconds
    `+1`) returns `2013-12-31T18:00:01Z`;
  - `@2014-01-01T08:05+08:00.lowBoundary(14)` (text
    `2014-01-01T08:05:+08:00`, seconds `+0`) returns `2014-01-01T00:05:00Z`;
  - `@2014-01-01.lowBoundary(14)` (a `date` focus, text
    `2014-01-01T00:00:+14:00`) returns `2013-12-31T10:00:01Z`;
  - `@2014-01-01T08:05-05:00.highBoundary(14)` (text
    `2014-01-01T08:05:-05:00`, seconds `-0`) returns `2014-01-01T13:05:00Z`;
  - a `highBoundary(14)` whose text reads seconds `-1` (no offset on the focus,
    so `-12:00`, as `@2014-01-01T08.highBoundary(14)`) fails while the boundary
    is built, with the `Invalid date/time format` error of
    `path.boundary.015`, before any comparison is reached
- **AND** a `date + quantity` result and `today()` are `DateType` values and
  are never rewritten. `toDateTime()` builds no value: the R4 engine
  does not implement it (`DEV-0021`)
- **AND** an offset-less literal is placed in the evaluation zone first
  (`path.datetime.004`): at the suite's zone, offset 0,
  `(@2012-04-15T10:00:00).select($this < @2013-01-01T00:00:00Z and $this.toString() = '2012-04-15T10:00:00Z')`
  is `true`
- **AND** a literal whose precision is a day or coarser (`@2012-04-15`, a
  `DateType`) is never rewritten, and neither is an `instant`, because neither
  is a `DateTimeType` and `compareDateTimeElements` compares a copy
  (`:636-637`)
- **AND** a **second occurrence** of the same literal text is a separate value
  and is not rewritten:
  `(@2012-04-15T10:00:00+02:00 < @2013-01-01T00:00:00Z) and @2012-04-15T10:00:00+02:00.toString() = '2012-04-15T10:00:00+02:00'`
  is `true` (the lexer ends the literal before `.toString`: a `.` is part of a
  date token only at the millisecond position, `FHIRLexer.java:335-338`). A
  `@` literal is held in the parsed expression as its text
  (`processConstant` returns a `FHIRConstant`, `:1335-1336`), and every
  evaluation of the node builds a new value from that text (`resolveConstant`,
  `processDateConstant`, `:1653-1672`, `:1674-1736`)
- **AND** therefore the rewrite does **not** persist across evaluations of one
  parsed expression:
  `(@2012-04-15T10:00:00+02:00).select($this.toString() = '2012-04-15T10:00:00+02:00' and $this < @2013-01-01T00:00:00Z)`,
  parsed once and evaluated twice, is `true` on both evaluations (the string is
  read before the comparison, and the second evaluation starts from the
  literal's text again); a rewrite carried over from the first evaluation would
  make the second one `false`
- **AND** no live case grades it, so it is differential-only. The
  specification does not describe the mutation (`DEV-0017`); the wrong-answer
  hazard is `DEBT-0017`

#### Scenario: [path.compare.014] `<=` with a unit-less left quantity and a unit-bearing right quantity fails

- **WHEN** `1 'kg' <= 1 year` is evaluated
- **THEN** evaluation fails: `opLessOrEqual` evaluates `lunit.equals(runit)`
  with a null `lunit` (`FHIRPathEngine.java:2537`), a `NullPointerException` in
  HAPI and an execution error in Frevo (`path.errors.003`)
- **AND** no live case grades it, so it is differential-only

#### Scenario: [path.compare.010] a Period invariant written with raw comparison and one written with boundaries differ

- **WHEN** `Patient.identifier.period.all(start.hasValue().not() or end.hasValue().not() or (start <= end))`
  and
  `Patient.identifier.period.all(start.empty() or end.empty() or (start.lowBoundary() < end.highBoundary()))`
  are evaluated over `patient-example-period.xml`
- **THEN** the results are `false` and `true`
  (`period/testPeriodInvariantOld`, `period/testPeriodInvariantNew`)

### Requirement: Boolean logic

The engine SHALL implement `and`, `or`, `xor`, `implies`, `not` and `iif` over three-valued logic, with the singleton-evaluation rule `FHIRPathEngine` applies to a non-boolean operand.

Three-valued logic: an operand may be true, false or empty.

#### Scenario: [path.logic.001] `and` follows the engine's truth table including empty

- **WHEN** the `testBooleanLogicAnd` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.logic.002] `or` follows the engine's truth table including empty

- **WHEN** the `testBooleanLogicOr` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.logic.003] `xor` follows the engine's truth table including empty

- **WHEN** the `testBooleanLogicXOr` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.logic.004] `implies` follows the engine's truth table including empty

- **WHEN** the `testBooleanImplies` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.logic.005] a singleton non-boolean operand of `and` is treated as true

- **WHEN** `(true and 'foo').empty()` is evaluated
- **THEN** the result is `false`, because singleton evaluation of collections
  makes `'foo'` true (`from-Zulip/from-zulip-1`)

#### Scenario: [path.logic.006] `and` short-circuits on a false left operand

- **WHEN** `false and (1 < 'a')` is evaluated, whose right operand alone fails
  with a cannot-compare error
- **THEN** the result is `false` and no error is raised, because the engine
  decides `and` from a `false` left operand before evaluating the right one

#### Scenario: [path.logic.007] `or` and `implies` short-circuit, and `xor` does not

- **WHEN** `true or (1 < 'a')` and `false implies (1 < 'a')` are evaluated
- **THEN** both return `true` and raise no error
- **WHEN** `false xor (1 < 'a')` is evaluated
- **THEN** evaluation fails, because the engine short-circuits only `and`, `or`
  and `implies` and evaluates both operands of `xor`

#### Scenario: [path.logic.008] `iif` selects a branch and does not evaluate the other

- **WHEN** the `testIif` group's cases are evaluated, each over its input
  resource
- **THEN** every case yields its stated output (HAPI passes all 11)
- **AND** the static check of `iif` needs a focus type: `checkSingleton`
  dereferences it (`FHIRPathEngine.java:3441-3443`, `3839-3840`), so `iif(...)`
  with **no input resource** makes the check raise a `NullPointerException`
  rather than pass. A test of `iif` without an input must therefore evaluate
  with the check skipped, as `FhirPathR4` does (`path.check.013`), or supply an
  input; Frevo's check reports an error for the input-less form

#### Scenario: [path.logic.009] `iif` with an empty condition yields the else branch

- **WHEN** `iif({}, 1, 2)` is evaluated, over an input resource or with the
  check skipped (`path.logic.008`)
- **THEN** the result is the integer `2`

#### Scenario: [path.logic.010] `iif` with no else branch and a false condition yields empty

- **WHEN** `iif(false, 1)` is evaluated, over an input resource or with the
  check skipped (`path.logic.008`)
- **THEN** the result is an empty collection

### Requirement: Arithmetic

The engine SHALL implement the arithmetic operators over integers, decimals, strings, quantities, dates and dateTimes as `FHIRPathEngine` does. The operators here use exact decimal arithmetic; the comparison operators and the transcendental math functions do not, and `path.compare.012` states where that boundary falls.

#### Scenario: [path.math.001] addition works on integers, decimals, strings and quantities

- **WHEN** the `testPlus` group's cases are evaluated
- **THEN** every case HAPI passes yields its stated output
- **AND** Frevo's required status per case follows HAPI's own recorded run (`oracles/fhirpath/hapi-8.12.0-results.json`): `passed` for every case HAPI passes, and `failed` (parity under `path.oracle.012`) for the cases named below, which HAPI fails. A Frevo pass on one of them would be listed as `ahead-of-hapi`, not accepted as a pass of this scenario
- **AND** the failed cases are `testPlusDate23` (`@2015-02-04T10:00Z + 1 hour =
  @2015-02-04T11:00Z`) and `testPlusDate24` (`@2015-02-04T10Z + 1 hour = ...`):
  `dateAdd` calls `DateTimeType.copy()` (`FHIRPathEngine.java:2716`), which
  rejects a minute- or hour-precision `dateTime`; these carry an offset (`Z`),
  so the message is `Invalid date/time string (datatype DateTimeType does not
  support MINUTE precision)` (`BaseDateTimeType.java:103-106`) rather than
  `Invalid date/time format`, so evaluation fails.
  Recorded as `DEV-0017`

#### Scenario: [path.math.002] the concatenation operator joins strings and treats empty as the empty string

- **WHEN** the `testConcatenate` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.math.003] subtraction works on integers, decimals and quantities

- **WHEN** the `testMinus` group's cases are evaluated
- **THEN** every case HAPI passes yields its stated output
- **AND** Frevo's required status per case follows HAPI's own recorded run (`oracles/fhirpath/hapi-8.12.0-results.json`): `passed` for every case HAPI passes, and `failed` (parity under `path.oracle.012`) for the cases named below, which HAPI fails. A Frevo pass on one of them would be listed as `ahead-of-hapi`, not accepted as a pass of this scenario
- **AND** the failed case is `testMinus7` (`@2015-02-04T10:00Z - 1 day =
  @2015-02-03T10:00Z`), for the reason in `path.math.001`
- **AND** a `Quantity` on the right of `-` is accepted only when the left is a
  date, dateTime, instant, `Quantity`, or a number whose string value is exactly
  `0` (`FHIRPathEngine.java:2953`); `5 - 3 'mg'` fails with a wrong-type error
- **AND** when the left is the number `0` the result is the right quantity with
  its value made **absolute**, not negated: `0 - 5 'mg'` and `0 - (-5 'mg')` both
  yield `5 'mg'` (`:2965-2970`); `0.0 - 5 'mg'` fails the guard at `:2953`, as
  its string value is not `0`, and raises the right-value wrong-type error
  (`:2954`), as `5 - 3 'mg'` does
- **AND** when the left is a `Quantity` (`q - q`, for example `5 'mg' - 3 'mg'`)
  the branch adds nothing and the result is an **empty** collection, not a
  difference (`:2965-2970`)
- **AND** these are quirks of the pinned engine, not of the suite or the
  specification, which gives `-q` as negation and `q - q` as a difference;
  recorded as `DEV-0017` and resolved as HAPI's behaviour

#### Scenario: [path.math.004] multiplication works on integers, decimals and quantities

- **WHEN** the `testMultiply` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.math.005] division always yields a decimal

- **WHEN** the `testDivide` group's cases are evaluated
- **THEN** every case HAPI passes yields its stated output
- **AND** Frevo's required status per case follows HAPI's own recorded run (`oracles/fhirpath/hapi-8.12.0-results.json`): `passed` for every case HAPI passes, and `failed` (parity under `path.oracle.012`) for the cases named below, which HAPI fails. A Frevo pass on one of them would be listed as `ahead-of-hapi`, not accepted as a pass of this scenario
- **AND** the failed case is `testDivide7` (`0.0 / 0`): `opDivideBy` swallows the
  `UcumException` the decimal division raises (`FHIRPathEngine.java:3005-3011`),
  but `org.fhir.ucum.Decimal.divide` returns zero for a zero dividend before it
  checks the divisor, so HAPI answers one item where the suite expects none
- **AND** the digits `/` yields where the quotient does not terminate (`1 / 3`)
  are those of `org.fhir.ucum.Decimal.divide`, whose source is not in
  `reference/`; the required precision is therefore not stated here and is
  listed in `tasks/blockers.md`

#### Scenario: [path.math.006] `div` truncates toward zero

- **WHEN** the `testDiv` group's cases are evaluated
- **THEN** every case HAPI passes yields its stated output
- **AND** Frevo's required status per case follows HAPI's own recorded run (`oracles/fhirpath/hapi-8.12.0-results.json`): `passed` for every case HAPI passes, and `failed` (parity under `path.oracle.012`) for the cases named below, which HAPI fails. A Frevo pass on one of them would be listed as `ahead-of-hapi`, not accepted as a pass of this scenario
- **AND** the failed case is `testDiv6` (`0 div 0.0`), for the zero-dividend
  reason in `path.math.005`; `opDiv` (`FHIRPathEngine.java:3050-3064`) answers
  one item

#### Scenario: [path.math.007] `mod` returns the remainder

- **WHEN** the `testMod` group's cases are evaluated
- **THEN** every case HAPI passes yields its stated output
- **AND** Frevo's required status per case follows HAPI's own recorded run (`oracles/fhirpath/hapi-8.12.0-results.json`): `passed` for every case HAPI passes, and `failed` (parity under `path.oracle.012`) for the cases named below, which HAPI fails. A Frevo pass on one of them would be listed as `ahead-of-hapi`, not accepted as a pass of this scenario
- **AND** the failed cases are `testMod6` (`5.5 mod 0`) and `testMod7`
  (`5 mod 0.0`), which fail with an execution error, because the decimal arm of
  `opMod` rethrows the `UcumException` as a `PathEngineException`
  (`FHIRPathEngine.java:3098-3105`); and `testMod8` (`0.0 mod 0`), which answers
  one item where the suite expects none

#### Scenario: [path.math.008] division by zero yields empty only where the pinned engine says so

- **WHEN** `1 / 0`, `1 div 0`, `1 mod 0`, `5 div 0` and `5 mod 0` are evaluated
- **THEN** each yields an empty collection: integer `div` and `mod` guard a zero
  divisor (`FHIRPathEngine.java:3051-3054`, `3092-3095`), and `/` swallows the
  `UcumException` (`:3009-3011`)
- **WHEN** `0.0 / 0`, `0 div 0.0` and `0.0 mod 0` are evaluated
- **THEN** each yields one item, not an empty collection (the suite cases
  `testDivide7`, `testDiv6`, `testMod8`, recorded `failed` by HAPI); the value
  is `org.fhir.ucum.Decimal`'s, whose source is not in `reference/`, so it is
  not stated here
- **WHEN** `5.5 mod 0` and `5 mod 0.0` are evaluated
- **THEN** evaluation fails with an execution error (`testMod6`, `testMod7`,
  recorded `failed` by HAPI)
- **AND** the FHIRPath specification makes every division by zero empty; recorded
  as `DEV-0017` and resolved as HAPI's behaviour

#### Scenario: [path.math.009] arithmetic with an empty operand yields empty

- **WHEN** `{} + 1` and `1 + {}` are evaluated
- **THEN** both yield an empty collection

#### Scenario: [path.math.010] arithmetic on incompatible types is an execution error

- **WHEN** `1 + true` is evaluated
- **THEN** evaluation fails

#### Scenario: [path.math.011] integer arithmetic that overflows behaves as the pinned engine does

- **WHEN** `2147483647 + 1` is evaluated
- **THEN** the engine behaves as `FHIRPathEngine` at the pinned SHA does, and
  the harness records which behaviour that is

#### Scenario: [path.math.012] date arithmetic adds and subtracts calendar durations

- **WHEN** `@2014-01-01 + 1 year`, `@2014-01-31 + 1 month` and
  `@2014-01-01 - 1 day` are evaluated
- **THEN** each yields a `date` computed as the engine at the pinned SHA
  computes it, including its month-end behaviour
- **AND** the suite cases that reach `dateAdd` are graded as HAPI ran them:
  `testMinus5` (`@1974-12-25 - 1 'month'`) and `testMinus6` pass, while
  `testMinus7` is `failed` (`oracles/fhirpath/hapi-8.12.0-results.json:835`,
  `Invalid date/time string (datatype DateTimeType does not support MINUTE
  precision)`), as are `testPlusDate23` and `testPlusDate24`; Frevo fails them
  with the same message (`path.math.001`, `path.math.003`, `DEV-0017`)

#### Scenario: [path.math.013] date arithmetic preserves the operand's precision

- **WHEN** `@2014 + 1 year` is evaluated
- **THEN** the result is a year-precision `date`

#### Scenario: [path.math.014] subtracting two dateTimes is not addition of a duration

- **WHEN** `@2014-01-02 - @2014-01-01` is evaluated
- **THEN** the engine behaves as it does at the pinned SHA, and the harness
  records whether that is a quantity or an error

### Requirement: Math functions

The engine SHALL implement the math functions `FHIRPathEngine` implements, each over a singleton numeric focus. An empty focus yields an empty collection for `exp` only; the other nine raise (`path.mathfn.010`). `round` and `truncate` are exact; the other eight go through a binary `double` (`path.compare.012`).

#### Scenario: [path.mathfn.001] `abs` works on integers, decimals and quantities

- **WHEN** the `testAbs` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.mathfn.002] `ceiling` rounds up

- **WHEN** the `testCeiling` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.mathfn.003] `floor` rounds down

- **WHEN** the `testFloor` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.mathfn.004] `round` rounds to the requested number of digits

- **WHEN** the `testRound` group's cases are evaluated
- **THEN** every case yields its stated output
- **AND** a decimal result is rendered as Java's `BigDecimal.toString` renders
  it (`DecimalType.encode`, `DecimalType.java:101-102`): plain digits when the
  scale is zero or positive and the adjusted exponent is at least -6, otherwise
  scientific with a signed exponent. `round` with a negative digit count uses
  `setScale` (`FHIRPathEngine.java:4412`), so `12345.round(-2)` is `1.23E+4` and
  `1234.5.round(-3)` is `1E+3`
- **AND** the `double`-based functions build the result with
  `BigDecimal.valueOf(double)` (`DecimalType.java:66-69`), which parses
  `Double.toString`: `10000000000.abs()` is `1.0E+10`, `0.0000001.abs()` is
  `1.0E-7`, `0.0001.abs()` is `0.00010` and `12345678.abs()` is `12345678`.
  This is a pinned-engine rendering, not the specification's, which has no
  exponent form; it is HAPI's behaviour and Frevo matches it. Differential-only

#### Scenario: [path.mathfn.005] `sqrt` of a negative number yields empty

- **WHEN** the `testSqrt` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.mathfn.006] `exp` and `ln` are inverses within the engine's precision

- **WHEN** the `testExp` and `testLn` groups' cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.mathfn.007] `log` takes the base as its argument

- **WHEN** the `testLog` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.mathfn.008] `power` of a negative base with a fractional exponent yields empty

- **WHEN** the `testPower` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.mathfn.009] `truncate` discards the fractional part

- **WHEN** the `testTruncate` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.mathfn.010] a math function on an empty collection yields empty only for `exp`

- **WHEN** `{}.exp()` is evaluated
- **THEN** the result is an empty collection: `funcExp` guards an empty focus
  (`FHIRPathEngine.java:4177`)
- **WHEN** `{}.abs()`, `{}.ceiling()`, `{}.floor()`, `{}.sqrt()`, `{}.ln()`,
  `{}.log(2)`, `{}.power(2)`, `{}.round(2)` and `{}.truncate()` are evaluated
  with the check skipped
- **THEN** each fails with an execution error, because each opens with
  `if (focus.size() != 1) throw makeExceptionPlural(...)`, which covers zero
  (`funcAbs` `:4116`, `funcCeiling` `:4139`, `funcFloor` `:4157`, `funcSqrt`
  `:4096`, `funcLn` `:4201`, `funcLog` `:4221`, `funcPower` `:4249`, `funcRound`
  `:4397`, `funcTruncate` `:4273`)
- **AND** the requirement's "yielding an empty collection for an empty focus" is
  the FHIRPath specification's, not HAPI's; recorded as `DEV-0019` and resolved
  as HAPI's behaviour. No live case grades it, so it is differential-only

#### Scenario: [path.mathfn.011] a math function on a collection of more than one is an execution error

- **WHEN** `(1 | 2).abs()` is evaluated
- **THEN** evaluation fails

### Requirement: String functions

The engine SHALL implement the string functions `FHIRPathEngine` implements, including the R3-era additions `trim`, `split`, `join`, `encode`, `decode`, `escape` and `unescape`.

#### Scenario: [path.string.001] `upper` and `lower` change case

- **WHEN** the `testCase` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.string.002] `toChars` splits a string into single characters

- **WHEN** the `testToChars` group's cases are evaluated
- **THEN** every case yields its stated output
- **AND** `toChars` yields one item per UTF-16 code unit
  (`for (char c : s.toCharArray())`, `FHIRPathEngine.java:5835`), so
  `'a😀'.toChars()` (U+1F600) yields three items: `a`, the high surrogate
  U+D83D and the low surrogate U+DE00, each a one-unit string; `count()` is `3`

#### Scenario: [path.string.003] `indexOf` returns a zero-based position or -1

- **WHEN** the `testIndexOf` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.string.004] `substring` takes a start and an optional length

- **WHEN** the `testSubstring` group's cases are evaluated
- **THEN** every case HAPI passes yields its stated output (8 of 16)
- **AND** Frevo's required status per case follows HAPI's own recorded run
  (`oracles/fhirpath/hapi-8.12.0-results.json:352-359`): `failed` (parity under
  `path.oracle.012`) for `testSubstring11` to `18`, and `passed` for the rest
- **AND** the eight failures are, in `funcSubstring`
  (`FHIRPathEngine.java:5864-5895`):
  - `testSubstring11`, `12` (`{}.substring({})`, `'string'.substring({})`): the
    start argument is read with no emptiness guard
    (`Integer.parseInt(n1.get(0)...)`, `:5867`), an index-out-of-bounds failure
  - `testSubstring13`, `14`, `15` (`substring(3, -1)`, `substring(0, -1)`,
    `substring(1, 2147483647)`): the end is `min(length, start + count)` with
    an `int` sum (`:5884`), which falls before the start or overflows, and
    `String.substring` fails
  - `testSubstring16` (`substring(3, 0)`): the empty result is dropped
    (`:5888`), so the answer is empty and `= ''` is empty
  - `testSubstring17`, `18` (`substring(3, {})`, `substring(0, {})`): an empty
    length argument returns empty (`:5871-5872`) rather than the rest of the
    string
- **AND** the specification requires `{}` for an empty start, the rest of the
  string for an empty length, and `''` for a zero count; recorded as `DEV-0019`
- **AND** the start and count are UTF-16 code-unit offsets, so `substring` can
  split a surrogate pair (`path.string.022`)

#### Scenario: [path.string.005] `substring` with an out-of-range start yields empty

- **WHEN** `'abc'.substring(5)` and `'abc'.substring(-1)` are evaluated
- **THEN** both yield an empty collection

#### Scenario: [path.string.006] `startsWith` and `endsWith` test affixes

- **WHEN** the `testStartsWith` and `testEndsWith` groups' cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.string.007] `contains` tests substring presence

- **WHEN** the `testContainsString` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.string.008] `matches` tests a regular expression unanchored

- **WHEN** the `testMatches` group's cases are evaluated (it includes the
  `matchesFull` cases); HAPI passes all of them
- **THEN** every case yields its stated output
- **AND** `matches` prefixes the pattern with `(?s)`, so `.` matches a line
  terminator, and finds it anywhere in the string
  (`FHIRPathEngine.java:5698`)
- **AND** on the edges, `funcMatches` (`:5683-5707`) gives: an empty focus
  collection or an empty pattern **collection** yields an empty collection
  (`:5688`); an empty pattern **string** (`'abc'.matches('')`) yields `false`
  (`:5705-5706`); an empty focus **string** (`''.matches('a')`) yields `false`
  (`:5693-5694`); a focus that is not string-like with a non-empty pattern
  (`1.matches('1')`) yields an empty collection (`DEV-0006`); a non-string
  pattern is converted to its string (`'abc'.matches(1)` tests the pattern `1`
  and is `false`); a focus of more than one item yields `false`
- **AND** a pattern Java cannot compile raises an execution error, as does a
  match that exceeds HAPI's 500 ms regex timeout (`Timeout evaluating regex`,
  `:5699-5700`); Frevo has no wall-clock timeout (`DEV-0023`)

#### Scenario: [path.string.009] `matchesFull` requires the whole string to match

- **WHEN** `'abc'.matches('b')` and `'abc'.matchesFull('b')` are evaluated
- **THEN** the results are `true` and `false`
- **AND** `matchesFull` prefixes `(?s)` and matches the whole string
  (`RegexTimeout.matches`, `FHIRPathEngine.java:5725`)
- **AND** `funcMatchesFull` (`:5711-5735`) has **no empty-propagation branch**,
  unlike `matches`: `{}.matchesFull('b')` yields `false`, not an empty
  collection, because `focus.size() == 1` is false and the else arm adds
  `false` (`:5733-5734`); `'abc'.matchesFull({})` and `'abc'.matchesFull('')` yield
  `false` (the pattern string is empty); `''.matchesFull('a')` yields `false`;
  `1.matchesFull('1')` with a non-string focus yields an empty collection
  (`DEV-0006`)
- **AND** the FHIRPath specification yields empty for an empty input; the
  difference is HAPI's and is recorded as `DEV-0023`

#### Scenario: [path.string.010] `replaceMatches` substitutes regular-expression matches

- **WHEN** the `testReplaceMatches` group's cases are evaluated
- **THEN** every case yields its stated output
- **AND** `replaceMatches` does **not** add the `(?s)` prefix, so `.` does not
  match a line terminator in it (`:4879`), and the replacement uses Java's
  `$n`, `${name}` and backslash syntax (`Matcher.replaceAll`)
- **AND** on the edges, `funcReplaceMatches` (`FHIRPathEngine.java:4865-4888`):
  an empty focus collection, empty pattern collection or empty replacement
  collection yields an empty collection (`:4872`); an empty pattern **string**
  (`'abc'.replaceMatches('', 'x')`) returns the focus string unchanged
  (`testReplaceMatches2`, `:4885-4886`); a focus of more than one item returns the
  **first** item's string with no error (`:4885-4886`); a focus that is not
  string-like with a non-empty pattern yields an empty collection (`DEV-0006`)
#### Scenario: [path.string.025] a pattern construct whose meaning would silently differ from Java's is an execution error

- **WHEN** `'a'.matches('(?u)a')`, `'a'.matches('(?U)a')`, `'a'.matches('(?x)a')`,
  `'a'.matches('\\v')`, `'a'.matches('a*+')`, `'a'.matches('\\p{Alpha}')` and
  `'a'.matches('[a-z--b]')` and `'a'.matches('[a-z~~b]')` are evaluated (the
  last two test `--` and `~~` after a range inside a class), each also through
  `matchesFull` and `replaceMatches`
- **THEN** each fails with an execution error naming the construct
- **AND** HAPI accepts every one of them with Java's meaning: a possessive
  quantifier, the vertical-whitespace class, the `U`, `u` and `x` flags and a
  POSIX-named property each work in Java, and in `[a-z--b]` Java reads the
  union of `a-z`, `-` and `b` (so `'-'` and `'b'` match) where the `regex`
  crate reads set difference (so `'b'` would not); `~~` is likewise a literal
  `~` in Java and symmetric difference in the crate. The departure is recorded
  as `DEV-0023`, and the engine is the `regex` crate behind a compatibility
  layer (ADR 0005, decision item 5)
- **AND** `[a--b]` is a `PatternSyntaxException` in Java as well (an invalid
  range), so it is not a departure
- **AND** backreferences and lookaround are the other way round: Java accepts
  `'aa'.matches('(a)\\1')` (`true`) and `a(?=b)`, atomic groups and `\\Q..\\E`,
  and the `regex` crate has no such construct, so Frevo raises an execution
  error where HAPI answers. This is a consequence of ADR 0005's choice of the
  `regex` crate, which the owner approved, and is recorded as `DEV-0023`
  (verified against an OpenJDK 25 probe)
- **AND** an unbalanced parenthesis (`'a'.matches('(a')`) is an execution error
  in HAPI as well, a `PatternSyntaxException` wrapped by `RegexTimeout`
  (`RegexTimeout.java:28-30`), so it is not a departure
- **AND** no pinned suite case grades any of these, so they are differential-only

#### Scenario: [path.string.026] `\d`, `\w`, `\s` and `\b` mean their ASCII sets, as in Java's default mode

- **WHEN** `'٣'.matches('\\d')` (an Arabic-Indic digit), `'é'.matches('\\w')`,
  `'é'.matches('\\W')` and `'a b'.matches('a\\sb')` (a no-break space) are
  evaluated
- **THEN** the results are `false`, `false`, `true` and `false`
- **AND** `'7'.matches('\\d')`, `'_'.matches('\\w')`, `'a b'.matches('a\\sb')` and
  `'ab'.matches('\\bab\\b')` are `true`
- **AND** the same holds inside a character class (`'é'.matches('[\\w]')` is
  `false`), and `\D`, `\W`, `\S` and `\B` are the complements
- **AND** this is Java's default (non-Unicode-class) meaning; differential-only

#### Scenario: [path.string.027] the `replaceMatches` replacement string uses Java's syntax

- **WHEN** `'abc'.replaceMatches('(b)', '[$1]')`,
  `'abc'.replaceMatches('(?<x>b)', '<${x}>')`, `'abc'.replaceMatches('b', '\\$')`
  and `'a.c'.replaceMatches('\\.', '\\\\')` are evaluated (the literals pass
  `\$` and two backslashes to the engine)
- **THEN** the results are `a[b]c`, `a<b>c`, `a$c` and `a\c`
- **AND** `$n` takes further digits only while the number still names a group:
  with ten groups `$10` is group 10, and with one group `$10` is group 1
  followed by `0`
- **AND** a reference to a missing group (`'abc'.replaceMatches('(b)', '$2')`,
  `${nope}`), a trailing `$` and a trailing `\` are execution errors
- **AND** a group that did not participate in the match contributes nothing
- **AND** the engine's own `$` template syntax is never used: `'abc'.replaceMatches('b', '$$')`
  is an execution error as in Java, not `a$c`

#### Scenario: [path.string.028] `replaceMatches` finds empty matches as Java's `Matcher.find` does

- **WHEN** `'abc'.replaceMatches('x*', '-')` and `'abc'.replaceMatches('b*', '-')`
  are evaluated
- **THEN** the results are `-a-b-c-` and `-a--c-`: an empty match is found at
  every position including the end, and after an empty match the search steps one
  UTF-16 code **unit** forward
- **AND** that step can split a surrogate pair: `'😀'.replaceMatches('x*', '-')`
  (U+1F600, two UTF-16 units) is the five-unit string `-`, the high surrogate
  U+D83D, `-`, the low surrogate U+DE00, `-`; and `'a😀'.replaceMatches('x*', '-')`
  is `-`, `a`, `-`, U+D83D, `-`, U+DE00, `-`. Verified against `String.replaceAll`
  on an OpenJDK 25 probe; the pinned HAPI's JVM is expected to agree, since
  `Matcher.find` advances by one `char` after an empty match
- **AND** the two results hold lone surrogates, which the string type must be
  able to carry (`path.string.022`)
- **AND** an empty pattern string is returned unchanged rather than matched at
  every position (`'abc'.replaceMatches('', 'x')` is `abc`, `path.string.010`)
- **AND** differential-only

#### Scenario: [path.string.029] a pattern over the compiled-size or nesting limit is an execution error

- **WHEN** `'a'.matches('(?:a{1000}){1000}')` (a compiled program over 2 MiB) and
  a pattern of 200 nested groups are evaluated
- **THEN** each fails with an execution error (compiled-program `size_limit` of
  2 MiB, group `nest_limit` of 100), not a panic (`path.errors.003`)
- **AND** a pattern nested 50 deep and a small pattern evaluate normally
- **AND** HAPI compiles both with `java.util.regex` and answers, so this is a
  departure recorded as `DEV-0023`; the limit values are owed to `design.md`
  (`tasks/blockers.md`). Differential-only

#### Scenario: [path.string.011] `replace` substitutes literal substrings

- **WHEN** the `testReplace` group's cases are evaluated
- **THEN** every case yields its stated output
- **AND** the substitution is Java's `String.replace` (`FHIRPathEngine.java:4854`),
  with no pattern: an empty target inserts the replacement between every
  character and at both ends (`'abc'.replace('', '-')` is `-a-b-c-`); an empty
  focus string yields `''` whatever the arguments (`:4851-4852`); an empty focus
  collection or an empty argument yields empty (`:4846`); a focus of more than
  one item is an execution error (`:4859`)

#### Scenario: [path.string.012] `length` counts characters

- **WHEN** the `testLength` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.string.013] `trim` removes leading and trailing whitespace

- **WHEN** the `testTrim` group's cases are evaluated
- **THEN** every case yields its stated output
- **AND** the characters removed are those Java's `String.trim` removes, every
  character up to and including U+0020 (`FHIRPathEngine.java:4527`), so a
  U+00A0 no-break space is **not** removed

#### Scenario: [path.string.014] `split` keeps empty segments

- **WHEN** the `testSplit` group's cases are evaluated, including
  `'A,,C'.split(',').join(',') = 'A,,C'` and the nine-segment `[stop]` case
- **THEN** every case yields its stated output
- **AND** `split` is `Utilities.simpleSplit` (`Utilities.java:1978-1996`): the
  separator is a literal, not a pattern, and a trailing separator yields a
  trailing empty segment
- **WHEN** `'abc'.split('')` is evaluated (an empty separator)
- **THEN** HAPI never terminates: `simpleSplit` finds a match of length zero at
  every cursor, does not advance the cursor, and appends an empty segment to an
  unbounded list on each pass until memory is exhausted
  (`Utilities.java:1985-1994`, `1998-2007`). There is no defined HAPI result
- **AND** Frevo returns an execution error, as `path.errors.004` requires of an
  expression that would otherwise run unbounded and `path.errors.003` of a
  failure; recorded as `DEV-0022`. A differential test cannot run this case
  against HAPI and grades Frevo's error alone

#### Scenario: [path.string.015] `join` concatenates with a separator and yields an empty string for an empty collection

- **WHEN** the `testJoin` group's cases are evaluated
- **THEN** every case HAPI passes yields its stated output
- **AND** the failed case is `testJoinEmpty` (`name.prefix.join(',')`, expected
  empty): `funcJoin` ends with `result.add(new StringType(b.toString()))` outside
  any emptiness test (`FHIRPathEngine.java:4547-4565`), so an empty focus
  yields **one empty string**, not an empty collection
  (`hapi-8.12.0-results.json:457`). Frevo's required status is `failed`
  (parity under `path.oracle.012`); a Frevo pass would be listed
  `ahead-of-hapi`
- **AND** the FHIRPath specification yields empty for an empty input; recorded as
  `DEV-0022` and resolved as HAPI's behaviour

#### Scenario: [path.string.016] `encode` supports base64, urlbase64 and hex

- **WHEN** the `testEncodeDecode` group's `encode` cases are evaluated,
  including `'subjects?_d'.encode('urlbase64')` yielding `c3ViamVjdHM_X2Q=`
- **THEN** every case yields its stated output

#### Scenario: [path.string.017] `decode` reverses `encode` for each supported form

- **WHEN** the `testEncodeDecode` group's `decode` cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.string.018] `encode` and `decode` with an unsupported format yield empty or fail as the pinned engine does

- **WHEN** `'x'.encode('rot13')` and `'x'.decode('rot13')` are evaluated
- **THEN** each yields an empty collection: the format is compared to `hex`,
  `base64` and `urlbase64` in turn and nothing is added when none matches
  (`FHIRPathEngine.java:4440-4481`)
- **AND** `'!!'.decode('base64')` fails with an execution error, because
  `Base64.Decoder.decode` raises `IllegalArgumentException` on malformed input
- **AND** text is converted to bytes with the JVM default charset
  (`getBytes()`); Frevo uses UTF-8, which is identical wherever the default is
  UTF-8

#### Scenario: [path.string.019] `escape` and `unescape` support `html` and `json`

- **WHEN** the `testEscapeUnescape` group's cases are evaluated
- **THEN** every case HAPI passes yields its stated output
- **AND** Frevo's required status per case follows HAPI's recorded run
  (`oracles/fhirpath/hapi-8.12.0-results.json:439-441`): `failed` (parity under
  `path.oracle.012`) for `testUnescapeJson2`, `3` and `4`, `passed` for the rest
- **AND** the failures come from `Utilities.unescapeJson`
  (`Utilities.java:481-525`):
  - `case '"'` appends the letter `b`, not a quote (`:492-494`): the string
    `\"1<2\"` (a backslash before each quote) unescapes to `b1<2b`. That is the
    outcome of `testUnescapeJson2`, whose escape step produces exactly that
    string, and of `testUnescapeJson4`, whose literal `'\\"1<2\\"'` is that
    string
  - after `\u` the cursor advances one place and not past the four hex digits
    (`:516-519`, `:525`), so they are appended as text: the string
    made of a backslash then `u0041B` (suite literal `'\\u0041B'`, `testUnescapeJson3`) unescapes to
    `A0041B`, not `AB`
  - an unknown escape such as `\x` raises `Unknown JSON escape` (`:521`), an
    execution error; a trailing lone backslash raises an index error
- **AND** `testUnescapeJson` (the literal `'\"1<2\"'`, which the lexer has
  already turned into `"1<2"`) and `testEscapeJson` pass
- **AND** `escape` and `unescape` also accept `url` and `md`
  (`FHIRPathEngine.java:4492-4495`, `4513-4516`), and any other format yields an
  empty collection. No pinned suite case grades either, so they are
  differential-only:
  - `escape('url')` is Java's `URLEncoder.encode(s, "UTF-8")`
    (`Utilities.java:880-886`), the form-urlencoded rule: ASCII letters, digits
    and `.` `-` `*` `_` are kept; a space becomes `+`; every other character is
    its UTF-8 bytes as `%XX` with upper-case hex. So `'a b&c=d/é~'` escapes
    to `a+b%26c%3Dd%2F%C3%A9%7E` (the `~` is encoded)
  - `unescape('url')` is `URLDecoder.decode(s, "UTF-8")` (`:889-895`): `+`
    becomes a space and `%XX` sequences are decoded as UTF-8, so `a+b%20c`
    gives `a b c`; a malformed `%` sequence such as `%zz` raises
    `IllegalArgumentException`, an execution error
  - `escape('md')` is `MarkDownProcessor.makeStringSafeAsMarkdown`
    (`MarkDownProcessor.java:254-274`): a backslash is inserted before each of
    exactly seven characters, `*` `&` `#` `[` `>` `<` and the backtick, and
    before nothing else (not `]`, `_`, `(`, `)` or a backslash). So
    `a*b&c#d[e>f<g` followed by a backtick and `h` escapes to the same text with
    a backslash before each of those seven characters
  - `unescape('md')` is `makeMarkdownForString` (`:280-288`): a backslash is
    dropped when, and only when, the next character is one of those seven and
    the backslash is not the last character; every other backslash is kept. So
    `a\*b` gives `a*b`, while `a\xb` and a trailing backslash are unchanged, and
    `\\*` (two backslashes then `*`) gives a single backslash then `*`
- **AND** the specification requires `\"` to unescape to a quote and a `\u`
  escape to consume its four digits; recorded as `DEV-0022` and resolved as
  HAPI's behaviour

#### Scenario: [path.string.020] a string function on a non-string yields empty

- **WHEN** `1.startsWith('1')` is evaluated
- **THEN** the result is an empty collection, because the function appends a
  result only when the focus has one of the 12 string-like FHIR types
  (`string`, `uri`, `code`, `oid`, `id`, `uuid`, `sid`, `markdown`,
  `base64Binary`, `canonical`, `url`, `xhtml`) or implicit string conversion is
  switched on
- **AND** the same guard governs `upper`, `lower`, `contains`, `endsWith`,
  `substring`, `indexOf`, `matches`, `matchesFull`, `replace`,
  `replaceMatches`, `length` and `toChars`
- **AND** `trim`, `split` and `join` do **not** check the focus type
  (`FHIRPathEngine.java:4523-4529`, `4532-4545`, `4547-4565`): they use the
  item's primitive value, so `1.trim()` is `'1'`, `1.split(',')` is the one
  item `'1'`, and `1.join(',')` is `'1'`
- **AND** on a **complex** (non-primitive) focus the primitive value is `null`
  (`Base.primitiveValue()` returns `null`) and, evaluated with no focus-type
  check in the way (the static check passes it, `:3622-3630`):
  - `Patient.name.first().trim()` fails at **evaluation** with a
    `NullPointerException` on `cnt.trim()` (`:4527`), an execution error in
    Frevo (`path.errors.003`)
  - `Patient.name.first().split(',')` yields an empty collection, because
    `simpleSplit` returns no segments for a `null` string (`Utilities.java:1979-1981`)
  - `Patient.name.join(',')` yields one empty string `''`: each `null` item is
    skipped, with no separator added for it (`CommaSeparatedStringBuilder`
    `commit`, `append`)
- **AND** `join` takes zero or one argument: `checkParameters` allows between 0
  and 1 (`FHIRPathEngine.java:1427`, `checkParamCount` `:1355-1359`), so
  `(1 | 2 | 3).join(',', ' and ')` is a **parse** failure, raised while parsing
  with the message `The function "join" requires between 0 and 1 parameters`
  and never reaching the check or evaluation. The second-argument branch inside
  `funcJoin` (`:4553-4556`) is unreachable. `join()` with no argument joins with
  an empty separator
- **AND** this differs from the FHIRPath specification, which makes a
  type-mismatched focus an error; the divergence is recorded as `DEV-0006` and
  resolved as HAPI's behaviour

#### Scenario: [path.string.021] a string function on an empty collection yields empty

- **WHEN** `{}.upper()`, `{}.length()` and `{}.substring(0)` are evaluated
- **THEN** each yields an empty collection

#### Scenario: [path.string.023] the functions that do reject a bad focus reject it with an error

- **WHEN** `(1 | 2).round(2)`, `(1 | 2).truncate()` and `(1 | 2).single()` are
  evaluated
- **THEN** each fails, because those functions raise on a focus whose size is
  not 1 rather than returning empty — unlike the string functions in
  `path.string.020`
- **AND** `'abc'.round(2)` is rejected by the **check** on the focus type, which
  is the other shape of rejection the engine has: `checkContextDecimal`
  (`FHIRPathEngine.java:3690`, `3890-3892`) raises `FHIRPATH_DECIMAL_ONLY`
  ("The function round can only be used on a decimal but found
  [System.String]"). With the check skipped it does **not** fail at evaluation:
  the else arm of `funcRound` builds an exception and never throws it
  (`:4414`), so the result is empty

#### Scenario: [path.string.024] implicit string conversion changes the outcome of every string function

- **WHEN** `1.startsWith('1')` is evaluated with implicit string conversion
  switched off, which is the default and what both HAPI wrappers use
- **THEN** the result is an empty collection
- **WHEN** the same expression is evaluated with implicit string conversion
  switched on
- **THEN** the result is `true`, because the focus is converted to its string
  value before the test
- **AND** the engine exposes this as a public setting
  (`setDoImplicitStringConversion`), so both outcomes are reachable

#### Scenario: [path.string.022] string functions count UTF-16 code units, not bytes

- **WHEN** a string containing non-ASCII characters is measured with `length()`
  and sliced with `substring()` or searched with `indexOf()`
- **THEN** all three count UTF-16 code units, as a Java `String` does
  (`FHIRPathEngine.java:5771`, `5884`), so a character above U+FFFF counts as
  two; a character in the Basic Multilingual Plane counts as one and is never
  split, and a byte count is never used
- **AND** `'a😀b'.length()` is `4` and `'a😀b'.indexOf('b')` is `3`
- **AND** `'a😀b'.substring(1, 1)` is the one-unit string holding the high
  surrogate U+D83D and `'a😀b'.substring(2)` is the low surrogate U+DE00
  followed by `b`: `substring` slices by unit and can split a surrogate pair
  (`:5884-5886`); the result is a string that is not valid Unicode scalar text
  and Frevo must represent it unchanged
- **AND** `toChars()` yields one item per UTF-16 unit (`path.string.002`)

### Requirement: Type conversion

The engine SHALL implement the `toX` functions the R4 engine implements and each `convertsToX` predicate, so that a failed conversion yields an empty collection while the predicate yields `false`, except `toDateTime` and `toTime`, which are stubs that always raise (`path.convert.011`, `path.convert.013`). The set is not symmetric: `convertsToDate` exists and `toDate` does not.

#### Scenario: [path.convert.001] `toInteger` converts strings, booleans and integers

- **WHEN** the `testToInteger` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.convert.002] `toDecimal` converts strings, booleans, integers and decimals

- **WHEN** the `testToDecimal` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.convert.003] `toString` converts every convertible type

- **WHEN** the `testToString` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.convert.004] `convertsToDate` matches the dateTime pattern, not a date pattern

- **WHEN** `'2015'.convertsToDate()`, `'2015-02'.convertsToDate()` and
  `'2015-02-04'.convertsToDate()` are evaluated
- **THEN** each yields `true` (`testTypes` group: `testStringYearConvertsToDate`,
  `testStringMonthConvertsToDate`, `testStringDayConvertsToDate`, all passed by
  HAPI)
- **AND** `funcIsDate` tests a string against `RegexConstants.DATE_TIME_REGEX`
  and not `DATE_REGEX`, with an upstream `FIXME` saying so
  (`FHIRPathEngine.java:5957-5965`), so `'2015-02-04T14'.convertsToDate()`,
  `'2015-02-04T14:34'.convertsToDate()` and
  `'2015-02-04T14:34:28+10:00'.convertsToDate()` also yield `true`, while
  `'abc'`, `'2015-13'` and `'2015-02-04T25'` yield `false`
- **AND** a `date` or `dateTime` value, whatever its precision, yields `true`
  (`:5961`), and a focus that is not exactly one item yields `false`
- **AND** no case in the pinned suite grades the time forms, so they are
  differential-only; the FHIRPath specification does not accept a time
  component for `convertsToDate`, and this is recorded as `DEV-0021` and
  resolved as HAPI's behaviour

#### Scenario: [path.convert.005] `convertsToDateTime` accepts every precision and offset form

- **WHEN** `'2015'`, `'2015-02'`, `'2015-02-04'`, `'2015-02-04T14'`,
  `'2015-02-04T14:34'`, `'2015-02-04T14:34:28'`, `'2015-02-04T14:34:28.123'`,
  `'2015-02-04T14:34:28Z'` and `'2015-02-04T14:34:28+10:00'` are tested with
  `convertsToDateTime()`
- **THEN** each yields `true` (`testTypes` group)

#### Scenario: [path.convert.006] `convertsToTime` accepts hour, minute, second and millisecond precision

- **WHEN** `'14'`, `'14:34'`, `'14:34:28'` and `'14:34:28.123'` are tested with
  `convertsToTime()`
- **THEN** each yields `true` (`testTypes` group)

#### Scenario: [path.convert.007] `convertsToBoolean` accepts the engine's accepted spellings

- **WHEN** the `testTypes` group's `convertsToBoolean` cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.convert.008] `convertsToQuantity` and `toQuantity` convert strings and numbers

- **WHEN** the `testTypes` group's `convertsToQuantity` and `toQuantity` cases
  are evaluated
- **THEN** every case HAPI passes yields its stated output
- **AND** the two cases HAPI fails stay `failed` in Frevo (parity under
  `path.oracle.012`): `testStringQuantityMonthToQuantityEquivalent`
  (`'1 month'.toQuantity() ~ 1 month`) and
  `testStringQuantityYearToQuantityEquivalent`, which raise because
  `qtyEquivalent` returns `null` (`hapi-8.12.0-results.json:192-193`,
  `path.equality.004`, `DEV-0017`)
- **AND** `toQuantity` converts an `integer` or `decimal` to a quantity with
  system `http://unitsofmeasure.org` and code `1` (`FHIRPathEngine.java:4957-4973`)

#### Scenario: [path.convert.009] a failed conversion yields empty while its `convertsTo` yields false

- **WHEN** `'abc'.toInteger()` and `'abc'.convertsToInteger()` are evaluated
- **THEN** the results are an empty collection and `false`

#### Scenario: [path.convert.010] a conversion function on a collection of more than one is an execution error

- **WHEN** `(1 | 2).toString()` is evaluated
- **THEN** evaluation fails

#### Scenario: [path.convert.012] `toDate()` does not exist and is a parse failure

- **WHEN** `'2015'.toDate()` is parsed
- **THEN** parsing fails, because the R4 function table has no `toDate` entry
  even though it has `convertsToDate`
- **AND** `'2015'.convertsToDate()` parses and evaluates to `true`, so the
  asymmetry is observable from one expression pair
- **AND** this differs from the FHIRPath specification, which defines
  `toDate()`; the divergence is recorded as `DEV-0009` and resolved as HAPI's
  behaviour
- **AND** no live case grades it, so it is differential-only

#### Scenario: [path.convert.013] `toTime` is not implemented; `convertsToTime` is

- **WHEN** `'14:34:28'.toTime()`, `'abc'.toTime()` and `{}.toTime()` are
  evaluated
- **THEN** each fails at **evaluation** with the error "The function toTime is
  not implemented" (`FHIRPATH_NOT_IMPLEMENTED`, `Messages.properties:347`):
  `funcToTime` is a stub whose whole body is that `throw`
  (`FHIRPathEngine.java:4983-4987`), whatever the focus
- **AND** the static check passes for a primitive focus and types the result as
  a `time` (`:3654-3656`), so the failure is at evaluation and not at parse or
  check; a non-primitive focus is rejected by the check with "The function ToTime
  can only be used on primitives" (`FHIRPATH_PRIMITIVE_ONLY`), the name written with
  a capital `T` (`:3655`)
- **AND** `convertsToTime` is implemented (`funcIsTime`, `:5986-5990`): `true`
  for a `time` value or a string matching
  `(T)?([01][0-9]|2[0-3])(:[0-5][0-9](:([0-5][0-9]|60))?)?(\.[0-9]+)?(Z|...)?`,
  so `'abc'.convertsToTime()` is `false` and `'14'`, `'14:34'`, `'14:34:28'` and
  `'14:34:28.123'` are `true` (`path.convert.006`)
- **AND** the FHIRPath specification defines `toTime`; recorded as `DEV-0021`
  and resolved as HAPI's behaviour. The pinned suite has no case for it, so it
  is differential-only

#### Scenario: [path.convert.014] `toBoolean` converts the engine's accepted spellings and yields empty on failure

- **WHEN** `'true'.toBoolean()`, `'yes'.toBoolean()`, `'0'.toBoolean()` and
  `'abc'.toBoolean()` are evaluated
- **THEN** each yields what the engine at the pinned SHA accepts for that
  spelling, with an unaccepted spelling yielding an empty collection
- **AND** the accepted set is the same one `convertsToBoolean` reports `true`
  for (`path.convert.007`)

#### Scenario: [path.convert.011] `toDateTime` is not implemented; `convertsToDateTime` is

- **WHEN** `@2014.toDateTime()`, `'2015-02-04'.toDateTime()` and
  `{}.toDateTime()` are evaluated
- **THEN** each fails at **evaluation** with the error "The function
  toDateTime is not implemented" (`FHIRPATH_NOT_IMPLEMENTED`,
  `Messages.properties:347`): `funcToDateTime` is a stub whose whole body is
  that `throw` (`FHIRPathEngine.java:4976-4981`), whatever the focus
- **AND** the static check passes for a primitive focus and types the result as
  a `dateTime` (`:3650-3652`); a non-primitive focus is rejected by the check with
  "The function ToDateTime can only be used on primitives" (`FHIRPATH_PRIMITIVE_ONLY`,
  `Messages.properties:353`), the function name written with a capital `T`
  (`:3651`)
- **AND** `convertsToDateTime` is implemented (`funcIsDateTime`, `:5942-5955`):
  `true` for a `date` or `dateTime` value or a string matching
  `RegexConstants.DATE_TIME_REGEX`, `false` for any other focus including a
  collection of other than one item (`path.convert.005`)
- **AND** the FHIRPath specification defines `toDateTime`, with precision
  preserved; recorded as `DEV-0021` and resolved as HAPI's behaviour. The pinned
  suite has no case for it, so it is differential-only

### Requirement: Date and time functions

The engine SHALL implement `today`, `now` and `precision`, and SHALL take the evaluation time zone from its configuration rather than from the host machine.

#### Scenario: [path.datetime.001] `today()` returns the current date

- **WHEN** the `testToday` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.datetime.002] `now()` returns the current dateTime

- **WHEN** the `testNow` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.datetime.003] `now()` is read afresh on every call

- **WHEN** `now() = now()` is evaluated
- **THEN** the result is whatever comparing two separately read instants gives;
  the engine caches no evaluation instant, so two calls in one expression may
  return different values
- **AND** this differs from the FHIRPath specification, which requires `now()`
  to be consistent within an evaluation; the divergence is recorded as
  `DEV-0008` and resolved as HAPI's behaviour
- **AND** no live case grades it — `testNow` only checks
  `Patient.birthDate < now()` and `now().toString().length() > 10` — so it is
  differential-only

#### Scenario: [path.datetime.004] `today()` and `now()` read a configured time zone, not the machine's

- **WHEN** `today()` and `now()` are evaluated on an engine configured with a
  given time zone
- **THEN** both are computed in that zone
- **AND** changing the machine's local time zone changes neither result, which
  is the property `FHIRPathTests.test` has to obtain by calling
  `TimeZone.setDefault` because the engine reads the platform default
- **AND** the oracle harness sets that configuration to **offset 0 (UTC)**, so
  the suite's result does not depend on where it runs. That is the zone HAPI's
  recorded run used: `FHIRPathTests.test` calls
  `TimeZone.setDefault(TimeZone.getTimeZone("UTC+1100"))`
  (`org.hl7.fhir.r4/src/test/java/org/hl7/fhir/r4/test/FHIRPathTests.java:174`),
  but `UTC+1100` is not a zone id `java.util.TimeZone` recognises (a custom id
  needs the `GMT` prefix), so `getTimeZone` returns `GMT`, raw offset 0. The
  comment above that line ("Grahame is in UTC+11") states the intent, not the
  effect
- **AND** the record confirms offset 0: `testEquality/testEquality23` and
  `testNEquality/testNEquality17`
  (`@2012-04-15T15:00:00Z = / != @2012-04-15T10:00:00`, expecting no output)
  are `passed` (`oracles/fhirpath/hapi-8.12.0-results.json:496`, `:530`). At
  offset 0 the offset-less operand is `10:00Z`, five hours from `15:00Z` and
  inside the +-14 hour window, so the answer is empty; at +11:00 it would be
  `23:00Z` the previous day, sixteen hours away, and the answer `false`
- **AND** at the harness's offset 0 Frevo's required status is `passed` for
  `testEquality23`, `testNEquality17`, `testLiterals/testDateTimeGreaterThanDate2`
  (`now() > today()`, expecting no output, `hapi-8.12.0-results.json:104`;
  `today()` and the Zulu form of `now()` then always fall on the same day, so
  the comparison is undecided), and the four clock cases `testToday/testToday1`,
  `testToday2`, `testNow/testNow1` and `testNow2`, at any time of day
- **AND** the zone the engine reads also decides how a `dateTime` **without** a
  UTC offset is placed on the time line. For `=` against one with an offset it
  sets the +-14 hour window of `couldBeTheSameTime`
  (`BaseDateTimeType.java:937-960`, via `getValue().getTime()`, `:434`,
  `:721-723`), so the answer is `false` or empty by zone. For `<`, `>`, `<=`
  and `>=`, `compareDateTimeElements` converts both operands to Zulu when finer
  than a day (`FHIRPathEngine.java:643-648`), so
  `@2015-02-04T10:00:00 < @2015-02-04T05:30:00Z` depends on the zone too: at
  offset 0, `<` and `<=` are `false` and `>` and `>=` are `true` (`10:00Z` is
  after `05:30Z`); on an engine configured at +11:00 (660 minutes), `<` and `<=`
  are `true` and `>` and `>=` are `false` (`23:00Z` the previous day). The
  suite evaluates at offset 0; Frevo reads a missing offset as the configured
  evaluation zone, which for the oracle harness is the same offset 0, and as
  UTC where none is configured. Recorded as `DEV-0018`

#### Scenario: [path.datetime.005] `precision()` reports the component count of a date, dateTime or time

- **WHEN** the `Precision` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.datetime.007] `today()` has day precision

- **WHEN** `today().precision()` is evaluated
- **THEN** the result is the integer `8`, because the engine builds the value at
  day precision

#### Scenario: [path.datetime.006] a dateTime with an offset and one in UTC compare correctly

- **WHEN** `@2014-01-05T10:30:00+10:00 = @2014-01-05T00:30:00Z` is evaluated
- **THEN** the result is the boolean the engine at the pinned SHA computes

### Requirement: Quantities and UCUM

The engine SHALL compare, add, subtract, multiply and divide quantities by converting to UCUM canonical form from the pinned `ucum-essence.xml`, and SHALL implement `comparable`, answering as HAPI's engine does under the suite's worker (`path.quantity.019`).

#### Scenario: [path.quantity.001] quantities in commensurable UCUM units compare equal after conversion

- **WHEN** `4.0000 'g' = 4000.0 'mg'` is evaluated
- **THEN** the result is `true` (`testQuantity/testQuantity1`)

#### Scenario: [path.quantity.002] equivalence of quantities rounds to the lower precision

- **WHEN** `4 'g' ~ 4000 'mg'` and `4 'g' ~ 4040 'mg'` are evaluated
- **THEN** both yield `true` (`testQuantity/testQuantity2`,
  `testQuantity/testQuantity4`)

#### Scenario: [path.quantity.003] equality of quantities does not round

- **WHEN** `4 'g' != 4040 'mg'` is evaluated
- **THEN** the result is `true` (`testQuantity/testQuantity3`)

#### Scenario: [path.quantity.004] calendar durations and UCUM units are commensurable where the engine says so

- **WHEN** `7 days = 1 week` and `7 days = 1 'wk'` are evaluated
- **THEN** both yield `true` (`testQuantity/testQuantity5`,
  `testQuantity/testQuantity6`)

#### Scenario: [path.quantity.005] quantities in commensurable units compare by magnitude

- **WHEN** `6 days < 1 week` and `8 days > 1 week` are evaluated
- **THEN** both yield `true` (`testQuantity/testQuantity7`,
  `testQuantity/testQuantity8`)

#### Scenario: [path.quantity.006] multiplying quantities multiplies their units

- **WHEN** `2.0 'cm' * 2.0 'm' = 0.040 'm2'` is evaluated
- **THEN** the result is `true` (`testQuantity/testQuantity9`)

#### Scenario: [path.quantity.007] dividing quantities divides their units

- **WHEN** `4.0 'g' / 2.0 'm' = 2 'g/m'` is evaluated
- **THEN** the result is `true` (`testQuantity/testQuantity10`)

#### Scenario: [path.quantity.008] dividing a unit by itself yields the dimensionless unit

- **WHEN** `1.0 'm' / 1.0 'm' = 1 '1'` is evaluated
- **THEN** the result is `true` (`testQuantity/testQuantity11`)

#### Scenario: [path.quantity.009] two unitless quantities: equality is empty and equivalence fails

- **WHEN** `Parameters.parameter[0].value = Parameters.parameter[1].value` and
  the same with the equivalence operator are evaluated over
  `parameters-example-quantities.xml` (two quantities of value `1`, with no
  `unit`, `code` or `system`)
- **THEN** equality yields an **empty** collection and equivalence **fails**:
  `qtyEqual` and `qtyEquivalent` (`FHIRPathEngine.java:2258-2290`,
  `:2331-2363`) first try UCUM canonical forms, which need a UCUM `system`
  (`qtyToCanonicalPair`, `:2292-2303`), so none of these quantities takes that
  path. With no `code` on either side they reach the unit guard
  `else if (!left.hasUnit() || right.hasUnit())` (`:2282`, `:2355`): it returns
  `null` when the **left** has no unit, or when the units differ, and otherwise
  falls through to compare the values, so a left quantity **with** a unit and a
  right one **without** compares by value alone. `opEquals` turns a `null` into
  an empty result (`:2114-2127`); `doEquivalent` unboxes it into a `boolean`
  (`:2204-2206`), which raises a `NullPointerException`. The left has no unit, so both return `null`
- **AND** Frevo's required status per case follows HAPI's own recorded run (`oracles/fhirpath/hapi-8.12.0-results.json`): `passed` for every case HAPI passes, and `failed` (parity under `path.oracle.012`) for the cases named here, which HAPI fails. A Frevo pass on one of them would be listed as `ahead-of-hapi`, not accepted as a pass of this scenario; both are failed: `testQuantityUnitComparison/testQuantityNoUnitEqual`
  (the suite expects `true`; HAPI finds no item, `hapi-8.12.0-results.json:632`)
  and `testQuantityNoUnitEquivalent` (the suite expects `true`; HAPI raises,
  `:633`)
- **AND** where HAPI raises a `NullPointerException`, Frevo returns an execution error, never a panic (`path.errors.003`), with no offset (`path.errors.002`) and with the message HAPI's run records, the JVM's text, exactly: `Cannot invoke "java.lang.Boolean.booleanValue()" because the return value of "org.hl7.fhir.r4.fhirpath.FHIRPathEngine.qtyEquivalent(org.hl7.fhir.r4.model.Quantity, org.hl7.fhir.r4.model.Quantity)" is null`
- **AND** the specification makes both `true`; recorded as `DEV-0017`, and the
  empty equality as a wrong-answer hazard, `DEBT-0019`

#### Scenario: [path.quantity.010] two quantities with the same unit text and no code compare equal

- **WHEN** `Parameters.parameter[2].value = Parameters.parameter[3].value` and
  the equivalence form are evaluated over `parameters-example-quantities.xml`
- **THEN** both yield `true`
  (`testQuantityUnitComparison/testQuantityUnitTextEqual`,
  `testQuantityUnitTextEquivalent`)

#### Scenario: [path.quantity.011] equality of a quantity with a unit and one without depends on which side has the unit

- **WHEN** `Parameters.parameter[2].value = Parameters.parameter[0].value` (left
  `1 'tablets'` as unit text only, right `1` with no unit) and the reverse are
  evaluated over `parameters-example-quantities.xml`
- **THEN** the first yields the single boolean **`true`** and the reverse an
  **empty** collection: `qtyEqual` and `qtyEquivalent` (`FHIRPathEngine.java:2258-2290`,
  `:2331-2363`) first try UCUM canonical forms, which need a UCUM `system`
  (`qtyToCanonicalPair`, `:2292-2303`), so none of these quantities takes that
  path. With no `code` on either side they reach the unit guard
  `else if (!left.hasUnit() || right.hasUnit())` (`:2282`, `:2355`): it returns
  `null` when the **left** has no unit, or when the units differ, and otherwise
  falls through to compare the values, so a left quantity **with** a unit and a
  right one **without** compares by value alone. `opEquals` turns a `null` into
  an empty result (`:2114-2127`); `doEquivalent` unboxes it into a `boolean`
  (`:2204-2206`), which raises a `NullPointerException`. With the unit on the left the guard
  is skipped and `1 = 1` is compared; with it on the right the guard returns
  `null`
- **AND** Frevo's required status per case follows HAPI's own recorded run (`oracles/fhirpath/hapi-8.12.0-results.json`): `passed` for every case HAPI passes, and `failed` (parity under `path.oracle.012`) for the cases named here, which HAPI fails. A Frevo pass on one of them would be listed as `ahead-of-hapi`, not accepted as a pass of this scenario: `testQuantityUnitComparison/testQuantityUnitVsNoUnitEqual`
  is failed (the suite expects no output; HAPI finds one item,
  `hapi-8.12.0-results.json:636`) and `testQuantityNoUnitVsUnitEqual` is
  passed (`:637`)
- **AND** the specification makes both empty; recorded as `DEV-0017`, and the
  `true` as a wrong-answer hazard, `DEBT-0019`

#### Scenario: [path.quantity.012] equivalence of a quantity with a unit and one without is `true` one way and fails the other

- **WHEN** the same two expressions with the equivalence operator are evaluated
- **THEN** `Parameters.parameter[2].value ~ Parameters.parameter[0].value`
  yields the single boolean **`true`**, and
  `Parameters.parameter[0].value ~ Parameters.parameter[2].value` **fails**,
  by the same guard as `path.quantity.011`, with `doEquivalent` unboxing the
  `null`
- **AND** Frevo's required status per case follows HAPI's own recorded run (`oracles/fhirpath/hapi-8.12.0-results.json`): `passed` for every case HAPI passes, and `failed` (parity under `path.oracle.012`) for the cases named here, which HAPI fails. A Frevo pass on one of them would be listed as `ahead-of-hapi`, not accepted as a pass of this scenario; both are failed:
  `testQuantityUnitComparison/testQuantityUnitVsNoUnitEquivalent` (the suite
  expects `false`; HAPI answers `true`, `hapi-8.12.0-results.json:638`) and
  `testQuantityNoUnitVsUnitEquivalent` (the suite expects `false`; HAPI raises,
  `:639`)
- **AND** where HAPI raises a `NullPointerException`, Frevo returns an execution error, never a panic (`path.errors.003`), with no offset (`path.errors.002`) and with the message HAPI's run records, the JVM's text, exactly: `Cannot invoke "java.lang.Boolean.booleanValue()" because the return value of "org.hl7.fhir.r4.fhirpath.FHIRPathEngine.qtyEquivalent(org.hl7.fhir.r4.model.Quantity, org.hl7.fhir.r4.model.Quantity)" is null`
- **AND** the specification makes both `false`; recorded as `DEV-0017`, and the
  `true` as a wrong-answer hazard, `DEBT-0019`

#### Scenario: [path.quantity.013] unit text without a code is handled per the suite's dedicated input

- **WHEN** the three cases using `parameters-quantity-unit-text.xml` (left
  `1` with unit text `mg/dL` only; right `2 'mg/dL'` with a UCUM `system` and
  `code`) are evaluated
- **THEN** `testLessOrEqual/testLessOrEqual30` (`<=`) and
  `testGreaterThan/testGreaterThan30` (`>`) yield `true`: those operators
  compare the `unit` children, which are equal, and then the values
  (`hapi-8.12.0-results.json:669`, `:728`, both passed)
- **AND** `testLessThan/testLessThan33` (`<`) **fails**: `<` compares the `code`
  children (`FHIRPathEngine.java:2438-2440`), which differ, so it compares the
  canonical decimals, and `qtyToCanonicalDecimal` returns `null` for the
  quantity with no UCUM system (`:2301-2303`); the recursive `opLessThan` calls
  `isPrimitive()` on it (`:2414`). This is the case `path.compare.001` lists
- **AND** Frevo's required status per case follows HAPI's own recorded run (`oracles/fhirpath/hapi-8.12.0-results.json`): `passed` for every case HAPI passes, and `failed` (parity under `path.oracle.012`) for the cases named here, which HAPI fails. A Frevo pass on one of them would be listed as `ahead-of-hapi`, not accepted as a pass of this scenario: `testLessThan33` is failed (the suite expects `true`;
  HAPI raises, `hapi-8.12.0-results.json:631`), the other two passed
- **AND** where HAPI raises a `NullPointerException`, Frevo returns an execution error, never a panic (`path.errors.003`), with no offset (`path.errors.002`) and with the message HAPI's run records, the JVM's text, exactly: `Cannot invoke "org.hl7.fhir.r4.model.Base.isPrimitive()" because the return value of "java.util.List.get(int)" is null`
- **AND** the specification makes it `true`; recorded as `DEV-0017`

#### Scenario: [path.quantity.014] `comparable()` reports whether two quantities are commensurable

- **WHEN** `1 'cm'.comparable(1 '[in_i]')`, `1 'cm'.comparable(1 '[s]')` and
  `1 'cm'.comparable(1 's')` are evaluated
- **THEN** the results are `true`, `false`, `false` (`Comparable/Comparable1`,
  `Comparable2`, `Comparable3`)

#### Scenario: [path.quantity.015] comparing incommensurable units returns a boolean, not an error

- **WHEN** `1 'cm' < 1 's'` is evaluated with the UCUM data loaded
- **THEN** the result is a boolean, because the engine converts each operand to
  its canonical magnitude and compares the two decimals with no dimension check
  — `0.01 < 1` is `true`
- **AND** the engine also has a branch for a worker whose `getUcumService()`
  returns `null`: there two quantities whose `code` (for `<`) or `unit` (for
  `<=`, `>`, `>=`) lists differ compare `false` outright
  (`FHIRPathEngine.java:2443-2445`, `:2490-2492`, `:2540-2542`, `:2588-2590`).
  Neither HAPI configuration reaches it: the suite's worker has a UCUM service,
  and HAPI's server-facing worker throws (`path.quantity.019`). Frevo has no
  configuration without UCUM data (`path.quantity.016`), so the branch is not
  reachable in Frevo and is not graded
- **AND** this differs from the FHIRPath specification, which requires
  incommensurable quantities to be an error or empty; the divergence is
  recorded as `DEV-0005` and resolved as HAPI's behaviour
- **AND** no live case grades it — the only `cm`/`s` pairs in the suite are
  `Comparable/Comparable2`, `Comparable3` and `testMinus/testMinus6` — so it is
  differential-only

#### Scenario: [path.quantity.016] UCUM data comes from the pinned `ucum-essence.xml`

- **WHEN** the engine canonicalises any unit
- **THEN** it does so from the `ucum-essence.xml` the repository pins, and the
  file's version is reported with the suite result
- **AND** an absent or unparseable UCUM file fails the run rather than silently
  treating units as incomparable

#### Scenario: [path.quantity.018] a quantity whose system is not UCUM is not canonicalised

- **WHEN** two quantities whose unit codes differ are compared and at least one
  carries a `system` other than `http://unitsofmeasure.org`
- **THEN** canonicalisation yields nothing for that operand, because the engine
  returns no canonical decimal unless the system is exactly
  `http://unitsofmeasure.org`
- **AND** the engine's observed behaviour at the pinned SHA for that case is
  recorded by the harness and matched, rather than assumed; it is not the same
  as the no-UCUM-data case in `path.quantity.015`
- **AND** no live case grades it, so it is differential-only

#### Scenario: [path.quantity.019] under HAPI's server-facing worker every UCUM-consulting quantity operation raises `HAPI-0274: `

- **WHEN** a quantity operation that consults the worker's UCUM service is
  evaluated by HAPI's server-facing engine: `FhirPathR4`, the `newFhirPath()`
  of every R4 context (`hapi-fhir-structures-r4/src/main/java/org/hl7/fhir/r4/hapi/ctx/FhirR4.java:54-57`),
  built on `HapiWorkerContext` (`FhirPathR4.java:34-36`), which is also the
  worker of `SearchParamExtractorR4`
  (`hapi-fhir-jpaserver-searchparam/.../SearchParamExtractorR4.java:99-101`)
- **THEN** evaluation fails with `java.lang.UnsupportedOperationException`
  whose message is exactly `HAPI-0274: `, because
  `HapiWorkerContext.getUcumService()` throws it
  (`hapi-fhir-structures-r4/src/main/java/org/hl7/fhir/r4/hapi/ctx/HapiWorkerContext.java:372-374`)
  and the engine calls it:
  - in `qtyEqual` and `qtyEquivalent` for every pair of quantities that both
    have a value (`FHIRPathEngine.java:2265`, `:2338`), so every `=`, `!=`,
    `~` and `!~` between two valued quantities raises: `path.quantity.001` to
    `.004`, `.009` to `.012`, `.017`, `1 'mg' = 1 'mg'`, and the
    `testQuantity1` to `6` and `9` to `11` cases;
  - in `<` when the two `code` lists differ, and in `<=`, `>` and `>=` when the
    two `unit` lists differ (`:2443`, `:2490`, `:2540`, `:2588`): `6 days < 1 week`
    and `8 days > 1 week` (`path.quantity.005`), `1 'cm' < 1 's'`
    (`path.quantity.015`) and `testLessThan33` (`path.quantity.013`) raise;
  - in `*` and `/` of two quantities (`:2793`, `:3013`: `path.quantity.006`,
    `.007`, `.008`) and in `comparable()` with a UCUM `system`
    (`:4631`, `path.quantity.014`)
- **AND** an ordered comparison whose `code` (for `<`) or `unit` (for the other
  three) lists are equal never consults the worker and answers by value:
  `testLessOrEqual30` and `testGreaterThan30` (`path.quantity.013`) are `true`
  under the server-facing worker as under the suite's
- **AND** the exception is not a `FHIRException`, so `FhirPathR4` does not wrap
  it as `HAPI-0255: ` (it catches only `FHIRException`, `FhirPathR4.java:66-70`),
  and `FhirPathFilterInterceptor`, which catches only
  `FhirPathExecutionException` (`FhirPathFilterInterceptor.java:80-84`), lets
  it escape. What the server then answers over HTTP is `add-server-skeleton`'s
- **AND** Frevo does not reproduce these failures: in every configuration it
  answers as the suite's worker does (the other `path.quantity.*` scenarios),
  recorded as `DEV-0029`, pending the owner (`tasks/blockers.md`, 2026-11-20,
  "quantity operations under HAPI's server-facing worker")
- **AND** every other `path.quantity.*` scenario states the engine's answer
  under the **suite's worker**, the `TestingUtilities` context that
  `FHIRPathTests` evaluates with, which carries a UCUM service
  (`org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/test/utils/TestingUtilities.java:89`);
  that is what the pinned suite's expectations and HAPI's recorded run reflect
- **AND** no live case grades it: the suite runs under its own worker. It is
  differential-only, against a live HAPI 8.12.0

#### Scenario: [path.quantity.017] a quantity with an unknown unit code does not crash

- **WHEN** `1 'zzz' = 1 'zzz'` and `1 'zzz' = 1 'cm'` are evaluated
- **THEN** the engine behaves as it does at the pinned SHA, and the harness
  records which behaviour that is

### Requirement: Boundary functions

The engine SHALL implement `lowBoundary`, `highBoundary` and `precision` over decimals, quantities, dates, dateTimes and times as `FHIRPathEngine` does at the pinned SHA, including its text-edited decimal boundaries with no carry, no borrow normalisation and only a lower-case `e` read as an exponent (`DEV-0025`).

#### Scenario: [path.boundary.001] `lowBoundary` on a decimal widens to the implied precision

- **WHEN** the `LowBoundary` group's 31 decimal cases are evaluated, including
  `1.587.lowBoundary()` yielding `1.58650000`, `1.587.lowBoundary(6)` yielding
  `1.586500`, `1.587.lowBoundary(2)` yielding `1.58` and `1.587.lowBoundary(0)`
  yielding `1`
- **THEN** every case HAPI passes yields its stated output (22 of 31)
- **AND** Frevo's required status per case follows HAPI's own recorded run
  (`oracles/fhirpath/hapi-8.12.0-results.json:932-962`): `failed` (parity under
  `path.oracle.012`) for `LowBoundaryDecimalCarry1`-`3`,
  `LowBoundaryDecimalBorrow1`-`2` and `LowBoundaryDecimalExponent1`-`4`, whose
  values and errors are `path.boundary.004` and `path.boundary.005`, and
  `passed` for the rest
- **AND** the result keeps the requested scale: `1.587.lowBoundary()` is not the
  decimal `1.5865`

#### Scenario: [path.boundary.002] `lowBoundary(n)` with an out-of-range precision yields empty

- **WHEN** `1.587.lowBoundary(-1)`, `1.587.lowBoundary(32)` and
  `1.587.lowBoundary(39)` are evaluated
- **THEN** each yields an empty collection (`LowBoundary/LowBoundaryDecimal3`,
  `LowBoundaryDecimal5`, `LowBoundaryDecimal6`)
- **AND** the range is checked only for a decimal or an integer focus, and is
  `0` to `16` inclusive (`precision >= 0 && precision < 17`,
  `FHIRPathEngine.java:4310`, `:4314`, `:4354`, `:4358`), for both functions; a
  quantity's precision is not checked (`path.boundary.012`), and a date, dateTime
  or time precision is checked against its own list (`path.boundary.015`,
  `path.boundary.016`)

#### Scenario: [path.boundary.003] `lowBoundary` on a negative decimal moves away from zero

- **WHEN** `(-1.587).lowBoundary()` and `(-1.587).lowBoundary(2)` are evaluated
- **THEN** the results are `-1.58750000` and `-1.59`
  (`LowBoundary/LowBoundaryNegDecimalDefault`, `LowBoundaryNegDecimal2`)

#### Scenario: [path.boundary.004] `lowBoundary` neither carries nor normalises a borrow

The decimal boundary is built as text (`Utilities.lowBoundaryForDecimal`,
`applyPrecision`, `minusOne`, `Utilities.java:1631-1695` at `73c432b5`):
rounding a digit up adds one to its character code, so a `9` becomes `:`;
subtracting one turns trailing `0`s into `9`s and keeps a leading `0`. The text
is then read as a decimal and kept as that decimal's lexical form; a text that
is not a decimal is an execution error carrying Java's `BigDecimal` message.

- **WHEN** `(-9.96).lowBoundary(1)`, `(-99.96).lowBoundary(1)` and
  `(-9).lowBoundary(0)` are evaluated
- **THEN** each is an execution error whose message is
  `Character : is neither a decimal digit number, decimal point, nor "e"
  notation exponential mark.`, because the high boundary of the magnitude
  rounds a `9` up to `:` (`LowBoundary/LowBoundaryDecimalCarry1`,
  `LowBoundaryDecimalCarry2`, `LowBoundaryDecimalCarry3`; `failed` on HAPI,
  `hapi-8.12.0-results.json:954-956`)
- **AND** `10.lowBoundary(0)` is the decimal `09` and `100.lowBoundary(2)` is
  the decimal `099.50`, each with that lexical form
  (`LowBoundary/LowBoundaryDecimalBorrow1`, `LowBoundaryDecimalBorrow2`;
  `failed` on HAPI with `Value should be 9 but was 09` and
  `Value should be 99.50 but was 099.50`, `results.json:957-958`)
- **AND** all five cases are `failed` for Frevo too (parity under
  `path.oracle.012`); the suite's expectations (`-10.0`, `-100.0`, `-10`, `9`,
  `99.50`) are the specification's, recorded as `DEV-0025`
- **AND** where the subtraction leaves no leading `0` the result is exact:
  `1.587.lowBoundary(2)` is `1.58` and `1.lowBoundary(0)` is `0`

#### Scenario: [path.boundary.005] `lowBoundary` on an exponent-form decimal appends the exponent's digits

Only the first lower-case `e` is split off; the boundary is computed on the
mantissa and the exponent's text is appended to the result with no `e`
(`Utilities.java:1635-1647`, `:1701-1713`). An upper-case `E` is an
ordinary character of the text. The four values of
`parameters-example-decimals.xml` are `9.5e2`, `1.5e-3`, `-9.5e2` and `1.5E2`.

- **WHEN** `Parameters.parameter[0].value.lowBoundary(0)`,
  `Parameters.parameter[1].value.lowBoundary()`,
  `Parameters.parameter[2].value.lowBoundary(0)` and
  `Parameters.parameter[3].value.lowBoundary(1)` are evaluated over
  `parameters-example-decimals.xml`
- **THEN** the results are, in order: the decimal `92`; an execution error
  `Character - is neither a decimal digit number, decimal point, nor "e"
  notation exponential mark.` (the text is `1.45000000-3`); an execution error
  naming `Character :`; and the decimal `1.5`
  (`LowBoundary/LowBoundaryDecimalExponent1` through `Exponent4`, `failed` on
  HAPI with these values and errors, `hapi-8.12.0-results.json:959-962`)
- **AND** all four are `failed` for Frevo too (parity under `path.oracle.012`);
  the suite's expectations (`945`, `0.00145000`, `-955`, `145.0`) are the
  specification's, recorded as `DEV-0025`, and the two that answer (`92`,
  `1.5`) are wrong numbers rather than errors, recorded as `DEBT-0016`

#### Scenario: [path.boundary.006] `highBoundary` on a decimal widens to the implied precision

- **WHEN** the `HighBoundary` group's 27 decimal cases are evaluated, including
  `1.587.highBoundary()` yielding `1.58750000`, `1.587.highBoundary(2)` yielding
  `1.59`, `1.587.highBoundary(6)` yielding `1.587500`,
  `(-1.587).highBoundary()` yielding `-1.58650000` and
  `(-1.587).highBoundary(2)` yielding `-1.58`
- **THEN** every case HAPI passes yields its stated output (18 of 27)
- **AND** Frevo's required status per case follows HAPI's own recorded run
  (`oracles/fhirpath/hapi-8.12.0-results.json:969-995`): `failed` (parity under
  `path.oracle.012`) for `HighBoundaryDecimalCarry1`-`4` and
  `HighBoundaryDecimalExponent1`-`5`, whose values and errors are
  `path.boundary.007` and `path.boundary.011`, and `passed` for the rest
- **AND** `1.587.highBoundary(-1)` and `1.587.highBoundary(39)` are empty, as
  for `lowBoundary` (`path.boundary.002`), and the result keeps the requested
  scale: `1.587.highBoundary()` is not the decimal `1.5875`

#### Scenario: [path.boundary.007] `highBoundary` does not carry past a `9`

- **WHEN** `9.95.highBoundary(1)`, `0.996.highBoundary(1)`,
  `99.96.highBoundary(1)` and `9.highBoundary(0)` are evaluated
- **THEN** each is an execution error whose message is
  `Character : is neither a decimal digit number, decimal point, nor "e"
  notation exponential mark.`, because the digit rounded up is a `9`
  (`Utilities.java:1672`) (`HighBoundary/HighBoundaryDecimalCarry1` through
  `Carry4`; `failed` on HAPI, `hapi-8.12.0-results.json:987-990`)
- **AND** all four are `failed` for Frevo too (parity under `path.oracle.012`);
  the suite's expectations (`10.0`, `1.0`, `100.0`, `10`) are the
  specification's, recorded as `DEV-0025`
- **AND** a round-up of a digit below `9` is exact: `1.587.highBoundary(2)` is
  `1.59` and `1.highBoundary(0)` is `2`

#### Scenario: [path.boundary.008] boundary functions work on quantities

- **WHEN** the `LowBoundary` and `HighBoundary` groups' quantity cases are
  evaluated, `1.587 'cm'.lowBoundary(8)` and `1.587 'm'.highBoundary(8)`
- **THEN** every case yields its stated output, and the two results are the
  quantities `1.58650000 'cm'` and `1.58750000 'm'`: the value is computed as
  for a decimal and the unit, code and system are copied unchanged

#### Scenario: [path.boundary.009] boundary functions on a date or dateTime yield a dateTime

Both functions build every date and dateTime boundary with
`new DateTimeType(...)` (`FHIRPathEngine.java:4318-4320`, `:4362-4364`) and
every time boundary with `new TimeType(...)` (`:4322`, `:4366`). The default
precision is 10 for a date, 17 for a dateTime and 9 for a time; a precision of
10 or less drops the offset, a higher one keeps a stated offset and otherwise
appends `+14:00` for `lowBoundary` and `-12:00` for `highBoundary`
(`DateTimeUtil.java:83-89`, `:174-180`).

- **WHEN** the `LowBoundary` and `HighBoundary` groups' date, dateTime and time
  cases are evaluated
- **THEN** every case yields its stated output, as it does on HAPI (10 of 10)
- **AND** `@2014.lowBoundary(6)` and `@2014.highBoundary(6)` are each a single
  item of FHIR type `dateTime` (not `date`) with the value `2014-01` and
  `2014-12`
- **AND** `@2014-01-01T08.lowBoundary(8)` is a single `dateTime` with the value
  `2014-01-01`, and `@2014.lowBoundary()` is a single `dateTime` with the value
  `2014-01-01`
- **AND** `@2014-01-01T08.lowBoundary(17)` is the `dateTime`
  `2014-01-01T08:00:00.000+14:00`, `@2014-01-01T08.highBoundary(17)` is
  `2014-01-01T08:00:59.999-12:00`, and
  `@2014-01-01T08:05+08:00.lowBoundary(17)` is
  `2014-01-01T08:05:00.000+08:00`
- **AND** `@T10:30.lowBoundary(9)` and `@T10:30.highBoundary(9)` are the `time`
  values `10:30:00.000` and `10:30:59.999`
- **AND** a high boundary widened from month to day precision uses the month's
  length, leap years included: `@2014-02.highBoundary(8)` is `2014-02-28` and
  `@2016-02.highBoundary(8)` is `2016-02-29`, each a `dateTime`
- **AND** the specification's result type for a date input is a date; recorded
  as `DEV-0025`, and the type-test consequence as `DEBT-0016`

#### Scenario: [path.boundary.010] an integer's boundaries are decimals half a unit away

- **WHEN** `1.lowBoundary(5)` and `1.highBoundary()` are evaluated
- **THEN** the results are `0.50000` and `1.50000000`
  (`LowBoundary/LowBoundaryDecimal9`, `HighBoundary/HighBoundaryDecimal8`)

#### Scenario: [path.boundary.011] `highBoundary` on an exponent-form decimal appends the exponent's digits

The exponent rule of `path.boundary.005`, for `highBoundary`
(`Utilities.java:1701-1713`).

- **WHEN** `Parameters.parameter[0].value.highBoundary(0)`,
  `Parameters.parameter[0].value.highBoundary()`,
  `Parameters.parameter[1].value.highBoundary()`,
  `Parameters.parameter[2].value.highBoundary(0)` and
  `Parameters.parameter[3].value.highBoundary(1)` are evaluated over
  `parameters-example-decimals.xml`
- **THEN** the results are, in order: an execution error naming `Character :`;
  the decimal `9.550000002`; an execution error naming `Character -`; the
  decimal `-92`; and the decimal `1.6`
  (`HighBoundary/HighBoundaryDecimalExponent1` through `Exponent5`, `failed` on
  HAPI with these values and errors, `hapi-8.12.0-results.json:991-995`)
- **AND** all five are `failed` for Frevo too (parity under `path.oracle.012`);
  the suite's expectations (`955`, `955.00000000`, `0.00155000`, `-945`,
  `155.0`) are the specification's, recorded as `DEV-0025`, and the three that
  answer are wrong numbers rather than errors, recorded as `DEBT-0016`
- **AND** the oracle is the suite's `HighBoundary` group

#### Scenario: [path.boundary.012] a quantity's boundary precision is not range-checked

`funcLowBoundary` and `funcHighBoundary` apply the `0`-`16` check of
`path.boundary.002` only to a decimal or an integer; the quantity arm passes
the precision straight to the decimal computation (`FHIRPathEngine.java:4323-4327`,
`:4367-4371`).

- **WHEN** `1.587 'cm'.lowBoundary(-1)` and `1.587 'cm'.highBoundary(-1)` are
  evaluated
- **THEN** each is a single Quantity with the unit, code `cm` and system
  `http://unitsofmeasure.org` copied and **no value**, not an empty collection
  and not an error: `applyPrecision` (`Utilities.java:1653-1680`) cuts the text
  to `""`, and `new DecimalType("")` sets a null value for a blank string
  (`DecimalType.java:153-157`), which `:4326` / `:4370` set on the copy
- **AND** `12.5 'cm'.lowBoundary(-1)` and `12.5 'cm'.highBoundary(-1)` are each
  the quantity `1 'cm'`: a negative precision truncates the text rather than
  rounding at the tens
- **AND** `1.587 'cm'.lowBoundary(-2)` and `1.587 'cm'.highBoundary(-2)` are
  execution errors: the cut index is `-1` and `charAt(-1)` raises
  `StringIndexOutOfBoundsException`, whose message text depends on the JDK and
  is not compared
- **AND** `1.587 'cm'.lowBoundary(20)` is the quantity
  `1.58650000000000000000 'cm'` (twenty fractional digits), where
  `1.587.lowBoundary(20)` is empty
- **AND** the specification's out-of-range result is empty, as the suite's
  decimal cases `LowBoundaryDecimal3` and `LowBoundaryNegDecimal3` show;
  recorded as `DEV-0025`, and the valueless or truncated quantity, a wrong
  answer rather than an error, as `DEBT-0016`
- **AND** no suite case grades it, so it is differential-only, with seam
  assertions in the boundary tests

#### Scenario: [path.boundary.013] `precision()` on an empty or many-item focus raises, naming `highBoundary`

`funcPrecision` opens with `if (focus.size() != 1) throw
makeExceptionPlural(..., "highBoundary", ...)` (`FHIRPathEngine.java:4379-4381`).

- **WHEN** `{}.precision()` and `(1.5 | 2.5).precision()` are evaluated
- **THEN** each is an execution error whose message is, in full,
  `Error evaluating FHIRPath expression: focus for highBoundary can only have
  one value, but has 0 values (@char 4)` and
  `Error evaluating FHIRPath expression: focus for highBoundary can only have
  one value, but has 2 values (@char 15)`, naming `highBoundary` and not
  `precision` (`FHIRPATH_FOCUS_other`, `Messages.properties:332`). The holder
  is the `precision` function node, whose start is set, so `makeExceptionPlural`
  appends its column under `path.errors.002`'s rule (`:572-582`): `{}` is one
  token and `.` another, so `precision` starts at column 4 (byte offset 3); in
  `(1.5 | 2.5).precision()` the two spaces count 2 each, so it starts at
  column 15 (byte offset 12)
- **AND** `{}.lowBoundary()` and `{}.highBoundary()` are empty, not errors
  (`:4291-4293`, `:4335-4337`)
- **AND** `(1.5 | 2.5).lowBoundary()` and `(1.5 | 2.5).highBoundary()` are
  execution errors with the messages `Error evaluating FHIRPath expression:
  focus for lowBoundary can only have one value, but has 2 values (@char 15)`
  and the same naming `highBoundary` (`:4294-4296`, `:4338-4340`)
- **AND** the specification's result for an empty input is empty; recorded as
  `DEV-0025`
- **AND** no suite case grades it, so it is differential-only

#### Scenario: [path.boundary.014] a focus of a type the functions do not take is empty, not an error

The last arm of each function builds an exception with `makeException(...)`
and does not throw it (`FHIRPathEngine.java:4329`, `:4373`, `:4391`), so the
result is empty. The static check is stricter: `checkContextContinuous`
(`:3896-3900`) admits a decimal, date, dateTime, time or Quantity focus, and an
integer for the two boundaries but not for `precision` (`:3716`, `:3731`).

- **WHEN** `1.precision()`, `'abc'.precision()`, `'abc'.lowBoundary()` and
  `true.highBoundary()` are evaluated with the static check skipped, as
  `FhirPathR4` evaluates (`path.check.013`)
- **THEN** each yields an empty collection
- **AND** `1 'cm'.precision()` yields an empty collection and passes the static
  check, because a Quantity focus is admitted by the check but not by
  `funcPrecision`
- **AND** with the static check run, `1.precision()` and `'abc'.lowBoundary()`
  are rejected with `FHIRPATH_CONTINUOUS_ONLY`, while `1.lowBoundary()` passes
- **AND** dispatch is by the exact `fhirType()`, compared case-insensitively
  with no subtype following (`Base.hasType`, `Base.java:159-165`), so a model
  element of a specialised type is not taken either: with the static check
  skipped, `Patient.telecom.rank.first().lowBoundary()`,
  `.highBoundary()` and `.precision()` (a `positiveInt`) and
  `Patient.gender.lowBoundary()`, `.highBoundary()` and `.precision()` (a
  `code`) over `patient-example.xml`, and
  `Patient.meta.lastUpdated.lowBoundary()`, `.highBoundary()` and
  `.precision()` (an `instant`) over `patient-glossy-example.xml`, each yield an
  empty collection; an `unsignedInt` element is the same. An `instant` is not
  treated as a dateTime and a `positiveInt` or `unsignedInt` is not treated as
  an integer, whatever the model's type hierarchy says (`path.types.*`)
- **AND** no suite case grades it, so it is differential-only

#### Scenario: [path.boundary.015] a date or dateTime boundary precision is one of seven values

`DateTimeUtil.applyDatePrecision` (`DateTimeUtil.java:130-144`) cuts the widened
text at 4 (year), 6 or 7 (month), 8 or 10 (day), 14 (17 characters) or 17 (the
whole text), and throws for anything else.

- **WHEN** `@2014-01-15.lowBoundary(7)` and `@2014.lowBoundary(10)` are
  evaluated
- **THEN** the results are the dateTimes `2014-01` and `2014-01-01`
- **AND** `@2014.lowBoundary(5)`, `@2014.highBoundary(0)`,
  `@2014-01-01T08.lowBoundary(12)` and `@2014.lowBoundary(20)` are each an
  execution error with the message
  `Unsupported Date precision for boundary operation: <n>`; there is no range
  check returning empty
- **AND** precision 14 keeps 17 characters and then appends the offset, and
  `new DateTimeType` reads the two characters after the minute's `:` as the
  seconds (`BaseDateTimeType.java:330-342`, `:484-492`, `:540-553`), so the
  result depends on the first two characters of the offset:
  - `@2014-01-01T08.lowBoundary(14)` is a single `dateTime` whose lexical form
    is `2014-01-01T08:00:+14:00`: the seconds text `+1` parses as `1`
  - `@2014-01-01T08.highBoundary(14)` is an execution error with the message
    `Invalid date/time format: "2014-01-01T08:00:-12:00"`: the seconds text
    `-1` is out of range (`BaseDateTimeType.java:796-798`)
  - with a stated offset, any `+` offset and any `-` offset whose hour starts
    with `0` parse: `@2014-01-01T08:05+08:00.lowBoundary(14)` is the dateTime
    `2014-01-01T08:05:+08:00` and `@2014-01-01T08:05-05:00.highBoundary(14)`
    is the dateTime `2014-01-01T08:05:-05:00`, while
    `@2014-01-01T08:05-10:00.highBoundary(14)` is the `Invalid date/time
    format` error for its text
  - with a stated `Z`, `@2014-01-01T08:05Z.lowBoundary(14)` builds
    `2014-01-01T08:05:Z`, shorter than the 19 characters a seconds field
    needs, and is the `Invalid date/time format` error for that text
- **AND** the specification's seconds precision 14 yields a well-formed
  dateTime at seconds; recorded as `DEV-0025`, and the malformed dateTimes
  that parse as `DEBT-0016`
- **AND** no suite case grades it, so it is differential-only

#### Scenario: [path.boundary.016] a time boundary precision below 9 keeps a trailing separator

`DateTimeUtil.applyTimePrecision` (`DateTimeUtil.java:146-154`) cuts the
widened text `hh:mm:ss.fff` at 3, 6 or 9 characters for precisions 2, 4 and 6,
and keeps it whole for 9; `new TimeType(...)` does not validate the text.

- **WHEN** `@T10:30.lowBoundary(2)`, `@T10:30.lowBoundary(4)` and
  `@T10:30.lowBoundary(6)` are evaluated
- **THEN** the results are the times `10:`, `10:30:` and `10:30:00.`, each with
  that lexical form
- **AND** `@T10:30.highBoundary(2)` and `@T10:30.highBoundary(6)` are the times
  `10:` and `10:30:59.`
- **AND** `@T10:30.lowBoundary(3)`, `@T10:30.lowBoundary(8)` and
  `@T10:30.lowBoundary(17)` are each an execution error with the message
  `Unsupported Time precision for boundary operation: <n>`
- **AND** the specification's results are the times `10`, `10:30` and
  `10:30:00`; recorded as `DEV-0025`, and the malformed values as `DEBT-0016`
- **AND** no suite case grades it, so it is differential-only

#### Scenario: [path.boundary.017] the precision argument must be a single value read as an integer

Each function evaluates its argument and throws unless it yields exactly one
item, with a message that names `lowBoundary` in both functions
(`FHIRPathEngine.java:4300-4301`, `:4344-4345`); the item's primitive text is
then read with `Integer.parseInt` (`:4303`, `:4347`).

- **WHEN** `1.587.lowBoundary(1 | 2)`, `1.587.highBoundary(1 | 2)` and
  `1.587.lowBoundary({})` are evaluated with the static check skipped
- **THEN** each is an execution error whose message is, in full,
  `Error evaluating FHIRPath expression: The parameter type Multiple Values is
  not legal for lowBoundary parameter 0. expecting integer (@char 7)`
  (`FHIRPATH_WRONG_PARAM_TYPE`, `Messages.properties:371`), naming
  `lowBoundary` for `highBoundary` too. The holder is the function node
  (`:4301`, `:4345`): `1.587` is one token, so the function name starts at
  column 7 (byte offset 6), under `path.errors.002`'s rule
- **AND** with the static check skipped, `1.587.lowBoundary('2')` is the decimal
  `1.58`, because the string's text parses as an integer, and
  `1.587.lowBoundary(1.5)` is an execution error with the message
  `For input string: "1.5"`
- **AND** no suite case grades it, so it is differential-only

#### Scenario: [path.boundary.018] a zero decimal takes a fixed half-unit text

Both decimal boundaries test `isZero` first (`Utilities.java:1639`, `:1705`,
`:1718-1720`), which strips `.`, `-` and `0` and so takes `0`, `0.0`, `0.000`
and `(-0.0)` alike. The zero branch does not go through `minusOne`: it applies
the precision to the fixed text `-0.5000000000000000000000000` (low) or
`0.50000000000000000000000000000` (high), and a lower-case exponent's digits
are not appended.

- **WHEN** `0.lowBoundary()`, `0.highBoundary()`, `0.lowBoundary(0)` and
  `0.highBoundary(0)` are evaluated
- **THEN** the results are the decimals `-0.50000000`, `0.50000000`, `-0` and
  `1`, each with that lexical form (`1` because the `5` rounds the leading `0`
  up)
- **AND** `0.0.lowBoundary(1)`, `(-0.0).lowBoundary(1)` and
  `0.000.highBoundary(1)` are `-0.5`, `-0.5` and `0.5`
- **AND** no suite case grades it (`LowBoundaryDecimal14` and `15` are
  `0.0034`, not zero), so it is differential-only

#### Scenario: [path.boundary.019] a quantity with no value has no boundary

The quantity arm reads the value with `getNamedValue`
(`FHIRPathEngine.java:4643-4649`), which is `null` when the element has no
`value`, and `lowBoundaryForDecimal` / `highBoundaryForDecimal` throw for a
null or empty text (`Utilities.java:1632-1634`, `:1698-1700`).

- **WHEN** `Observation.value.lowBoundary()` and
  `Observation.value.highBoundary()` are evaluated over an Observation whose
  `valueQuantity` carries `unit`, `system` and `code` but no `value`
- **THEN** each is an execution error with the message
  `Unable to calculate lowBoundary for a null decimal string` and
  `Unable to calculate highBoundary for a null decimal string` respectively
- **AND** this holds for any precision argument, because the text is tested
  before the precision is used
- **AND** no suite case or fixture has such a quantity, so it is
  differential-only, with a hand-built input for the seam assertions

### Requirement: Type operators and reflection

The engine SHALL implement `is`, `as`, `ofType` and `type()` over both the `System` and `FHIR` namespaces, resolving type names through the model's type hierarchy rather than through any concrete Rust type.

#### Scenario: [path.types.001] `is` tests the System namespace for a literal

- **WHEN** `true is Boolean`, `true is System.Boolean`, `1.is(Integer)` and
  `1.is(System.Integer)` are evaluated
- **THEN** each yields `true` (`testType` group, `testTypes` group)

#### Scenario: [path.types.002] `is` tests the FHIR namespace for a model element

- **WHEN** `Patient.active.is(boolean)`, `Patient.active.is(FHIR.boolean)`,
  `Patient.active.is(Boolean).not()` and
  `Patient.active.is(System.Boolean).not()` are evaluated over
  `patient-example.xml`
- **THEN** each yields `true` (`testType/testType11` through `testType14`)

#### Scenario: [path.types.003] `is` on a resource tests the resource type

- **WHEN** `Patient.is(Patient)`, `Patient.is(FHIR.Patient)` and the
  backtick-delimited form are evaluated over `patient-example.xml`
- **THEN** each yields `true` (`testType/testType17` through `testType19`)

#### Scenario: [path.types.004] `is` follows the FHIR type hierarchy

- **WHEN** `Patient.gender.is(code)`, `Patient.gender.is(string)` and
  `Patient.gender.is(id)` are evaluated over `patient-example.xml`
- **THEN** the results are `true`, `true`, `false`
  (`testInheritance/testFHIRPathIsFunction1` through `3`)

#### Scenario: [path.types.005] `is` distinguishes `uri` from `url`

- **WHEN** `Questionnaire.url.is(uri)` and `Questionnaire.url.is(url)` are
  evaluated over `questionnaire-example.xml`
- **THEN** the results are `true` and `false`
  (`testInheritance/testFHIRPathIsFunction4`, `5`)

#### Scenario: [path.types.006] `is` on an extension's value resolves the concrete choice type

- **WHEN** `Observation.extension('http://example.com/fhir/StructureDefinition/patient-age').value is Age`,
  `... is Quantity` and `... is Duration` are evaluated over
  `observation-example.xml`
- **THEN** the results are `true`, `true`, `false`
  (`testInheritance/testFHIRPathIsFunction8` through `10`)
- **AND** the static check of `extension('...')` (an absolute URL) resolves the
  extension's definition from the model and, when it is not found, adds a
  warning and types the result as a plain `Extension` rather than failing
  (`FHIRPathEngine.java:3556-3591`, `path.utility.009`)

#### Scenario: [path.types.007] `as` yields the value when the type matches and empty when it does not

- **WHEN** `Patient.gender.as(code)`, `Patient.gender.as(string)` and
  `Patient.gender.as(id)` are evaluated over `patient-example.xml`
- **THEN** the first yields the `code` `male` and the other two yield empty
  (`testInheritance/testFHIRPathAsFunction12`, `11`, `13`)

#### Scenario: [path.types.008] `as` on a `string`-typed element does not match `code`

- **WHEN** `ValueSet.version.as(string)` and `ValueSet.version.as(code)` are
  evaluated over `valueset-example-expansion.xml`
- **THEN** the results are the string `20150622` and an empty collection
  (`testInheritance/testFHIRPathAsFunction14`, `15`)

#### Scenario: [path.types.009] `type()` reports a namespace and a name

- **WHEN** `1.type().namespace`, `1.type().name`, `'1'.type().name`,
  `true.type().name`, `Patient.active.type().namespace`,
  `Patient.active.type().name`, `Patient.type().namespace` and
  `Patient.type().name` are evaluated over `patient-example.xml`
- **THEN** the results are `System`, `Integer`, `String`, `Boolean`, `FHIR`,
  `boolean`, `FHIR`, `Patient` (`testType` group)

#### Scenario: [path.types.010] the `is` operator and the `is()` function agree

- **WHEN** `Patient.gender is code` and `Patient.gender.is(code)` are evaluated
- **THEN** both yield `true`

#### Scenario: [path.types.011] `is` with an unknown type name passes the check and is `false`; `as` and `ofType` raise

- **WHEN** `Patient.gender is NoSuchType` is checked and then evaluated
- **THEN** the static check **passes**: the right operand of `is` and `as` is
  read by `executeTypeName`, which consults nothing
  (`FHIRPathEngine.java:1627-1628`), and `operateTypes` `case Is` returns a
  boolean after a cardinality warning (`:2013-2015`)
- **AND** evaluation yields `false` (`opIs`, `:1940`; the function form `is()`
  likewise, `:5095`)
- **WHEN** `Patient.gender as NoSuchType`, `Patient.gender.as(NoSuchType)` and
  `Patient.gender.ofType(NoSuchType)` are evaluated
- **THEN** each fails at **evaluation** with "The type <name> is not valid"
  (`FHIRPATH_INVALID_TYPE`, raised after `isKnownType` fails: `:1881-1883`,
  `:5163-5165`, `:5206-5208`); the function forms name `FHIR.NoSuchType` and the
  operator form the name as written
- **AND** the FHIRPath specification makes an unknown type an error in every
  form; recorded as `DEV-0024` and resolved as HAPI's behaviour

#### Scenario: [path.types.012] `is` on an empty collection yields empty

- **WHEN** `{} is Boolean` is evaluated
- **THEN** the result is an empty collection

#### Scenario: [path.types.013] the whole `testTypes`, `testType` and `testInheritance` surface holds

- **WHEN** the `testTypes`, `testType` and `testInheritance` groups' cases are
  evaluated
- **THEN** every case HAPI passes yields its stated output
- **AND** the two cases HAPI fails stay `failed` in Frevo (parity under
  `path.oracle.012`): `testStringQuantityMonthToQuantityEquivalent` and
  `testStringQuantityYearToQuantityEquivalent`
  (`hapi-8.12.0-results.json:192-193`, `qtyEquivalent` returning `null`,
  `path.equality.004`, `DEV-0017`); every other case in the three groups passes
  in HAPI's recorded run
- **AND** the `toQuantity` cases need the quantity equality and UCUM algebra of
  tasks 43 and 44, and are graded when those land

### Requirement: Environment variables

The engine SHALL resolve the environment variables `FHIRPathEngine` resolves internally, SHALL delegate any other name to the host service, and SHALL fail rather than yield empty for a name nothing resolves.

#### Scenario: [path.env.001] `%sct`, `%loinc` and `%ucum` are the canonical system URLs

- **WHEN** `%sct = 'http://snomed.info/sct'`,
  `%loinc = 'http://loinc.org'` and
  `%ucum = 'http://unitsofmeasure.org'` are evaluated
- **THEN** each yields `true` (`testVariables/testVariables1` through `3`)

#### Scenario: [path.env.002] a value-set shorthand expands to an HL7 ValueSet URL

- **WHEN** a backtick-delimited `vs-administrative-gender` environment variable
  is compared with
  `'http://hl7.org/fhir/ValueSet/administrative-gender'`
- **THEN** the result is `true` (`testVariables/testVariables4`)

#### Scenario: [path.env.003] a code-system shorthand expands to an HL7 base URL

- **WHEN** a backtick-delimited `cs-` environment variable is evaluated
- **THEN** the result is `http://hl7.org/fhir/` followed by the name inside the
  backticks

#### Scenario: [path.env.004] an extension shorthand expands to an HL7 StructureDefinition URL

- **WHEN** a backtick-delimited `ext-` environment variable is evaluated
- **THEN** the result is
  `http://hl7.org/fhir/StructureDefinition/` followed by the name inside the
  backticks

#### Scenario: [path.env.005] `%us-zip` is the engine's built-in ZIP pattern

- **WHEN** `%us-zip` is evaluated
- **THEN** the result is the string `[0-9]{5}(-[0-9]{4}){0,1}`

#### Scenario: [path.env.006] `%resource` is the resource the focus is inside

- **WHEN** the six live `isSemanticChecks` cases that reference
  `%resource.status` — `isGuardAndOtherImplies`, `isGuardOtherAndImplies`,
  `isGuardOrOtherImpliesBad`, `isGuardOrOtherImpliesNoParensBad`,
  `isGuardOrInnerImplies` and `isGuardOrInnerImpliesBad` — are evaluated with
  `Observation.effective` as the context over `observation-example.xml`
- **THEN** each is graded as HAPI's own run grades it
  (`oracles/fhirpath/hapi-8.12.0-results.json:1057-1062`): `isGuardOrOtherImpliesBad`
  and `isGuardOrOtherImpliesNoParensBad` are `passed`; `isGuardAndOtherImplies`
  and `isGuardOtherAndImplies` are `failed` (parity under `path.oracle.012`),
  because the check rejects `$this.toString()` first: an `is` guard does **not**
  narrow the type the other operand is checked against (`path.check.005`;
  `executeType` has no guard logic and `toString` is checked against the unnarrowed focus type, `FHIRPathEngine.java:3639`);
  `isGuardOrInnerImplies` and `isGuardOrInnerImpliesBad` are
  `hapi-cannot-pass` and are graded on Frevo's own run (`path.oracle.016`)
- **AND** the `%resource` resolution those cases exist for is shown directly:
  `%resource.status` evaluated over `observation-example.xml` with the
  `Observation` as the focus yields `final`, and the check types it against the
  context's resource type, `Observation`, so it passes
- **AND** the grading selector is those six cases; the `testDollarResource`
  group this scenario previously named is commented out of the pinned suite and
  is not one of the 1,191

#### Scenario: [path.env.007] `%resource` with no focus resource is an execution error

- **WHEN** `%resource` is evaluated with no focus resource
- **THEN** evaluation fails, naming `%resource`: "Cannot use %resource in this
  context because no focus resource" (`FHIRPATH_CANNOT_USE`,
  `FHIRPathEngine.java:1763`)
- **AND** the static **check** rejects the same expression first, with the same
  message (`:3143-3145`), so a test of the evaluation error evaluates with the
  check skipped, as `FhirPathR4` does (`path.check.013`); an expression that
  passes the check and is then evaluated never reaches this error

#### Scenario: [path.env.008] `%rootResource` is the outermost resource

- **WHEN** `%rootResource` is evaluated with a contained resource as the focus
- **THEN** the result is the enclosing resource, not the contained one
- **AND** the resource is identified by `%rootResource.type().name` or
  `%rootResource.id`; `%rootResource.resourceType` is **empty**, not the type
  name: HAPI has no `resourceType` child (`getChildrenByName` is
  `item.listChildrenByName(name, false)`, `FHIRPathEngine.java:255-278`, and
  the R4 `Resource` declares only `id`, `meta`, `implicitRules` and `language`).
  The static check rejects it first ("The name 'resourceType' is not valid for
  any of the possible types: [...Patient]"), so the empty result is seen with
  the check skipped

#### Scenario: [path.env.009] `%rootResource` with no root resource is an execution error

- **WHEN** `%rootResource` is evaluated with no root resource
- **THEN** evaluation fails, naming `%rootResource`: "Cannot use %rootResource
  in this context because no focus rootResource" (`FHIRPATH_CANNOT_USE`,
  `FHIRPathEngine.java:1768`)
- **AND** the static **check** rejects it first with the same message
  (`:3148-3150`); test the evaluation error with the check skipped

#### Scenario: [path.env.010] `%context` is the node evaluation started from

- **WHEN** `%context` is evaluated with a `Patient.name` as the starting context
- **THEN** the result is that `HumanName`
- **AND** `%context.resourceType` is likewise empty (no `resourceType` child,
  `FHIRPathEngine.java:255-278`) and rejected by the check with "The name
  'resourceType' is not valid ..."; identify the node with `%context.type().name`

#### Scenario: [path.env.011] an unknown environment variable with no host service is an execution error

- **WHEN** `%nosuchvariable` is evaluated with no host service configured
- **THEN** evaluation fails, naming the constant: "Invalid FHIRPath Constant
  %nosuchvariable" (`FHIRPATH_UNKNOWN_CONSTANT`, `FHIRPathEngine.java:1782`)
- **AND** the static **check** rejects it first with the same message when
  there is no host service (`:3167-3168`) and when the host service answers
  `null` for the name (`:3175`); test the evaluation error with the check
  skipped

#### Scenario: [path.env.012] a host service may supply an environment variable, including a collection

- **WHEN** a host service that returns two strings for a named constant is
  configured and that constant is evaluated
- **THEN** the result is both strings in order, which is
  `FHIRPathTests.resolveConstantReturnsList`'s assertion

#### Scenario: [path.env.015] the host service is told which constant-evaluation mode is in force

- **WHEN** a host service is asked to resolve a constant, in either the
  evaluation pass or the check pass
- **THEN** it is told which of exactly four modes applies, and each mode is in
  force at exactly the sites below:
  - **explicit** — the expression itself wrote `%name` and the engine does not
    recognise the name internally. In force at the evaluation pass's constant
    node and at the check pass's constant node, and nowhere else
  - **no-value** — the engine was invoked with **no focus at all**. In force at
    exactly one site: the check pass's name node, when the node is at the entry
    of the expression and the focus type is absent. This is what checking an
    expression with no context type produces, which is how the 90 cases in the
    pinned suite that carry no `inputfile` are checked
  - **implicit-before** — the engine is about to navigate a named property and
    is offering the host the chance to supply a value first (`path.env.016`)
  - **implicit-after** — the engine navigated a named property, found nothing,
    and is offering the host the chance to supply a value (`path.env.017`)
- **AND** the name the host sees differs by stage. The **check** pass passes an
  explicit `%name` to `resolveConstantType` **with** its `%`
  (`FHIRPathEngine.java:3173`, `s` unstripped) and passes the no-value name
  (`:1586`) and the evaluation pass's implicit names (`:3184`, `:3214`)
  **without** one; the **evaluation** pass strips the `%` before calling
  `resolveConstant` (`:1783`, `s.substring(1)`). So a host keyed `owned` is asked
  for `%owned` by the check, and for `owned` by evaluation: such a host fails the
  **check** of `%owned` with "Invalid FHIRPath Constant %owned" and answers it at
  evaluation. A host must accept both spellings in the type call
- **AND** the no-value mode is **not** "resolved for its type with no value
  wanted": an explicit `%name` in the check pass is reported as **explicit**,
  not as no-value, so a host service that keys its type answers off the mode
  rather than off the pass must see the same split
- **AND** the no-value mode reaches the host only through the type-resolution
  entry point, and with no host service configured that site fails with the
  no-host-services error rather than yielding empty (`path.env.011`)
- **AND** the four modes map one-for-one onto the four the server-facing
  wrapper passes through to its own evaluation context, so a host service
  written against that wrapper sees the same value
- **AND** no live case grades the mode argument, so it is differential-only;
  `DEV-0010` records the implicit modes' divergence from the FHIRPath grammar

#### Scenario: [path.env.016] a host constant can shadow a real property

- **WHEN** an expression whose leading name is a real element of the focus
  resource — `Patient.name` evaluated as the expression `name`, say — is
  evaluated with a non-null application context **and** a host service whose
  implicit-before answer for `name` is a non-empty collection
- **THEN** the result is the host service's collection, and the element is
  **never navigated**: the host constant shadows the real property
- **AND** the host answer replaces only the **leading name**; the rest of the
  expression runs on it. With a host answering a string for `Patient`, the
  expression `Patient.name` yields an **empty** collection, because `.name` is
  navigated on that string (`execute`, `FHIRPathEngine.java:3181-3190`), and the
  expression `Patient` alone yields the host's string
- **AND** the host service is asked before any navigation and before the
  leading-type-name special case that matches a resource type, at the leading
  name of the expression **and** at the leading name of every argument the
  engine evaluates as an expression in its own right — `where()`'s criteria and
  `trace()`'s name among them — and once per focus item, because the per-item
  loop asks for each item separately
- **AND** an empty implicit-before answer leaves navigation to proceed normally,
  so a host service that answers empty for every unknown name is
  indistinguishable from no host service on this path
- **AND** this path is live for every caller this port ships that passes an
  application context — search-parameter extraction and the validator both do —
  so a host service must answer empty for any name it does not own, or a
  resource's own elements become unreachable
- **AND** no live case in the pinned suite grades this, so it is
  differential-only; `DEV-0010` records it

#### Scenario: [path.env.017] the implicit-after mode is a fall-through, and both implicit modes need an application context

- **WHEN** an expression whose leading name is not an element of the focus
  resource is evaluated with a non-null application context and a host service
- **THEN** navigation runs first and yields empty, and only then is the host
  service asked with the implicit-after mode, whose answer becomes the result
  (`FHIRPathEngine.java:3214`)
- **AND** this is observable only with a host that **differentiates the
  modes**: a host that answers non-empty for the implicit-before mode as well is
  asked first and its answer is returned at once (`:3184-3190`), so the
  implicit-after mode is never reached. A test of this scenario uses a host that
  answers empty for implicit-before and non-empty for implicit-after, and
  records the modes it was asked: `[ImplicitBefore, ImplicitAfter]`
- **AND** the static **check** rejects such an expression first: a leading name
  that is not an element of the focus type resolves to no type and raises
  `FHIRPATH_UNKNOWN_NAME` (`FHIRPathEngine.java:1587-1593`), and the check
  consults the host only when there is **no focus type at all**
  (`atEntry && focus == null`, `:1585-1586`, the no-value mode). So the
  implicit-after path is reachable only with the check skipped, as in
  `path.env.007` to `.014`
- **WHEN** the same expression is evaluated with the **same** host service but a
  **null** application context
- **THEN** neither implicit mode fires at all, the host service is not consulted
  for the leading name, and the result is empty
- **AND** that gate is on the application context, not on the host service, so
  installing a host service without an application context changes nothing about
  name resolution — only `%name`, `resolve()`, `conformsTo()`, `memberOf()`,
  `trace()` and custom functions still reach it
- **AND** no live case in the pinned suite grades this, so it is
  differential-only; `DEV-0010` records it

#### Scenario: [path.env.013] `defineVariable` introduces a variable for the rest of the expression

- **WHEN** an expression that calls `defineVariable('v', ...)` and then
  references the variable is evaluated
- **THEN** the variable resolves to the defined value

#### Scenario: [path.env.014] a variable defined by `defineVariable` shadows nothing and does not leak

- **WHEN** a variable defined inside one argument of an expression is
  referenced outside that argument's scope
- **THEN** evaluation fails, naming the constant ("Invalid FHIRPath Constant
  %v", `FHIRPATH_UNKNOWN_CONSTANT`)
- **AND** the static **check** rejects it first for the same reason as
  `path.env.011`: the check keeps its own defined-variable table, which does not
  hold a variable outside its scope (`FHIRPathEngine.java:3163-3168`); test the
  evaluation error with the check skipped

### Requirement: Utility and FHIR-specific functions

The engine SHALL implement the utility and FHIR-specific functions `FHIRPathEngine` implements, delegating `resolve`, `conformsTo` and `memberOf` to the host service.

#### Scenario: [path.utility.001] `trace` passes its input through and records a log line

- **WHEN** the `testTrace` group's cases are evaluated
- **THEN** every case yields its stated output
- **AND** the trace log is retrievable after evaluation and contains the names
  the expressions passed to `trace`
- **AND** the engine exposes a has-log predicate and a take-log accessor, and
  **taking the log clears it**: a second take with no intervening `trace`
  returns the empty string and the has-log predicate is false
- **AND** HAPI does both of these, separately: `takeLog` returns the log and
  replaces it with an empty one (`FHIRPathEngine.java:6692-6698`), and **every
  `evaluate` overload starts with an empty log** (`log = new StringBuilder()`,
  `:702`, `721`, `740`, `758`, `776`, `795`), discarding entries that were never
  taken. So in HAPI the next case's evaluation cannot see the previous case's
  lines whether or not the harness took them; the clear-on-take requirement is
  the take accessor's own behaviour, and the per-evaluation reset is the
  evaluate path's. A static `check` does not touch the log
- **AND** the oracle harness drains the log after each case, and the engine is
  reused across all 1,191 cases (`path.oracle.001`), so both must hold:
  evaluation starts with an empty log, and a take empties it
- **AND** a log entry is the trace name, then `: `, then the traced items'
  string values joined by `,`; entries are joined by `; ` in evaluation order
- **AND** `trace` is the only construct that writes to the log; no other
  function or operator appends to it

#### Scenario: [path.utility.002] `trace` with a projection logs the projection, not the input

- **WHEN** `'[stop]ONE[stop]'.split('[stop]').trace('n').count()` is evaluated
- **THEN** the count is as the suite states and the log is recorded under `n`
  (`testSplit/testSplit3`)
- **AND** a host service whose `log` hook answers `true` suppresses the entry,
  and the engine's own log stays empty (`FHIRPathEngine.java:927-928`)

#### Scenario: [path.utility.003] `check` passes its input through when the condition holds

- **WHEN** an expression using `check()` with a satisfied condition is evaluated
- **THEN** the input is returned unchanged
- **AND** `check` takes two arguments (`FHIRPathEngine.java:1411`), the condition
  and a message, and `funcCheck` evaluates the condition with the **focus** as
  its context (`:5649-5651`)
- **AND** the static check **rejects every boolean condition**. `case Check`
  (`FHIRPathEngine.java:3544-3547`) calls `checkParamTypes` with a singleton
  `string` as the type of the **first** parameter, the condition, and
  `checkParamTypes` raises `FHIRPATH_WRONG_PARAM_TYPE` for any other type
  (`:3820-3831`), rendered for `Patient.check(true, 'm')` as "Error evaluating
  FHIRPath expression: The parameter type http://hl7.org/fhirpath/System.Boolean
  is not legal for check parameter 1. expecting
  SINGLETON[http://hl7.org/fhirpath/System.String] (@char 9)", the suffix being
  the column of the `check` function node, which is the holder
  (`checkParamTypes(exp, ...)`, `:3545`; `path.errors.002`'s rule)
  (`Messages.properties:371`; the type is `TypeDetails.FP_Boolean`,
  `TypeDetails.java:66`, and the expected set prints as the collection status
  then the type list, `TypeDetails.toString`, `:474-476`). So `Patient.check(true, 'm')`
  and `Patient.check(active.exists(), 'm')` (`active` is a `Patient` element, so
  name resolution succeeds and the parameter type is what fails) both **fail the
  check**; the second parameter is not type-checked. A condition that is not boolean-typed
  (a `string`) passes it, but is then reduced to a boolean at evaluation
- **AND** `check` is also **not** in `isExpressionParameter` (`:3804-3817`, which
  lists `where`, `exists`, `all`, `select`, `repeat` and `aggregate` at index 0
  and `trace` and `defineVariable` at index 1), so the condition is typed
  against the **context item**, not the focus: a focus-relative name such as
  `given` in `Patient.name.check(given.exists(), 'm')` fails earlier, with "The
  name 'given' is not valid for any of the possible types ... Patient"
- **AND** there is therefore **no boolean-condition form of `check` that passes
  the static check**. The evaluation is graded with the check skipped (as
  `FhirPathR4` does, `path.check.013`): `Patient.check(active.exists(), 'm')`
  over `patient-example.xml` then evaluates the condition with the focus
  (`Patient`) as its context, and returns the `Patient` because the condition is
  true

#### Scenario: [path.utility.004] `check` fails when the condition does not hold

- **WHEN** an expression using `check()` with an unsatisfied condition is
  evaluated
- **THEN** evaluation fails with "Check failed with message '<message>'"
  (`FHIRPATH_CHECK_FAILED`, `FHIRPathEngine.java:5654`), the message being the
  second argument's value
- **AND** the static check rejects every boolean condition (`path.utility.003`),
  so this is graded with the check skipped

#### Scenario: [path.utility.005] `aggregate` folds with `$this` and `$total`

- **WHEN** `(1|2|3|4|5|6|7|8|9).aggregate($this+$total, 0) = 45` and
  `(1|2|3|4|5|6|7|8|9).aggregate($this+$total, 2) = 47` are evaluated
- **THEN** both yield `true` (`testAggregate/testAggregate1`, `testAggregate2`)

#### Scenario: [path.utility.006] `aggregate` without an initial value starts with an empty `$total`

- **WHEN** `(1|2|3|4|5|6|7|8|9).aggregate(iif($total.empty(), $this, iif($this < $total, $this, $total))) = 1`
  and the maximum form are evaluated
- **THEN** both yield `true` (`testAggregate/testAggregate3`, `testAggregate4`)

#### Scenario: [path.utility.007] `sort` orders a collection, with no argument and with a key

- **WHEN** the `testSort` group's cases are evaluated, including
  `(3 | 2 | 1).sort() = (1 | 2 | 3)` and
  `('a' | 'b' | 'c').sort(-$this) = ('c' | 'b' | 'a')`
- **THEN** every case yields its stated output

#### Scenario: [path.utility.008] `sort` accepts several keys and a descending key

- **WHEN** `Patient.name.sort(-family, -given.first()).first().use = 'usual'` is
  evaluated over `patient-example.xml`
- **THEN** the result is `true` (`testSort/testSort10`)

#### Scenario: [path.utility.009] `extension(url)` selects extensions by URL

- **WHEN** the `testExtension` group's cases are evaluated
- **THEN** every case yields its stated output
- **AND** the static check of `extension(url)` (`FHIRPathEngine.java:3556-3591`)
  never fails on an unknown URL: it types the argument as a singleton `string`,
  and for a constant URL it looks the extension definition up
  (`findExtensionDefinition`, `:3747-3765`); when found the result is the
  profiled `Extension` type, and when not found it adds the warning "Reference
  to an unknown extension - double check that the URL '<url>' is correct"
  (`FHIRPATH_UNKNOWN_EXTENSION`, `Messages.properties:368`) and answers a plain
  `Extension` type. Typing a
  known absolute URL therefore needs the extension's StructureDefinition from the
  model, so the check cannot be completed without it

#### Scenario: [path.utility.010] `extension(url)` with no match yields empty

- **WHEN** `Patient.extension('http://example.org/nothing')` is evaluated over
  `patient-example.xml`
- **THEN** the result is an empty collection

#### Scenario: [path.utility.011] `extension` reaches a primitive's extensions

- **WHEN** `Patient.birthDate.extension(url)` is evaluated for an extension
  carried in `_birthDate`
- **THEN** the extension is returned
- **AND** `funcExtension` reads the item's `extension` and `modifierExtension`
  children whatever the item's type, primitive included
  (`FHIRPathEngine.java:5464-5482`); the check of the call is as in
  `path.utility.009`

#### Scenario: [path.utility.012] `conformsTo` asks the host service

- **WHEN** `conformsTo('http://hl7.org/fhir/StructureDefinition/Patient')` and
  `conformsTo('http://hl7.org/fhir/StructureDefinition/Person')` are evaluated
  over `patient-example.xml` with the suite's host service
- **THEN** the results are `true` and `false`
  (`testConformsTo/testConformsTo1`, `testConformsTo2`)

#### Scenario: [path.utility.013] `conformsTo` with a profile the host service does not know is an execution error

- **WHEN** `conformsTo('http://trash')` is evaluated
- **THEN** evaluation fails (`testConformsTo/testConformsTo3`)

#### Scenario: [path.utility.014] `conformsTo` through the server-facing host service returns false for every URL

- **WHEN** `conformsTo(...)` is evaluated through the server-facing host
  service, the one `FhirPathR4.setEvaluationContext` installs, whose
  `conformsToProfile` returns `false` unconditionally
- **THEN** the result is `false` for every URL
- **AND** this is a configured host service, not the absence of one — see
  `path.utility.022` for the unset case

#### Scenario: [path.utility.015] `memberOf` asks the host service for the value set

- **WHEN** `memberOf(...)` is evaluated with a host service that resolves the
  value set
- **THEN** the result is the membership answer the host service implies

#### Scenario: [path.utility.016] `memberOf` with no resolvable value set yields empty

- **WHEN** `memberOf(...)` is evaluated with a host service that returns no
  value set
- **THEN** the result is an empty collection, not an error

#### Scenario: [path.utility.017] `resolve()` resolves a local `#id` reference against the root resource's `contained`

- **WHEN** `resolve()` is evaluated on a `Reference` whose value is `#id`
- **THEN** the id is looked up in the **root** resource's `contained` list, not
  in the focus resource's, and the matching contained resource is returned
- **AND** when the root resource has no `contained` child, or no contained
  resource has that id, the item contributes nothing to the result
- **AND** the function also accepts a primitive focus, treating its value as the
  reference, and reads `Reference.identifier` when present
- **AND** no live case in the pinned suite calls `resolve()` at all, so every
  `resolve()` scenario is differential-only

#### Scenario: [path.utility.018] `resolve()` has no Bundle `fullUrl` resolution of its own

- **WHEN** `resolve()` is evaluated on a `Reference` inside a `Bundle` whose
  entry `fullUrl` matches the reference, with no host service configured
- **THEN** the result is an empty collection, because the function resolves only
  a `#`-prefixed reference itself and delegates every other form to the host
  service
- **AND** with a host service configured, that service is asked for the URL and
  may answer from the Bundle; Bundle-internal resolution is therefore the host
  service's responsibility, not the engine's

#### Scenario: [path.utility.019] `resolve()` on an unresolvable reference yields empty

- **WHEN** `resolve()` is evaluated on an absolute reference with no host
  service configured
- **THEN** the result is an empty collection

#### Scenario: [path.utility.020] `resolve()` delegates an external reference to the host service and swallows its errors

- **WHEN** `resolve()` is evaluated on `Patient/123` with a host service
  configured
- **THEN** the host service is asked for `Patient/123` and its answer is the
  result
- **WHEN** the host service raises instead of answering
- **THEN** the item contributes nothing and evaluation continues; the error is
  not surfaced
- **AND** this is the one place the engine deliberately converts an error into
  an absent value, so it is specified rather than left to
  `path.errors.005`

#### Scenario: [path.utility.022] `conformsTo` with no host service at all is an execution error

- **WHEN** `conformsTo(...)` is evaluated on an engine with no host service set,
  which is the state most oracle cases run in
- **THEN** evaluation fails with an error naming `conformsTo`, because the
  function raises a no-host-services error before looking at its argument
- **AND** a focus of other than one item returns `false` rather than failing,
  once a host service is set
- **AND** no live case grades the unset path, so it is differential-only

#### Scenario: [path.utility.023] `extension(url)` also returns modifier extensions

- **WHEN** `extension('http://example.org/x')` is evaluated on an element that
  carries that URL in `modifierExtension` rather than in `extension`
- **THEN** the modifier extension is returned, because the function collects
  both `extension` and `modifierExtension` children before filtering by URL
- **AND** no live case grades it — the `testExtension` group has no
  modifier-extension case — so it is differential-only
- **AND** this differs from the FHIRPath/FHIR definition of `extension()`,
  which is `extension.where(url=...)`; the divergence is recorded as `DEV-0007`
  and resolved as HAPI's behaviour

#### Scenario: [path.utility.024] `memberOf` as a binary operator folds the whole left collection

- **WHEN** `Patient.name.use memberOf 'http://hl7.org/fhir/ValueSet/name-use'`
  is evaluated with a host service that resolves the value set
- **THEN** the result is a single boolean that is `true` if **any** item of the
  left collection validates, because the operator iterates the whole collection
  and ORs the outcomes
- **AND** this differs from the `memberOf()` function, which returns empty
  unless both the focus and the argument are singletons
- **AND** no live case grades either form, so both are differential-only

#### Scenario: [path.utility.025] `memberOf` falls back to the worker context when no host service is set

- **WHEN** either `memberOf` form is evaluated with no host service
- **THEN** the value set is looked up in the engine's own worker context rather
  than failing
- **AND** when the value set cannot be resolved, the function form returns an
  empty collection while the operator form returns `false`

#### Scenario: [path.utility.026] `memberOf` returns empty for a focus whose type it cannot validate

- **WHEN** `memberOf(...)` is evaluated on a focus whose FHIR type is not one of
  `code`, `string`, `uri`, `Coding` or `CodeableConcept` — a `boolean`, say
- **THEN** the result is an empty collection, not `false`
- **AND** `code`, `string` and `uri` are validated with system guessing enabled,
  while `Coding` and `CodeableConcept` are not

#### Scenario: [path.utility.027] a host service may supply a custom function

- **WHEN** an engine whose host service declares a function by name, with a
  minimum and maximum parameter count, parses and evaluates an expression
  calling it
- **THEN** the parser takes the arity from the host service's declaration, the
  check pass asks the host service for the result type, and evaluation calls the
  host service with the focus and the evaluated parameters
- **AND** an argument the host service declares to be a type name is passed
  unevaluated, as its `paramIsType` answer directs
- **AND** no live case grades it, so it is differential-only

#### Scenario: [path.utility.028] a host service may intercept `trace` and suppress the engine's log

- **WHEN** an expression calling `trace('n')` is evaluated with a host service
  whose log hook returns **true**
- **THEN** the hook is called with the trace name and the traced items, the
  expression's own result is still the traced input passed through, and the
  engine's trace log stays **empty**: the has-log predicate is false and the
  take-log accessor returns the empty string
- **WHEN** the same expression is evaluated with a host service whose log hook
  returns **false**
- **THEN** the hook is called and the engine **also** appends the entry to its
  own log, so the entry is observable both ways
- **AND** with no host service configured the engine appends the entry with no
  hook call, which is the configuration `path.utility.001` and the oracle
  harness run in
- **AND** the hook is reached only from `trace`; the math `log()` function has
  no connection to it
- **AND** the host-service surface has exactly ten methods and this is the last
  of them to get a scenario, so with it the surface is fully specified:
  constant and constant-type resolution by `path.env.012`, `path.env.015`,
  `path.env.016` and `path.env.017`; function resolution, function type
  checking, function execution and the type-parameter predicate by
  `path.utility.027`; reference resolution by `path.utility.020`; profile
  conformance by `path.utility.012`; value-set resolution by `path.utility.015`
- **AND** no live case grades it, so it is differential-only

#### Scenario: [path.utility.021] the appointment and explanation-of-benefit cases hold

- **WHEN** the cases whose `inputfile` is `appointment-examplereq.json`,
  `explanationofbenefit-example.json`, `codesystem-example.xml` or
  `valueset-example-expansion.xml` are evaluated
- **THEN** every case yields its stated output

### Requirement: The static type-check pass

The engine SHALL provide a check pass, separate from evaluation, that reports
the semantic errors `FHIRPathEngine.check` reports — 29 suite cases are
`invalid="semantic"`, and the `isSemanticChecks` group exists only to exercise
it. Like the pinned engine, it SHALL NOT narrow a type on a type guard
(`path.check.005` to `path.check.012`).

#### Scenario: [path.check.001] the check pass is separate from evaluation

- **WHEN** an expression is parsed, checked against a focus type, and then
  evaluated
- **THEN** the check reports its result without evaluating the expression
- **AND** an expression that fails the check is not evaluated

#### Scenario: [path.check.002] 27 of the 29 semantic-invalid oracle cases fail the check; the other 2 fail at parse, as for HAPI

- **WHEN** each of the suite's 29 `invalid="semantic"` cases is run
- **THEN** 27 of them parse cleanly and fail the **check**, and each is graded
  `passed`
- **AND** the check reports which element path or type caused the failure
- **AND** the other two never reach the check. They are
  `documentFunctions/testResourcesAbstract3` (`tests-fhir-r4.xml:1940`) and
  `documentFunctions/testHasCoding11` (`:1951`), and they call `resources()`
  and `hasCoding()`, which the R4 `ExpressionNode.Function` enum does not have.
  Both fail at the **parse** stage. The parser asks the host service only after
  `Function.fromCode` returns null (`FHIRPathEngine.java:1158-1166`), and the
  suite's host service raises from `resolveFunction`
  (`FHIRPathTests.java:72-76`)
- **AND** a parse-stage failure of a case expecting a `semantic` failure comes
  from the wrong stage, so both are graded `failed` under `path.oracle.005`'s
  stage rule; `path.oracle.014` lists them among the group's 111 `failed`
  cases. HAPI fails both
  the same way: its harness asserts a `SYNTAX` expectation when parsing fails
  (`FHIRPathTests.java:199-202`). This is parity
- **AND** each case is checked with the `context` and `contextType` the case
  carries, so this is graded against Frevo's context-aware run and not against
  HAPI's, which reads neither attribute (`path.oracle.016`)

#### Scenario: [path.check.003] navigating a non-existent element is a semantic error

- **WHEN** `Patient.nosuchelement` is checked against focus type `Patient`
- **THEN** the check fails, naming `nosuchelement`

#### Scenario: [path.check.004] the check infers the type of a choice element as the declared set

- **WHEN** `Observation.value` is checked against focus type `Observation`
- **THEN** the inferred type is the declared choice set, not a single type

#### Scenario: [path.check.005] an `is` guard in `implies` does not narrow the type the other operand is checked against

- **WHEN** `$this is dateTime implies $this.toString().length() > 10` is checked
  with `Observation.effective` as the context and `Period` as the context type
- **THEN** the check **fails** with the primitive-only diagnostic for
  `toString`. Its message id is `FHIRPATH_PRIMITIVE_ONLY` and its message is
  exactly `Error evaluating FHIRPath expression: The function toString can only
  be used on primitives (@char 37)`: the template at `Messages.properties:353`
  with `{0}` bound to `toString`, followed by the position of the `toString`
  function node under `path.errors.002`'s column rule (byte offset 32; the four
  spaces count 2 each). `makeException` (`ENGINE:584-594`) adds no
  ` (at ...)` fragment, because neither `FHIRPathTests` nor the oracle harness
  sets the engine's `location`. HAPI's record carries the same suffix for the
  `>= 10` form of this expression (`isEffectOnLogic1`,
  `hapi-8.12.0-results.json:1046`). This is the same diagnostic the unguarded
  expression of `path.check.006` produces, with its own position
  (` (@char 7)` for `$this.toString().length() > 10`). Wherever
  `path.check.006` to `.012` name "the `FHIRPATH_PRIMITIVE_ONLY` diagnostic for
  `toString`", they mean this message text followed by the ` (@char N)` of the
  `toString` node in that expression
- **AND** this is HAPI 8.12.0's behaviour: `executeType` types every operand of
  an operator chain against the **same** focus (`ENGINE:1621-1637`); the `is`
  operator types to `Boolean` and leaves the focus unchanged
  (`ENGINE:2013-2015`), as does `implies` (`ENGINE:2027`); and `toString`
  requires the focus to include a primitive type or `Quantity`
  (`ENGINE:3638-3640`, `:3872-3883`). The pinned R4 engine narrows on no
  guard anywhere. The group's note (`tests-fhir-r4.xml:1695-1702`) asks for
  narrowing; Frevo matches HAPI instead (owner ruling of 2026-11-12, recorded
  in `tasks/blockers.md`, "task 50", finding 3). Here and in `path.check.006`
  to `.012`, `ENGINE` is
  `reference/org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java`,
  `TypeDetails.java` its sibling in that package, and `Messages.properties`
  `org.hl7.fhir.utilities/src/main/resources/Messages.properties`, all at
  `73c432b5f43ddbce979978a760ec4aa81e929094`
- **AND** so the three `implies`-guard cases the suite expects to pass the
  check — `isSemanticChecks/isEffectOnLogic3`, `isGuardOrTypesImplies` and
  `isGuardNotImplies` (`tests-fhir-r4.xml:1690-1693`, `:1717-1720`,
  `:1781-1784`) — end at the **check** stage and are graded `failed`. HAPI's own
  run fails all three too, because its harness checks against the `Observation`
  root, which is not a primitive either. Under `path.oracle.012` this is parity,
  not a gap and not `ahead-of-hapi`. HAPI's recorded status is `failed` for
  `isEffectOnLogic3` and `isGuardOrTypesImplies`, and `hapi-cannot-pass` for
  `isGuardNotImplies`, which shares its expression with `isGuardNotImpliesBad`
  (`path.oracle.016`)
- **AND** the same guard passes the check when the focus does include a
  primitive: with `dateTime` as the context type (`isEffectOnLogic2`) and with no
  context type, where the focus is the whole `effective[x]` choice set
  (`isEffectOnLogic1`, `isGuardOrTypesImpliesAll`); those cases' graded status
  is then decided by evaluation. HAPI's recorded status for
  `isGuardOrTypesImpliesAll` is `hapi-cannot-pass` (`path.oracle.016`)
- **AND** every `implies`-guard case the suite expects to fail the check fails
  it and is graded `passed`: `isGuardImpliesBad`, `isGuardOrTypesImpliesBad`,
  `isGuardOrFnTypesImpliesBad`, `isGuardNotImpliesBad`, `isGuardSupertypeBad`
  and `isGuardOtherExpressionBad`. HAPI's recorded status for
  `isGuardOrTypesImpliesBad` and `isGuardNotImpliesBad` is `hapi-cannot-pass`
  (`path.oracle.016`)

#### Scenario: [path.check.006] the same expression without the guard fails the check

- **WHEN** `$this.toString().length() > 10` is checked with
  `Observation.effective` as the context and `Period` as the context type
- **THEN** the check fails as a semantic error, with the same
  `FHIRPATH_PRIMITIVE_ONLY` diagnostic for `toString` as `path.check.005`
  (`isSemanticChecks/isEffectOnLogic3Base`, graded `passed`)
- **AND** with `dateTime` as the context type (`isEffectOnLogic2Base`) or with no
  context type (`isEffectOnLogic1Base`) the same expression passes the check
- **AND** HAPI's recorded status for `isEffectOnLogic2Base` and
  `isEffectOnLogic3Base` is `hapi-cannot-pass` (`path.oracle.016`)
- **AND** for each of those three context types the guarded expression of
  `path.check.005` and this unguarded one get the **same** check outcome: the
  guard changes nothing the check pass observes. Where both fail, the message
  id and the message text are the same and only the position suffix differs,
  because the `toString` node sits at a different column: ` (@char 37)` for the
  guarded form and ` (@char 7)` for this one (`path.errors.002`)

#### Scenario: [path.check.007] the `is()` function narrows nothing either; only `ofType()` and `as` retype the focus

- **WHEN** `$this.is(dateTime) implies $this.toString().length() >= 10` is
  checked with `Observation.effective` as the context and `Period` as the
  context type
- **THEN** the check fails with the `FHIRPATH_PRIMITIVE_ONLY` diagnostic for
  `toString`, because the `is()` function types to `Boolean` and leaves the
  focus unchanged (`ENGINE:3401-3404`), exactly as the operator does
- **AND** so `isSemanticChecks/isGuardFnImplies` ends at the check stage and is
  graded `failed`, as it is for HAPI; `isGuardFnImpliesBad`
  (`$this.is(Period) implies ...` with `Period|Timing`) fails the check and is
  graded `passed`
- **WHEN** `$this.ofType(dateTime).toString().length() >= 10` or
  `($this as dateTime).toString().length() >= 10` is checked with the same
  context and context type
- **THEN** the check passes, because `ofType()` types its result as the named
  type whatever the focus (`ENGINE:3377-3384`) and `as` types its result as the
  named type on its right (`ENGINE:2016-2021`, with the right operand typed by
  name at `ENGINE:1627-1628`, `:1570-1572`) — `isGuardOfType` and `isGuardAs`
- **AND** the `ofType(dateTime)` form also yields an of-type-impossible warning,
  because a `Period` focus can never be a `dateTime` (`ENGINE:3782-3784`); a
  warning does not fail the check (`path.check.015`)

#### Scenario: [path.check.008] `and` checks its right operand against the unnarrowed focus

- **WHEN** `$this is dateTime and $this.toString().length() >= 10` is checked
  with `Observation.effective` as the context and `Period` as the context type
- **THEN** the check fails with the `FHIRPATH_PRIMITIVE_ONLY` diagnostic for
  `toString`: `and` types to `Boolean` (`ENGINE:2025`) and its right operand is
  typed against the same focus as its left (`ENGINE:1621-1637`)
- **AND** so the three `and`-guard cases the suite expects to pass the check —
  `isSemanticChecks/isGuardAnd`, `isGuardAndOtherImplies` and
  `isGuardOtherAndImplies` — end at the check stage and are graded `failed`, as
  they are for HAPI; their `%resource.status = 'final'` operand types cleanly
  and does not change which diagnostic is raised
- **AND** `isGuardAndBad` (`$this is Period and ...` with `Period|Timing`) fails
  the check and is graded `passed`

#### Scenario: [path.check.009] `or` and `xor` check their right operand against the unnarrowed focus

- **WHEN** `$this is Period or $this.toString().length() >= 10` is checked with
  `Observation.effective` as the context and `Period` as the context type
- **THEN** the check fails with the `FHIRPATH_PRIMITIVE_ONLY` diagnostic for
  `toString`: `or` types to `Boolean` (`ENGINE:2024`) and does not assume its
  left operand false when typing its right
- **AND** so the three `or`-guard cases the suite expects to pass the check —
  `isSemanticChecks/isGuardOr`, `isGuardOrBoth` and `isGuardOrInnerImplies` —
  end at the check stage, and Frevo grades all three `failed`
- **AND** HAPI's recorded status for `isGuardOrBoth` is `failed`. For
  `isGuardOr` and `isGuardOrInnerImplies` it is `hapi-cannot-pass`, because
  each belongs to a `path.oracle.016` group of cases whose expressions are
  byte-identical and differ only by `contextType`. `isGuardOr` is in a
  three-case group with `isGuardOr2` and `isGuardOrBad`, and
  `isGuardOrInnerImplies` is in a pair with `isGuardOrInnerImpliesBad`
- **AND** the `or` and `xor` cases the suite expects to fail the check fail it,
  and Frevo grades them `passed`: `isGuardOrBad`, `isGuardOrOtherImpliesBad`,
  `isGuardOrOtherImpliesNoParensBad`, `isGuardOrInnerImpliesBad` and
  `isGuardXorBad`. HAPI's recorded status for `isGuardOrBad`,
  `isGuardOrInnerImpliesBad` and `isGuardXorBad` is likewise
  `hapi-cannot-pass` (`path.oracle.016`)
- **AND** with `Period|dateTime` as the context type the same `or` expression
  (`isGuardOr2`) and its `xor` counterpart (`isGuardXorBad2`) pass the check,
  because the focus then includes a primitive (`path.check.012`); their graded
  status is then decided by evaluation. HAPI's recorded status for both
  `isGuardOr2` and `isGuardXorBad2` is `hapi-cannot-pass`. `isGuardOr2` shares
  its expression with `isGuardOr` and `isGuardOrBad` (`path.oracle.016`)

#### Scenario: [path.check.010] `iif` checks both branches against the unnarrowed focus

- **WHEN** `iif($this is dateTime, $this.toString().length() >= 10, true)` or
  `iif($this is Period, true, $this.toString().length() >= 10)` is checked with
  `Observation.effective` as the context and `Period` as the context type
- **THEN** the check fails with the `FHIRPATH_PRIMITIVE_ONLY` diagnostic for
  `toString`, whichever branch holds it: `iif`'s arguments are not expression
  parameters (`ENGINE:3804-3816`), so each is typed against the same `$this`
  (`ENGINE:3261-3271`), and `iif`'s result is the union of its branch types
  with no reference to the condition (`ENGINE:3441-3449`)
- **AND** so `isSemanticChecks/isGuardIifTrue` and `isGuardIifElse` end at the
  check stage and are graded `failed`, as they are for HAPI; `isGuardIifBad`
  fails the check and is graded `passed`

#### Scenario: [path.check.011] a guard that can never be true leaves the other operand checked as usual

- **WHEN** `$this is dateTime implies $this.toString().length() >= 10` is
  checked with `Observation.effective` as the context and `Period|Timing` as
  the context type, so the guard can never be true
- **THEN** the check fails with the `FHIRPATH_PRIMITIVE_ONLY` diagnostic for
  `toString`: the pinned engine has no notion of an unreachable operand, and the
  suite group's note asking for one (`tests-fhir-r4.xml:1695-1702`) describes
  behaviour HAPI 8.12.0 does not have
- **AND** so `isSemanticChecks/isGuardImpliesEmpty` and `isGuardNested`
  (`$this is Period implies ($this is dateTime implies ...)` with
  `Period|Timing`) end at the check stage and are graded `failed`, as they are
  for HAPI
- **AND** the only exemption the pinned engine has is a focus with **no types
  at all**, which the primitive-only rule does not check
  (`ENGINE:3873`, `!focus.hasNoTypes()`); a guard never produces one

#### Scenario: [path.check.012] a context type naming several types checks against all of them

- **WHEN** a case whose `contextType` is `Period|Timing` or `Period|dateTime` is
  checked
- **THEN** the focus is the set of all the named types, as `checkOnTypes` builds
  it from a type list (`ENGINE:477-485`)
- **AND** a focus rule is satisfied when **any** type in the set satisfies it
  (`TypeDetails.java:436-448`): `$this.toString()` passes the check with
  `Period|dateTime` (`isGuardOr2`, `isGuardXorBad2`) and fails it with
  `Period|Timing`, where neither type is a primitive (`isGuardOrBad`,
  `isGuardImpliesBad`, `isGuardFnImpliesBad`, `isGuardAndBad`,
  `isGuardNotImpliesBad`, `isGuardOrInnerImpliesBad`, all graded `passed`, and
  `isGuardImpliesEmpty`, `isGuardOrBoth`, `isGuardNested`, graded `failed` as
  for HAPI)

#### Scenario: [path.check.013] the check pass can be skipped

- **WHEN** an expression is evaluated with the check pass disabled
- **THEN** evaluation proceeds, as `FhirPathR4` does, which never calls `check`

#### Scenario: [path.check.015] the check pass returns a warning list separate from its failure

- **WHEN** an expression is checked through the surface that accepts a warning
  list
- **THEN** the check returns the inferred type **and** a list of warnings, each
  carrying a message and a stable message id
- **AND** a warning does not make the check fail, so an expression that produces
  warnings is still evaluated
- **AND** the warning channel is graded by an **in-process unit oracle** over
  the generated R4 model, not by the vendored suite and not by the differential
  harness: the suite's `invalid="semantic"` selector grades only whether the
  check fails, and a live HAPI server over HTTP never exposes the list. The
  oracle's case selector is the warning's message id, each of which is a stable
  constant name in the upstream engine. `path.check.015` through
  `path.check.019` are the only `path.check.*` scenarios graded this way; the
  rest are graded by the suite selector named in
  [`../../spec.md`](../../spec.md)
- **AND** the nine message ids the check pass can emit are the complete set:
  choice-without-type-specifier, choice-with-spurious-type-specifier,
  collection-status-on-an-operation's-left-operand, the same for the right
  operand, collection-status-on-a-function-parameter,
  collection-status-on-a-context, not-a-collection, unknown-extension and
  of-type-impossible. A tenth id, or a missing one, fails the oracle

#### Scenario: [path.check.016] comparing a collection with a singleton produces a warning, not a failure

- **WHEN** `Patient.name.given < 'Z'` is checked, whose left operand is a
  collection where a singleton is expected
- **THEN** the check succeeds and the warning list carries a
  collection-status warning identifying the operand side
- **AND** the same applies to a function parameter whose declared collection
  status is singleton and to a focus whose status is not singleton, each with
  its own message id
- **AND** turning any of these into a failure would reject expressions the
  engine accepts, so the distinction is specified rather than inferred

#### Scenario: [path.check.017] navigating a choice element without `ofType()` produces a warning only when the SQL-on-FHIR check is enabled

- **WHEN** `Observation.value` is checked with the SQL-on-FHIR warning switched
  off, which is the default
- **THEN** the warning list is empty
- **WHEN** the same expression is checked with it switched on
- **THEN** the warning list carries a choice-without-type-specifier warning
- **AND** `Observation.value.ofType(Quantity)` checked with it switched on
  produces no warning, while applying `ofType()` to a non-choice element
  produces a spurious-type-specifier warning
- **AND** this is the path search-parameter and ViewDefinition expressions take,
  so the setting is part of the public surface

#### Scenario: [path.check.018] an unknown extension URL and an impossible `ofType` produce warnings

- **WHEN** `extension('http://example.org/unknown')` is checked against a model
  that does not define that extension
- **THEN** the check succeeds with an unknown-extension warning
- **WHEN** `Patient.name.ofType(Quantity)` is checked, whose focus can never be
  a `Quantity`
- **THEN** the check succeeds with an of-type-impossible warning
- **AND** a navigation into a non-collection where a collection is expected
  produces its own warning, with its own message id

#### Scenario: [path.check.019] the simple check surface discards warnings

- **WHEN** an expression is checked through the surface that takes no warning
  list — the one the oracle harness calls
- **THEN** the check still succeeds or fails as `path.check.002` requires, and
  the warnings are not returned to the caller
- **AND** the warning list accumulated during a check is cleared before the next
  check, so warnings never leak between expressions on a reused engine

#### Scenario: [path.check.014] the harness records which cases the check pass is exercised for

- **WHEN** the oracle harness runs
- **THEN** its report distinguishes a case that failed the check from one that
  failed at evaluation, so a `syntax`, `semantic` and `execution` expectation
  cannot be satisfied by the wrong kind of failure

### Requirement: Strictness configuration

The engine SHALL expose the strictness switches `FHIRPathEngine` exposes, and SHALL default to the values `FhirPathR4` sets, because that is the configuration the server runs.

HAPI configures the same engine two ways, and both are observable. Neither is a
deviation from HAPI, so neither needs a `DEV-nnnn` entry: both are HAPI.

#### Scenario: [path.strictness.001] the default configuration is non-strict for every pre-R5 version

- **WHEN** an engine is created for R4 with no explicit configuration
- **THEN** the as-singleton rule is not enforced and `as` is not case-sensitive
- **AND** this comes from the engine's own initialisation, which sets both for
  any worker context below R5; `FhirPathR4`'s constructor re-asserts the same
  two values rather than changing them, so the server-facing default and the
  engine default coincide

#### Scenario: [path.strictness.002] the oracle harness runs the check pass non-strict and the evaluation strict

- **WHEN** the oracle harness runs a case
- **THEN** the check pass runs with the as-singleton rule not enforced and `as`
  not case-sensitive, and the evaluation runs with both enforced
- **AND** that ordering is what `FHIRPathTests.test` produces: it calls
  `check(...)` first and sets both flags to `false` only afterwards, before
  `evaluate(...)`
- **AND** the harness sets the flags per case rather than relying on a shared
  engine, so the first case behaves like every other one — in the pinned Java
  harness the engine is `static`, so only the very first case's check pass sees
  the initial `true` values and every later one sees `false`
- **AND** the harness reports which flag values were in force for each stage of
  each case, so a disagreement with HAPI can be attributed

#### Scenario: [path.strictness.003] the as-singleton rule rejects a multi-item focus when enforced

- **WHEN** `as` is applied to a collection of more than one item with the rule
  enforced
- **THEN** evaluation fails
- **AND** `Patient.name.as(HumanName)` over `patient-example.xml`, which has
  three `name`s, fails with the message exactly
  `Attempt to use 'as()' on more than one item ('3', 'as(HumanName)')`, and
  `Patient.name as HumanName` with exactly
  `Attempt to use 'as()' on more than one item ('3', 'Patient.name as HumanName')`:
  `FHIRPATH_AS_COLLECTION` (`Messages.properties:301`) has no
  `Error evaluating FHIRPath expression:` prefix, `{0}` is the item count and
  `{1}` the holder's rendering through `ExpressionNode.toString`
  (`ExpressionNode.java:396-464`). The function form's holder is the `as`
  function node (`funcAs`, `FHIRPathEngine.java:5166-5167`), rendered as its
  name and parameters; the operator form's holder is `last`, the `Patient`
  node that carries the operation (`:1510-1519`, `opAs`, `:1884-1885`),
  rendered with its inner `name` and its operation and operand
- **AND** neither message has a position suffix and Frevo's error carries no
  offset, because both are constructed without a location (`path.errors.002`)
- **AND** the type name is resolved first and must be known: `funcAs` tests
  `FHIR.HumanName` and `opAs` tests `HumanName`, each through
  `isKnownType` and `fetchTypeDefinition` (`:1880-1882`, `:1904-1935`,
  `:5163-5164`); an unknown name fails with `FHIRPATH_INVALID_TYPE` instead

#### Scenario: [path.strictness.004] the as-singleton rule allows a multi-item focus when not enforced

- **WHEN** the same expression is evaluated with the rule not enforced
- **THEN** evaluation succeeds and each item is tested

#### Scenario: [path.strictness.005] a mis-cased type name resolves only when case is not enforced

- **WHEN** `Patient.gender.as(Code)` is evaluated over `patient-example.xml`
  with `as` case sensitivity **not** enforced, which is what both R4
  configurations set (`initFlags`, `FHIRPathEngine.java:211-216`;
  `FhirPathR4`'s constructor,
  `hapi-fhir-structures-r4/src/main/java/org/hl7/fhir/r4/hapi/fluentpath/FhirPathR4.java:34-40`)
- **THEN** the result is the `code` `male`. `funcAs` first tests
  `isKnownType("FHIR.Code")` (`FHIRPathEngine.java:5163-5165`, `:1904-1935`),
  which asks the worker for `.../StructureDefinition/Code`. HAPI's
  server-facing worker (`HapiWorkerContext.fetchTypeDefinition`,
  `hapi-fhir-structures-r4/src/main/java/org/hl7/fhir/r4/hapi/ctx/HapiWorkerContext.java:412-414`,
  over `DefaultProfileValidationSupport`) misses, and because the name's
  **first letter is upper-case** retries it with that letter lower-cased and
  finds `code` (`hapi-fhir-base/src/main/java/ca/uhn/fhir/context/support/DefaultProfileValidationSupportBundleStrategy.java:276-297`,
  for R4, R4B and R5). The item then matches because `compareTypeNames`
  ignores case when case is not enforced (`:1896-1902`, `:5185`)
- **WHEN** the same expression is evaluated with case sensitivity enforced,
  which only the setter reaches
- **THEN** the type is still known, through the same retry, and the result is an
  empty collection, because `Code` and `code` now compare unequal
- **AND** `Patient.gender as Code` behaves as `as(Code)` in both states
  (`opAs`: `isKnownType("Code")`, then `compareTypeNames`, `:1880-1890`), and
  `Patient.gender.ofType(Code)` is empty in both, because `funcOfType` compares
  with `equals`, not `compareTypeNames` (`:5205-5226`)
- **AND** the retry applies only to an upper-case initial: in either state,
  `Observation.value.as(quantity)` over `observation-example.xml` fails with the
  message exactly `The type FHIR.quantity is not valid`
  (`FHIRPATH_INVALID_TYPE`, `Messages.properties:336`), with no prefix, no
  position suffix and no Frevo offset, because it is constructed without a
  location (`path.errors.002`). HAPI's own `FhirPathEngineR4Test.testAs`
  (`hapi-fhir-validation/src/test/java/org/hl7/fhir/r4/utils/FhirPathEngineR4Test.java:93-115`)
  exercises only `string`, `FHIR.string`, `String` and `FHIR.String`
- **AND** the retry belongs to HAPI's **server-facing** lookup, not to the
  engine. The worker the vendored suite's Java harness uses resolves a type
  name exactly (`BaseWorkerContext.fetchTypeDefinition`,
  `org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/context/BaseWorkerContext.java:1183-1188`),
  so there `Patient.gender.as(Code)` fails with `The type FHIR.Code is not
  valid`. Frevo applies the retry in every configuration, the oracle harness
  included. No suite case names a mis-cased type in `as`, `as()` or
  `ofType()`, so the suite's results are unaffected. Recorded as `DEV-0026`
- **AND** no live case grades any of this, so it is differential-only

#### Scenario: [path.strictness.006] lenient polymorphic names are supported by the engine and left unset by HAPI

- **WHEN** `Observation.valueQuantity.exists()` is checked and evaluated over
  `observation-example.xml` with polymorphic names **not** allowed, which is the
  default and what both HAPI wrappers leave it at
- **THEN** the static check fails as a semantic error
  (`polymorphics/testPolymorphicsB`)
- **WHEN** the same expression is checked and evaluated with polymorphic names
  allowed
- **THEN** the check passes and the result is `true`, because the flag is read
  both when resolving children during evaluation and when resolving an element
  definition during the check
- **AND** the suite's comment that lenient polymorphic mode "is not supported in
  R4" describes two `modeTest` elements that are themselves commented out and
  are not among the 1,191; it does not describe the engine, which has the field,
  a public getter and setter, and honours it
- **AND** with polymorphic names allowed, the choice element's **own** base
  name navigates to nothing: `Observation.value` over `observation-example.xml`
  evaluates to an empty collection, and `Observation.value.exists()` to
  `false`. `getChildrenByName` (`FHIRPathEngine.java:255-279`) tests
  `name.startsWith(n)` for each `[x]` child, so `value` matches `value[x]` with
  an empty type suffix (`:262-264`), and an item is kept only when its
  `fhirType()` equals that suffix ignoring case (`:273`), which no type does.
  With polymorphic names not allowed, `Observation.value` yields the
  `Quantity` (`path.navigation.006`)
- **AND** no live case grades the allowed state, so it is differential-only

#### Scenario: [path.strictness.008] the SQL-on-FHIR warning is off by default

- **WHEN** an engine is created by either HAPI configuration
- **THEN** the SQL-on-FHIR warning is off, so checking a choice element without
  `ofType()` produces no warning (`path.check.017`)

#### Scenario: [path.strictness.009] implicit string conversion is off by default

- **WHEN** an engine is created by either HAPI configuration
- **THEN** implicit string conversion is off, so a string function over a
  non-string focus yields empty (`path.string.020`, `path.string.024`)

#### Scenario: [path.strictness.010] liquid mode is out of scope and must be off

- **WHEN** an engine is created by any configuration this change ships
- **THEN** liquid mode is off, so a `||` token terminates nothing and the parser
  does not hand control back to a host template engine
- **AND** liquid mode exists only to embed FHIRPath in Liquid templates, which is
  `add-narrative`'s concern; setting it is out of scope here and that change
  owns the scenario for the enabled state

#### Scenario: [path.strictness.007] legacy mode is not enabled for R4

- **WHEN** an R4 engine is created by either configuration
- **THEN** legacy mode is off, so the inequality operator is not valid for empty
  sets in the way DSTU2 and DSTU3 constraints assume

### Requirement: Execution errors

The engine SHALL report an evaluation failure as an error carrying HAPI's message, with the position of the failing node exactly where HAPI's message carries one, and SHALL never panic or substitute an empty collection for a failure.

#### Scenario: [path.errors.001] 21 of the 27 execution-invalid oracle cases fail at evaluation; the other 6 fail at parse, as for HAPI

- **WHEN** each of the suite's 27 `invalid="execution"` cases is run
- **THEN** for 21 of them parsing and checking succeed and evaluation fails, and
  each is graded `passed`
- **AND** the other six are in `documentFunctions`: `testSection11`,
  `testSection12`, `testEntryIn5`, `testEntryIn6`, `testResources7` and
  `testResourcesIn3`. They call a function the R4 engine does not implement, so
  they fail at the **parse** stage and are graded `failed` under
  `path.oracle.005`'s stage rule, as `path.oracle.014` states. HAPI fails them
  the same way (`FHIRPathTests.java:199-202`)

#### Scenario: [path.errors.002] an error carries HAPI's message, a position only where HAPI's failing node has a start, and the sub-expression's text only where HAPI's template includes it

- **WHEN** `1 < 'a'` is evaluated, and then `(1=1) and 1 < 'a'` and
  `(1=1) and (1=1) and 1 < 'a'`
- **THEN** each fails with an execution error whose message is exactly
  `Error evaluating FHIRPath expression: Unable to compare values of type integer and string`
  followed by ` (@char 1)`, ` (@char 13)` and ` (@char 25)` respectively:
  `makeException` formats `FHIRPATH_CANT_COMPARE` with both `fhirType()`s
  (`FHIRPathEngine.java:2435`, `:584-594`) and constructs
  `PathEngineException(message, id, holder.getStart(), holder.toString())`,
  whose constructor appends only `rep(location, expression)`
  (`org.hl7.fhir.utilities/src/main/java/org/hl7/fhir/exceptions/PathEngineException.java:81-93`)
- **AND** `@char` is HAPI's lexer **column**, which is not the byte offset plus
  one. The column starts at 1 (`SourceLocation(1, 1)`, `FHIRLexer.java:56`) and
  advances as follows:
  - a token adds its length once (`incColumn(cursor - currentStart)`,
    `FHIRLexer.java:285`), the length of a string or delimited token counting
    its quotes and escapes as written;
  - a whitespace character other than CR and LF adds **2**:
    `skipWhitespaceAndComments` calls `checkChar` (`setColumn(column + 1)`) and
    then `incColumn()` (`FHIRLexer.java:321-324`, `SourceLocation.java`
    `checkChar`, `incColumn`);
  - a CR, or an LF not preceded by a CR, starts a new line at column 1
    (`newLine`), and the `incColumn()` that follows makes it column 2; an LF
    after a CR adds 1;
  - a `//` comment adds 1 per character, the `//` included, up to the line end
    (`:294-303`); a `/* */` comment adds 2 per character from the `/*` up to
    the `*/`, with CR and LF handled as above, and 2 for the `*/`
    (`:304-320`);
  - a node's position is the column at which its first token starts
    (`currentStartLocation`, `:147`; `setStart`, `FHIRPathEngine.java:1076`)
- **AND** so in `(1=1) and 1 < 'a'` the `1` starts at byte offset 10 and column
  13 (`(1=1)` reaches 6, the space 8, `and` 11, the space 13), and in
  `(1=1) and (1=1) and 1 < 'a'` at byte offset 20 and column 25. HAPI's own
  record shows the rule: `isSemanticChecks/isEffectOnLogic1` reports `toString`
  in `$this is dateTime implies $this.toString().length() >= 10`, at byte
  offset 32 after four spaces, as ` (@char 37)`, and `isEffectOnLogic1Base`
  (`$this.toString().length() >= 10`, no space) as ` (@char 7)`
  (`oracles/fhirpath/hapi-8.12.0-results.json:1046`, `:1043`)
- **AND** a node on a later line is reported as ` (@line L char C)` with the
  same column rule: `(1=1) and` followed by an LF and `1 < 'a'` fails with
  ` (@line 2 char 2)`
- **AND** Frevo's error `offset` is a separate value from the message suffix:
  it is the zero-based byte offset of the failing node's first token in the
  expression text (0, 10 and 20 for the three expressions above), present
  exactly when HAPI's message carries a position suffix, and absent otherwise.
  The suffix in the message is computed by HAPI's column rule, never derived as
  `offset + 1`. A unary sign's wrapper node is the one holder whose recorded
  start is not its first token (`path.errors.006`)
- **WHEN** `1 + 1 < 'a'` is evaluated, and then `(1=1) and 1 + 1 < 'a'`
- **THEN** each fails with the message exactly
  `Error evaluating FHIRPath expression: Unable to compare values of type integer and string`,
  with **no** position suffix at all
- **AND** the reason is that the node carrying the failing `<` is a precedence
  group, which has no start: `organisePrecedence` runs `gatherPrecedence` once
  per precedence level (`FHIRPathEngine.java:1224-1237`), and the `+` level
  wraps `1 + 1` in a node built by `newGroup` (`:1239-1311`, `:1315-1321`),
  which never sets a start, then moves `<` onto that group. `execute` passes the
  group as the holder (`last = exp`, `:1508-1522`; `operate`, `:1843-1850`),
  `makeException` passes its `null` start (`:589-590`), and
  `rep(null, <non-empty text>)` returns `""` (`PathEngineException.java:81-93`).
  In `(1=1) and 1 + 1 < 'a'` the `<` node is again the group the `+` level
  built
- **AND** the same holds for any failing operator whose left operand is a run
  of operators of higher precedence, gathered by `organisePrecedence`
  (`a + b < c`, `x * 2 > 'y'`, `a | b = c`), and for any error raised with a
  `null` holder (`PathEngineException(fmt, constName)`, `:591-592`)
- **AND** for these errors Frevo's error carries **no** offset, because HAPI's
  exception exposes no position for them: `PathEngineException` never assigns
  its `location` field (`PathEngineException.java`, constructors), so the
  message suffix is the only place HAPI reports one
- **WHEN** `(1 + 1) < 'a'` is evaluated
- **THEN** the message ends ` (@char 1)` and the offset is 0: a parenthesised
  group is built by `parseExpression` itself, which sets its start
  (`:1076`, `:1135-1143`), and the `<` level finds no work to gather
- **AND** for an error built through `makeException`, `makeExceptionPlural` or
  `rep`, the message does **not** contain the failing sub-expression's text
  (`1 < 'a'`): `rep`'s `expression` argument is used only when the location is
  `null` and the expression text is empty, where it appends ` (@~` and the
  empty text and `)`; with a non-null location on a later line it appends
  ` (@line L char C)` instead of ` (@char C)`
- **AND** the one message template that does carry a node's text is
  `FHIRPATH_AS_COLLECTION` (`Messages.properties:301`,
  `Attempt to use ''as()'' on more than one item (''{0}'', ''{1}'')`), which
  `opAs` and `funcAs` format with the holder's `toString()`
  (`FHIRPathEngine.java:1884-1885`, `:5166-5167`; `ExpressionNode.toString`,
  `ExpressionNode.java:396-464`) and construct without a location; the exact
  messages are in `path.strictness.003`
- **AND** an engine with no `location` configured, which is every engine the
  suite and the harness run, adds no ` (at ...)` fragment
  (`FHIRPATH_LOCATION`, `:586-588`)
- **AND** the position rule holds for every execution error raised through
  `makeException` or `makeExceptionPlural` (`:572-594`) with a non-null holder
  whose start is set: a term `parseExpression` built (a literal, a name, a
  function call, a parenthesised group), with the column rule above. A holder
  built by `newGroup`, the indexer node built for `[...]` (which is created
  without a start, `:1188-1192`), or a `null` holder gives the bare message and
  no offset. This covers check-stage errors raised the same way, such as
  `path.check.005`'s and `path.utility.003`'s.
- **AND** errors the engine constructs directly rather than through
  `makeException` follow their own location argument:
  - the date-arithmetic and calendar-unit errors pass the **operator token's**
    start, `getOpStart()` (`FHIRPATH_ARITHMETIC_PLUS`, `_MINUS`, `_QTY`,
    `_UNIT`, `FHIRPathEngine.java:2063`, `:2080`, `:2725`, `:2731`, `:2763`),
    so the suffix and Frevo's offset name the `+` or `-` token;
  - the wrapped `UcumException` of a quantity multiplication in `opTimes`
    (`:2801`) passes `getOpStart()` too, so its suffix and offset name the `*`
    token;
  - in each case the column rule above applies, and the suffix and offset are
    absent when the operator was moved onto a `newGroup` node, which has no
    operator start either;
  - the variable, `as` and type-name errors pass no location (`:1007`, `:1014`,
    `:1057`, `:1064`, `:1882`, `:1885`, `:5164`, `:5167`, `:5207`), and the
    decimal arm of `opMod` wraps its `UcumException` as
    `new PathEngineException(e)` (`:3102`, raised by `5.5 mod 0` and
    `5 mod 0.0`, `path.math.007`, `.008`), whose message is the cause's
    `toString()`: these carry no suffix, and Frevo's error carries no offset
- **AND** the 21 evaluation-stage cases of `path.errors.001` are graded on
  their stage, not on their message

#### Scenario: [path.errors.003] evaluation never panics

- **WHEN** every expression in the suite, including the invalid ones, is parsed,
  checked and evaluated
- **THEN** no case panics; every failure is a returned error

#### Scenario: [path.errors.004] evaluation is bounded for an adversarial expression

- **WHEN** an expression nests 10,000 parentheses, or repeats a projection over
  a cyclic structure
- **THEN** the engine returns a depth or step-limit error rather than
  overflowing the stack or running unbounded

#### Scenario: [path.errors.005] an error from `frevo-model` is surfaced, not swallowed into empty

- **WHEN** navigation asks the model for a child and the model reports a decode
  error
- **THEN** evaluation fails with that error
- **AND** the result is not an empty collection, which is the failure mode
  ADR 0004 identifies as the worst available one

#### Scenario: [path.errors.006] a unary sign's error position is HAPI's, just past the token after the sign

- **WHEN** `-'a'` and `- 'a'` are evaluated with the static check skipped
- **THEN** each fails with the message exactly
  `Error evaluating FHIRPath expression -: left and right operand have incompatible or invalid types (integer, string)`
  (`FHIRPATH_OP_INCOMPATIBLE`, `Messages.properties:350`), raised by `opMinus`
  for `0 - 'a'` (`FHIRPathEngine.java:2977-2978`), followed by ` (@char 5)`
  and ` (@char 7)` respectively
- **AND** the reason is where HAPI records the sign's position. A leading `-`
  or `+` becomes a `Unary` wrapper node carrying the sign as its operation
  (`:1082-1088`), executed as the integer `0` followed by that operation on the
  operand (`case Unary`, `:1470-1471`; `:1508-1522`), so the wrapper is the
  error's holder. Its start is read with `getCurrentLocation()` **after**
  `lexer.take()`, which has already lexed the next token
  (`FHIRLexer.java:101-105`), so it is the column just past the operand's
  first token, not the sign's: `-` reaches column 2 and `'a'` 5; with the
  space, 2, 4 and then 7
- **AND** Frevo's error offset is the zero-based byte offset of that same
  position, just past the operand's first token: 4 for `-'a'` and 5 for
  `- 'a'`
- **AND** no live case grades it, so it is differential-only

#### Scenario: [path.errors.007] the `iif` singleton and `is` parameter-kind errors carry HAPI's prefix and position

- **WHEN** `(1 | 2).iif(true, 'a')` is evaluated
- **THEN** it fails with the message exactly
  `Error evaluating FHIRPath expression: The function iif can only be used on a singleton value but found 2 (@char 11)`.
  `funcIif` throws `FHIRPATH_NO_COLLECTION` for a focus of more than one item
  (`FHIRPathEngine.java:5007-5010`, `Messages.properties:344`) with the `iif`
  function node as the holder, at column 11 and byte offset 8; the static
  check only records a warning for it (`checkSingleton`, `:3839-3843`)
- **WHEN** `1.is('x')` and `1.is(FHIR.toString())` are evaluated with the static
  check skipped
- **THEN** they fail with the messages exactly
  `Error evaluating FHIRPath expression: The expression type Constant is not supported for parameter 0 on function is (@char 3)`
  and
  `Error evaluating FHIRPath expression: The expression type Name is not supported for parameter 1 on function is (@char 3)`
  (`FHIRPATH_PARAM_WRONG`, `Messages.properties:352`; `funcIs`,
  `:5102-5109`), the holder being the `is` function node, at column 3 and byte
  offset 2
- **AND** the second message names the kind of the **outer** parameter node
  (`Name`), not that of the inner node that failed the test, because `funcIs`
  passes `texp.getKind()` in both branches (`:5108`)
- **AND** no live case grades the messages, so they are differential-only

### Requirement: The public evaluation surface

The crate SHALL expose a synchronous evaluation surface that separates parsing from evaluation, so a caller can parse once and evaluate many times inside a write transaction.

Shaped after `ca.uhn.fhir.fhirpath.IFhirPath`, so that the callers this port
will add later — search-parameter extraction, validation invariants, CQL — find
the operations they expect.

#### Scenario: [path.api.001] an expression can be evaluated from its text

- **WHEN** an expression string and a node are given to the evaluate operation
- **THEN** the result collection is returned

#### Scenario: [path.api.002] a parsed expression can be evaluated repeatedly

- **WHEN** an expression is parsed once and evaluated against 1,000 nodes
- **THEN** every evaluation returns its own result and the expression is parsed
  once

#### Scenario: [path.api.003] a first-match operation returns at most one value

- **WHEN** the evaluate-first operation is used with an expression that matches
  several items
- **THEN** exactly one value, the first in order, is returned

#### Scenario: [path.api.004] a first-match operation on no match returns nothing

- **WHEN** the evaluate-first operation is used with an expression that matches
  nothing
- **THEN** no value is returned, and this is distinguishable from a value that
  is empty

#### Scenario: [path.api.005] a result of an unexpected type is reported, not silently dropped

- **WHEN** a caller asks for results of one type and the expression yields
  another
- **THEN** an error naming both types is returned, as `FhirPathR4` does: HAPI
  checks every result item with `theReturnType.isAssignableFrom(next.getClass())`
  and fails on the first that is not (`FhirPathR4.java:72-77`)
- **AND** the requested type is one of what HAPI's own callers pass to
  `IFhirPath.evaluate` and `evaluateFirst`:
  - `Base` (HAPI's `IBase` or `Base`), which accepts every item;
  - the interface kind **primitive** (`IPrimitiveType`, passed by
    `FieldValidatingInterceptor.java:105`, `DaoResourceLinkResolver.java:466`,
    `:468`, `ServeMediaResourceRawInterceptor.java:92`, `:100`,
    `JpaPackageCache.java:418`, `PackageInstallerSvcImpl.java:1081`);
  - the interface kind **composite** (`ICompositeType`,
    `SearchParameterAndValueSetRuleImpl.java:232`);
  - the interface kind **reference** (`IBaseReference`,
    `BulkExportMdmFullResourceExpander.java:252`,
    `OverridePathBasedReferentialIntegrityForDeletesInterceptor.java:123`);
  - a FHIR type name, standing for HAPI's R4 model class for that type
    (`string` for `StringType`, `HumanName` for `HumanName`, `Patient` for
    `Patient`, and the abstract `Resource`, `DomainResource`, `Element` and
    `BackboneElement` for the classes of the same names)
- **AND** an item is accepted exactly when the requested type is among the
  **R4 Java class ancestry** of the item's HAPI model class: the class, its
  superclasses and every interface they implement, directly or through
  another interface, as declared in the pinned model sources
  (`reference/org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/model/*.java`
  at `73c432b5`, and the `org.hl7.fhir.instance.model.api` interfaces in
  `reference/hapi-fhir/hapi-fhir-base` at `v8.12.0`). The facts that decide
  the examples below are:
  - every primitive class extends `PrimitiveType`, which implements
    `IPrimitiveType`: `StringType`, with `CodeType` and `MarkdownType` under
    it; `UriType`, with `UrlType`, `CanonicalType`, `OidType`, `UuidType`,
    `SidType` and **`IdType`** under it (`IdType.java:120`); `IntegerType`,
    with `PositiveIntType` and `UnsignedIntType` under it; `BaseDateTimeType`,
    with `DateType`, `DateTimeType` and `InstantType` under it; and
    `BooleanType`, `DecimalType`, `Base64BinaryType`, `TimeType`, `XhtmlType`;
  - a `code` element that HAPI's model declares as `Enumeration<...>` (for
    example `Patient.gender`, `Patient.java:1508`; `Observation.status`,
    `Observation.java:1715`) is an **`Enumeration`**, which extends
    `PrimitiveType` and is neither a `CodeType` nor a `StringType`
    (`Enumeration.java:76`); every other `code` element is a `CodeType`;
  - every complex datatype class implements `ICompositeType`, directly or
    through `IBaseReference` (`Reference`), `IBaseMetaType` (`Meta`),
    `INarrative` (`Narrative`) or `IBaseExtension` (`Extension`); `Age`,
    `Count`, `Distance`, `Duration`, `MoneyQuantity` and `SimpleQuantity`
    extend `Quantity`;
  - a resource's nested component (`Patient.contact`, a `ContactComponent`)
    extends `BackboneElement` and is **not** an `ICompositeType`
    (`Patient.java:230`, `BackboneElement.java:49`);
  - a resource extends `DomainResource` or `Resource` (`Patient.java:53`,
    `DomainResource.java:48`, `Resource.java:45`);
  - values the engine builds have the class the engine builds: a string,
    integer, decimal, boolean, date, dateTime or time result is a
    `StringType`, `IntegerType`, `DecimalType`, `BooleanType`, `DateType`,
    `DateTimeType` or `TimeType`, a quantity a `Quantity`, and `Resource.id`
    an `IdType` (`filterIdType`, `FHIRPathEngine.java:280-284`);
  - `Narrative.div` is the one element whose runtime class differs from its
    declared one: the `@Child` declares `XhtmlType` (`Narrative.java:232`), but
    navigating to `div` by name returns a **`StringType`** holding the composed
    XHTML (`getProperty`, `Narrative.java:364-366`), while `children()` and
    `descendants()`, which navigate with `*`, return an `XhtmlType`
    (`listChildren`, `Narrative.java:343`; `Base.listChildrenByName`,
    `Base.java:226-236`; `FHIRPathEngine.java:4819`, `:4833`). So
    `Patient.text.div` is accepted as `string` and as primitive, and as
    `xhtml` fails with
    `HAPI-0256: FhirPath expression returned unexpected type StringType - Expected org.hl7.fhir.r4.model.XhtmlType`;
    `Patient.text.children()` yields an `XhtmlType` among its items
    (`path.navigation.025`)
- **AND** the table of each FHIR type's R4 class ancestry, of which `code`
  elements are `Enumeration`s, and of the runtime exceptions such as
  `Narrative.div` above, is generated, emitted as an `@generated` table and
  never hand-written. Its content must equal what this scenario states; a
  source that reads only declarations (field types, `@Child` annotations)
  misses the `div` exception and must carry it explicitly. Which source the
  generator reads is open in `tasks/blockers.md` (2026-11-20, "the R4 Java
  class ancestry table")
- **AND** every result item is checked, not only the first, and the error is
  raised for the first item that is not accepted, also by the evaluate-first
  operation, which checks the whole result before taking its first item
  (`FhirPathR4.java:80-89`). An empty result satisfies any requested type
- **AND** the error is an execution error, with no offset, whose message is
  exactly
  `HAPI-0256: FhirPath expression returned unexpected type <item class> - Expected <requested type>`
  (`Msg.code(256)`, `Msg.java:23`, `:31-32`, rendered `HAPI-` and the code as
  four digits and `: `). `<item class>` is the item's class's Java simple name
  (`getSimpleName()`): `HumanName`, `Enumeration`, `IdType`, `DateType`, or a
  nested component's own name without its outer class, `ContactComponent`.
  `<requested type>` is the requested type's Java fully qualified name
  (`getName()`): `org.hl7.fhir.r4.model.<Class>` for a model class, with `$`
  before a nested class's name (`org.hl7.fhir.r4.model.Patient$ContactComponent`),
  and `org.hl7.fhir.instance.model.api.IPrimitiveType`,
  `org.hl7.fhir.instance.model.api.ICompositeType`,
  `org.hl7.fhir.instance.model.api.IBaseReference` and
  `org.hl7.fhir.instance.model.api.IBase` for the kinds
- **AND** over `patient-example.xml` and `observation-example.xml`:
  - primitive: `Patient.gender` (an `Enumeration`), `Patient.active` and
    `Patient.birthDate` are accepted; `Patient.name` fails with
    `HAPI-0256: FhirPath expression returned unexpected type HumanName - Expected org.hl7.fhir.instance.model.api.IPrimitiveType`;
    and `Patient.active | Patient.name` fails the same way, also through
    evaluate-first
  - composite: `Observation.code` and `Observation.subject` are accepted;
    `Observation.status` fails with
    `HAPI-0256: FhirPath expression returned unexpected type Enumeration - Expected org.hl7.fhir.instance.model.api.ICompositeType`,
    and `Patient.contact` with
    `HAPI-0256: FhirPath expression returned unexpected type ContactComponent - Expected org.hl7.fhir.instance.model.api.ICompositeType`
  - reference: `Observation.subject` is accepted; `Observation.code` fails
    with
    `HAPI-0256: FhirPath expression returned unexpected type CodeableConcept - Expected org.hl7.fhir.instance.model.api.IBaseReference`
  - FHIR type names: `Patient.name.family` as `string` and `Patient.id` as
    `uri` are accepted; `Patient.name` as `string` fails with
    `HAPI-0256: FhirPath expression returned unexpected type HumanName - Expected org.hl7.fhir.r4.model.StringType`
    (`FhirPathTest.testEvaluateWrongType`); `Patient.id` as `string` with
    `... unexpected type IdType - Expected org.hl7.fhir.r4.model.StringType`;
    `Patient.gender` as `code` with
    `... unexpected type Enumeration - Expected org.hl7.fhir.r4.model.CodeType`;
    and `Patient.birthDate` as `string` with
    `... unexpected type DateType - Expected org.hl7.fhir.r4.model.StringType`
- **AND** the wrapping of a parse or evaluation failure inside `FhirPathR4`
  (`HAPI-2409: `, `HAPI-0255: `, `FhirPathR4.java:49`, `:69`) is not part of
  this scenario; see the proposal's out-of-scope table
- **AND** no live case grades it, so it is graded by Frevo's own tests against
  these statements and by the server changes whose callers pass these types

#### Scenario: [path.api.006] evaluation is synchronous

- **WHEN** an expression is evaluated
- **THEN** the operation completes without an asynchronous runtime, so it can be
  called from inside a write transaction

#### Scenario: [path.api.007] a host service is optional and replaceable

- **WHEN** an engine is created with no host service, and again with one
- **THEN** both evaluate successfully for expressions that need no host
  service, and only the second resolves host-service-dependent functions

#### Scenario: [path.api.008] `frevo-path` does not depend on `frevo-model`

- **WHEN** `frevo-path`'s manifest and sources are read
- **THEN** neither names `frevo-model`, and the crate compiles and its unit
  tests pass without it (ADR 0004 decision 5)

### Requirement: The oracle harness and the gate

The oracle harness SHALL run every case in the vendored suite without modifying it, SHALL attribute each failure to the parse, check or evaluate stage, and SHALL report a per-case status that never confuses a case that did not run with one that passed.

#### Scenario: [path.oracle.001] the harness parses every case in the suite

- **WHEN** the harness reads `oracles/fhirpath/tests-fhir-r4.xml`
- **THEN** it reports 103 groups and 1,191 cases, counted by XML parse and not
  by text match
- **AND** a count that differs fails the run, because the suite has been
  re-pinned without the harness being updated
- **AND** the harness counts `<group>` and `<test>` elements the XML parser
  returns, so the commented-out `testDollarResource` block at
  `tests-fhir-r4.xml:1576-1582` — whose bodies are leftover Java text, not
  well-formed test elements — contributes nothing

#### Scenario: [path.oracle.002] the harness reads every attribute the case carries

- **WHEN** the harness parses the pinned suite
- **THEN** it accepts, per element, exactly the attributes that element carries
  in the pinned file, and no others:
  - `<tests>` (root) — `name`, `description`, `reference`
  - `<group>` — `name`, `description`, `mode`, `reference`
  - `<test>` — `name`, `description`, `inputfile`, `mode`, `context`,
    `contextType`, `version`, `testing`, `predicate`, `ordered`,
    `checkOrderedFunctions`
  - `<expression>` — `invalid`, `mode`
  - `<output>` — `type`
- **AND** an attribute on an element where that element does not carry it in the
  pinned file fails the run rather than being silently ignored, because an
  unread attribute is a silently weakened oracle
- **AND** a run against the pinned file therefore reports **zero** unhandled
  attributes, which is the assertion this scenario exists to make: `reference`
  appears on the root `<tests>` element and on `<group name="from-Zulip">`
  (`tests-fhir-r4.xml:1583`), and `mode` appears on one `<expression>`
  (`tests-fhir-r4.xml:625`), on 11 `<test>` elements and on two `<group>`
  elements
- **AND** the attribute value domains the pinned file uses are recorded and
  asserted: `expression/@invalid` ∈ {`syntax`, `semantic`, `execution`},
  `expression/@mode` ∈ {`strict`}, `test/@mode` ∈ {`element`, `strict`},
  `group/@mode` ∈ {`custom`, `semantic`}, `test/@predicate` ∈ {`true`, `false`},
  `test/@ordered` ∈ {`false`}, `test/@checkOrderedFunctions` ∈ {`true`},
  `test/@version` ∈ {`2.1.0`}; a value outside the recorded domain fails the run
  with status `did-not-run`, never `passed`
- **AND** `tests/@reference` and `group/@reference` are recorded and reported but
  change no grading, `test/@testing` drives `path.oracle.014`'s out-of-scope
  reporting, and the expression-level `mode` is handled as `path.oracle.017`
  requires

#### Scenario: [path.oracle.017] the expression-level `mode` attribute changes no outcome

- **WHEN** `testIif/testIif6` is run, whose expression element is
  `<expression mode="strict" invalid="semantic">iif('non boolean criteria', 'true-result', 'true-result')</expression>`
  at `tests-fhir-r4.xml:625` — the only expression-level `mode` in the pinned
  file, and one of the 27 cases `path.check.002` requires to fail the check
- **THEN** the case fails the check pass and is therefore graded `passed`,
  identically to a run that ignored the attribute, which is what HAPI's pinned
  harness does
- **AND** the harness records that it read `mode="strict"` on the expression and
  that it changed no grading, rather than leaving the attribute unhandled
- **AND** the outcome does not depend on the attribute because the engine types
  `iif`'s first parameter against a singleton boolean unconditionally — the
  `case Iif` arm of the type pass runs `checkParamTypes` with no mode gate — so
  the case is a semantic failure in every configuration the engine offers
- **AND** the harness must not implement the attribute as "fail the check only in
  strict mode", because that would let an implementation which accepts a
  non-boolean `iif` condition in the default configuration pass this case

#### Scenario: [path.oracle.003] the harness loads each referenced input file

- **WHEN** the harness resolves an `inputfile` value
- **THEN** it loads it from `oracles/fhir-test-cases/fetched/r4/`, which is the
  directory `TestingUtilities.loadTestResource("r4", input)` resolves to
- **AND** a missing input file fails the run with status `did-not-run`, never
  `passed`

#### Scenario: [path.oracle.004] the harness handles both XML and JSON inputs

- **WHEN** the harness loads `patient-example.xml` and
  `appointment-examplereq.json`
- **THEN** both become navigable nodes over which the same expression evaluates
  identically

#### Scenario: [path.oracle.005] the harness evaluates in three stages and attributes the failure to the right one

- **WHEN** a case is run
- **THEN** parse, check and evaluate run in that order, and a case expecting a
  `syntax`, `semantic` or `execution` failure passes only if the failure comes
  from the matching stage

#### Scenario: [path.oracle.006] the harness applies `predicate="true"` as a boolean conversion

- **WHEN** a case carries `predicate="true"`
- **THEN** the outcome is converted to a single boolean before comparison, as
  `FHIRPathTests` does with `convertToBoolean`

#### Scenario: [path.oracle.007] the harness applies `ordered="false"` as an unordered comparison

- **WHEN** a case carries `ordered="false"` — which in the pinned file is exactly
  one case, `testMiscellaneousAccessorTests/testExpressions` at
  `tests-fhir-r4.xml:321`
- **THEN** each outcome item is matched against **any** expected `<output>`
  element rather than against the one at its index, and the item passes if some
  expected element satisfies both of (`type` absent or empty, or `type` equal to
  the item's FHIR type name) and (body absent or empty, or body equal to the
  item's string value)
- **AND** the unordered branch compares the **plain** string value — a
  `Quantity` item through the engine's quantity-to-string conversion and every
  other item through its primitive string value — and therefore carries **no**
  `@` or `@T` prefix, unlike the ordered branch (`path.oracle.008`)
- **AND** the two branches must not be unified on one comparison function: the
  one `ordered="false"` case in the pinned file yields strings, so the
  divergence is latent today and would surface as a whole-suite failure the
  first time a temporal result appears in an unordered case
- **AND** an expected element may be matched by more than one outcome item,
  because the match is existential per item and consumes nothing

#### Scenario: [path.oracle.008] the harness compares count, and compares type and value only when the expectation states them

- **WHEN** a case's outputs are compared on the ordered branch, which is every
  case without `ordered="false"`
- **THEN** the number of outputs must match the number of result items
- **AND** an output whose `type` attribute is absent or empty means "do not
  check the type", and an output whose body is absent or empty means "do not
  check the value" — 71 of the suite's 1,103 outputs carry no `type` attribute,
  so a harness that required one would fail cases HAPI passes
- **AND** where the `type` attribute is present it must equal the result item's
  FHIR type name, and where the body is present it must equal the result item's
  FHIRPath string value, which is **not** the primitive value for a temporal
  type: a `date`, `dateTime` or `instant` result renders as `@` followed by its
  primitive value, a `time` result as `@T` followed by its primitive value, and
  every other primitive as its primitive value unchanged
- **AND** that is why the suite's expectations read
  `<output type="date">@1974-12-25</output>` (`tests-fhir-r4.xml:59`) and
  `<output type="dateTime">@1974-01-01T00:00:00.000+10:00</output>`
  (`tests-fhir-r4.xml:1282`), and the harness compares the prefixed form
- **AND** the prefix is a property of the comparison, not of the result: the
  engine's own string conversion, the `toString()` function and the value Frevo
  returns to a caller all carry no `@`
- **AND** stripping the `@` from an expectation, or normalising it away on
  either side, is weakening the oracle and is forbidden by constitution rule 3;
  a mismatch on a temporal case is a bug in the renderer the harness compares
  with, not in the suite

#### Scenario: [path.oracle.009] the harness picks the Quantity comparison from the result item's type, not from the expectation

- **WHEN** a result item is a `Quantity`
- **THEN** the expected text is parsed as a quantity string and compared deeply,
  as `FHIRPathTests` does with `parseQuantityString` and `equalsDeep`
- **AND** the branch is chosen by the **result item's** runtime type, because
  the suite has exactly one `<output type="Quantity">` while
  `LowBoundary/LowBoundaryQuantity` and `HighBoundary/HighBoundaryQuantity`
  expect `1.58650000 'cm'` and `1.58750000 'm'` from a bare `<output>`
- **AND** that one typed case is `testTypes/testStringIntegerLiteralToQuantity`
  (`'1'.toQuantity()`, expecting `1 '1'`, `tests-fhir-r4.xml:413`), in group
  `testTypes`, not `testLiterals`; HAPI records it `passed`
  (`oracles/fhirpath/hapi-8.12.0-results.json:186`), and it is graded together
  with the two boundary cases, all three required `passed`
- **AND** every other result item is compared as a primitive, and a non-Quantity
  result item that is not a primitive fails the case

#### Scenario: [path.oracle.010] the harness reports per-case status with four distinct values

- **WHEN** the harness finishes
- **THEN** every case has one of `passed`, `failed`, `did-not-run` or
  `skipped-for-licence`, and `did-not-run` is never counted as a pass

#### Scenario: [path.oracle.011] the harness records HAPI's result for the same case

- **WHEN** the harness report is written
- **THEN** each case carries HAPI 8.12.0's measured status alongside Frevo's, so
  the gate can be evaluated per case rather than only in aggregate
- **AND** HAPI's status is one of `passed`, `failed` or `hapi-cannot-pass`, the
  last for a case the pinned R4 harness cannot express because it reads none of
  `mode`, `context`, `contextType` or `version` (`path.oracle.016`)

#### Scenario: [path.oracle.012] the gate is pass rate at or above HAPI's on the same cases

- **WHEN** the gate is evaluated
- **THEN** it passes only if Frevo's `passed` count is at least HAPI's
- **AND** a case HAPI passes that Frevo fails is listed individually in the gate
  report, regardless of the aggregate
- **AND** a case Frevo passes that HAPI fails is listed too, under a status of
  `ahead-of-hapi`, so the aggregate cannot be read as parity when it is in fact
  a surplus

#### Scenario: [path.oracle.016] a case HAPI structurally cannot pass is recorded as such, and Frevo is graded on its own run

- **WHEN** the harness runs a group of two or more cases that have a
  byte-identical expression and the same `inputfile`, that differ only by their
  `context` and `contextType` attributes, and whose expected outcomes differ
  (an `invalid` value, or a different output)
- **THEN** each case in the group has its HAPI status recorded as
  `hapi-cannot-pass`, a status distinct from `failed`. The pinned R4 harness
  reads none of `mode`, `context`, `contextType` or `version` and always checks
  against the resource root (`FHIRPathTests.java:219-224`), so it gives every
  case in the group the same outcome and necessarily fails at least one
- **AND** in the pinned suite that rule selects exactly six groups, 13 cases, all
  in `isSemanticChecks`. They were computed mechanically by grouping every
  `<test>` in `tests-fhir-r4.xml` on expression text and `inputfile`:
  - `isEffectOnLogic2Base`, `isEffectOnLogic3Base`
  - `isGuardOr`, `isGuardOr2`, `isGuardOrBad`
  - `isGuardXorBad`, `isGuardXorBad2`
  - `isGuardOrInnerImplies`, `isGuardOrInnerImpliesBad`
  - `isGuardOrTypesImpliesBad`, `isGuardOrTypesImpliesAll`
  - `isGuardNotImplies`, `isGuardNotImpliesBad`
- **AND** the only other groups sharing an expression but differing in
  `context`/`contextType` are excluded, because their expected outcomes are
  identical, so HAPI is not forced to fail any of them. Their HAPI status is
  whatever HAPI's run measures:
  - `isEffectOnLogic1` and `isGuardImpliesEmpty`
  - `isEffectOnLogic2` and `isEffectOnLogic3`
- **AND** Frevo's status for those cases comes from its own context-aware run,
  which honours all four attributes (`path.oracle.002`). It is **not** `passed`
  for every member. Of the 13:
  - 3 expect success but end at the check stage graded `failed`, because the
    check pass does not narrow (`path.check.005`, `path.check.009`):
    `isGuardOr`, `isGuardOrInnerImplies` and `isGuardNotImplies`
  - 6 are `invalid="semantic"`, fail the check and are graded `passed`:
    `isEffectOnLogic3Base`, `isGuardOrBad`, `isGuardXorBad`,
    `isGuardOrInnerImpliesBad`, `isGuardOrTypesImpliesBad` and
    `isGuardNotImpliesBad`
  - 4 expect success, pass the check, and are graded by evaluation:
    `isEffectOnLogic2Base`, `isGuardOr2`, `isGuardOrTypesImpliesAll` and
    `isGuardXorBad2`. Despite its name, `isGuardXorBad2` carries no `invalid`
    attribute and expects `false` (`tests-fhir-r4.xml:1777-1779`)
- **AND** `path.check.002` is graded against that context-aware run, not against
  HAPI's status. Requiring 27 semantic-invalid cases to fail the check is
  therefore consistent with the aggregate gate rather than stricter than it. The
  other two, `documentFunctions/testResourcesAbstract3` and
  `testHasCoding11`, fail at parse and are graded `failed` for Frevo and HAPI
  alike (`path.check.002`, `path.oracle.014`)

#### Scenario: [path.oracle.013] the harness can run the same cases through `helios-fhirpath`

- **WHEN** the harness is asked for the differential reference
- **THEN** it runs the same cases through `helios-fhirpath` 0.2.7 and reports a
  three-way comparison
- **AND** `helios-fhirpath` appears in no shipped crate's dependency graph
  (ADR 0004 decision 3)

#### Scenario: [path.oracle.014] the `documentFunctions` group is reported as out of scope; only its four syntax-invalid cases pass

- **WHEN** the harness runs the 115 cases in the `mode="custom"`
  `documentFunctions` group, every one of which calls at least one of
  `section`, `entry`, `entryIn`, `resources`, `resourcesIn`, `sameSubject` or
  `hasCoding`. The R4 engine implements none of them, so every case fails at
  the **parse** stage
- **THEN** the unknown function is reported as a real parse error, a
  diagnostic and not "not implemented". Otherwise the harness would grade the
  cases `did-not-run` (`run.rs`'s classification of a not-implemented error)
- **AND** the four `invalid="syntax"` cases are graded `passed`, because a
  parse-stage failure is what they expect (`path.oracle.005`):
  `testSection13` (`tests-fhir-r4.xml:1860`), `testEntry12` (`:1879`),
  `testSameSubject16` (`:1930`) and `testHasCoding10` (`:1950`)
- **AND** the other 111 are reported `failed` with the reason that the R4 engine
  does not implement the function. That is 103 that expect success, 6 that
  expect an `execution` failure and 2 that expect a `semantic` failure
  (`path.check.002`, `path.errors.001`)
- **AND** HAPI's recorded status is the same, 4 `passed` and 111 `failed`. Its
  harness passes a syntax case when parsing fails and fails every other case
  whose parse fails (`FHIRPathTests.java:199-202`), so the aggregate gate is
  unaffected

#### Scenario: [path.oracle.015] the harness does not modify the suite

- **WHEN** `bash oracles/fetch.sh --verify` runs after a harness run
- **THEN** `tests-fhir-r4.xml` is byte-identical to its pinned content

### Requirement: FHIRPath performance budgets

`frevo-path` SHALL meet the budgets below, which are gate conditions, and SHALL report measured before and after figures for any change that touches them.

Budgets, their derivation and the workloads that measure them are in
[`../../design.md`](../../design.md). They are gate conditions, and they exist
because this engine runs inside the write transaction.

#### Scenario: [path.perf.001] compiling an expression stays within budget

- **WHEN** the `path-compile` workload parses and checks a typical
  search-parameter expression
- **THEN** the median time is at most 20 µs

#### Scenario: [path.perf.002] evaluating a compiled expression does not re-parse it

- **WHEN** the `path-eval-compiled` workload evaluates a pre-parsed expression
  1,000 times
- **THEN** no parse is performed after the first
- **AND** the median per-evaluation time is at most 5 µs for a simple path over
  a 4 KiB `Patient`

#### Scenario: [path.perf.003] extracting a resource's whole search-parameter set stays within budget

- **WHEN** the `path-extract-patient` workload evaluates every enabled `Patient`
  search-parameter expression over one 4 KiB `Patient`
- **THEN** the median total time is at most 300 µs, which is the per-resource
  share of the 5,000 resources/s bulk-ingest target in `bench/targets.md`

#### Scenario: [path.perf.004] evaluation over the lazy model does not force full materialisation

- **WHEN** the `path-extract-patient` workload runs over a byte-backed resource
- **THEN** the elements no expression touches report as not materialised

#### Scenario: [path.perf.005] the whole suite runs fast enough to be a per-pull-request gate

- **WHEN** the oracle harness runs all 1,191 cases
- **THEN** the run completes in at most 10 s on the reference environment
  described in `bench/targets.md`

#### Scenario: [path.perf.006] no regression against the recorded baseline

- **WHEN** any `bench/micro/` path workload is compared with the baseline
  recorded for the previous commit on `main`
- **THEN** no workload is more than 10% slower, per the regression policy in
  `bench/targets.md`
