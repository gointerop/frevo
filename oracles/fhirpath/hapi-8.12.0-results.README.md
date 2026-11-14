# HAPI's own result on the pinned FHIRPath suite

`hapi-8.12.0-results.json` records what HAPI's R4 FHIRPath test harness
(`org.hl7.fhir.r4.test.FHIRPathTests`, `hapifhir/org.hl7.fhir.core` 6.9.12, pin
`73c432b5f43ddbce979978a760ec4aa81e929094`) does with each of the 1,191
`<test>` cases of `tests-fhir-r4.xml`. `crates/frevo-path/tests/path_oracle.rs`
reads it. Address = `group/name[n]`, `n` being the 1-based position of the
`<test>` among its group's child elements, as `FHIRPathTests.getName` computes it.

## How it was produced

`org.hl7.fhir.r4:6.9.12:tests` is not published, but `TestingUtilities` ships in
the main jar. `harness/hapi-fhirpath-results/` holds a Maven project (outside
the cargo workspace) with a driver derived from `FHIRPathTests.test` (same
assertions, same order, time zone `UTC+1100`, `hl7.fhir.r4.core#4.0.1` context,
UCUM from `org.hl7.fhir.r4/src/test/resources/ucum/ucum-essence.xml` at the
pin). It runs every case individually, catching each failure. Input resources
come from `oracles/fhir-test-cases/fetched/r4/` (run `oracles/fetch.sh`). The
suite is this repository's pinned `tests-fhir-r4.xml`, not the copy inside HAPI.

```
cd harness/hapi-fhirpath-results
mvn -q -B compile dependency:build-classpath -Dmdep.outputFile=target/cp.txt
mkdir -p target/ucum && curl -sfL -o target/ucum/ucum-essence.xml \
  https://raw.githubusercontent.com/hapifhir/org.hl7.fhir.core/73c432b5f43ddbce979978a760ec4aa81e929094/org.hl7.fhir.r4/src/test/resources/ucum/ucum-essence.xml
java -cp "target/classes;$(cat target/cp.txt)" dev.frevo.harness.HapiFhirPathResults \
  ../../oracles/fhirpath/tests-fhir-r4.xml ../../oracles/fhir-test-cases/fetched/r4 raw.json target/ucum/ucum-essence.xml
python -I postprocess.py ../../oracles/fhirpath/tests-fhir-r4.xml raw.json ../../oracles/fhirpath/hapi-8.12.0-results.json
```

(Use `:` instead of `;` as the classpath separator off Windows.)

## Status rules and counts

| Status | Count | Meaning |
|---|---|---|
| `passed` | 968 | HAPI's harness passes the case |
| `failed` | 210 | HAPI's harness fails the case (111 of them are `documentFunctions`) |
| `hapi-cannot-pass` | 13 | see below |

`documentFunctions`: 115 cases, all failing at parse in HAPI; 111 `failed` and 4
`passed` (`testSection13`, `testEntry12`, `testSameSubject16`, `testHasCoding10`,
the `invalid="syntax"` ones).

`hapi-cannot-pass` is computed mechanically by `postprocess.py`: cases in one
group with byte-identical expression and `inputfile` that differ only in
`context`/`contextType` and have differing expected outcomes. HAPI's harness
reads neither attribute, so it cannot tell them apart; the measured outcome is
kept in each `reason`. The 13 cases, all in `isSemanticChecks`:
isEffectOnLogic2Base, isEffectOnLogic3Base, isGuardOr, isGuardOr2,
isGuardOrBad, isGuardXorBad, isGuardXorBad2, isGuardOrInnerImplies,
isGuardOrInnerImpliesBad, isGuardOrTypesImpliesBad, isGuardOrTypesImpliesAll,
isGuardNotImplies, isGuardNotImpliesBad.
