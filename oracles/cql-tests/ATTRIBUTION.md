# Attribution: CQL test suite

`tests/` is vendored unmodified from:

- Repository: <https://github.com/cqframework/cql-tests>
- Commit: `5747d2b72f29d733fbc114383c4a15a30999ffaf`
- Licence: Apache License, Version 2.0 (`LICENSE` in that repository)
- Copyright: The CQL Framework contributors

Licensed under the Apache License, Version 2.0. You may obtain a copy of the
License at <http://www.apache.org/licenses/LICENSE-2.0>.

**Unmodified.** These are oracle cases; `oracles/fetch.sh --verify` re-downloads
them from the pinned commit and fails on any difference.

The reference CQL engine does not pass every case in this suite. The gate is
therefore **parity with the reference engine on the same cases**, not an
absolute pass rate.
