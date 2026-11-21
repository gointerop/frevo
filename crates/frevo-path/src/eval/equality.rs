//! The equality and emptiness primitives the collection functions share.
//!
//! Derived from org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java
//! (`doEquals` at `:2190`, `decEqual` at `:2180`, `removeTrailingZeros` at
//! `:2161`), org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/model/Base.java
//! (`compareDeep` at `:281`, `isEmpty` at `:238`), both at
//! 73c432b5f43ddbce979978a760ec4aa81e929094, and
//! hapi-fhir-base/src/main/java/ca/uhn/fhir/util/ElementUtil.java
//! (`isEmpty(List)` at `:92`) at 7ab4c7c7c49281496faa1b3c2498a652457669ae.
//! Original work Copyright HL7, Inc. and contributors, and Smile CDR, Inc.,
//! licensed under the Apache License, Version 2.0.
//! Modified by the Frevo authors: values are the `Value` enum rather than
//! `Base` objects, so "is this a `DecimalType`" is a question about the value's
//! FHIR type name; `equalsDeep` is one structural comparison over `Node`
//! instead of one generated method per class; the date rules live in
//! [`super::temporal`]; and the quantity rules reach the UCUM service through
//! [`super::quantity`].
//!
//! `funcHasValue` is at `:5776`, `funcComparable` at `:4609`. Task 35 added equivalence (`~`, `!~`) and the date and quantity rules of
//! `doEquals`.
//!
//! `doEquals` answers `Boolean` upstream, where `null` means "cannot be
//! compared"; that is the `None` here.

use crate::seam::model::Model;
use crate::seam::value::{Decimal, Value};

use super::navigate::fhir_type;
use super::operand::{self, STRING_TYPES, has_type, is_primitive, is_quantity, text_of};
use super::temporal;
use super::{Context, EvalError, NodeId, Run};

impl Run<'_> {
    /// `funcHasValue` (`FHIRPathEngine.java:5776`): a single item whose string
    /// value (`convertToString`) is not empty. A complex element has the text
    /// of its type name upstream, so it has a value; a primitive with only an
    /// `id` or extensions has none.
    pub(super) fn func_has_value(&self, focus: &[Value]) -> Vec<Value> {
        let [item] = focus else {
            return vec![Value::Boolean(false)];
        };
        let has = match primitive_value_of(self.model, item) {
            Some(text) => text.is_some_and(|t| !t.is_empty()),
            None => !text_for_message(item).is_empty(),
        };
        vec![Value::Boolean(has)]
    }

    /// `funcComparable` (`FHIRPathEngine.java:4609-4641`): whether two
    /// quantities could be compared.
    ///
    /// Every branch that needs no unit algebra is HAPI's: a focus that is not
    /// exactly one item whose FHIR type is `Quantity` -- an empty focus, two or
    /// more items, or one item of another type -- is `false` before the
    /// argument is evaluated (`:4610-4612`); an argument that is not exactly
    /// one `Quantity` is `false` (`:4613-4616`); a missing or
    /// differing `system` is `false` (`:4622-4624`), a missing `code` on
    /// either side is `false` (`:4625-4627`), equal codes are `true`
    /// (`:4628-4630`), and two different codes outside UCUM are `false`
    /// (`:4637-4639`). None of those consults the worker's UCUM service, so
    /// they are the same answers under every worker (`path.quantity.019`).
    ///
    /// Two different UCUM codes ask the service's `isComparable`
    /// (`:4631-4636`), and a code it cannot canonicalise is `false`.
    pub(super) fn func_comparable(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        let offset = self.expr.node(id).start();
        let focus_quantity = match focus {
            [item] if fhir_type(item) == "Quantity" => item,
            _ => return Ok(vec![Value::Boolean(false)]),
        };
        let other = self.execute(context, focus, self.param(id, 0)?, true)?;
        let other_quantity = match other.as_slice() {
            [other] if fhir_type(other) == "Quantity" => other,
            _ => return Ok(vec![Value::Boolean(false)]),
        };
        let (l, r) = (
            operand::qty_of(focus_quantity)?,
            operand::qty_of(other_quantity)?,
        );
        let (Some(s1), Some(s2)) = (&l.system, &r.system) else {
            return Ok(vec![Value::Boolean(false)]);
        };
        if s1 != s2 {
            return Ok(vec![Value::Boolean(false)]);
        }
        let (Some(u1), Some(u2)) = (&l.code, &r.code) else {
            return Ok(vec![Value::Boolean(false)]);
        };
        if u1 == u2 {
            return Ok(vec![Value::Boolean(true)]);
        }
        if s1 == operand::UCUM_SYSTEM {
            // `isComparable`, whose `getCanonicalUnits` re-wraps every
            // exception as `UcumException`, which is caught (`:4632-4636`).
            let _ = offset;
            return match super::quantity::service()?.is_comparable(Some(u1), Some(u2)) {
                Ok(b) => Ok(vec![Value::Boolean(b)]),
                Err(super::UcumError::Error(m)) => Err(EvalError::plain(m)),
                Err(_) => Ok(vec![Value::Boolean(false)]),
            };
        }
        Ok(vec![Value::Boolean(false)])
    }
}

/// `StringUtils.isBlank`.
fn is_blank(s: &str) -> bool {
    s.chars().all(char::is_whitespace)
}

/// `Base.primitiveValue()` for a value that is a primitive:
/// `Some(Some(text))` for one with a value, `Some(None)` for a primitive
/// element that carries only an id or extensions, `None` for anything that is
/// not a primitive at all.
pub(super) fn primitive_value_of(model: &dyn Model, value: &Value) -> Option<Option<String>> {
    match value {
        Value::Boolean(b) => Some(Some(b.to_string())),
        Value::Integer(i) => Some(Some(i.to_string())),
        Value::Decimal(d) => Some(Some(d.lexical().to_string())),
        Value::String(s) => Some(Some(s.clone())),
        Value::Date(d) => Some(Some(d.lexical.clone())),
        Value::DateTime(d) => Some(Some(d.lexical.clone())),
        Value::Time(t) => Some(Some(t.lexical.clone())),
        Value::Quantity(_) => None,
        Value::Node(node) => match node.primitive() {
            Some(p) => Some(Some(p.lexical)),
            None if model.is_primitive(node.type_name()) => Some(None),
            None => None,
        },
    }
}

/// `left instanceof DecimalType`, which for an element-model node is its FHIR
/// type.
fn is_decimal(value: &Value) -> bool {
    matches!(value, Value::Decimal(_)) || fhir_type(value) == "decimal"
}

/// `Base.isDateTime()`: a `BaseDateTimeType`, which is `date`, `dateTime` and
/// `instant` and is not `time`.
fn is_date_time(value: &Value) -> bool {
    matches!(value, Value::Date(_) | Value::DateTime(_))
        || matches!(fhir_type(value), "date" | "dateTime" | "instant")
}

/// `removeTrailingZeros` (`FHIRPathEngine.java:2152`), as written: it strips
/// trailing `0`s from a number that has **no** decimal point too, so `100`
/// becomes `1`. That is the pinned engine's behaviour and it is kept.
fn remove_trailing_zeros(s: &str) -> String {
    if s.is_empty() {
        return String::new();
    }
    let chars: Vec<char> = s.chars().collect();
    let mut i = chars.len() - 1;
    let mut done = false;
    let mut dot = false;
    while i > 0 && !done {
        if chars[i] == '.' {
            i -= 1;
            dot = true;
        } else if !dot && chars[i] == '0' {
            i -= 1;
        } else {
            done = true;
        }
    }
    chars[..=i].iter().collect()
}

/// `decEqual` (`FHIRPathEngine.java:2178`).
fn dec_equal(left: &str, right: &str) -> bool {
    remove_trailing_zeros(left) == remove_trailing_zeros(right)
}

/// Both operands as the dates they are, when both are dates or dateTimes
/// (`left.isDateTime() && right.isDateTime()`).
fn both_dates(
    left: &Value,
    right: &Value,
) -> Result<Option<(temporal::Dt, temporal::Dt)>, EvalError> {
    if !(is_date_time(left) && is_date_time(right)) {
        return Ok(None);
    }
    match (temporal::from_value(left)?, temporal::from_value(right)?) {
        (Some(l), Some(r)) => {
            temporal::check_copyable(&l, &text_for_message(left))?;
            temporal::check_copyable(&r, &text_for_message(right))?;
            Ok(Some((l, r)))
        }
        _ => Ok(None),
    }
}

fn text_for_message(value: &Value) -> String {
    super::collection::convert_to_string(value)
}

fn decimal(text: &str) -> Result<Decimal, EvalError> {
    Decimal::parse(text)
        .ok_or_else(|| EvalError::plain(format!("\"{text}\" is not a decimal value")))
}

/// `qtyEqual` (`FHIRPathEngine.java:2253`) and `qtyEquivalent` (`:2324`), which
/// differ only in how the two decimals are compared. `None` is upstream's
/// `null`: the quantities cannot be compared.
///
/// Both quantities are first converted to canonical form (`qtyToCanonicalPair`),
/// which yields nothing outside the UCUM system or for a code the service
/// refuses; only when both convert are the canonical units and values
/// compared (`path.quantity.020`, `.022`).
fn qty_compare(
    model: &dyn Model,
    left: &Value,
    right: &Value,
    equivalent: bool,
    offset: usize,
) -> Result<Option<bool>, EvalError> {
    let (l, r) = (operand::qty_of(left)?, operand::qty_of(right)?);
    if l.value.is_none() && r.value.is_none() {
        return Ok(Some(true));
    }
    let (Some(lv), Some(rv)) = (&l.value, &r.value) else {
        return Ok(None);
    };
    // `worker.getUcumService() != null`, which holds in every Frevo
    // configuration (`DEV-0029`): both sides are canonicalised, left first,
    // before either is looked at, so an exception from either escapes.
    let dl = super::quantity::canonical_pair(&l)?;
    let dr = super::quantity::canonical_pair(&r)?;
    if let (Some((lv, lc)), Some((rv, rc))) = (dl, dr) {
        if lc != rc {
            return Ok(Some(false));
        }
        // `new DecimalType(asDecimal())` on each side, compared as strings
        // (`decEqual`) or by `Utilities.equivalentNumber`.
        return Ok(Some(if equivalent {
            equivalent_number(&lv, &rv)
        } else {
            dec_equal(&lv, &rv)
        }));
    }
    if l.code.is_some() || r.code.is_some() {
        if l.code.is_none() || r.code.is_none() || l.code != r.code {
            return Ok(None);
        }
    } else if l.unit.is_none() || r.unit.is_some() {
        // As written upstream: `!left.hasUnit() || right.hasUnit()`.
        if l.unit.is_none() || r.unit.is_none() || l.unit != r.unit {
            return Ok(None);
        }
    }
    let a = Value::Decimal(decimal(lv)?);
    let b = Value::Decimal(decimal(rv)?);
    if equivalent {
        do_equivalent(model, &a, &b, offset).map(Some)
    } else {
        do_equals(model, &a, &b, offset)
    }
}

/// `doEquals(left, right)` (`FHIRPathEngine.java:2190`).
///
/// `offset` is where a failure is attributed.
pub(super) fn do_equals(
    model: &dyn Model,
    left: &Value,
    right: &Value,
    offset: usize,
) -> Result<Option<bool>, EvalError> {
    if is_quantity(model, left) && is_quantity(model, right) {
        return qty_compare(model, left, right, false, offset);
    }
    if let Some((l, r)) = both_dates(left, right)? {
        return Ok(temporal::equals_using_fhir_rules(&l, &r));
    }
    let left_primitive = primitive_value_of(model, left);
    let right_primitive = primitive_value_of(model, right);
    if is_decimal(left) || is_decimal(right) {
        let text = |p: &Option<Option<String>>| p.clone().flatten().unwrap_or_default();
        return Ok(Some(dec_equal(
            &text(&left_primitive),
            &text(&right_primitive),
        )));
    }
    if let (Some(l), Some(r)) = (&left_primitive, &right_primitive) {
        return Ok(Some(l == r));
    }
    compare_deep(model, left, right, offset).map(Some)
}

/// `opEquals` (`FHIRPathEngine.java:2103`) and `opNotEquals` (`:2130`):
/// either side empty is empty, a size mismatch is a plain answer, and a pair
/// `doEquals` cannot compare makes the whole answer empty unless another pair
/// already decided it.
pub(super) fn op_equals(
    model: &dyn Model,
    left: &[Value],
    right: &[Value],
    negate: bool,
    offset: usize,
) -> Result<Vec<Value>, EvalError> {
    if left.is_empty() || right.is_empty() {
        return Ok(Vec::new());
    }
    if left.len() != right.len() {
        return Ok(vec![Value::Boolean(negate)]);
    }
    let mut res = true;
    let mut nil = false;
    for (l, r) in left.iter().zip(right) {
        match do_equals(model, l, r, offset)? {
            None => nil = true,
            Some(eq) if eq == negate => {
                res = false;
                break;
            }
            Some(_) => {}
        }
    }
    Ok(if !res {
        vec![Value::Boolean(false)]
    } else if nil {
        Vec::new()
    } else {
        vec![Value::Boolean(true)]
    })
}

/// `Utilities.equivalent`: both blank, or equal ignoring case.
fn equivalent_text(l: &str, r: &str) -> bool {
    match (l.is_empty(), r.is_empty()) {
        (true, true) => true,
        (true, false) | (false, true) => false,
        (false, false) => l.to_lowercase() == r.to_lowercase(),
    }
}

/// `Utilities.equivalentNumber`: the two numbers at the smaller scale, rounded
/// half up.
fn equivalent_number(l: &str, r: &str) -> bool {
    match (l.is_empty(), r.is_empty()) {
        (true, true) => return true,
        (true, false) | (false, true) => return false,
        _ => {}
    }
    let (Some(a), Some(b)) = (Decimal::parse(l), Decimal::parse(r)) else {
        return false;
    };
    let scale = a.scale().min(b.scale());
    match (round_half_up(&a, scale), round_half_up(&b, scale)) {
        (Some(x), Some(y)) => x == y,
        _ => false,
    }
}

/// A decimal's mantissa at a smaller or equal scale, rounded half up (away
/// from zero on a tie, as `RoundingMode.HALF_UP`).
fn round_half_up(d: &Decimal, scale: u32) -> Option<i128> {
    let drop = d.scale() - scale;
    if drop == 0 {
        return Some(d.mantissa());
    }
    let divisor = 10i128.checked_pow(drop)?;
    let q = d.mantissa() / divisor;
    let rem = d.mantissa() % divisor;
    let up = rem.abs().checked_mul(2)? >= divisor;
    Some(if up { q + d.mantissa().signum() } else { q })
}

/// `doEquivalent(left, right)` (`FHIRPathEngine.java:2204`).
///
/// A `null` from the quantity rules is unboxed to a `boolean` upstream and
/// raises a `NullPointerException`; it is an execution error here.
pub(super) fn do_equivalent(
    model: &dyn Model,
    left: &Value,
    right: &Value,
    offset: usize,
) -> Result<bool, EvalError> {
    // The JVM's helpful `NullPointerException` text for unboxing the `null`
    // a `Boolean`-returning method answered: the method is named with its
    // declaring class and parameter types, fully qualified.
    let unboxed = |answer: Option<bool>, method: &str| {
        answer.ok_or_else(|| {
            EvalError::plain(format!(
                "Cannot invoke \"java.lang.Boolean.booleanValue()\" because the return value \
                 of \"org.hl7.fhir.r4.fhirpath.FHIRPathEngine.{method}\" is null"
            ))
        })
    };
    const QTY_EQUIVALENT: &str =
        "qtyEquivalent(org.hl7.fhir.r4.model.Quantity, org.hl7.fhir.r4.model.Quantity)";
    const DO_EQUALS: &str = "doEquals(org.hl7.fhir.r4.model.Base, org.hl7.fhir.r4.model.Base)";
    if is_quantity(model, left) && is_quantity(model, right) {
        return unboxed(
            qty_compare(model, left, right, true, offset)?,
            QTY_EQUIVALENT,
        );
    }
    if has_type(left, &["integer"]) && has_type(right, &["integer"]) {
        return unboxed(do_equals(model, left, right, offset)?, DO_EQUALS);
    }
    if has_type(left, &["boolean"]) && has_type(right, &["boolean"]) {
        return unboxed(do_equals(model, left, right, offset)?, DO_EQUALS);
    }
    let numeric = ["integer", "decimal", "unsignedInt", "positiveInt"];
    if has_type(left, &numeric) && has_type(right, &numeric) {
        return Ok(equivalent_number(
            &text_of(model, left).unwrap_or_default(),
            &text_of(model, right).unwrap_or_default(),
        ));
    }
    let temporal_types = ["date", "dateTime", "time", "instant"];
    if has_type(left, &temporal_types) && has_type(right, &temporal_types) {
        return match both_dates(left, right)? {
            Some((l, r)) => Ok(temporal::equals_using_fhir_rules(&l, &r) == Some(true)),
            // `new DateTimeType(timeText)` cannot be parsed.
            None => Err(EvalError::plain(format!(
                "Invalid date/time format: \"{}\"",
                text_for_message(if has_type(left, &["time"]) {
                    left
                } else {
                    right
                })
            ))),
        };
    }
    if has_type(left, &STRING_TYPES) && has_type(right, &STRING_TYPES) {
        return Ok(equivalent_text(
            &text_for_message(left),
            &text_for_message(right),
        ));
    }
    let (lp, rp) = (is_primitive(model, left), is_primitive(model, right));
    if lp && rp {
        return Ok(equivalent_text(
            &text_of(model, left).unwrap_or_default(),
            &text_of(model, right).unwrap_or_default(),
        ));
    }
    if lp || rp {
        return Ok(false);
    }
    let (Value::Node(ln), Value::Node(rn)) = (left, right) else {
        return Ok(false);
    };
    if ln.type_name() != rn.type_name() {
        return Ok(false);
    }
    let lc = ln.all_children()?;
    let rc = rn.all_children()?;
    let mut names: Vec<&str> = Vec::new();
    for (name, _) in lc.iter().chain(rc.iter()) {
        if !names.contains(&name.as_str()) {
            names.push(name);
        }
    }
    for name in names {
        let l: Vec<&Value> = lc
            .iter()
            .filter(|(n, _)| n == name)
            .map(|(_, v)| v)
            .collect();
        let r: Vec<&Value> = rc
            .iter()
            .filter(|(n, _)| n == name)
            .map(|(_, v)| v)
            .collect();
        if l.len() != r.len() {
            return Ok(false);
        }
        for (a, b) in l.into_iter().zip(r) {
            if !do_equivalent(model, a, b, offset)? {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

/// `opEquivalent` (`FHIRPathEngine.java:2351`) and `opNotEquivalent`
/// (`:2372`): always a boolean, and each left item need only be equivalent to
/// **some** right item, so order does not matter.
pub(super) fn op_equivalent(
    model: &dyn Model,
    left: &[Value],
    right: &[Value],
    negate: bool,
    offset: usize,
) -> Result<Vec<Value>, EvalError> {
    if left.len() != right.len() {
        return Ok(vec![Value::Boolean(negate)]);
    }
    let mut res = true;
    for l in left {
        let mut found = false;
        for r in right {
            if do_equivalent(model, l, r, offset)? {
                found = true;
                break;
            }
        }
        if !found {
            res = false;
            break;
        }
    }
    Ok(vec![Value::Boolean(res != negate)])
}

/// `doContains(list, item)` (`FHIRPathEngine.java:2846`): an item is contained
/// when `doEquals` answers `true`; a `null` answer is not a match.
pub(super) fn do_contains(
    model: &dyn Model,
    list: &[Value],
    item: &Value,
    offset: usize,
) -> Result<bool, EvalError> {
    for test in list {
        if do_equals(model, test, item, offset)? == Some(true) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// `Base.primitiveValue()`: the text of a primitive that has a value, and
/// `None` for anything else.
pub(super) fn primitive_text(model: &dyn Model, value: &Value) -> Option<String> {
    primitive_value_of(model, value).flatten()
}

/// The Java class of the value a primitive holds, which is what
/// `PrimitiveType.equalsDeep` compares: an `integer` is never equal to the
/// `string` `"1"`, and a `code` is a `string` because `CodeType` extends
/// `StringType`.
fn value_class(value: &Value) -> u8 {
    match fhir_type(value) {
        "integer" | "unsignedInt" | "positiveInt" => 0,
        "decimal" => 1,
        "boolean" => 2,
        "date" | "dateTime" | "instant" => 3,
        "time" => 4,
        _ => 5,
    }
}

/// `Base.compareDeep(left, right, false)` for two values (`Base.java:281`),
/// which is `left.equalsDeep(right)`.
pub(super) fn compare_deep(
    model: &dyn Model,
    left: &Value,
    right: &Value,
    offset: usize,
) -> Result<bool, EvalError> {
    if let (Value::Quantity(l), Value::Quantity(r)) = (left, right) {
        return Ok(l == r);
    }
    if matches!(left, Value::Quantity(_)) || matches!(right, Value::Quantity(_)) {
        // A literal quantity against a quantity element: the field-by-field
        // comparison over the element's children and the literal's fields.
        if is_quantity(model, left) && is_quantity(model, right) {
            let (l, r) = (operand::qty_of(left)?, operand::qty_of(right)?);
            return Ok(l.value == r.value
                && l.unit == r.unit
                && l.code == r.code
                && l.system == r.system);
        }
        return Ok(false);
    }
    let left_primitive = primitive_value_of(model, left);
    let right_primitive = primitive_value_of(model, right);
    match (left_primitive, right_primitive) {
        (Some(l), Some(r)) => {
            if value_class(left) != value_class(right) || l != r {
                return Ok(false);
            }
            // `Element.equalsDeep`: the `id` and the extensions too.
            let (Value::Node(ln), Value::Node(rn)) = (left, right) else {
                return Ok(true);
            };
            let (lp, rp) = (ln.primitive(), rn.primitive());
            let l_id = lp.as_ref().and_then(|p| p.id.as_deref());
            let r_id = rp.as_ref().and_then(|p| p.id.as_deref());
            if l_id != r_id {
                return Ok(false);
            }
            let l_ext = lp.map(|p| p.extensions).unwrap_or_default();
            let r_ext = rp.map(|p| p.extensions).unwrap_or_default();
            lists_deep_equal(model, &l_ext, &r_ext, offset)
        }
        // A primitive is never deeply equal to a complex value.
        (Some(_), None) | (None, Some(_)) => Ok(false),
        (None, None) => {
            let (Value::Node(ln), Value::Node(rn)) = (left, right) else {
                return Ok(false);
            };
            // `other instanceof HumanName`: the same class.
            if ln.type_name() != rn.type_name() {
                return Ok(false);
            }
            let lc = ln.all_children()?;
            let rc = rn.all_children()?;
            let mut names: Vec<&str> = Vec::new();
            for (name, _) in lc.iter().chain(rc.iter()) {
                if !names.contains(&name.as_str()) {
                    names.push(name);
                }
            }
            for name in names {
                let l: Vec<Value> = lc
                    .iter()
                    .filter(|(n, _)| n == name)
                    .map(|(_, v)| v.clone())
                    .collect();
                let r: Vec<Value> = rc
                    .iter()
                    .filter(|(n, _)| n == name)
                    .map(|(_, v)| v.clone())
                    .collect();
                if !lists_deep_equal(model, &l, &r, offset)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
    }
}

/// `Base.compareDeep(List, List, true)` (`Base.java:263`): two empty lists are
/// equal, an empty and a non-empty one are not, and otherwise the lists match
/// item for item.
fn lists_deep_equal(
    model: &dyn Model,
    left: &[Value],
    right: &[Value],
    offset: usize,
) -> Result<bool, EvalError> {
    if left.len() != right.len() {
        return Ok(false);
    }
    for (l, r) in left.iter().zip(right) {
        if !compare_deep(model, l, r, offset)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// `Base.isEmpty()` (`Base.java:238` and its overrides): a primitive is empty
/// when it has no non-blank value, no `id` and no extensions; a complex value
/// is empty when every child is.
pub(super) fn is_empty(value: &Value) -> Result<bool, EvalError> {
    match value {
        Value::String(s) => Ok(is_blank(s)),
        Value::Boolean(_)
        | Value::Integer(_)
        | Value::Decimal(_)
        | Value::Date(_)
        | Value::DateTime(_)
        | Value::Time(_)
        | Value::Quantity(_) => Ok(false),
        Value::Node(node) => {
            if let Some(p) = node.primitive() {
                if !is_blank(&p.lexical) || p.id.is_some() || !p.extensions.is_empty() {
                    return Ok(false);
                }
            }
            for (_, child) in node.all_children()? {
                if !is_empty(&child)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
    }
}
