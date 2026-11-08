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

#### Scenario: [path.navigation.002] the resource type name may be omitted

- **WHEN** `name.given.first()` is evaluated over `patient-example.xml`
- **THEN** the result is `Peter`

#### Scenario: [path.navigation.003] a leading type name that is not the focus type yields empty

- **WHEN** `Observation.value` is evaluated over `patient-example.xml`
- **THEN** the result is an empty collection

#### Scenario: [path.navigation.004] navigation flattens across a repeating element

- **WHEN** `Patient.name.given` is evaluated over `patient-example.xml`
- **THEN** the result is the five strings `Peter`, `James`, `Jim`, `Peter`,
  `James` in document order (`testJoin/testJoin`)

#### Scenario: [path.navigation.005] an absent element yields empty, not an error

- **WHEN** `Patient.name.prefix` is evaluated over `patient-example.xml`, which
  has none
- **THEN** the result is an empty collection (`testJoin/testJoinEmpty`)

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
- **THEN** the first result is `Peter` and every `testIndexer` case yields its
  stated output

#### Scenario: [path.navigation.009] an out-of-range index yields empty

- **WHEN** `Patient.name[99]` is evaluated over `patient-example.xml`
- **THEN** the result is an empty collection

#### Scenario: [path.navigation.010] a negative index yields empty

- **WHEN** `Patient.name[-1]` is evaluated over `patient-example.xml`
- **THEN** the result is an empty collection

#### Scenario: [path.navigation.011] a non-integer index is an execution error

- **WHEN** `Patient.name['x']` is evaluated
- **THEN** evaluation fails

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
- **THEN** the result is an empty collection

#### Scenario: [path.navigation.020] `children()` returns the immediate children

- **WHEN** `Patient.children().count()` is evaluated over `patient-example.xml`
- **THEN** the count equals the number of child elements present on the resource

#### Scenario: [path.navigation.021] `descendants()` returns the transitive children

- **WHEN** `contained.select(('#'+id in %resource.descendants().reference).not()).empty()`
  is evaluated over `patient-example.xml`
- **THEN** the result is `true` (`testDollarResource`, second case)

#### Scenario: [path.navigation.022] `children()` produces an unordered collection

- **WHEN** `Patient.children().skip(1)` is checked in strict mode with ordered
  function checking enabled
- **THEN** the static check fails as a semantic error
  (`testDollar/testDollarOrderNotAllowed`)

#### Scenario: [path.navigation.023] navigation preserves document order across repeating elements

- **WHEN** `Patient.name.skip(1).given` is evaluated over `patient-example.xml`
- **THEN** the result is `Jim`, `Peter`, `James` in that order
  (`testDollar/testDollarOrderAllowed`)

#### Scenario: [path.navigation.024] the miscellaneous accessor cases all hold

- **WHEN** the `testMiscellaneousAccessorTests`, `testBasics` and
  `testObservations` groups' cases are evaluated
- **THEN** every case yields its stated output

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

#### Scenario: [path.collection.011] `supersetOf` compares by equality

- **WHEN** the `testSuperSetOf` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.collection.012] `isDistinct` detects duplicates

- **WHEN** `Patient.name.given.isDistinct()` is evaluated over
  `patient-example.xml`, whose given names repeat
- **THEN** the result is `false`

#### Scenario: [path.collection.013] `distinct` removes duplicates and keeps first-occurrence order

- **WHEN** the `testDistinct` group's cases are evaluated
- **THEN** every case yields its stated output

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

#### Scenario: [path.filter.003] `where` with a non-boolean criteria is an execution error

- **WHEN** `Patient.name.where('x')` is evaluated
- **THEN** evaluation fails

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

- **WHEN** `repeat(item)` is evaluated over `questionnaire-example.xml`, whose
  `item` elements nest
- **THEN** evaluation terminates and returns each nested item once

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

### Requirement: Subsetting

The engine SHALL implement the subsetting functions, and SHALL yield an empty collection rather than an error for an index or count outside the collection.

#### Scenario: [path.subset.001] `single` returns the only item

- **WHEN** the `testSingle` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.subset.002] `single` on a collection of more than one is an execution error

- **WHEN** `Patient.name.single()` is evaluated over `patient-example.xml`,
  which has three names
- **THEN** evaluation fails

#### Scenario: [path.subset.003] `single` on an empty collection yields empty

- **WHEN** `Patient.name.prefix.single()` is evaluated over
  `patient-example.xml`
- **THEN** the result is an empty collection

#### Scenario: [path.subset.004] `first` and `last` select the ends

- **WHEN** the `testFirstLast` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.subset.005] `tail` drops the first item

- **WHEN** the `testTail` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.subset.006] `skip` drops the first n items

- **WHEN** the `testSkip` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.subset.007] `skip` past the end yields empty

- **WHEN** `Patient.name.skip(3).given` is evaluated over `patient-example.xml`
- **THEN** the result is an empty collection
  (`testDollar/testDollarOrderAllowedA`)

#### Scenario: [path.subset.008] `take` keeps the first n items

- **WHEN** the `testTake` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.subset.009] `take(0)` and `take(-1)` yield empty

- **WHEN** `Patient.name.take(0)` and `Patient.name.take(-1)` are evaluated
- **THEN** both yield an empty collection

#### Scenario: [path.subset.010] `intersect` keeps items present in both, without duplicates

- **WHEN** the `testIntersect` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.subset.011] `exclude` removes items present in the argument

- **WHEN** the `testExclude` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.subset.012] `exclude` keeps duplicates that are not excluded

- **WHEN** `(1 | 1 | 2).exclude(2).count()` is evaluated
- **THEN** the result is the integer `2`

### Requirement: Combining

The engine SHALL implement the union operator, `union`, `combine`, `in` and `contains` with the duplicate handling `FHIRPathEngine` applies.

#### Scenario: [path.combine.001] the union operator unions and removes duplicates

- **WHEN** the `testUnion` group's cases are evaluated
- **THEN** every case yields its stated output

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

#### Scenario: [path.combine.006] `in` with an empty left operand yields empty

- **WHEN** `{} in (1 | 2)` is evaluated
- **THEN** the result is an empty collection

### Requirement: Equality and equivalence

The engine SHALL implement equality, inequality, equivalence and non-equivalence as `FHIRPathEngine` does, including the cases where equality yields an empty collection and equivalence yields a boolean.

#### Scenario: [path.equality.001] the equality operator compares values of the same type

- **WHEN** the `testEquality` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.equality.002] the inequality operator is the negation of equality

- **WHEN** the `testNEquality` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.equality.003] equality with an empty operand yields empty

- **WHEN** `{} = 1` and `1 = {}` are evaluated
- **THEN** both yield an empty collection

#### Scenario: [path.equality.004] the equivalence operator ignores case and whitespace for strings

- **WHEN** the `testEquivalent` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.equality.005] the non-equivalence operator is the negation of equivalence

- **WHEN** the `testNotEquivalent` group's cases are evaluated
- **THEN** every case yields its stated output

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
- **THEN** every case yields its stated output

### Requirement: Ordered comparison

The engine SHALL implement the four ordered comparison operators as `FHIRPathEngine` does, yielding an empty collection where the comparison is undecidable and failing where the operands are not comparable.

#### Scenario: [path.compare.001] the less-than operator compares integers, decimals, strings, dates and quantities

- **WHEN** the `testLessThan` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.compare.002] the less-or-equal operator compares the same types

- **WHEN** the `testLessOrEqual` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.compare.003] the greater-than operator compares the same types

- **WHEN** the `testGreaterThan` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.compare.004] the greater-or-equal operator compares the same types

- **WHEN** the `testGreatorOrEqual` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.compare.005] comparing different types is an execution error

- **WHEN** `1 < 'a'` is evaluated
- **THEN** evaluation fails

#### Scenario: [path.compare.006] comparison with an empty operand yields empty

- **WHEN** `{} < 1` is evaluated
- **THEN** the result is an empty collection

#### Scenario: [path.compare.007] comparison of a collection of more than one is an execution error

- **WHEN** `(1 | 2) < 3` is evaluated
- **THEN** evaluation fails

#### Scenario: [path.compare.008] strings compare by code point

- **WHEN** `'a' < 'b'` and `'A' < 'a'` are evaluated
- **THEN** both yield `true`

#### Scenario: [path.compare.009] dates of insufficient precision compare to empty

- **WHEN** `@2014 < @2014-01-05` is evaluated
- **THEN** the result is an empty collection

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

- **WHEN** `false and (1 'cm' < 1 's')` is evaluated, whose right operand alone
  would fail
- **THEN** the result is `false` and no error is raised

#### Scenario: [path.logic.007] `or` short-circuits on a true left operand

- **WHEN** `true or (1 'cm' < 1 's')` is evaluated
- **THEN** the result is `true` and no error is raised

#### Scenario: [path.logic.008] `iif` selects a branch and does not evaluate the other

- **WHEN** the `testIif` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.logic.009] `iif` with an empty condition yields the else branch

- **WHEN** `iif({}, 1, 2)` is evaluated
- **THEN** the result is the integer `2`

#### Scenario: [path.logic.010] `iif` with no else branch and a false condition yields empty

- **WHEN** `iif(false, 1)` is evaluated
- **THEN** the result is an empty collection

### Requirement: Arithmetic

The engine SHALL implement the arithmetic operators over integers, decimals, strings, quantities, dates and dateTimes as `FHIRPathEngine` does, using decimal arithmetic rather than binary floating point.

#### Scenario: [path.math.001] addition works on integers, decimals, strings and quantities

- **WHEN** the `testPlus` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.math.002] the concatenation operator joins strings and treats empty as the empty string

- **WHEN** the `testConcatenate` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.math.003] subtraction works on integers, decimals and quantities

- **WHEN** the `testMinus` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.math.004] multiplication works on integers, decimals and quantities

- **WHEN** the `testMultiply` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.math.005] division always yields a decimal

- **WHEN** the `testDivide` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.math.006] `div` truncates toward zero

- **WHEN** the `testDiv` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.math.007] `mod` returns the remainder

- **WHEN** the `testMod` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.math.008] division by zero yields empty

- **WHEN** `1 / 0`, `1 div 0` and `1 mod 0` are evaluated
- **THEN** each yields an empty collection

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

#### Scenario: [path.math.013] date arithmetic preserves the operand's precision

- **WHEN** `@2014 + 1 year` is evaluated
- **THEN** the result is a year-precision `date`

#### Scenario: [path.math.014] subtracting two dateTimes is not addition of a duration

- **WHEN** `@2014-01-02 - @2014-01-01` is evaluated
- **THEN** the engine behaves as it does at the pinned SHA, and the harness
  records whether that is a quantity or an error

### Requirement: Math functions

The engine SHALL implement the math functions `FHIRPathEngine` implements, each over a singleton numeric focus, yielding an empty collection for an empty focus.

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

#### Scenario: [path.mathfn.010] a math function on an empty collection yields empty

- **WHEN** `{}.abs()`, `{}.sqrt()` and `{}.round(2)` are evaluated
- **THEN** each yields an empty collection

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

#### Scenario: [path.string.003] `indexOf` returns a zero-based position or -1

- **WHEN** the `testIndexOf` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.string.004] `substring` takes a start and an optional length

- **WHEN** the `testSubstring` group's cases are evaluated
- **THEN** every case yields its stated output

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

- **WHEN** the `testMatches` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.string.009] `matchesFull` requires the whole string to match

- **WHEN** `'abc'.matches('b')` and `'abc'.matchesFull('b')` are evaluated
- **THEN** the results are `true` and `false`

#### Scenario: [path.string.010] `replaceMatches` substitutes regular-expression matches

- **WHEN** the `testReplaceMatches` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.string.011] `replace` substitutes literal substrings

- **WHEN** the `testReplace` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.string.012] `length` counts characters

- **WHEN** the `testLength` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.string.013] `trim` removes leading and trailing whitespace

- **WHEN** the `testTrim` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.string.014] `split` keeps empty segments

- **WHEN** the `testSplit` group's cases are evaluated, including
  `'A,,C'.split(',').join(',') = 'A,,C'` and the nine-segment `[stop]` case
- **THEN** every case yields its stated output

#### Scenario: [path.string.015] `join` concatenates with a separator and yields empty for an empty collection

- **WHEN** the `testJoin` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.string.016] `encode` supports base64, urlbase64 and hex

- **WHEN** the `testEncodeDecode` group's `encode` cases are evaluated,
  including `'subjects?_d'.encode('urlbase64')` yielding `c3ViamVjdHM_X2Q=`
- **THEN** every case yields its stated output

#### Scenario: [path.string.017] `decode` reverses `encode` for each supported form

- **WHEN** the `testEncodeDecode` group's `decode` cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.string.018] `encode` and `decode` with an unsupported format yield empty or fail as the pinned engine does

- **WHEN** `'x'.encode('rot13')` is evaluated
- **THEN** the engine behaves as it does at the pinned SHA, and the harness
  records which behaviour that is

#### Scenario: [path.string.019] `escape` and `unescape` support `html` and `json`

- **WHEN** the `testEscapeUnescape` group's cases are evaluated, including
  `unescape('json')` turning a unicode escape into its character
- **THEN** every case yields its stated output

#### Scenario: [path.string.020] a string function on a non-string is an execution error

- **WHEN** `1.startsWith('1')` is evaluated
- **THEN** evaluation fails

#### Scenario: [path.string.021] a string function on an empty collection yields empty

- **WHEN** `{}.upper()`, `{}.length()` and `{}.substring(0)` are evaluated
- **THEN** each yields an empty collection

#### Scenario: [path.string.022] string functions operate on unicode scalar values, not bytes

- **WHEN** a string containing non-ASCII characters is measured with `length()`
  and sliced with `substring()`
- **THEN** both count characters as the engine at the pinned SHA counts them,
  and never split a character

### Requirement: Type conversion

The engine SHALL implement each `toX` function and its matching `convertsToX` predicate, so that a failed conversion yields an empty collection while the predicate yields `false`.

#### Scenario: [path.convert.001] `toInteger` converts strings, booleans and integers

- **WHEN** the `testToInteger` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.convert.002] `toDecimal` converts strings, booleans, integers and decimals

- **WHEN** the `testToDecimal` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.convert.003] `toString` converts every convertible type

- **WHEN** the `testToString` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.convert.004] `convertsToDate` accepts year, month and day precision strings

- **WHEN** `'2015'.convertsToDate()`, `'2015-02'.convertsToDate()` and
  `'2015-02-04'.convertsToDate()` are evaluated
- **THEN** each yields `true` (`testTypes` group)

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
- **THEN** every case yields its stated output

#### Scenario: [path.convert.009] a failed conversion yields empty while its `convertsTo` yields false

- **WHEN** `'abc'.toInteger()` and `'abc'.convertsToInteger()` are evaluated
- **THEN** the results are an empty collection and `false`

#### Scenario: [path.convert.010] a conversion function on a collection of more than one is an execution error

- **WHEN** `(1 | 2).toString()` is evaluated
- **THEN** evaluation fails

#### Scenario: [path.convert.011] `toDateTime` on a date widens precision without inventing components

- **WHEN** `@2014.toDateTime()` is evaluated
- **THEN** the result is a year-precision `dateTime`

### Requirement: Date and time functions

The engine SHALL implement `today`, `now` and `precision`, and SHALL take the evaluation time zone from its configuration rather than from the host machine.

#### Scenario: [path.datetime.001] `today()` returns the current date

- **WHEN** the `testToday` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.datetime.002] `now()` returns the current dateTime

- **WHEN** the `testNow` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.datetime.003] `now()` is stable within a single evaluation

- **WHEN** `now() = now()` is evaluated
- **THEN** the result is `true`

#### Scenario: [path.datetime.004] the evaluation time zone is a parameter of the harness, not of the machine

- **WHEN** the oracle harness runs
- **THEN** it fixes the time zone to `UTC+1100`, as
  `FHIRPathTests.test` does with `TimeZone.setDefault`
- **AND** the suite's result does not change with the machine's local time zone

#### Scenario: [path.datetime.005] `precision()` reports the component count of a date, dateTime or time

- **WHEN** the `Precision` group's cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.datetime.006] a dateTime with an offset and one in UTC compare correctly

- **WHEN** `@2014-01-05T10:30:00+10:00 = @2014-01-05T00:30:00Z` is evaluated
- **THEN** the result is the boolean the engine at the pinned SHA computes

### Requirement: Quantities and UCUM

The engine SHALL compare, add, subtract, multiply and divide quantities by converting to UCUM canonical form from the pinned `ucum-essence.xml`, and SHALL implement `comparable`.

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

#### Scenario: [path.quantity.009] two unitless quantities compare equal and equivalent

- **WHEN** `Parameters.parameter[0].value = Parameters.parameter[1].value` and
  the same with the equivalence operator are evaluated over
  `parameters-example-quantities.xml`
- **THEN** both yield `true`
  (`testQuantityUnitComparison/testQuantityNoUnitEqual`,
  `testQuantityNoUnitEquivalent`)

#### Scenario: [path.quantity.010] two quantities with the same unit text and no code compare equal

- **WHEN** `Parameters.parameter[2].value = Parameters.parameter[3].value` and
  the equivalence form are evaluated over `parameters-example-quantities.xml`
- **THEN** both yield `true`
  (`testQuantityUnitComparison/testQuantityUnitTextEqual`,
  `testQuantityUnitTextEquivalent`)

#### Scenario: [path.quantity.011] a quantity with a unit compared to one without yields empty for equality

- **WHEN** `Parameters.parameter[2].value = Parameters.parameter[0].value` and
  the reverse are evaluated over `parameters-example-quantities.xml`
- **THEN** both yield an empty collection
  (`testQuantityUnitComparison/testQuantityUnitVsNoUnitEqual`,
  `testQuantityNoUnitVsUnitEqual`)

#### Scenario: [path.quantity.012] a quantity with a unit compared to one without yields false for equivalence

- **WHEN** the same two expressions with the equivalence operator are evaluated
- **THEN** both yield `false`
  (`testQuantityUnitComparison/testQuantityUnitVsNoUnitEquivalent`,
  `testQuantityNoUnitVsUnitEquivalent`)

#### Scenario: [path.quantity.013] unit text without a code is handled per the suite's dedicated input

- **WHEN** the three cases using `parameters-quantity-unit-text.xml` are
  evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.quantity.014] `comparable()` reports whether two quantities are commensurable

- **WHEN** `1 'cm'.comparable(1 '[in_i]')`, `1 'cm'.comparable(1 '[s]')` and
  `1 'cm'.comparable(1 's')` are evaluated
- **THEN** the results are `true`, `false`, `false` (`Comparable/Comparable1`,
  `Comparable2`, `Comparable3`)

#### Scenario: [path.quantity.015] comparing incommensurable units is an execution error

- **WHEN** `1 'cm' < 1 's'` is evaluated
- **THEN** evaluation fails

#### Scenario: [path.quantity.016] UCUM data comes from the pinned `ucum-essence.xml`

- **WHEN** the engine canonicalises any unit
- **THEN** it does so from the `ucum-essence.xml` the repository pins, and the
  file's version is reported with the suite result
- **AND** an absent or unparseable UCUM file fails the run rather than silently
  treating units as incomparable

#### Scenario: [path.quantity.017] a quantity with an unknown unit code does not crash

- **WHEN** `1 'zzz' = 1 'zzz'` and `1 'zzz' = 1 'cm'` are evaluated
- **THEN** the engine behaves as it does at the pinned SHA, and the harness
  records which behaviour that is

### Requirement: Boundary functions

The engine SHALL implement `lowBoundary`, `highBoundary` and `precision` over decimals, quantities, dates, dateTimes and times as `FHIRPathEngine` does, including the carry and borrow cases.

#### Scenario: [path.boundary.001] `lowBoundary` on a decimal widens to the implied precision

- **WHEN** the `LowBoundary` group's decimal cases are evaluated, including
  `1.587.lowBoundary()` yielding `1.58650000`
- **THEN** every case yields its stated output

#### Scenario: [path.boundary.002] `lowBoundary(n)` with an out-of-range precision yields empty

- **WHEN** `1.587.lowBoundary(-1)`, `1.587.lowBoundary(32)` and
  `1.587.lowBoundary(39)` are evaluated
- **THEN** each yields an empty collection (`LowBoundary/LowBoundaryDecimal3`,
  `LowBoundaryDecimal5`, `LowBoundaryDecimal6`)

#### Scenario: [path.boundary.003] `lowBoundary` on a negative decimal moves away from zero

- **WHEN** `(-1.587).lowBoundary()` and `(-1.587).lowBoundary(2)` are evaluated
- **THEN** the results are `-1.58750000` and `-1.59`
  (`LowBoundary/LowBoundaryNegDecimalDefault`, `LowBoundaryNegDecimal2`)

#### Scenario: [path.boundary.004] `lowBoundary` carries and borrows across digit positions

- **WHEN** `(-9.96).lowBoundary(1)`, `(-9).lowBoundary(0)`,
  `10.lowBoundary(0)` and `100.lowBoundary(2)` are evaluated
- **THEN** the results are `-10.0`, `-10`, `9` and `99.50`
  (`LowBoundary/LowBoundaryDecimalCarry1`, `LowBoundaryDecimalCarry3`,
  `LowBoundaryDecimalBorrow1`, `LowBoundaryDecimalBorrow2`)

#### Scenario: [path.boundary.005] `lowBoundary` reads precision from a decimal's stored lexical form

- **WHEN** `Parameters.parameter[0].value.lowBoundary(0)` and
  `Parameters.parameter[1].value.lowBoundary()` are evaluated over
  `parameters-example-decimals.xml`
- **THEN** the results are `945` and `0.00145000`
  (`LowBoundary/LowBoundaryDecimalExponent1`, `LowBoundaryDecimalExponent2`)

#### Scenario: [path.boundary.006] `highBoundary` on a decimal widens to the implied precision

- **WHEN** the `HighBoundary` group's decimal cases are evaluated, including
  `1.587.highBoundary()` yielding `1.58750000`
- **THEN** every case yields its stated output

#### Scenario: [path.boundary.007] `highBoundary` carries across digit positions

- **WHEN** `9.95.highBoundary(1)`, `0.996.highBoundary(1)`,
  `99.96.highBoundary(1)` and `9.highBoundary(0)` are evaluated
- **THEN** the results are `10.0`, `1.0`, `100.0` and `10`
  (`HighBoundary/HighBoundaryDecimalCarry1` through `Carry4`)

#### Scenario: [path.boundary.008] boundary functions work on quantities

- **WHEN** the `LowBoundary` and `HighBoundary` groups' quantity cases are
  evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.boundary.009] boundary functions work on dates, dateTimes and times

- **WHEN** the `LowBoundary` and `HighBoundary` groups' date, dateTime and time
  cases are evaluated
- **THEN** every case yields its stated output

#### Scenario: [path.boundary.010] an integer's boundaries are decimals half a unit away

- **WHEN** `1.lowBoundary(5)` and `1.highBoundary()` are evaluated
- **THEN** the results are `0.50000` and `1.50000000`
  (`LowBoundary/LowBoundaryDecimal9`, `HighBoundary/HighBoundaryDecimal8`)

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

#### Scenario: [path.types.011] `is` with an unknown type name is a semantic error

- **WHEN** `Patient.gender is NoSuchType` is checked
- **THEN** the static check fails as a semantic error

#### Scenario: [path.types.012] `is` on an empty collection yields empty

- **WHEN** `{} is Boolean` is evaluated
- **THEN** the result is an empty collection

#### Scenario: [path.types.013] the whole `testTypes`, `testType` and `testInheritance` surface holds

- **WHEN** the `testTypes`, `testType` and `testInheritance` groups' cases are
  evaluated
- **THEN** every case yields its stated output

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

- **WHEN** `reference.startsWith('#').not() or (reference.substring(1) in %resource.contained.id)`
  is evaluated with a `Reference` inside `patient-example.xml` as the focus
- **THEN** the result is `true` (`testDollarResource`, first case)

#### Scenario: [path.env.007] `%resource` with no focus resource is an execution error

- **WHEN** `%resource` is evaluated with no focus resource
- **THEN** evaluation fails, naming `%resource`

#### Scenario: [path.env.008] `%rootResource` is the outermost resource

- **WHEN** `%rootResource` is evaluated with a contained resource as the focus
- **THEN** the result is the enclosing resource, not the contained one

#### Scenario: [path.env.009] `%rootResource` with no root resource is an execution error

- **WHEN** `%rootResource` is evaluated with no root resource
- **THEN** evaluation fails, naming `%rootResource`

#### Scenario: [path.env.010] `%context` is the node evaluation started from

- **WHEN** `%context` is evaluated with a `Patient.name` as the starting context
- **THEN** the result is that `HumanName`

#### Scenario: [path.env.011] an unknown environment variable with no host service is an execution error

- **WHEN** `%nosuchvariable` is evaluated with no host service configured
- **THEN** evaluation fails, naming the constant

#### Scenario: [path.env.012] a host service may supply an environment variable, including a collection

- **WHEN** a host service that returns two strings for a named constant is
  configured and that constant is evaluated
- **THEN** the result is both strings in order, which is
  `FHIRPathTests.resolveConstantReturnsList`'s assertion

#### Scenario: [path.env.013] `defineVariable` introduces a variable for the rest of the expression

- **WHEN** an expression that calls `defineVariable('v', ...)` and then
  references the variable is evaluated
- **THEN** the variable resolves to the defined value

#### Scenario: [path.env.014] a variable defined by `defineVariable` shadows nothing and does not leak

- **WHEN** a variable defined inside one argument of an expression is
  referenced outside that argument's scope
- **THEN** evaluation fails, naming the constant

### Requirement: Utility and FHIR-specific functions

The engine SHALL implement the utility and FHIR-specific functions `FHIRPathEngine` implements, delegating `resolve`, `conformsTo` and `memberOf` to the host service.

#### Scenario: [path.utility.001] `trace` passes its input through and records a log line

- **WHEN** the `testTrace` group's cases are evaluated
- **THEN** every case yields its stated output
- **AND** the trace log is retrievable after evaluation and contains the names
  the expressions passed to `trace`

#### Scenario: [path.utility.002] `trace` with a projection logs the projection, not the input

- **WHEN** `'[stop]ONE[stop]'.split('[stop]').trace('n').count()` is evaluated
- **THEN** the count is as the suite states and the log is recorded under `n`
  (`testSplit/testSplit3`)

#### Scenario: [path.utility.003] `check` passes its input through when the condition holds

- **WHEN** an expression using `check()` with a satisfied condition is evaluated
- **THEN** the input is returned unchanged

#### Scenario: [path.utility.004] `check` fails when the condition does not hold

- **WHEN** an expression using `check()` with an unsatisfied condition is
  evaluated
- **THEN** evaluation fails

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

#### Scenario: [path.utility.010] `extension(url)` with no match yields empty

- **WHEN** `Patient.extension('http://example.org/nothing')` is evaluated over
  `patient-example.xml`
- **THEN** the result is an empty collection

#### Scenario: [path.utility.011] `extension` reaches a primitive's extensions

- **WHEN** `Patient.birthDate.extension(url)` is evaluated for an extension
  carried in `_birthDate`
- **THEN** the extension is returned

#### Scenario: [path.utility.012] `conformsTo` asks the host service

- **WHEN** `conformsTo('http://hl7.org/fhir/StructureDefinition/Patient')` and
  `conformsTo('http://hl7.org/fhir/StructureDefinition/Person')` are evaluated
  over `patient-example.xml` with the suite's host service
- **THEN** the results are `true` and `false`
  (`testConformsTo/testConformsTo1`, `testConformsTo2`)

#### Scenario: [path.utility.013] `conformsTo` with a profile the host service does not know is an execution error

- **WHEN** `conformsTo('http://trash')` is evaluated
- **THEN** evaluation fails (`testConformsTo/testConformsTo3`)

#### Scenario: [path.utility.014] `conformsTo` with no host service returns false, as `FhirPathR4` does

- **WHEN** `conformsTo(...)` is evaluated through the server-facing
  configuration, whose `conformsToProfile` returns `false` unconditionally
- **THEN** the result is `false` for every URL

#### Scenario: [path.utility.015] `memberOf` asks the host service for the value set

- **WHEN** `memberOf(...)` is evaluated with a host service that resolves the
  value set
- **THEN** the result is the membership answer the host service implies

#### Scenario: [path.utility.016] `memberOf` with no resolvable value set yields empty

- **WHEN** `memberOf(...)` is evaluated with a host service that returns no
  value set
- **THEN** the result is an empty collection, not an error

#### Scenario: [path.utility.017] `resolve()` resolves a local `#id` reference into `contained`

- **WHEN** `resolve()` is evaluated on a `Reference` whose value is `#id` and
  whose enclosing resource contains that id
- **THEN** the contained resource is returned

#### Scenario: [path.utility.018] `resolve()` resolves a Bundle-internal reference by `fullUrl`

- **WHEN** `resolve()` is evaluated on a `Reference` inside a `Bundle` whose
  entry `fullUrl` matches the reference
- **THEN** that entry's resource is returned

#### Scenario: [path.utility.019] `resolve()` on an unresolvable reference yields empty

- **WHEN** `resolve()` is evaluated on an absolute reference with no host
  service configured
- **THEN** the result is an empty collection

#### Scenario: [path.utility.020] `resolve()` delegates an external reference to the host service

- **WHEN** `resolve()` is evaluated on `Patient/123` with a host service
  configured
- **THEN** the host service is asked for `Patient/123` and its answer is the
  result

#### Scenario: [path.utility.021] the appointment and explanation-of-benefit cases hold

- **WHEN** the cases whose `inputfile` is `appointment-examplereq.json`,
  `explanationofbenefit-example.json`, `codesystem-example.xml` or
  `valueset-example-expansion.xml` are evaluated
- **THEN** every case yields its stated output

### Requirement: The static type-check pass

The engine SHALL provide a check pass, separate from evaluation, that reports
the semantic errors `FHIRPathEngine.check` reports — 29 suite cases are
`invalid="semantic"`, and the `isSemanticChecks` group exists only to exercise
it.

#### Scenario: [path.check.001] the check pass is separate from evaluation

- **WHEN** an expression is parsed, checked against a focus type, and then
  evaluated
- **THEN** the check reports its result without evaluating the expression
- **AND** an expression that fails the check is not evaluated

#### Scenario: [path.check.002] the 29 semantic-invalid oracle cases all fail the check

- **WHEN** each of the suite's 29 `invalid="semantic"` cases is checked
- **THEN** each one fails the check
- **AND** the check reports which element path or type caused the failure

#### Scenario: [path.check.003] navigating a non-existent element is a semantic error

- **WHEN** `Patient.nosuchelement` is checked against focus type `Patient`
- **THEN** the check fails, naming `nosuchelement`

#### Scenario: [path.check.004] the check infers the type of a choice element as the declared set

- **WHEN** `Observation.value` is checked against focus type `Observation`
- **THEN** the inferred type is the declared choice set, not a single type

#### Scenario: [path.check.005] the check narrows the type after an `is` guard in `implies`

- **WHEN** `$this is dateTime implies $this.toString().length() > 10` is checked
  with `Observation.effective` as the context and `Period` as the context type
- **THEN** the check passes, because the guarded operand is checked assuming the
  guard is true (`isSemanticChecks/isEffectOnLogic3`)

#### Scenario: [path.check.006] the same expression without the guard fails the check

- **WHEN** `$this.toString().length() > 10` is checked with
  `Observation.effective` as the context and `Period` as the context type
- **THEN** the check fails as a semantic error
  (`isSemanticChecks/isEffectOnLogic3Base`)

#### Scenario: [path.check.007] the `is()` function narrows the type as the `is` operator does

- **WHEN** `$this.is(dateTime) implies $this.toString().length() >= 10` is
  checked with `Observation.effective` as the context and `Period` as the
  context type
- **THEN** the check passes (`isSemanticChecks/isGuardFnImplies`)

#### Scenario: [path.check.008] `and` narrows the right operand assuming the left is true

- **WHEN** the `isSemanticChecks` group's `and` guard cases are checked
- **THEN** each yields its stated result

#### Scenario: [path.check.009] `or` narrows the right operand assuming the left is false

- **WHEN** the `isSemanticChecks` group's `or` guard cases are checked
- **THEN** each yields its stated result

#### Scenario: [path.check.010] `iif` narrows its branches by the condition's polarity

- **WHEN** the `isSemanticChecks` group's `iif` guard cases are checked
- **THEN** each yields its stated result

#### Scenario: [path.check.011] a guard that narrows to the empty type set makes the guarded operand unreachable, not an error

- **WHEN** a guard narrows the focus type set to empty
- **THEN** the guarded operand is not reported as an error
  (`isSemanticChecks`, per the group's own note)

#### Scenario: [path.check.012] a context type naming several types checks against all of them

- **WHEN** a case whose `contextType` is `Period|Timing` is checked
- **THEN** the check considers the focus to be either type

#### Scenario: [path.check.013] the check pass can be skipped

- **WHEN** an expression is evaluated with the check pass disabled
- **THEN** evaluation proceeds, as `FhirPathR4` does, which never calls `check`

#### Scenario: [path.check.014] the harness records which cases the check pass is exercised for

- **WHEN** the oracle harness runs
- **THEN** its report distinguishes a case that failed the check from one that
  failed at evaluation, so a `syntax`, `semantic` and `execution` expectation
  cannot be satisfied by the wrong kind of failure

### Requirement: Strictness configuration

The engine SHALL expose the strictness switches `FHIRPathEngine` exposes, and SHALL default to the values `FhirPathR4` sets, because that is the configuration the server runs.

HAPI configures the same engine two ways, and both are observable. Neither is a
deviation from HAPI, so neither needs a `DEV-nnnn` entry: both are HAPI.

#### Scenario: [path.strictness.001] the default configuration matches `FhirPathR4`

- **WHEN** an engine is created with no explicit configuration
- **THEN** the as-singleton rule is not enforced and `as` is not case-sensitive,
  which is what `FhirPathR4`'s constructor sets

#### Scenario: [path.strictness.002] the oracle harness uses the strict configuration

- **WHEN** the oracle harness creates its engine
- **THEN** the as-singleton rule is enforced and `as` is case-sensitive, which is
  what `FHIRPathTests.test` sets before evaluating

#### Scenario: [path.strictness.003] the as-singleton rule rejects a multi-item focus when enforced

- **WHEN** `as` is applied to a collection of more than one item with the rule
  enforced
- **THEN** evaluation fails

#### Scenario: [path.strictness.004] the as-singleton rule allows a multi-item focus when not enforced

- **WHEN** the same expression is evaluated with the rule not enforced
- **THEN** evaluation succeeds and each item is tested

#### Scenario: [path.strictness.005] case-sensitive `as` rejects a mis-cased type name when enforced

- **WHEN** `Patient.gender.as(Code)` is evaluated with case sensitivity enforced
- **THEN** the result differs from the correctly cased form, and the harness
  records what the pinned engine returns for each

#### Scenario: [path.strictness.006] lenient polymorphic names are not supported in R4

- **WHEN** `Observation.valueQuantity.exists()` is checked with lenient
  polymorphic names requested
- **THEN** the behaviour is the strict one, because the suite's own note records
  that lenient polymorphic mode is not supported in R4 and the R4 engine's
  `allowPolymorphicNames` is left unset by both HAPI wrappers

#### Scenario: [path.strictness.007] legacy mode is not enabled for R4

- **WHEN** an R4 engine is created by either configuration
- **THEN** legacy mode is off, so the inequality operator is not valid for empty
  sets in the way DSTU2 and DSTU3 constraints assume

### Requirement: Execution errors

The engine SHALL report an evaluation failure as an error naming the sub-expression, and SHALL never panic or substitute an empty collection for a failure.

#### Scenario: [path.errors.001] the 27 execution-invalid oracle cases all fail at evaluation

- **WHEN** each of the suite's 27 `invalid="execution"` cases is parsed, checked
  and evaluated
- **THEN** parsing and checking succeed and evaluation fails for each

#### Scenario: [path.errors.002] an error names the expression and the position

- **WHEN** any execution error is reported
- **THEN** the error names the sub-expression and its offset within the
  expression

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
- **THEN** an error naming both types is returned, as `FhirPathR4` does

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
- **THEN** it reports 104 groups and 1,193 cases
- **AND** a count that differs fails the run, because the suite has been
  re-pinned without the harness being updated

#### Scenario: [path.oracle.002] the harness reads every attribute the case carries

- **WHEN** the harness parses a case
- **THEN** it records `name`, `description`, `inputfile`, `expression`, the
  expression's `invalid` value, `predicate`, `ordered`, `mode`, `context`,
  `contextType`, `version`, `testing` and `checkOrderedFunctions`
- **AND** an attribute it does not understand fails the run rather than being
  ignored

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

- **WHEN** a case carries `ordered="false"`
- **THEN** each outcome item is matched against any expected output rather than
  against the one at its index

#### Scenario: [path.oracle.008] the harness compares count, type and value

- **WHEN** a case's outputs are compared
- **THEN** the number of outputs must match, each output's `type` attribute must
  equal the result item's FHIR type name, and each output's text must equal the
  result item's FHIRPath string value

#### Scenario: [path.oracle.009] the harness compares a Quantity output by parsing it

- **WHEN** an expected output's type is `Quantity`
- **THEN** the expected text is parsed as a quantity and compared deeply, as
  `FHIRPathTests` does with `parseQuantityString` and `equalsDeep`

#### Scenario: [path.oracle.010] the harness reports per-case status with four distinct values

- **WHEN** the harness finishes
- **THEN** every case has one of `passed`, `failed`, `did-not-run` or
  `skipped-for-licence`, and `did-not-run` is never counted as a pass

#### Scenario: [path.oracle.011] the harness records HAPI's result for the same case

- **WHEN** the harness report is written
- **THEN** each case carries HAPI 8.12.0's measured status alongside Frevo's, so
  the gate can be evaluated per case rather than only in aggregate

#### Scenario: [path.oracle.012] the gate is pass rate at or above HAPI's on the same cases

- **WHEN** the gate is evaluated
- **THEN** it passes only if Frevo's `passed` count is at least HAPI's
- **AND** a case HAPI passes that Frevo fails is listed individually in the gate
  report, regardless of the aggregate

#### Scenario: [path.oracle.013] the harness can run the same cases through `helios-fhirpath`

- **WHEN** the harness is asked for the differential reference
- **THEN** it runs the same cases through `helios-fhirpath` 0.2.7 and reports a
  three-way comparison
- **AND** `helios-fhirpath` appears in no shipped crate's dependency graph
  (ADR 0004 decision 3)

#### Scenario: [path.oracle.014] the `documentFunctions` group is reported as out of scope, not as passing

- **WHEN** the harness runs the 114 cases in the `mode="custom"`
  `documentFunctions` group
- **THEN** each is reported `failed` with the reason that the R4 engine
  implements none of `section`, `entry`, `entryIn`, `resources`, `resourcesIn`,
  `sameSubject` or `hasCoding`
- **AND** HAPI's recorded status for the same cases is `failed` too, so the
  aggregate gate is unaffected

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

- **WHEN** the oracle harness runs all 1,193 cases
- **THEN** the run completes in at most 10 s on the reference environment
  described in `bench/targets.md`

#### Scenario: [path.perf.006] no regression against the recorded baseline

- **WHEN** any `bench/micro/` path workload is compared with the baseline
  recorded for the previous commit on `main`
- **THEN** no workload is more than 10% slower, per the regression policy in
  `bench/targets.md`
