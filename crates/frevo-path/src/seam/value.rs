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
        // `BigDecimal(String)` also reads an exponent: `1.23E+4`.
        let mut exponent: i64 = 0;
        if matches!(bytes.get(at), Some(b'e' | b'E')) {
            let exp_text = &text[at + 1..];
            let digits_only = exp_text.trim_start_matches(['-', '+']);
            if digits_only.is_empty()
                || exp_text.len() - digits_only.len() > 1
                || !digits_only.bytes().all(|c| c.is_ascii_digit())
            {
                return None;
            }
            exponent = exp_text.trim_start_matches('+').parse().ok()?;
            at = bytes.len();
        }
        if at != bytes.len() {
            return None;
        }
        let mut mantissa: i128 = digits.parse().ok()?;
        if negative {
            mantissa = -mantissa;
        }
        // The value `mantissa * 10^-(scale - exponent)`, held at a
        // non-negative scale.
        let net = i64::from(scale) - exponent;
        if net < 0 {
            let lift = u32::try_from(-net).ok()?;
            mantissa = mantissa.checked_mul(10i128.checked_pow(lift)?)?;
            scale = 0;
        } else {
            scale = u32::try_from(net).ok()?;
        }
        Some(Self {
            lexical: text.to_string(),
            mantissa,
            scale,
        })
    }

    /// The decimal `unscaled * 10^-scale`, written as Java's
    /// `BigDecimal.toString` writes it (`DecimalType.encode`,
    /// `DecimalType.java:101-102`): plain digits when `scale >= 0` and the
    /// adjusted exponent is at least -6, otherwise scientific with a signed
    /// exponent. A negative `scale` is carried in the lexical form only; the
    /// numeric fields hold the value at scale 0. `None` when the value does not
    /// fit.
    pub fn from_unscaled(unscaled: i128, scale: i64) -> Option<Self> {
        let digits = unscaled.unsigned_abs().to_string();
        let len = i64::try_from(digits.len()).ok()?;
        let adjusted = len - 1 - scale;
        let mut text = String::new();
        if unscaled < 0 {
            text.push('-');
        }
        if scale == 0 {
            text.push_str(&digits);
        } else if scale > 0 && adjusted >= -6 {
            let scale = usize::try_from(scale).ok()?;
            let padded = format!("{digits:0>width$}", width = scale + 1);
            let split = padded.len() - scale;
            text.push_str(&padded[..split]);
            text.push('.');
            text.push_str(&padded[split..]);
        } else {
            text.push_str(&digits[..1]);
            if digits.len() > 1 {
                text.push('.');
                text.push_str(&digits[1..]);
            }
            if adjusted != 0 {
                text.push('E');
                if adjusted > 0 {
                    text.push('+');
                }
                text.push_str(&adjusted.to_string());
            }
        }
        let (mantissa, scale) = if scale < 0 {
            let lift = u32::try_from(-scale).ok()?;
            (unscaled.checked_mul(10i128.checked_pow(lift)?)?, 0)
        } else {
            (unscaled, u32::try_from(scale).ok()?)
        };
        Some(Self {
            lexical: text,
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
///
/// Equality, hashing and `Debug` read the three public fields only.
#[derive(Clone)]
pub struct PartialDateTime {
    pub lexical: String,
    pub precision: TemporalPrecision,
    pub offset_minutes: Option<i32>,
    /// Which evaluated `@` literal this value is, or 0 for any other value.
    ///
    /// Upstream resolves a `@` literal to a new `DateTimeType` object every
    /// time its node is evaluated (`resolveConstant`, `processDateConstant`,
    /// `FHIRPathEngine.java:1653-1736`), and an ordered comparison rewrites
    /// that object to Zulu in place (`compareDateTimeElements`, `:635-648`),
    /// so every holder of that object -- `$this`, the items `where` returns --
    /// sees the rewrite and a second occurrence of the same text does not
    /// (`path.compare.015`). A value is cloned freely here, so the object's
    /// identity is carried as this number, which every clone shares.
    pub(crate) occurrence: u64,
}

impl PartialDateTime {
    /// A dateTime value that is not an evaluated `@` literal.
    pub fn new(lexical: String, precision: TemporalPrecision, offset_minutes: Option<i32>) -> Self {
        Self {
            lexical,
            precision,
            offset_minutes,
            occurrence: 0,
        }
    }
}

impl PartialEq for PartialDateTime {
    fn eq(&self, other: &Self) -> bool {
        self.lexical == other.lexical
            && self.precision == other.precision
            && self.offset_minutes == other.offset_minutes
    }
}

impl Eq for PartialDateTime {}

impl std::hash::Hash for PartialDateTime {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.lexical.hash(state);
        self.precision.hash(state);
        self.offset_minutes.hash(state);
    }
}

impl std::fmt::Debug for PartialDateTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PartialDateTime")
            .field("lexical", &self.lexical)
            .field("precision", &self.precision)
            .field("offset_minutes", &self.offset_minutes)
            .finish()
    }
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
