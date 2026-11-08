# 1. Development toolchain and build targets

- **Status:** accepted
- **Date:** 2026-11-08
- **Context phase:** Phase 0 (bootstrap)

## Context

Frevo ships as a Linux container image built from a static binary, and CI runs
on Linux. Development, however, happens on a Windows host.

Rust's default host triple on Windows is `x86_64-pc-windows-msvc`, which
requires the Visual Studio C++ build tools to link. On the bootstrap machine
those are absent: no `link.exe`, no `cl.exe`, no Visual Studio installation. The
first `cargo check` failed at link time, and — because Git Bash puts GNU
coreutils `link` on `PATH` — the failure surfaced as a confusing
`link: extra operand` error rather than a missing-linker error.

Three options were available:

1. Install Visual Studio Build Tools (the VC++ workload). Standard on Windows,
   but several gigabytes of toolchain that nothing else in this project needs.
2. Use the `x86_64-pc-windows-gnu` toolchain, which rustup ships with its own
   bundled MinGW linker. Tier 1, roughly 400 MB, needs no administrator rights.
3. Do all Rust work inside a Linux container, matching CI exactly.

The deciding constraint is hook latency. `post-edit.sh` runs `cargo fmt -p <c>`
and `cargo clippy -p <c> -- -D warnings` after every edit to a crate. Routing
each of those through a container adds seconds of startup to an inner loop that
runs hundreds of times a session, which would push agents toward disabling the
hook — defeating the enforcement this project depends on.

## Decision

1. **Local development on Windows uses `x86_64-pc-windows-gnu`**, set as the
   rustup default host so that `rust-toolchain.toml` can stay portable:

   ```bash
   rustup toolchain install stable-x86_64-pc-windows-gnu
   rustup set default-host x86_64-pc-windows-gnu
   ```

   `rust-toolchain.toml` pins only `channel = "stable"` plus `rustfmt` and
   `clippy`, so it resolves correctly on every host.

2. **The authoritative targets are Linux.** CI builds and runs every gate on
   `x86_64-unknown-linux-gnu`; the released image is built for
   `x86_64-unknown-linux-musl` and `aarch64-unknown-linux-musl`. Cross targets
   are added inside the container build, not in `rust-toolchain.toml`, so a
   local checkout does not download standard libraries it cannot link.

3. **A green local build is not evidence.** Only the Linux CI result closes a
   gate. The local toolchain exists to make the inner loop fast, not to decide
   correctness.

4. **Invoke `cargo` so the correct linker wins.** Scripts that run `cargo` from
   a Git Bash environment must not inherit a `PATH` where `/usr/bin/link`
   precedes the real linker. Gate and hook scripts therefore call `cargo`
   directly rather than through a shell that re-orders `PATH`, and CI runs on
   Linux where the collision cannot occur.

## Consequences

Good:

- No multi-gigabyte IDE dependency for contributors on Windows.
- Per-edit `fmt` and `clippy` stay fast enough that hooks remain tolerable.
- `rust-toolchain.toml` stays host-neutral, so Linux and macOS contributors need
  no special setup.

Bad, and accepted:

- The local target differs from the shipping target. A crate that behaves
  differently under MinGW than under glibc or musl would be caught only in CI.
  This is mitigated by requiring CI for every gate, and is expected to matter
  mainly once `frevo-store` links a PostgreSQL client and `frevo-search`
  links `tantivy`.
- Contributors who already have MSVC installed will use a third configuration.
  CI remains the arbiter, so this is tolerable, but `--target` assumptions must
  stay out of committed scripts.

## Revisit when

- A dependency fails to build under `windows-gnu` but builds under `msvc`, or
- the differential harness becomes fast enough to run locally in containers
  anyway, at which point option 3 becomes attractive and would remove the
  host-target divergence entirely.
