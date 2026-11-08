# Attribution: FHIRPath test suite

`tests-fhir-r4.xml` is vendored unmodified from:

- Repository: <https://github.com/FHIR/fhir-test-cases>
- Path: `r4/fhirpath/tests-fhir-r4.xml`
- Commit: `c95adbee099c875de97d25ed7616aae7d66dd85b`
- Licence: Apache License, Version 2.0 (`LICENSE.txt` in that repository)
- Copyright: Health Level Seven International and contributors

Licensed under the Apache License, Version 2.0. You may obtain a copy of the
License at <http://www.apache.org/licenses/LICENSE-2.0>.

**Unmodified.** This file is an oracle: it defines what correct means. Editing
it would move the standard we measure against, so `oracles/fetch.sh --verify`
re-downloads it from the pinned commit and fails on any difference.

Note on provenance: the FHIRPath specification repository `HL7/FHIRPath`
carries no licence file, so its copy of these tests is not redistributable.
This Apache-2.0 copy, from `FHIR/fhir-test-cases`, is used instead. See
[`../PINS.md`](../PINS.md).
