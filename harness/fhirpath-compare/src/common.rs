//! What the helios and haste bindings share: the suite inputs as FHIR JSON,
//! and the outcome rows each binary writes.
//!
//! Both reference engines read FHIR JSON only, so the suite's XML inputs are
//! converted to JSON here, once, from the same parsed document the oracle
//! harness hands Frevo (`tests/oracle/input.rs`), with the element shapes taken
//! from the pinned R4 type package the harness loads (`r4_model()`): JSON
//! property names (a choice element's `value` + `Quantity`), arrays for a
//! repeating element, `true`/`false` and bare numbers for `boolean` and the
//! numeric primitives, the `_name` sibling for a primitive's `id` and
//! `extension`, and a narrative `div` as its markup. Decimals keep the
//! lexical form the XML gives them (`9.5e2`, `1.0`). JSON inputs are passed
//! through byte for byte. Both engines therefore read the identical text.

use std::fmt::Write as _;

use crate::oracle::engine::r4_model;
use crate::oracle::input::{Document, SuiteInput};
use crate::oracle::value::{QuantityValue, ResultItem, parse_quantity_string};
use crate::oracle::xml::Element;

use frevo_path::Model;

/// The input as FHIR JSON text, or why it cannot be converted.
pub fn input_json(input: &SuiteInput) -> Result<String, String> {
    match &input.document {
        Document::Json(_) => {
            String::from_utf8(input.bytes.clone()).map_err(|e| format!("{}: {e}", input.file))
        }
        Document::Xml(doc) => {
            let model = r4_model().ok_or("the pinned R4 type package is absent")?;
            let mut out = String::new();
            resource(model, &doc.root, &mut out)?;
            Ok(out)
        }
    }
}

fn json_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn resource(model: &dyn Model, el: &Element, out: &mut String) -> Result<(), String> {
    out.push_str("{\"resourceType\":");
    json_string(&el.name, out);
    members(model, el, &el.name, true, out, true)?;
    out.push('}');
    Ok(())
}

/// The JSON members of `el` read as type `ty`, each preceded by a comma when
/// `comma_first` (the object already has a member).
fn members(
    model: &dyn Model,
    el: &Element,
    ty: &str,
    at_resource_root: bool,
    out: &mut String,
    comma_first: bool,
) -> Result<(), String> {
    let elements = model
        .elements(ty)
        .ok_or_else(|| format!("the R4 package has no elements for {ty}"))?;
    let mut first = !comma_first;
    let mut sep = |out: &mut String| {
        if !first {
            out.push(',');
        }
        first = false;
    };
    for e in &elements {
        // `Element.id` (not `Resource.id`) and `Extension.url` are attributes.
        if (e.name == "id" && !at_resource_root) || (e.name == "url" && ty == "Extension") {
            if el.has_attr(&e.name) {
                sep(out);
                json_string(&e.name, out);
                out.push(':');
                json_string(el.attr(&e.name), out);
            }
            continue;
        }
        // The children this element takes, in document order, with the type
        // each one is (a choice's type is its name's suffix).
        let mut taken: Vec<(&Element, String, String)> = Vec::new();
        for child in el.child_elements() {
            if e.choice {
                let Some(suffix) = child.name.strip_prefix(e.name.as_str()) else {
                    continue;
                };
                if let Some(t) = e.types.iter().find(|t| upper_first(t) == suffix) {
                    taken.push((child, t.clone(), child.name.clone()));
                }
            } else if child.name == e.name {
                let t = e.types.first().cloned().unwrap_or_default();
                taken.push((child, t, e.name.clone()));
            }
        }
        if taken.is_empty() {
            continue;
        }
        // A choice takes one type per instance; the key is that type's.
        let key = taken[0].2.clone();
        let t = taken[0].1.clone();
        let items: Vec<&Element> = taken.iter().map(|(c, _, _)| *c).collect();
        let primitive = model.is_primitive(&t) && t != "xhtml";
        if primitive {
            let values: Vec<Option<String>> = items
                .iter()
                .map(|c| c.has_attr("value").then(|| c.attr("value").to_string()))
                .collect();
            let has_sibling = items
                .iter()
                .any(|c| c.has_attr("id") || !c.children_named("extension").is_empty());
            if values.iter().any(Option::is_some) {
                sep(out);
                json_string(&key, out);
                out.push(':');
                list(e.repeating, &values, out, |v, out| match v {
                    Some(v) => primitive_json(&t, v, out),
                    None => out.push_str("null"),
                });
            }
            if has_sibling {
                sep(out);
                json_string(&format!("_{key}"), out);
                out.push(':');
                let mut err = Ok(());
                list(e.repeating, &items, out, |c, out| {
                    if !c.has_attr("id") && c.children_named("extension").is_empty() {
                        out.push_str("null");
                        return;
                    }
                    out.push('{');
                    if let Err(x) = members(model, c, "Element", false, out, false) {
                        err = Err(x);
                    }
                    out.push('}');
                });
                err?;
            }
            continue;
        }
        sep(out);
        json_string(&key, out);
        out.push(':');
        let mut err = Ok(());
        list(e.repeating, &items, out, |c, out| {
            let r = if t == "xhtml" {
                json_string(&c.markup(), out);
                Ok(())
            } else if t == "Resource" {
                match c.child_elements().first() {
                    Some(r) => resource(model, r, out),
                    None => {
                        out.push_str("null");
                        Ok(())
                    }
                }
            } else {
                out.push('{');
                let r = members(model, c, &t, false, out, false);
                out.push('}');
                r
            };
            if let Err(x) = r {
                err = Err(x);
            }
        });
        err?;
    }
    Ok(())
}

fn list<T>(repeating: bool, items: &[T], out: &mut String, mut f: impl FnMut(&T, &mut String)) {
    if repeating {
        out.push('[');
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            f(item, out);
        }
        out.push(']');
    } else if let Some(item) = items.first() {
        f(item, out);
    }
}

fn primitive_json(t: &str, v: &str, out: &mut String) {
    match t {
        "boolean" if v == "true" || v == "false" => out.push_str(v),
        "integer" | "unsignedInt" | "positiveInt" | "decimal" | "integer64"
            if is_json_number(v) =>
        {
            out.push_str(v)
        }
        _ => json_string(v, out),
    }
}

/// A JSON number: `-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?`.
fn is_json_number(v: &str) -> bool {
    let re = regex::Regex::new(r"^-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?$");
    re.is_ok_and(|re| re.is_match(v))
}

fn upper_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// A quantity result as the harness compares it: the suite's own quantity
/// string syntax, parsed as the expectations are (`value::parse_quantity_string`),
/// so a calendar word stays a unit and anything else becomes a UCUM code.
pub fn quantity_item(value: &str, unit: &str) -> ResultItem {
    let calendar = [
        "year", "years", "month", "months", "week", "weeks", "day", "days", "hour", "hours",
        "minute", "minutes", "second", "seconds", "millisecond", "milliseconds",
    ];
    let text = if calendar.contains(&unit) {
        format!("{value} {unit}")
    } else {
        format!("{value} '{unit}'")
    };
    ResultItem::Quantity(
        parse_quantity_string(&text).unwrap_or_else(|| QuantityValue::from_ucum(value, unit)),
    )
}

/// The FHIR type name the suite writes for a result's type: a `System` type
/// with its first letter lower-cased (`System.DateTime` is `dateTime`) except
/// `Quantity`; a FHIR type as named.
pub fn suite_type_name(namespace: &str, name: &str) -> String {
    if namespace.eq_ignore_ascii_case("System") && name != "Quantity" {
        let mut c = name.chars();
        match c.next() {
            Some(f) => f.to_lowercase().collect::<String>() + c.as_str(),
            None => String::new(),
        }
    } else {
        name.to_string()
    }
}

/// One engine's outcome per case, as `haste-results.json` records it.
pub fn outcome_json(address: &str, status: &str, reason: &str, out: &mut String) {
    out.push_str("{\"address\":");
    json_string(address, out);
    out.push_str(",\"status\":");
    json_string(status, out);
    out.push_str(",\"reason\":");
    json_string(reason, out);
    out.push('}');
}

pub fn write_json_string(s: &str, out: &mut String) {
    json_string(s, out);
}
