# FHIRPath differential reference

Task 2 of `add-fhirpath-and-codegen-r4` (`path.oracle.013`, ADR 0004). It runs
the 1,191 cases of the vendored suite (`oracles/fhirpath/tests-fhir-r4.xml`)
through two other Rust FHIRPath engines, `helios-fhirpath` and
`haste-fhirpath`, beside Frevo and HAPI 8.12.0's recorded run, and writes the
per-case outcomes to `results.json`.

These are dev-only crates, outside the product workspace. The root `Cargo.toml`
lists both directories in `[workspace] exclude`, each has a `[workspace]` of its
own, and no `frevo-*` crate depends on either. Neither reference engine, nor
`haste-fhirpath`'s wildcard dependencies, can therefore enter the product's
dependency graph or the `cargo deny` gate. A reference engine is a debugging
aid. It is never a gate.

## Layout

| Path | What it is |
|---|---|
| `fhirpath-compare/` | Frevo and `helios-fhirpath` 0.2.7. It merges `haste-results.json` and HAPI's run into `results.json`. |
| `../fhirpath-compare-haste/` | `haste-fhirpath` 0.129.2. It writes `haste-results.json`. |

There are two crates because the engines' dependency graphs cannot be unified.
`helios-fhir` 0.2.7 pins `serde_json =1.0.144`, while `haste-fhir-model`
0.129.2 needs `^1.0.151`.

## Pins

Both crates commit their `Cargo.lock` files. Every reference crate is pinned
exactly, and every package in both lockfiles was released at least 14 days
before 2026-11-21 (owner decision, 2026-11-21).

| Crate | Version | Released | Licence |
|---|---|---|---|
| `helios-fhirpath`, `-support`, `helios-fhir`, `-macro`, `helios-serde-support`, `helios-observability` | 0.2.7 | 2026-11-01 | MIT |
| `haste-fhirpath`, `haste-fhir-model`, `haste-reflect`, `-derive`, `haste-fhir-serialization-json`, `-derive` | 0.129.2 | 2026-11-02 | Apache-2.0 |

Task 2 names `haste-fhirpath` 0.129.3. That release was published on
2026-11-08 and was 13.5 days old on the day of the pin, so 0.129.2 is the
newest release that meets the 14-day rule. `haste-fhirpath` declares its own
dependencies as `"0.*"`, and they are pinned to 0.129.2 here as well.

## How a case is run

Every engine goes through the oracle harness Frevo's gate uses
(`crates/frevo-path/tests/oracle/`, included by path). Inputs are loaded, cases
are staged (parse, check, evaluate) and outcomes are compared (`compare.rs`)
in exactly the same way for every engine.

**Inputs.** Both reference engines read FHIR JSON only. The JSON inputs are used
as fetched. The XML inputs are converted to FHIR JSON once, by `src/common.rs`,
using the element shapes of the pinned R4 type package. The conversion keeps
each decimal's lexical form. Both engines therefore read the same text.

**Each engine's binding** is documented at the top of its `main.rs`. In short,
each is bound the way its own suite runner binds it:
- Parse uses `helios`'s parser. `haste` has no public parser, so its parse step
  counts only a `ParseError` raised while evaluating over no input.
- Check uses `helios`'s `check_semantics` when a case has a `contextType`;
  `haste` has no static check.
- Evaluation applies the case's `context` expression first, then the
  expression.
- Each case runs on a thread of its own with a 10 s limit, so a panic or a hang
  is that case's failure.

**Did-not-run.** An input that an engine's model cannot deserialise leaves that
case `did-not-run`, and the reason is recorded.

## Running

From the repository root, with the oracle content fetched (`bash oracles/fetch.sh`):

```sh
cargo run --release --manifest-path harness/fhirpath-compare-haste/Cargo.toml
cargo run --release --manifest-path harness/fhirpath-compare/Cargo.toml
```

`results.json` has one entry per case: the `address`, then the `hapi`, `frevo`,
`helios` and `haste` statuses, plus a `<engine>_reason` for each engine's
non-passes. It also holds each engine's totals and the wall-clock of each full
run (HAPI's is not recorded).
