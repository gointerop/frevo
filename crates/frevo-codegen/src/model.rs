//! The model the generator emits: every R4 type, its element set, and the
//! names each takes in Rust.
//!
//! This is the whole of tasks 8, 9 and 10 of `add-fhirpath-and-codegen-r4`
//! except the writing of it, which is [`crate::emit`]. One pass over the
//! package's snapshots produces:
//!
//! - a type per primitive, complex datatype, concrete resource, abstract
//!   element set and the one logical definition (`codegen.types.001` to
//!   `codegen.types.005`);
//! - a **named nested type per backbone path**, at every level
//!   (`codegen.types.006`, `codegen.types.007`);
//! - choice elements under their base name, with the full JSON property-name
//!   map (`codegen.choice.001` to `codegen.choice.003`);
//! - each element's declared cardinality (`codegen.cardinality.001` to
//!   `codegen.cardinality.004`).
//!
//! # Why nothing recurses
//!
//! `codegen.types.008` and `codegen.types.009` are about termination, and the
//! reason generation terminates is structural rather than guarded: this module
//! never follows a type reference. A backbone type is produced from a *path
//! prefix* of the snapshot it is declared in, and an element's type is recorded
//! as a **name**. So `Questionnaire.item.item`, whose `contentReference` points
//! back at `Questionnaire.item`, becomes one element naming one type that
//! already exists, and the `Reference` / `Identifier` cycle is two complete
//! types naming each other. There is no expansion depth to exhaust, which is
//! why neither scenario needs a depth limit to satisfy — and why a depth limit
//! would be the bug.
//!
//! # What is recorded, and what is resolved
//!
//! Two views of an element's type are kept, because the oracle and the codec
//! need different ones:
//!
//! | View | Field | `Patient.contact` | `Questionnaire.item.item` |
//! |---|---|---|---|
//! | as the snapshot declares it | [`Element::types`] | `BackboneElement` | *(none)* |
//! | as a reader navigates it | [`Element::nav_types`] | `Patient.contact` | `Questionnaire.item` |
//!
//! Neither is invented: the first is the snapshot's `type.code`, the second is
//! the nested type that snapshot path produced. Both are emitted.

use std::collections::BTreeMap;

use crate::bindings::{self, Codes, Enumeration, Resolution, UnresolvedBinding};
use crate::error::{CodegenError, FailureKind, Result};
use crate::inventory::Classification;
use crate::package::{Package, SnapshotElement};
use crate::synthetic;

/// An element's declared upper bound.
///
/// `Zero` is not hypothetical: `profiles-types.xml` gives `xhtml.extension` a
/// maximum of `0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Max {
    Zero,
    One,
    Many,
}

impl Max {
    /// Parse the snapshot's literal `max`.
    ///
    /// R4 core uses only `0`, `1` and `*`; a numeric bound above one would be
    /// a list with an upper bound, so it is [`Max::Many`].
    pub fn parse(literal: &str) -> Max {
        match literal {
            "0" => Max::Zero,
            "1" => Max::One,
            _ => Max::Many,
        }
    }

    /// Whether the element may hold more than one value.
    pub fn is_repeating(self) -> bool {
        self == Max::Many
    }

    /// The identifier the generated table spells this with.
    pub fn as_rust(self) -> &'static str {
        match self {
            Max::Zero => "Max::Zero",
            Max::One => "Max::One",
            Max::Many => "Max::Many",
        }
    }
}

/// What kind of thing a generated type is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TypeKind {
    /// One of the 20 primitive datatypes.
    Primitive,
    /// One of the 43 `complex-type` definitions, `Element` included.
    Complex,
    /// One of the 146 concrete resources.
    Resource,
    /// A named nested type generated from a backbone path.
    Backbone,
    /// `Resource` and `DomainResource`: an element set shared into every
    /// resource, never instantiable (`codegen.types.004`).
    AbstractElementSet,
    /// `MetadataResource`, the one `logical` definition.
    Logical,
}

impl TypeKind {
    /// The order the generated table lists types in: inputs before the things
    /// built out of them.
    fn rank(self) -> u8 {
        match self {
            TypeKind::Primitive => 0,
            TypeKind::Complex => 1,
            TypeKind::Backbone => 2,
            TypeKind::AbstractElementSet => 3,
            TypeKind::Logical => 4,
            TypeKind::Resource => 5,
        }
    }

    /// The `crate::codec::tables::TypeKind` variant the emitted table uses.
    ///
    /// The codec distinguishes four cases where this distinguishes six, and
    /// one arm of the mapping is not the obvious one: an **abstract element
    /// set** is what an element declares when it accepts any resource, so it
    /// carries the codec's `AnyResource`, which is what tells a decode to read
    /// the concrete type out of the instance's `resourceType`. In R4 core that
    /// is reached through `Resource`, which 145 `contained` elements plus
    /// `Bundle.entry.resource`, `Bundle.entry.response.outcome` and
    /// `Parameters.parameter.resource` declare; no element declares
    /// `DomainResource`, which therefore takes the same arm without ever being
    /// reached by it.
    pub fn as_codec_rust(self) -> &'static str {
        match self {
            TypeKind::Primitive => "TypeKind::Primitive",
            TypeKind::Complex | TypeKind::Backbone | TypeKind::Logical => "TypeKind::Complex",
            TypeKind::Resource => "TypeKind::Resource",
            TypeKind::AbstractElementSet => "TypeKind::AnyResource",
        }
    }
}

/// How a primitive's lexical form is spelled in JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonForm {
    Quoted,
    Boolean,
    Number,
}

impl JsonForm {
    /// The form a root primitive's `value` takes, from the FHIRPath system
    /// type the snapshot declares for it.
    ///
    /// Read from the **root** of the primitive's base chain, never from the
    /// primitive itself: `positiveInt.value` and `unsignedInt.value` both
    /// declare `http://hl7.org/fhirpath/System.String` in R4, and both are
    /// JSON numbers. Their base, `integer`, declares `System.Integer`.
    fn of_system_code(code: &str) -> JsonForm {
        match code {
            "http://hl7.org/fhirpath/System.Boolean" => JsonForm::Boolean,
            "http://hl7.org/fhirpath/System.Integer" | "http://hl7.org/fhirpath/System.Decimal" => {
                JsonForm::Number
            }
            _ => JsonForm::Quoted,
        }
    }

    /// The identifier the generated table spells this with.
    pub fn as_rust(self) -> &'static str {
        match self {
            JsonForm::Quoted => "JsonForm::Quoted",
            JsonForm::Boolean => "JsonForm::Boolean",
            JsonForm::Number => "JsonForm::Number",
        }
    }
}

/// An element's value-set binding, as the snapshot declares it.
///
/// `enumeration` is the name of the closed enumeration the generator chose to
/// emit for it, and is `None` unless the binding is `required` *and* its value
/// set expands from the input alone. [`crate::bindings`] is where that
/// decision is made and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub strength: String,
    pub value_set: String,
    pub enumeration: Option<String>,
}

/// One generated element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    /// The FHIR property name the element is addressed by: the base name for a
    /// choice, so `value` and not `value[x]`.
    pub fhir_name: String,
    pub rust_name: String,
    /// The snapshot path, `[x]` included.
    pub path: String,
    /// The types the snapshot declares, in declared order.
    pub types: Vec<String>,
    /// The types a reader navigates into: a backbone path, or a resolved
    /// `contentReference` where the snapshot declares a structural
    /// placeholder.
    pub nav_types: Vec<String>,
    pub is_choice: bool,
    /// Every JSON property name that maps to this element: one for an ordinary
    /// element, one per declared type for a choice.
    pub json_property_names: Vec<String>,
    pub min: u32,
    pub max: Max,
    pub binding: Option<Binding>,
    /// `Questionnaire.item.item` carries `#Questionnaire.item` here.
    pub content_reference: Option<String>,
}

/// One generated type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Type {
    pub fhir_name: String,
    pub rust_name: String,
    pub kind: TypeKind,
    /// The immediate base type's name: `code` has `string`, `Age` has
    /// `Quantity`, `Patient.contact` has `BackboneElement`.
    pub base: Option<String>,
    /// `false` for every `abstract` definition, which is what stops a value of
    /// `Resource` or `DomainResource` being constructible
    /// (`codegen.types.004`).
    pub instantiable: bool,
    pub elements: Vec<Element>,
    /// Only meaningful for a primitive.
    pub json_form: JsonForm,
    /// The `structuredefinition-regex` extension on a primitive's `value`.
    /// Every R4 primitive but `xhtml` declares one.
    pub value_regex: Option<String>,
}

impl Type {
    /// The element order the snapshot declares, by FHIR property name.
    pub fn element_order(&self) -> Vec<String> {
        self.elements.iter().map(|e| e.fhir_name.clone()).collect()
    }

    pub fn element(&self, fhir_name: &str) -> Option<&Element> {
        self.elements.iter().find(|e| e.fhir_name == fhir_name)
    }
}

/// Every type the generator emits, plus the resource-type registry and the
/// enumerations its required bindings closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    pub types: Vec<Type>,
    /// FHIR type name to Rust type name, for the 146 concrete resources only
    /// (`codegen.types.005`).
    pub resource_registry: Vec<(String, String)>,
    /// One closed enumeration per resolvable `required` binding, in value-set
    /// order (`codegen.binding.001`).
    pub enumerations: Vec<Enumeration>,
    /// Every `required` binding whose value set the input could not expand,
    /// reported rather than passed over (`codegen.binding.004`).
    pub unresolved_bindings: Vec<UnresolvedBinding>,
}

impl Model {
    pub fn ty(&self, fhir_name: &str) -> Option<&Type> {
        self.types.iter().find(|t| t.fhir_name == fhir_name)
    }

    pub fn of_kind(&self, kind: TypeKind) -> impl Iterator<Item = &Type> {
        self.types.iter().filter(move |t| t.kind == kind)
    }
}

// ---------------------------------------------------------------------------
// The normalised input one root definition contributes
// ---------------------------------------------------------------------------

/// One element of a root definition's snapshot, reduced to what generation
/// reads.
#[derive(Debug, Clone)]
struct RawElement {
    path: String,
    min: u32,
    max: Max,
    types: Vec<String>,
    content_reference: Option<String>,
    binding: Option<Binding>,
}

/// One definition, classified, with its snapshot.
#[derive(Debug, Clone)]
struct Root {
    fhir_name: String,
    kind: TypeKind,
    base: Option<String>,
    instantiable: bool,
    /// The path prefix the snapshot's elements are rooted at.
    ///
    /// Not always the definition's name: a `constraint` profile on a core type
    /// — `MoneyQuantity`, `SimpleQuantity` — declares
    /// `StructureDefinition.type` `Quantity`, and its snapshot paths read
    /// `Quantity.value`.
    root_path: String,
    /// The FHIRPath system type code on this definition's own `value`, kept
    /// verbatim because it is what decides a primitive's JSON form.
    value_system_code: Option<String>,
    value_regex: Option<String>,
    elements: Vec<RawElement>,
}

/// The last segment of a canonical URL, which for a core definition is the
/// type name.
fn type_name_of_url(url: &str) -> String {
    url.rsplit('/').next().unwrap_or(url).to_string()
}

/// Build the model from the pinned package's StructureDefinitions.
pub fn from_package(package: &Package) -> Result<Model> {
    let mut roots = Vec::new();
    for definition in &package.definitions {
        let kind = match Classification::of(definition) {
            Classification::PrimitiveDatatype => TypeKind::Primitive,
            Classification::ComplexDatatype => TypeKind::Complex,
            Classification::ConcreteResource => TypeKind::Resource,
            Classification::AbstractResource => TypeKind::AbstractElementSet,
            Classification::LogicalResource => TypeKind::Logical,
            Classification::Profile | Classification::OutOfScope => continue,
        };
        let root_path = definition
            .type_name
            .clone()
            .unwrap_or_else(|| definition.name.clone());
        let value_path = format!("{root_path}.value");
        let value = definition
            .snapshot
            .iter()
            .find(|element| element.path == value_path);

        roots.push(Root {
            fhir_name: definition.name.clone(),
            kind,
            base: definition.base_definition.as_deref().map(type_name_of_url),
            instantiable: !definition.is_abstract,
            root_path,
            value_system_code: value.and_then(|v| v.types.first().cloned()),
            value_regex: value.and_then(|v| v.value_regex.clone()),
            elements: definition.snapshot.iter().map(raw_element).collect(),
        });
    }
    build(roots, &Codes::of_package(package))
}

fn raw_element(element: &SnapshotElement) -> RawElement {
    RawElement {
        path: element.path.clone(),
        min: element.min.unwrap_or(0),
        max: element.max.as_deref().map(Max::parse).unwrap_or(Max::One),
        types: element.fhir_types.clone(),
        content_reference: element.content_reference.clone(),
        binding: element.binding.as_ref().map(|binding| Binding {
            strength: binding.strength.clone(),
            value_set: binding.value_set.clone(),
            enumeration: None,
        }),
    }
}

/// Build the model from definitions a caller wrote rather than read.
pub fn from_synthetic(request: &synthetic::Request) -> Result<Model> {
    let roots = request
        .definitions
        .iter()
        .map(|definition| Root {
            fhir_name: definition.name.clone(),
            kind: match definition.kind {
                synthetic::Kind::PrimitiveType => TypeKind::Primitive,
                synthetic::Kind::ComplexType => TypeKind::Complex,
                synthetic::Kind::Resource if definition.is_abstract => TypeKind::AbstractElementSet,
                synthetic::Kind::Resource => TypeKind::Resource,
                synthetic::Kind::Logical => TypeKind::Logical,
            },
            base: definition.base.as_deref().map(type_name_of_url),
            instantiable: !definition.is_abstract,
            root_path: definition.name.clone(),
            value_system_code: None,
            value_regex: None,
            elements: definition
                .elements
                .iter()
                .map(|element| RawElement {
                    path: element.path.clone(),
                    min: element.min,
                    max: Max::parse(&element.max),
                    types: element.types.clone(),
                    content_reference: element.content_reference.clone(),
                    binding: element.binding.as_ref().map(|binding| Binding {
                        strength: binding.strength.clone(),
                        value_set: binding.value_set.clone(),
                        enumeration: None,
                    }),
                })
                .collect(),
        })
        .collect();
    build(roots, &Codes::of_synthetic(&request.value_sets))
}

// ---------------------------------------------------------------------------
// The build
// ---------------------------------------------------------------------------

fn build(mut roots: Vec<Root>, codes: &Codes) -> Result<Model> {
    roots.sort_by(|a, b| {
        a.kind
            .rank()
            .cmp(&b.kind.rank())
            .then_with(|| a.fhir_name.cmp(&b.fhir_name))
    });
    let forms = json_forms(&roots);

    let mut types: Vec<Type> = Vec::new();
    for root in &roots {
        types.extend(expand(root, &forms)?);
    }
    check_type_names(&types)?;

    let mut resource_registry: Vec<(String, String)> = types
        .iter()
        .filter(|t| t.kind == TypeKind::Resource)
        .map(|t| (t.fhir_name.clone(), t.rust_name.clone()))
        .collect();
    resource_registry.sort();

    let resolution = resolve_bindings(&mut types, codes);

    Ok(Model {
        types,
        resource_registry,
        enumerations: resolution.enumerations,
        unresolved_bindings: resolution.unresolved,
    })
}

/// Decide which `required` bindings close, and write the chosen enumeration
/// onto each element that binds one.
///
/// Two passes, because the decision is per *value set* and the record is per
/// *element*: the first collects every declared binding so that names are
/// assigned in value-set order rather than in whichever order an element was
/// visited, and the second writes the answer back. An element bound to a value
/// set the input cannot expand keeps `enumeration: None` and stays open, which
/// is the whole of `codegen.binding.004`.
fn resolve_bindings(types: &mut [Type], codes: &Codes) -> Resolution {
    let declared: Vec<(String, String, String)> = types
        .iter()
        .flat_map(|ty| ty.elements.iter())
        .filter_map(|element| {
            element.binding.as_ref().map(|binding| {
                (
                    element.path.clone(),
                    binding.strength.clone(),
                    binding.value_set.clone(),
                )
            })
        })
        .collect();
    let bound: Vec<bindings::BoundElement<'_>> = declared
        .iter()
        .map(|(path, strength, value_set)| bindings::BoundElement {
            path,
            strength,
            value_set,
        })
        .collect();
    let resolution = bindings::resolve(&bound, codes);

    for ty in types.iter_mut() {
        for element in ty.elements.iter_mut() {
            if let Some(binding) = element.binding.as_mut() {
                binding.enumeration =
                    bindings::enumeration_of(&resolution, &binding.strength, &binding.value_set)
                        .map(str::to_string);
            }
        }
    }
    resolution
}

/// Each primitive's JSON form, resolved from the root of its base chain.
fn json_forms(roots: &[Root]) -> BTreeMap<String, JsonForm> {
    let primitives: BTreeMap<&str, &Root> = roots
        .iter()
        .filter(|r| r.kind == TypeKind::Primitive)
        .map(|r| (r.fhir_name.as_str(), r))
        .collect();

    let root_code = |name: &str| -> Option<String> {
        let mut current = name;
        for _ in 0..16 {
            let root = primitives.get(current)?;
            match root.base.as_deref() {
                // A primitive whose base is another primitive takes its JSON
                // form; one whose base is `Element` is the root of the chain.
                Some(base) if primitives.contains_key(base) => current = base,
                _ => return root.value_system_code.clone(),
            }
        }
        None
    };

    primitives
        .keys()
        .map(|name| {
            let form = root_code(name)
                .map(|code| JsonForm::of_system_code(&code))
                .unwrap_or(JsonForm::Quoted);
            ((*name).to_string(), form)
        })
        .collect()
}

/// One root definition's types: itself, then one per backbone path in snapshot
/// order.
fn expand(root: &Root, forms: &BTreeMap<String, JsonForm>) -> Result<Vec<Type>> {
    // A path owns a nested type when some other path continues it.
    let is_parent = |path: &str| -> bool {
        let prefix = format!("{path}.");
        root.elements
            .iter()
            .any(|other| other.path.starts_with(&prefix))
    };
    // The generated type name for a snapshot path: the definition's own name
    // replaces the snapshot's root prefix, which matters only for a constraint
    // profile whose snapshot is rooted at the type it profiles.
    let type_name_of = |path: &str| -> String {
        match path.strip_prefix(&root.root_path) {
            Some("") => root.fhir_name.clone(),
            Some(tail) => format!("{}{tail}", root.fhir_name),
            None => path.to_string(),
        }
    };

    let mut built: Vec<Type> = vec![Type {
        fhir_name: root.fhir_name.clone(),
        rust_name: type_rust_name(&root.fhir_name, root.kind),
        kind: root.kind,
        base: root.base.clone(),
        instantiable: root.instantiable,
        elements: Vec::new(),
        json_form: forms
            .get(&root.fhir_name)
            .copied()
            .unwrap_or(JsonForm::Quoted),
        value_regex: root.value_regex.clone(),
    }];

    // Every backbone path, in snapshot order, before any element is placed, so
    // that an element is always placed into a type that is already there.
    for element in &root.elements {
        if !element.path.contains('.') || !is_parent(&element.path) {
            continue;
        }
        let fhir_name = type_name_of(&element.path);
        built.push(Type {
            rust_name: type_rust_name(&fhir_name, TypeKind::Backbone),
            fhir_name,
            kind: TypeKind::Backbone,
            base: Some(
                element
                    .types
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "BackboneElement".to_string()),
            ),
            instantiable: true,
            elements: Vec::new(),
            json_form: JsonForm::Quoted,
            value_regex: None,
        });
    }

    for element in &root.elements {
        let Some((owner_path, segment)) = element.path.rsplit_once('.') else {
            continue; // the snapshot's root element is the type, not an element
        };
        let owner = type_name_of(owner_path);
        let is_choice = segment.ends_with("[x]");
        let fhir_name = segment.trim_end_matches("[x]").to_string();

        let nav_types = match element.content_reference.as_deref() {
            Some(reference) => vec![type_name_of(reference.trim_start_matches('#'))],
            None if is_parent(&element.path) => vec![type_name_of(&element.path)],
            None => element.types.clone(),
        };
        let json_property_names = if is_choice {
            element
                .types
                .iter()
                .map(|t| choice_property_name(&fhir_name, t))
                .collect()
        } else {
            vec![fhir_name.clone()]
        };

        let built_element = Element {
            rust_name: element_rust_name(&fhir_name),
            fhir_name,
            path: element.path.clone(),
            types: element.types.clone(),
            nav_types,
            is_choice,
            json_property_names,
            min: element.min,
            max: element.max,
            binding: element.binding.clone(),
            content_reference: element.content_reference.clone(),
        };

        let Some(target) = built.iter_mut().find(|t| t.fhir_name == owner) else {
            return Err(CodegenError::other(format!(
                "{} has no generated type to hold it: no snapshot element declares {owner}",
                element.path
            )));
        };
        target.elements.push(built_element);
    }

    for ty in &built {
        check_element_names(ty)?;
    }
    Ok(built)
}

// ---------------------------------------------------------------------------
// Names
// ---------------------------------------------------------------------------

/// The Rust type name for a FHIR type.
///
/// A resource or datatype keeps its FHIR name verbatim — no pluralisation, no
/// case change, no prefix (`codegen.naming.004`). A primitive's name is
/// lower-case in FHIR, so it is upper-cased to be a type name; a backbone path
/// becomes the concatenation of its upper-cased segments.
pub fn type_rust_name(fhir_name: &str, kind: TypeKind) -> String {
    match kind {
        TypeKind::Primitive | TypeKind::Backbone => fhir_name
            .split('.')
            .map(upper_first)
            .collect::<Vec<_>>()
            .concat(),
        _ => fhir_name.to_string(),
    }
}

/// The Rust field name for a FHIR element name: snake_case, with a keyword
/// escaped by a trailing underscore.
pub fn element_rust_name(fhir_name: &str) -> String {
    let snake = snake_case(fhir_name);
    if RUST_KEYWORDS.contains(&snake.as_str()) {
        return format!("{snake}_");
    }
    snake
}

/// The Rust field name for a snapshot element path, which is the naming rule
/// asked about a path rather than a name.
pub fn rust_field_name_of_path(path: &str) -> String {
    let segment = path.rsplit('.').next().unwrap_or(path);
    element_rust_name(segment.trim_end_matches("[x]"))
}

/// `value` plus `Quantity` is `valueQuantity`.
pub fn choice_property_name(base: &str, type_name: &str) -> String {
    format!("{base}{}", upper_first(type_name))
}

fn upper_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// camelCase to snake_case over the ASCII names FHIR uses.
///
/// `birthDate` becomes `birth_date`, and `someId` and `someID` both become
/// `some_id` — which is the collision [`check_element_names`] exists to catch
/// rather than to resolve.
fn snake_case(name: &str) -> String {
    let bytes = name.as_bytes();
    let mut out = String::with_capacity(name.len() + 4);
    for (index, &byte) in bytes.iter().enumerate() {
        if byte.is_ascii_uppercase() {
            let previous = index.checked_sub(1).map(|i| bytes[i]);
            let next = bytes.get(index + 1).copied();
            let starts_a_word = previous.is_some_and(|p| !p.is_ascii_uppercase() && p != b'_')
                || (previous.is_some_and(|p| p.is_ascii_uppercase())
                    && next.is_some_and(|n| n.is_ascii_lowercase()));
            if starts_a_word {
                out.push('_');
            }
            out.push(byte.to_ascii_lowercase() as char);
        } else {
            out.push(byte as char);
        }
    }
    out
}

/// Rust's keywords, reserved words included, so an escaped name is never a
/// keyword in a later edition (`codegen.naming.002`).
const RUST_KEYWORDS: [&str; 51] = [
    "as", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "false", "fn",
    "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
    "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe",
    "use", "where", "while", "async", "await", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "try", "typeof", "unsized", "virtual", "yield",
];

/// Two elements of one type that would take the same Rust name is a refusal,
/// never a type that silently loses one of them (`codegen.naming.003`).
fn check_element_names(ty: &Type) -> Result<()> {
    let mut seen: BTreeMap<&str, &str> = BTreeMap::new();
    for element in &ty.elements {
        if let Some(previous) = seen.insert(element.rust_name.as_str(), element.path.as_str()) {
            return Err(CodegenError::failed(
                FailureKind::NameCollision,
                format!(
                    "{previous} and {} would both generate the Rust field name {:?} on {}, \
                     so no model was generated. Two FHIR element names that differ only in a \
                     way snake_case erases cannot share a type; the generator refuses rather \
                     than emitting a type that silently loses one of them.",
                    element.path, element.rust_name, ty.fhir_name
                ),
            ));
        }
    }
    Ok(())
}

/// The same rule for type names, which a backbone path could collide on.
fn check_type_names(types: &[Type]) -> Result<()> {
    let mut seen: BTreeMap<&str, &str> = BTreeMap::new();
    for ty in types {
        if let Some(previous) = seen.insert(ty.rust_name.as_str(), ty.fhir_name.as_str()) {
            return Err(CodegenError::failed(
                FailureKind::NameCollision,
                format!(
                    "{previous} and {} would both generate the Rust type name {:?}, so no \
                     model was generated.",
                    ty.fhir_name, ty.rust_name
                ),
            ));
        }
    }
    Ok(())
}
