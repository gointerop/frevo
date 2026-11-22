//! The string functions that need no regular-expression engine: `replace`,
//! `trim`, `split`, `join`, `encode`, `decode`, `escape` and `unescape`.
//!
//! Derived from org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRPathEngine.java
//! at 73c432b5f43ddbce979978a760ec4aa81e929094: `bytesToHex` at `:4420`,
//! `hexStringToByteArray` at `:4430`, `funcEncode` at `:4440`, `funcDecode` at
//! `:4461`, `funcEscape` at `:4481`, `funcUnescape` at `:4502`, `funcTrim` at
//! `:4523`, `funcSplit` at `:4532`, `funcJoin` at `:4547` and `funcReplace` at
//! `:4839`; and from org.hl7.fhir.utilities/src/main/java/org/hl7/fhir/utilities/
//! `Utilities.java` (`escapeXml`, `unescapeXml`, `escapeJson`, `unescapeJson`,
//! `simpleSplit`, `URLEncode`, `URLDecode`, `isWhitespace`),
//! `CommaSeparatedStringBuilder.java` and `MarkDownProcessor.java`
//! (`makeStringSafeAsMarkdown`, `makeMarkdownForString`).
//! Original work Copyright HL7, Inc. and contributors, licensed under the
//! Apache License, Version 2.0.
//! Modified by the Frevo authors: strings are processed as UTF-16 code units
//! as a Java `String` is (see `strfn.rs`); `java.util.Base64`,
//! `java.net.URLEncoder` and `URLDecoder` are written out by hand rather than
//! pulled in as a crate; a Java exception becomes an execution error.
//!
//! What the pinned engine does here and the specification does not say, and
//! what is kept:
//!
//! - `trim`, `split` and `join` carry **no** `FHIR_TYPES_STRING` guard: they
//!   read `primitiveValue()` behind a size check only;
//! - `join` always appends one item, so an **empty focus yields the empty
//!   string, not an empty collection** (`:4561`). The pinned suite's
//!   `testJoinEmpty` expects empty and HAPI fails it;
//! - `unescape('json')` has two defects, both kept: `\"` appends the letter
//!   `b` (`Utilities.java:488`), and after `\uXXXX` the cursor is not moved
//!   past the four hex digits, so they are appended as text as well
//!   (`AB` is `A0041B`). The suite's `testUnescapeJson2`, `3` and `4`
//!   expect the corrected result and HAPI fails them;
//! - an unknown `encode`/`decode`/`escape`/`unescape` format appends nothing;
//! - `split` with an empty separator never advances in `simpleSplit` and so
//!   never returns; here it is an execution error instead;
//! - `Character.digit` in the hex decoder also reads non-ASCII digits; only
//!   ASCII digits are read here.

use crate::seam::value::Value;
use crate::syntax::ast::{Function, NodeId};

use super::collection::{convert_list_to_string, convert_to_string};
use super::error::At;
use super::operand::{STRING_TYPES, has_type, text_of};
use super::strfn::{from_utf16, utf16};
use super::{Context, EvalError, Run};

fn oob() -> EvalError {
    EvalError::plain("Index 0 out of bounds for length 0")
}

fn npe(what: &str) -> EvalError {
    EvalError::plain(format!(
        "Cannot invoke \"String.{what}()\" because the primitive value is null"
    ))
}

/// `String.getBytes()`: UTF-8, an unpaired surrogate becoming `?`.
fn java_bytes(text: &str) -> Vec<u8> {
    char::decode_utf16(utf16(text))
        .map(|r| r.unwrap_or('?'))
        .collect::<String>()
        .into_bytes()
}

/// `new String(bytes)`: UTF-8, a malformed sequence becoming U+FFFD.
fn java_string(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(char::from(HEX[usize::from(b >> 4)]));
        out.push(char::from(HEX[usize::from(b & 0x0F)]));
    }
    out
}

/// `hexStringToByteArray`: a non-digit reads as -1, the pair wrapping as a
/// Java `byte`; an odd length reads past the end and raises.
fn hex_to_bytes(text: &str) -> Result<Vec<u8>, EvalError> {
    let units = utf16(text);
    let digit = |u: u16| -> i32 {
        u8::try_from(u)
            .ok()
            .and_then(|b| char::from(b).to_digit(16))
            .map_or(-1, |d| i32::try_from(d).unwrap_or(-1))
    };
    let mut out = Vec::with_capacity(units.len() / 2);
    let mut i = 0;
    while i < units.len() {
        let Some(&low) = units.get(i + 1) else {
            return Err(EvalError::plain(format!(
                "Index {} out of bounds for length {}",
                i + 1,
                units.len()
            )));
        };
        let v = (digit(units[i]) << 4) + digit(low);
        out.push(v.to_le_bytes()[0]);
        i += 2;
    }
    Ok(out)
}

fn base64_encode(bytes: &[u8], url: bool) -> String {
    let alphabet: &[u8; 64] = if url {
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_"
    } else {
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
    };
    let at = |v: u32| char::from(alphabet[usize::try_from(v & 63).unwrap_or(0)]);
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = u32::from(chunk[0]);
        let b1 = chunk.get(1).map_or(0, |b| u32::from(*b));
        let b2 = chunk.get(2).map_or(0, |b| u32::from(*b));
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(at(n >> 18));
        out.push(at(n >> 12));
        out.push(if chunk.len() > 1 { at(n >> 6) } else { '=' });
        out.push(if chunk.len() > 2 { at(n) } else { '=' });
    }
    out
}

/// `Base64.getDecoder()` / `getUrlDecoder()`: padding is optional, anything
/// outside the alphabet raises.
fn base64_decode(text: &str, url: bool) -> Result<Vec<u8>, EvalError> {
    let value = |c: u8| -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some(u32::from(c - b'A')),
            b'a'..=b'z' => Some(u32::from(c - b'a') + 26),
            b'0'..=b'9' => Some(u32::from(c - b'0') + 52),
            b'+' if !url => Some(62),
            b'/' if !url => Some(63),
            b'-' if url => Some(62),
            b'_' if url => Some(63),
            _ => None,
        }
    };
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3 + 2);
    let (mut acc, mut n, mut i) = (0u32, 0u32, 0usize);
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'=' {
            let ok = match n {
                2 => bytes.get(i + 1) == Some(&b'='),
                3 => true,
                _ => false,
            };
            if !ok {
                return Err(EvalError::plain(
                    "Input byte array has wrong 4-byte ending unit",
                ));
            }
            i += if n == 2 { 2 } else { 1 };
            if i < bytes.len() {
                return Err(EvalError::plain(format!(
                    "Input byte array has incorrect ending byte at {i}"
                )));
            }
            break;
        }
        let Some(v) = value(c) else {
            return Err(EvalError::plain(format!("Illegal base64 character {c:x}")));
        };
        acc = (acc << 6) | v;
        n += 1;
        if n == 4 {
            out.extend_from_slice(&acc.to_be_bytes()[1..]);
            acc = 0;
            n = 0;
        }
        i += 1;
    }
    match n {
        1 => {
            return Err(EvalError::plain(
                "Last unit does not have enough valid bits",
            ));
        }
        2 => out.push((acc >> 4).to_le_bytes()[0]),
        3 => {
            out.push((acc >> 10).to_le_bytes()[0]);
            out.push((acc >> 2).to_le_bytes()[0]);
        }
        _ => {}
    }
    Ok(out)
}

/// `Utilities.escapeXml`.
fn escape_xml(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            other => out.push(other),
        }
    }
    out
}

/// `Utilities.unescapeXml`: an entity with no `;`, or an unknown one, raises.
fn unescape_xml(text: &str) -> Result<String, EvalError> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '&' {
            i += 1;
            let mut name = String::new();
            loop {
                let Some(&c) = chars.get(i) else {
                    return Err(EvalError::plain(format!(
                        "Index {i} out of bounds for length {}",
                        chars.len()
                    )));
                };
                if c == ';' {
                    break;
                }
                name.push(c);
                i += 1;
            }
            match name.as_str() {
                "lt" => out.push('<'),
                "gt" => out.push('>'),
                "amp" => out.push('&'),
                "quot" => out.push('"'),
                "mu" => out.push('\u{3bc}'),
                other => {
                    return Err(EvalError::plain(format!("unknown XML entity \"{other}\"")));
                }
            }
        } else {
            out.push(chars[i]);
        }
        i += 1;
    }
    Ok(out)
}

/// `Utilities.isWhitespace`.
fn is_java_whitespace(c: char) -> bool {
    matches!(
        u32::from(c),
        0x09..=0x0D | 0x20 | 0x85 | 0xA0 | 0x1680 | 0x2000..=0x200A | 0x2028 | 0x2029 | 0x202F
            | 0x205F | 0x3000
    )
}

/// `Utilities.escapeJson(value)`, with `escapeUnicodeWhitespace` on.
fn escape_json(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    for c in text.chars() {
        match c {
            '\r' => out.push_str("\\r"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            ' ' => out.push(' '),
            c if is_java_whitespace(c) || u32::from(c) < 32 => {
                out.push_str(&format!("\\u{:04x}", u32::from(c)));
            }
            c => out.push(c),
        }
    }
    out
}

/// `Utilities.unescapeJson`, defects included (see the module notes).
fn unescape_json(text: &str) -> Result<String, EvalError> {
    let units = utf16(text);
    let mut out: Vec<u16> = Vec::with_capacity(units.len());
    let bounds = |i: usize| {
        EvalError::plain(format!(
            "Index {i} out of bounds for length {}",
            units.len()
        ))
    };
    let mut i = 0;
    while i < units.len() {
        if units[i] == u16::from(b'\\') {
            i += 1;
            let ch = *units.get(i).ok_or_else(|| bounds(i))?;
            match u8::try_from(ch).ok().map(char::from) {
                // `b.append('b')`: the pinned engine's defect.
                Some('"') => out.push(u16::from(b'b')),
                Some('\\') => out.push(u16::from(b'\\')),
                Some('/') => out.push(u16::from(b'/')),
                Some('b') => out.push(0x08),
                Some('f') => out.push(0x0C),
                Some('n') => out.push(u16::from(b'\n')),
                Some('r') => out.push(u16::from(b'\r')),
                Some('t') => out.push(u16::from(b'\t')),
                Some('u') => {
                    if i + 5 > units.len() {
                        return Err(EvalError::plain(format!(
                            "begin {}, end {}, length {}",
                            i + 1,
                            i + 5,
                            units.len()
                        )));
                    }
                    let hex = from_utf16(&units[i + 1..i + 5]);
                    let code = i32::from_str_radix(&hex, 16)
                        .map_err(|_| EvalError::plain(format!("For input string: \"{hex}\"")))?;
                    let Ok(unit) = u16::try_from(code) else {
                        return Err(EvalError::plain(format!(
                            "Not a valid Unicode code point: 0x{code:X}"
                        )));
                    };
                    out.push(unit);
                }
                _ => {
                    return Err(EvalError::plain(format!(
                        "Unknown JSON escape \\{}",
                        from_utf16(&[ch])
                    )));
                }
            }
        } else {
            out.push(units[i]);
        }
        i += 1;
    }
    Ok(from_utf16(&out))
}

/// `URLEncoder.encode(string, "UTF-8")`.
fn url_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for b in java_bytes(text) {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'.' | b'-' | b'*' | b'_' => {
                out.push(char::from(b));
            }
            b' ' => out.push('+'),
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// `URLDecoder.decode(string, "UTF-8")`.
fn url_decode(text: &str) -> Result<String, EvalError> {
    let bytes = text.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' => {
                let hex = bytes
                    .get(i + 1..i + 3)
                    .ok_or_else(|| EvalError::plain("Incomplete trailing escape (%) pattern"))?;
                let value = std::str::from_utf8(hex)
                    .ok()
                    .filter(|h| h.bytes().all(|b| b.is_ascii_hexdigit()))
                    .and_then(|h| u8::from_str_radix(h, 16).ok())
                    .ok_or_else(|| {
                        EvalError::plain("Illegal hex characters in escape (%) pattern")
                    })?;
                out.push(value);
                i += 3;
            }
            other => {
                out.push(other);
                i += 1;
            }
        }
    }
    Ok(java_string(&out))
}

/// `MarkDownProcessor.makeStringSafeAsMarkdown`.
fn markdown_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if is_escable_markdown(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

fn is_escable_markdown(c: char) -> bool {
    matches!(c, '*' | '&' | '#' | '[' | '>' | '<' | '`')
}

/// `MarkDownProcessor.makeMarkdownForString`.
fn markdown_unescape(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    for (i, &c) in chars.iter().enumerate() {
        let drop = c == '\\' && i + 1 < chars.len() && is_escable_markdown(chars[i + 1]);
        if !drop {
            out.push(c);
        }
    }
    out
}

/// `String.replace(CharSequence, CharSequence)` over UTF-16 code units.
fn java_replace(text: &[u16], target: &[u16], with: &[u16]) -> Vec<u16> {
    let mut out = Vec::with_capacity(text.len());
    if target.is_empty() {
        out.extend_from_slice(with);
        for &u in text {
            out.push(u);
            out.extend_from_slice(with);
        }
        return out;
    }
    let mut i = 0;
    while i < text.len() {
        if text[i..].starts_with(target) {
            out.extend_from_slice(with);
            i += target.len();
        } else {
            out.push(text[i]);
            i += 1;
        }
    }
    out
}

/// `Utilities.simpleSplit`: every separator gives a segment, empty ones kept.
fn simple_split(text: &[u16], sep: &[u16]) -> Vec<String> {
    let mut parts = Vec::new();
    let (mut cursor, mut last) = (0, 0);
    while cursor < text.len() {
        if text[cursor..].starts_with(sep) {
            parts.push(from_utf16(&text[last..cursor]));
            cursor += sep.len();
            last = cursor;
        } else {
            cursor += 1;
        }
    }
    parts.push(from_utf16(&text[last..cursor]));
    parts
}

impl Run<'_> {
    /// `evaluateFunction` for the string functions that need no regex engine.
    pub(super) fn func_string_ext(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
        function: Function,
    ) -> Result<Vec<Value>, EvalError> {
        match function {
            Function::Replace => self.func_replace(context, focus, id),
            Function::Trim => self.func_trim(focus),
            Function::Split => self.func_split(context, focus, id),
            Function::Join => self.func_join(context, focus, id),
            other => self.func_codec(context, focus, id, other),
        }
    }

    /// `funcReplace`: a literal substitution behind the string guard.
    fn func_replace(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        let p0 = self.param(id, 0)?;
        let target = self.execute(context, focus, p0, true)?;
        let p1 = self.param(id, 1)?;
        let with = self.execute(context, focus, p1, true)?;
        if focus.is_empty() || target.is_empty() || with.is_empty() {
            return Ok(Vec::new());
        }
        let [one] = focus else {
            return Err(EvalError::execution(
                format!(
                    "Error evaluating FHIRPath expression: The function replace can only be used on a singleton value but found {}",
                    focus.len()
                ),
                At::node(self.expr.node(id)),
            ));
        };
        if !(self.implicit_string_conversion || has_type(one, &STRING_TYPES)) {
            return Ok(Vec::new());
        }
        let f = convert_to_string(one);
        if f.is_empty() {
            return Ok(vec![Value::String(String::new())]);
        }
        let t = convert_list_to_string(&target);
        let r = convert_list_to_string(&with);
        Ok(vec![Value::String(from_utf16(&java_replace(
            &utf16(&f),
            &utf16(&t),
            &utf16(&r),
        )))])
    }

    /// `funcTrim`: `String.trim()` behind a size check; no type guard.
    fn func_trim(&self, focus: &[Value]) -> Result<Vec<Value>, EvalError> {
        let [one] = focus else {
            return Ok(Vec::new());
        };
        let cnt = text_of(self.model, one).ok_or_else(|| npe("trim"))?;
        // `trim` removes every character up to and including U+0020.
        Ok(vec![Value::String(
            cnt.trim_matches(|c: char| c <= ' ').to_string(),
        )])
    }

    /// `funcSplit`.
    fn func_split(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        let p = self.param(id, 0)?;
        let nl = self.execute(context, focus, p, true)?;
        let first = nl.first().ok_or_else(oob)?;
        let sep = text_of(self.model, first);
        let [one] = focus else {
            return Ok(Vec::new());
        };
        let cnt = text_of(self.model, one);
        // `Utilities.simpleSplit` (`Utilities.java:1979`) answers an empty
        // array for a null text, before the separator is touched.
        let Some(cnt) = cnt else {
            return Ok(Vec::new());
        };
        let Some(sep) = sep else {
            return Err(npe("length"));
        };
        if sep.is_empty() {
            return Err(EvalError::plain(
                "split() with an empty separator does not terminate in the pinned engine",
            ));
        }
        Ok(simple_split(&utf16(&cnt), &utf16(&sep))
            .into_iter()
            .map(Value::String)
            .collect())
    }

    /// `funcJoin`: one string, always, even for an empty focus.
    fn func_join(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
    ) -> Result<Vec<Value>, EvalError> {
        let mut sep = String::new();
        if !self.expr.node(id).parameters().is_empty() {
            let p = self.param(id, 0)?;
            let nl = self.execute(context, focus, p, true)?;
            let first = nl.first().ok_or_else(oob)?;
            // `StringBuilder.append((String) null)` writes `null`.
            sep = text_of(self.model, first).unwrap_or_else(|| "null".to_string());
        }
        // A null item is skipped by `CommaSeparatedStringBuilder`.
        let parts: Vec<String> = focus
            .iter()
            .filter_map(|v| text_of(self.model, v))
            .collect();
        Ok(vec![Value::String(parts.join(&sep))])
    }

    /// `funcEncode`, `funcDecode`, `funcEscape` and `funcUnescape`.
    fn func_codec(
        &self,
        context: Context<'_>,
        focus: &[Value],
        id: NodeId,
        function: Function,
    ) -> Result<Vec<Value>, EvalError> {
        let p = self.param(id, 0)?;
        let nl = self.execute(context, focus, p, true)?;
        let first = nl.first().ok_or_else(oob)?;
        let format = text_of(self.model, first);
        let [one] = focus else {
            return Ok(Vec::new());
        };
        let cnt = text_of(self.model, one).ok_or_else(|| npe("getBytes"))?;
        let out = match (function, format.as_deref()) {
            (Function::Encode, Some("hex")) => bytes_to_hex(&java_bytes(&cnt)),
            (Function::Encode, Some("base64")) => base64_encode(&java_bytes(&cnt), false),
            (Function::Encode, Some("urlbase64")) => base64_encode(&java_bytes(&cnt), true),
            (Function::Decode, Some("hex")) => java_string(&hex_to_bytes(&cnt)?),
            (Function::Decode, Some("base64")) => java_string(&base64_decode(&cnt, false)?),
            (Function::Decode, Some("urlbase64")) => java_string(&base64_decode(&cnt, true)?),
            (Function::Escape, Some("html")) => escape_xml(&cnt),
            (Function::Escape, Some("json")) => escape_json(&cnt),
            (Function::Escape, Some("url")) => url_encode(&cnt),
            (Function::Escape, Some("md")) => markdown_escape(&cnt),
            (Function::Unescape, Some("html")) => unescape_xml(&cnt)?,
            (Function::Unescape, Some("json")) => unescape_json(&cnt)?,
            (Function::Unescape, Some("url")) => url_decode(&cnt)?,
            (Function::Unescape, Some("md")) => markdown_unescape(&cnt),
            _ => return Ok(Vec::new()),
        };
        Ok(vec![Value::String(out)])
    }
}
