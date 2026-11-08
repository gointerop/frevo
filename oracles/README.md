# oracles/

The official test suites that decide whether Frevo is correct.

Specification prose is not the source of truth in this project. Correctness is
established by these suites, then by differential testing against a live HAPI
8.12.0, then by HAPI's own JUnit tests mined for HTTP-level behaviour — in that
order of authority.

Pins, licences and dispositions: [`PINS.md`](PINS.md).

## Layout

| Path | Suite | Disposition |
|---|---|---|
| `fhirpath/` | official FHIRPath tests | vendored (231 KB, Apache-2.0) |
| `cql-tests/` | `cqframework/cql-tests` | vendored (431 KB, Apache-2.0) |
| `fhir-test-cases/` | HL7 validator and others | fetched — 286 MB |
| `r4-example-roundtrip/` | R4 example resources, `hl7.fhir.r4.examples#4.0.1` | fetched — 19.6 MB, 5,311 resources, CC0-1.0, pinned by SHA-256 |
| `inferno-us-core/` | Inferno US Core kit | fetched — needs Ruby and a running server |
| `inferno-bulk-data/` | Inferno Bulk Data kit | fetched |
| `inferno-smart/` | Inferno SMART App Launch kit | fetched |

```bash
bash oracles/fetch.sh            # fetch what is not vendored
bash oracles/fetch.sh --verify   # verify vendored content against its pin
bash oracles/fetch.sh --list     # show pins and dispositions
```

Fetched trees land in `<suite>/fetched/` and are git-ignored.

## The rules that make these worth having

**Never edit an oracle case.** Not to fix a parse error, not to adjust an
expectation, not to narrow a filter. Making a failing case pass by changing the
case is the single most damaging thing anyone could do to this project, because
every later gate would then be measuring against a standard we moved ourselves.
`fetch.sh --verify` re-downloads vendored content and diffs it, so a local edit
is caught; `protect-paths.sh` blocks edits under `oracles/` outside a
`chore/oracles-*` branch; and the `openspec-guard` CI job enforces the same.

The remedy for a case that cannot pass is
[`docs/known-gaps.md`](../docs/known-gaps.md), with the exact case identifiers
and owner approval.

**The gate is parity with HAPI, not an absolute rate.** Several of these suites
contain cases that HAPI 8.12.0 itself fails, and `cql-tests` contains cases the
reference CQL engine fails. A suite therefore passes when Frevo's rate is at or
above the reference implementation's rate **on the same cases**. Each runner
records the reference rate alongside ours, so the comparison is explicit rather
than remembered.

**An unrun case is not a passing case.** Cases that need licensed terminology
(SNOMED CT, LOINC, ICD-10-CM, UMLS) cannot run without operator-supplied
content, which this repository must never contain. Runners report those as
`skipped-for-licence` — a status deliberately distinct from `passed` — so a
suite summary can never imply coverage that did not happen. A gate whose suite
could not run is **not green**; see `.claude/agents/gatekeeper.md`.

## Runner status

Each suite has a `runner.sh` that reports what it can do today. During Phase 0
they are stubs: they verify the suite is present and pinned, count the cases,
and exit with a clear "not implemented yet" rather than a misleading pass. A
runner that returned success before anything was implemented would be the
easiest possible way to fake progress.

They become real as part of the change that implements the capability they
grade — `oracle-builder` wires them up before any implementation exists.
