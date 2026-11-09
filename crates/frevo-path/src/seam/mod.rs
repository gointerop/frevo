//! The evaluation seam: three traits plus the owned value type.
//!
//! ADR 0004 decision 5 requires `frevo-path` to evaluate over a trait, not
//! over `frevo-model`. The lesson it draws from `haste-fhirpath` is specific:
//! 77 `downcast_ref` calls to one model's concrete types, and FHIRPath
//! semantics turn a failed downcast into an empty collection rather than an
//! error.
//!
//! The seam is split in three, mirroring how `FHIRPathEngine` is split between
//! `Base`, `IWorkerContext` and `IHostApplicationServices`, because that split
//! is what makes the engine testable without a model.
//!
//! The dependency edge runs **from** `frevo-model` **to** here
//! (`design.md` section 4.5): `path.api.008` makes it a test that
//! `frevo-path`'s manifest and sources never name `frevo-model`.
//!
//! This module is definitions only. Task 16 of
//! `add-fhirpath-and-codegen-r4` adds the seam; tasks 28 onward add the engine
//! behind it.

pub mod host;
pub mod model;
pub mod node;
pub mod value;
