//! The lazy, byte-preserving JSON codec.
//!
//! `design.md` section 3 and section 9 of `add-fhirpath-and-codegen-r4`. A
//! resource is one of two things, and callers cannot tell which
//! (`model.bytes.006`):
//!
//! ```text
//! Resource
//! ├── Stored   { bytes: Arc<[u8]>, lazily built index, recorded writes }
//! └── Built    { owned element tree }
//! ```
//!
//! The first principle of the constitution's performance rules is the reason
//! the split exists: a read that returns a resource unchanged must not parse or
//! re-serialize it. [`Resource::write_to`] on an unedited [`Stored`] is one
//! `write_all` of the buffer the resource was loaded with, and
//! [`Resource::untouched_bytes`] hands that buffer back by `Arc` clone, so the
//! server can pass it to a socket without the codec having looked at a single
//! JSON token.

pub mod built;
pub mod decode;
pub mod doc;
pub mod nav;
pub mod path;
pub mod provisional_r4;
pub mod scan;
pub mod stored;
pub mod tables;

#[cfg(test)]
mod tests;

use std::io;
use std::sync::Arc;

pub use built::Built;
pub use nav::{R4Model, ResourceNode};
pub use scan::{DecodeError, Meter, Span};
pub use stored::Stored;
pub use tables::{ElementDef, JsonForm, Tables, TypeDef, TypeKind};

/// The tables the codec reads: `frevo-codegen`'s generated R4 output.
///
/// Tasks 7 to 14 landed `crates/frevo-model/src/r4/`, so this is now the real
/// thing -- 685 types and 7,500 elements read from
/// `hl7.fhir.r4.core#4.0.1` -- rather than the hand-written stand-in that
/// preceded it. The generated set was compared programmatically against that
/// stand-in before the switch: it covers all 70 of its types and all 191 of
/// their elements, and differs in three ways, each of which the stand-in had
/// wrong. `Extension.value[x]` carries all **50** open types rather than 40,
/// so a `valueDosage` or a `valueMeta` now decodes at all; `Timing.repeat` is
/// the named nested type `Timing.repeat` rather than a bare
/// `BackboneElement`, so `Timing.repeat.frequency` is navigable; and the root
/// primitives carry `base: Some("Element")`, which is what the
/// StructureDefinition says.
///
/// [`provisional_r4`] is kept as the fixture the codec's own unit tests read,
/// so a change to the generated table cannot quietly rewrite what those tests
/// assert, and so the comparison above stays runnable.
pub const R4: Tables = crate::r4::types::TABLES;

/// The hand-written stand-in the codec was first written against.
pub const PROVISIONAL_R4: Tables = provisional_r4::TABLES;

/// What a run looked at, as the `model.bytes.*`, `model.lazy.*` and
/// `model.perf.*` scenarios state it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// JSON tokens scanned. Zero for a load-and-serialise-unchanged.
    pub tokens_scanned: u64,
    /// Heap allocations whose size scaled with the document. Zero for a
    /// load-and-serialise-unchanged: an `Arc` clone is not proportional to the
    /// document.
    pub size_proportional_allocations: u64,
    /// Every element path that was materialised, in the order it happened.
    pub materialised: Vec<String>,
    /// Whether any narrative XHTML was parsed or validated.
    pub xhtml_validated: bool,
}

#[derive(Debug)]
enum Inner {
    Stored(Arc<Stored>),
    Built(Built),
}

/// A FHIR resource, byte-backed or constructed.
#[derive(Debug)]
pub struct Resource {
    inner: Inner,
    meter: Arc<Meter>,
}

impl Resource {
    /// Load from stored canonical JSON bytes. **Nothing is scanned here.** The
    /// buffer is taken by `Arc`, not copied, which is what makes
    /// `model.bytes.002`'s zero-allocation requirement reachable: in the server
    /// the buffer arrives owned from the database driver.
    pub fn from_stored(bytes: Arc<[u8]>) -> Self {
        let meter = Arc::new(Meter::new());
        Self {
            inner: Inner::Stored(Arc::new(Stored::new(bytes, R4, Arc::clone(&meter)))),
            meter,
        }
    }

    /// Load from stored bytes with the materialised-path log turned off.
    ///
    /// Every counter still works; only the per-element path `String` is not
    /// built. This is the constructor the `bench/micro/` workloads use, because
    /// `model.perf.003` budgets the decode and not the instrumentation of it.
    pub fn from_stored_untracked(bytes: Arc<[u8]>) -> Self {
        let meter = Arc::new(Meter::untracked());
        Self {
            inner: Inner::Stored(Arc::new(Stored::new(bytes, R4, Arc::clone(&meter)))),
            meter,
        }
    }

    /// Construct in memory rather than decoding. Serialising the result
    /// consults the generated element order (`model.bytes.005`).
    pub fn build(resource_type: &str) -> Self {
        let meter = Arc::new(Meter::new());
        Self {
            inner: Inner::Built(Built::new(resource_type, R4)),
            meter,
        }
    }

    pub fn meter(&self) -> &Arc<Meter> {
        &self.meter
    }

    pub fn report(&self) -> Report {
        Report {
            tokens_scanned: self.meter.tokens_scanned(),
            size_proportional_allocations: self.meter.allocations(),
            materialised: self.meter.materialised_paths(),
            xhtml_validated: self.meter.xhtml_was_validated(),
        }
    }

    pub fn root_type(&self) -> Result<String, DecodeError> {
        match &self.inner {
            Inner::Stored(stored) => stored.root_type(),
            Inner::Built(built) => Ok(built.root_type().to_string()),
        }
    }

    /// The stored buffer, when the resource is still exactly its bytes. `None`
    /// for a constructed resource and once a write has happened.
    pub fn untouched_bytes(&self) -> Option<Arc<[u8]>> {
        match &self.inner {
            Inner::Stored(stored) => stored.untouched_bytes(),
            Inner::Built(_) => None,
        }
    }

    /// Whether the resource differs from the bytes it was loaded with. A
    /// resource that was never bytes has nothing to differ from, so it reports
    /// unmodified.
    pub fn is_modified(&self) -> bool {
        match &self.inner {
            Inner::Stored(stored) => stored.is_modified(),
            Inner::Built(_) => false,
        }
    }

    /// Read the primitive at `path`: the concrete type name and the lexical
    /// value, which is `None` for an element that exists without one.
    pub fn read(&self, path: &str) -> Result<Option<(String, Option<String>)>, DecodeError> {
        match &self.inner {
            Inner::Stored(stored) => stored.read(path),
            Inner::Built(built) => built.read(path),
        }
    }

    /// How many repetitions `path` holds, without materialising any of them.
    pub fn count(&self, path: &str) -> Result<usize, DecodeError> {
        match &self.inner {
            Inner::Stored(stored) => stored.count(path),
            Inner::Built(built) => built.count(path),
        }
    }

    /// Write `lexical` to the primitive at `path`.
    pub fn set(&mut self, path: &str, lexical: &str) -> Result<(), DecodeError> {
        match &mut self.inner {
            Inner::Stored(stored) => stored.set(path, lexical),
            Inner::Built(built) => built.set(path, lexical),
        }
    }

    /// Decode the whole document. The `model-full-parse` workload.
    pub fn materialise_all(&self) -> Result<(), DecodeError> {
        match &self.inner {
            Inner::Stored(stored) => stored.materialise_all(),
            Inner::Built(_) => Ok(()),
        }
    }

    /// Serialise in the stored form: the original bytes when nothing was
    /// written, the generated element order for a constructed resource.
    pub fn write_to(&self, out: &mut dyn io::Write) -> io::Result<()> {
        match &self.inner {
            Inner::Stored(stored) => stored.write_to(out),
            Inner::Built(built) => {
                let mut buffer = Vec::new();
                self.meter.size_proportional_allocation();
                built.write(&mut buffer);
                out.write_all(&buffer)
            }
        }
    }

    /// Serialise canonically: property names sorted lexicographically, no
    /// whitespace.
    pub fn write_canonical(&self, out: &mut dyn io::Write) -> io::Result<()> {
        match &self.inner {
            Inner::Stored(stored) => stored.write_canonical(out),
            Inner::Built(built) => {
                let mut buffer = Vec::new();
                self.meter.size_proportional_allocation();
                built.write_canonical(&mut buffer);
                out.write_all(&buffer)
            }
        }
    }

    /// How many bytes the stored form occupies, without producing it.
    pub fn stored_len(&self) -> Option<usize> {
        match &self.inner {
            Inner::Stored(stored) => Some(stored.stored_len()),
            Inner::Built(_) => None,
        }
    }

    /// The stored form as bytes. The buffer belongs to the caller, not to the
    /// codec: the server writes to a socket and never needs this, which is why
    /// the allocation is not counted against `model.bytes.002`.
    pub fn to_stored_vec(&self) -> io::Result<Vec<u8>> {
        let mut out = Vec::with_capacity(self.stored_len().unwrap_or(0));
        self.write_to(&mut out)?;
        Ok(out)
    }

    /// The canonical form as bytes.
    pub fn to_canonical_vec(&self) -> io::Result<Vec<u8>> {
        let mut out = Vec::with_capacity(self.stored_len().unwrap_or(1024));
        self.write_canonical(&mut out)?;
        Ok(out)
    }

    /// The byte-backed document behind this resource, when there is one.
    pub fn stored(&self) -> Option<&Arc<Stored>> {
        match &self.inner {
            Inner::Stored(stored) => Some(stored),
            Inner::Built(_) => None,
        }
    }

    /// The constructed tree behind this resource, when there is one.
    pub fn built(&self) -> Option<&Built> {
        match &self.inner {
            Inner::Built(built) => Some(built),
            Inner::Stored(_) => None,
        }
    }

    /// A navigable handle on the root, for `frevo-path` to evaluate over.
    pub fn as_node(&self) -> Result<ResourceNode, DecodeError> {
        let stored = self.stored().ok_or_else(|| {
            DecodeError::new(
                "",
                0,
                "only a byte-backed resource can hand out a span-based node yet",
            )
        })?;
        ResourceNode::root(Arc::clone(stored))
    }
}
