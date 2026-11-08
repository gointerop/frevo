<!--
Frevo is spec-first. The openspec-guard job checks most of this mechanically;
this template is here so a human reviewer sees the same picture the guard does.
-->

## What this changes

<!-- One paragraph. What behaviour exists after this that did not before. -->

## Change type

- [ ] Product work on a `change/<id>` branch (an OpenSpec change)
- [ ] Governance or infrastructure on a `chore/<slug>` branch (no behavioural
      edits under `crates/**/src`)

**OpenSpec change id:** <!-- e.g. add-fhirpath-and-codegen-r4, or n/a -->

## Checklist for product work

- [ ] `openspec validate <id> --strict` passes
- [ ] `openspec/changes/<id>/review.md` begins with `APPROVED`
- [ ] Every new scenario ID has at least one test tagged `// covers: <id>`
- [ ] Tests were written before the implementation and failed for the right
      reason first
- [ ] `gates/<id>/report.json` is green for this head commit
- [ ] No test was ignored, deleted, weakened, or filtered out
- [ ] Generated code was regenerated, not hand-edited
- [ ] `docs/inventory.md` rows covered by this change now point at a passing test
- [ ] New upstream-derived files carry a provenance header and a
      `docs/provenance.md` row

## Oracles and parity

| Gate | Before | After | HAPI 8.12.0 |
|---|---|---|---|
| Relevant HL7 suite | | | |
| Differential (unexplained diffs) | | 0 | — |

<!--
Unexplained diffs must be zero. An explained diff needs an entry in
docs/deviations.md, with its ID below.
-->

**New deviations:** <!-- DEV-00NN, or none -->
**New known gaps:** <!-- GAP-00NN, or none. These need owner approval. -->

## Performance

<!--
Required for anything on a read, search, or ingest path. Give the benchmark
name and the before/after numbers, or state why no workload is affected.
-->

## Compatibility

- [ ] No new runtime dependency besides PostgreSQL
- [ ] `hapi.fhir.*` configuration keys keep their upstream names and semantics
- [ ] No breaking change to a documented endpoint, or it is called out above

## Sign-off

- [ ] Commits are conventional and signed off (`git commit -s`)
