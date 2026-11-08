//! The result values the harness compares, and the two renderers it compares
//! them through.
//!
//! The two renderers are **not** the same function, and unifying them would be
//! a latent whole-suite failure (`path.oracle.007`, `path.oracle.008`):
//!
//! | Branch | Java | Renderer |
//! |---|---|---|
//! | ordered, every case without `ordered="false"` | `((PrimitiveType) item).fpValue()` | [`fhirpath_string`] — `@` for `date`/`dateTime`/`instant`, `@T` for `time` |
//! | unordered, the one `ordered="false"` case | `((PrimitiveType) item).asStringValue()`, or `fp.convertToString(item)` for a `Quantity` | [`plain_string`] — no prefix at all |
//!
//! The prefixes come from `BaseDateTimeType.fpValue()` (`"@" + primitiveValue()`;
//! `DateType`, `DateTimeType` and `InstantType` extend it) and
//! `TimeType.fpValue()` (`"@T" + primitiveValue()`), pinned at
//! `73c432b5f43ddbce979978a760ec4aa81e929094`. They are why the suite's
//! expectations read `<output type="date">@1974-12-25</output>`
//! (`tests-fhir-r4.xml:59`). The prefix is a property of the comparison, not of
//! the value: an `@` is never stripped from an expectation and never added to
//! what the engine returns to a caller.

/// A quantity as the harness compares it: the fields
/// `org.hl7.fhir.r4.model.Quantity.equalsDeep` compares, plus whether the
/// element carries an `id` or extensions, because `equalsDeep` calls
/// `super.equalsDeep` first and `Element.equalsDeep` compares both.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuantityValue {
    /// The decimal in its presented lexical form; never a binary float.
    pub value: String,
    pub comparator: Option<String>,
    pub unit: Option<String>,
    pub system: Option<String>,
    pub code: Option<String>,
    pub has_element_id: bool,
    pub has_extensions: bool,
}

impl QuantityValue {
    /// A UCUM quantity, as `Quantity.fromUcum(v, code)` builds one.
    pub fn from_ucum(value: &str, code: &str) -> Self {
        QuantityValue {
            value: value.to_string(),
            comparator: None,
            unit: None,
            system: Some(UCUM_SYSTEM.to_string()),
            code: Some(code.to_string()),
            has_element_id: false,
            has_extensions: false,
        }
    }

    /// `Quantity.equalsDeep`: value, comparator, unit, system and code, with
    /// the decimal compared as `BigDecimal.equals` compares it — by unscaled
    /// value **and** scale, so `1.0` and `1.00` are not equal. That is what
    /// makes `LowBoundary/LowBoundaryQuantity`'s expectation of
    /// `1.58650000 'cm'` a real assertion about retained precision.
    pub fn equals_deep(&self, other: &QuantityValue) -> bool {
        if self.has_element_id
            || self.has_extensions
            || other.has_element_id
            || other.has_extensions
        {
            return false;
        }
        decimal_equals(&self.value, &other.value)
            && self.comparator == other.comparator
            && self.unit == other.unit
            && self.system == other.system
            && self.code == other.code
    }
}

pub const UCUM_SYSTEM: &str = "http://unitsofmeasure.org";

/// One item of an evaluation result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResultItem {
    /// A FHIR primitive: its type name and its primitive value.
    Primitive {
        fhir_type: String,
        value: String,
    },
    Quantity(QuantityValue),
    /// Anything else: a complex element or a resource. The pinned harness
    /// casts to `PrimitiveType` on the value branch, so a complex item there
    /// fails the case rather than comparing as something else.
    Complex {
        fhir_type: String,
    },
}

impl ResultItem {
    pub fn fhir_type(&self) -> &str {
        match self {
            ResultItem::Primitive { fhir_type, .. } => fhir_type,
            ResultItem::Quantity(_) => "Quantity",
            ResultItem::Complex { fhir_type } => fhir_type,
        }
    }
}

/// The ordered branch's renderer: `PrimitiveType.fpValue()`.
///
/// `None` means the item has no FHIRPath string value, which on the ordered
/// branch is a case failure, not a match.
pub fn fhirpath_string(item: &ResultItem) -> Option<String> {
    match item {
        ResultItem::Primitive { fhir_type, value } => Some(match fhir_type.as_str() {
            "date" | "dateTime" | "instant" => format!("@{value}"),
            "time" => format!("@T{value}"),
            _ => value.clone(),
        }),
        ResultItem::Quantity(_) | ResultItem::Complex { .. } => None,
    }
}

/// The unordered branch's renderer: `asStringValue()` for a primitive, and
/// `FHIRPathEngine.convertToString` for a `Quantity`. No `@` prefix on either.
pub fn plain_string(item: &ResultItem) -> Option<String> {
    match item {
        ResultItem::Primitive { value, .. } => Some(value.clone()),
        ResultItem::Quantity(q) => quantity_to_string(q),
        ResultItem::Complex { .. } => None,
    }
}

/// The calendar-duration unit words `FHIRPathEngine.convertToString`
/// special-cases.
const CALENDAR_UNITS: &[&str] = &[
    "year",
    "years",
    "month",
    "months",
    "week",
    "weeks",
    "day",
    "days",
    "hour",
    "hours",
    "minute",
    "minutes",
    "second",
    "seconds",
    "millisecond",
    "milliseconds",
];

/// `FHIRPathEngine.convertToString(Base)`'s `Quantity` arm.
///
/// Its third branch falls through to `Quantity.toString()`, which is a Java
/// object rendering rather than a specified form, so the harness returns `None`
/// there instead of inventing one. The pinned file's single `ordered="false"`
/// case yields strings, so that branch is unreachable today; returning `None`
/// makes it a visible failure the first time it is not.
pub fn quantity_to_string(q: &QuantityValue) -> Option<String> {
    let system_is_ucum = q.system.as_deref() == Some(UCUM_SYSTEM);
    if let Some(unit) = q.unit.as_deref()
        && CALENDAR_UNITS.contains(&unit)
        && (q.system.is_none() || system_is_ucum)
    {
        return Some(format!("{} {}", q.value, unit));
    }
    if system_is_ucum {
        return Some(format!("{} '{}'", q.value, q.code.as_deref().unwrap_or("")));
    }
    None
}

/// `FHIRPathEngine.parseQuantityString`, which is how the ordered branch turns
/// an expected `<output>` body into a quantity to compare deeply
/// (`path.oracle.009`).
pub fn parse_quantity_string(s: &str) -> Option<QuantityValue> {
    let s = s.trim();
    if let Some(space) = s.find(' ') {
        let v = s[..space].trim();
        let rest = s[space..].trim();
        if !is_decimal(v, false) {
            return None;
        }
        if rest.len() >= 2 && rest.starts_with('\'') && rest.ends_with('\'') {
            return Some(QuantityValue::from_ucum(v, &rest[1..rest.len() - 1]));
        }
        let code = match rest {
            "year" | "years" => "a",
            "month" | "months" => "mo_s",
            "week" | "weeks" => "wk",
            "day" | "days" => "d",
            "hour" | "hours" => "h",
            "minute" | "minutes" => "min",
            "second" | "seconds" => "s",
            "millisecond" | "milliseconds" => "ms",
            _ => return None,
        };
        Some(QuantityValue::from_ucum(v, code))
    } else if is_decimal(s, true) {
        Some(QuantityValue {
            value: s.to_string(),
            comparator: None,
            unit: None,
            system: Some(UCUM_SYSTEM.to_string()),
            code: Some("1".to_string()),
            has_element_id: false,
            has_extensions: false,
        })
    } else {
        None
    }
}

/// `Utilities.isDecimal(value, allowExponent)` with `allowLeadingZero` false,
/// which is the form `parseQuantityString` calls.
pub fn is_decimal(value: &str, allow_exponent: bool) -> bool {
    if value.trim().is_empty() || value.ends_with('.') {
        return false;
    }
    // `checkDecimal`'s leading-zero rule, with `allowLeadingZero` false.
    for prefix in ["0", "-0", "+0"] {
        if value.starts_with(prefix) && value != prefix && !value[prefix.len()..].starts_with('.') {
            return false;
        }
    }

    let mut rest = value;
    if let Some(stripped) = rest.strip_prefix(['-', '+']) {
        rest = stripped;
    }
    let (mantissa, exponent) = match rest.find(['e', 'E']) {
        Some(at) if allow_exponent => (&rest[..at], Some(&rest[at + 1..])),
        Some(_) => return false,
        None => (rest, None),
    };
    let (int_part, frac_part) = match mantissa.find('.') {
        Some(at) => (&mantissa[..at], Some(&mantissa[at + 1..])),
        None => (mantissa, None),
    };
    if int_part.is_empty() || !int_part.bytes().all(|c| c.is_ascii_digit()) {
        return false;
    }
    if let Some(f) = frac_part
        && (f.is_empty() || !f.bytes().all(|c| c.is_ascii_digit()))
    {
        return false;
    }
    if let Some(e) = exponent {
        let digits = e.strip_prefix(['-', '+']).unwrap_or(e);
        if digits.is_empty() || !digits.bytes().all(|c| c.is_ascii_digit()) {
            return false;
        }
    }
    true
}

/// `BigDecimal.equals`: equal unscaled value **and** equal scale.
///
/// `EqualsBuilder.append(Object, Object)` in `PrimitiveType.equalsDeep`
/// dispatches to `BigDecimal.equals`, which is scale-sensitive. Comparing by
/// numeric value instead would silently accept a result that lost precision,
/// which is the one thing the boundary cases exist to catch.
pub fn decimal_equals(a: &str, b: &str) -> bool {
    match (unscaled_and_scale(a), unscaled_and_scale(b)) {
        (Some(x), Some(y)) => x == y,
        // Not decimals at all: fall back to exact text, so a malformed value
        // can never compare equal to a different malformed value.
        _ => a == b,
    }
}

/// Decompose a decimal literal into (signed digit string without insignificant
/// leading zeros, scale), the pair `BigDecimal.equals` compares.
fn unscaled_and_scale(s: &str) -> Option<(String, i64)> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let (sign, rest) = match s.as_bytes()[0] {
        b'-' => (-1i8, &s[1..]),
        b'+' => (1, &s[1..]),
        _ => (1, s),
    };
    let (mantissa, exponent) = match rest.find(['e', 'E']) {
        Some(at) => (&rest[..at], rest[at + 1..].parse::<i64>().ok()?),
        None => (rest, 0i64),
    };
    let (int_part, frac_part) = match mantissa.find('.') {
        Some(at) => (&mantissa[..at], &mantissa[at + 1..]),
        None => (mantissa, ""),
    };
    if int_part.is_empty() && frac_part.is_empty() {
        return None;
    }
    if !int_part.bytes().all(|c| c.is_ascii_digit())
        || !frac_part.bytes().all(|c| c.is_ascii_digit())
    {
        return None;
    }
    let scale = frac_part.len() as i64 - exponent;
    let all_digits = format!("{int_part}{frac_part}");
    let trimmed = all_digits.trim_start_matches('0');
    let digits = if trimmed.is_empty() { "0" } else { trimmed };
    let signed = if sign < 0 && digits != "0" {
        format!("-{digits}")
    } else {
        digits.to_string()
    };
    Some((signed, scale))
}
