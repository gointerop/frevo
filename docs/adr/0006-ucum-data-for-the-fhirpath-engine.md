# 6. Where the FHIRPath engine gets its UCUM data

- **Status:** accepted by the owner on 2026-11-21 as proposed. The implementer of
  task 44 adopted it first.
- **Date:** 2026-11-21
- **Context phase:** `add-fhirpath-and-codegen-r4`, task 44

## Context

Quantity `=`, `~`, the ordered operators, `comparable()`, `*` and `/` consult
`worker.getUcumService()` upstream. `path.quantity.015` and `.019` (`DEV-0029`)
require Frevo to answer as the suite's worker does in every configuration, and
`path.quantity.016` requires data that cannot be loaded to be a failure, never
a silent fall into the engine's no-UCUM branch. `design.md` section 5 makes
`ucum-essence.xml` a fetched file loaded at run time, not data embedded in the
binary.

The evaluator API (`Evaluator::new(model)` and its `with_*` builders) does not
carry UCUM data. The arithmetic, equality and comparison rules are free
functions over a model, called from many places (`distinct()`, `contains`,
`subsetOf()` and others reach `doEquals`). The oracle harness builds its
evaluator with `Evaluator::new(model)` and supplies no UCUM data.

## Options considered

1. **A `with_ucum(...)` builder on `Evaluator`, threaded down to every
   equality and arithmetic call.** This is the most explicit option. But it
   touches every caller of `doEquals`. It also needs a rule for an evaluator
   built without data, which is exactly the no-UCUM branch the spec rules
   out, and the harness would have to change to pass the data.
2. **One process-wide UCUM service, loaded once on first use.** This mirrors a
   worker context that owns one `UcumEssenceService`. No evaluator can be
   without it, and no call site changes.
3. **Embed the file in the binary.** `design.md` section 5 rules this out.

## Decision

Option 2. `frevo_path::install_ucum_essence(UcumEssence)` installs the data
before first use. Otherwise the first quantity operation that needs UCUM loads,
in order:

1. the file named by the environment variable `FREVO_UCUM_ESSENCE`;
2. failing that, the fetched copy in the source tree the crate was built from,
   `oracles/fhir-test-cases/fetched/ucum/ucum-essence.xml`.

The data is fixed for the life of the process. If it cannot be loaded, every
operation that needs it fails with an execution error naming the file and the
cause.

## Consequences

- Good: there is no configuration without UCUM data, as `path.quantity.015`
  requires. The harness and every existing call site work unchanged, and the
  file is loaded once.
- Bad: the second fallback is a path inside the source tree. It suits
  development and CI, but a deployed binary must set `FREVO_UCUM_ESSENCE` or
  call `install_ucum_essence`. `add-server-skeleton` has to do one of the two.
- Bad: a process cannot run two different UCUM data sets at once. No HAPI
  configuration does either.

## What would change this

A consumer that needs different UCUM data in one process, or a server
configuration surface that owns the path. Either would move this to option 1,
with the data held by the evaluator.
