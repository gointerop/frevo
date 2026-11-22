//! Literals: what each written form denotes.
//!
//! Derived from org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java
//! (`processConstant`, the quantity-literal arm of `parseExpression`,
//! `processDateConstant`) and org.hl7.fhir.core/org.hl7.fhir.utilities/src/main/java/org/hl7/fhir/utilities/Utilities.java
//! (`isInteger`, `checkDecimal`) at 73c432b5f43ddbce979978a760ec4aa81e929094.
//! Original work Copyright HL7, Inc. and contributors, licensed under the
//! Apache License, Version 2.0.
//! Modified by the Frevo authors: a decimal keeps its written form instead of
//! becoming a `BigDecimal` whose scale is the only trace of it
//! (`path.literal.006`), and the temporal forms are returned as the seam's
//! partial date, dateTime and time rather than as `java.util.Date`-backed
//! HAPI types.

use crate::seam::value::{PartialDate, PartialDateTime, PartialTime, TemporalPrecision, Value};

/// `Utilities.isInteger` (`Utilities.java:83`), bounds check included: a value
/// of more than ten digits, or a ten-digit value above `2147483647`, is not an
/// integer. The next arm of `processConstant` then reads it as a decimal,
/// which is why `2147483648` is a `System.Decimal` (`path.literal.015`).
pub(crate) fn is_integer(text: &str) -> bool {
    if text.trim().is_empty() {
        return false;
    }
    let negative = text.starts_with('-');
    let value = if negative { &text[1..] } else { text };
    if value.is_empty() {
        return false;
    }
    if !value.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    if value.len() > 10 {
        return false;
    }
    // Upstream compares the *signed* text against the bound, so for a negative
    // value the comparison can never fire — `'-'` sorts below `'2'`. Kept as
    // it is, because `processConstant` never sees a signed run anyway: the
    // lexer emits `-` as its own token.
    if !negative && value.len() == 10 && text > "2147483647" {
        return false;
    }
    true
}

/// `Utilities.checkDecimal`'s verdicts. `Range` is a value whose digit count
/// is beyond upstream's arbitrary bound, and `isDecimal` accepts it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DecimalStatus {
    Ok,
    Range,
    Syntax,
    Blank,
}

/// `Utilities.isDecimal(value, allowExponent)` (`Utilities.java:228`), which
/// `processConstant` calls with `allowExponent = false` and so with leading
/// zeros rejected: `01.5` is not a decimal and the literal is refused.
pub(crate) fn is_decimal(text: &str, allow_exponent: bool, allow_leading_zero: bool) -> bool {
    matches!(
        check_decimal(text, allow_exponent, allow_leading_zero),
        DecimalStatus::Ok | DecimalStatus::Range
    )
}

fn check_decimal(value: &str, allow_exponent: bool, allow_leading_zero: bool) -> DecimalStatus {
    if value.trim().is_empty() {
        return DecimalStatus::Blank;
    }
    if !allow_leading_zero {
        for prefix in ["0", "-0", "+0"] {
            if value.starts_with(prefix)
                && value != prefix
                && !value.starts_with(&format!("{prefix}."))
            {
                return DecimalStatus::Syntax;
            }
        }
    }
    if value.ends_with('.') {
        return DecimalStatus::Syntax;
    }

    let mut have_period = false;
    let mut have_exponent = false;
    let mut have_sign = false;
    let mut have_digits = false;
    let mut pre_dec_length = 0usize;
    let mut post_dec_length = 0usize;
    let mut exponent_length = 0usize;
    let mut length = 0usize;
    for next in value.chars() {
        if next == '.' {
            if !have_digits || have_period || have_exponent {
                return DecimalStatus::Syntax;
            }
            have_period = true;
            pre_dec_length = length;
            length = 0;
        } else if next == '-' || next == '+' {
            if have_digits || have_sign {
                return DecimalStatus::Syntax;
            }
            have_sign = true;
        } else if next == 'e' || next == 'E' {
            if !have_digits || have_exponent || !allow_exponent {
                return DecimalStatus::Syntax;
            }
            have_exponent = true;
            have_sign = false;
            have_digits = false;
            if have_period {
                post_dec_length = length;
            } else {
                pre_dec_length = length;
            }
            length = 0;
        } else if !next.is_ascii_digit() {
            return DecimalStatus::Syntax;
        } else {
            have_digits = true;
            length += 1;
        }
    }
    if have_exponent && !have_digits {
        return DecimalStatus::Syntax;
    }
    if have_exponent {
        exponent_length = length;
    } else if have_period {
        post_dec_length = length;
    } else {
        pre_dec_length = length;
    }

    if exponent_length > 4 {
        return DecimalStatus::Range;
    }
    if pre_dec_length + post_dec_length > 18 {
        return DecimalStatus::Range;
    }
    DecimalStatus::Ok
}

/// The sixteen calendar-duration keywords the quantity-literal arm of
/// `parseExpression` recognises (`FHIRPathEngine.java:1105`), singular and
/// plural.
pub(crate) const CALENDAR_UNITS: &[&str] = &[
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

/// The UCUM code a calendar-duration keyword maps to, where upstream maps one.
///
/// `year` and `month` map to **nothing**: upstream declines them with the
/// comment "this is not the UCUM year" (`FHIRPathEngine.java:1112-1115`), so
/// the literal carries neither a system nor a code and only the unit word
/// identifies it (`path.literal.013`).
pub(crate) fn calendar_unit_code(word: &str) -> Option<&'static str> {
    match word {
        "year" | "years" | "month" | "months" => None,
        "week" | "weeks" => Some("wk"),
        "day" | "days" => Some("d"),
        "hour" | "hours" => Some("h"),
        "minute" | "minutes" => Some("min"),
        "second" | "seconds" => Some("s"),
        _ => Some("ms"),
    }
}

/// `http://unitsofmeasure.org`, the system a quantity literal carries when it
/// has a UCUM code at all.
pub const UCUM_SYSTEM: &str = "http://unitsofmeasure.org";

/// `processDateConstant` (`FHIRPathEngine.java:1674`): what an `@...` literal
/// denotes, with the leading `@` already removed.
///
/// Upstream splits the run into date, time and offset, pads a short time,
/// then builds a `DateType`, `DateTimeType` or `TimeType` and calls
/// `setPrecision`, which **truncates** the padding back off
/// (`TimeType.java:138`, `BaseDateTimeType.setPrecision`). The padding is
/// therefore invisible in the result, and the forms below are what a caller
/// sees. Two consequences worth naming, both upstream's:
///
/// - `@2015T` is a **date**, not a dateTime: `"2015T".split("T")` drops the
///   trailing empty element, so the time is absent. `is(DateTime)` is still
///   true for it, because `funcIs` answers true for a date asked about
///   `System.DateTime` (`FHIRPathEngine.java:5128`);
/// - a time with an offset — `@T10:30:00Z` — is an **error**, which is why the
///   suite's two such cases are `invalid="execution"`.
pub fn process_date_constant(value: &str) -> Result<Value, String> {
    let (date, mut time) = if let Some(rest) = value.strip_prefix('T') {
        (None, Some(rest.to_string()))
    } else if !value.contains('T') {
        (Some(value.to_string()), None)
    } else {
        // `split("T")` in Java drops trailing empty elements, so a run ending
        // in `T` yields the date alone.
        let mut parts = value.splitn(2, 'T');
        let date = parts.next().unwrap_or("").to_string();
        let time = match parts.next() {
            None | Some("") => None,
            Some(t) => Some(t.to_string()),
        };
        (Some(date), time)
    };

    let mut tz: Option<String> = None;
    let mut precision = TemporalPrecision::Second;
    if let Some(t) = time.as_mut() {
        let cut = t.find('-').or_else(|| t.find('+')).or_else(|| t.find('Z'));
        if let Some(at) = cut {
            tz = Some(t[at..].to_string());
            t.truncate(at);
        }
        if t.len() == 2 {
            // Upstream pads to `hh:00:00` and then `setPrecision(MINUTE)`
            // truncates to five characters, so the value is `hh:00`.
            t.push_str(":00");
            precision = TemporalPrecision::Minute;
        } else if t.len() == 5 {
            precision = TemporalPrecision::Minute;
        } else if t.contains('.') {
            precision = TemporalPrecision::Millisecond;
        } else {
            precision = TemporalPrecision::Second;
        }
    }

    match (date, time) {
        (None, Some(time)) => {
            if tz.is_some() {
                return Err(format!(
                    "Error evaluating FHIRPath expression: the context of the \
                     expression cannot be determined: @T{time}{} is not a valid time",
                    tz.unwrap_or_default()
                ));
            }
            check_time(&time)?;
            Ok(Value::Time(PartialTime {
                lexical: time,
                precision,
            }))
        }
        (Some(date), Some(time)) => {
            let date_precision = check_date(&date)?;
            if date_precision != TemporalPrecision::Day {
                return Err(format!(
                    "@{date}T{time} is not a valid dateTime: a time needs a full date"
                ));
            }
            check_time(&time)?;
            let offset_minutes = match tz.as_deref() {
                None => None,
                Some(text) => Some(parse_offset(text)?),
            };
            Ok(Value::DateTime(PartialDateTime {
                lexical: format!("{date}T{time}{}", tz.unwrap_or_default()),
                precision,
                offset_minutes,
                occurrence: 0,
            }))
        }
        (Some(date), None) => {
            let precision = check_date(&date)?;
            Ok(Value::Date(PartialDate {
                lexical: date,
                precision,
            }))
        }
        (None, None) => Err("@ is not a valid date, dateTime or time".to_string()),
    }
}

/// The precision a date run was written at, or why it is not a date.
fn check_date(date: &str) -> Result<TemporalPrecision, String> {
    let parts: Vec<&str> = date.split('-').collect();
    let bad = || format!("@{date} is not a valid date");
    let numeric = |s: &str, width: usize| -> bool {
        s.len() == width && s.bytes().all(|b| b.is_ascii_digit())
    };
    if !numeric(parts[0], 4) {
        return Err(bad());
    }
    match parts.len() {
        1 => Ok(TemporalPrecision::Year),
        2 => {
            if !numeric(parts[1], 2) || !(1..=12).contains(&number(parts[1])) {
                return Err(bad());
            }
            Ok(TemporalPrecision::Month)
        }
        3 => {
            if !numeric(parts[1], 2) || !(1..=12).contains(&number(parts[1])) {
                return Err(bad());
            }
            if !numeric(parts[2], 2) || !(1..=31).contains(&number(parts[2])) {
                return Err(bad());
            }
            Ok(TemporalPrecision::Day)
        }
        _ => Err(bad()),
    }
}

/// A two- or four-digit run as a number, or 0 if it is not one. Every caller
/// has already checked that it is all digits.
fn number(text: &str) -> u32 {
    text.parse().unwrap_or(0)
}

/// `hh:mm`, `hh:mm:ss` or `hh:mm:ss.fff`, with the ranges FHIR allows.
fn check_time(time: &str) -> Result<(), String> {
    let bad = || format!("@T{time} is not a valid time");
    let (clock, fraction) = match time.split_once('.') {
        None => (time, None),
        Some((clock, fraction)) => (clock, Some(fraction)),
    };
    if let Some(fraction) = fraction {
        if fraction.is_empty() || !fraction.bytes().all(|b| b.is_ascii_digit()) {
            return Err(bad());
        }
    }
    let parts: Vec<&str> = clock.split(':').collect();
    if parts.len() < 2 || parts.len() > 3 {
        return Err(bad());
    }
    let bounds = [23u32, 59, 60];
    for (at, part) in parts.iter().enumerate() {
        if part.len() != 2 || !part.bytes().all(|b| b.is_ascii_digit()) {
            return Err(bad());
        }
        if number(part) > bounds[at] {
            return Err(bad());
        }
    }
    Ok(())
}

/// `Z`, `+hh:mm` or `-hh:mm`, as minutes east of UTC.
fn parse_offset(text: &str) -> Result<i32, String> {
    let bad = || format!("{text} is not a valid timezone offset");
    if text == "Z" {
        return Ok(0);
    }
    let sign = match text.as_bytes().first() {
        Some(b'+') => 1,
        Some(b'-') => -1,
        _ => return Err(bad()),
    };
    let (hours, minutes) = text[1..].split_once(':').ok_or_else(bad)?;
    if hours.len() != 2 || minutes.len() != 2 {
        return Err(bad());
    }
    let hours: i32 = hours.parse().map_err(|_| bad())?;
    let minutes: i32 = minutes.parse().map_err(|_| bad())?;
    if hours > 14 || minutes > 59 {
        return Err(bad());
    }
    Ok(sign * (hours * 60 + minutes))
}
