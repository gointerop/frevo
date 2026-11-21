# HAPI 8.12.0 in-process probe for the differential-only FHIRPath scenarios

Owner decision for `add-fhirpath-and-codegen-r4`: the scenarios that no case of
the vendored suite grades (`openspec/changes/add-fhirpath-and-codegen-r4/spec.md`,
the thirty-five named scenarios and the six partial ones, plus every scenario in
`specs/path/spec.md` that calls itself differential-only) are graded by this
probe. It runs HAPI 8.12.0's R4 `FHIRPathEngine` in-process, the same way task
61a confirmed `DEV-0001` to `DEV-0010`. It needs no Docker and no server.

This is a dev-only Maven project. It sits outside the cargo workspace, and
nothing in the product depends on it.

## What it runs

| | |
|---|---|
| engine | `ca.uhn.hapi.fhir:org.hl7.fhir.r4:6.9.12` `FHIRPathEngine`, the `fhir_core_version` of `hapi-fhir` `v8.12.0` |
| `server` worker | `HapiWorkerContext(FhirContext.forR4(), ctx.getValidationSupport())` from `hapi-fhir-structures-r4:8.12.0`. This is the worker `FhirPathR4` is built on (`FhirPathR4.java:34-36`), and so the one a live server uses |
| `suite` worker | `SimpleWorkerContext.fromPackage(hl7.fhir.r4.core#4.0.1)` with a `UcumEssenceService`, the worker `FHIRPathTests` evaluates with (`TestingUtilities.java:89`) |
| UCUM | `org.fhir:ucum:1.0.9`, the version live HAPI 8.12.0 resolves (`reference/hapi-fhir/pom.xml:1148`), over `oracles/fhir-test-cases/fetched/ucum/ucum-essence.xml` |
| configuration | as `FhirPathR4` sets it: both `as` rules not enforced, no host service, a null application context and no static check, except where a probe says otherwise. The JVM zone is GMT, which is what `FHIRPathTests`' `UTC+1100` yields |

Each probe is evaluated under both workers. The scenario names the worker it is
graded under: `server` for `path.quantity.019` and `path.strictness.005`, and
`suite` everywhere else (`DEV-0029`, `DEV-0026`).

## Files

| Path | What it is |
|---|---|
| `../fhirpath-compare/differential/probes.py` | the probes, scenario by scenario, and the hand-built inputs. It writes `probes.json` and `inputs/` |
| `../fhirpath-compare/differential/hapi-observed.json` | this probe's output: every probe's outcome under both workers |
| `../fhirpath-compare/src/bin/differential.rs` | runs the same probes on Frevo, through the oracle harness's seam, and writes the comparison |
| `../fhirpath-compare/differential-results.json` | the comparison, keyed by scenario id |

## Running

From this directory, with the oracle content fetched (`bash oracles/fetch.sh`)
and `hl7.fhir.r4.core#4.0.1` in the FHIR package cache:

```sh
python -I ../fhirpath-compare/differential/probes.py
mvn -q -B compile dependency:build-classpath -Dmdep.outputFile=target/cp.txt
java -cp "target/classes;$(cat target/cp.txt)" dev.frevo.harness.HapiFhirPathDifferential \
  ../fhirpath-compare/differential/probes.json ../../oracles/fhir-test-cases/fetched/r4 \
  ../fhirpath-compare/differential/inputs ../../oracles/fhir-test-cases/fetched/ucum/ucum-essence.xml \
  ../fhirpath-compare/differential/hapi-observed.json
cd ../.. && cargo run --release --manifest-path harness/fhirpath-compare/Cargo.toml --bin differential
```

Off Windows, use `:` instead of `;` as the classpath separator.

## How an outcome is written

An outcome has a `stage`. It is `ok` with `items`, `hasLog` and `log`, or it is
the stage that failed: `input`, `parse`, `check`, `eval`, `timeout` or
`harness`, with the `error` message. A check-only probe that passes is
`checked`. Each item is `type:value` for a primitive,
`Quantity:value|unit|system|code` for a quantity, and the type name for
anything else. Every UTF-16 unit outside printable ASCII is written as
`<U+XXXX>`, on both sides, so that lone surrogates and NUL characters can be
compared. A probe with a host service also records `hostCalls`, which lists
every call the engine made to the host, in order.

The verdict is `agree` when Frevo's outcome equals HAPI's under the graded
worker. That means the same stage, then the same items, log and host calls, or
the same message. A parse failure is the one exception. Its message is
compared without the `Error @<line>, <column>: ` prefix of HAPI's
`FHIRLexerException`, because Frevo's `ParseError::message` is upstream's
wording without that prefix and carries the position as a byte offset instead
(`crates/frevo-path/src/syntax/error.rs:37-39`, `path.parse.007`). The
position is not compared. A probe graded `error` compares only the failing stage,
because its scenario says the text is the JVM's. A probe graded `shape`
compares only the item types (`now()`). It is `disagree-documented` when the
probe names the register entry under which the scenario already says Frevo
differs from HAPI, and `disagree` otherwise.
