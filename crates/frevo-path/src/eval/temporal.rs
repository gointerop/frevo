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
//! Task 38 adds `dateAdd` (`FHIRPathEngine.java:2715`, over
//! `BaseDateTimeType.add` at `:117`), `funcNow` and `funcToday` (`:4772`,
//! `:4779`), and the evaluation time zone.
//!
//! Where the pinned engine reads the JVM's default time zone -- a dateTime with
//! **no** offset is a `Date` in that zone, so it matters when such a value is
//! compared with one that has an offset, in `couldBeTheSameTime` and in the
//! conversion to Zulu, and `today()` and `now()` are read in it -- this port
//! reads the **evaluation zone** the [`super::Evaluator`] was configured with,
//! a fixed offset, UTC when none is configured (`DEV-0018`). A zone with
//! daylight saving is not modelled: the offset is fixed for an evaluation.
//! Two values that both lack an offset compare identically in any zone.

use std::cell::Cell;
use std::cmp::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::seam::value::{PartialDate, PartialDateTime, TemporalPrecision, Value};

use super::EvalError;
use super::navigate::fhir_type;
use super::operand::Qty;

thread_local! {
    /// The evaluation zone, minutes east of UTC, for the evaluation running on
    /// this thread. Held here rather than threaded through every equality and
    /// membership helper that can end in a date comparison.
    static EVAL_ZONE: Cell<i32> = const { Cell::new(0) };
}

/// The evaluation zone in minutes east of UTC.
fn zone() -> i32 {
    EVAL_ZONE.with(Cell::get)
}

/// Sets the evaluation zone for its lifetime and restores the previous one.
pub(super) struct ZoneGuard(i32);

impl ZoneGuard {
    pub(super) fn set(minutes: i32) -> Self {
        Self(EVAL_ZONE.with(|z| z.replace(minutes)))
    }
}

impl Drop for ZoneGuard {
    fn drop(&mut self) {
        EVAL_ZONE.with(|z| z.set(self.0));
    }
}

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
    /// The zone designator as written (`Z`, `+10:00`), so a result renders it
    /// back exactly.
    zone_text: Option<String>,
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

/// `Integer.parseInt`: an optional `+` or `-`, then at least one ASCII
/// digit.
fn java_parse_int(text: &str) -> Option<i64> {
    let (negative, body) = match text.as_bytes().first() {
        Some(b'-') => (true, &text[1..]),
        Some(b'+') => (false, &text[1..]),
        _ => (false, text),
    };
    let magnitude = digits(body)?;
    Some(if negative { -magnitude } else { magnitude })
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
        let mut zone_text = None;
        if let Some(time) = time {
            if precision != TemporalPrecision::Day {
                return None;
            }
            let (clock, zone) = match time.find(['Z', '+', '-']) {
                Some(at) => (&time[..at], Some(&time[at..])),
                None => (time, None),
            };
            zone_text = zone.map(str::to_string);
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
            match (fields.next(), zone) {
                // `hh:mm:` straight before the offset: `BaseDateTimeType.parse`
                // reads the seconds positionally, as `Integer.parseInt` of the
                // two characters after the `:` (`:489`-`:491`), so they are the
                // offset's first two and `+1` or `-0` is a second. A boundary at
                // precision 14 builds such a text (`path.boundary.015`).
                (Some(""), Some(z)) if fraction.is_none() => {
                    second = java_parse_int(z.get(..2)?).filter(|s| (0..=60).contains(s))?;
                    precision = TemporalPrecision::Second;
                }
                (Some(s), _) => {
                    second = digits(s)?;
                    precision = TemporalPrecision::Second;
                }
                (None, _) => {}
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
            zone_text: offset.and(zone_text),
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
        let offset = i64::from(self.offset.unwrap_or_else(zone));
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
        ((days * 24 + self.hour) * 60 + self.minute - i64::from(self.offset.unwrap_or_else(zone)))
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
        // `copy()` re-parses the string. With **no** zone the parse itself
        // fails (`BaseDateTimeType.java:797`); with one it succeeds and the
        // precision check then raises (`:103-106`) (`DEV-0017`).
        let message = if d.offset.is_some() {
            format!(
                "Invalid date/time string (datatype DateTimeType does not support MINUTE \
                 precision): {text_of}"
            )
        } else {
            format!("Invalid date/time format: \"{text_of}\"")
        };
        return Err(EvalError::execution(message, offset));
    }
    Ok(())
}

/// `Math.floorMod`-style month arithmetic: the last day of a month.
fn days_in_month(year: i64, month: i64) -> i64 {
    let next = if month == 12 {
        days_from_civil(year + 1, 1, 1)
    } else {
        days_from_civil(year, month + 1, 1)
    };
    next - days_from_civil(year, month, 1)
}

impl Dt {
    /// `DateUtils.addYears` / `addMonths`: whole months, the day clamped to the
    /// last day of the target month.
    fn add_months(&mut self, months: i64) {
        let total = self.year * 12 + (self.month - 1) + months;
        self.year = total.div_euclid(12);
        self.month = total.rem_euclid(12) + 1;
        self.day = self.day.min(days_in_month(self.year, self.month));
    }

    /// `addDays` through `addMilliseconds`: a fixed-offset shift of the local
    /// fields by `ms` milliseconds.
    fn add_ms(&mut self, ms: i64) {
        let local = ((days_from_civil(self.year, self.month, self.day) * 24 + self.hour) * 60
            + self.minute)
            * MS_PER_MINUTE
            + self.second * 1000
            + self.milli
            + ms;
        let days = local.div_euclid(MS_PER_DAY);
        let of_day = local.rem_euclid(MS_PER_DAY);
        let (y, m, d) = civil_from_days(days);
        self.year = y;
        self.month = m;
        self.day = d;
        self.hour = of_day / (60 * MS_PER_MINUTE);
        self.minute = of_day / MS_PER_MINUTE % 60;
        self.second = of_day / 1000 % 60;
        self.milli = of_day % 1000;
    }

    /// `setValue(Date, precision)`: the value written at its precision, with
    /// the zone designator it had.
    fn render(&self) -> String {
        use TemporalPrecision as P;
        let mut text = format!("{:04}", self.year);
        if self.precision >= P::Month {
            text.push_str(&format!("-{:02}", self.month));
        }
        if self.precision >= P::Day {
            text.push_str(&format!("-{:02}", self.day));
        }
        if self.precision > P::Day {
            text.push_str(&format!("T{:02}:{:02}", self.hour, self.minute));
            if self.precision >= P::Second {
                text.push_str(&format!(":{:02}", self.second));
            }
            if self.precision == P::Millisecond {
                text.push_str(&format!(".{:03}", self.milli));
            }
            if let Some(z) = &self.zone_text {
                text.push_str(z);
            }
        }
        text
    }

    fn into_value(self) -> Value {
        let lexical = self.render();
        if self.is_date_time {
            Value::DateTime(PartialDateTime {
                lexical,
                precision: self.precision,
                offset_minutes: self.offset,
            })
        } else {
            Value::Date(PartialDate {
                lexical,
                precision: self.precision,
            })
        }
    }
}

/// `BigDecimal.intValue()` of a quantity's value: the integer part, low 32
/// bits.
fn int_value(text: &str) -> i32 {
    let integer_part = text.split('.').next().unwrap_or("0");
    let (negative, digits) = match integer_part.strip_prefix('-') {
        Some(d) => (true, d),
        None => (false, integer_part.trim_start_matches('+')),
    };
    let magnitude: i128 = digits.parse().unwrap_or(0);
    let signed = if negative { -magnitude } else { magnitude };
    signed as i32
}

/// `dateAdd(d, q, negate, holder)` (`FHIRPathEngine.java:2715`).
///
/// The unit is the quantity's code when it has one, else its unit text, so
/// `1 year` (unit only) and `1 'd'` (code) both reach a calendar arm while
/// `1 'a'` and `1 'mo'` are refused.
pub(super) fn date_add(
    value: &Value,
    q: &Qty,
    negate: bool,
    at: usize,
) -> Result<Value, EvalError> {
    let Some(mut dt) = from_value(value, at)? else {
        return Err(EvalError::execution(
            format!("{} is not a date or dateTime", fhir_type(value)),
            at,
        ));
    };
    check_copyable(&dt, &super::collection::convert_to_string(value), at)?;
    let amount = int_value(q.value.as_deref().unwrap_or("0"));
    let amount = i64::from(if negate {
        0i32.wrapping_sub(amount)
    } else {
        amount
    });
    let unit = q.code.as_deref().or(q.unit.as_deref());
    let shown = q.code.as_deref().unwrap_or("null");
    match unit {
        Some("years" | "year") => dt.add_months(amount * 12),
        Some("a") => {
            return Err(EvalError::execution(
                format!(
                    "Error in date arithmetic: attempt to add a definite quantity duration \
                     time unit {shown}"
                ),
                at,
            ));
        }
        Some("months" | "month") => dt.add_months(amount),
        Some("mo") => {
            return Err(EvalError::execution(
                format!(
                    "Error in date arithmetic: attempt to add a definite quantity duration \
                     time unit {shown}"
                ),
                at,
            ));
        }
        Some("weeks" | "week" | "wk") => dt.add_ms(amount * 7 * MS_PER_DAY),
        Some("days" | "day" | "d") => dt.add_ms(amount * MS_PER_DAY),
        Some("hours" | "hour" | "h") => dt.add_ms(amount * 60 * MS_PER_MINUTE),
        Some("minutes" | "minute" | "min") => dt.add_ms(amount * MS_PER_MINUTE),
        Some("seconds" | "second" | "s") => dt.add_ms(amount * 1000),
        Some("milliseconds" | "millisecond" | "ms") => dt.add_ms(amount),
        _ => {
            return Err(EvalError::execution(
                format!("Error in date arithmetic: unrecognized time unit {shown}"),
                at,
            ));
        }
    }
    Ok(dt.into_value())
}

/// The current instant as milliseconds since the epoch.
fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

/// `funcToday`: `new DateType(new Date(), DAY)`, the date in the evaluation
/// zone.
pub(super) fn today() -> Value {
    let local = now_ms() + i64::from(zone()) * MS_PER_MINUTE;
    let (year, month, day) = civil_from_days(local.div_euclid(MS_PER_DAY));
    Value::Date(PartialDate {
        lexical: format!("{year:04}-{month:02}-{day:02}"),
        precision: TemporalPrecision::Day,
    })
}

/// `funcNow`: `DateTimeType.now()`, millisecond precision in the evaluation
/// zone with its numeric offset. Read afresh on every call (`DEV-0008`).
pub(super) fn now() -> Value {
    let offset = zone();
    let local = now_ms() + i64::from(offset) * MS_PER_MINUTE;
    let (year, month, day) = civil_from_days(local.div_euclid(MS_PER_DAY));
    let of_day = local.rem_euclid(MS_PER_DAY);
    let sign = if offset < 0 { '-' } else { '+' };
    let magnitude = offset.abs();
    let lexical = format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}{sign}{:02}:{:02}",
        of_day / (60 * MS_PER_MINUTE),
        of_day / MS_PER_MINUTE % 60,
        of_day / 1000 % 60,
        of_day % 1000,
        magnitude / 60,
        magnitude % 60,
    );
    Value::DateTime(PartialDateTime {
        lexical,
        precision: TemporalPrecision::Millisecond,
        offset_minutes: Some(offset),
    })
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
    // Only a value that has a time zone is moved to Zulu
    // (`BaseDateTimeType.java:926-931`); two values without one compare by
    // the fields as written.
    let to_zulu = |d: &Dt| {
        if d.offset.is_some() {
            d.zulu()
        } else {
            d.clone()
        }
    };
    compare_times(&to_zulu(l), &to_zulu(r)).map(|o| o == Ordering::Equal)
}

/// The string `setTimeZoneZulu(true)` leaves on a `DateTimeType` finer than a
/// day (`BaseDateTimeType.java:690-694`, `encode` at `:190`): the instant in
/// UTC, at the same precision, the fractional seconds as written padded to
/// three digits, and `Z`. A value with no offset is read in the evaluation
/// zone, as upstream reads it in the JVM default zone. `None` for a value at
/// day precision or coarser, which is not rewritten, and for one that does
/// not parse.
pub(super) fn zulu_lexical(text: &str) -> Option<String> {
    let d = Dt::parse(text, true)?;
    if d.precision <= TemporalPrecision::Day {
        return None;
    }
    let mut z = d.zulu();
    z.offset = Some(0);
    z.zone_text = Some("Z".to_string());
    let mut rendered = z.render();
    if z.precision == TemporalPrecision::Millisecond {
        // `myFractionalSeconds` is kept as written, not re-derived.
        let fraction = text
            .split_once('.')
            .map(|(_, f)| f.split(['Z', '+', '-']).next().unwrap_or(""))
            .unwrap_or("");
        let fraction = format!("{fraction:0<3}");
        if let Some((head, _)) = rendered.split_once('.') {
            rendered = format!("{head}.{fraction}Z");
        }
    }
    Some(rendered)
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
