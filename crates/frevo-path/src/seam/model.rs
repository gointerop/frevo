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

/// One child element of a type, described the way the static check pass reads
/// a StructureDefinition snapshot (`getChildTypesByName`,
/// `FHIRPathEngine.java:6322`).
///
/// Two type lists are carried because upstream reads two different things in
/// two different places, and they disagree for the elements that matter most:
/// navigation by name reads `TypeRefComponent.getWorkingCode()` — `Element.id`
/// is a `string` — while `children()` and `descendants()` read the declared
/// `getCode()` — `Element.id` is a `System.String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementInfo {
    /// The element name, `[x]` removed.
    pub name: String,
    /// The element's path in the definition that declares it, as
    /// `Patient.contact`. A backbone element's type is named by this path.
    pub path: String,
    /// The types navigation by name yields, in declared order: the working
    /// code of each type; a backbone or `Element`-typed element with children
    /// as its own path; a content reference as the referenced element's path;
    /// a `Resource`-typed element as `Resource`.
    pub types: Vec<String>,
    /// The declared type codes, unmapped: `BackboneElement`, `Element` and
    /// `Resource` as written, and a FHIRPath System type as `System.String`.
    /// Empty for a content reference, which declares no type.
    pub codes: Vec<String>,
    /// Whether the element is a choice (`value[x]`).
    pub choice: bool,
    /// Whether the element repeats (`ElementDefinition.unbounded()`: a `max`
    /// of `*` or above 1).
    pub repeating: bool,
    /// For a content reference, the referenced element's type name.
    pub content_reference: Option<String>,
}

/// The type information the check pass and the `is`/`as`/`ofType` operators
/// need. It is the whole input to the static check pass, which is why the check
/// pass can be tested with a stub `Model` and no resource at all.
///
/// The methods after [`Model::derives_from`] are what the check pass needs
/// beyond evaluation, and each defaults to `None`, meaning **this model cannot
/// say**. The check pass answers a question it needs one of them for with
/// `CheckError::NotImplemented`, never with a guess: a model that cannot list
/// its elements does not get every name rejected, or every name accepted.
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

    /// The direct child elements of `type_name`, in snapshot order.
    /// `type_name` may be a backbone type named by its path
    /// (`Patient.contact`). `None` when the model cannot describe them.
    fn elements(&self, _type_name: &str) -> Option<Vec<ElementInfo>> {
        None
    }

    /// The types whose definition specialises `type_name` directly — a base
    /// definition of `type_name` and a derivation of `specialization`; a
    /// constraint profile is not one. Upstream's `addTypeAndDescendents`
    /// walks these, so `resolve().name` looks for `name` in every resource.
    /// `None` when the model cannot enumerate them.
    fn specializations(&self, _type_name: &str) -> Option<Vec<String>> {
        None
    }

    /// `IWorkerContext.getResourceNames()`: every resource type whose
    /// derivation is `specialization`, sorted — which excludes `Resource`
    /// itself, whose definition has no derivation. A `Resource`-typed element
    /// is checked as all of them.
    fn resource_type_names(&self) -> Option<Vec<String>> {
        None
    }

    /// Every non-logical type whose derivation is `specialization`: what
    /// upstream's `anything()` answers for `$total` and `aggregate()`.
    fn type_names(&self) -> Option<Vec<String>> {
        None
    }

    /// Whether a StructureDefinition with canonical URL `url` exists, for
    /// `extension(url)`. `None` when the model cannot say.
    fn has_definition_url(&self, _url: &str) -> Option<bool> {
        None
    }

    /// The value types the extension defined at canonical URL `url` allows,
    /// in declared order: what `findExtensionDefinition` finds and the
    /// profiled `Extension` type it yields lets `.value` be typed as
    /// (`FHIRPathEngine.java:3556`-`:3591`, `:3747`). `None` when the model
    /// cannot say or has no such definition; the check pass then keeps the
    /// extension a plain `Extension`, whose `value` is the whole choice.
    /// Defaulted, so a model that does not carry extension definitions still
    /// compiles.
    fn extension_value_types(&self, _url: &str) -> Option<Vec<String>> {
        None
    }
}
