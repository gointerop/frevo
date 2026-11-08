# Frevo container image.
#
# Two stages. The builder produces a statically linked musl binary; the final
# stage is distroless, so it has no shell, no package manager and no libc to
# attack, and runs as a non-root user. The image therefore contains exactly one
# executable file plus CA certificates.
#
# Built per-platform by buildx (linux/amd64 and linux/arm64). Each platform
# compiles natively for its own musl target rather than cross-linking, which
# keeps the build honest at the cost of emulation time on the non-native arch.

# ---------------------------------------------------------------------------
# Builder
# ---------------------------------------------------------------------------
# No --platform here on purpose: buildx runs this stage under the *target*
# platform, so each architecture compiles its own binary for its own musl
# target. Pinning it to BUILDPLATFORM would silently ship the host's
# architecture under both tags.
FROM rust:1.99-alpine3.22 AS builder

# musl-dev provides the static libc; pkgconfig is needed by common build
# scripts; binutils provides readelf, used below to prove the output is static.
RUN apk add --no-cache musl-dev pkgconfig binutils file

WORKDIR /src

# Static linking. The official rust:alpine image sets
# RUSTFLAGS=-Ctarget-feature=-crt-static so that proc macros and dylibs can be
# built, which means its default output is *dynamically* linked against musl
# and would not run in a distroless image. We override it.
#
# This is also why every cargo invocation below passes an explicit --target:
# with a target specified, RUSTFLAGS applies only to target artifacts, leaving
# build scripts and proc macros to be built for the host without crt-static.
# Without --target the proc-macro crates would fail to link.
ENV RUSTFLAGS="-C target-feature=+crt-static"

# Dependency layer: build the dependency graph against stub sources so it can be
# reused across code changes. That matters because the arm64 build runs under
# emulation and is slow.
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN set -eux; \
    rustc -vV | sed -n 's/^host: //p' > /target-triple; \
    TARGET="$(cat /target-triple)"; \
    find crates -path '*/src/*' -name '*.rs' -delete; \
    for d in crates/*/; do \
      name=$(basename "$d"); \
      mkdir -p "$d/src"; \
      if [ "$name" = "frevo-server" ]; then \
        echo 'fn main() {}' > "$d/src/main.rs"; \
      else \
        : > "$d/src/lib.rs"; \
      fi; \
    done; \
    cargo build --profile dist --locked --target "$TARGET" -p frevo-server; \
    rm -rf crates

# Real sources.
COPY crates ./crates
RUN set -eux; \
    TARGET="$(cat /target-triple)"; \
    # Docker COPY preserves the build context's mtimes, which can be OLDER than
    # the stub files written above. Cargo would then judge the real sources
    # unchanged and ship the stub binary. Dropping our own fingerprints forces
    # the workspace crates to recompile while keeping dependency artifacts.
    rm -rf "target/$TARGET/dist/.fingerprint/"frevo-*; \
    cargo build --profile dist --locked --target "$TARGET" -p frevo-server; \
    bin="target/$TARGET/dist/frevo"; \
    \
    # Assert the binary is really static. A dynamically linked binary would not
    # run in the distroless stage, and the failure would only appear at runtime.
    #
    # Do NOT use ldd for this: musl's ldd prints the loader path for a
    # static-pie executable too, so its output looks dynamic when it is not.
    # The meaningful test is the ELF headers -- a static binary has no
    # PT_INTERP segment and no DT_NEEDED entries.
    if readelf -l "$bin" 2>/dev/null | grep -q 'INTERP'; then \
      echo "FATAL: $bin has a program interpreter, so it is not static:"; \
      readelf -l "$bin" | grep -A1 INTERP; exit 1; \
    fi; \
    if readelf -d "$bin" 2>/dev/null | grep -q 'NEEDED'; then \
      echo "FATAL: $bin has shared library dependencies:"; \
      readelf -d "$bin" | grep NEEDED; exit 1; \
    fi; \
    echo "verified: $bin is statically linked ($(file -b "$bin" 2>/dev/null || echo 'static-pie'))"; \
    \
    # Assert this is the real binary and not the dependency-layer stub. The
    # stub has no CapabilityStatement in it; the real one does.
    if ! grep -q 'CapabilityStatement' "$bin"; then \
      echo "FATAL: $bin looks like the dependency-layer stub, not the real server"; exit 1; \
    fi; \
    \
    install -m 0555 "$bin" /frevo; \
    echo "built frevo -> $(stat -c %s /frevo) bytes"

# ---------------------------------------------------------------------------
# Final image: distroless, non-root, no shell.
# ---------------------------------------------------------------------------
FROM gcr.io/distroless/static-debian12:nonroot

COPY --from=builder /frevo /frevo

# 65532 is distroless "nonroot".
USER 65532:65532

EXPOSE 8080
ENV FREVO_PORT=8080 \
    RUST_LOG=info

# No shell in this image, so readiness is probed over HTTP from outside
# (GET /healthz) by compose, Helm or the orchestrator, not by a shell command
# in here.

ENTRYPOINT ["/frevo"]

# OCI metadata. The source and revision labels are what links a running
# container back to the commit that produced it.
LABEL org.opencontainers.image.title="Frevo" \
      org.opencontainers.image.description="A high-performance HL7 FHIR server in Rust on PostgreSQL, compatible with HAPI FHIR JPA Server Starter configuration." \
      org.opencontainers.image.source="https://github.com/gointerop/frevo" \
      org.opencontainers.image.licenses="Apache-2.0" \
      org.opencontainers.image.vendor="The Frevo authors" \
      org.opencontainers.image.base.name="gcr.io/distroless/static-debian12:nonroot"
