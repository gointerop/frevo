//! The `model-nav` in-process oracle: the navigation surface `frevo-path`
//! evaluates over.
//!
//! Task 5 of `add-fhirpath-and-codegen-r4`. Run it with:
//!
//! ```text
//! cargo test -p frevo-model --test model_nav
//! ```
//!
//! Scenario coverage: `model.nav.001` .. `model.nav.012`.
//!
//! ## What the seam mirrors
//!
//! `design.md` section 4.2 and 4.3: the `Node` trait, whose returns are
//! **owned** so the engine holds no borrow into the model, and the `Model`
//! trait that answers type-hierarchy and element-type questions. ADR 0004
//! decision 5 is the reason both exist: the lesson it draws from
//! `haste-fhirpath` is 77 `downcast_ref` calls to one model's concrete types,
//! with FHIRPath turning a failed downcast into an empty collection rather than
//! an error.
//!
//! The seam here is addressed by element path rather than by handing trait
//! objects around, because a path-addressed oracle can be driven before any
//! node type exists. The four load-bearing properties of the real signature are
//! each asserted:
//!
//! - **owned returns** -- `model.nav.007`, through
//!   [`NavOracle::children_outliving_their_parent`];
//! - **`Result`, not `Option`** -- `model.nav.005`, where an absent element is
//!   `Ok(empty)` and not an error, so a malformed document stays
//!   distinguishable from an absent one;
//! - **`children(name)` takes a base name** -- `model.nav.003` and
//!   `model.nav.004`;
//! - **no `resource()` on the node** -- `model.nav.008` asks each node whether
//!   it *is* a resource, which is the only question a span-based handle can
//!   answer without knowing its ancestors.
//!
//! ## `model.nav.012` and the test-only node
//!
//! The scenario requires the same navigation over a `frevo-model` node and over
//! "the harness's test-only node built from an XML oracle input", with equal
//! results. The XML-backed node is implemented **in this file**, over a
//! minimal XML reader, and it works: the comparison therefore fails on the
//! `frevo-model` side alone, which is the side that does not exist yet.
//!
//! The comparison is over what both nodes can answer -- element names,
//! primitive values and document order. FHIR type names are model information,
//! which a node built from an XML document with no StructureDefinitions behind
//! it does not have, so the test-only node declines those questions rather than
//! guessing and the comparison does not include them.
//!
//! ## The seam
//!
//! The unbound binding returns [`NavError::NotImplemented`] from every surface,
//! and every helper turns that into an explicit "nothing is bound yet" failure
//! rather than letting it read as a pass.

use std::fmt;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// The seam
// ---------------------------------------------------------------------------

/// One child value, owned, paired with the element name it came back under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Child {
    /// The element name, with no `[x]` suffix and no typed spelling.
    pub name: String,
    /// The FHIR type name of the value.
    pub type_name: String,
    /// The primitive value, or `None` when the node has none -- which is
    /// exactly what `hasValue()` reports.
    pub primitive: Option<String>,
    pub is_resource: bool,
}

impl Child {
    /// The projection both the `frevo-model` node and the test-only node can
    /// answer: the element name, the primitive value, in document order.
    fn portable(&self) -> (String, Option<String>) {
        (self.name.clone(), self.primitive.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavError {
    /// Navigation failed -- a decode error, which must propagate as an error
    /// rather than as an empty collection.
    Failed(String),
    /// The node cannot answer this question at all. The test-only node uses
    /// this for model information it does not have.
    Unanswerable(String),
    /// No model is bound to this oracle yet.
    NotImplemented,
}

impl fmt::Display for NavError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NavError::Failed(m) => write!(f, "navigation failed: {m}"),
            NavError::Unanswerable(m) => write!(f, "the node cannot answer that: {m}"),
            NavError::NotImplemented => f.write_str(NOT_BOUND),
        }
    }
}

const NOT_BOUND: &str = "no model is bound to the model-nav oracle yet \
     (crates/frevo-model/tests/model_nav.rs::node_under_test)";

/// The node-navigation surface, addressed by element path. `path` is rooted at
/// the resource type: `Patient`, `Patient.name[0]`,
/// `Patient.name[0].given[1]`.
pub trait NavOracle {
    /// The FHIR type name of the node at `path`.
    fn type_name_at(&self, doc: &str, path: &str) -> Result<String, NavError>;

    /// Whether the node at `path` is a resource.
    fn is_resource_at(&self, doc: &str, path: &str) -> Result<bool, NavError>;

    /// The children of `path` under the element name `name`, in document order,
    /// owned. `name` carries no `[x]` suffix: a choice element is reached by
    /// its base name and the typed spelling yields nothing.
    fn children(&self, doc: &str, path: &str, name: &str) -> Result<Vec<Child>, NavError>;

    /// Every child of `path`, paired with its element name, in document order.
    fn all_children(&self, doc: &str, path: &str) -> Result<Vec<Child>, NavError>;

    /// The primitive value of the node at `path`, or `None` when it has none.
    fn primitive_at(&self, doc: &str, path: &str) -> Result<Option<String>, NavError>;

    /// Obtain the children of `path` under `name`, **drop the node they came
    /// from**, and then return them. The values are owned, so this is possible
    /// at all; a borrow with the receiver's lifetime would not compile, which
    /// is the point of `model.nav.007`.
    fn children_outliving_their_parent(
        &self,
        doc: &str,
        path: &str,
        name: &str,
    ) -> Result<Vec<Child>, NavError>;
}

/// The model-information surface.
pub trait ModelInfo {
    /// The immediate base type: `code` -> `string`, `Age` -> `Quantity`.
    fn base_type(&self, type_name: &str) -> Result<Option<String>, NavError>;

    /// Whether `type_name` derives from `ancestor`, transitively.
    fn derives_from(&self, type_name: &str, ancestor: &str) -> Result<bool, NavError>;

    /// The declared types of an element: for a choice element, the full choice
    /// list.
    fn element_types(
        &self,
        type_name: &str,
        element: &str,
    ) -> Result<Option<Vec<String>>, NavError>;
}

/// The `frevo-model` node this oracle grades. Phase C replaces this body with
/// the real one.
fn node_under_test() -> Box<dyn NavOracle> {
    Box::new(UnboundModel)
}

/// The `frevo-model` model information this oracle grades.
fn model_under_test() -> Box<dyn ModelInfo> {
    Box::new(UnboundModel)
}

struct UnboundModel;

impl NavOracle for UnboundModel {
    fn type_name_at(&self, _doc: &str, _path: &str) -> Result<String, NavError> {
        Err(NavError::NotImplemented)
    }

    fn is_resource_at(&self, _doc: &str, _path: &str) -> Result<bool, NavError> {
        Err(NavError::NotImplemented)
    }

    fn children(&self, _doc: &str, _path: &str, _name: &str) -> Result<Vec<Child>, NavError> {
        Err(NavError::NotImplemented)
    }

    fn all_children(&self, _doc: &str, _path: &str) -> Result<Vec<Child>, NavError> {
        Err(NavError::NotImplemented)
    }

    fn primitive_at(&self, _doc: &str, _path: &str) -> Result<Option<String>, NavError> {
        Err(NavError::NotImplemented)
    }

    fn children_outliving_their_parent(
        &self,
        _doc: &str,
        _path: &str,
        _name: &str,
    ) -> Result<Vec<Child>, NavError> {
        Err(NavError::NotImplemented)
    }
}

impl ModelInfo for UnboundModel {
    fn base_type(&self, _type_name: &str) -> Result<Option<String>, NavError> {
        Err(NavError::NotImplemented)
    }

    fn derives_from(&self, _type_name: &str, _ancestor: &str) -> Result<bool, NavError> {
        Err(NavError::NotImplemented)
    }

    fn element_types(
        &self,
        _type_name: &str,
        _element: &str,
    ) -> Result<Option<Vec<String>>, NavError> {
        Err(NavError::NotImplemented)
    }
}

fn must_children(doc: &str, path: &str, name: &str, what: &str) -> Vec<Child> {
    match node_under_test().children(doc, path, name) {
        Ok(c) => c,
        Err(e) => {
            panic!("{what}: asking for the {name:?} children of {path} must succeed, but: {e}")
        }
    }
}

fn must_all_children(doc: &str, path: &str, what: &str) -> Vec<Child> {
    match node_under_test().all_children(doc, path) {
        Ok(c) => c,
        Err(e) => panic!("{what}: enumerating the children of {path} must succeed, but: {e}"),
    }
}

fn must_type_name(doc: &str, path: &str, what: &str) -> String {
    match node_under_test().type_name_at(doc, path) {
        Ok(t) => t,
        Err(e) => panic!("{what}: the type name of {path} must be reportable, but: {e}"),
    }
}

// ---------------------------------------------------------------------------
// A minimal XML reader and the harness's test-only node
//
// frevo-model has no dependencies and this change adds none. The reader handles
// what FHIR XML oracle inputs are: elements, a `value` attribute, no mixed
// content. It is deliberately small -- it exists so model.nav.012 has a second,
// genuinely different node implementation to compare against, not to be a
// general XML parser.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
struct XmlElement {
    name: String,
    value: Option<String>,
    children: Vec<XmlElement>,
}

fn parse_xml(src: &str) -> Result<XmlElement, String> {
    let b = src.as_bytes();
    let mut at = 0usize;
    loop {
        skip_xml_trivia(b, &mut at);
        if b.get(at) == Some(&b'<') && matches!(b.get(at + 1), Some(b'?') | Some(b'!')) {
            skip_xml_prolog(b, &mut at)?;
            continue;
        }
        break;
    }
    let root = parse_xml_element(b, &mut at)?;
    skip_xml_trivia(b, &mut at);
    Ok(root)
}

fn skip_xml_trivia(b: &[u8], at: &mut usize) {
    while matches!(b.get(*at), Some(b' ' | b'\t' | b'\r' | b'\n')) {
        *at += 1;
    }
}

fn skip_xml_prolog(b: &[u8], at: &mut usize) -> Result<(), String> {
    if b[*at..].starts_with(b"<!--") {
        let end = find_from(b, *at, b"-->").ok_or("unterminated comment")?;
        *at = end + 3;
        return Ok(());
    }
    let end = find_from(b, *at, b">").ok_or("unterminated prolog")?;
    *at = end + 1;
    Ok(())
}

fn find_from(b: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    (from..b.len().saturating_sub(needle.len() - 1)).find(|&i| b[i..].starts_with(needle))
}

fn parse_xml_element(b: &[u8], at: &mut usize) -> Result<XmlElement, String> {
    if b.get(*at) != Some(&b'<') {
        return Err(format!("expected an element at offset {at}"));
    }
    *at += 1;
    let name_start = *at;
    while matches!(b.get(*at), Some(c) if !c.is_ascii_whitespace() && *c != b'>' && *c != b'/') {
        *at += 1;
    }
    let name = String::from_utf8_lossy(&b[name_start..*at]).into_owned();

    // Attributes. Only `value` is meaningful in FHIR XML; `xmlns` and anything
    // else is recorded nowhere, which is what the oracle inputs need.
    let mut value = None;
    loop {
        skip_xml_trivia(b, at);
        match b.get(*at) {
            Some(b'/') => {
                *at += 1;
                if b.get(*at) != Some(&b'>') {
                    return Err(format!("expected > at offset {at}"));
                }
                *at += 1;
                return Ok(XmlElement {
                    name,
                    value,
                    children: Vec::new(),
                });
            }
            Some(b'>') => {
                *at += 1;
                break;
            }
            Some(_) => {
                let key_start = *at;
                while matches!(b.get(*at), Some(c) if *c != b'=' && !c.is_ascii_whitespace()) {
                    *at += 1;
                }
                let key = String::from_utf8_lossy(&b[key_start..*at]).into_owned();
                skip_xml_trivia(b, at);
                if b.get(*at) != Some(&b'=') {
                    return Err(format!("expected = after {key} at offset {at}"));
                }
                *at += 1;
                skip_xml_trivia(b, at);
                let quote = *b.get(*at).ok_or("unterminated attribute")?;
                if quote != b'"' && quote != b'\'' {
                    return Err(format!("expected a quoted value at offset {at}"));
                }
                *at += 1;
                let v_start = *at;
                while matches!(b.get(*at), Some(c) if *c != quote) {
                    *at += 1;
                }
                let raw = String::from_utf8_lossy(&b[v_start..*at]).into_owned();
                *at += 1;
                if key == "value" {
                    value = Some(unescape_xml(&raw));
                }
            }
            None => return Err("unterminated element".to_string()),
        }
    }

    // Children until the closing tag. Character data is not expected in the
    // oracle inputs this reader serves, and is rejected rather than ignored.
    let mut children = Vec::new();
    loop {
        skip_xml_trivia(b, at);
        match b.get(*at) {
            Some(b'<') if b[*at..].starts_with(b"</") => {
                let end = find_from(b, *at, b">").ok_or("unterminated closing tag")?;
                let closing = String::from_utf8_lossy(&b[*at + 2..end]).into_owned();
                if closing.trim() != name {
                    return Err(format!("closing tag {closing} does not match {name}"));
                }
                *at = end + 1;
                return Ok(XmlElement {
                    name,
                    value,
                    children,
                });
            }
            Some(b'<') if b[*at..].starts_with(b"<!--") => skip_xml_prolog(b, at)?,
            Some(b'<') => children.push(parse_xml_element(b, at)?),
            Some(c) => {
                return Err(format!(
                    "unexpected character data {:?} at offset {at}",
                    *c as char
                ));
            }
            None => return Err(format!("unterminated element {name}")),
        }
    }
}

fn unescape_xml(raw: &str) -> String {
    raw.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// One step of an element path: a name and a repetition index.
fn path_steps(path: &str) -> Vec<(String, usize)> {
    path.split('.')
        .map(|step| match step.split_once('[') {
            Some((name, rest)) => {
                let index = rest
                    .trim_end_matches(']')
                    .parse()
                    .unwrap_or_else(|_| panic!("{step:?} is not an indexed element name"));
                (name.to_string(), index)
            }
            None => (step.to_string(), 0),
        })
        .collect()
}

/// The harness's test-only node: navigation over an XML oracle input, with no
/// `frevo-model` type anywhere in it.
struct XmlHarnessNode;

impl XmlHarnessNode {
    fn resolve<'a>(&self, root: &'a XmlElement, path: &str) -> Result<&'a XmlElement, NavError> {
        let steps = path_steps(path);
        let (first, index) = &steps[0];
        if first != &root.name || *index != 0 {
            return Err(NavError::Failed(format!(
                "{path} is not rooted at <{}>",
                root.name
            )));
        }
        let mut cur = root;
        for (name, index) in &steps[1..] {
            cur = cur
                .children
                .iter()
                .filter(|c| &c.name == name)
                .nth(*index)
                .ok_or_else(|| NavError::Failed(format!("{path}: no {name}[{index}]")))?;
        }
        Ok(cur)
    }

    fn root(&self, doc: &str) -> Result<XmlElement, NavError> {
        parse_xml(doc).map_err(NavError::Failed)
    }
}

impl NavOracle for XmlHarnessNode {
    fn type_name_at(&self, doc: &str, path: &str) -> Result<String, NavError> {
        let root = self.root(doc)?;
        if path_steps(path).len() == 1 {
            self.resolve(&root, path).map(|e| e.name.clone())
        } else {
            Err(NavError::Unanswerable(format!(
                "a node built from an XML document has no StructureDefinitions \
                 behind it, so it cannot name the FHIR type of {path}"
            )))
        }
    }

    fn is_resource_at(&self, doc: &str, path: &str) -> Result<bool, NavError> {
        let root = self.root(doc)?;
        if path_steps(path).len() == 1 {
            self.resolve(&root, path).map(|_| true)
        } else {
            Err(NavError::Unanswerable(format!(
                "the test-only node cannot tell whether {path} is a resource"
            )))
        }
    }

    fn children(&self, doc: &str, path: &str, name: &str) -> Result<Vec<Child>, NavError> {
        Ok(self
            .all_children(doc, path)?
            .into_iter()
            .filter(|c| c.name == name)
            .collect())
    }

    fn all_children(&self, doc: &str, path: &str) -> Result<Vec<Child>, NavError> {
        let root = self.root(doc)?;
        let node = self.resolve(&root, path)?;
        Ok(node
            .children
            .iter()
            .map(|c| Child {
                name: c.name.clone(),
                // Model information the test-only node does not have. The
                // comparison in model.nav.012 uses `portable()`, which excludes
                // it, rather than letting a placeholder be compared.
                type_name: String::new(),
                primitive: c.value.clone(),
                is_resource: false,
            })
            .collect())
    }

    fn primitive_at(&self, doc: &str, path: &str) -> Result<Option<String>, NavError> {
        let root = self.root(doc)?;
        self.resolve(&root, path).map(|e| e.value.clone())
    }

    fn children_outliving_their_parent(
        &self,
        doc: &str,
        path: &str,
        name: &str,
    ) -> Result<Vec<Child>, NavError> {
        // The owned values outlive the tree they came from, which is dropped at
        // the end of all_children.
        self.children(doc, path, name)
    }
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// A `Patient` with two names, so document order is observable.
const PATIENT_TWO_NAMES: &str = concat!(
    r##"{"resourceType":"Patient","id":"p","active":true,"##,
    r##""name":[{"family":"Alpha","given":["Ann"]},{"family":"Bravo","given":["Bob"]}],"##,
    r##""gender":"male","birthDate":"1974-12-25"}"##
);

/// An `Observation` carrying `valueQuantity`.
const OBSERVATION_VALUE_QUANTITY: &str = concat!(
    r##"{"resourceType":"Observation","id":"o","status":"final","##,
    r##""code":{"text":"Glucose"},"##,
    r##""valueQuantity":{"value":6.3,"unit":"mmol/L"}}"##
);

/// A `Patient` with no `birthDate`.
const PATIENT_WITHOUT_BIRTHDATE: &str = r##"{"resourceType":"Patient","id":"p","gender":"male"}"##;

/// A `Patient` carrying `extension` and `modifierExtension`, which
/// `model.nav.006` requires to come back as ordinary named children.
const PATIENT_WITH_EXTENSIONS: &str = concat!(
    r##"{"resourceType":"Patient","id":"p","##,
    r##""extension":[{"url":"https://example.org/a","valueString":"A"}],"##,
    r##""modifierExtension":[{"url":"https://example.org/b","valueString":"B"}],"##,
    r##""active":true,"##,
    r##""name":[{"family":"Alpha"},{"family":"Bravo"}],"##,
    r##""gender":"male"}"##
);

/// A `Patient` with a contained resource and a name, for `model.nav.008`.
const PATIENT_WITH_CONTAINED: &str = concat!(
    r##"{"resourceType":"Patient","id":"p","##,
    r##""contained":[{"resourceType":"Organization","id":"org1","name":"Acme Healthcare"}],"##,
    r##""name":[{"family":"Chalmers"}]}"##
);

/// A `Patient` whose `birthDate` exists only as a `_birthDate` sibling, so it
/// has extensions and no value.
const PATIENT_EXTENSION_ONLY_BIRTHDATE: &str = concat!(
    r##"{"resourceType":"Patient","id":"p","##,
    r##""_birthDate":{"extension":[{"url":"https://example.org/birth-time","##,
    r##""valueString":"08:00"}]}}"##
);

/// The R4 declared choice list of `Observation.value[x]`, from the
/// StructureDefinition snapshot: `Quantity | CodeableConcept | string |
/// boolean | integer | Range | Ratio | SampledData | time | dateTime |
/// Period`.
const OBSERVATION_VALUE_CHOICES: &[&str] = &[
    "Quantity",
    "CodeableConcept",
    "string",
    "boolean",
    "integer",
    "Range",
    "Ratio",
    "SampledData",
    "time",
    "dateTime",
    "Period",
];

/// The same logical `Patient` as [`NAV_COMPARISON_JSON`], as an XML oracle
/// input. `model.nav.012` navigates both and requires equal results.
const NAV_COMPARISON_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Patient xmlns="http://hl7.org/fhir">
  <id value="p"/>
  <name>
    <family value="Chalmers"/>
    <given value="Peter"/>
    <given value="James"/>
  </name>
  <name>
    <family value="Windsor"/>
  </name>
  <gender value="male"/>
  <birthDate value="1974-12-25"/>
</Patient>"#;

const NAV_COMPARISON_JSON: &str = concat!(
    r##"{"resourceType":"Patient","id":"p","##,
    r##""name":[{"family":"Chalmers","given":["Peter","James"]},{"family":"Windsor"}],"##,
    r##""gender":"male","birthDate":"1974-12-25"}"##
);

// ---------------------------------------------------------------------------
// model.nav.001 .. model.nav.012
// ---------------------------------------------------------------------------

// covers: model.nav.001
#[test]
fn a_node_reports_its_fhir_type_name() {
    let doc = PATIENT_TWO_NAMES;
    let what = "a decoded Patient";
    assert_eq!(must_type_name(doc, "Patient", what), "Patient");
    assert_eq!(
        must_type_name(doc, "Patient.name[0]", what),
        "HumanName",
        "the name child reports its datatype, not its element name"
    );
    assert_eq!(
        must_type_name(doc, "Patient.name[0].family", what),
        "string",
        "and a primitive reports its primitive type"
    );
}

// covers: model.nav.002
#[test]
fn children_are_returned_by_element_name_in_document_order() {
    let children = must_children(
        PATIENT_TWO_NAMES,
        "Patient",
        "name",
        "the name children of a decoded Patient",
    );
    assert_eq!(children.len(), 2, "two name children");
    assert!(
        children
            .iter()
            .all(|c| c.name == "name" && c.type_name == "HumanName"),
        "each comes back under the element name asked for: {:?}",
        children
            .iter()
            .map(|c| (&c.name, &c.type_name))
            .collect::<Vec<_>>()
    );
    assert!(
        children.iter().all(|c| c.primitive.is_none()),
        "a HumanName is not a primitive, so neither child carries a value"
    );

    // Document order, observable through the content of each repetition.
    let families: Vec<Option<String>> = (0..2)
        .map(|i| {
            must_children(
                PATIENT_TWO_NAMES,
                &format!("Patient.name[{i}]"),
                "family",
                "the family of a name repetition",
            )
            .first()
            .and_then(|c| c.primitive.clone())
        })
        .collect();
    assert_eq!(
        families,
        vec![Some("Alpha".to_string()), Some("Bravo".to_string())],
        "the two name children are returned in document order, Alpha then Bravo"
    );
}

// covers: model.nav.003
#[test]
fn a_choice_element_is_reached_by_its_base_name() {
    let children = must_children(
        OBSERVATION_VALUE_QUANTITY,
        "Observation",
        "value",
        "the value children of an Observation carrying valueQuantity",
    );
    assert_eq!(children.len(), 1, "one child is returned");
    assert_eq!(
        children[0].type_name, "Quantity",
        "whose type name is Quantity"
    );
    assert_eq!(
        children[0].name, "value",
        "and which comes back under the base element name, not the typed \
         spelling"
    );
}

// covers: model.nav.004
#[test]
fn the_typed_property_name_yields_no_children() {
    let children = must_children(
        OBSERVATION_VALUE_QUANTITY,
        "Observation",
        "valueQuantity",
        "the valueQuantity children of that same Observation",
    );
    assert!(
        children.is_empty(),
        "no children are returned -- this is what makes oracle case \
         polymorphics/testPolymorphicsB an empty result at runtime, with the \
         static check being what rejects the expression: {children:?}"
    );
}

// covers: model.nav.005
#[test]
fn an_absent_element_yields_an_empty_child_list_not_an_error() {
    // Result, not Option: an absent element is Ok(empty), so a malformed
    // document stays distinguishable from an absent element rather than both
    // becoming an empty collection.
    match node_under_test().children(PATIENT_WITHOUT_BIRTHDATE, "Patient", "birthDate") {
        Ok(children) => assert!(
            children.is_empty(),
            "an empty list is returned, not an error: {children:?}"
        ),
        Err(NavError::NotImplemented) => panic!("{NOT_BOUND}"),
        Err(e) => panic!("an absent element must yield an empty child list, not an error: {e}"),
    }
}

// covers: model.nav.006
#[test]
fn all_children_can_be_enumerated_with_their_names() {
    let children = must_all_children(
        PATIENT_WITH_EXTENSIONS,
        "Patient",
        "every child of a decoded Patient",
    );
    let names: Vec<&str> = children.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "id",
            "extension",
            "modifierExtension",
            "active",
            "name",
            "name",
            "gender"
        ],
        "each child comes back paired with its element name, in document order, \
         with extension and modifierExtension as ordinary named children"
    );
    assert!(
        !names.contains(&"resourceType"),
        "resourceType is not an element and must not be enumerated as one: {names:?}"
    );
    assert_eq!(
        children
            .iter()
            .find(|c| c.name == "extension")
            .map(|c| c.type_name.as_str()),
        Some("Extension"),
        "and an extension child reports Extension as its type"
    );
}

// covers: model.nav.007
#[test]
fn returned_values_are_owned_and_outlive_the_node_they_came_from() {
    // The seam drops the node the children came from before returning them, so
    // a value that is still readable here is an owned value. A borrow with the
    // receiver's lifetime could not be returned at all -- which is the specific
    // thing ADR 0004 says haste-fhirpath's borrowed &'a dyn MetaValue made
    // impossible for a lazy model.
    let children = match node_under_test().children_outliving_their_parent(
        PATIENT_TWO_NAMES,
        "Patient.name[0]",
        "family",
    ) {
        Ok(c) => c,
        Err(e) => panic!("obtaining an owned child value must succeed, but: {e}"),
    };
    assert_eq!(children.len(), 1);
    assert_eq!(
        children[0].primitive.as_deref(),
        Some("Alpha"),
        "the value is still readable after the node it was obtained from was \
         dropped"
    );
    assert_eq!(children[0].type_name, "string");
}

// covers: model.nav.008
#[test]
fn a_node_reports_whether_it_is_a_resource() {
    let node = node_under_test();
    let answers: Vec<(&str, bool)> = ["Patient", "Patient.contained[0]", "Patient.name[0]"]
        .iter()
        .map(|path| {
            let answer = match node.is_resource_at(PATIENT_WITH_CONTAINED, path) {
                Ok(a) => a,
                Err(e) => panic!("{path} must be able to say whether it is a resource: {e}"),
            };
            (*path, answer)
        })
        .collect();
    assert_eq!(
        answers,
        [
            ("Patient", true),
            ("Patient.contained[0]", true),
            ("Patient.name[0]", false),
        ],
        "the Patient and its contained[0] report as resources, and its name \
         does not"
    );
}

// covers: model.nav.009
#[test]
fn a_primitive_node_reports_its_value_or_reports_having_none() {
    let node = node_under_test();

    let with_value = match node.primitive_at(PATIENT_TWO_NAMES, "Patient.birthDate") {
        Ok(v) => v,
        Err(e) => panic!("a birthDate with a value must report it: {e}"),
    };
    assert_eq!(
        with_value.as_deref(),
        Some("1974-12-25"),
        "the first returns 1974-12-25"
    );

    let extension_only =
        match node.primitive_at(PATIENT_EXTENSION_ONLY_BIRTHDATE, "Patient.birthDate") {
            Ok(v) => v,
            Err(e) => panic!(
                "an extension-only birthDate is an element with no value, not an \
                 error: {e}"
            ),
        };
    assert_eq!(
        extension_only, None,
        "and the second returns none -- which is exactly what hasValue() \
         reports"
    );
}

// covers: model.nav.010
#[test]
fn the_model_answers_type_hierarchy_questions() {
    let model = model_under_test();

    for (child, ancestor) in [("code", "string"), ("Age", "Quantity")] {
        match model.derives_from(child, ancestor) {
            Ok(true) => {}
            Ok(false) => panic!(
                "{child} must derive from {ancestor}: it is what makes \
                 Patient.gender.is(string) and ...value is Quantity true in \
                 oracle group testInheritance"
            ),
            Err(e) => panic!("the model must answer whether {child} derives from {ancestor}: {e}"),
        }
    }

    // And the immediate base type, which is how the transitive answer is built.
    for (type_name, base) in [("code", "string"), ("Age", "Quantity")] {
        match model.base_type(type_name) {
            Ok(Some(b)) => assert_eq!(b, base, "the base type of {type_name} is {base}"),
            Ok(None) => panic!("{type_name} must report a base type of {base}"),
            Err(e) => panic!("the model must report the base type of {type_name}: {e}"),
        }
    }

    // A type that does not derive from Quantity must say so, or the answers
    // above carry no information.
    match model.derives_from("Duration", "Period") {
        Ok(false) => {}
        Ok(true) => panic!("Duration does not derive from Period"),
        Err(e) => panic!("the model must answer this too: {e}"),
    }
}

// covers: model.nav.011
#[test]
fn the_model_answers_element_type_questions_for_the_static_check() {
    let model = model_under_test();
    let declared = match model.element_types("Observation", "value") {
        Ok(Some(t)) => t,
        Ok(None) => panic!("Observation.value[x] must report its declared types"),
        Err(e) => panic!("the model must report the declared types of Observation.value[x]: {e}"),
    };

    let mut got = declared.clone();
    got.sort();
    let mut expected: Vec<String> = OBSERVATION_VALUE_CHOICES
        .iter()
        .map(|s| s.to_string())
        .collect();
    expected.sort();
    assert_eq!(
        got, expected,
        "it returns the full declared choice list of Observation.value[x]: \
         {OBSERVATION_VALUE_CHOICES:?}, but got {declared:?}"
    );
    assert!(
        !declared.iter().any(|t| t == "Duration"),
        "and nothing else: Duration is not in the choice list, which is why \
         ...value is Duration is false in oracle group testInheritance"
    );
}

// covers: model.nav.012
#[test]
fn navigation_works_over_a_node_that_is_not_frevo_model() {
    let frevo = node_under_test();
    let harness = XmlHarnessNode;

    // The test-only node really does work, so a failure below is a failure on
    // the frevo-model side.
    let harness_root = harness
        .all_children(NAV_COMPARISON_XML, "Patient")
        .unwrap_or_else(|e| panic!("the test-only XML node must navigate: {e}"));
    assert_eq!(
        harness_root
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>(),
        ["id", "name", "name", "gender", "birthDate"],
        "the test-only node is a working node, not a stub"
    );

    for path in ["Patient", "Patient.name[0]", "Patient.name[1]"] {
        let a = match frevo.all_children(NAV_COMPARISON_JSON, path) {
            Ok(c) => c,
            Err(e) => panic!("the frevo-model node must navigate {path}: {e}"),
        };
        let b = harness
            .all_children(NAV_COMPARISON_XML, path)
            .unwrap_or_else(|e| panic!("the test-only node must navigate {path}: {e}"));
        assert_eq!(
            a.iter().map(Child::portable).collect::<Vec<_>>(),
            b.iter().map(Child::portable).collect::<Vec<_>>(),
            "{path}: the results must be equal -- the same element names, the \
             same primitive values, in the same document order"
        );
    }

    for path in [
        "Patient.gender",
        "Patient.birthDate",
        "Patient.name[0].given[1]",
        "Patient.name[1].family",
    ] {
        let a = match frevo.primitive_at(NAV_COMPARISON_JSON, path) {
            Ok(v) => v,
            Err(e) => panic!("the frevo-model node must read {path}: {e}"),
        };
        let b = harness
            .primitive_at(NAV_COMPARISON_XML, path)
            .unwrap_or_else(|e| panic!("the test-only node must read {path}: {e}"));
        assert_eq!(a, b, "{path}: both nodes must return the same value");
        assert!(
            a.is_some(),
            "{path}: and the value must actually be there, or the comparison is \
             vacuous"
        );
    }

    // And frevo-path contains no reference to any frevo-model type, which is
    // what makes a second node implementation possible at all. path.api.008
    // grades the manifest edge in its own crate; this is the model-side half of
    // the same claim.
    let path_crate = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frevo-path");
    let manifest = std::fs::read_to_string(path_crate.join("Cargo.toml"))
        .unwrap_or_else(|e| panic!("crates/frevo-path/Cargo.toml must be readable: {e}"));
    assert!(
        !manifest.contains("frevo-model"),
        "crates/frevo-path/Cargo.toml must not name frevo-model"
    );
    let offenders = rust_files_mentioning(&path_crate.join("src"), "frevo_model");
    assert!(
        offenders.is_empty(),
        "no file under crates/frevo-path/src may reference a frevo-model type, \
         but these do: {offenders:?}"
    );
}

/// Every `.rs` file under `dir`, recursively, whose text contains `needle`.
fn rust_files_mentioning(dir: &Path, needle: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            out.extend(rust_files_mentioning(&path, needle));
        } else if path.extension().is_some_and(|e| e == "rs")
            && std::fs::read_to_string(&path).is_ok_and(|t| t.contains(needle))
        {
            out.push(path);
        }
    }
    out
}
