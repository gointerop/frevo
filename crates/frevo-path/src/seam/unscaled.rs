//! `Unscaled` -- the arbitrary-precision signed integer a [`Decimal`] carries
//! as its unscaled value.
//!
//! `design.md` section 6 of `add-fhirpath-and-codegen-r4` names the fallback a
//! decimal takes when no allow-listed crate provides one: its lexical form plus
//! an integer mantissa and a scale. Task 44b widens that mantissa from 128 bits
//! to any number of digits (`path.literal.017`, `DEBT-0011`), so that no
//! literal, arithmetic operator, comparison or canonical value fails on a limit
//! of significant digits, as `BigDecimal` does not.
//!
//! The digits are held in base ten because every operation the engine needs
//! on a decimal's unscaled value other than `+`, `-` and `*` is a power of ten:
//! aligning two scales, rounding at a scale, and rendering the value as text.
//!
//! [`Decimal`]: super::value::Decimal

use std::cmp::Ordering;
use std::fmt;

/// The most places a value is ever shifted left by a power of ten.
///
/// A scale is aligned by appending zeros to the digits, and `BigDecimal` keeps
/// a value such as `1E+2000000000` as one digit and a negative scale where this
/// type, which holds a value at a scale of at least zero (`DEBT-0014`), would
/// hold two thousand million digits. A shift is bounded instead: one of more
/// than this many places is refused as unsupported rather than allocated. The
/// bound is on a shift, never on the number of significant digits a value
/// carries.
pub const MAX_SHIFT: usize = 10_000;

/// An arbitrary-precision signed integer in base ten.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Unscaled {
    /// Never set for zero, so zero has one representation.
    negative: bool,
    /// The magnitude's ASCII digits, most significant first, with no leading
    /// zero; zero is `b"0"`.
    digits: Vec<u8>,
}

impl Unscaled {
    /// Zero.
    pub fn zero() -> Self {
        Self {
            negative: false,
            digits: vec![b'0'],
        }
    }

    /// The value whose magnitude `digits` writes (ASCII decimal digits, leading
    /// zeros allowed, empty for zero). `None` when a byte is not a digit.
    pub fn from_digits(negative: bool, digits: &[u8]) -> Option<Self> {
        if !digits.iter().all(u8::is_ascii_digit) {
            return None;
        }
        Some(Self::from_magnitude(negative, trim(digits).to_vec()))
    }

    /// The value of a 128-bit integer.
    pub fn from_i128(value: i128) -> Self {
        Self::from_magnitude(value < 0, value.unsigned_abs().to_string().into_bytes())
    }

    /// The value as a 128-bit integer, when it fits.
    pub fn to_i128(&self) -> Option<i128> {
        let magnitude: u128 = self.magnitude_digits().parse().ok()?;
        if self.negative {
            0i128.checked_sub_unsigned(magnitude)
        } else {
            i128::try_from(magnitude).ok()
        }
    }

    /// Whether the value is zero.
    pub fn is_zero(&self) -> bool {
        self.digits == *b"0"
    }

    /// Whether the value is below zero.
    pub fn is_negative(&self) -> bool {
        self.negative
    }

    /// The magnitude's digits, with no sign and no leading zero.
    pub fn magnitude_digits(&self) -> &str {
        // Every byte is an ASCII digit by construction.
        std::str::from_utf8(&self.digits).unwrap_or("0")
    }

    /// Whether the value is `10^k`.
    pub fn is_power_of_ten(&self, k: usize) -> bool {
        !self.negative
            && self.digits.len() == k.saturating_add(1)
            && self.digits[0] == b'1'
            && self.digits[1..].iter().all(|d| *d == b'0')
    }

    /// The value with its sign inverted.
    pub fn negated(&self) -> Self {
        Self::from_magnitude(!self.negative, self.digits.clone())
    }

    /// The value times `10^k`, or `None` when `k` exceeds [`MAX_SHIFT`]. Zero
    /// stays zero at any `k`.
    pub fn shifted_left(&self, k: usize) -> Option<Self> {
        if self.is_zero() || k == 0 {
            return Some(self.clone());
        }
        if k > MAX_SHIFT {
            return None;
        }
        let mut digits = Vec::with_capacity(self.digits.len() + k);
        digits.extend_from_slice(&self.digits);
        digits.resize(self.digits.len() + k, b'0');
        Some(Self {
            negative: self.negative,
            digits,
        })
    }

    /// The value divided by `10^k` and rounded as `RoundingMode.HALF_UP`
    /// rounds: a discarded fraction of one half or more -- a first discarded
    /// digit of 5 or more -- moves the quotient one unit away from zero, so
    /// `-125` over `10^1` is `-13` and `-5` over `10^1` is `-1`.
    pub fn shifted_right_half_up(&self, k: usize) -> Self {
        if k == 0 {
            return self.clone();
        }
        let len = self.digits.len();
        if k > len {
            return Self::zero();
        }
        let keep = len - k;
        let kept = trim(&self.digits[..keep]);
        let magnitude = if self.digits[keep] >= b'5' {
            add_magnitudes(kept, b"1")
        } else {
            kept.to_vec()
        };
        Self::from_magnitude(self.negative, magnitude)
    }

    /// `self + other`.
    pub fn add(&self, other: &Self) -> Self {
        if self.negative == other.negative {
            return Self::from_magnitude(
                self.negative,
                add_magnitudes(&self.digits, &other.digits),
            );
        }
        match compare_magnitudes(&self.digits, &other.digits) {
            Ordering::Equal => Self::zero(),
            Ordering::Greater => Self::from_magnitude(
                self.negative,
                subtract_magnitudes(&self.digits, &other.digits),
            ),
            Ordering::Less => Self::from_magnitude(
                other.negative,
                subtract_magnitudes(&other.digits, &self.digits),
            ),
        }
    }

    /// `self - other`.
    pub fn sub(&self, other: &Self) -> Self {
        self.add(&other.negated())
    }

    /// `self * other`.
    pub fn mul(&self, other: &Self) -> Self {
        if self.is_zero() || other.is_zero() {
            return Self::zero();
        }
        let (a, b) = (&self.digits, &other.digits);
        // Base-ten slots, least significant first, with the carries deferred:
        // a slot gains at most 81 per row, so a carry pass every million rows
        // keeps every slot far below `u64::MAX`.
        let mut acc = vec![0u64; a.len() + b.len()];
        for (i, x) in a.iter().rev().enumerate() {
            let x = u64::from(x - b'0');
            if x != 0 {
                for (j, y) in b.iter().rev().enumerate() {
                    acc[i + j] += x * u64::from(y - b'0');
                }
            }
            if i % 1_000_000 == 999_999 {
                carry(&mut acc);
            }
        }
        carry(&mut acc);
        let digits: Vec<u8> = acc
            .iter()
            .rev()
            .map(|d| b'0' + u8::try_from(*d).unwrap_or(0))
            .collect();
        Self::from_magnitude(self.negative != other.negative, trim(&digits).to_vec())
    }

    fn from_magnitude(negative: bool, digits: Vec<u8>) -> Self {
        let digits = if digits.is_empty() {
            vec![b'0']
        } else {
            digits
        };
        let zero = digits == *b"0";
        Self {
            negative: negative && !zero,
            digits,
        }
    }
}

/// Normalise base-ten slots holding values above nine, least significant
/// first. The top slot never overflows: the product of an `m`-digit and an
/// `n`-digit number has at most `m + n` digits.
fn carry(acc: &mut [u64]) {
    let mut c = 0u64;
    for slot in acc.iter_mut() {
        let v = *slot + c;
        *slot = v % 10;
        c = v / 10;
    }
}

/// `digits` with its leading zeros removed (empty for zero).
fn trim(digits: &[u8]) -> &[u8] {
    let first = digits
        .iter()
        .position(|d| *d != b'0')
        .unwrap_or(digits.len());
    &digits[first..]
}

fn compare_magnitudes(a: &[u8], b: &[u8]) -> Ordering {
    a.len().cmp(&b.len()).then_with(|| a.cmp(b))
}

fn add_magnitudes(a: &[u8], b: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(a.len().max(b.len()) + 1);
    let (mut i, mut j, mut c) = (a.len(), b.len(), 0u8);
    while i > 0 || j > 0 || c > 0 {
        let mut s = c;
        if i > 0 {
            i -= 1;
            s += a[i] - b'0';
        }
        if j > 0 {
            j -= 1;
            s += b[j] - b'0';
        }
        out.push(b'0' + s % 10);
        c = s / 10;
    }
    out.reverse();
    trim(&out).to_vec()
}

/// `a - b`, for magnitudes with `a >= b`.
fn subtract_magnitudes(a: &[u8], b: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(a.len());
    let (mut i, mut j, mut borrow) = (a.len(), b.len(), 0u8);
    while i > 0 {
        i -= 1;
        let mut take = borrow;
        if j > 0 {
            j -= 1;
            take += b[j] - b'0';
        }
        let have = a[i] - b'0';
        let d = if have >= take {
            borrow = 0;
            have - take
        } else {
            borrow = 1;
            have + 10 - take
        };
        out.push(b'0' + d);
    }
    out.reverse();
    trim(&out).to_vec()
}

impl fmt::Display for Unscaled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.negative {
            f.write_str("-")?;
        }
        f.write_str(self.magnitude_digits())
    }
}

impl fmt::Debug for Unscaled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl PartialEq<i128> for Unscaled {
    fn eq(&self, other: &i128) -> bool {
        self.to_i128() == Some(*other)
    }
}

impl PartialEq<i128> for &Unscaled {
    fn eq(&self, other: &i128) -> bool {
        (**self).eq(other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(text: &str) -> Unscaled {
        let (negative, digits) = match text.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, text),
        };
        Unscaled::from_digits(negative, digits.as_bytes()).expect("digits")
    }

    // covers: path.literal.017
    #[test]
    fn arithmetic_agrees_with_i128_where_i128_holds_the_value() {
        let values: [i128; 10] = [
            0,
            1,
            -1,
            9,
            -10,
            99,
            12345,
            -98765,
            1_000_000_007,
            -1_000_000_000_000_000_000_000,
        ];
        for a in values {
            for b in values {
                let (x, y) = (Unscaled::from_i128(a), Unscaled::from_i128(b));
                assert_eq!(x.add(&y), a + b, "{a} + {b}");
                assert_eq!(x.sub(&y), a - b, "{a} - {b}");
                if let Some(p) = a.checked_mul(b) {
                    assert_eq!(x.mul(&y), p, "{a} * {b}");
                }
            }
        }
        assert_eq!(Unscaled::from_i128(i128::MIN), i128::MIN);
        assert_eq!(Unscaled::from_i128(i128::MAX), i128::MAX);
    }

    // covers: path.literal.017
    #[test]
    fn a_value_wider_than_128_bits_is_carried() {
        let pi = n("31415926535897932384626433832795028841971693993751058209749445923");
        assert_eq!(
            pi.mul(&n("2")).to_string(),
            "62831853071795864769252867665590057683943387987502116419498891846"
        );
        assert_eq!(
            n("99999999999999999999999999999999999999")
                .add(&n("1"))
                .to_string(),
            "100000000000000000000000000000000000000"
        );
        assert_eq!(
            n("100000000000000000000000000000000000000")
                .sub(&n("1"))
                .to_string(),
            "99999999999999999999999999999999999999"
        );
        assert_eq!(n("-0").to_string(), "0");
        assert!(!n("-0").is_negative());
        assert!(!n("-5").add(&n("5")).is_negative());
        assert_eq!(n("1").shifted_left(40).expect("shift").to_i128(), None);
    }

    // covers: path.literal.017
    #[test]
    fn a_power_of_ten_shifts_and_splits_the_digits() {
        assert_eq!(n("12").shifted_left(3), Some(n("12000")));
        assert_eq!(n("0").shifted_left(MAX_SHIFT + 1), Some(n("0")));
        assert_eq!(n("1").shifted_left(MAX_SHIFT + 1), None);
        assert_eq!(n("-12345").shifted_right_half_up(2), -123);
        assert_eq!(n("-12355").shifted_right_half_up(2), -124);
        assert_eq!(n("12345").shifted_right_half_up(3), 12);
        assert_eq!(n("55").shifted_right_half_up(2), 1);
        assert_eq!(n("-5").shifted_right_half_up(1), -1);
        assert_eq!(n("-4").shifted_right_half_up(1), 0);
        assert!(!n("-4").shifted_right_half_up(1).is_negative());
        assert_eq!(n("55").shifted_right_half_up(3), 0);
        assert_eq!(n("999").shifted_right_half_up(1), 100);
        assert!(n("1000").is_power_of_ten(3));
        assert!(!n("-1000").is_power_of_ten(3));
        assert!(!n("1001").is_power_of_ten(3));
    }
}
