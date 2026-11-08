# 4. FHIRPath and models: reuse, fork, or rewrite

- **Status:** accepted, provisional — see "What would change this"
- **Date:** 2026-11-08
- **Context phase:** Phase 0, deciding the scope of `add-fhirpath-and-codegen-r4`

## Context

`frevo-path` and `frevo-model` are the foundation: search-parameter extraction,
validation invariants, and CQL all evaluate FHIRPath, and every read path goes
through the model codec. Several Rust crates already implement these, so
building our own has to be justified rather than assumed.

Candidates evaluated, all actively maintained as of 2026-11-08:

| Crate | Version | Licence | Public repo | Scope |
|---|---|---|---|---|
| `haste-fhirpath` | 0.129.3 | **Apache-2.0** | **none** | FHIRPath engine |
| `haste-fhir-model` | 0.129.3 | **Apache-2.0** | **none** | R4 + R4B models |
| `helios-fhirpath` | 0.2.7 | MIT | [HeliosSoftware/hfs](https://github.com/HeliosSoftware/hfs) | FHIRPath engine |
| `helios-fhirpath-support` | 0.2.7 | MIT | same | model-agnostic evaluation types |
| `helios-fhir` | 0.2.7 | MIT | same | models, plus `fhir-gen` generator |
| `fhir-sdk` | 0.16.0 | MIT | [FlixCoder/fhir-sdk](https://github.com/FlixCoder/fhir-sdk) | client SDK |
| `fhir` (fhir-rust) | 4.2.4 | MIT OR Apache-2.0 OR BSD-3 OR GPL-2.0 OR GPL-3.0 | [fhir-rust](https://github.com/fhir-rust/fhir-rust/) | R2–R6 models |

### What was actually measured

Honesty about the evidence matters here, because this decision is easy to
dress up. What follows is from reading the published sources, not from running
the oracle:

- licences, versions, sizes, dependency manifests, public API surface;
- for `haste-fhirpath`: 8,131 lines of source, of which 964 are spec tests
  covering "the examples the FHIRPath N1 specification gives for each function
  and operator" — real evidence of maturity;
- for both engines: the exact type of their evaluation seam.

**Pass rates against `oracles/fhirpath` (1,191 cases) were not measured.**

> The case count here was 1,193 in the first draft of this ADR, and that was
> wrong. `tests-fhir-r4.xml` wraps the whole `testDollarResource` group in an
> XML comment whose bodies still contain `<test ...>` text, so a textual count
> reports 104 groups and 1,193 cases where parsing reports **103 and 1,191**.
> The independent spec review caught it. Count by parsing, never by matching. The
suite references external FHIR example files in XML, so running it needs a FHIR
XML parser and a test harness — which is precisely the harness
`oracle-builder` builds as task 1 of this change. Quoting a pass rate now would
mean inventing one.

### The finding that decides it

The two engines differ fundamentally in how they take input.

**`haste-fhirpath` is hard-coupled to `haste-fhir-model`'s R4 types.** Its
evaluation seam is `haste_reflect::MetaValue`:

```rust
fn get_field<'a>(&'a self, field: &str) -> Option<&'a dyn MetaValue>;
```

Two problems. First, the return is a **borrow with the receiver's lifetime**, so
an implementation must already hold a materialised value to hand out a
reference to — exactly what a lazy, byte-preserving model is designed not to
do. Satisfying it would mean eagerly parsing every element, abandoning the
central performance premise.

Second, and fatally: the engine body contains **77 `downcast_ref` calls to
`haste-fhir-model`'s concrete R4 types** (`FHIRBoolean` ×32, `FHIRString` ×15,
`Reference`, `Quantity`, `Identifier`, even `Patient`), and imports
`haste_fhir_model::r4::*` directly. A different model implementing `MetaValue`
would fail every downcast — and FHIRPath's semantics mean a failed downcast
yields an empty collection rather than an error. Reuse would therefore produce
an engine that silently returns nothing instead of failing loudly, in the
component that extracts search parameters and evaluates validation invariants.
That is the worst available failure mode.

**`helios-fhirpath` is model-agnostic.** `helios-fhirpath-support` (1,693
lines) defines an owned enum and a conversion trait:

```rust
pub enum EvaluationResult {
    Empty,
    Boolean(bool, Option<TypeInfoResult>, Option<Box<PrimitiveElement>>),
    String(String, Option<TypeInfoResult>, Option<Box<PrimitiveElement>>),
    // ...
}
pub trait IntoEvaluationResult { /* ... */ }
```

Values are **owned**, so a lazy model can materialise on demand and return
them. It even carries `PrimitiveElement` for the `_field` sibling form, meaning
it was designed with FHIR's primitive-extension representation in mind. This is
a genuine reuse seam.

Other blocking facts:

- `haste-fhirpath` declares `haste-fhir-model = "0.*"`, `haste-reflect = "0.*"`
  and `haste-reflect-derive = "0.*"` — **wildcard dependencies**, which
  `deny.toml` rejects (`wildcards = "deny"`). A wildcard is not auditable by
  version, so this is not a rule to relax.
- `haste-*` publish **no public repository**. We could not read history, audit
  a change, or contribute a fix upstream — only consume whatever is published.
- `haste-fhir-model` covers **R4 and R4B only**. Frevo needs DSTU2, DSTU3, R4,
  R4B and R5.
- `haste-fhirpath`'s API is **async** (`FPEngine::evaluate(...).await`).
  Search-parameter extraction runs inside a write transaction; making that path
  async for the engine's benefit is a cost with no return.
- Constitution rule 4 already settles the model question independently:
  **models are generated from StructureDefinitions, never adopted as a
  dependency**, because we need all five versions, a lazy byte-preserving
  codec, and search-parameter tables generated from the same inputs. No
  third-party model crate can satisfy that, so `haste-fhir-model`,
  `helios-fhir` and `fhir` are out by design, not by quality.
- `fhir-sdk` is a client SDK — the wrong shape entirely for a server. Not a
  candidate.
- `fhir` (fhir-rust) offers R2–R6 models under a disjunctive licence list that
  includes GPL options. The `OR` means we may choose MIT or Apache-2.0, so it
  is usable, but it is a model crate and rule 4 excludes it.

## Decision

1. **Write `frevo-path` ourselves.** Not a fork. No candidate is
   simultaneously Apache-2.0-compatible, model-agnostic, multi-version, and
   synchronous, and this component is too central to accept an impedance
   mismatch in — it is on the write path for every resource stored.

2. **Write `frevo-model` ourselves, generated by `frevo-codegen`.** Already
   required by constitution rule 4; the evaluation confirms no existing crate
   offers a lazy byte-preserving codec.

3. **Use `helios-fhirpath` as a differential reference during development, not
   as a dependency.** Its owned `EvaluationResult` makes it cheap to run the
   same expression through both engines and diff — the same technique the
   project already uses against HAPI. It becomes a dev-dependency of the test
   harness only, never of a shipped crate. MIT is compatible for that, and
   nothing in the product links it.

4. **The gate stays the official suite.** `oracles/fhirpath`, 1,191 cases, at
   or above HAPI 8.12.0's rate on the same cases. A second implementation to
   diff against is a debugging aid, never the standard.

5. **Design `frevo-path` against a trait, not against `frevo-model`
   concretely.** This is the lesson taken directly from `haste-fhirpath`: an
   engine welded to one model's concrete types cannot be reused, and worse,
   fails silently when handed anything else. `frevo-path` will evaluate over a
   small navigation trait that `frevo-model` implements, with values returned
   owned. That keeps it usable over the lazy model, over a parsed tree, and
   over whatever DSTU2/R5 need, and it keeps the engine independently testable.

## Consequences

Good:

- No dependency with a wildcard version, no dependency without a public
  repository, nothing that constrains us to R4 and R4B.
- The lazy byte-preserving model stays possible; adopting either engine's
  borrowed-reference seam would have ended it.
- A second independent engine to diff against, cheaply, for the whole of
  development.

Bad, and accepted:

- We write a FHIRPath engine. The reference implementation is roughly 8k lines
  in `haste-fhirpath`'s case, which is a fair estimate of the work, and the
  1,191-case suite is unforgiving. This is the largest single piece of
  avoidable work being deliberately taken on.
- `helios-fhirpath` could have been a genuine shortcut if MIT-only
  dependencies and a shared model were acceptable. If the pass-rate measurement
  shows it at or above HAPI's rate, declining it will have cost real time.

## What would change this

This decision is provisional on evidence that does not exist yet. Task 1 of
`add-fhirpath-and-codegen-r4` builds the oracle harness; task 2 runs
`helios-fhirpath` and `haste-fhirpath` through it and records their actual pass
rates and throughput in this ADR.

Reverse this decision if **either** holds:

- `helios-fhirpath` passes the 1,191-case suite at or above HAPI 8.12.0's rate
  **and** `IntoEvaluationResult` can be implemented over the lazy model without
  materialising whole resources. Then reuse it, pin it, and spend the saved
  time on `frevo-store`.
- Our own engine is below HAPI's rate after a bounded effort. Then fork
  `helios-fhirpath` (MIT permits it) rather than keep grinding, and record the
  fork in `docs/provenance.md`.

Either outcome is recorded here, with the measured numbers, before the change
is archived.
