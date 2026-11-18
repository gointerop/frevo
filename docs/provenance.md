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
| `frevo-model` | empty | — | (will be `generated` from R4 StructureDefinitions) | — |
| `frevo-path` | derived | `org.hl7.fhir.core` | `org.hl7.fhir.r4/.../r4/fhirpath/{FHIRLexer,ExpressionNode,FHIRPathEngine,FHIRPathConstant,TypeDetails}.java`, `org.hl7.fhir.r4/.../r4/model/{IdType,Base,BaseDateTimeType,TimeType}.java`, `org.hl7.fhir.utilities/.../Utilities.java` | `73c432b5f43ddbce979978a760ec4aa81e929094` |
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
`funcHasValue`, `FHIR_TYPES_STRING`, `Base.java`'s `hasType` and
`isPrimitive`, `BaseDateTimeType.java`'s `equalsUsingFhirPathRules`,
`compareTimes` and `getHighEdge`, `TimeType.java`'s field readers, and
`Utilities.java`'s `equivalent` and `equivalentNumber`) — extend the first
two, and boolean logic — `src/eval/logic.rs` (`preOperate`, `opAnd`,
`opOr`, `opXor`, `opImplies`, `isBoolean`, `funcIif`) — extends the same,
and arithmetic — `src/eval/arith.rs` (`opPlus`, `opMinus`, `opTimes`,
`opDivideBy`, `opDiv`, `opMod`, `opConcatenate`) — extends it again, and
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
`FHIRPathBaseSorter` and `convertToBoolean`) — and the
server-facing host service, `src/seam/server.rs`, from `hapi-fhir`'s
`FhirPathR4.setEvaluationContext` at `7ab4c7c7c49281496faa1b3c2498a652457669ae`.
Each file
carries the required header naming the upstream path, the pinned SHA and what
was modified. Every
other row is updated by the change that first ports into that crate;
`openspec-guard` checks that a crate whose sources gained a provenance header
also gained a row here.

## Derived harness files

| File | Upstream path | Pinned SHA | Notes |
|---|---|---|---|
| `harness/hapi-fhirpath-results/src/main/java/dev/frevo/harness/HapiFhirPathResults.java` | `hapifhir/org.hl7.fhir.core` `org.hl7.fhir.r4/src/test/java/org/hl7/fhir/r4/test/FHIRPathTests.java` | `73c432b5f43ddbce979978a760ec4aa81e929094` | Apache-2.0. Test body kept; JUnit parameterisation replaced by a catch-per-case loop that writes JSON. Used only to record HAPI's measured results (`oracles/fhirpath/hapi-8.12.0-results.json`); not shipped. |
