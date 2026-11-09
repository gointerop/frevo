//! Byte spans, the instrumentation counters, and the JSON scanner.
//!
//! The scanner is deliberately not a parser. It answers two questions —
//! "where does this value end" and "what are this object's members" — and
//! counts what it looked at while answering, because `model.bytes.002` and
//! `model.perf.001` are stated in terms of a token counter the codec exposes,
//! so the counter is part of the specified surface rather than a debug
//! convenience.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// A byte range into the stored buffer. Half-open: `start..end`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    pub fn is_empty(&self) -> bool {
        self.end <= self.start
    }

    pub fn of<'b>(&self, bytes: &'b [u8]) -> &'b [u8] {
        &bytes[self.start..self.end]
    }
}

/// A decode failure, with the element path and the byte offset
/// `model.errors.001` requires of every error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeError {
    pub path: String,
    pub offset: usize,
    pub message: String,
}

impl DecodeError {
    pub fn new(path: impl Into<String>, offset: usize, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            offset,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {} (at byte {})",
            self.path, self.message, self.offset
        )
    }
}

impl std::error::Error for DecodeError {}

/// What a run looked at. Shared by every handle into one document, so the
/// numbers describe the whole run rather than one navigation.
#[derive(Debug)]
pub struct Meter {
    tokens: AtomicU64,
    size_proportional_allocations: AtomicU64,
    xhtml_validated: AtomicBool,
    record_paths: AtomicBool,
    materialised: Mutex<Vec<String>>,
}

impl Default for Meter {
    fn default() -> Self {
        Self {
            tokens: AtomicU64::new(0),
            size_proportional_allocations: AtomicU64::new(0),
            xhtml_validated: AtomicBool::new(false),
            record_paths: AtomicBool::new(true),
            materialised: Mutex::new(Vec::new()),
        }
    }
}

impl Meter {
    /// A meter that records the path of every materialised element. This is
    /// what the `model.lazy.*` scenarios observe, and it costs one `String` per
    /// element.
    pub fn new() -> Self {
        Self::default()
    }

    /// A meter that counts but does not build the path log.
    ///
    /// The distinction is not cosmetic. A full decode of a 4 KiB `Patient`
    /// materialises 231 elements, so the log is 231 allocations that the decode
    /// itself does not need. `model.perf.003` budgets the **decode**, so the
    /// `bench/micro/` workload has to measure it without the log, while the
    /// `model.lazy.*` scenarios need the log on. Both numbers are real; they
    /// measure different things, and conflating them would make the budget look
    /// missed when it is not.
    pub fn untracked() -> Self {
        let meter = Self::default();
        meter.record_paths.store(false, Ordering::Relaxed);
        meter
    }

    /// Count `n` JSON tokens as scanned. A token is one structural character
    /// (`{`, `}`, `[`, `]`), one property name, or one scalar value. A string
    /// is one token however long it is, which is why a 50 KiB `text.div` that
    /// is stepped over costs one token and not fifty thousand
    /// (`model.lazy.006`).
    pub fn tokens(&self, n: u64) {
        self.tokens.fetch_add(n, Ordering::Relaxed);
    }

    /// Count one heap allocation whose size scales with the document: a
    /// materialised value, an object's member list, or an output buffer the
    /// codec created. `model.bytes.002` requires this to stay at zero across a
    /// load-and-serialise-unchanged, so every such allocation is counted at
    /// its site rather than inferred.
    pub fn size_proportional_allocation(&self) {
        self.size_proportional_allocations
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn mark_xhtml_validated(&self) {
        self.xhtml_validated.store(true, Ordering::Relaxed);
    }

    /// Record that `path` was materialised: an owned value for it now exists.
    /// Indexing an object is not materialising it — that is the distinction
    /// `model.lazy.003` grades.
    pub fn materialise(&self, path: &str) {
        if !self.record_paths.load(Ordering::Relaxed) {
            return;
        }
        self.size_proportional_allocation();
        if let Ok(mut log) = self.materialised.lock() {
            log.push(path.to_string());
        }
    }

    /// Whether the path log is being built. A caller that would have to format
    /// a path string only for the log can skip the work.
    pub fn records_paths(&self) -> bool {
        self.record_paths.load(Ordering::Relaxed)
    }

    pub fn tokens_scanned(&self) -> u64 {
        self.tokens.load(Ordering::Relaxed)
    }

    pub fn allocations(&self) -> u64 {
        self.size_proportional_allocations.load(Ordering::Relaxed)
    }

    pub fn xhtml_was_validated(&self) -> bool {
        self.xhtml_validated.load(Ordering::Relaxed)
    }

    pub fn materialised_paths(&self) -> Vec<String> {
        self.materialised
            .lock()
            .map(|log| log.clone())
            .unwrap_or_default()
    }
}

/// Advance past JSON whitespace. Whitespace is not a token.
pub fn skip_ws(bytes: &[u8], mut at: usize) -> usize {
    while matches!(bytes.get(at), Some(b' ' | b'\t' | b'\r' | b'\n')) {
        at += 1;
    }
    at
}

/// Scan one JSON string starting at its opening quote. Returns the span of the
/// text **inside** the quotes and the offset just past the closing quote.
/// Counts one token.
pub fn scan_string(bytes: &[u8], at: usize, meter: &Meter) -> Result<(Span, usize), DecodeError> {
    if bytes.get(at) != Some(&b'"') {
        return Err(DecodeError::new("", at, "expected a JSON string"));
    }
    meter.tokens(1);
    let mut cursor = at + 1;
    loop {
        match bytes.get(cursor) {
            None => return Err(DecodeError::new("", at, "unterminated JSON string")),
            Some(b'"') => return Ok((Span::new(at + 1, cursor), cursor + 1)),
            Some(b'\\') => cursor += 2,
            Some(_) => cursor += 1,
        }
    }
}

/// Step over one JSON value without materialising it. Returns the span of the
/// whole value and the offset just past it. Counts the tokens it had to look at
/// to find the end: a scalar is one, an object or array is its own braces plus
/// whatever is nested inside it.
pub fn skip_value(bytes: &[u8], at: usize, meter: &Meter) -> Result<(Span, usize), DecodeError> {
    let start = skip_ws(bytes, at);
    match bytes.get(start) {
        Some(b'"') => {
            let (_, next) = scan_string(bytes, start, meter)?;
            Ok((Span::new(start, next), next))
        }
        Some(b'{') => {
            meter.tokens(1);
            let mut cursor = skip_ws(bytes, start + 1);
            if bytes.get(cursor) == Some(&b'}') {
                meter.tokens(1);
                return Ok((Span::new(start, cursor + 1), cursor + 1));
            }
            loop {
                cursor = skip_ws(bytes, cursor);
                let (_, next) = scan_string(bytes, cursor, meter)?;
                cursor = skip_ws(bytes, next);
                if bytes.get(cursor) != Some(&b':') {
                    return Err(DecodeError::new("", cursor, "expected ':'"));
                }
                let (_, next) = skip_value(bytes, cursor + 1, meter)?;
                cursor = skip_ws(bytes, next);
                match bytes.get(cursor) {
                    Some(b',') => cursor += 1,
                    Some(b'}') => {
                        meter.tokens(1);
                        return Ok((Span::new(start, cursor + 1), cursor + 1));
                    }
                    _ => return Err(DecodeError::new("", cursor, "expected ',' or '}'")),
                }
            }
        }
        Some(b'[') => {
            meter.tokens(1);
            let mut cursor = skip_ws(bytes, start + 1);
            if bytes.get(cursor) == Some(&b']') {
                meter.tokens(1);
                return Ok((Span::new(start, cursor + 1), cursor + 1));
            }
            loop {
                let (_, next) = skip_value(bytes, cursor, meter)?;
                cursor = skip_ws(bytes, next);
                match bytes.get(cursor) {
                    Some(b',') => cursor += 1,
                    Some(b']') => {
                        meter.tokens(1);
                        return Ok((Span::new(start, cursor + 1), cursor + 1));
                    }
                    _ => return Err(DecodeError::new("", cursor, "expected ',' or ']'")),
                }
            }
        }
        Some(_) => {
            // `true`, `false`, `null` or a number: one token, ended by the
            // first structural character.
            meter.tokens(1);
            let mut cursor = start;
            while matches!(bytes.get(cursor), Some(c)
                if !matches!(c, b',' | b'}' | b']' | b' ' | b'\t' | b'\r' | b'\n'))
            {
                cursor += 1;
            }
            if cursor == start {
                return Err(DecodeError::new("", start, "expected a JSON value"));
            }
            Ok((Span::new(start, cursor), cursor))
        }
        None => Err(DecodeError::new("", start, "unexpected end of input")),
    }
}

/// Whether the span of a JSON value is the literal `null`.
pub fn is_null(bytes: &[u8], span: Span) -> bool {
    span.of(bytes) == b"null"
}

/// Decode the text inside a JSON string's quotes. Only called when a value is
/// actually materialised, which is what keeps `model.lazy.*` honest.
pub fn unescape(raw: &[u8]) -> Result<String, DecodeError> {
    if !raw.contains(&b'\\') {
        return std::str::from_utf8(raw)
            .map(str::to_owned)
            .map_err(|e| DecodeError::new("", 0, format!("invalid UTF-8: {e}")));
    }
    let text = std::str::from_utf8(raw)
        .map_err(|e| DecodeError::new("", 0, format!("invalid UTF-8: {e}")))?;
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('"') => out.push('"'),
            Some('\\') => out.push('\\'),
            Some('/') => out.push('/'),
            Some('b') => out.push('\u{8}'),
            Some('f') => out.push('\u{c}'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('u') => {
                let hex: String = chars.by_ref().take(4).collect();
                let code = u32::from_str_radix(&hex, 16)
                    .map_err(|_| DecodeError::new("", 0, format!("bad \\u escape: {hex}")))?;
                if (0xD800..0xDC00).contains(&code) {
                    // A high surrogate: the low surrogate follows as a second
                    // \u escape.
                    let mut low = String::new();
                    if chars.next() == Some('\\') && chars.next() == Some('u') {
                        low = chars.by_ref().take(4).collect();
                    }
                    let low_code = u32::from_str_radix(&low, 16)
                        .map_err(|_| DecodeError::new("", 0, "unpaired high surrogate"))?;
                    let combined =
                        0x1_0000 + ((code - 0xD800) << 10) + (low_code.wrapping_sub(0xDC00));
                    out.push(
                        char::from_u32(combined)
                            .ok_or_else(|| DecodeError::new("", 0, "bad surrogate pair"))?,
                    );
                } else {
                    out.push(
                        char::from_u32(code)
                            .ok_or_else(|| DecodeError::new("", 0, "bad \\u escape"))?,
                    );
                }
            }
            other => {
                return Err(DecodeError::new(
                    "",
                    0,
                    format!("unknown JSON escape: {other:?}"),
                ));
            }
        }
    }
    Ok(out)
}

/// Encode `text` as a JSON string, quotes included.
pub fn escape_into(text: &str, out: &mut Vec<u8>) {
    out.push(b'"');
    for c in text.chars() {
        match c {
            '"' => out.extend_from_slice(b"\\\""),
            '\\' => out.extend_from_slice(b"\\\\"),
            '\n' => out.extend_from_slice(b"\\n"),
            '\r' => out.extend_from_slice(b"\\r"),
            '\t' => out.extend_from_slice(b"\\t"),
            '\u{8}' => out.extend_from_slice(b"\\b"),
            '\u{c}' => out.extend_from_slice(b"\\f"),
            c if (c as u32) < 0x20 => {
                out.extend_from_slice(format!("\\u{:04x}", c as u32).as_bytes());
            }
            c => {
                let mut buffer = [0u8; 4];
                out.extend_from_slice(c.encode_utf8(&mut buffer).as_bytes());
            }
        }
    }
    out.push(b'"');
}
