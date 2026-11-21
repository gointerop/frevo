# Provenance

Every crate that contains logic derived from an upstream Apache-2.0 project is
listed here with the upstream module and the pinned commit it was derived from.
Pins are defined in [`reference/PINS.md`](../reference/PINS.md).

Apache-2.0 section 4 obligations this file helps discharge:

- derived files carry a notice of modification,
- upstream copyright and attribution notices are retained (see [`NOTICE`](../NOTICE)),
- the origin of each derived work is identifiable.

## Required header on derived files

Any Rust file whose logic was derived from upstream Java starts with:

```rust
// Derived from <upstream repo>/<path> at <commit SHA>.
// Original work Copyright <upstream copyright holder>, licensed under the
// Apache License, Version 2.0.
// Modified by the Frevo authors: <one line on what changed>.
```

Generated code instead carries an `@generated` marker and names the generator
and the input StructureDefinition package; generated model code is **not** a
translation of upstream Java and is not listed as derived.

## Crate provenance

Status values: `generated` (produced by `frevo-codegen` from specification
artifacts), `derived` (logic ported from upstream source), `independent` (written
against the specification and the oracles, not from upstream source), `empty`
(Phase 0 placeholder).

| Crate | Status | Upstream repo | Upstream module(s) | Pinned SHA |
|---|---|---|---|---|
| `frevo-codegen` | empty | — | — | — |
| `frevo-model` | generated (`src/r4/`); derived (`src/codec/{decode,built}.rs`) | `hapi-fhir` | `hapi-fhir-base/.../parser/{ParserState,JsonParser,LenientErrorHandler,StrictErrorHandler,ParseLocation,IParserErrorHandler,BaseParser}.java`, `hapi-fhir-base/.../util/{FhirTerser,UrlUtil}.java`, `hapi-fhir-base/.../context/FhirContext.java` | `7ab4c7c7c49281496faa1b3c2498a652457669ae` |
| `frevo-path` | derived | `org.hl7.fhir.core` | `org.hl7.fhir.r4/.../r4/fhirpath/{FHIRLexer,ExpressionNode,FHIRPathEngine,FHIRPathConstant,TypeDetails}.java`, `org.hl7.fhir.r4/.../r4/model/{IdType,Base,BaseDateTimeType,TimeType}.java`, `org.hl7.fhir.r4/.../r4/terminologies/ValueSetCheckerSimple.java`, `org.hl7.fhir.utilities/.../{Utilities,SourceLocation}.java`, `org.hl7.fhir.utilities/.../exceptions/PathEngineException.java`, `org.hl7.fhir.r4/.../r4/model/{Narrative,XhtmlType}.java`, `org.hl7.fhir.utilities/.../xhtml/{XhtmlParser,XhtmlComposer,XhtmlNode}.java`; `hapi-fhir` `hapi-fhir-base/.../model/primitive/XhtmlDt.java` at `7ab4c7c7c49281496faa1b3c2498a652457669ae`; `Ucum-java` `src/main/java/org/fhir/ucum/{Decimal,Utilities,Lexer,ExpressionParser,Converter,Canonical,ExpressionComposer,UcumEssenceService,UcumModel}.java`, `.../ucum/definitions/XmlDefinitionsParser.java`, `.../ucum/utils/XmlUtils.java`, `.../ucum/special/{Registry,HoldingHandler,CelsiusHandler,FahrenheitHandler}.java` at `db9d22ade3f0f4a667b45817b85a9ac4534e4224` (BSD-3-Clause) | `73c432b5f43ddbce979978a760ec4aa81e929094` |
| `frevo-convert` | empty | — | — | — |
| `frevo-package` | empty | — | — | — |
| `frevo-validator` | empty | — | — | — |
| `frevo-terminology` | empty | — | — | — |
| `frevo-store` | empty | — | — | — |
| `frevo-search` | empty | — | — | — |
| `frevo-jobs` | empty | — | — | — |
| `frevo-subscription` | empty | — | — | — |
| `frevo-mdm` | empty | — | — | — |
| `frevo-cql` | empty | — | — | — |
| `frevo-cr` | empty | — | — | — |
| `frevo-ips` | empty | — | — | — |
| `frevo-graphql` | empty | — | — | — |
| `frevo-hfql` | empty | — | — | — |
| `frevo-narrative` | empty | — | — | — |
| `frevo-openapi` | empty | — | — | — |
| `frevo-mcp` | empty | — | — | — |
| `frevo-config` | empty | — | — | — |
| `frevo-server` | independent | — | — | — |

`frevo-path`'s front end — `src/syntax/{lexer,literal,parser,ast,error,source}.rs`
— is the first derived code in the repository, and its evaluator core —
`src/eval/{mod,navigate}.rs`, from `FHIRPathEngine.java`'s `execute`,
`getChildrenByName`, `filterIdType`, `funcItem`, `funcChildren` and
`funcDescendants` and from `IdType.java`'s `setValue` — the second. Its
collection predicates and subsetting — `src/eval/{collection,equality,filter}.rs`
(`funcWhere`, `funcSelect`, `funcRepeat`, `funcOfType`, `isKnownType`,
`funcSingle`, `funcFirst`, `funcLast`, `funcTail`, `funcSkip`, `funcTake`,
`funcIntersect`, `funcExclude`), and combining and membership —
`src/eval/combine.rs` (`opUnion`, `funcUnion`, `funcCombine`, `opIn`,
`opContains`, `doContains`) — and the predicates from
`FHIRPathEngine.java`'s `funcEmpty`, `funcExists`, `funcNot`, `funcAll`, the
four boolean folds, `funcSubsetOf`, `funcSupersetOf`, `funcIsDistinct`,
`funcDistinct`, `funcCount`, `asBool`, `doEquals`, `opEquals` and
`opNotEquals`, from `Base.java`'s `compareDeep` and `isEmpty`, and from
`ca.uhn.fhir.util.ElementUtil.isEmpty` in `hapi-fhir` at
`7ab4c7c7c49281496faa1b3c2498a652457669ae` — extend it. Its static
check pass — `src/check/{mod,pass,signature,types}.rs`, from
`FHIRPathEngine.java`'s `check`, `checkOnTypes`, `executeType`,
`operateTypes`, `resolveConstantType`, `evaluateFunctionType`,
`checkParamTypes`, the `checkContext*` family and `getChildTypesByName`, and
from `TypeDetails.java` — the third. Equality, equivalence and ordered
comparison — `src/eval/{equality,compare,operand,temporal}.rs`
(`opEquivalent`, `opNotEquivalent`, `doEquivalent`, `qtyEqual`,
`qtyEquivalent`, `opLessThan`, `opGreater`, `opLessOrEqual`,
`opGreaterOrEqual`, `compareDateTimeElements`, `compareTimeElements`,
`funcHasValue`, `funcComparable` (its branches that need no UCUM service),
`FHIR_TYPES_STRING`, `Base.java`'s `hasType` and
`isPrimitive`, `BaseDateTimeType.java`'s `equalsUsingFhirPathRules`,
`compareTimes` and `getHighEdge`, `TimeType.java`'s field readers, and
`Utilities.java`'s `equivalent` and `equivalentNumber`) — extend the first
two, and boolean logic — `src/eval/logic.rs` (`preOperate`, `opAnd`,
`opOr`, `opXor`, `opImplies`, `isBoolean`, `funcIif`) — extends the same,
and arithmetic — `src/eval/arith.rs` (`opPlus`, `opMinus`, `opTimes`,
`opDivideBy`, `opDiv`, `opMod`, `opConcatenate`) — extends it again, with
the decimal arithmetic of `/`, `div` and `mod` in `src/eval/ucum_decimal.rs`,
a port of `Ucum-java`'s `org.fhir.ucum.Decimal` (`Decimal(String)`,
`setValueDecimal`, `setValueScientific`, `asDecimal`, `trunc`, `subtract`,
`doAdd`, `doSubtract`, `multiply`, `divide`, `divInt`, `modulo` and their
string helpers) and `Utilities` (`isDecimal`, `isInteger`, `noString`,
`padLeft`) at `db9d22ade3f0f4a667b45817b85a9ac4534e4224` (tag `1.0.10`),
under the BSD 3-Clause notice the file and `NOTICE` reproduce, and the
UCUM unit algebra over it — `src/eval/ucum.rs` (`XmlDefinitionsParser`'s
`parse`, `parseUnit`, `parseValue`, `parseBaseUnit` and `parsePrefix`,
`XmlUtils`' DOM helpers and `UcumModel.getUnit`) and
`src/eval/ucum_algebra.rs` (`Lexer`, `ExpressionParser`, `Converter`,
`Canonical`, `ExpressionComposer.compose(Canonical, false)`, the
`special/Registry` handlers, and `UcumEssenceService`'s
`getCanonicalForm`, `getCanonicalUnits`, `multiply`, `divideBy` and
`isComparable`), at the same SHA and under the same notice, and
date arithmetic and the clock — `src/eval/temporal.rs` (`dateAdd`,
`funcToday`, `funcNow`, and `BaseDateTimeType.java`'s `add`, `copy`
precision check and string rendering), and the math functions —
`src/eval/mathfn.rs` (`funcAbs`, `funcCeiling`, `funcFloor`, `funcRound`,
`funcTruncate`, `funcSqrt`, `funcExp`, `funcLn`, `funcLog`, `funcPower` and
`DecimalType(double)`), and the base string functions — `src/eval/strfn.rs`
(`funcUpper`, `funcLower`, `funcToChars`, `funcIndexOf`, `funcSubstring`,
`funcStartsWith`, `funcEndsWith`, `funcContains`, `funcLength`), and the
regular-expression functions — `src/eval/strregex.rs` (`funcMatches`,
`funcMatchesFull`, `funcReplaceMatches`; the `regex` crate stands in for
`java.util.regex`, ADR 0005), and the
string functions that need no regular-expression engine —
`src/eval/strext.rs` (`funcReplace`, `funcTrim`, `funcSplit`, `funcJoin`,
`funcEncode`, `funcDecode`, `funcEscape`, `funcUnescape`, `bytesToHex`,
`hexStringToByteArray`, and `Utilities.java`'s `escapeXml`, `unescapeXml`,
`escapeJson`, `unescapeJson`, `simpleSplit`, `URLEncode`, `URLDecode` and
`isWhitespace`, `CommaSeparatedStringBuilder.java` and
`MarkDownProcessor.java`'s markdown escaping), and type
conversion — `src/eval/convert.rs` (`funcToInteger`, `funcToDecimal`,
`funcToString`, `funcToBoolean`, `funcToQuantity`, `funcToDateTime`,
`funcToTime`, the `funcIsX` family, `parseQuantityString`, and
`Utilities.java`'s `isInteger`, `isDecimal` and `checkDecimal` and
`RegexConstants.java`'s `DATE_TIME_REGEX`), and the type operators and
reflection — `src/eval/types.rs` (`executeTypeName`, `opIs`, `opAs`,
`compareTypeNames`, `isKnownType`, `funcIs`, `funcAs`, `funcType` and
`FHIRPathUtilityClasses.ClassTypeInfo`), the environment variables —
`src/eval/env.rs` (`resolveConstant` in both forms, `isSystemVariable`, the
name-step overload of `execute` with its two implicit host-service lookups,
`funcDefineVariable` and `ExecutionContext.setDefinedVariable`), the utility
functions — `src/eval/utility.rs` (`log`, `hasLog`, `takeLog`, `funcTrace`,
`funcCheck`, `funcAggregate`, `funcExtension`, `funcSort`,
`FHIRPathBaseSorter` and `convertToBoolean`), the boundary functions —
`src/eval/boundary.rs` (`funcLowBoundary`, `funcHighBoundary`,
`funcPrecision`, `Utilities.java`'s `lowBoundaryForDecimal`,
`highBoundaryForDecimal`, `applyPrecision`, `minusOne`, `isZero` and
`getDecimalPrecision`, and `DateTimeUtil.java`'s `lowBoundaryForDate`,
`highBoundaryForDate`, `lowBoundaryForTime`, `highBoundaryForTime`,
`applyDatePrecision`, `applyTimePrecision`, `splitTimezone`,
`getDatePrecision`, `getTimePrecision` and `dayCount`), the host-service
functions — `src/eval/hostfn.rs` (`funcConformsTo`, `funcMemberOf`,
`opMemberOf`, `funcResolve`, `makeIdentifier` and the `Custom` arm of
`evaluateFunction`, with the check pass's `Custom` arm of
`evaluateFunctionType` in `src/check/pass.rs`), the in-memory code
validation behind `memberOf` — `src/eval/terminology.rs` (the
include/exclude loop, `findCodeInConcept` and `checkExpansion` of
`org.hl7.fhir.r4/.../r4/terminologies/ValueSetCheckerSimple.java`), the
ordered comparison's in-place Zulu rewrite of a `dateTime` operand —
`src/eval/rewrite.rs` (`compareDateTimeElements`' `setTimeZoneZulu(true)`, and
`BaseDateTimeType.java`'s `setTimeZoneZulu` and `encode`, kept as a
per-evaluation overlay rather than a write to the input, reaching the items
each step returns, and for a `@` literal keyed by an occurrence number standing
for the new object `resolveConstant`/`processDateConstant` build per node
evaluation) — and the
server-facing host service, `src/seam/server.rs`, from `hapi-fhir`'s
`FhirPathR4.setEvaluationContext` at `7ab4c7c7c49281496faa1b3c2498a652457669ae`,
and the narrative's recomposition for `Narrative.div` —
`src/eval/{xhtml,xhtml_entities}.rs` and the `div` arms of
`src/eval/navigate.rs` (`XhtmlNode.setValueAsString`, the default
`XhtmlParser` path it reaches, with `DEFINED_ENTITIES` and the
`parseLiteral` fallback names transcribed into one sorted table, the XML-mode
non-pretty `XhtmlComposer`, `Narrative.getProperty` and `listChildren`,
`XhtmlType`, and `hapi-fhir`'s `XhtmlDt.setValueAsString` and
`preprocessXhtmlNamespaceDeclaration` at the same `hapi-fhir` SHA). The
`org.hl7.fhir.utilities` `xhtml` sources and the R4 model sources carry
HL7's BSD-style licence rather than Apache-2.0; `xhtml.rs` and
`xhtml_entities.rs` reproduce that notice in full.
Each file
carries the required header naming the upstream path, the pinned SHA and what
was modified. Every
other row is updated by the change that first ports into that crate;
`openspec-guard` checks that a crate whose sources gained a provenance header
also gained a row here.

`frevo-model`'s generated tables (`src/r4/`) are not derived, as above. Its
hand-written codec has two derived files. `src/codec/decode.rs` re-implements
the lenient and strict decode behaviour of `hapi-fhir`'s parser at
`7ab4c7c7c49281496faa1b3c2498a652457669ae`: `ParserState`'s
`PrimitiveState.attributeValue`, `enteringNewElement`,
`ContainedResourcesStateHl7Org.wereBack` and `weaveContainedResources`;
`JsonParser`'s `parseChildren` branches, its `resourceType` check and the
not-contained guard in `encodeResourceToJsonStreamWriter`; the fatal-condition
rule and message texts of `LenientErrorHandler`; the `Msg.code`s and message
texts of `StrictErrorHandler`; `ParseLocation.toString`; the condition set of
`IParserErrorHandler`; the default handler in `FhirContext`; the encode-side
check in `BaseParser`'s `determineReferenceText` and
`isValidInternalReference` and `FhirTerser`'s
`ContainedResources.referenceMatchesAContainedResource`; and
`UrlUtil.sanitizeUrlPart` and `isNeedsSanitization`, transcribed. Its header
names `org.hl7.fhir.core`'s `org.hl7.fhir.r4.formats.JsonParser` as the
behavioural reference for the decoded shape (`resourceType` dispatch, choice
property names, `contained`, the `_field` sibling), but that class is
per-type generated Java and the decoder is driven by the generated element
tables, so nothing is transcribed from it and its BSD-style notice is not
engaged. `src/codec/built.rs` derives only its encode-side `#` reference
check, from the same `BaseParser`, `FhirTerser` (`containResources` included)
and `JsonParser` sources. All of these `hapi-fhir-base` sources are Apache-2.0,
Copyright (C) 2014 - 2026 Smile CDR, Inc., which the existing HAPI FHIR
section of `NOTICE` already covers. The other
codec files (`doc`, `mod`, `nav`, `path`, `pretty`, `scan`, `stored`,
`tables`, `provisional_r4`) are independent: they cite HAPI only for a
`Msg.code` or a layout convention they match, not for logic they port.

## Derived harness files

| File | Upstream path | Pinned SHA | Notes |
|---|---|---|---|
| `harness/hapi-fhirpath-results/src/main/java/dev/frevo/harness/HapiFhirPathResults.java` | `hapifhir/org.hl7.fhir.core` `org.hl7.fhir.r4/src/test/java/org/hl7/fhir/r4/test/FHIRPathTests.java` | `73c432b5f43ddbce979978a760ec4aa81e929094` | Apache-2.0. Test body kept; JUnit parameterisation replaced by a catch-per-case loop that writes JSON. Used only to record HAPI's measured results (`oracles/fhirpath/hapi-8.12.0-results.json`); not shipped. |
