# Input package pins

`frevo-codegen` generates `frevo-model` from a published FHIR definition
package. The package is **fetched, not vendored**: it is 12 MB compressed and
31 MB extracted, and `design.md` section 2 records the discipline — fetched once
by a script into a git-ignored directory and verified against a SHA-256 recorded
here.

`reference/` is the **cross-check**, never the input. A generator that read it
would make the model unreproducible for anyone who had not run
`reference/clone.sh` (`codegen.inputs.004`).

## The pin

| | |
|---|---|
| Package | `hl7.fhir.r4.core` |
| Version | `4.0.1` |
| `fhirVersion` | `4.0.1` |
| Licence | CC0-1.0, as `package.json` declares |
| Registry | `https://packages.fhir.org/hl7.fhir.r4.core/4.0.1` |
| Mirror | `https://packages.simplifier.net/hl7.fhir.r4.core/4.0.1` |
| Archive SHA-256 | `ebd7731df7d36b5b7d39d5fb6c9d77b44bb7fe5742f1a2e87f164738c3289d44` |
| Content SHA-256 | `8af9fe30008c2a02c351728c198759141b05482f91dee63475777aa23f06354c` |
| Content covers | 4,583 files, 31,260,763 bytes |
| Extracted to | `crates/frevo-codegen/packages/fetched/hl7.fhir.r4.core-4.0.1/package/` |

Both registries served byte-identical archives when the pin was taken, which is
why the mirror is recorded: either host satisfies the archive digest.

### Why two digests

They verify different things, and only together do they cover the path from the
registry to the generator's input.

- **Archive SHA-256** is the digest of the `.tgz` as the registry serves it.
  `fetch.sh` checks it before extracting, so a substituted or truncated download
  fails with nothing unpacked. It is also the value a reviewer can reproduce
  with `curl … | sha256sum` against the published artifact.
- **Content SHA-256** is the digest of the extracted definition directory. The
  generator checks that one on every run (`codegen.inputs.002`), because it is
  the digest of what the generator actually reads: an archive digest says
  nothing about a tree somebody edited after extracting it.
  `frevo-codegen verify` exposes the same check on its own.

`fetch.sh` deliberately does not recompute the content digest. Expressing its
canonical manifest a second time, in shell, would mean two definitions of one
digest that could disagree; the one that matters is the one the generator
enforces. What the script does check is the archive digest, the package version
and the extracted file count.

The content digest is defined by `frevo_codegen::package::content_digest`. For
every regular file below the package directory, sorted by its relative path's
bytes:

```text
update(relative path, with / separators, as UTF-8)
update(b"\n")
update(byte length, as 8 big-endian bytes)
update(contents)
```

The length prefix stops a rename plus a concatenation from colliding with the
original. The separator is normalised so Windows and Linux agree.

## Fetching

```bash
bash crates/frevo-codegen/packages/fetch.sh             # fetch and verify
bash crates/frevo-codegen/packages/fetch.sh --verify    # verify what is there
bash crates/frevo-codegen/packages/fetch.sh --list      # print the pin

cargo run -q -p frevo-codegen --bin frevo-codegen -- verify   # content digest
```

The script is idempotent: an extracted, digest-matching package is left alone.
`fetched/` is git-ignored (`packages/.gitignore`).

To re-pin, delete `fetched/`, update both digests in this table and in
`crates/frevo-codegen/src/pin.rs`, and say in the same commit what changed
upstream. Moving this pin changes the definition of correct for every generated
file, so it is a reviewed change and not a routine bump.

## What the package contains, and what is scoped out

The published package is not split the way HAPI's copy is. It ships 658
`StructureDefinition`s and 1,400 `SearchParameter`s in one flat directory,
because it also carries the R4 extension registry, four logical models that are
documentation rather than definitions, and a handful of example parameters.

`frevo_codegen::inventory` documents the scoping rule that recovers the
specification's own 256 definitions and 1,375 search parameters from that, and
`codegen.inputs.005` asserts the resulting URL set against HAPI's
`profile/*.xml` at `7ab4c7c7c49281496faa1b3c2498a652457669ae`.
