# Contributing to Frevo

Frevo is developed spec-first. Code is never the first artifact: a change is
described, reviewed against the upstream behaviour it must match, covered by
tests, and only then implemented. This is enforced by repository hooks and by
the `openspec-guard` CI job, so a pull request that skips a step fails rather
than relying on a reviewer to notice.

## Ground rules

1. **HAPI FHIR 8.12.0 is the reference.** Where the specification and HAPI
   disagree, HAPI wins by default and the case is recorded in
   [`docs/deviations.md`](docs/deviations.md).
2. **Never weaken a test or an oracle case to make a gate pass.** Do not add
   `#[ignore]`, delete a test, or narrow an oracle filter. An unfixable case goes
   to [`docs/known-gaps.md`](docs/known-gaps.md) with a reason, and needs
   maintainer approval.
3. **Generated code is not hand-edited.** `crates/frevo-model`, the converters
   and the search-parameter tables come from `frevo-codegen`. Fix the generator.
4. **PostgreSQL is the only runtime dependency.** No new service, broker, or
   search engine.
5. Every behaviour has a scenario ID, and every scenario ID has at least one
   test tagged `// covers: <scenario-id>`.

## Local setup

```bash
# Spec tooling
npm i -g @fission-ai/openspec@latest

# Test runner used by the gates
cargo install cargo-nextest --locked
```

The authoritative build target is Linux. On Windows, either install the
`x86_64-pc-windows-gnu` toolchain (`rustup set default-host
x86_64-pc-windows-gnu`) or build inside the dev container; the default `msvc`
host needs a Visual Studio C++ toolchain that is not otherwise required by this
project. See [`docs/adr/0001-development-toolchain.md`](docs/adr/0001-development-toolchain.md).

## The workflow

```
proposal -> spec delta (scenario IDs) -> design -> tasks
         -> tests (from the spec and the oracles, before the implementation)
         -> implementation -> gates -> archive
```

Concretely:

```bash
# Product work happens on change/<id> branches, one change per branch.
git switch -c change/add-my-capability

# Write the proposal, spec delta, design and tasks under
# openspec/changes/add-my-capability/, then validate strictly:
openspec validate add-my-capability --strict
```

The spec must be reviewed next: `review.md` has to begin with `APPROVED` before
any file under `crates/` may be edited. Then write the tests — they must fail
for the right reason before you implement — implement, and run the gates:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
bash harness/run.sh
```

Open a pull request when they are green. Never push to `main`.

Branch naming is part of the contract:

| Branch | For | Constraints |
|---|---|---|
| `change/<id>` | product work | exactly one active OpenSpec change; must touch `openspec/changes/<id>/` |
| `chore/<slug>` | CI, harness, docs, tooling | no behavioural edits under `crates/**/src` |
| `chore/governance-*` | the enforcement machinery itself | the only branches that may modify `.claude/**`, `gates/**`, or `openspec-guard.yml` |
| `chore/oracles-*` | vendoring or re-pinning oracle suites | the only branches that may modify `oracles/**` |

## Commits and sign-off

Conventional commits (`feat:`, `fix:`, `docs:`, `chore:`, `test:`, `perf:`,
`refactor:`), scoped to a crate where that helps: `feat(frevo-search): ...`.

All commits require a Developer Certificate of Origin sign-off:

```bash
git commit -s -m "feat(frevo-path): add collection navigation"
```

Commits made by an agent additionally carry a session trailer identifying the
run that produced them.

## What never goes in this repository

This repository is public. Do not commit:

- secrets, tokens, or credentials of any kind;
- real patient data or PHI — use Synthea output and specification examples only;
- licensed terminology distributions (SNOMED CT, LOINC, ICD-10-CM, UMLS). Frevo
  ships loaders and tests that accept user-supplied files; only samples whose
  licence permits redistribution are committed.

Before vendoring anything into `oracles/`, check its licence. If it is not
redistributable, fetch it in CI instead.

## Reporting problems

Behavioural differences from HAPI 8.12.0 are the most valuable bug reports. The
best form is a request and response pair, plus HAPI's response to the same
request. Security issues go through [`SECURITY.md`](SECURITY.md), not the issue
tracker.
