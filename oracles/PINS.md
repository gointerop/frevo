# Oracle suite pins and licence determinations

These suites define correctness. Re-pinning one changes every gate in the
project, so pins move only on a `chore/oracles-*` branch, reviewed as a
decision rather than as a dependency bump.

Every suite's licence was checked before anything was committed, as the
constitution requires. Where content is not redistributable, it is **fetched**
rather than vendored, and the fetch is verified against the pin.

| Suite | Source | Pinned commit | Licence | Disposition |
|---|---|---|---|---|
| `fhirpath` | `FHIR/fhir-test-cases` `r4/fhirpath/tests-fhir-r4.xml` | `c95adbee099c875de97d25ed7616aae7d66dd85b` | Apache-2.0 | **vendored** (231 KB, one file) |
| `cql-tests` | `cqframework/cql-tests` | `5747d2b72f29d733fbc114383c4a15a30999ffaf` | Apache-2.0 | **vendored** (431 KB) |
| `fhir-test-cases` | `FHIR/fhir-test-cases` (whole repo) | `c95adbee099c875de97d25ed7616aae7d66dd85b` | Apache-2.0 | **fetched** — 286 MB, too large to commit |
| `inferno-us-core` | `inferno-framework/us-core-test-kit` | `87ddb5358b73a851fc64b765963dbccab41e50a2` | Apache-2.0 | **fetched** — needs Ruby and a running server |
| `inferno-bulk-data` | `inferno-framework/bulk-data-test-kit` | `7428430c05fa399ae3e84d411e6b7e270f14fb5e` | Apache-2.0 | **fetched** — same |
| `inferno-smart` | `inferno-framework/smart-app-launch-test-kit` | `980e54e4ed632b28267d797013399a8588772174` | Apache-2.0 | **fetched** — same |

## Licence findings worth recording

**`HL7/FHIRPath` has no licence file.** The repository that hosts the FHIRPath
specification and its test suite carries no `LICENSE`, `COPYING` or `NOTICE`,
and GitHub reports no detected licence. Under the constitution's rule 10 that
makes it non-redistributable: we may read it, but we may not commit any part of
it to a public Apache-2.0 repository.

This turned out not to cost anything. The same official FHIRPath test suite is
distributed inside **`FHIR/fhir-test-cases`**, which *is* Apache-2.0, as
`r4/fhirpath/tests-fhir-r4.xml`. That Apache-2.0 copy is what
`oracles/fhirpath/` vendors, so the FHIRPath gate has a redistributable source
and `HL7/FHIRPath` is not used at all.

The same file also ships inside `org.hl7.fhir.core`
(`org.hl7.fhir.r4/src/test/resources/fhirpath/tests-fhir-r4.xml`), which is
Apache-2.0 as well, giving a second independent route to the same cases.

**GitHub's licence detection is not evidence.**
`inferno-framework/smart-app-launch-test-kit` reports no SPDX licence through
the API, but it contains a full Apache-2.0 `LICENSE` plus a `NOTICE.md`. The
determination above is from reading the files, not from the API field.

## Licensed terminology is never vendored

No suite here brings SNOMED CT, LOINC, ICD-10-CM or UMLS content. Oracle cases
that need a full code system are **environment-conditional**: the runner reports
them as `skipped-for-licence`, a status deliberately distinct from `passed`, so
an unrun case can never be mistaken for a passing one. See
[`docs/known-gaps.md`](../docs/known-gaps.md).

## Refreshing

```bash
bash oracles/fetch.sh            # fetch everything not vendored, verify pins
bash oracles/fetch.sh --verify   # check vendored content against its pin
```

Fetched trees land in `oracles/<suite>/fetched/`, which is git-ignored.
