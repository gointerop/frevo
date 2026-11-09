//! `Model` — type information.
//!
//! `design.md` section 4.3 of `add-fhirpath-and-codegen-r4`. Definitions only.

use std::fmt;

/// A set of FHIR or System type names, as the check pass propagates them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TypeSet(Vec<String>);

impl TypeSet {
    pub fn empty() -> Self {
        Self(Vec::new())
    }

    pub fn of(name: impl Into<String>) -> Self {
        Self(vec![name.into()])
    }

    pub fn from_iter_names<I, S>(names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut set = Self::empty();
        for name in names {
            set.insert(name);
        }
        set
    }

    /// Add `name` if it is not already present, preserving insertion order:
    /// the declared order of a choice element is observable in diagnostics.
    pub fn insert(&mut self, name: impl Into<String>) {
        let name = name.into();
        if !self.0.contains(&name) {
            self.0.push(name);
        }
    }

    pub fn contains(&self, name: &str) -> bool {
        self.0.iter().any(|existing| existing == name)
    }

    pub fn names(&self) -> &[String] {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl fmt::Display for TypeSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.join(", "))
    }
}

/// The type information the check pass and the `is`/`as`/`ofType` operators
/// need. It is the whole input to the static check pass, which is why the check
/// pass can be tested with a stub `Model` and no resource at all.
pub trait Model: Send + Sync {
    /// Whether `type_name` is a type this model knows.
    fn is_type(&self, type_name: &str) -> bool;

    /// The immediate base type: `code` -> `string`, `Age` -> `Quantity`.
    fn base_type(&self, type_name: &str) -> Option<&str>;

    /// The declared types of an element. For a choice element, the full choice
    /// set, in declared order.
    fn element_types(&self, type_name: &str, element: &str) -> Option<Vec<String>>;

    /// Whether `type_name` is a FHIR primitive type.
    fn is_primitive(&self, type_name: &str) -> bool;

    /// Whether `type_name` is a resource type.
    fn is_resource_type(&self, type_name: &str) -> bool;

    /// Whether `type_name` derives from `ancestor`, transitively. The default
    /// walks [`Model::base_type`], which is how the transitive answer is built
    /// from the immediate one.
    fn derives_from(&self, type_name: &str, ancestor: &str) -> bool {
        let mut current = type_name;
        let mut guard = 0usize;
        while let Some(base) = self.base_type(current) {
            if base == ancestor {
                return true;
            }
            current = base;
            guard += 1;
            if guard > 64 {
                // A cyclic hierarchy is a generator defect, not a runtime
                // condition; refusing to spin is the safe answer.
                return false;
            }
        }
        false
    }
}
