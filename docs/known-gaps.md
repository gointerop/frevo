# Known gaps

Oracle cases and behaviours that Frevo does not pass, recorded here instead of
being skipped, ignored, or filtered out of a suite.

A gate may only be considered green with a gap open if the gap is listed here
**and** has owner approval. Adding an entry is not a way to make a red gate
green; it is a way to make a red gate visible.

## Rules

- Never add `#[ignore]`, delete a test, or narrow an oracle filter to avoid a
  failure. Record it here instead.
- Each entry names the suite, the exact case identifiers, and why it fails.
- Each entry has an owner decision: `accepted` (we will not fix), `deferred`
  (we will fix, with the change that will do it), or `blocked` (something
  outside the project must change first).
- Pass-rate gates are expressed relative to HAPI: a suite must pass at or above
  HAPI's own rate. A case that HAPI also fails is not a gap — it is parity, and
  belongs in a note on the suite's runner, not here.

## Entry format

```markdown
### GAP-0001 — short title

- **Suite:** fhirpath | fhir-test-cases | cql-tests | inferno-us-core | differential
- **Cases:** exact identifiers, not a count.
- **Observed:** what fails and how.
- **HAPI 8.12.0 on the same cases:** passes | fails | not applicable.
- **Cause:** root cause, or "not yet diagnosed".
- **Decision:** accepted | deferred | blocked
- **Unblocked by:** change id, or what must change externally.
- **Approved:** date, by whom.
```

## Entries

_None yet._ Phase 0 implements no capability, so there is nothing to fail.

## Expected sources of gaps

Recorded in advance so that they are not mistaken for surprises later:

- **Licensed terminology.** SNOMED CT, LOINC, ICD-10-CM and UMLS content cannot
  be redistributed, so any oracle case that requires a full code system will run
  only where the operator supplies the files. These are environment-conditional
  cases, not gaps, and the runner must report them as skipped-for-licence with a
  distinct status so they can never be confused with passes.
- **Inferno kits** need a running server and a published endpoint; they will
  start as a scheduled job rather than a per-pull-request gate.
- **CQL engine coverage.** `cql-tests` includes cases the reference engine
  itself does not pass; the gate is parity with the reference engine, not an
  absolute rate.
