//! Navigation: one name step over one focus item.
//!
//! Derived from org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java
//! (`execute(ExecutionContext, Base, ExpressionNode, boolean)` at `:3182`,
//! `getChildrenByName` at `:255`, `filterIdType` at `:280`) and
//! org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/model/IdType.java
//! (`setValue` at `:411`, `toUnqualifiedVersionless` at `:671`,
//! `withResourceType` at `:687`),
//! org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/model/Narrative.java
//! (`listChildren` at `:338-344`, `getProperty` at `:364-366`) and
//! org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/model/XhtmlType.java
//! (`fhirType` and `listChildren` at `:55-61`), at 73c432b5f43ddbce979978a760ec4aa81e929094.
//! Original work Copyright HL7, Inc. and contributors, licensed under the
//! Apache License, Version 2.0; `Narrative.java` and `XhtmlType.java` carry
//! HL7's BSD-style licence, whose full text is reproduced in `xhtml.rs`.
//! Modified by the Frevo authors: navigation goes through the `Node` and
//! `Model` traits instead of `Base.listChildrenByName` and
//! `IWorkerContext.fetchTypeDefinition`, and the `IdType` value is rebuilt from
//! its lexical form instead of from a parsed `IdType` object; and `div`'s
//! string is composed from the stored narrative without editing it.

use std::sync::{Arc, OnceLock};

use crate::seam::model::Model;
use crate::seam::node::{Node, NodeError};
use crate::seam::value::{Primitive, Quantity, Value};

/// The FHIR type name `FHIRPathEngine` sees for a value: `Base.fhirType()`.
///
/// A literal in upstream is a FHIR primitive object (`StringType`,
/// `IntegerType`, ...), so its type name is the FHIR one, not the
/// `System.*` name [`Value::type_name`] reports.
pub(crate) fn fhir_type(value: &Value) -> &str {
    match value {
        Value::Boolean(_) => "boolean",
        Value::Integer(_) => "integer",
        Value::Decimal(_) => "decimal",
        Value::String(_) => "string",
        Value::Date(_) => "date",
        Value::DateTime(_) => "dateTime",
        Value::Time(_) => "time",
        Value::Quantity(_) => "Quantity",
        Value::Node(node) => node.type_name(),
    }
}

/// The leading-type-name special case (`FHIRPathEngine.java:3192-3207`): at
/// the entry of an expression, a name starting with an upper-case letter is a
/// type test against the focus item and its base types, not a navigation.
///
/// A type the model does not know is upstream's "logical model" branch, which
/// compares the name with the item's type name and nothing else.
pub(crate) fn type_name_matches(model: &dyn Model, item: &Value, name: &str) -> bool {
    let type_name = fhir_type(item);
    if !model.is_type(type_name) {
        return name == type_name;
    }
    // `sd.getType().equals(name) || sd.getTypeTail().equals(name)`, then
    // `sd = fetch(sd.getBaseDefinition())` until there is no base.
    let mut current = Some(type_name);
    let mut guard = 0usize;
    while let Some(t) = current {
        if t == name || type_tail(t) == name {
            return true;
        }
        guard += 1;
        if guard > 64 {
            // A cyclic hierarchy is a model defect; refusing to spin is safe.
            return false;
        }
        current = model.base_type(t);
    }
    false
}

/// `StructureDefinition.getTypeTail()`: the type after its last `/`.
fn type_tail(type_name: &str) -> &str {
    type_name.rsplit('/').next().unwrap_or(type_name)
}

/// The name `getChildrenByName` uses for "every child" (`Base.java:227`).
pub(crate) const ALL_CHILDREN: &str = "*";

/// `getChildrenByName(item, name, result)`, appending to `out` in document
/// order.
///
/// - a value that is not a node — a literal — has no children: upstream's
///   primitive objects carry no `id` or `extension` unless one was set, and a
///   literal never has one set;
/// - a primitive node whose model returns nothing for `extension` or `id`
///   falls back to the `_field` sibling information [`Primitive`] carries,
///   which is where a model that keeps the sibling out of its child list puts
///   it (`design.md` section 4.1, `path.navigation.014`);
/// - every child is passed through [`filter_id_type`].
///
/// Polymorphic typed names (`valueQuantity`) are not rewritten here:
/// `isAllowPolymorphicNames()` is `false` unless a caller sets it, and the
/// suite's harness never does, so `Observation.valueQuantity` navigates
/// nothing at runtime and the check pass is what rejects it
/// (`path.navigation.007`). With it set, the name step goes through
/// [`polymorphic_children_by_name`] instead.
pub(crate) fn children_by_name(
    item: &Value,
    name: &str,
    out: &mut Vec<Value>,
) -> Result<(), NodeError> {
    let node = match item {
        Value::Node(node) => node,
        Value::Quantity(q) => {
            quantity_element(q, name, out);
            return Ok(());
        }
        _ => return Ok(()),
    };
    let parent_is_resource = node.is_resource();
    let before = out.len();
    if name == ALL_CHILDREN {
        let mut has_div = false;
        for (child_name, child) in node.all_children()? {
            let is_resource_id = parent_is_resource && child_name == "id";
            let child = filter_id_type(super::rewrite::apply(child), is_resource_id);
            has_div |= is_xhtml(&child).is_some();
            let child = xhtml_item(child);
            super::classes::note(&child, node.type_name(), &child_name, true);
            out.push(child);
        }
        // `Narrative.listChildren` adds `new XhtmlType(this)` whether or not
        // the narrative holds a `div` (`Narrative.java:343`), last.
        if !has_div && node.type_name() == NARRATIVE {
            out.push(absent_xhtml_item());
        }
    } else {
        let is_resource_id = parent_is_resource && name == "id";
        for child in node.children(name)? {
            let child = filter_id_type(super::rewrite::apply(child), is_resource_id);
            if let Some(child) = xhtml_by_name(child)? {
                super::classes::note(&child, node.type_name(), name, false);
                out.push(child);
            }
        }
    }
    if out.len() == before && matches!(name, "id" | "extension" | ALL_CHILDREN) {
        primitive_sibling(node.as_ref(), name, out);
    }
    Ok(())
}

/// `getChildrenByName` with `isAllowPolymorphicNames()` on
/// (`FHIRPathEngine.java:255-279`): the first choice element of the item's
/// type, in definition order, whose name `name` **starts with** is navigated
/// instead, keeping the children whose type name equals the rest of `name`
/// ignoring case. A name no choice element prefixes navigates as usual.
///
/// Upstream's test is `name.startsWith(n)`, so the choice element's own name
/// matches too, with an empty rest that no type equals: with the setting on,
/// `Observation.value` navigates nothing. That is upstream's behaviour, kept.
///
/// `Ok(None)` is "this model cannot list the type's elements", which the
/// caller must report as not implemented rather than navigate around: a
/// polymorphic name is only recognisable from the definition.
pub(crate) fn polymorphic_children_by_name(
    model: &dyn Model,
    item: &Value,
    name: &str,
    out: &mut Vec<Value>,
) -> Result<Option<()>, NodeError> {
    let node = match item {
        Value::Node(node) => node,
        // A quantity the engine built has no choice element.
        Value::Quantity(_) => {
            children_by_name(item, name, out)?;
            return Ok(Some(()));
        }
        _ => return Ok(Some(())),
    };
    let Some(elements) = model.elements(node.type_name()) else {
        return Ok(None);
    };
    let choice = elements
        .iter()
        .find(|e| e.choice && name.starts_with(e.name.as_str()));
    match choice {
        None => children_by_name(item, name, out)?,
        Some(e) => {
            let tn = &name[e.name.len()..];
            let mut found = Vec::new();
            children_by_name(item, &e.name, &mut found)?;
            out.extend(
                found
                    .into_iter()
                    .filter(|v| fhir_type(v).eq_ignore_ascii_case(tn)),
            );
        }
    }
    Ok(Some(()))
}

/// A member of a quantity the engine built (`path.quantity.025`): the literal
/// is the FHIR `Quantity` of `FHIRPathEngine.java:1132`, and a `*` or `/`
/// result the one `pairToQty` builds (`:2314-2316`), so `value`, `unit`,
/// `system` and `code` navigate as on a `Quantity` read from a resource, typed
/// `decimal`, `string`, `uri` and `code`, and an element the engine did not set
/// is empty. Every other name, `comparator` included, is never set on such a
/// quantity and is empty too.
///
/// Only a named step reaches here: `*` (`children()`, `descendants()`) is not
/// a member navigation and keeps answering nothing for a value that is not a
/// node.
fn quantity_element(q: &Quantity, name: &str, out: &mut Vec<Value>) {
    let (type_name, lexical) = match name {
        "value" => ("decimal", Some(q.value.lexical())),
        "unit" => ("string", q.unit.as_deref()),
        "system" => ("uri", q.system.as_deref()),
        "code" => ("code", q.code.as_deref()),
        _ => return,
    };
    if let Some(lexical) = lexical {
        out.push(Value::Node(Arc::new(QuantityElement {
            type_name,
            lexical: lexical.to_owned(),
        })));
    }
}

/// One element of a quantity the engine built: a primitive with no `id`, no
/// extensions and no children of its own.
#[derive(Debug)]
struct QuantityElement {
    type_name: &'static str,
    lexical: String,
}

impl Node for QuantityElement {
    fn type_name(&self) -> &str {
        self.type_name
    }

    fn is_resource(&self) -> bool {
        false
    }

    fn children(&self, _name: &str) -> Result<Vec<Value>, NodeError> {
        Ok(Vec::new())
    }

    fn all_children(&self) -> Result<Vec<(String, Value)>, NodeError> {
        Ok(Vec::new())
    }

    fn primitive(&self) -> Option<Primitive> {
        Some(Primitive {
            type_name: self.type_name.to_string(),
            lexical: self.lexical.clone(),
            id: None,
            extensions: Vec::new(),
        })
    }
}

/// The `_field` sibling's `id` and `extension`, for a primitive node whose
/// model did not list them as children. `Element.listChildren` puts `id`
/// before `extension`, which is the order kept here for `*`.
fn primitive_sibling(node: &dyn Node, name: &str, out: &mut Vec<Value>) {
    let Some(primitive) = node.primitive() else {
        return;
    };
    if matches!(name, "id" | ALL_CHILDREN) {
        if let Some(id) = primitive.id {
            // `Element.id` is a `string` in R4.
            out.push(Value::String(id));
        }
    }
    if matches!(name, "extension" | ALL_CHILDREN) {
        out.extend(primitive.extensions);
    }
}

/// `filterIdType` (`FHIRPathEngine.java:280`): a child that is an `IdType`
/// is replaced by `toUnqualifiedVersionless().withResourceType(null)`.
///
/// In the R4 object model two things are an `IdType`: every element whose
/// FHIR type is `id`, and `Resource.id` — whose StructureDefinition declares
/// `System.String` with a `structuredefinition-fhir-type` of `string`, but
/// whose Java field is an `IdType` all the same. A model that reports
/// `Resource.id` as `string` is therefore still filtered, which is what
/// `is_resource_id` is for; the result's type is `id` either way, because the
/// replacement is a new `IdType`.
///
/// The replacement is a **new** object: it carries the logical id and
/// nothing else, so an `id` or `extension` the original element had does not
/// survive the step. That is upstream's behaviour, kept.
fn filter_id_type(child: Value, is_resource_id: bool) -> Value {
    let Value::Node(node) = &child else {
        return child;
    };
    if !(is_resource_id || node.type_name() == "id") {
        return child;
    }
    let logical = node.primitive().and_then(|p| logical_id(&p.lexical));
    Value::Node(Arc::new(IdNode { value: logical }))
}

/// Whether `value` is an element of FHIR type `xhtml`. In R4 that is only
/// ever `Narrative.div`, the one element the object model holds as an
/// `XhtmlNode`.
fn is_xhtml(value: &Value) -> Option<&Arc<dyn Node>> {
    match value {
        Value::Node(node) if node.type_name() == XHTML => Some(node),
        _ => None,
    }
}

const XHTML: &str = "xhtml";
const NARRATIVE: &str = "Narrative";

/// The string of the `xhtml` item for a narrative with no `div`: HAPI's read
/// creates an empty `div` (`getDiv`, `Narrative.java:316-323`); Frevo creates
/// nothing and gives the XML-mode composition of an empty `div` (`DEV-0028`,
/// `DEBT-0018`).
const EMPTY_DIV: &str = "<div/>";

/// `div` by name (`path.navigation.025`): `Narrative.getProperty` answers it
/// with `new StringType(new XhtmlComposer(true).composeEx(div))`
/// (`Narrative.java:364-366`), a **`string`** element holding the parsed node
/// recomposed in XML mode. The stored narrative is read, never edited.
///
/// A narrative the composer cannot produce -- one HAPI's parser would have
/// rejected when the resource was read, or one with no node type -- fails the
/// navigation, as a decode error does, rather than yielding something HAPI
/// never holds.
///
/// An `xhtml` node with no value is a `div` the object model does not hold,
/// which `getProperty` answers with nothing.
fn xhtml_by_name(child: Value) -> Result<Option<Value>, NodeError> {
    let Some(node) = is_xhtml(&child) else {
        return Ok(Some(child));
    };
    let Some(stored) = node.primitive().map(|p| p.lexical) else {
        return Ok(None);
    };
    super::xhtml::compose_narrative(&stored)
        .map(|value| Some(Value::Node(Arc::new(DivString { value }))))
        .map_err(|e| NodeError::new("Narrative.div", e.message()))
}

/// The `StringType` `getProperty` builds for `div`. It is an element of the
/// object model, not a value the engine made, so it is not
/// `disallowExtensions`: `is string` follows the type hierarchy for it as for
/// any `string` element.
#[derive(Debug)]
struct DivString {
    value: String,
}

impl Node for DivString {
    fn type_name(&self) -> &str {
        "string"
    }

    fn is_resource(&self) -> bool {
        false
    }

    fn children(&self, _name: &str) -> Result<Vec<Value>, NodeError> {
        Ok(Vec::new())
    }

    fn all_children(&self) -> Result<Vec<(String, Value)>, NodeError> {
        Ok(Vec::new())
    }

    fn primitive(&self) -> Option<Primitive> {
        Some(Primitive {
            type_name: "string".to_string(),
            lexical: self.value.clone(),
            id: None,
            extensions: Vec::new(),
        })
    }
}

/// `div` through `*` (`children()`, `descendants()`): `listChildren` holds
/// `new XhtmlType(this)` (`Narrative.java:343`), an item of type `xhtml`
/// with no children of its own (`XhtmlType.java:60-61`). Its string is
/// composed when it is read, and is the same XML-mode composition the name
/// step gives: HAPI's is the HTML-mode one, which edits the narrative first
/// (`DEV-0028`, owed as `DEBT-0018`).
/// A `div` the object model does not hold, by an absent element or an
/// `xhtml` node with no value, yields an item whose string is [`EMPTY_DIV`].
fn xhtml_item(child: Value) -> Value {
    match is_xhtml(&child) {
        Some(node) => Value::Node(Arc::new(XhtmlItem {
            stored: node.primitive().map(|p| p.lexical),
            composed: OnceLock::new(),
        })),
        None => child,
    }
}

/// The `xhtml` item for a narrative that holds no `div`.
fn absent_xhtml_item() -> Value {
    Value::Node(Arc::new(XhtmlItem {
        stored: None,
        composed: OnceLock::new(),
    }))
}

/// The `XhtmlType` `*` navigation yields for a narrative.
#[derive(Debug)]
struct XhtmlItem {
    stored: Option<String>,
    composed: OnceLock<Option<String>>,
}

impl Node for XhtmlItem {
    fn type_name(&self) -> &str {
        XHTML
    }

    fn is_resource(&self) -> bool {
        false
    }

    fn children(&self, _name: &str) -> Result<Vec<Value>, NodeError> {
        Ok(Vec::new())
    }

    fn all_children(&self) -> Result<Vec<(String, Value)>, NodeError> {
        Ok(Vec::new())
    }

    /// The composition, read lazily so that navigating past the item costs
    /// nothing. A narrative the composer cannot produce reports its stored
    /// text: a string cannot carry the error, and HAPI never holds such a
    /// narrative to compare against.
    ///
    /// With no stored `div` the string is [`EMPTY_DIV`] and nothing is
    /// created, so `text.div` stays empty.
    fn primitive(&self) -> Option<Primitive> {
        let lexical = match self.stored.as_ref() {
            None => EMPTY_DIV.to_string(),
            Some(stored) => self
                .composed
                .get_or_init(|| super::xhtml::compose_narrative(stored).ok())
                .clone()
                .unwrap_or_else(|| stored.clone()),
        };
        Some(Primitive {
            type_name: XHTML.to_string(),
            lexical,
            id: None,
            extensions: Vec::new(),
        })
    }
}

const URN_PREFIX: &str = "urn:";

/// `Utilities.isBlank` / `StringUtils.isBlank`: empty or whitespace only.
fn is_blank(s: &str) -> bool {
    s.chars().all(char::is_whitespace)
}

/// The value of `new IdType(v).toUnqualifiedVersionless().withResourceType(null)`.
///
/// `IdType.setValue` splits `v` into base URL, resource type, id part and
/// version; the two calls keep the id part alone, except that a local
/// reference (`#...`) or a URN is kept whole. `None` is an `IdType` with no
/// value, which is what a blank id part produces.
///
/// `http://base/Patient/123/_history/222` yields `123`
/// (`FHIRPathTests.testEvaluate_Id`, `path.navigation.013`).
pub(crate) fn logical_id(v: &str) -> Option<String> {
    if is_blank(v) {
        return None;
    }
    // `setValue`'s two early branches keep the value whole.
    if (v.starts_with('#') && v.len() > 1) || v.starts_with(URN_PREFIX) {
        return Some(v.to_string());
    }
    let (id_part, id_index): (&str, Option<usize>) = match v.find("/_history/") {
        Some(vid) => {
            // `lastIndexOf('/', vidIndex - 1)`.
            let idx = v[..vid].rfind('/');
            let start = idx.map_or(0, |i| i + 1);
            (&v[start..vid], idx)
        }
        None => {
            let idx = v.rfind('/');
            let start = idx.map_or(0, |i| i + 1);
            (&v[start..], idx)
        }
    };
    let mut id_part = Some(id_part);
    // The branch that decides a trailing segment is a resource type and not
    // an id: `http://example.org/foo` has no id at all.
    if let Some(id_index) = id_index.filter(|i| *i > 0) {
        if let Some(mut type_index) = v[..id_index].rfind('/') {
            let bytes = v.as_bytes();
            let mut moved_past_end = false;
            if type_index > 0 && bytes[type_index - 1] == b'/' {
                // `indexOf('/', typeIndex + 1)`, which can be -1.
                match v[type_index + 1..].find('/') {
                    Some(next) => type_index = type_index + 1 + next,
                    None => moved_past_end = true,
                }
            }
            if !moved_past_end && type_index >= id_index {
                id_part = None;
            }
        }
    }
    let id_part = id_part?;
    // `toUnqualifiedVersionless` and `withResourceType` both return the value
    // whole for a local reference or a URN, decided on the parsed id part.
    if id_part.starts_with('#') || id_part.starts_with(URN_PREFIX) {
        return Some(v.to_string());
    }
    if is_blank(id_part) {
        return None;
    }
    Some(id_part.to_string())
}

/// The `IdType` `filterIdType` produces: an `id` primitive with no `id`, no
/// extensions and no children of its own.
#[derive(Debug)]
struct IdNode {
    value: Option<String>,
}

impl Node for IdNode {
    fn type_name(&self) -> &str {
        "id"
    }

    fn is_resource(&self) -> bool {
        false
    }

    fn children(&self, _name: &str) -> Result<Vec<Value>, NodeError> {
        Ok(Vec::new())
    }

    fn all_children(&self) -> Result<Vec<(String, Value)>, NodeError> {
        Ok(Vec::new())
    }

    fn primitive(&self) -> Option<Primitive> {
        self.value.as_ref().map(|lexical| Primitive {
            type_name: "id".to_string(),
            lexical: lexical.clone(),
            id: None,
            extensions: Vec::new(),
        })
    }
}
