//! The R4 model class table (`codegen.ancestry.001` to `.005`, task 14b).
//!
//! HAPI checks a typed evaluation's result with
//! `theReturnType.isAssignableFrom(next.getClass())` (`path.api.005`), so
//! reproducing it needs, for every value the engine can hand back, the HAPI R4
//! model class it would be and that class's ancestry. This module derives that
//! table from the pinned R4 StructureDefinitions and one reviewed exception
//! file, `crates/frevo-codegen/data/r4-class-table-exceptions.toml`, by the
//! rules the spec fixes:
//!
//! - rows (`codegen.ancestry.001`): one per FHIR type of kind
//!   `primitive-type`, `complex-type` and `resource` defined by specialization
//!   (abstract ones included), one per nested component, one for
//!   `Enumeration`;
//! - names and ancestry (`codegen.ancestry.002`): a primitive is its name
//!   upper-cased on its first letter plus `Type`; a complex type or resource
//!   its name, `List` being `ListResource`; a nested component its
//!   `structuredefinition-explicit-type-name` plus `Component`, or else its
//!   path's segments, each upper-cased on its first letter, plus `Component`;
//!   a class is assignable to its own FHIR type and every type on its
//!   `baseDefinition` chain that is itself a row;
//! - interface kinds: `IBase` everywhere, `IPrimitiveType` on primitives and
//!   `Enumeration`, `ICompositeType` on concrete complex types, and
//!   `IBaseReference` on `Reference`;
//! - an element's value class is its declared type's row; a `code` element is
//!   `Enumeration` exactly when `codegen.ancestry.003`'s rule holds;
//! - then the exception file (`codegen.ancestry.004`), each of whose entries
//!   must cite its source, name a target in the package and change the
//!   table, or the run is refused.
//!
//! No `.java` file is read: the request's `reference_dir` is accepted and
//! never opened. Original Frevo work, not a port.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use crate::emit::{GENERATOR_VERSION, GeneratedFile};
use crate::error::{CodegenError, FailureKind, Result};
use crate::model::{self, Model, TypeKind};
use crate::package::{self, Package, SnapshotElement};
use crate::pin::{self, GENERATOR_NAME, PACKAGE_NAME, PACKAGE_VERSION};
use crate::sha256;

/// The reviewed exception file, relative to the repository root.
pub const EXCEPTIONS_FILE: &str = "crates/frevo-codegen/data/r4-class-table-exceptions.toml";
/// Where `frevo-path` embeds the table it evaluates against.
pub const CHECKED_IN_TABLE: &str = "crates/frevo-path/src/eval/r4_classes.tsv";
/// The file a run writes, relative to its output directory.
pub const TABLE_FILE: &str = "r4_classes.tsv";
/// The Java package every model class lives in.
pub const MODEL_PACKAGE: &str = "org.hl7.fhir.r4.model";

/// Binding names HAPI holds as `CodeType` although their binding is closed
/// (`codegen.ancestry.003`).
const TYPE_LIST_BINDINGS: [&str; 3] = ["ResourceType", "FHIRAllTypes", "FHIRDefinedType"];

/// One class-table run's inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub exceptions: PathBuf,
    pub out_dir: PathBuf,
    /// Where `reference/` would be. Never read.
    pub reference_dir: Option<PathBuf>,
}

/// The four interface kinds a row records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Interface {
    IBase,
    IPrimitiveType,
    ICompositeType,
    IBaseReference,
}

impl Interface {
    fn as_str(self) -> &'static str {
        match self {
            Interface::IBase => "IBase",
            Interface::IPrimitiveType => "IPrimitiveType",
            Interface::ICompositeType => "ICompositeType",
            Interface::IBaseReference => "IBaseReference",
        }
    }
}

/// What a row stands for.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RowOf {
    /// A FHIR type, by its name.
    Type(String),
    /// A nested component, by its element path.
    Component(String),
    /// HAPI's `Enumeration`.
    Enumeration,
}

/// One row of the table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub of: RowOf,
    pub simple_name: String,
    pub qualified_name: String,
    pub assignable_to: BTreeSet<String>,
    pub interfaces: BTreeSet<Interface>,
}

/// The class an element's value is held in, by row qualified name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementClass {
    pub by_name: String,
    pub through_star: String,
}

/// One run's result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassTable {
    pub file: GeneratedFile,
    pub inputs_read: Vec<PathBuf>,
    pub rows: Vec<Row>,
    pub elements: BTreeMap<String, ElementClass>,
}

/// One exception entry: its `key = "value"` pairs, and the line it starts on.
#[derive(Debug, Clone)]
struct Entry {
    line: usize,
    pairs: BTreeMap<String, String>,
}

fn refuse(message: impl Into<String>) -> CodegenError {
    CodegenError::failed(FailureKind::Other, message)
}

/// Read the exception file: `[[exception]]` tables of string pairs, `#`
/// comments. Anything else is refused.
fn read_entries(text: &str, path: &Path) -> Result<Vec<Entry>> {
    let mut entries: Vec<Entry> = Vec::new();
    for (n, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "[[exception]]" {
            entries.push(Entry {
                line: n + 1,
                pairs: BTreeMap::new(),
            });
            continue;
        }
        let where_ = || format!("{}:{}", path.display(), n + 1);
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| refuse(format!("{}: not `key = \"value\"`: {raw}", where_())))?;
        let value = value
            .trim()
            .strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .ok_or_else(|| refuse(format!("{}: a value must be a string: {raw}", where_())))?;
        let entry = entries
            .last_mut()
            .ok_or_else(|| refuse(format!("{}: a key outside any [[exception]]", where_())))?;
        if entry
            .pairs
            .insert(key.trim().to_string(), value.to_string())
            .is_some()
        {
            return Err(refuse(format!(
                "{}: `{}` is given twice",
                where_(),
                key.trim()
            )));
        }
    }
    Ok(entries)
}

fn upper_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// What the generator needs of the package beyond the model: each
/// definition's snapshot by path, and which definitions are constraints.
struct Inputs<'p> {
    snapshots: HashMap<&'p str, HashMap<&'p str, &'p SnapshotElement>>,
    constraints: BTreeSet<&'p str>,
    package: &'p Package,
}

impl<'p> Inputs<'p> {
    fn of(package: &'p Package) -> Inputs<'p> {
        let mut snapshots = HashMap::new();
        let mut constraints = BTreeSet::new();
        for definition in &package.definitions {
            // Only the core definition of each type: an extension or profile
            // may share a type's name (`rendering-markdown` is named
            // `markdown`), but its URL does not end in that name.
            if definition.url
                != format!(
                    "http://hl7.org/fhir/StructureDefinition/{}",
                    definition.name
                )
            {
                continue;
            }
            if definition.derivation.as_deref() == Some("constraint") {
                constraints.insert(definition.name.as_str());
            }
            snapshots.insert(
                definition.name.as_str(),
                definition
                    .snapshot
                    .iter()
                    .map(|e| (e.path.as_str(), e))
                    .collect(),
            );
        }
        Inputs {
            snapshots,
            constraints,
            package,
        }
    }

    /// The snapshot element of `definition` at `path`.
    fn element(&self, definition: &str, path: &str) -> Option<&'p SnapshotElement> {
        self.snapshots.get(definition)?.get(path).copied()
    }

    /// Whether the value set behind `url` is closed by `codegen.ancestry.003`:
    /// every include lists its concepts, or names a complete code system in
    /// the package, with no filter; an imported value set closed the same way.
    fn closed(&self, url: &str, seen: &mut BTreeSet<String>) -> bool {
        let url = url.split('|').next().unwrap_or(url);
        if !seen.insert(url.to_string()) {
            return false;
        }
        let Some(value_set) = self.package.value_sets.iter().find(|v| v.url == url) else {
            return false;
        };
        if !value_set.has_compose || value_set.includes.is_empty() {
            return false;
        }
        value_set.includes.iter().all(|include| {
            if include.has_filter {
                return false;
            }
            let system_ok = match &include.system {
                // An include with no system must import a value set.
                None => !include.imports.is_empty(),
                Some(_) if !include.concepts.is_empty() => true,
                Some(system) => self
                    .package
                    .code_systems
                    .iter()
                    .any(|c| &c.url == system && c.content.as_deref() == Some("complete")),
            };
            system_ok
                && include
                    .imports
                    .iter()
                    .all(|imported| self.closed(imported, &mut seen.clone()))
        })
    }
}

/// A row being built, before the exceptions and the final ordering.
struct Draft {
    rows: Vec<Row>,
    /// Row index by what it stands for.
    index: BTreeMap<RowOf, usize>,
    /// Each row type's immediate base, for re-deriving an ancestry.
    bases: BTreeMap<String, Option<String>>,
}

impl Draft {
    fn row(&self, of: &RowOf) -> Option<&Row> {
        self.index.get(of).map(|&i| &self.rows[i])
    }

    /// The FHIR types on `name`'s `baseDefinition` chain that are rows,
    /// `name` included.
    fn chain(&self, name: &str) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        let mut at = Some(name.to_string());
        while let Some(current) = at {
            if !self.index.contains_key(&RowOf::Type(current.clone()))
                || !out.insert(current.clone())
            {
                break;
            }
            at = self.bases.get(&current).cloned().flatten();
        }
        out
    }

    fn qualified(&self, of: &RowOf) -> Option<String> {
        self.row(of).map(|r| r.qualified_name.clone())
    }
}

/// One `frevo-codegen class-table` run.
pub fn generate(request: &Request) -> Result<ClassTable> {
    let dir = pin::default_package_dir();
    let loaded = package::load(&dir)?;
    package::verify_digest(&loaded, pin::PACKAGE_CONTENT_SHA256)?;
    let model = model::from_package(&loaded)?;
    let inputs = Inputs::of(&loaded);

    let exceptions_text = std::fs::read_to_string(&request.exceptions).map_err(|e| {
        CodegenError::failed(
            FailureKind::PackageMissing,
            format!(
                "cannot read the exception file {}: {e}",
                request.exceptions.display()
            ),
        )
    })?;
    let entries = read_entries(&exceptions_text, &request.exceptions)?;

    let mut draft = rows(&model, &inputs);
    let mut elements = element_classes(&model, &inputs, &draft)?;
    apply(
        &entries,
        &request.exceptions,
        &model,
        &mut draft,
        &mut elements,
    )?;

    let mut inputs_read = loaded.files_read.clone();
    inputs_read.push(request.exceptions.clone());

    let file = emit(&draft.rows, &elements, &exceptions_text);
    std::fs::create_dir_all(&request.out_dir).map_err(|e| {
        CodegenError::other(format!("cannot create {}: {e}", request.out_dir.display()))
    })?;
    let out = request.out_dir.join(TABLE_FILE);
    std::fs::write(&out, &file.bytes)
        .map_err(|e| CodegenError::other(format!("cannot write {}: {e}", out.display())))?;

    Ok(ClassTable {
        file,
        inputs_read,
        rows: draft.rows,
        elements,
    })
}

/// The rows, by `codegen.ancestry.001` and `.002`, before the exceptions.
fn rows(model: &Model, inputs: &Inputs<'_>) -> Draft {
    let mut draft = Draft {
        rows: Vec::new(),
        index: BTreeMap::new(),
        bases: BTreeMap::new(),
    };
    let is_type_row = |t: &model::Type| {
        matches!(
            t.kind,
            TypeKind::Primitive
                | TypeKind::Complex
                | TypeKind::Resource
                | TypeKind::AbstractElementSet
        ) && !inputs.constraints.contains(t.fhir_name.as_str())
    };
    // Type rows first, so a component can name its outer class.
    for ty in model.types.iter().filter(|t| is_type_row(t)) {
        let simple = match ty.kind {
            TypeKind::Primitive => format!("{}Type", upper_first(&ty.fhir_name)),
            _ if ty.fhir_name == "List" => "ListResource".to_string(),
            _ => ty.fhir_name.clone(),
        };
        let mut interfaces = BTreeSet::from([Interface::IBase]);
        match ty.kind {
            TypeKind::Primitive => {
                interfaces.insert(Interface::IPrimitiveType);
            }
            TypeKind::Complex if ty.instantiable => {
                interfaces.insert(Interface::ICompositeType);
                if ty.fhir_name == "Reference" {
                    interfaces.insert(Interface::IBaseReference);
                }
            }
            _ => {}
        }
        let of = RowOf::Type(ty.fhir_name.clone());
        draft.index.insert(of.clone(), draft.rows.len());
        draft.bases.insert(ty.fhir_name.clone(), ty.base.clone());
        draft.rows.push(Row {
            of,
            qualified_name: format!("{MODEL_PACKAGE}.{simple}"),
            simple_name: simple,
            assignable_to: BTreeSet::new(),
            interfaces,
        });
    }
    let type_names: Vec<String> = draft.bases.keys().cloned().collect();
    for name in type_names {
        let chain = draft.chain(&name);
        let i = draft.index[&RowOf::Type(name)];
        draft.rows[i].assignable_to = chain;
    }

    for ty in model.types.iter().filter(|t| t.kind == TypeKind::Backbone) {
        let root = ty.fhir_name.split('.').next().unwrap_or(&ty.fhir_name);
        // The `constraint` profiles (`SimpleQuantity`, `MoneyQuantity`) are not
        // rows, and nor is anything nested in one.
        if !draft.index.contains_key(&RowOf::Type(root.into())) {
            continue;
        }
        let explicit = inputs
            .element(root, &ty.fhir_name)
            .and_then(|e| e.explicit_type_name.clone());
        let simple = match explicit {
            Some(name) => format!("{}Component", upper_first(&name)),
            None => {
                let joined: String = ty.fhir_name.split('.').map(upper_first).collect();
                format!("{joined}Component")
            }
        };
        let outer = draft
            .qualified(&RowOf::Type(root.to_string()))
            .unwrap_or_else(|| format!("{MODEL_PACKAGE}.{root}"));
        let base = ty
            .base
            .clone()
            .unwrap_or_else(|| "BackboneElement".to_string());
        let of = RowOf::Component(ty.fhir_name.clone());
        draft.index.insert(of.clone(), draft.rows.len());
        draft.rows.push(Row {
            of,
            qualified_name: format!("{outer}${simple}"),
            simple_name: simple,
            assignable_to: draft.chain(&base),
            interfaces: BTreeSet::from([Interface::IBase]),
        });
    }

    draft.index.insert(RowOf::Enumeration, draft.rows.len());
    draft.rows.push(Row {
        of: RowOf::Enumeration,
        simple_name: "Enumeration".to_string(),
        qualified_name: format!("{MODEL_PACKAGE}.Enumeration"),
        assignable_to: BTreeSet::from(["Element".to_string()]),
        interfaces: BTreeSet::from([Interface::IBase, Interface::IPrimitiveType]),
    });
    draft
}

/// Whether `ty` contributes rows and elements.
fn is_row_type(draft: &Draft, ty: &model::Type) -> bool {
    match ty.kind {
        TypeKind::Backbone => draft
            .index
            .contains_key(&RowOf::Component(ty.fhir_name.clone())),
        _ => draft.index.contains_key(&RowOf::Type(ty.fhir_name.clone())),
    }
}

/// Every element's value class, before the exceptions.
fn element_classes(
    model: &Model,
    inputs: &Inputs<'_>,
    draft: &Draft,
) -> Result<BTreeMap<String, ElementClass>> {
    let enumeration = draft.qualified(&RowOf::Enumeration).unwrap_or_default();
    let mut out = BTreeMap::new();
    for ty in model.types.iter().filter(|t| is_row_type(draft, t)) {
        let root = ty.fhir_name.split('.').next().unwrap_or(&ty.fhir_name);
        for element in &ty.elements {
            // A primitive's `value` is a Java value, not a model class.
            if ty.kind == TypeKind::Primitive && element.fhir_name == "value" {
                continue;
            }
            // The element's own path, and the same rebased onto the owning
            // type's name (they differ only for a `Quantity` specialisation,
            // whose snapshot is rooted at `Quantity`).
            let (owner_raw, _) = element
                .path
                .rsplit_once('.')
                .unwrap_or((element.path.as_str(), ""));
            let owners: BTreeSet<String> = [owner_raw.to_string(), ty.fhir_name.clone()]
                .into_iter()
                .collect();
            let snapshot = inputs.element(root, &element.path);
            for (i, declared) in element.types.iter().enumerate() {
                let class = value_class(element, declared, inputs, draft, snapshot, &enumeration)
                    .ok_or_else(|| {
                    CodegenError::other(format!(
                        "{} declares {declared}, which has no row in the class table",
                        element.path
                    ))
                })?;
                let property = if element.is_choice {
                    element
                        .json_property_names
                        .get(i)
                        .cloned()
                        .unwrap_or_default()
                } else {
                    element.fhir_name.clone()
                };
                for owner in &owners {
                    out.insert(
                        format!("{owner}.{property}"),
                        ElementClass {
                            by_name: class.clone(),
                            through_star: class.clone(),
                        },
                    );
                }
            }
            if element.types.is_empty() {
                // A `contentReference` element declares no type.
                if let Some(reference) = &element.content_reference {
                    let target = reference.trim_start_matches('#');
                    let class = draft
                        .qualified(&RowOf::Component(target.to_string()))
                        .ok_or_else(|| {
                            CodegenError::other(format!(
                                "{} references {reference}, which is not a component row",
                                element.path
                            ))
                        })?;
                    for owner in &owners {
                        out.insert(
                            format!("{owner}.{}", element.fhir_name),
                            ElementClass {
                                by_name: class.clone(),
                                through_star: class.clone(),
                            },
                        );
                    }
                }
            }
        }
    }
    Ok(out)
}

/// The class one declared type of an element is held in.
fn value_class(
    element: &model::Element,
    declared: &str,
    inputs: &Inputs<'_>,
    draft: &Draft,
    snapshot: Option<&SnapshotElement>,
    enumeration: &str,
) -> Option<String> {
    if let Some(reference) = &element.content_reference {
        return draft.qualified(&RowOf::Component(
            reference.trim_start_matches('#').to_string(),
        ));
    }
    if matches!(declared, "BackboneElement" | "Element")
        && let Some(component) = draft.qualified(&RowOf::Component(element.path.clone()))
    {
        return Some(component);
    }
    if declared == "code" && element.types.len() == 1 {
        let binding = snapshot.and_then(|s| s.binding.as_ref());
        let is_enumeration = binding.is_some_and(|b| {
            b.strength == "required"
                && !b
                    .binding_name
                    .as_deref()
                    .is_some_and(|n| TYPE_LIST_BINDINGS.contains(&n))
                && inputs.closed(&b.value_set, &mut BTreeSet::new())
        });
        if is_enumeration {
            return Some(enumeration.to_string());
        }
    }
    // A `System.*` type was read through its `structuredefinition-fhir-type`
    // extension (`package::fhir_type_of`), so `Element.id` is `string` and
    // `Extension.url` is `uri`. `xhtml.id` declares `System.String` with no
    // such extension; the package reader's fallback reads it as `string`, the
    // type `Element.id` has, so it is a `StringType` like every other
    // element's `id`. The spec fixes no rule for it (a gap the proposer is
    // closing); this is the reading the rest of the table implies.
    draft.qualified(&RowOf::Type(declared.to_string()))
}

/// Apply the exception file, refusing an entry with no source, an unknown
/// target or no effect.
fn apply(
    entries: &[Entry],
    path: &Path,
    model: &Model,
    draft: &mut Draft,
    elements: &mut BTreeMap<String, ElementClass>,
) -> Result<()> {
    for entry in entries {
        let at = format!("{}:{}", path.display(), entry.line);
        let get = |key: &str| -> Result<&str> {
            entry
                .pairs
                .get(key)
                .map(String::as_str)
                .filter(|v| !v.is_empty())
                .ok_or_else(|| refuse(format!("{at}: the entry has no `{key}`")))
        };
        let source = get("source")?;
        if !source.contains(".java:") {
            return Err(refuse(format!(
                "{at}: `source` must cite <file>.java:<line>, not {source:?}"
            )));
        }
        let kind = get("kind")?;
        let target = get("target")?;
        let no_effect = || {
            refuse(format!(
                "{at}: the {kind} entry for {target} changes nothing"
            ))
        };
        match kind {
            "superclass" => {
                let extends = get("extends")?;
                let i = *draft
                    .index
                    .get(&RowOf::Type(target.to_string()))
                    .ok_or_else(|| {
                        refuse(format!("{at}: {target} is not a type in the package"))
                    })?;
                if !draft.index.contains_key(&RowOf::Type(extends.to_string())) {
                    return Err(refuse(format!(
                        "{at}: {extends} is not a type in the package"
                    )));
                }
                let mut assignable = draft.chain(extends);
                assignable.insert(target.to_string());
                if assignable == draft.rows[i].assignable_to {
                    return Err(no_effect());
                }
                draft.rows[i].assignable_to = assignable;
            }
            "class-name" => {
                let name = get("class_name")?;
                let i = *draft
                    .index
                    .get(&RowOf::Component(target.to_string()))
                    .ok_or_else(|| {
                        refuse(format!("{at}: {target} is not a component in the package"))
                    })?;
                if draft.rows[i].simple_name == name {
                    return Err(no_effect());
                }
                let old = draft.rows[i].qualified_name.clone();
                let outer = old
                    .rsplit_once('$')
                    .map(|(o, _)| o.to_string())
                    .unwrap_or_default();
                let new = format!("{outer}${name}");
                draft.rows[i].simple_name = name.to_string();
                draft.rows[i].qualified_name = new.clone();
                for class in elements.values_mut() {
                    if class.by_name == old {
                        class.by_name = new.clone();
                    }
                    if class.through_star == old {
                        class.through_star = new.clone();
                    }
                }
            }
            "element-class" | "runtime-class" => {
                let class_type = get(if kind == "element-class" {
                    "class"
                } else {
                    "by_name"
                })?;
                let class = draft
                    .qualified(&RowOf::Type(class_type.to_string()))
                    .ok_or_else(|| refuse(format!("{at}: {class_type} is not a type row")))?;
                if !elements.contains_key(target) {
                    return Err(refuse(format!(
                        "{at}: {target} is not an element in the package"
                    )));
                }
                // `Resource.id` is inherited by every resource: the entry
                // applies to the `id` of every resource and abstract resource.
                let mut keys = vec![target.to_string()];
                if target == "Resource.id" {
                    keys.extend(
                        model
                            .types
                            .iter()
                            .filter(|t| {
                                matches!(t.kind, TypeKind::Resource | TypeKind::AbstractElementSet)
                            })
                            .map(|t| format!("{}.id", t.fhir_name)),
                    );
                }
                let mut changed = false;
                for key in keys {
                    if let Some(current) = elements.get_mut(&key) {
                        if current.by_name != class {
                            changed = true;
                        }
                        current.by_name = class.clone();
                        if kind == "element-class" {
                            current.through_star = class.clone();
                        }
                    }
                }
                if !changed {
                    return Err(no_effect());
                }
            }
            other => {
                return Err(refuse(format!(
                    "{at}: unknown kind {other:?}; one of superclass, element-class, \
                     runtime-class, class-name"
                )));
            }
        }
    }
    Ok(())
}

/// The table file: tab-separated, one `row` line per class and one `element`
/// line per element path, in a fixed order, under the generated header.
fn emit(
    rows: &[Row],
    elements: &BTreeMap<String, ElementClass>,
    exceptions: &str,
) -> GeneratedFile {
    let recorded = pin::recorded_pin();
    let mut order: Vec<&Row> = rows.iter().collect();
    order.sort_by(|a, b| {
        a.qualified_name
            .cmp(&b.qualified_name)
            .then(a.of.cmp(&b.of))
    });
    let mut text = format!(
        "# @generated by {GENERATOR_NAME} {GENERATOR_VERSION} -- do not edit.\n\
         #\n\
         # Source package: {PACKAGE_NAME} {PACKAGE_VERSION}\n\
         # Package SHA-256: {}\n\
         # Exception file: {EXCEPTIONS_FILE} (SHA-256 {})\n\
         #\n\
         # The R4 model class table (codegen.ancestry.001 to .005). Every edit to\n\
         # this file is overwritten by the next run of\n\
         # `{GENERATOR_NAME} class-table --version r4`; a wrong row is fixed in the\n\
         # generator or by a reviewed exception entry, never by editing here.\n\
         #\n\
         # row\\t<of>\\t<name>\\t<simple>\\t<qualified>\\t<assignable,...>\\t<interfaces,...>\n\
         # element\\t<path>\\t<by-name row>\\t<through-star row>\n\
         #\n\
         # A row number counts the `row` lines from 0, in file order.\n",
        recorded.sha256,
        sha256::hex_of(exceptions.replace("\r\n", "\n").as_bytes()),
    );
    for row in &order {
        let (of, name) = match &row.of {
            RowOf::Type(n) => ("type", n.as_str()),
            RowOf::Component(p) => ("component", p.as_str()),
            RowOf::Enumeration => ("enumeration", "-"),
        };
        let assignable: Vec<&str> = row.assignable_to.iter().map(String::as_str).collect();
        let interfaces: Vec<&str> = row.interfaces.iter().map(|i| i.as_str()).collect();
        text.push_str(&format!(
            "row\t{of}\t{name}\t{}\t{}\t{}\t{}\n",
            row.simple_name,
            row.qualified_name,
            assignable.join(","),
            interfaces.join(",")
        ));
    }
    let number: BTreeMap<&str, usize> = order
        .iter()
        .enumerate()
        .map(|(i, row)| (row.qualified_name.as_str(), i))
        .collect();
    for (path, class) in elements {
        let at = |q: &str| number.get(q).copied().unwrap_or(usize::MAX);
        text.push_str(&format!(
            "element\t{path}\t{}\t{}\n",
            at(&class.by_name),
            at(&class.through_star)
        ));
    }
    GeneratedFile::new(TABLE_FILE, &text)
}
