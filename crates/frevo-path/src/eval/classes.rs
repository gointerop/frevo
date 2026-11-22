//! The HAPI R4 model class of a result, and its ancestry (`path.api.005`).
//!
//! `FhirPathR4` checks each result item with
//! `theReturnType.isAssignableFrom(next.getClass())`. The class a value has is
//! read from the generated class table `r4_classes.tsv` (`@generated` by
//! `frevo-codegen class-table`, `codegen.ancestry.001` to `.005`), which is
//! never hand-edited:
//!
//! - a value navigated to is looked up by the element it came from
//!   (`Patient.gender` is an `Enumeration`, `Patient.id` an `IdType`), by
//!   name or through `*` (`Narrative.div` is a `StringType` by name and an
//!   `XhtmlType` through `children()`). The element is recorded while the
//!   evaluation navigates, only when a typed evaluation asked for it;
//! - any other value by its type: a resource or datatype node by its FHIR
//!   type, a nested component by its path, and a value the engine builds by
//!   the class the engine builds (`'a'` is a `StringType`, `1 'mg'` a
//!   `Quantity`).
//!
//! Original Frevo work over the generated table.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use crate::seam::node::Node;
use crate::seam::value::Value;

/// The generated table, embedded.
const TABLE: &str = include_str!("r4_classes.tsv");

/// The Java package of the API interfaces the interface kinds name.
const API_PACKAGE: &str = "org.hl7.fhir.instance.model.api";

/// One class.
pub(crate) struct ClassRow {
    pub(crate) simple: String,
    pub(crate) qualified: String,
    assignable: Vec<String>,
    interfaces: Vec<String>,
}

struct Table {
    rows: Vec<ClassRow>,
    by_type: HashMap<String, usize>,
    by_component: HashMap<String, usize>,
    /// Element path to (by-name row, through-`*` row).
    elements: HashMap<String, (usize, usize)>,
}

fn table() -> &'static Table {
    static PARSED: OnceLock<Table> = OnceLock::new();
    PARSED.get_or_init(|| {
        let mut t = Table {
            rows: Vec::new(),
            by_type: HashMap::new(),
            by_component: HashMap::new(),
            elements: HashMap::new(),
        };
        for line in TABLE.lines() {
            let fields: Vec<&str> = line.split('\t').collect();
            match fields.as_slice() {
                ["row", of, name, simple, qualified, assignable, interfaces] => {
                    let at = t.rows.len();
                    match *of {
                        "type" => {
                            t.by_type.insert((*name).to_string(), at);
                        }
                        "component" => {
                            t.by_component.insert((*name).to_string(), at);
                        }
                        _ => {}
                    }
                    let list = |s: &str| -> Vec<String> {
                        s.split(',')
                            .filter(|x| !x.is_empty())
                            .map(str::to_string)
                            .collect()
                    };
                    t.rows.push(ClassRow {
                        simple: (*simple).to_string(),
                        qualified: (*qualified).to_string(),
                        assignable: list(assignable),
                        interfaces: list(interfaces),
                    });
                }
                ["element", path, by_name, through_star] => {
                    if let (Ok(a), Ok(b)) = (by_name.parse(), through_star.parse()) {
                        t.elements.insert((*path).to_string(), (a, b));
                    }
                }
                _ => {}
            }
        }
        t
    })
}

/// Where a navigated node came from: the owning type and the element name,
/// and whether it was reached through `*`.
struct Origin {
    /// Held so the address the key names cannot be reused while recorded.
    _node: Arc<dyn Node>,
    owner: String,
    name: String,
    star: bool,
}

thread_local! {
    static ORIGINS: RefCell<Option<HashMap<usize, Origin>>> = const { RefCell::new(None) };
}

fn key_of(node: &Arc<dyn Node>) -> usize {
    Arc::as_ptr(node).cast::<()>() as usize
}

/// Records where navigated nodes came from for as long as it lives. Only a
/// typed evaluation creates one, so an ordinary evaluation records nothing.
pub(crate) struct Recording {
    outer: Option<HashMap<usize, Origin>>,
}

impl Recording {
    pub(crate) fn begin() -> Recording {
        let outer = ORIGINS.with(|o| o.borrow_mut().replace(HashMap::new()));
        Recording { outer }
    }
}

impl Drop for Recording {
    fn drop(&mut self) {
        let outer = self.outer.take();
        ORIGINS.with(|o| *o.borrow_mut() = outer);
    }
}

/// Note that `child` was navigated to from a node of type `owner` through
/// the element `name` (`*` when `star`).
pub(crate) fn note(child: &Value, owner: &str, name: &str, star: bool) {
    let Value::Node(node) = child else {
        return;
    };
    ORIGINS.with(|o| {
        if let Some(map) = o.borrow_mut().as_mut() {
            map.entry(key_of(node)).or_insert_with(|| Origin {
                _node: Arc::clone(node),
                owner: owner.to_string(),
                name: name.to_string(),
                star,
            });
        }
    });
}

fn upper_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// The class of one result item, or `None` when the table has no row for it.
pub(crate) fn class_of(item: &Value) -> Option<&'static ClassRow> {
    let t = table();
    let by_type = |name: &str| t.by_type.get(name).map(|&i| &t.rows[i]);
    match item {
        Value::Boolean(_) => by_type("boolean"),
        Value::Integer(_) => by_type("integer"),
        Value::Decimal(_) => by_type("decimal"),
        Value::String(_) => by_type("string"),
        Value::Date(_) => by_type("date"),
        Value::DateTime(_) => by_type("dateTime"),
        Value::Time(_) => by_type("time"),
        Value::Quantity(_) => by_type("Quantity"),
        Value::Node(node) => {
            let from_origin = ORIGINS.with(|o| {
                let map = o.borrow();
                let origin = map.as_ref()?.get(&key_of(node))?;
                let plain = format!("{}.{}", origin.owner, origin.name);
                let choice = format!(
                    "{}.{}{}",
                    origin.owner,
                    origin.name,
                    upper_first(node.type_name())
                );
                let (by_name, star) = t.elements.get(&plain).or_else(|| t.elements.get(&choice))?;
                Some(if origin.star { *star } else { *by_name })
            });
            if let Some(i) = from_origin {
                return Some(&t.rows[i]);
            }
            let name = node.type_name();
            by_type(name).or_else(|| t.by_component.get(name).map(|&i| &t.rows[i]))
        }
    }
}

/// Whether `row` is assignable to the requested type, and the requested
/// type's Java name for the message: an interface kind, or a FHIR type name
/// standing for its model class. `None` when the request names no class the
/// table knows.
pub(crate) fn accepts(row: &ClassRow, requested: &str) -> (bool, String) {
    let t = table();
    if matches!(
        requested,
        "IPrimitiveType" | "ICompositeType" | "IBaseReference" | "IBase"
    ) {
        let ok = requested == "IBase" || row.interfaces.iter().any(|i| i == requested);
        return (ok, format!("{API_PACKAGE}.{requested}"));
    }
    let expected = t
        .by_type
        .get(requested)
        .map(|&i| t.rows[i].qualified.clone())
        .unwrap_or_else(|| requested.to_string());
    (row.assignable.iter().any(|a| a == requested), expected)
}
