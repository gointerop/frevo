//! `org.fhir.ucum.Decimal`: the precision-aware decimal the pinned engine's
//! `/`, and `div` and `mod` with a decimal operand, compute with
//! (`path.math.005` to `.008`, `.015`, `.016`, `DEV-0030`).
//!
//! Derived from Ucum-java/src/main/java/org/fhir/ucum/Decimal.java (the
//! `Decimal(String)` constructor at `:76-83`, `setValueDecimal`, `allZeros`,
//! `countSignificants`, `delete` and `setValueScientific` at `:114-208`,
//! `insert`, `toString`, `copy`, `zero` and `isZero` at `:214-246`,
//! `isWholeNumber` and `asDecimal` at `:289-308`, `trunc` at `:338-351`,
//! `subtract`, `doAdd`, `doSubtract`, `stringAddition` and
//! `stringSubtraction` at `:368-532`, `multiply` at `:535-625`, `divide` at
//! `:627-769`, `trimLeadingZeros`, `divInt` and `modulo` at `:772-798`, the
//! `Decimal(String, int)` and `Decimal(int)` constructors and `one` at
//! `:96-112`, `:248-254`, and `checkForCouldBeWholeNumber` at `:831-855`) and
//! from Ucum-java/src/main/java/org/fhir/ucum/Utilities.java (`isDecimal`,
//! `isInteger`, `noString` and `padLeft`), at
//! db9d22ade3f0f4a667b45817b85a9ac4534e4224 (tag `1.0.10`; live HAPI 8.12.0
//! resolves 1.0.9, whose `Decimal` is bytecode-identical, `path.math.015`).
//! Original work Copyright (c) 2006+, Health Intersections Pty Ltd, licensed
//! under the BSD 3-Clause License reproduced below.
//! Modified by the Frevo authors: only the operations the FHIRPath engine
//! reaches are ported; strings are ASCII byte vectors rather than Java
//! `String`s; a Java `Error` (`"Unhandled"`, `"internal logic error"`) and a
//! `UcumException` are the two arms of [`Failure`] rather than thrown; the
//! `assert`s (disabled in a default JVM) are dropped, as they are in HAPI's
//! run. Every quirk the engine can observe is kept on purpose: the digit rule
//! for a non-terminating quotient, the rounding carry that moves the point one
//! place left (`Decimal.java:751-754`), the negative zero `subtract` leaves,
//! and `Utilities.isDecimal`'s `float` test of a scientific mantissa.
//!
//! Where this differs from the Java, and why each is unreachable from a value
//! HAPI would have accepted:
//!
//! - `Character.isDigit` accepts every Unicode decimal digit; only ASCII
//!   digits are read here, and anything else is "not a valid decimal" (a FHIR
//!   `decimal` or `integer` primitive is ASCII by its regular expression);
//! - `Float.parseFloat`'s hexadecimal form (`0x1p3`) is not recognised by
//!   [`is_decimal`]; a hexadecimal mantissa fails `setValueDecimal` either
//!   way, so only the message would differ.
//!
//! The Ucum-java sources named above carry this notice, reproduced as their
//! licence requires:
//!
//! > BSD 3-Clause License
//! >
//! > Copyright (c) 2006+, Health Intersections Pty Ltd
//! > All rights reserved.
//! >
//! > Redistribution and use in source and binary forms, with or without
//! > modification, are permitted provided that the following conditions are
//! > met:
//! >
//! > * Redistributions of source code must retain the above copyright notice,
//! >   this list of conditions and the following disclaimer.
//! >
//! > * Redistributions in binary form must reproduce the above copyright
//! >   notice, this list of conditions and the following disclaimer in the
//! >   documentation and/or other materials provided with the distribution.
//! >
//! > * Neither the name of the copyright holder nor the names of its
//! >   contributors may be used to endorse or promote products derived from
//! >   this software without specific prior written permission.
//! >
//! > THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS
//! > IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO,
//! > THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
//! > PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR
//! > CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
//! > EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
//! > PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
//! > PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF
//! > LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
//! > NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
//! > SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

use std::cmp::{max, min};

/// What `Decimal` throws.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Failure {
    /// `org.fhir.ucum.UcumException` with this message.
    Ucum(String),
    /// `java.lang.Error` with this message: `doAdd`'s and `doSubtract`'s
    /// `"Unhandled"`, and the `"internal logic error"` of `stringSubtraction`
    /// and `multiply`. The engine catches only `UcumException`, so this
    /// escapes every operator.
    Error(&'static str),
}

/// An ASCII string, as the Java `String`s of `Decimal` hold.
type Str = Vec<u8>;

/// `stringMultiply('0', n)`, which is `Utilities.padLeft("", '0', n)`: no
/// characters for `n <= 0`.
fn zeros(n: i32) -> Str {
    vec![b'0'; usize::try_from(n).unwrap_or(0)]
}

/// `String.length()` as a Java `int`.
fn ilen(s: &[u8]) -> i32 {
    i32::try_from(s.len()).unwrap_or(i32::MAX)
}

/// An `int` index or length that is non-negative where the Java uses it.
fn idx(i: i32) -> usize {
    usize::try_from(i).unwrap_or(0)
}

/// `allZeros(s, start)`.
fn all_zeros(s: &[u8], start: usize) -> bool {
    s.iter().skip(start).all(|&c| c == b'0')
}

/// `delete(value, offset, length)`.
fn delete(value: &[u8], offset: usize, length: usize) -> Str {
    let mut out = value[..offset].to_vec();
    out.extend_from_slice(&value[offset + length..]);
    out
}

/// `insert(ins, value, offset)`.
fn insert(ins: &[u8], value: &[u8], offset: usize) -> Str {
    let mut out = value[..offset].to_vec();
    out.extend_from_slice(ins);
    out.extend_from_slice(&value[offset..]);
    out
}

/// `dig(c)`: `c - '0'`, which is `-1` for the `'/'` `stringSubtraction`'s
/// borrow can write.
fn dig(c: u8) -> i32 {
    i32::from(c) - i32::from(b'0')
}

/// `cdig(i)`: `(char) (i + '0')`.
fn cdig(i: i32) -> u8 {
    u8::try_from(i + i32::from(b'0')).unwrap_or(b'0')
}

/// `stringAddition`: the operands have equal length; a final carry is lost
/// (the Java's `assert(c == 0)` is disabled).
fn string_addition(s1: &[u8], s2: &[u8]) -> Str {
    let mut result = vec![b'0'; s2.len()];
    let mut c = 0;
    for i in (0..s1.len()).rev() {
        let t = c + dig(s1[i]) + dig(s2[i]);
        result[i] = cdig(t % 10);
        c = t / 10;
    }
    result
}

/// `stringSubtraction`: `s1 - s2` for operands of equal length with
/// `s1 >= s2`. A borrow is taken by decrementing the next character of a
/// local copy of `s1`, as the Java does.
fn string_subtraction(s1: &[u8], s2: &[u8]) -> Result<Str, Failure> {
    let mut s1 = s1.to_vec();
    let mut result = vec![b'0'; s2.len()];
    let c = 0;
    for i in (0..s1.len()).rev() {
        let mut t = c + (dig(s1[i]) - dig(s2[i]));
        if t < 0 {
            t += 10;
            if i == 0 {
                return Err(Failure::Error("internal logic error"));
            }
            s1[i - 1] = cdig(dig(s1[i - 1]) - 1);
        }
        result[i] = cdig(t);
    }
    Ok(result)
}

/// `trimLeadingZeros`: `"0"` when nothing else is left.
fn trim_leading_zeros(s: &[u8]) -> Str {
    match s.iter().position(|&c| c != b'0') {
        Some(i) => s[i..].to_vec(),
        None => b"0".to_vec(),
    }
}

/// `Utilities.isInteger`: `Integer.parseInt` succeeds (and `i != i + 1`,
/// which is always true for an `int`).
fn is_integer(s: &[u8]) -> bool {
    std::str::from_utf8(s)
        .ok()
        .and_then(|t| t.parse::<i32>().ok())
        .is_some()
}

/// `Utilities.isDecimal`: `Float.parseFloat` succeeds and the `float` `r`
/// satisfies `r != r + 1`. `r` is the mantissa rounded half to even to a
/// binary32 value, and `r + 1` is rounded the same way, so the test is on the
/// rounded `float`, for either sign, not on a magnitude threshold: it is false
/// for an infinity and for any `r` whose `r + 1` rounds back to `r`
/// (`16777216` and `-16777220` are refused, `16777218` and `-16777216`
/// accepted, `path.math.017`).
fn is_decimal(s: &[u8]) -> bool {
    if s.is_empty() {
        return false;
    }
    // `FloatingDecimal.readJavaFormatString` trims as `String.trim` does.
    let start = s.iter().position(|&c| c > b' ');
    let Some(start) = start else {
        return false;
    };
    let end = s.iter().rposition(|&c| c > b' ').map_or(start, |e| e + 1);
    let t = &s[start..end];
    let (negative, body) = match t.first() {
        Some(b'-') => (true, &t[1..]),
        Some(b'+') => (false, &t[1..]),
        _ => (false, t),
    };
    let r: f32 = if body == b"NaN" {
        f32::NAN
    } else if body == b"Infinity" {
        f32::INFINITY
    } else {
        // Digits with at most one `.`, at least one digit, and an optional
        // `f`/`F`/`d`/`D` suffix. No exponent can be present: the caller split
        // the text at its first `e`.
        let body = match body.last() {
            Some(b'f' | b'F' | b'd' | b'D') => &body[..body.len() - 1],
            _ => body,
        };
        let dots = body.iter().filter(|&&c| c == b'.').count();
        let digits = body.iter().filter(|c| c.is_ascii_digit()).count();
        if dots > 1 || digits == 0 || digits + dots != body.len() {
            return false;
        }
        let Some(value) = std::str::from_utf8(body)
            .ok()
            .and_then(|b| b.parse::<f32>().ok())
        else {
            return false;
        };
        value
    };
    let r = if negative { -r } else { r };
    #[allow(clippy::float_cmp)]
    let differs = r != r + 1.0;
    differs
}

/// `org.fhir.ucum.Decimal`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct UcumDecimal {
    precision: i32,
    scientific: bool,
    negative: bool,
    digits: Str,
    decimal: i32,
}

impl UcumDecimal {
    /// `new Decimal(String)`: lower-cased, then scientific when it contains an
    /// `e`.
    pub(super) fn new(value: &str) -> Result<Self, Failure> {
        let value = value.to_lowercase();
        let mut result = UcumDecimal::default();
        if value.contains('e') {
            result.set_value_scientific(value.as_bytes())?;
        } else {
            result.set_value_decimal(value.as_bytes())?;
        }
        Ok(result)
    }

    /// `new Decimal(String, int)`: the value read as `new Decimal(String)`
    /// reads it, then given `precision` (`Decimal.java:96-104`).
    pub(super) fn with_precision(value: &str, precision: i32) -> Result<Self, Failure> {
        let mut result = Self::new(value)?;
        result.precision = precision;
        Ok(result)
    }

    /// `new Decimal(int)`: `setValueDecimal(Integer.toString(i))`
    /// (`Decimal.java:106-112`).
    pub(super) fn from_int(i: i32) -> Self {
        let mut result = UcumDecimal::default();
        let _ = result.set_value_decimal(i.to_string().as_bytes());
        result
    }

    /// `Decimal.one()`: `new Decimal("1")`.
    pub(super) fn one() -> Self {
        Self::from_int(1)
    }

    fn not_valid(value: &[u8], suffix: &str) -> Failure {
        Failure::Ucum(format!(
            "'{}' is not a valid decimal{suffix}",
            String::from_utf8_lossy(value)
        ))
    }

    /// `setValueDecimal`. On failure the fields it had not yet reached keep
    /// their values, as the Java's do.
    fn set_value_decimal(&mut self, value: &[u8]) -> Result<(), Failure> {
        self.scientific = false;
        let mut dec: i32 = -1;
        self.negative = value.first() == Some(&b'-');
        let mut value = if self.negative { &value[1..] } else { value };
        while value.first() == Some(&b'0') && value.len() > 1 {
            value = &value[1..];
        }
        for (i, &c) in value.iter().enumerate() {
            if c == b'.' && dec == -1 {
                dec = i32::try_from(i).unwrap_or(i32::MAX);
            } else if !c.is_ascii_digit() {
                return Err(Self::not_valid(value, ""));
            }
        }
        if dec == -1 {
            self.precision = ilen(value);
            self.decimal = ilen(value);
            self.digits = value.to_vec();
        } else if dec == ilen(value) - 1 {
            return Err(Self::not_valid(value, ""));
        } else {
            self.decimal = dec;
            if all_zeros(value, 1) {
                self.precision = ilen(value) - 1;
            } else {
                self.precision = count_significants(value);
            }
            self.digits = delete(value, idx(self.decimal), 1);
            if all_zeros(&self.digits, 0) {
                self.precision += 1;
            } else {
                while self.digits.first() == Some(&b'0') {
                    self.digits.remove(0);
                    self.decimal -= 1;
                }
            }
        }
        Ok(())
    }

    /// `setValueScientific`.
    fn set_value_scientific(&mut self, value: &[u8]) -> Result<(), Failure> {
        let at = value.iter().position(|&c| c == b'e').unwrap_or(value.len());
        let s = &value[..at];
        let e = value.get(at + 1..).unwrap_or(&[]);
        if s.is_empty() || s == b"-" || !is_decimal(s) {
            return Err(Self::not_valid(value, " (numeric)"));
        }
        if e.is_empty() || e == b"-" || !is_integer(e) {
            return Err(Self::not_valid(value, " (exponent)"));
        }
        self.set_value_decimal(s)?;
        self.scientific = true;
        let start = usize::from(e.first() == Some(&b'-'));
        if e[start..].iter().any(|c| !c.is_ascii_digit()) {
            // The Java's message opens with no quote: `""+value+"' is ..."`.
            return Err(Failure::Ucum(format!(
                "{}' is not a valid decimal",
                String::from_utf8_lossy(value)
            )));
        }
        let exponent = std::str::from_utf8(e)
            .ok()
            .and_then(|t| t.parse::<i32>().ok())
            .unwrap_or(0);
        self.decimal = self.decimal.wrapping_add(exponent);
        Ok(())
    }

    /// `Decimal.zero()`: `new Decimal("0")`.
    pub(super) fn zero() -> Self {
        UcumDecimal {
            precision: 1,
            scientific: false,
            negative: false,
            digits: b"0".to_vec(),
            decimal: 1,
        }
    }

    /// `isZero`.
    pub(super) fn is_zero(&self) -> bool {
        all_zeros(&self.digits, 0)
    }

    /// `isWholeNumber`: the written form has no `.`.
    pub(super) fn is_whole_number(&self) -> bool {
        !self.as_decimal().contains('.')
    }

    /// `asDecimal`, which is also `toString`. The `-` is written unless the
    /// text is all `0`s, and a `.` is not a `0`, so a zero with a fractional
    /// part is `-0.0`.
    pub(super) fn as_decimal(&self) -> String {
        let mut result = self.digits.clone();
        if self.decimal != ilen(&self.digits) {
            if self.decimal < 0 {
                let mut text = b"0.".to_vec();
                text.extend(zeros(-self.decimal));
                text.extend_from_slice(&self.digits);
                result = text;
            } else if self.decimal < ilen(&result) {
                if self.decimal == 0 {
                    let mut text = b"0.".to_vec();
                    text.extend_from_slice(&result);
                    result = text;
                } else {
                    result = insert(b".", &result, idx(self.decimal));
                }
            } else {
                let pad = zeros(self.decimal - ilen(&result));
                result.extend(pad);
            }
        }
        if self.negative && !all_zeros(&result, 0) {
            result.insert(0, b'-');
        }
        String::from_utf8_lossy(&result).into_owned()
    }

    /// `trunc`.
    pub(super) fn trunc(&self) -> Self {
        if self.decimal < 0 {
            return Self::zero();
        }
        let mut result = self.clone();
        if ilen(&result.digits) >= result.decimal {
            result.digits.truncate(idx(result.decimal));
        }
        if result.digits.is_empty() {
            result.digits = b"0".to_vec();
            result.decimal = 1;
            result.negative = false;
        }
        result
    }

    /// The two operands' digits aligned at the point, each with a leading
    /// `0`, and right-padded to one length; and `max(decimal)`.
    fn aligned(&self, other: &Self) -> (Str, Str, i32) {
        let max_dec = max(self.decimal, other.decimal);
        let mut s1 = zeros(max_dec - self.decimal + 1);
        s1.extend_from_slice(&self.digits);
        let mut s2 = zeros(max_dec - other.decimal + 1);
        s2.extend_from_slice(&other.digits);
        if s1.len() < s2.len() {
            let pad = zeros(ilen(&s2) - ilen(&s1));
            s1.extend(pad);
        } else if s2.len() < s1.len() {
            let pad = zeros(ilen(&s1) - ilen(&s2));
            s2.extend(pad);
        }
        (s1, s2, max_dec)
    }

    /// `subtract`: every sign path leaves a zero difference negative.
    pub(super) fn subtract(&self, other: &Self) -> Result<Self, Failure> {
        let result = if self.negative && !other.negative {
            let mut r = self.do_add(other)?;
            r.negative = true;
            r
        } else if !self.negative && other.negative {
            self.do_add(other)?
        } else if self.negative && other.negative {
            let mut r = self.do_subtract(other)?;
            r.negative = !r.negative;
            r
        } else {
            let mut r = other.do_subtract(self)?;
            r.negative = !r.negative;
            r
        };
        Ok(result)
    }

    /// The point placement shared by `doAdd` and `doSubtract`, and their
    /// `"Unhandled"` errors.
    fn place_point(mut max_dec: i32, mut s3: Str) -> Result<Str, Failure> {
        if s3.first() == Some(&b'1') {
            max_dec += 1;
        } else {
            s3 = delete(&s3, 0, 1);
        }
        if max_dec != ilen(&s3) {
            if max_dec < 0 {
                return Err(Failure::Error("Unhandled"));
            } else if max_dec < ilen(&s3) {
                s3 = insert(b".", &s3, idx(max_dec));
            } else {
                return Err(Failure::Error("Unhandled"));
            }
        }
        Ok(s3)
    }

    /// The precision `doAdd` and `doSubtract` give their result.
    fn sum_precision(&self, other: &Self) -> i32 {
        if self.decimal < other.decimal {
            self.precision
        } else if other.decimal < self.decimal {
            other.precision
        } else {
            min(self.precision, other.precision)
        }
    }

    /// `doAdd`.
    fn do_add(&self, other: &Self) -> Result<Self, Failure> {
        let (s1, s2, max_dec) = self.aligned(other);
        let s3 = Self::place_point(max_dec, string_addition(&s1, &s2))?;
        let mut result = UcumDecimal::default();
        // "won't happen": the Java swallows a failure here.
        let _ = result.set_value_decimal(&s3);
        result.scientific = self.scientific || other.scientific;
        result.precision = self.sum_precision(other);
        Ok(result)
    }

    /// `doSubtract`: the magnitude of the difference, negative when `other`
    /// is the larger.
    fn do_subtract(&self, other: &Self) -> Result<Self, Failure> {
        let (mut s1, mut s2, max_dec) = self.aligned(other);
        let neg = s1 < s2;
        if neg {
            std::mem::swap(&mut s1, &mut s2);
        }
        let s3 = Self::place_point(max_dec, string_subtraction(&s1, &s2)?)?;
        let mut result = UcumDecimal::default();
        // "won't happen": the Java swallows a failure here.
        let _ = result.set_value_decimal(&s3);
        result.negative = neg;
        result.scientific = self.scientific || other.scientific;
        result.precision = self.sum_precision(other);
        Ok(result)
    }

    /// `multiply`: the long multiplication, trimmed of trailing zeros down to
    /// the result's precision.
    pub(super) fn multiply(&self, other: &Self) -> Result<Self, Failure> {
        if self.is_zero() || other.is_zero() {
            return Ok(Self::zero());
        }
        let (mut s1, mut s2, max_dec) = self.aligned(other);
        if s2 > s1 {
            std::mem::swap(&mut s1, &mut s2);
        }
        let mut s: Vec<Str> = vec![Str::new(); s2.len()];
        for i in (0..s2.len()).rev() {
            let mut row = zeros(ilen(&s2) - i32::try_from(i + 1).unwrap_or(i32::MAX));
            let mut c = 0;
            let mut t = 0;
            for j in (0..s1.len()).rev() {
                t = c + dig(s1[j]) * dig(s2[i]);
                row.insert(0, cdig(t % 10));
                c = t / 10;
            }
            // Never entered: both operands open with the `0` `aligned`
            // prepends, so the last step's `t` is the carry alone, below 10.
            // Were it entered, the Java `while (c > 0)` would never end, as
            // neither `t` nor `c` changes; here it writes the digit once.
            if c > 0 {
                row.insert(0, cdig(t % 10));
            }
            s[i] = row;
        }
        let width = s.iter().map(Vec::len).max().unwrap_or(0);
        for row in &mut s {
            let mut padded = vec![b'0'; width - row.len()];
            padded.append(row);
            *row = padded;
        }
        let mut res = Str::new();
        let mut c = 0;
        for i in (0..width).rev() {
            for row in &s {
                c += dig(row[i]);
            }
            res.insert(0, cdig(c % 10));
            c /= 10;
        }
        if c > 0 {
            return Err(Failure::Error("internal logic error"));
        }
        let mut dec = ilen(&res) - ((ilen(&s1) - (max_dec + 1)) * 2);
        while !res.is_empty() && res != b"0" && res.first() == Some(&b'0') {
            res.remove(0);
            dec -= 1;
        }
        let prec = if self.is_whole_number() && other.is_whole_number() {
            // "at least the specified precision, and possibly more"
            max(
                max(ilen(&self.digits), ilen(&other.digits)),
                min(self.precision, other.precision),
            )
        } else if self.is_whole_number() {
            other.precision
        } else if other.is_whole_number() {
            self.precision
        } else {
            min(self.precision, other.precision)
        };
        while ilen(&res) > prec && res.last() == Some(&b'0') {
            res.pop();
        }
        let mut result = UcumDecimal::default();
        // "won't happen": the Java swallows a failure here.
        let _ = result.set_value_decimal(&res);
        result.precision = prec;
        result.decimal = dec;
        result.negative = self.negative != other.negative;
        result.scientific = self.scientific || other.scientific;
        Ok(result)
    }

    /// `divide`: a zero dividend is `zero()` before the divisor is looked at;
    /// a zero divisor is a `UcumException`; otherwise the long division of
    /// `:637-711` and the digit rule of `:713-760`.
    pub(super) fn divide(&self, other: &Self) -> Result<Self, Failure> {
        if self.is_zero() {
            return Ok(Self::zero());
        }
        if other.is_zero() {
            return Err(Failure::Ucum(format!(
                "Attempt to divide {} by zero",
                self.as_decimal()
            )));
        }
        let mut s = b"0".to_vec();
        s.extend_from_slice(&other.digits);
        // max loops we'll do
        let m = max(ilen(&self.digits), ilen(&other.digits)) + 40;
        let mut tens: Vec<Str> = Vec::with_capacity(10);
        tens.push(string_addition(&zeros(ilen(&s)), &s));
        for i in 1..10 {
            let next = string_addition(&tens[i - 1], &s);
            tens.push(next);
        }
        let width = tens[0].len();
        let mut v = self.digits.clone();
        let mut r = Str::new();
        let mut l: i32 = 0;
        let mut d =
            (ilen(&self.digits) - self.decimal + 1) - (ilen(&other.digits) - other.decimal + 1);
        while v.len() < width {
            v.push(b'0');
            d += 1;
        }
        let od = other.digits.len();
        let mut w: Str;
        let mut vi: usize;
        if v[..od] < other.digits[..] {
            if v.len() == width {
                v.push(b'0');
                d += 1;
            }
            w = v[..=od].to_vec();
            vi = w.len();
        } else {
            w = b"0".to_vec();
            w.extend_from_slice(&v[..od]);
            vi = w.len() - 1;
        }

        let stop = |handled: bool, l: i32, vi: usize, w: &[u8]| {
            handled && (l > m || (vi >= v.len() && (w.is_empty() || all_zeros(w, 0))))
        };
        let mut handled = false;
        while !stop(handled, l, vi, &w) {
            l += 1;
            handled = true;
            let mut proc = false;
            for i in (0..=8usize).rev() {
                if tens[i] <= w {
                    proc = true;
                    r.push(cdig(i32::try_from(i).unwrap_or(0) + 1));
                    w = trim_leading_zeros(&string_subtraction(&w, &tens[i])?);
                    if !stop(handled, l, vi, &w) {
                        if vi < v.len() {
                            w.push(v[vi]);
                            vi += 1;
                            handled = false;
                        } else {
                            w.push(b'0');
                            d += 1;
                        }
                        while w.len() < width {
                            w.insert(0, b'0');
                        }
                    }
                    break;
                }
            }
            if !proc {
                if !w.is_empty() {
                    w.remove(0);
                }
                r.push(b'0');
                if !stop(handled, l, vi, &w) {
                    if vi < v.len() {
                        w.push(v[vi]);
                        vi += 1;
                        handled = false;
                    } else {
                        w.push(b'0');
                        d += 1;
                    }
                    while w.len() < width {
                        w.insert(0, b'0');
                    }
                }
            }
        }

        let prec;
        if self.is_whole_number() && other.is_whole_number() && l < m {
            // `for (int i = 0; i < d; i++)`, with `d` falling as zeros go.
            let mut i = 0;
            while i < d {
                if r.last() == Some(&b'0') {
                    r.pop();
                    d -= 1;
                }
                i += 1;
            }
            prec = 100;
        } else {
            prec = if self.is_whole_number() && other.is_whole_number() {
                max(ilen(&self.digits), ilen(&other.digits))
            } else if self.is_whole_number() {
                max(other.precision, ilen(&r) - d)
            } else if other.is_whole_number() {
                max(self.precision, ilen(&r) - d)
            } else {
                max(min(self.precision, other.precision), ilen(&r) - d)
            };
            if ilen(&r) > prec {
                d -= ilen(&r) - prec;
                let p = idx(prec);
                let up = r[p] >= b'5';
                if up {
                    let mut rs = r[..p].to_vec();
                    let mut carry = true;
                    let mut i = rs.len();
                    while carry && i > 0 {
                        i -= 1;
                        if rs[i] == b'9' {
                            rs[i] = b'0';
                        } else {
                            rs[i] += 1;
                            carry = false;
                        }
                    }
                    if carry {
                        // The defect of `:751-754`: the carried `1` is
                        // prepended AND `d` is raised, so the point moves one
                        // place left of where the rounded value puts it.
                        let mut carried = b"1".to_vec();
                        carried.extend(rs);
                        r = carried;
                        d += 1; // cause we added one at the start
                    } else {
                        r = rs;
                    }
                } else {
                    r.truncate(p);
                }
            }
        }

        let mut result = UcumDecimal::default();
        result.set_value_decimal(&r)?;
        result.decimal = ilen(&r) - d;
        result.negative = self.negative != other.negative;
        result.precision = prec;
        result.scientific = self.scientific || other.scientific;
        Ok(result)
    }

    /// `checkForCouldBeWholeNumber` (`Decimal.java:831-855`): with a
    /// precision above 17 and more than three digits, a run of `9`s or `0`s
    /// ending at the second-to-last digit, starting after the first digit and
    /// more than three digits from the end, is cut off (rounding the digit
    /// before a run of `9`s up). The point does not move.
    pub(super) fn check_for_could_be_whole_number(&mut self) {
        if self.precision > 17 && self.digits.len() > 3 {
            let mut i = self.digits.len() - 2;
            let ch = self.digits[i];
            if ch == b'9' {
                while i > 0 && self.digits[i - 1] == b'9' {
                    i -= 1;
                }
                if i > 0 && i < self.digits.len() - 3 {
                    let mut cut = self.digits[..i - 1].to_vec();
                    cut.push(self.digits[i - 1] + 1);
                    self.digits = cut;
                    self.precision = ilen(&self.digits);
                }
            } else if ch == b'0' {
                while i > 0 && self.digits[i - 1] == b'0' {
                    i -= 1;
                }
                if i > 0 && i < self.digits.len() - 3 {
                    self.digits.truncate(i);
                    self.precision = ilen(&self.digits);
                }
            }
        }
    }

    /// `divInt`: `trunc` of the rounded quotient.
    pub(super) fn div_int(&self, other: &Self) -> Result<Self, Failure> {
        Ok(self.divide(other)?.trunc())
    }

    /// `modulo`: `this - trunc(this / other) * other`.
    pub(super) fn modulo(&self, other: &Self) -> Result<Self, Failure> {
        let t = self.div_int(other)?;
        let t2 = t.multiply(other)?;
        self.subtract(&t2)
    }
}

/// `countSignificants`.
fn count_significants(value: &[u8]) -> i32 {
    let mut value = value.to_vec();
    if let Some(i) = value.iter().position(|&c| c == b'.') {
        value = delete(&value, i, 1);
    }
    while value.first() == Some(&b'0') {
        value.remove(0);
    }
    ilen(&value)
}
