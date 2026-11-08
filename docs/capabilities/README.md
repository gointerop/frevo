# Capability register

One entry per crate: what the capability is for, which upstream module defines
its behaviour, which oracle will grade it, and what is still unknown.

These are **not** specifications. They carry no requirements, and nothing here
is binding. Requirements live in `openspec/specs/<capability>/spec.md` and get
there in exactly one way:

```
openspec/changes/<id>/   proposal, spec delta, design, tasks
      -> reviewed (review.md: APPROVED)
      -> tests written first
      -> implemented
      -> gates green
      -> openspec archive <id>      <-- the only writer of openspec/specs/
```

That is why this register lives under `docs/`. A requirement-less file in
`openspec/specs/` would fail `openspec validate --strict`, and hand-editing that
directory is blocked by `.claude/hooks/protect-paths.sh` and by the
`openspec-guard` CI job.

The "open questions" in each entry are the decisions a proposer has to resolve,
with evidence from `reference/`, before that capability can be specified. They
are the most useful thing in this register: each one is a place where guessing
would produce a parity bug.

| Capability | Crate | Phase |
|---|---|---|
| [codegen](codegen.md) | `frevo-codegen` | 1 |
| [model](model.md) | `frevo-model` | 1 |
| [path](path.md) | `frevo-path` | 1 |
| [store](store.md) | `frevo-store` | 2 |
| [search](search.md) | `frevo-search` | 3 |
| [validator](validator.md) | `frevo-validator` | 4 |
| [terminology](terminology.md) | `frevo-terminology` | 4 |
| [package](package.md) | `frevo-package` | 4 |
| [jobs](jobs.md) | `frevo-jobs` | 5 |
| [subscription](subscription.md) | `frevo-subscription` | 6 |
| [mdm](mdm.md) | `frevo-mdm` | 6 |
| [cql](cql.md) | `frevo-cql` | 7 |
| [cr](cr.md) | `frevo-cr` | 8 |
| [convert](convert.md) | `frevo-convert` | 9 |
| [ips](ips.md) | `frevo-ips` | 9 |
| [graphql](graphql.md) | `frevo-graphql` | 9 |
| [hfql](hfql.md) | `frevo-hfql` | 9 |
| [narrative](narrative.md) | `frevo-narrative` | 9 |
| [openapi](openapi.md) | `frevo-openapi` | 9 |
| [mcp](mcp.md) | `frevo-mcp` | 9 |
| [config](config.md) | `frevo-config` | 1 |
| [server](server.md) | `frevo-server` | 1 |

Phases follow the dependency order in `tasks/graph.yaml`.
