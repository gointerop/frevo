#!/usr/bin/env bash
# Entry point for the r4-example-roundtrip oracle suite. See oracles/run.sh for
# status and exit-code meaning (2 = did not run, which is NOT a pass).
exec bash "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/run.sh" "r4-example-roundtrip" "$@"
