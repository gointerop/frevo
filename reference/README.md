# reference/

Read-only upstream sources, pinned in `PINS.md` and fetched by `clone.sh`.

Rules (enforced by `.claude/hooks/protect-paths.sh` and by `openspec-guard`):

- Never edit anything under this directory.
- Never add it to the Cargo workspace or compile it.
- Nothing here is committed; `.gitignore` keeps only `PINS.md`, `clone.sh` and
  this file.
- When a crate's source is derived from a file here, record it in
  `docs/provenance.md` and put a provenance header in the derived file.
