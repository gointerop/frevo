//! The base string functions: `upper`, `lower`, `toChars`, `indexOf`,
//! `substring`, `startsWith`, `endsWith`, `contains` and `length`.
//!
//! Derived from org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java
//! at 73c432b5f43ddbce979978a760ec4aa81e929094: `funcEndsWith` at `:4892`,
//! `funcContains` at `:5739`, `funcLength` at `:5767`, `funcStartsWith` at
//! `:5787`, `funcLower` at `:5809`, `funcUpper` at `:5820`, `funcToChars` at
//! `:5831`, `funcIndexOf` at `:5842` and `funcSubstring` at `:5864`.
//! Original work Copyright HL7, Inc. and contributors, licensed under the
//! Apache License, Version 2.0.
//! Modified by the Frevo authors: positions and lengths are counted in UTF-16
//! code units, as a Java `String` counts them, by converting to code units at
//! the call; `toChars` yields one item per UTF-16 unit, and a lone surrogate
//! half, which a Rust `String` cannot hold, is carried as the code point
//! `U+10F800 + (unit - 0xD800)` (the last 2048 code points of plane 16, a
//! private-use range). [`utf16`] reads such a code point back as the one unit
//! it stands for and [`from_utf16`] writes an unpaired unit as one, so
//! `length`, `indexOf`, `substring` and equality see Java's string. The cost
//! is that a real character in `U+10F800..=U+10FFFF` is read as a surrogate;
//! the guard's `doImplicitStringConversion` is the evaluator's setting.
//!
//! What the pinned engine does here and the specification does not say, and
//! what is kept:
//!
//! - the guard `focus.get(0).hasType(FHIR_TYPES_STRING) ||
//!   doImplicitStringConversion` is on all of these: a focus that is not
//!   string-like answers **empty**, not an error (`DEV-0006`);
//! - `upper`, `lower` and `substring` answer empty for an empty result string;
//! - `startsWith`, `endsWith` and `indexOf` read the parameter against the
//!   focus, `contains` against `$this`;
//! - an empty needle is `true` for the three tests and `0` for `indexOf`;
//! - `substring` parses its start **before** looking at the focus, so an empty
//!   or non-integer start raises even over an empty focus.

use crate::seam::value::Value;
use crate::syntax::ast::{Function, NodeId};

use super::collection::{convert_list_to_string, convert_to_string};
use super::operand::{STRING_TYPES, has_type, int_of, text_of};
use super::{Context, EvalError, Run};

/// The first code point that stands for a lone surrogate.
pub(super) const LONE_BASE: u32 = 0x10F800;

pub(super) fn lone_unit(c: char) -> Option<u16> {
    let c = u32::from(c);
    (LONE_BASE..=0x10FFFF)
        .contains(&c)
        .then(|| u16::try_from(0xD800 + (c - LONE_BASE)).unwrap_or(0))
}

fn has_lone(text: &str) -> bool {
    // Plane 16 is encoded with the lead byte 0xF4; ordinary text skips this.
    text.as_bytes().contains(&0xF4) && text.chars().any(|c| lone_unit(c).is_some())
}

/// The UTF-16 code units of a Java string.
pub(super) fn utf16(text: &str) -> Vec<u16> {
    let mut units = Vec::with_capacity(text.len());
    for c in text.chars() {
        match lone_unit(c) {
            Some(unit) => units.push(unit),
            None => {
                let mut buf = [0u16; 2];
                units.extend_from_slice(c.encode_utf16(&mut buf));
            }
        }
    }
    units
}

/// A Java string of these code units; an unpaired surrogate is kept.
pub(super) fn from_utf16(units: &[u16]) -> String {
    char::decode_utf16(units.iter().copied())
        .map(|r| {
            r.unwrap_or_else(|e| {
                let at = u32::from(e.unpaired_surrogate()) - 0xD800;
                char::from_u32(LONE_BASE + at).unwrap_or(char::REPLACEMENT_CHARACTER)
            })
        })
        .collect()
}

fn string(text: String) -> Value {
    Value::String(text)
}

fn boolean(b: bool) -> Value {
    Value::Boolean(b)
}

fn integer(i: usize) -> Value {
    Value::Integer(i64::try_from(i).unwrap_or(i64::MAX))
}

/// `String.indexOf(needle)` over UTF-16 code units: the position, or -1.
fn index_of(haystack: &[u16], needle: &[u16]) -> i64 {
    if needle.len() > haystack.len() {
        return -1;
    }
    (0..=haystack.len() - needle.len())
        .find(|&at| haystack[at..at + needle.len()] == *needle)
        .map_or(-1, |at| i64::try_from(at).unwrap_or(i64::MAX))
}

impl Run<'_> {
    /// `hasType(FHIR_TYPES_STRING) || doImplicitStringConversion`.
    fn string_guard(&self, value: &Value) -> bool {
        self.implicit_string_conversion || has_type(value, &STRING_TYPES)
    }

    /// `evaluateFunction` for the base string functions.
    pub(super) fn func_string(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
        function: Function,
    ) -> Result<Vec<Value>, EvalError> {
        match function {
            Function::Length => Ok(match focus {
                [one] if self.string_guard(one) => {
                    vec![integer(utf16(&convert_to_string(one)).len())]
                }
                _ => Vec::new(),
            }),
            Function::Upper | Function::Lower => Ok(match focus {
                [one] if self.string_guard(one) => {
                    let s = convert_to_string(one);
                    if s.is_empty() {
                        Vec::new()
                    } else if function == Function::Upper {
                        vec![string(s.to_uppercase())]
                    } else {
                        vec![string(s.to_lowercase())]
                    }
                }
                _ => Vec::new(),
            }),
            Function::ToChars => Ok(match focus {
                [one] if self.string_guard(one) => utf16(&convert_to_string(one))
                    .iter()
                    .map(|unit| string(from_utf16(&[*unit])))
                    .collect(),
                _ => Vec::new(),
            }),
            Function::StartsWith | Function::EndsWith | Function::IndexOf => {
                self.func_affix(context, focus, id, function)
            }
            Function::Contains => self.func_contains(context, focus, id),
            Function::Substring => self.func_substring(context, focus, id),
            other => Err(EvalError::unsupported(
                format!("the function {}()", other.to_code()),
                self.expr.node(id).start(),
            )),
        }
    }

    /// `funcStartsWith`, `funcEndsWith` and `funcIndexOf`: the parameter is
    /// read against the focus.
    fn func_affix(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
        function: Function,
    ) -> Result<Vec<Value>, EvalError> {
        let param = self.param(id, 0)?;
        let needle = self.execute(context, focus, param, true)?;
        let sw = convert_list_to_string(&needle);
        let Some(first) = focus.first() else {
            return Ok(Vec::new());
        };
        if needle.is_empty() {
            return Ok(Vec::new());
        }
        if sw.is_empty() {
            return Ok(vec![if function == Function::IndexOf {
                Value::Integer(0)
            } else {
                boolean(true)
            }]);
        }
        if !self.string_guard(first) {
            return Ok(Vec::new());
        }
        let s = convert_to_string(first);
        // A pair split into halves is matched by unit, as Java matches it.
        let by_unit = has_lone(&s) || has_lone(&sw);
        Ok(vec![match function {
            Function::StartsWith if by_unit => boolean(utf16(&s).starts_with(&utf16(&sw))),
            Function::StartsWith => boolean(s.starts_with(&sw)),
            // `focus.size() == 1 && !noString(sw)`, else `false`.
            Function::EndsWith if by_unit => {
                boolean(focus.len() == 1 && utf16(&s).ends_with(&utf16(&sw)))
            }
            Function::EndsWith => boolean(focus.len() == 1 && s.ends_with(&sw)),
            _ => Value::Integer(index_of(&utf16(&s), &utf16(&sw))),
        }])
    }

    /// `funcContains`: the parameter is read against `$this`, not the focus.
    fn func_contains(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        let param = self.param(id, 0)?;
        let this: &[Value] = context.this.map_or(&[], std::slice::from_ref);
        let needle = self.execute(context, this, param, true)?;
        let sw = convert_list_to_string(&needle);
        let ([one], [_]) = (focus, needle.as_slice()) else {
            return Ok(Vec::new());
        };
        if sw.is_empty() {
            return Ok(vec![boolean(true)]);
        }
        if !self.string_guard(one) {
            return Ok(Vec::new());
        }
        let st = convert_to_string(one);
        let found = if has_lone(&st) || has_lone(&sw) {
            index_of(&utf16(&st), &utf16(&sw)) >= 0
        } else {
            st.contains(&sw)
        };
        Ok(vec![boolean(!st.is_empty() && found)])
    }

    /// `funcSubstring`.
    fn func_substring(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        let node = self.expr.node(id);
        let p1 = self.param(id, 0)?;
        let n1 = self.execute(context, focus, p1, true)?;
        // `n1.get(0)` on an empty list raises.
        let Some(first) = n1.first() else {
            return Err(EvalError::plain("Index 0 out of bounds for length 0"));
        };
        let start = int_of(self.model, first)?;
        let mut length = -1;
        if node.parameters().len() == 2 {
            let p2 = self.param(id, 1)?;
            let n2 = self.execute(context, focus, p2, true)?;
            let parsed = n2
                .first()
                .and_then(|v| text_of(self.model, v).and_then(|t| t.parse::<i32>().ok()));
            match parsed {
                Some(i) => length = i,
                None => return Ok(Vec::new()),
            }
        }
        let [one] = focus else {
            return Ok(Vec::new());
        };
        if !self.string_guard(one) {
            return Ok(Vec::new());
        }
        let units = utf16(&convert_to_string(one));
        let total = i32::try_from(units.len()).unwrap_or(i32::MAX);
        if start < 0 || start >= total {
            return Ok(Vec::new());
        }
        let begin = usize::try_from(start).unwrap_or(0);
        let end = if node.parameters().len() == 2 {
            // `Math.min(sw.length(), i1 + i2)`, the sum wrapping as a Java int.
            let end = total.min(start.wrapping_add(length));
            let Ok(end) = usize::try_from(end) else {
                return Err(substring_range(start, end, total));
            };
            if end < begin {
                return Err(substring_range(
                    start,
                    i32::try_from(end).unwrap_or(0),
                    total,
                ));
            }
            end
        } else {
            units.len()
        };
        let s = from_utf16(&units[begin..end]);
        Ok(if s.is_empty() {
            Vec::new()
        } else {
            vec![string(s)]
        })
    }
}

fn substring_range(begin: i32, end: i32, length: i32) -> EvalError {
    EvalError::plain(format!("begin {begin}, end {end}, length {length}"))
}
