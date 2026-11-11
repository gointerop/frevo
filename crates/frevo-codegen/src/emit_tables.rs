//! Writing the generated R4 type tables.
//!
//! Two files come out of [`crate::model::Model`]:
//!
//! | File | Holds | Read by |
//! |---|---|---|
//! | `types.rs` | every type, its base, its kind, its JSON form and its elements **in snapshot order** | the codec, as `crate::codec::tables::Tables` |
//! | `elements.rs` | each element's declared cardinality, its value-set binding and its `contentReference` | anything that needs more than the codec's four fields |
//!
//! # Why two, and why this shape
//!
//! `crates/frevo-model/src/codec/tables.rs` is hand-written and declares the
//! *shape* the codec reads: `TypeDef { name, base, kind, json_form, elements }`
//! and `ElementDef { name, choice, repeating, types }`. Emitting exactly that
//! shape is what makes swapping the codec off its provisional stand-in and on
//! to this output a single `const` rebinding in `codec/mod.rs` — which is a
//! `frevo-model` change, and deliberately not made here.
//!
//! That shape has nowhere to put a **minimum** cardinality, and `repeating`
//! cannot express the `0..0` that `xhtml.extension` declares. Both are
//! required (`codegen.cardinality.002`, `codegen.cardinality.004`,
//! `codegen.types.001`), so they go in `elements.rs` rather than being
//! dropped or bolted onto a hand-written struct from here.
//!
//! # The element identity `elements.rs` uses
//!
//! A row is keyed by its owning generated type and its FHIR property name, so
//! the snapshot path is `owner.name`, plus `[x]` where `types.rs` records the
//! element as a choice. That holds for every R4 core element except the two
//! `constraint` profiles `MoneyQuantity` and `SimpleQuantity`, whose
//! `StructureDefinition.type` is `Quantity` and whose snapshot paths therefore
//! read `Quantity.value`. Their rows are keyed by the profile, which is the
//! type a reader asks about.

use std::collections::{BTreeMap, BTreeSet};

use crate::emit::{GeneratedFile, header};
use crate::model::{Element, Model, TypeKind};
use crate::pin::PackagePin;
use crate::rustsrc::{INDENT, Item, array_statement, pad, quote, scalar_statement};

/// The type table: `types.rs`.
pub fn types_file(pin: &PackagePin, model: &Model) -> GeneratedFile {
    let type_sets = TypeSets::of(model);
    let mut source = header(pin);
    source.push_str(&types_doc(model));

    source.push_str(
        "\nuse crate::codec::tables::{ElementDef, JsonForm, Tables, TypeDef, TypeKind};\n",
    );
    source.push_str(&constructors(model));
    source.push_str(&type_sets.emit());
    source.push_str(&element_sets(model, &type_sets));
    source.push_str(&type_table(model));
    source.push_str(&name_lists(model));

    GeneratedFile::new("types.rs", &source)
}

/// The element metadata table: `elements.rs`.
pub fn elements_file(pin: &PackagePin, model: &Model) -> GeneratedFile {
    let value_sets = ValueSets::of(model);
    let mut source = header(pin);
    source.push_str(&elements_doc(model));
    source.push_str(ELEMENTS_PREAMBLE);
    source.push_str(&value_sets.emit());
    source.push_str(&element_table(model, &value_sets));
    source.push_str(ELEMENTS_LOOKUP);

    GeneratedFile::new("elements.rs", &source)
}

// ---------------------------------------------------------------------------
// types.rs
// ---------------------------------------------------------------------------

fn types_doc(model: &Model) -> String {
    let count = |kind: TypeKind| model.of_kind(kind).count();
    format!(
        "\n//! The generated R4 type table.\n\
         //!\n\
         //! One entry per type, each carrying its base type, its kind, the JSON\n\
         //! form its values take and its elements **in the order the\n\
         //! StructureDefinition snapshot declares them** -- which is the order a\n\
         //! stored resource is serialised in.\n\
         //!\n\
         //! | Kind | Count |\n\
         //! |---|---|\n\
         //! | primitive datatypes | {primitive} |\n\
         //! | complex datatypes | {complex} |\n\
         //! | nested backbone types | {backbone} |\n\
         //! | abstract element sets | {abstract_sets} |\n\
         //! | logical definitions | {logical} |\n\
         //! | concrete resources | {resource} |\n\
         //! | **total** | **{total}** |\n\
         //!\n\
         //! A choice element is listed once, under its base name, with the full\n\
         //! declared type set; the JSON property names it answers to are that\n\
         //! base name followed by each type with its first letter upper-cased.\n\
         //!\n\
         //! An element's types are the ones a reader navigates by, so a backbone\n\
         //! element names the nested type its path produced rather than the\n\
         //! `BackboneElement` the snapshot declares, and a `contentReference` is\n\
         //! resolved to the type it points at. The declared form, the\n\
         //! cardinality and the binding are in `elements.rs`.\n",
        primitive = count(TypeKind::Primitive),
        complex = count(TypeKind::Complex),
        backbone = count(TypeKind::Backbone),
        abstract_sets = count(TypeKind::AbstractElementSet),
        logical = count(TypeKind::Logical),
        resource = count(TypeKind::Resource),
        total = model.types.len(),
    )
}

/// Which of the `ElementDef` constructors the table needs.
fn constructors(model: &Model) -> String {
    let mut used: BTreeSet<&str> = BTreeSet::new();
    for ty in &model.types {
        for element in &ty.elements {
            used.insert(constructor_of(element));
        }
    }

    let mut out = String::new();
    for (name, doc, choice, repeating) in [
        (
            "one",
            "An element that holds at most one value.",
            false,
            false,
        ),
        ("many", "An element that may repeat.", false, true),
        (
            "pick",
            "A `value[x]`-style choice element, holding at most one value of\n/// one of its declared types.",
            true,
            false,
        ),
        ("picks", "A repeating choice element.", true, true),
    ] {
        if !used.contains(name) {
            continue;
        }
        out.push_str(&format!(
            "\n/// {doc}\nconst fn {name}(name: &'static str, types: &'static [&'static str]) -> ElementDef {{\n\
             {pad}ElementDef {{\n\
             {pad2}name,\n\
             {pad2}choice: {choice},\n\
             {pad2}repeating: {repeating},\n\
             {pad2}types,\n\
             {pad}}}\n\
             }}\n",
            pad = pad(INDENT),
            pad2 = pad(INDENT * 2),
        ));
    }
    out
}

fn constructor_of(element: &Element) -> &'static str {
    match (element.is_choice, element.max.is_repeating()) {
        (false, false) => "one",
        (false, true) => "many",
        (true, false) => "pick",
        (true, true) => "picks",
    }
}

/// The hoisted `&[&str]` type sets, deduplicated.
///
/// R4's 7,712 elements declare only 141 distinct type lists, so hoisting them
/// keeps every element row on one line and the file a third of the size it
/// would otherwise be.
struct TypeSets {
    /// Joined type list to constant name.
    names: BTreeMap<String, String>,
}

impl TypeSets {
    fn of(model: &Model) -> TypeSets {
        let mut distinct: BTreeSet<Vec<String>> = BTreeSet::new();
        for ty in &model.types {
            for element in &ty.elements {
                distinct.insert(element.nav_types.clone());
            }
        }
        let mut names: BTreeMap<String, String> = BTreeMap::new();
        for (index, types) in distinct.iter().enumerate() {
            names.insert(types.join("\u{1}"), format!("T_{index:04}"));
        }
        TypeSets { names }
    }

    fn name_of(&self, types: &[String]) -> &str {
        self.names
            .get(&types.join("\u{1}"))
            .expect("every element's type set is hoisted")
    }

    fn emit(&self) -> String {
        let mut by_name: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for (joined, name) in &self.names {
            let types = if joined.is_empty() {
                Vec::new()
            } else {
                joined.split('\u{1}').collect()
            };
            by_name.insert(name.as_str(), types);
        }
        let mut out = String::from(
            "\n// The type sets elements declare, in lexical order of their contents and\n\
             // numbered rather than named: R4's 7,500 elements declare only a few dozen\n\
             // distinct lists, and a name built from a 106-character snapshot path would\n\
             // push the declaration past the margin and buy nothing the literal beside it\n\
             // does not already say.\n",
        );
        for (name, types) in by_name {
            let items: Vec<Item> = types.iter().map(|t| Item::string(t)).collect();
            out.push_str(&array_statement(
                0,
                &format!("const {name}: &[&str] = "),
                &items,
                ";",
            ));
        }
        out
    }
}

/// One `const E_*` per type, in table order.
fn element_sets(model: &Model, type_sets: &TypeSets) -> String {
    let mut out = String::from(
        "\n// The element set of each type, in snapshot order, numbered in the order\n\
         // TYPES lists them. Each carries the FHIR type it belongs to as a comment,\n\
         // and TYPES names it again beside the reference.\n",
    );
    for (index, ty) in model.types.iter().enumerate() {
        let items: Vec<Item> = ty
            .elements
            .iter()
            .map(|element| {
                Item::call(
                    constructor_of(element),
                    vec![
                        quote(&element.fhir_name),
                        type_sets.name_of(&element.nav_types).to_string(),
                    ],
                )
            })
            .collect();
        out.push_str(&format!("\n// {}\n", ty.fhir_name));
        out.push_str(&array_statement(
            0,
            &format!("const {}: &[ElementDef] = ", element_set_name(index)),
            &items,
            ";",
        ));
    }
    out
}

fn element_set_name(index: usize) -> String {
    format!("E_{index:04}")
}

fn type_table(model: &Model) -> String {
    let mut out = String::from("\n/// Every generated R4 type.\nconst TYPES: &[TypeDef] = &[\n");
    for (index, ty) in model.types.iter().enumerate() {
        let base = match &ty.base {
            Some(base) => format!("Some({})", quote(base)),
            None => "None".to_string(),
        };
        out.push_str(&format!(
            "{pad}TypeDef {{\n\
             {pad2}name: {name},\n\
             {pad2}base: {base},\n\
             {pad2}kind: {kind},\n\
             {pad2}json_form: {form},\n\
             {pad2}elements: {elements},\n\
             {pad}}},\n",
            pad = pad(INDENT),
            pad2 = pad(INDENT * 2),
            name = quote(&ty.fhir_name),
            kind = ty.kind.as_codec_rust(),
            form = ty.json_form.as_rust(),
            elements = element_set_name(index),
        ));
    }
    out.push_str("];\n");
    out.push_str(
        "\n/// The type tables the codec reads.\npub const TABLES: Tables = Tables { types: TYPES };\n",
    );
    out
}

fn name_lists(model: &Model) -> String {
    let mut out = String::new();

    let resources: Vec<Item> = model
        .resource_registry
        .iter()
        .map(|(fhir_name, _)| Item::string(fhir_name))
        .collect();
    out.push_str(&format!(
        "\n/// Every concrete resource type, alphabetically: the registry that maps a\n\
         /// FHIR type name onto its generated type and back. The Rust type name is\n\
         /// the FHIR name verbatim, so one list is the whole mapping.\n\
         /// There are {}.\n",
        resources.len()
    ));
    out.push_str(&array_statement(
        0,
        "pub const RESOURCE_TYPES: &[&str] = ",
        &resources,
        ";",
    ));

    let primitives: Vec<Item> = model
        .of_kind(TypeKind::Primitive)
        .map(|ty| Item::string(&ty.fhir_name))
        .collect();
    out.push_str(&format!(
        "\n/// Every primitive datatype. There are {}.\n",
        primitives.len()
    ));
    out.push_str(&array_statement(
        0,
        "pub const PRIMITIVE_TYPES: &[&str] = ",
        &primitives,
        ";",
    ));

    let regexes: Vec<Item> = model
        .of_kind(TypeKind::Primitive)
        .filter_map(|ty| {
            ty.value_regex
                .as_ref()
                .map(|regex| Item::call("rx", vec![quote(&ty.fhir_name), quote(regex)]))
        })
        .collect();
    out.push_str(
        "\n/// The regular expression a primitive's lexical value must match, as the\n\
         /// `structuredefinition-regex` extension on its `value` element declares it.\n\
         /// `xhtml` declares none, so it is absent rather than unconstrained-by-\n\
         /// accident.\n\
         const fn rx(name: &'static str, regex: &'static str) -> (&'static str, &'static str) {\n\
         \x20   (name, regex)\n\
         }\n",
    );
    out.push_str(&array_statement(
        0,
        "pub const PRIMITIVE_VALUE_REGEX: &[(&str, &str)] = ",
        &regexes,
        ";",
    ));
    out
}

// ---------------------------------------------------------------------------
// elements.rs
// ---------------------------------------------------------------------------

fn elements_doc(model: &Model) -> String {
    let elements: usize = model.types.iter().map(|t| t.elements.len()).sum();
    let bound = model
        .types
        .iter()
        .flat_map(|t| &t.elements)
        .filter(|e| e.binding.is_some())
        .count();
    let references = model
        .types
        .iter()
        .flat_map(|t| &t.elements)
        .filter(|e| e.content_reference.is_some())
        .count();
    format!(
        "\n//! The declared cardinality of every generated element.\n\
         //!\n\
         //! {elements} rows, one per element of every type in `types.rs`, sorted by\n\
         //! owning type and then by FHIR property name so that a lookup is a binary\n\
         //! search. {bound} of them carry a value-set binding and {references} a\n\
         //! `contentReference`.\n\
         //!\n\
         //! The snapshot path of a row is its owner and its name joined by a dot,\n\
         //! with `[x]` appended where `types.rs` records the element as a choice.\n\
         //! The exceptions are the two `constraint` profiles `MoneyQuantity` and\n\
         //! `SimpleQuantity`, whose `StructureDefinition.type` is `Quantity` and\n\
         //! whose snapshot paths therefore read `Quantity.value`; their rows are\n\
         //! keyed by the profile, which is the type a reader asks about.\n\
         //!\n\
         //! A binding is recorded whatever its strength. Which of them generate a\n\
         //! closed Rust enumeration is a separate decision, and the enumerations\n\
         //! are not here.\n",
    )
}

const ELEMENTS_PREAMBLE: &str = "\n\
/// An element's declared upper bound.\n\
///\n\
/// `Zero` is not hypothetical: the R4 snapshot gives `xhtml.extension` a\n\
/// maximum of `0`.\n\
#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n\
pub enum Max {\n\
\x20   /// `..0`: the element is declared and then prohibited.\n\
\x20   Zero,\n\
\x20   /// `..1`: at most one value.\n\
\x20   One,\n\
\x20   /// `..*`: an ordered list.\n\
\x20   Many,\n\
}\n\
\n\
impl Max {\n\
\x20   /// Whether the element may hold more than one value.\n\
\x20   pub const fn is_repeating(self) -> bool {\n\
\x20       matches!(self, Max::Many)\n\
\x20   }\n\
}\n\
\n\
/// An element's value-set binding, as the snapshot declares it.\n\
#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n\
pub struct BindingMeta {\n\
\x20   /// `required`, `extensible`, `preferred` or `example`.\n\
\x20   pub strength: &'static str,\n\
\x20   /// The canonical URL, version suffix included where R4 states one.\n\
\x20   pub value_set: &'static str,\n\
}\n\
\n\
/// One element's declared cardinality, binding and `contentReference`.\n\
#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n\
pub struct ElementMeta {\n\
\x20   /// The generated type that owns the element.\n\
\x20   pub owner: &'static str,\n\
\x20   /// The FHIR property name: the base name for a choice element.\n\
\x20   pub name: &'static str,\n\
\x20   /// `ElementDefinition.min`.\n\
\x20   pub min: u32,\n\
\x20   /// `ElementDefinition.max`.\n\
\x20   pub max: Max,\n\
\x20   pub binding: Option<BindingMeta>,\n\
\x20   /// `Questionnaire.item.item` carries `#Questionnaire.item` here.\n\
\x20   pub content_reference: Option<&'static str>,\n\
}\n\
\n\
/// A row with no binding and no `contentReference`.\n\
const fn c(owner: &'static str, name: &'static str, min: u32, max: Max) -> ElementMeta {\n\
\x20   ElementMeta {\n\
\x20       owner,\n\
\x20       name,\n\
\x20       min,\n\
\x20       max,\n\
\x20       binding: None,\n\
\x20       content_reference: None,\n\
\x20   }\n\
}\n\
\n\
/// A row whose element binds a value set.\n\
const fn cb(\n\
\x20   owner: &'static str,\n\
\x20   name: &'static str,\n\
\x20   min: u32,\n\
\x20   max: Max,\n\
\x20   strength: &'static str,\n\
\x20   value_set: &'static str,\n\
) -> ElementMeta {\n\
\x20   ElementMeta {\n\
\x20       owner,\n\
\x20       name,\n\
\x20       min,\n\
\x20       max,\n\
\x20       binding: Some(BindingMeta {\n\
\x20           strength,\n\
\x20           value_set,\n\
\x20       }),\n\
\x20       content_reference: None,\n\
\x20   }\n\
}\n\
\n\
/// A row whose element takes its definition from another path.\n\
const fn cr(\n\
\x20   owner: &'static str,\n\
\x20   name: &'static str,\n\
\x20   min: u32,\n\
\x20   max: Max,\n\
\x20   content_reference: &'static str,\n\
) -> ElementMeta {\n\
\x20   ElementMeta {\n\
\x20       owner,\n\
\x20       name,\n\
\x20       min,\n\
\x20       max,\n\
\x20       binding: None,\n\
\x20       content_reference: Some(content_reference),\n\
\x20   }\n\
}\n";

const ELEMENTS_LOOKUP: &str = "\n\
/// The declared cardinality of one element, or `None` when the type does not\n\
/// declare it.\n\
pub fn element_meta(owner: &str, name: &str) -> Option<&'static ElementMeta> {\n\
\x20   let found = ELEMENTS\n\
\x20       .binary_search_by(|row| (row.owner, row.name).cmp(&(owner, name)))\n\
\x20       .ok()?;\n\
\x20   Some(&ELEMENTS[found])\n\
}\n";

/// The hoisted value-set URLs.
struct ValueSets {
    names: BTreeMap<String, String>,
}

impl ValueSets {
    fn of(model: &Model) -> ValueSets {
        let distinct: BTreeSet<&str> = model
            .types
            .iter()
            .flat_map(|t| &t.elements)
            .filter_map(|e| e.binding.as_ref())
            .map(|binding| binding.value_set.as_str())
            .collect();
        let mut names = BTreeMap::new();
        let mut taken = BTreeSet::new();
        for url in distinct {
            let stem = url
                .rsplit('/')
                .next()
                .unwrap_or(url)
                .split('|')
                .next()
                .unwrap_or(url);
            let name = unique(&format!("VS_{}", screaming(stem)), &mut taken);
            names.insert(url.to_string(), name);
        }
        ValueSets { names }
    }

    fn name_of(&self, url: &str) -> &str {
        self.names
            .get(url)
            .expect("every bound value set is hoisted")
    }

    fn emit(&self) -> String {
        if self.names.is_empty() {
            return String::new();
        }
        let mut out = String::from(
            "\n/// The value sets elements bind, hoisted so that each URL is written once.\n",
        );
        let mut by_name: BTreeMap<&str, &str> = BTreeMap::new();
        for (url, name) in &self.names {
            by_name.insert(name.as_str(), url.as_str());
        }
        for (name, url) in by_name {
            out.push_str(&scalar_statement(
                0,
                &format!("const {name}: &str = "),
                &quote(url),
                ";",
            ));
        }
        out
    }
}

fn element_table(model: &Model, value_sets: &ValueSets) -> String {
    let mut rows: Vec<(String, String, Item)> = Vec::new();
    for ty in &model.types {
        for element in &ty.elements {
            let owner = quote(&ty.fhir_name);
            let name = quote(&element.fhir_name);
            let min = element.min.to_string();
            let max = element.max.as_rust().to_string();
            let item = match (&element.binding, &element.content_reference) {
                (Some(binding), _) => Item::call(
                    "cb",
                    vec![
                        owner.clone(),
                        name.clone(),
                        min,
                        max,
                        quote(&binding.strength),
                        value_sets.name_of(&binding.value_set).to_string(),
                    ],
                ),
                (None, Some(reference)) => Item::call(
                    "cr",
                    vec![owner.clone(), name.clone(), min, max, quote(reference)],
                ),
                (None, None) => Item::call("c", vec![owner.clone(), name.clone(), min, max]),
            };
            rows.push((ty.fhir_name.clone(), element.fhir_name.clone(), item));
        }
    }
    rows.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));

    let items: Vec<Item> = rows.into_iter().map(|(_, _, item)| item).collect();
    let mut out = String::from(
        "\n/// Every element of every generated type, by owning type and then by name.\n",
    );
    out.push_str(&array_statement(
        0,
        "pub const ELEMENTS: &[ElementMeta] = ",
        &items,
        ";",
    ));
    out
}

// ---------------------------------------------------------------------------
// Constant names
// ---------------------------------------------------------------------------

/// A FHIR name as a `SCREAMING_SNAKE_CASE` constant stem.
///
/// Derived from the whole name, so two names that differ produce two stems:
/// the dots of a backbone path become underscores and everything else that is
/// not alphanumeric goes the same way.
fn screaming(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_uppercase());
        } else {
            out.push('_');
        }
    }
    out
}

/// `stem`, or `stem_2`, `stem_3` … if it is taken.
fn unique(stem: &str, taken: &mut BTreeSet<String>) -> String {
    if taken.insert(stem.to_string()) {
        return stem.to_string();
    }
    for suffix in 2..1000 {
        let candidate = format!("{stem}_{suffix}");
        if taken.insert(candidate.clone()) {
            return candidate;
        }
    }
    unreachable!("a thousand constants cannot share one stem")
}
