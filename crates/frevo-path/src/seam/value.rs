//! `Value` — the owned result type the engine evaluates over.
//!
//! `design.md` section 4.1 of `add-fhirpath-and-codegen-r4`. Definitions only:
//! no engine behaviour lives here, and nothing in this module knows what a
//! `frevo-model` node is.

use std::sync::Arc;

use super::node::Node;

/// An arbitrary-precision decimal that keeps the lexical form it was written
/// with.
///
/// `model.json.009`, `model.canonical.003` and `path.literal.006` all turn on
/// the same property: `1.58700` must re-serialise as `1.58700` and report a
/// precision of 5. A binary float loses both. `design.md` section 6 permits an
/// arbitrary-precision crate from `deny.toml`'s allow-list, and names the
/// fallback this type takes: lexical form plus an integer mantissa and scale.
/// No crate on the allow-list provides this, so the fallback is what is built,
/// and the decision is recorded in `tasks.md` under task 16.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Decimal {
    lexical: String,
    mantissa: i128,
    scale: u32,
}

impl Decimal {
    /// Parse a FHIRPath/FHIR decimal from its lexical form, keeping that form
    /// verbatim. Returns `None` when `text` is not a decimal.
    pub fn parse(text: &str) -> Option<Self> {
        let bytes = text.as_bytes();
        let mut at = 0usize;
        let negative = match bytes.first() {
            Some(b'-') => {
                at += 1;
                true
            }
            Some(b'+') => {
                at += 1;
                false
            }
            _ => false,
        };
        let int_start = at;
        while matches!(bytes.get(at), Some(c) if c.is_ascii_digit()) {
            at += 1;
        }
        if at == int_start {
            return None;
        }
        let mut digits = String::from(&text[int_start..at]);
        let mut scale = 0u32;
        if bytes.get(at) == Some(&b'.') {
            at += 1;
            let frac_start = at;
            while matches!(bytes.get(at), Some(c) if c.is_ascii_digit()) {
                at += 1;
            }
            if at == frac_start {
                return None;
            }
            scale = u32::try_from(at - frac_start).ok()?;
            digits.push_str(&text[frac_start..at]);
        }
        if at != bytes.len() {
            return None;
        }
        let mut mantissa: i128 = digits.parse().ok()?;
        if negative {
            mantissa = -mantissa;
        }
        Some(Self {
            lexical: text.to_string(),
            mantissa,
            scale,
        })
    }

    /// The lexical form, byte for byte as it was written.
    pub fn lexical(&self) -> &str {
        &self.lexical
    }

    /// The integer mantissa: `1.58700` has a mantissa of `158700`.
    pub fn mantissa(&self) -> i128 {
        self.mantissa
    }

    /// The number of fractional digits: `1.58700` has a scale of 5.
    pub fn scale(&self) -> u32 {
        self.scale
    }

    /// The number of significant digits FHIRPath's `precision()` reports.
    pub fn precision(&self) -> u32 {
        self.scale
    }
}

/// How much of a date or dateTime was written. FHIRPath comparison and
/// `lowBoundary`/`highBoundary` are defined in terms of it, so it is carried
/// rather than normalised away.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TemporalPrecision {
    Year,
    Month,
    Day,
    Hour,
    Minute,
    Second,
    Millisecond,
}

/// A date at the precision it was written.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PartialDate {
    pub lexical: String,
    pub precision: TemporalPrecision,
}

/// A dateTime at the precision it was written, with the offset it carried.
/// `offset_minutes` is `None` for a local dateTime with no offset at all.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PartialDateTime {
    pub lexical: String,
    pub precision: TemporalPrecision,
    pub offset_minutes: Option<i32>,
}

/// A time at the precision it was written.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PartialTime {
    pub lexical: String,
    pub precision: TemporalPrecision,
}

/// A FHIRPath quantity: a decimal value and a unit, which is either a UCUM
/// code or one of the calendar-duration keywords.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Quantity {
    pub value: Decimal,
    pub unit: Option<String>,
    pub system: Option<String>,
    pub code: Option<String>,
}

/// A primitive element's value together with the `_field` sibling information
/// that travelled with it.
///
/// Carrying the sibling here is what `helios-fhirpath-support`'s
/// `PrimitiveElement` does, and ADR 0004 cites it approvingly for this reason:
/// a value that came from a primitive element keeps its `id` and `extension`
/// reachable.
#[derive(Debug, Clone)]
pub struct Primitive {
    /// The FHIR primitive type name: `code`, `date`, `boolean`.
    pub type_name: String,
    /// The lexical form, exactly as the document carried it.
    pub lexical: String,
    /// `Element.id` from the `_field` sibling, when there was one.
    pub id: Option<String>,
    /// `Element.extension` from the `_field` sibling, in document order.
    pub extensions: Vec<Value>,
}

/// An owned evaluation result.
///
/// Owned, as ADR 0004 decision 5 requires. `Arc<dyn Node>` is the one
/// indirection, and it is there so a lazy model can hand out a handle rather
/// than a materialised tree.
#[derive(Debug, Clone)]
pub enum Value {
    Boolean(bool),
    Integer(i64),
    Decimal(Decimal),
    String(String),
    Date(PartialDate),
    DateTime(PartialDateTime),
    Time(PartialTime),
    Quantity(Quantity),
    /// A FHIR element or resource, as a handle rather than a tree.
    Node(Arc<dyn Node>),
}

impl Value {
    /// The FHIRPath type name of this value: `System.Boolean` for a literal
    /// boolean, the node's FHIR type name for a node.
    pub fn type_name(&self) -> &str {
        match self {
            Value::Boolean(_) => "System.Boolean",
            Value::Integer(_) => "System.Integer",
            Value::Decimal(_) => "System.Decimal",
            Value::String(_) => "System.String",
            Value::Date(_) => "System.Date",
            Value::DateTime(_) => "System.DateTime",
            Value::Time(_) => "System.Time",
            Value::Quantity(_) => "System.Quantity",
            Value::Node(node) => node.type_name(),
        }
    }

    /// The node behind this value, when it is one.
    pub fn as_node(&self) -> Option<&Arc<dyn Node>> {
        match self {
            Value::Node(node) => Some(node),
            _ => None,
        }
    }
}
