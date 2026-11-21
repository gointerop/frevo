# Oracle suite pins and licence determinations

These suites define correctness. Re-pinning one changes every gate in the
project, so pins move only on a `chore/oracles-*` branch, reviewed as a
decision rather than as a dependency bump.

Every suite's licence was checked before anything was committed, as the
constitution requires. Where content is not redistributable, it is **fetched**
rather than vendored, and the fetch is verified against the pin.

| Suite | Source | Pin | Licence | Disposition |
|---|---|---|---|---|
| `fhirpath` | `FHIR/fhir-test-cases` `r4/fhirpath/tests-fhir-r4.xml` | `c95adbee099c875de97d25ed7616aae7d66dd85b` | Apache-2.0 | **vendored** (231 KB, one file) |
| `cql-tests` | `cqframework/cql-tests` | `5747d2b72f29d733fbc114383c4a15a30999ffaf` | Apache-2.0 | **vendored** (431 KB) |
| `fhir-test-cases` | `FHIR/fhir-test-cases` (whole repo) | `c95adbee099c875de97d25ed7616aae7d66dd85b` | Apache-2.0 | **fetched** — 286 MB, too large to commit |
| `inferno-us-core` | `inferno-framework/us-core-test-kit` | `87ddb5358b73a851fc64b765963dbccab41e50a2` | Apache-2.0 | **fetched** — needs Ruby and a running server |
| `inferno-bulk-data` | `inferno-framework/bulk-data-test-kit` | `7428430c05fa399ae3e84d411e6b7e270f14fb5e` | Apache-2.0 | **fetched** — same |
| `inferno-smart` | `inferno-framework/smart-app-launch-test-kit` | `980e54e4ed632b28267d797013399a8588772174` | Apache-2.0 | **fetched** — same |
| `r4-example-roundtrip` | `hl7.fhir.r4.examples#4.0.1` via `packages.fhir.org` | `sha256:e18b31e7a52145a31f1f3f409cf6847583b08b7306cc4e9460a95a7b5efba930` (the package is not a repository, so no commit exists) | CC0-1.0 — see below | **fetched** — 19.6 MB, 5,311 resources |

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

**`hl7.fhir.r4.examples#4.0.1` is CC0-1.0, declared by the package itself.**
This is the one suite that is not Apache-2.0, so the evidence is recorded rather
than summarised — and it was nearly recorded wrongly, because the two obvious
places to look do not answer the question.

- **The package manifest declares it.** `package/package.json` inside the
  published tarball carries `"license": "CC0-1.0"` and `"author": "HL7 Inc"`.
  That is the publisher stating the licence for this specific artifact, which is
  exactly what the two sources below fail to do.
- **HL7's R4 licence page grants CC0 but does not reach this artifact on its
  own.** `https://hl7.org/fhir/R4/license.html` says "This document is licensed
  under Creative Commons 'No Rights Reserved' (CC0)" and "You can redistribute
  FHIR", but it scopes the specification to "the set of materials included in
  the fhir-spec.zip file available from the Downloads page". Neither it nor the
  R5 page mentions example resources or the examples package, so that page alone
  would have left the question to inference.
- **No registry declares it.** `packages.fhir.org` returns no `license` field
  for the package, and the Simplifier page
  (`simplifier.net/packages/hl7.fhir.r4.examples/4.0.1`) and `registry.fhir.org`
  carry no licence label, no package-level publisher and no release notes. The
  manifest is the only public statement, and it is inside the tarball — which
  is why it has to be downloaded to be read.

CC0-1.0 permits redistribution, so vendoring this suite would be lawful. The
disposition is still **fetched**, on size alone: 19.6 MB compressed and 5,311
JSON resources, the same reason `fhir-test-cases` is fetched. Nothing from the
package enters this repository, so it has no `ATTRIBUTION.md` — and CC0 waives
the attribution requirement in any case.

HL7's third-party IP carve-out is untouched by the package licence:
"Acceptance of these License Terms does not grant any rights with respect to
Third Party IP", and for SNOMED CT specifically, "Implementer use of SNOMED CT
is not covered by this agreement". R4 examples do reference SNOMED CT and LOINC
codes. That costs nothing here, because a round-trip test moves bytes and
performs no terminology lookup: referencing a code is not redistributing a code
system, which is what rule 10 forbids. Cases that need an actual expansion live
in `fhir-test-cases` and report `skipped-for-licence`.

Because a package has no commit to check out, the pin is a digest, and
`oracles/fetch.sh` fails rather than warns when the bytes do not match it. The
pin is **SHA-256**, computed from the downloaded artifact:
`e18b31e7a52145a31f1f3f409cf6847583b08b7306cc4e9460a95a7b5efba930`. The registry publishes only a SHA-1,
`ef0ea43649d05ff246b471cad4583b662f1f27b5`, which matched; it is recorded here
as corroboration, not as the pin. A suite that defines correctness should not
rest on SHA-1.

**`ucum/ucum-essence.xml` inside `fhir-test-cases` carries its own licence.**
`FHIR/fhir-test-cases` is Apache-2.0 as a repository, but the UCUM definition
file at `ucum/ucum-essence.xml` (82,332 bytes at the pinned commit `c95adbee`,
SHA-256 `3e4ba7b0ce2c7f83c9a6afe36c07d8cd8cfd7c0d22b6a2a790d4fec4efa89624`) is
Regenstrief Institute's work, not the repository's. The file has no licence
header of its own (it opens with the XML declaration and a `version="2.0.1"`
root), so the terms are those published at `https://ucum.org/license`: the UCUM
Copyright Notice and License, Version 1.1, June 2024, copyright 1999-2024
Regenstrief Institute, Inc. It permits royalty-free redistribution of the Work
unmodified, on condition of a copyright notice, a reference to the licence, a
reference to its disclaimer of warranties and the licence text or URL
(Section 3(b)), and forbids modifying the Work's content (Section 3(a)(iv)).
That decides how it is handled here:

- It is **fetched, never committed**. It sits only inside the git-ignored
  `oracles/fhir-test-cases/fetched/` tree. The FHIRPath result-generation
  procedure (`oracles/fhirpath/hapi-8.12.0-results.README.md`) separately
  downloads a copy from `org.hl7.fhir.core` at a pinned commit, to a git-ignored
  `target/` directory; that copy is under the same UCUM terms. Nothing in this
  repository redistributes either.
- It is **never edited**, which is the oracle rule anyway.
- Regenstrief's notice is carried in [`NOTICE`](../NOTICE), so a distribution
  of Frevo that bundles the file, or a binary that embeds UCUM data derived from
  it, already has the notice and the licence URL the licence asks for.
- The Apache-2.0 determination above for `fhir-test-cases` therefore covers the
  repository, **not** this one file. The licence text was read from
  `https://ucum.org/license` on 2026-11-21; the page, not a registry field, is
  the evidence.

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
