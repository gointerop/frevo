//! The check pass's type sets: a port of `TypeDetails`.
//!
//! Derived from org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/TypeDetails.java
//! at 73c432b5f43ddbce979978a760ec4aa81e929094.
//! Original work Copyright HL7, Inc. and contributors, licensed under the
//! Apache License, Version 2.0.
//! Modified by the Frevo authors: types are named rather than held as
//! canonical URLs, the model is asked through the `Model` trait rather than an
//! `IWorkerContext`, and type profiles, bindings and reference targets are not
//! carried (see [`Types`]).
//!
//! # Naming
//!
//! Upstream holds every type as a URL. Here a type is a name, and the mapping
//! is one-to-one:
//!
//! | Upstream | Here |
//! |---|---|
//! | `http://hl7.org/fhir/StructureDefinition/Patient` | `Patient` |
//! | `http://hl7.org/fhir/StructureDefinition/Patient#Patient.contact` | `Patient.contact` |
//! | `http://hl7.org/fhirpath/System.String` | `System.String` |
//!
//! which is also the naming `frevo-model`'s tables use for backbone types.
//! [`url_of`] turns a name back into upstream's URL, so a diagnostic describes
//! a type set exactly as upstream's `describe()` does.

use std::collections::HashSet;

use crate::seam::model::{Model, TypeSet};

/// `TypeDetails.FP_NS` and `FHIR_NS`.
pub(crate) const FP_NS: &str = "http://hl7.org/fhirpath/";
pub(crate) const FHIR_NS: &str = "http://hl7.org/fhir/StructureDefinition/";

pub(crate) const SYSTEM_STRING: &str = "System.String";
pub(crate) const SYSTEM_BOOLEAN: &str = "System.Boolean";
pub(crate) const SYSTEM_INTEGER: &str = "System.Integer";
pub(crate) const SYSTEM_DECIMAL: &str = "System.Decimal";
pub(crate) const SYSTEM_QUANTITY: &str = "System.Quantity";
pub(crate) const SYSTEM_DATE_TIME: &str = "System.DateTime";
pub(crate) const SYSTEM_TIME: &str = "System.Time";
pub(crate) const SYSTEM_SIMPLE_TYPE_INFO: &str = "System.SimpleTypeInfo";
pub(crate) const SYSTEM_CLASS_INFO: &str = "System.ClassInfo";

/// The names `hasType(IWorkerContext, String...)` and `hasType(Set)` map to a
/// System type (`TypeDetails.java:248`, `:441`).
const SYSTEM_MAPPED: &[&str] = &[
    "boolean",
    "string",
    "integer",
    "decimal",
    "Quantity",
    "dateTime",
    "time",
    "ClassInfo",
    "SimpleTypeInfo",
];

/// The same list in `hasType(String)` (`TypeDetails.java:428`), which alone
/// also has `date`. There is no `System.Date` in any type set, so the extra
/// entry never matches, but the lists are kept apart because upstream's are.
const SYSTEM_MAPPED_PLAIN: &[&str] = &[
    "boolean",
    "string",
    "integer",
    "decimal",
    "Quantity",
    "date",
    "dateTime",
    "time",
    "ClassInfo",
    "SimpleTypeInfo",
];

/// The FHIR types whose StructureDefinition `hasType(IWorkerContext, ...)`
/// maps to a System type (`TypeDetails.java:256`).
const SD_SYSTEM_MAPPED: &[&str] = &[
    "boolean", "string", "integer", "decimal", "Quantity", "dateTime", "time",
];

/// The System counterparts of the FHIR primitive types the engine's
/// `primitiveTypes` set holds and `hasType(Set)` maps.
const SYSTEM_PRIMITIVES: &[&str] = &[
    SYSTEM_BOOLEAN,
    SYSTEM_STRING,
    SYSTEM_INTEGER,
    SYSTEM_DECIMAL,
    SYSTEM_DATE_TIME,
    SYSTEM_TIME,
];

/// Whether `name` is a FHIRPath System type.
pub(crate) fn is_system(name: &str) -> bool {
    name.starts_with("System.")
}

/// Whether `name` names a backbone type by its path (`Patient.contact`),
/// upstream's `sd#path` form.
pub(crate) fn is_backbone(name: &str) -> bool {
    !is_system(name) && !name.contains("://") && name.contains('.')
}

/// `Utilities.capitalize`.
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// `System.X` for a mapped name.
fn system_of(name: &str) -> String {
    format!("System.{}", capitalize(name))
}

/// Upstream's URL for a type name.
pub(crate) fn url_of(name: &str) -> String {
    if is_system(name) {
        format!("{FP_NS}{name}")
    } else if name.contains("://") {
        name.to_string()
    } else if is_backbone(name) {
        let owner = name.split('.').next().unwrap_or(name);
        format!("{FHIR_NS}{owner}#{name}")
    } else {
        format!("{FHIR_NS}{name}")
    }
}

/// `ExpressionNode.CollectionStatus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectionStatus {
    Singleton,
    Ordered,
    Unordered,
}

impl CollectionStatus {
    /// `CollectionStatus.isList()`.
    pub fn is_list(self) -> bool {
        self != CollectionStatus::Singleton
    }
}

/// `TypeDetails`: a set of types, a collection status, and the choice flag
/// the SQL-on-FHIR warning reads.
///
/// The status is an `Option` because upstream's is nullable and observably
/// so: `$index` is typed with no status at all, and `update` treats a null
/// status differently from a singleton one.
///
/// Not carried, and why that changes no verdict: type **profiles** and
/// **bindings** (upstream's `ProfiledType` extras), and reference **targets**.
/// None of the three is read by any check that can fail. Profiles are read
/// by `typeCastIsImpossible`, which only warns, and by `extension(url)` on a
/// relative URL, which this pass reports as not implemented; targets are only
/// copied along. They matter to the warning channel (task 50a).
#[derive(Debug, Clone, Default)]
pub(crate) struct Types {
    names: Vec<String>,
    index: HashSet<String>,
    status: Option<CollectionStatus>,
    choice: bool,
}

impl Types {
    /// `new TypeDetails(status)`.
    pub(crate) fn new(status: Option<CollectionStatus>) -> Self {
        Types {
            status,
            ..Types::default()
        }
    }

    /// `new TypeDetails(status, names...)`.
    pub(crate) fn of(status: CollectionStatus, names: &[&str]) -> Self {
        let mut t = Types::new(Some(status));
        for n in names {
            t.add(n);
        }
        t
    }

    /// `new TypeDetails(CollectionStatus.SINGLETON, names...)`.
    pub(crate) fn single(names: &[&str]) -> Self {
        Types::of(CollectionStatus::Singleton, names)
    }

    /// `TypeDetails.empty()`: a singleton with no types.
    pub(crate) fn empty() -> Self {
        Types::new(Some(CollectionStatus::Singleton))
    }

    pub(crate) fn status(&self) -> Option<CollectionStatus> {
        self.status
    }

    pub(crate) fn names(&self) -> &[String] {
        &self.names
    }

    pub(crate) fn first(&self) -> Option<&str> {
        self.names.first().map(String::as_str)
    }

    pub(crate) fn is_choice(&self) -> bool {
        self.choice
    }

    pub(crate) fn set_choice(&mut self) {
        self.choice = true;
    }

    /// `isList()`: a null status is not a list.
    pub(crate) fn is_list(&self) -> bool {
        self.status.is_some_and(CollectionStatus::is_list)
    }

    /// `hasNoTypes()` / `isEmpty()`.
    pub(crate) fn has_no_types(&self) -> bool {
        self.names.is_empty()
    }

    /// `addType(ProfiledType)`: add if absent, keeping first-insertion order.
    pub(crate) fn add(&mut self, name: &str) {
        if self.index.insert(name.to_string()) {
            self.names.push(name.to_string());
        }
    }

    /// `addType(CollectionStatus, ProfiledType)`.
    pub(crate) fn add_with_status(&mut self, status: CollectionStatus, name: &str) {
        self.add(name);
        match self.status {
            None => self.status = Some(status),
            Some(current) => match status {
                CollectionStatus::Ordered => {
                    if current == CollectionStatus::Singleton {
                        self.status = Some(status);
                    }
                }
                CollectionStatus::Singleton => {}
                CollectionStatus::Unordered => self.status = Some(status),
            },
        }
    }

    /// `typesContains`.
    pub(crate) fn contains(&self, name: &str) -> bool {
        self.index.contains(name)
    }

    /// `update(TypeDetails)`.
    pub(crate) fn update(&mut self, source: &Types) {
        for n in &source.names {
            self.add(n);
        }
        self.status = match self.status {
            None | Some(CollectionStatus::Singleton) => source.status,
            Some(_) if source.status == Some(CollectionStatus::Unordered) => source.status,
            Some(_) => Some(CollectionStatus::Ordered),
        };
        if source.choice {
            self.choice = true;
        }
    }

    fn list_status(&self, right: &Types) -> CollectionStatus {
        if right.status == Some(CollectionStatus::Unordered)
            || self.status == Some(CollectionStatus::Unordered)
        {
            CollectionStatus::Unordered
        } else {
            CollectionStatus::Ordered
        }
    }

    /// `union(TypeDetails)`.
    pub(crate) fn union(&self, right: &Types) -> Types {
        let mut result = Types::new(Some(self.list_status(right)));
        for n in self.names.iter().chain(&right.names) {
            result.add(n);
        }
        result
    }

    /// `intersect(TypeDetails)` — which, as upstream writes it, keeps the
    /// left types the right has and then adds **every** right type.
    pub(crate) fn intersect(&self, right: &Types) -> Types {
        let mut result = Types::new(Some(self.list_status(right)));
        for n in &self.names {
            if right.contains(n) {
                result.add(n);
            }
        }
        for n in &right.names {
            result.add(n);
        }
        result
    }

    /// `toSingleton()`, `toOrdered()`, `toUnordered()`: the types only.
    pub(crate) fn with_status(&self, status: CollectionStatus) -> Types {
        let mut result = Types::new(Some(status));
        for n in &self.names {
            result.add(n);
        }
        result
    }

    /// `new TypeDetails(status, other.getTypes())`.
    pub(crate) fn names_with_status(&self, status: Option<CollectionStatus>) -> Types {
        let mut result = Types::new(status);
        for n in &self.names {
            result.add(n);
        }
        result
    }

    /// `addTypes(TypeDetails)`.
    pub(crate) fn add_types(&mut self, other: &Types) {
        if other.status != Some(CollectionStatus::Singleton) {
            self.status = if other.status == Some(CollectionStatus::Unordered)
                || self.status == Some(CollectionStatus::Unordered)
            {
                Some(CollectionStatus::Unordered)
            } else {
                Some(CollectionStatus::Ordered)
            };
        }
        for n in &other.names {
            self.add(n);
        }
    }

    /// `contains(TypeDetails)`: the same status, and every type of `other`.
    pub(crate) fn contains_all(&self, other: &Types) -> bool {
        other.status == self.status && other.names.iter().all(|n| self.contains(n))
    }

    /// `hasType(String)`: `n` as written, or its System counterpart for a
    /// handful of bare names. `n` is a bare name an upstream call site wrote.
    pub(crate) fn has_type_plain(&self, n: &str) -> bool {
        self.contains(n) || (SYSTEM_MAPPED_PLAIN.contains(&n) && self.contains(&system_of(n)))
    }

    /// `hasType(Set<String>)` over the engine's `primitiveTypes`: a FHIR
    /// primitive type, or one of the six System types the set's names map to.
    pub(crate) fn has_primitive_type(&self, model: &dyn Model) -> bool {
        self.names.iter().any(|n| {
            SYSTEM_PRIMITIVES.contains(&n.as_str())
                || (!is_system(n) && !is_backbone(n) && model.is_primitive(n))
        })
    }

    /// `hasType(IWorkerContext, String...)` for one name.
    ///
    /// `bare` says which of upstream's two kinds of argument `n` is, because
    /// the method treats them differently and the difference decides
    /// verdicts: a call site's bare name (`"integer"`) is also matched by its
    /// System counterpart, while a type taken **from another set** arrives as
    /// a URL and is not — so a `positiveInt` argument does not satisfy an
    /// `integer` parameter, while a `code` argument does satisfy a `string`
    /// one, through the "work around for R3" (`TypeDetails.java:289`).
    pub(crate) fn has_type_worker(&self, model: &dyn Model, n: &str, bare: bool) -> bool {
        if self.contains(n) {
            return true;
        }
        if bare && SYSTEM_MAPPED.contains(&n) && self.contains(&system_of(n)) {
            return true;
        }
        let fhir = !is_system(n) && !n.contains("://") && !is_backbone(n);
        if !fhir || !model.is_type(n) {
            // A backbone's `#tail` test is the containment above; a System
            // type has no definition to walk.
            return false;
        }
        if SD_SYSTEM_MAPPED.contains(&n) && self.contains(&system_of(n)) {
            return true;
        }
        // Walk the definition chain. `getSystemType` builds `System..X`, with
        // two dots, so it never matches anything and is not ported.
        let mut current = Some(n.to_string());
        let mut guard = 0usize;
        while let Some(t) = current {
            if self.contains(&t) {
                return true;
            }
            if t == "string" && self.contains(SYSTEM_STRING) {
                return true;
            }
            guard += 1;
            if guard > 64 {
                return false;
            }
            current = if t == "uri" {
                Some("string".to_string())
            } else {
                model.base_type(&t).map(str::to_string)
            };
        }
        false
    }

    /// Whether any of `names`, each a bare call-site name, is here by
    /// [`Types::has_type_worker`].
    pub(crate) fn has_any_worker(&self, model: &dyn Model, names: &[&str]) -> bool {
        names.iter().any(|n| self.has_type_worker(model, n, true))
    }

    /// `describe()`: the URLs, sorted.
    pub(crate) fn describe(&self) -> String {
        let mut urls: Vec<String> = self.names.iter().map(|n| url_of(n)).collect();
        urls.sort();
        format!("[{}]", urls.join(", "))
    }

    /// The set as the public surface reports it.
    pub(crate) fn to_type_set(&self) -> TypeSet {
        TypeSet::from_iter_names(self.names.iter().cloned())
    }

    /// Whether any type is a System type, and whether any is not
    /// (`type()`'s result rule).
    pub(crate) fn system_and_class(&self) -> (bool, bool) {
        let s = self.names.iter().any(|n| is_system(n));
        let c = self.names.iter().any(|n| !is_system(n));
        (s, c)
    }
}
