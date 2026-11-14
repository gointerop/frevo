CHANGES_REQUESTED

Eighth review of `add-fhirpath-and-codegen-r4`. It covers commit `ebd65da`, which
changes `specs/path/spec.md` (`path.check.002`, `path.oracle.016`,
`path.oracle.017`), the `spec.md` scenario index and task 50. I checked it
against `oracles/fhirpath/tests-fhir-r4.xml`,
`reference/org.hl7.fhir.core/org.hl7.fhir.r4/src/test/java/org/hl7/fhir/r4/test/FHIRPathTests.java`
(pin `73c432b5`), `.../r4/fhirpath/FHIRPathEngine.java`, `.../r4/fhirpath/ExpressionNode.java`
and `crates/frevo-path/tests/oracle/run.rs`.

## What I verified independently

**The 29 / 27 / 2 split is correct.** I computed it with a script, not from the
proposal. The script parsed the suite, took every `<expression invalid="semantic">`,
extracted each `name(` call and compared it with the 98 codes that
`ExpressionNode.Function.fromCode` accepts.

- There are 29 `invalid="semantic"` cases.
- Exactly two call a function that the R4 enum does not have:
  `documentFunctions/testResourcesAbstract3` (`tests-fhir-r4.xml:1940`,
  `resources`) and `documentFunctions/testHasCoding11` (`:1951`, `hasCoding`).
  The script also flagged `isSemanticChecks/isGuardOrInnerImpliesBad`, but that
  was the operator `or (`, so it is a false positive.
- The other 27 call only enum functions and reach the check.

For those two cases, `FHIRPathEngine.java:1158-1166` calls
`hostServices.resolveFunction` only when `fromCode` returns null. The suite's
host service throws `NotImplementedException` there (`FHIRPathTests.java:72-76`).
That exception is caught at `FHIRPathTests.java:199-202`, where
`assertTrue(fail == SYNTAX)` fails for a `SEMANTICS` expectation. So HAPI grades
both cases failed. In `run.rs`, a parse-stage `Diagnostic` goes to
`graded_failure`, which returns `Failed` when `expected_failure != Some(Parse)`
(`run.rs:266`, `:369-388`). Frevo also grades both failed, so this is parity.

**The amended texts agree with each other.** These all say 27 cases fail the
check and 2 fail at parse, graded failed for both engines:

- the `path.check.002` heading and its bullets;
- the last bullet of `path.oracle.016`;
- `path.oracle.017` ("one of the 27 cases"; `testIif/testIif6` at `:625` is one
  of the 27);
- the `spec.md:727` index line;
- task 50.

The remaining mentions of "29" are all correct as stated:

- `spec.md:120` and `proposal.md:121`, which describe the oracle source;
- `spec.md:134` and `proposal.md:151`, which give the attribute counts;
- the requirement prose at `specs/path/spec.md:2030`;
- the `WHEN` of `path.check.002`.

**Scenario IDs are unchanged.** `git diff 0642f36 HEAD -- openspec` changes no
scenario ID. Only the title text of `path.check.002` changed, in the heading and
the index. No ID is duplicated across `specs/**`.

## Findings

1. **`path.oracle.014` misstates the status of four `documentFunctions` cases,
   for both Frevo and HAPI.** The scenario says that each of the 115 cases "is
   reported `failed`" and that "HAPI's recorded status for the same cases is
   `failed` too" (`specs/path/spec.md:2764-2772`). Four cases in the group are
   `invalid="syntax"`:
   - `testSection13` (`tests-fhir-r4.xml:1860`)
   - `testEntry12` (`:1879`)
   - `testSameSubject16` (`:1930`)
   - `testHasCoding10` (`:1950`)

   All four call a function the R4 enum does not have, so they fail at parse.

   - **HAPI:** `resolveFunction` throws (`FHIRPathTests.java:72-76`), the
     exception is caught at `:199-202`, and `assertTrue(fail == SYNTAX)` holds.
     After that, `node` is null and no later assertion runs (`:205`, `:244`,
     `:259`). All four **pass** in HAPI.
   - **Frevo:** a parse-stage `Diagnostic` with `expected_failure == Some(Parse)`
     is graded `Passed` (`run.rs:369-372`).

   The correct breakdown is 111 `failed` (103 valid, 6 `execution`, 2
   `semantic`) and 4 `passed`, the same for both engines. This also explains
   the "111 failed / 7 passed" in task 50's note, where the 7 are all seven
   syntax-invalid cases.

   The covering test already asserts this and so contradicts its own scenario:
   `crates/frevo-path/tests/path_oracle.rs:438-466` asserts that the four are
   `CaseStatus::Passed` and the other 111 are `Failed`. An oracle-builder working
   from the scenario alone would write the opposite assertion. The aggregate
   conclusion ("the aggregate gate is unaffected") still holds, because both
   sides pass the same four.

   Amend `path.oracle.014` (ID unchanged):
   - 111 cases are `failed` with the out-of-scope reason naming the function.
   - The four `invalid="syntax"` cases are `passed` at the parse stage, because
     an unknown function is a parse error (`FHIRPathEngine.java:1158-1166`).
     HAPI passes the same four (`FHIRPathTests.java:199-202`).
   - The unknown-function parse error must be a `Diagnostic`, not
     `NotImplemented`. Otherwise `classify` would make these four `did-not-run`
     (`run.rs:197-202`).

   Correct the statements that depend on this:
   - `path.check.002` says the two parse-stage cases are graded `failed` "as
     `path.oracle.014` requires of every `documentFunctions` case". The real
     reason is `path.oracle.005`'s wrong-stage rule. `path.oracle.014` does not
     require it of every case.
   - `tasks.md:90` (task 3) says "Confirm and record that all 115
     `documentFunctions` cases fail for HAPI too". It should be 111 fail and 4
     pass.
   - `proposal.md:225` says "All 115 cases are therefore failures for HAPI
     8.12.0 too". It should be 111.
   - `proposal.md:245-246` and `design.md:517-521` describe 115 cases "the R4
     engine cannot pass". It passes 4 of them.
