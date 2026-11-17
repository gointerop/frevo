//! Date, dateTime and time comparison.
//!
//! Derived from org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java
//! (`compareDateTimeElements` at `:635`, `compareTimeElements` at `:652`) and
//! org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/model/BaseDateTimeType.java
//! (`equalsUsingFhirPathRules` at `:916`, `compareTimes` at `:993`,
//! `getHighEdge`), org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/model/TimeType.java
//! (`getHour`, `getMinute`, `getSecond`, `getPrecision`), all at
//! 73c432b5f43ddbce979978a760ec4aa81e929094.
//! Original work Copyright HL7, Inc. and contributors, licensed under the
//! Apache License, Version 2.0.
//! Modified by the Frevo authors: a date or dateTime is parsed once from its
//! lexical form into [`Dt`] instead of being a `Calendar`-backed object, and
//! the conversion to UTC is integer arithmetic over a proleptic Gregorian day
//! count.
//!
//! Where the pinned engine reads the JVM's default time zone -- a dateTime with
//! **no** offset is a `Date` in that zone, so it matters when such a value is
//! compared with one that has an offset, in `couldBeTheSameTime` and in the
//! conversion to Zulu -- this port reads a missing offset as UTC. The answer
//! is the pinned engine's on a JVM whose default zone is UTC and differs from
//! it by at most the zone's offset otherwise; two values that both lack an
//! offset compare identically in any zone.

use std::cmp::Ordering;

use crate::seam::value::{TemporalPrecision, Value};

use super::EvalError;
use super::navigate::fhir_type;

/// A date or dateTime as `BaseDateTimeType` sees it.
#[derive(Debug, Clone)]
pub(super) struct Dt {
    year: i64,
    month: i64,
    day: i64,
    hour: i64,
    minute: i64,
    second: i64,
    milli: i64,
    pub(super) precision: TemporalPrecision,
    /// Minutes east of UTC; `None` for no time zone at all
    /// (`BaseDateTimeType.hasTimezone`).
    pub(super) offset: Option<i32>,
    /// Whether this is a `dateTime` or `instant`, as opposed to a `date`.
    /// `DateTimeType` rejects a minute-precision value on `copy()`.
    pub(super) is_date_time: bool,
}

fn digits(text: &str) -> Option<i64> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// Days since 1970-01-01 of a proleptic Gregorian date (Howard Hinnant's
/// `days_from_civil`).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The inverse of [`days_from_civil`].
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

impl Dt {
    /// Parse `YYYY`, `YYYY-MM`, `YYYY-MM-DD` or `YYYY-MM-DDThh:mm[:ss[.f+]]`
    /// followed by an optional `Z` or `+hh:mm`.
    pub(super) fn parse(text: &str, is_date_time: bool) -> Option<Dt> {
        let (date, time) = match text.split_once('T') {
            Some((d, t)) => (d, Some(t)),
            None => (text, None),
        };
        let mut parts = date.split('-');
        let year = digits(parts.next()?).filter(|_| date.len() >= 4)?;
        let month = parts.next();
        let day = parts.next();
        if parts.next().is_some() {
            return None;
        }
        let (month_value, mut precision) = match month {
            None => (1, TemporalPrecision::Year),
            Some(m) => (digits(m)?, TemporalPrecision::Month),
        };
        let day_value = match day {
            None => 1,
            Some(d) => {
                precision = TemporalPrecision::Day;
                digits(d)?
            }
        };
        let (mut hour, mut minute, mut second, mut milli) = (0, 0, 0, 0);
        let mut offset = None;
        if let Some(time) = time {
            if precision != TemporalPrecision::Day {
                return None;
            }
            let (clock, zone) = match time.find(['Z', '+', '-']) {
                Some(at) => (&time[..at], Some(&time[at..])),
                None => (time, None),
            };
            offset = match zone {
                None => None,
                Some("Z") => Some(0),
                Some(z) => {
                    let sign = if z.starts_with('-') { -1 } else { 1 };
                    let (h, m) = z[1..].split_once(':')?;
                    Some(sign * i32::try_from(digits(h)? * 60 + digits(m)?).ok()?)
                }
            };
            let (clock, fraction) = match clock.split_once('.') {
                Some((c, f)) => (c, Some(f)),
                None => (clock, None),
            };
            let mut fields = clock.split(':');
            hour = digits(fields.next()?)?;
            minute = digits(fields.next()?)?;
            precision = TemporalPrecision::Minute;
            if let Some(s) = fields.next() {
                second = digits(s)?;
                precision = TemporalPrecision::Second;
            }
            if fields.next().is_some() {
                return None;
            }
            if let Some(f) = fraction {
                digits(f)?;
                let padded = format!("{f:0<3}");
                milli = digits(&padded[..3])?;
                precision = TemporalPrecision::Millisecond;
            }
        }
        Some(Dt {
            year,
            month: month_value,
            day: day_value,
            hour,
            minute,
            second,
            milli,
            precision,
            offset,
            is_date_time,
        })
    }

    /// The fields in UTC (`setTimeZoneZulu(true)`), which `BaseDateTimeType`
    /// applies only to a value more precise than a day. A value with no offset
    /// is read as UTC.
    fn zulu(&self) -> Dt {
        if self.precision <= TemporalPrecision::Day {
            return self.clone();
        }
        let offset = i64::from(self.offset.unwrap_or(0));
        let days = days_from_civil(self.year, self.month, self.day);
        let minutes = days * 1440 + self.hour * 60 + self.minute - offset;
        let (y, m, d) = civil_from_days(minutes.div_euclid(1440));
        let of_day = minutes.rem_euclid(1440);
        Dt {
            year: y,
            month: m,
            day: d,
            hour: of_day / 60,
            minute: of_day % 60,
            ..self.clone()
        }
    }
}

/// The date or dateTime a value is, or `None` when it is neither
/// (`Base.isDateTime`, `dateTimeValue`).
pub(super) fn from_value(value: &Value, offset: usize) -> Result<Option<Dt>, EvalError> {
    let (text, is_date_time) = match value {
        Value::Date(d) => (d.lexical.clone(), false),
        Value::DateTime(d) => (d.lexical.clone(), true),
        Value::Node(node) => match fhir_type(value) {
            "date" => (
                node.primitive().map(|p| p.lexical).unwrap_or_default(),
                false,
            ),
            "dateTime" | "instant" => (
                node.primitive().map(|p| p.lexical).unwrap_or_default(),
                true,
            ),
            _ => return Ok(None),
        },
        _ => return Ok(None),
    };
    Dt::parse(&text, is_date_time).map(Some).ok_or_else(|| {
        EvalError::execution(format!("Invalid date/time format: \"{text}\""), offset)
    })
}

/// `BaseDateTimeType.compareTimes(left, right, null)` over fields already in
/// the zone the caller wants: `None` is "not comparable at these precisions".
fn compare_times(l: &Dt, r: &Dt) -> Option<Ordering> {
    use TemporalPrecision as P;
    let both = |p: P| l.precision == p && r.precision == p;
    let either = |p: P| l.precision == p || r.precision == p;
    match l.year.cmp(&r.year) {
        Ordering::Equal => {}
        o => return Some(o),
    }
    if both(P::Year) {
        return Some(Ordering::Equal);
    }
    if either(P::Year) {
        return None;
    }
    match l.month.cmp(&r.month) {
        Ordering::Equal => {}
        o => return Some(o),
    }
    if both(P::Month) {
        return Some(Ordering::Equal);
    }
    if either(P::Month) {
        return None;
    }
    match l.day.cmp(&r.day) {
        Ordering::Equal => {}
        o => return Some(o),
    }
    if both(P::Day) {
        return Some(Ordering::Equal);
    }
    if either(P::Day) {
        return None;
    }
    match l.hour.cmp(&r.hour) {
        Ordering::Equal => {}
        o => return Some(o),
    }
    match l.minute.cmp(&r.minute) {
        Ordering::Equal => {}
        o => return Some(o),
    }
    if both(P::Minute) {
        return Some(Ordering::Equal);
    }
    if either(P::Minute) {
        return None;
    }
    match l.second.cmp(&r.second) {
        Ordering::Equal => {}
        o => return Some(o),
    }
    if both(P::Second) {
        return Some(Ordering::Equal);
    }
    Some(l.milli.cmp(&r.milli))
}

const MS_PER_MINUTE: i64 = 60_000;
const MS_PER_DAY: i64 = 86_400_000;

impl Dt {
    /// The instant in milliseconds since the epoch, with no offset read as UTC.
    fn instant_ms(&self) -> i64 {
        let days = days_from_civil(self.year, self.month, self.day);
        ((days * 24 + self.hour) * 60 + self.minute - i64::from(self.offset.unwrap_or(0)))
            * MS_PER_MINUTE
            + self.second * 1000
            + self.milli
    }

    /// `getHighEdge()`: the instant one unit of this value's precision later.
    fn high_edge_ms(&self) -> i64 {
        use TemporalPrecision as P;
        let low = self.instant_ms();
        match self.precision {
            P::Year => Dt {
                year: self.year + 1,
                ..self.clone()
            }
            .instant_ms(),
            P::Month => {
                let (year, month) = if self.month == 12 {
                    (self.year + 1, 1)
                } else {
                    (self.year, self.month + 1)
                };
                Dt {
                    year,
                    month,
                    ..self.clone()
                }
                .instant_ms()
            }
            P::Day => low + MS_PER_DAY,
            P::Hour => low + 60 * MS_PER_MINUTE,
            P::Minute => low + MS_PER_MINUTE,
            P::Second => low + 1000,
            P::Millisecond => low,
        }
    }

    /// One side of `couldBeTheSameTime`: the range the value covers, widened by
    /// 14 hours either way when it has no time zone.
    fn range_ms(&self) -> (i64, i64) {
        let (mut low, mut high) = (self.instant_ms(), self.high_edge_ms());
        if self.offset.is_none() {
            let widen = 14 * 60 * MS_PER_MINUTE;
            low -= widen;
            high += widen;
        }
        (low, high)
    }
}

/// `couldBeTheSameTime`: whether the two could be one instant in some zone.
fn could_be_the_same_time(l: &Dt, r: &Dt) -> bool {
    let ((low_left, high_left), (low_right, high_right)) = (l.range_ms(), r.range_ms());
    !(high_right < low_left || high_left < low_right)
}

/// `compareDateTimeElements(l, r, false)`: the ordering used by `<`, `>`,
/// `<=` and `>=`. Both sides are moved to UTC when they are more precise than a
/// day.
pub(super) fn compare_ordered(l: &Dt, r: &Dt) -> Option<Ordering> {
    compare_times(&l.zulu(), &r.zulu())
}

/// `DateTimeType.copy()` re-parses the value and a `DateTimeType` does not
/// accept minute precision, so an equality or equivalence involving one is
/// an error upstream (`Invalid date/time format`).
pub(super) fn check_copyable(d: &Dt, text_of: &str, offset: usize) -> Result<(), EvalError> {
    if d.is_date_time && d.precision == TemporalPrecision::Minute {
        return Err(EvalError::execution(
            format!("Invalid date/time format: \"{text_of}\""),
            offset,
        ));
    }
    Ok(())
}

/// `BaseDateTimeType.equalsUsingFhirPathRules`.
pub(super) fn equals_using_fhir_rules(l: &Dt, r: &Dt) -> Option<bool> {
    if l.offset.is_some() != r.offset.is_some() {
        return if could_be_the_same_time(l, r) {
            None
        } else {
            Some(false)
        };
    }
    compare_times(&l.zulu(), &r.zulu()).map(|o| o == Ordering::Equal)
}

/// A `time`'s hour, minute and second as `TimeType` reads them, and its
/// precision.
struct Tm {
    hour: i64,
    minute: i64,
    second: f32,
    precision: TemporalPrecision,
}

fn parse_time(text: &str) -> Tm {
    let hour = text.get(..2).and_then(digits).unwrap_or(0);
    let minute = text.get(3..5).and_then(digits).unwrap_or(0);
    let second = text
        .get(6..)
        .filter(|s| text.len() >= 8 && !s.is_empty())
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(0.0);
    let precision = match text.len() {
        5 => TemporalPrecision::Minute,
        8 => TemporalPrecision::Second,
        n if n > 9 => TemporalPrecision::Millisecond,
        _ => TemporalPrecision::Second,
    };
    Tm {
        hour,
        minute,
        second,
        precision,
    }
}

/// `compareTimeElements(l, r, false)`.
pub(super) fn compare_time(left: &str, right: &str) -> Option<Ordering> {
    let (l, r) = (parse_time(left), parse_time(right));
    match l.hour.cmp(&r.hour) {
        Ordering::Equal => {}
        o => return Some(o),
    }
    match l.minute.cmp(&r.minute) {
        Ordering::Equal => {}
        o => return Some(o),
    }
    let minute = TemporalPrecision::Minute;
    if l.precision == minute && r.precision == minute {
        return Some(Ordering::Equal);
    }
    if l.precision == minute || r.precision == minute {
        return None;
    }
    Some(l.second.partial_cmp(&r.second).unwrap_or(Ordering::Equal))
}
