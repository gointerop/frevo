# Changelog

## 0.1.0 (2026-11-12)


### Features

* **bench,tasks:** HAPI 8.12.0 baseline, proposed targets, and the work queue ([f3a25de](https://github.com/gointerop/frevo/commit/f3a25dedbe731481df88ed807879a995faa4f2fc))
* **bench:** medium baseline at 1,128 patients, and a method defect it exposed ([#10](https://github.com/gointerop/frevo/issues/10)) ([f739cbe](https://github.com/gointerop/frevo/commit/f739cbe5fb86a00b64f21777625a98f8628739f3))
* **docs:** a technical-debt register, and the owner decision that needs it ([#11](https://github.com/gointerop/frevo/issues/11)) ([52903ba](https://github.com/gointerop/frevo/commit/52903ba2ba9003f710d4ee7e0700aebaf7e59c17))
* **harness:** differential test harness v0, self-test passing at zero diffs ([a50f508](https://github.com/gointerop/frevo/commit/a50f50834f079ae291db41326c90b7c1e5b42602))
* **inventory:** extract the 454-row parity checklist, and fix three CI failures ([eb321cb](https://github.com/gointerop/frevo/commit/eb321cb0b52e3ea036bd7a8cba0da4a627188704))
* **oracles:** pin hl7.fhir.r4.examples[#4](https://github.com/gointerop/frevo/issues/4).0.1 as the round-trip oracle ([#13](https://github.com/gointerop/frevo/issues/13)) ([1098b31](https://github.com/gointerop/frevo/commit/1098b31c01ab0e6c5ef6e48437a1ff2332a486e0))
* **oracles:** vendor and pin the HL7 and CQL suites, with licence findings ([3c71657](https://github.com/gointerop/frevo/commit/3c716573d5383936a90449d6ba5a80c51927f993))


### Bug Fixes

* **ci,harness:** repair the release workflow and a latent differential flake ([0e71a21](https://github.com/gointerop/frevo/commit/0e71a21c65a5e1128b5b652fb2e9d66e7c946875))
* **ci:** authenticate the vulnerability scan to GHCR ([7c388be](https://github.com/gointerop/frevo/commit/7c388be0063ddfc9f1150672491a25b0737e682c))
* **ci:** stop release-please proposing 1.0.0 for a project with no features ([f9c25ef](https://github.com/gointerop/frevo/commit/f9c25ef58ccb29d221213b1636d0032f5fa218f5))
* **hooks:** a block message must not execute anything ([#21](https://github.com/gointerop/frevo/issues/21)) ([9d97bdc](https://github.com/gointerop/frevo/commit/9d97bdc20217297a14f21735419c5065475ebbd3))
* **hooks:** judge each edit in the worktree that contains the file ([#22](https://github.com/gointerop/frevo/issues/22)) ([769b7a5](https://github.com/gointerop/frevo/commit/769b7a56e75656cd1cca27be16010f77906c81af))
* **hooks:** resolve the repository root per worktree, and stop failing open ([#17](https://github.com/gointerop/frevo/issues/17)) ([3efbc97](https://github.com/gointerop/frevo/commit/3efbc97a49d87ca205f8f6032e9011bc0109b77d))
* **hooks:** scope the generated-code rule to the generated directories ([#20](https://github.com/gointerop/frevo/issues/20)) ([b72fb63](https://github.com/gointerop/frevo/commit/b72fb63f55311bef8a3f6dcf104aed1b177db879))
* **oracles:** count FHIRPath cases by parsing, not by matching text ([e3784c1](https://github.com/gointerop/frevo/commit/e3784c104d6904f41be215e920f1159701349246))
* **tasks:** mark add-fhirpath-and-codegen-r4 in_progress ([#18](https://github.com/gointerop/frevo/issues/18)) ([97edd8b](https://github.com/gointerop/frevo/commit/97edd8bb1668501f436461abccca017a5be4320b))
* **tech-debt:** add DEBT-0010, and catch the id collision that nearly hid it ([#15](https://github.com/gointerop/frevo/issues/15)) ([35a1041](https://github.com/gointerop/frevo/commit/35a10413bad43088c7148e3648b31c64139777a0))
