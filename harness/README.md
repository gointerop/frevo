# harness/

Differential testing against a live HAPI FHIR 8.12.0.

Specification prose is not the source of truth in this project; HAPI's
observable behaviour is. This harness is how that is checked: identical
requests go to HAPI and to Frevo, and the responses are compared after
normalisation.

## Quick start

```bash
docker compose -f harness/docker-compose.yml up -d
```

```bash
bash harness/run.sh --self-test
```

The self-test runs the corpus twice against HAPI alone. **Expected result: zero
diffs.** It is what proves the runner and the normalisation rules are not
themselves producing false differences — without it, a green differential run
would mean nothing.

```bash
bash harness/run.sh --candidate http://localhost:8080/fhir
```

Reports land in `harness/out/` (git-ignored): `differential.md` for humans and
`differential.xml` (JUnit) for CI.

## Layout

| Path | What it is |
|---|---|
| `docker-compose.yml` | HAPI 8.12.0 pinned by digest, its PostgreSQL, and a separate PostgreSQL for Frevo |
| `run.py` / `run.sh` | the runner: sends, normalises, diffs, reports |
| `normalize.py` | the normalisation rules, each documented with what it stops checking |
| `corpus/*.yaml` | request corpora |
| `hook-tests/run.sh` | self-tests for the enforcement hooks (74 cases) |
| `scripts/synthea.sh` | generate and load Synthea datasets |
| `scripts/load_bundles.py` | concurrent transaction-bundle loader, also the ingest benchmark workload |
| `scripts/scenario-coverage.sh` | scenario coverage, both directions |
| `scripts/gen-inventory.py` | regenerates `docs/inventory.md` from `reference/` |
| `data/`, `out/` | generated datasets and reports; git-ignored |

## The corpus format

```yaml
suite: search.basic
cases:
  - id: search.token.001
    description: token search on identifier returns exactly the tagged resource
    scenarios: [search.token.001]        # ids from the spec delta
    setup:                               # optional, runs before the request
      - method: POST
        path: /Patient
        body: { resourceType: Patient, identifier: [...] }
        capture:
          pid: id                        # bind the created id to {{pid}}
    request:
      method: GET
      path: /Patient?identifier=http://frevo.test/run|{{run}}
    expect_status_class: 2xx             # or an exact code, e.g. 404
    compare_headers: false               # headers are compared only on request
    normalize: [ids, meta_lastupdated, paging_links, bundle_entry_order]
```

### Isolation

Each run gets its own id, available as `{{run}}`. Cases tag the resources they
create with it and scope their searches to it.

This is not cosmetic. A search that is not scoped to the run returns every
resource any previous run created, so any assertion about a count or an order
becomes meaningless as soon as the suite has been run twice. It is also what
makes the self-test honest: the baseline run and the candidate run get
different run ids, so running against a single server is still two independent
runs rather than the second one reading the first one's rows.

### Normalisation is the dangerous part

Two conformant servers differ in ways that carry no meaning: resource ids,
timestamps, paging tokens, and Bundle entry order where the specification
leaves it undefined. Those differences have to be removed or every case would
report a diff.

**Every rule applied is a field no longer being checked.** A rule list that is
too broad turns a parity test into a test that passes whatever the server does
— worse than no test, because it reports confidence nobody earned.

So:

- rules are opt-in per case, never global;
- every rule is documented in `normalize.py` with what it stops checking
  (`python harness/run.py --list-rules`);
- an unknown rule name is a hard error, not a no-op;
- rules are chosen as narrowly as the case allows.

Examples of getting this right:

- `store.update.001` does **not** use `meta_versionid`: the point of the case
  is that the version advanced, so the value has to be compared.
- `search.sort.001` does **not** use `bundle_entry_order`: the order *is* the
  behaviour under test.
- `store.history.001` does **not** use `bundle_entry_order`: history is ordered
  by time.

**Never add a rule to make a diff go away.** A real difference is either a bug
or a recorded deviation in [`docs/deviations.md`](../docs/deviations.md). The
harness cannot tell the two apart; that judgement is the reason the deviations
log exists.

### What the harness is not for

There is deliberately no CapabilityStatement case. Its contents *are* the
parity progress, so until parity is reached it differs from HAPI's on every run
by design, and the only way to make such a case pass would be to normalise away
the very thing being built. `CapabilityStatement` behaviour is specified in
`openspec/specs/server/spec.md` and tested directly.

## Datasets

```bash
bash harness/scripts/synthea.sh generate small           # 100 patients
bash harness/scripts/synthea.sh generate medium          # 10 000 patients
bash harness/scripts/synthea.sh load small http://localhost:8090/fhir
```

Synthea output is synthetic, so it carries no PHI and is safe to generate in a
public project — but it is large, so `harness/data/` is git-ignored and
datasets are regenerated rather than committed.

The jar is pinned to a tagged release (`v4.0.0`) and verified by SHA-256, and
generation uses fixed seeds. `master-branch-latest` is deliberately not used:
it moves, and a dataset that changes underneath a recorded benchmark baseline
makes the baseline meaningless without anyone noticing.

## The oracle's configuration is part of the definition of correct

`docker-compose.yml` sets HAPI's `hapi.fhir.*` flags explicitly rather than
leaving them to defaults, because each one changes what "correct" means. Two
choices worth knowing about:

- **Narrative generation is off.** HAPI templates with Thymeleaf and Frevo with
  minijinja, so byte-identical narrative is not achievable; comparing it would
  produce noise on every case. Narrative parity is tracked separately.
- **Scheduling, subscriptions, MDM and clinical reasoning are off by default.**
  Their background jobs make response timing and resource state vary between
  runs, which is fatal to a differential comparison. Suites that need them
  enable them through an override file.

HAPI's image is pinned by digest, matching the starter release in
[`reference/PINS.md`](../reference/PINS.md). Changing it redefines correctness
for every gate and invalidates every recorded baseline, which is why Dependabot
is configured not to touch it.
