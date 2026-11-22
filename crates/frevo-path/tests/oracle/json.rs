//! A minimal JSON reader for the oracle harness's input files.
//!
//! Test-only. `serde_json` is in the workspace, but it is not used here for one
//! reason that matters to this project: it parses a number into `f64` or into
//! an arbitrary-precision type behind a feature, and `model.json.010` forbids a
//! decimal ever travelling through a binary float. The harness keeps every
//! number as the exact text the file carried, so an input file can never lose
//! precision on the way into a case.
//!
//! Object member order is preserved for the same reason: the suite's inputs are
//! navigated in document order (`model.nav.002`).

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    /// The number's text exactly as written. Never parsed into a float.
    Number(String),
    String(String),
    Array(Vec<Value>),
    /// Members in document order. Duplicate keys are preserved as written, so
    /// the harness can see them rather than silently collapsing them.
    Object(Vec<(String, Value)>),
}

impl Value {
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(m) => m.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonError {
    pub offset: usize,
    pub message: String,
}

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JSON error at byte {}: {}", self.offset, self.message)
    }
}

pub fn parse(src: &str) -> Result<Value, JsonError> {
    let mut p = Parser {
        b: src.as_bytes(),
        src,
        i: 0,
        depth: 0,
    };
    p.ws();
    let v = p.value()?;
    p.ws();
    if p.i != p.b.len() {
        return p.err("trailing content after the top-level value");
    }
    Ok(v)
}

/// A depth limit rather than a stack overflow, the same discipline
/// `model.errors.005` requires of the product decoder.
const MAX_DEPTH: usize = 256;

struct Parser<'a> {
    b: &'a [u8],
    src: &'a str,
    i: usize,
    depth: usize,
}

impl<'a> Parser<'a> {
    fn err<T>(&self, message: impl Into<String>) -> Result<T, JsonError> {
        Err(JsonError {
            offset: self.i,
            message: message.into(),
        })
    }

    fn ws(&mut self) {
        while matches!(self.b.get(self.i), Some(b' ' | b'\t' | b'\r' | b'\n')) {
            self.i += 1;
        }
    }

    fn value(&mut self) -> Result<Value, JsonError> {
        if self.depth >= MAX_DEPTH {
            return self.err("nesting deeper than the harness depth limit");
        }
        match self.b.get(self.i) {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => Ok(Value::String(self.string()?)),
            Some(b't') => self.literal("true", Value::Bool(true)),
            Some(b'f') => self.literal("false", Value::Bool(false)),
            Some(b'n') => self.literal("null", Value::Null),
            Some(c) if *c == b'-' || c.is_ascii_digit() => self.number(),
            Some(_) => self.err("expected a JSON value"),
            None => self.err("unexpected end of input"),
        }
    }

    fn literal(&mut self, text: &str, v: Value) -> Result<Value, JsonError> {
        if self.b[self.i..].starts_with(text.as_bytes()) {
            self.i += text.len();
            Ok(v)
        } else {
            self.err(format!("expected '{text}'"))
        }
    }

    fn number(&mut self) -> Result<Value, JsonError> {
        let start = self.i;
        if self.b.get(self.i) == Some(&b'-') {
            self.i += 1;
        }
        let digits_start = self.i;
        while self.b.get(self.i).is_some_and(u8::is_ascii_digit) {
            self.i += 1;
        }
        if self.i == digits_start {
            return self.err("expected a digit");
        }
        if self.b.get(self.i) == Some(&b'.') {
            self.i += 1;
            let frac = self.i;
            while self.b.get(self.i).is_some_and(u8::is_ascii_digit) {
                self.i += 1;
            }
            if self.i == frac {
                return self.err("expected a digit after '.'");
            }
        }
        if matches!(self.b.get(self.i), Some(b'e' | b'E')) {
            self.i += 1;
            if matches!(self.b.get(self.i), Some(b'+' | b'-')) {
                self.i += 1;
            }
            let exp = self.i;
            while self.b.get(self.i).is_some_and(u8::is_ascii_digit) {
                self.i += 1;
            }
            if self.i == exp {
                return self.err("expected a digit in the exponent");
            }
        }
        Ok(Value::Number(self.src[start..self.i].to_string()))
    }

    fn string(&mut self) -> Result<String, JsonError> {
        self.i += 1; // opening quote
        let mut out = String::new();
        loop {
            match self.b.get(self.i) {
                None => return self.err("unterminated string"),
                Some(b'"') => {
                    self.i += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.i += 1;
                    let c = match self.b.get(self.i) {
                        Some(b'"') => '"',
                        Some(b'\\') => '\\',
                        Some(b'/') => '/',
                        Some(b'b') => '\u{8}',
                        Some(b'f') => '\u{c}',
                        Some(b'n') => '\n',
                        Some(b'r') => '\r',
                        Some(b't') => '\t',
                        Some(b'u') => {
                            let hex_at = self.i + 1;
                            if hex_at + 4 > self.src.len() {
                                return self.err("truncated \\u escape");
                            }
                            let hi = u32::from_str_radix(&self.src[hex_at..hex_at + 4], 16)
                                .map_err(|_| JsonError {
                                    offset: hex_at,
                                    message: "invalid \\u escape".into(),
                                })?;
                            self.i = hex_at + 3;
                            if (0xD800..0xDC00).contains(&hi) {
                                // Surrogate pair.
                                if self.src[self.i + 1..].starts_with("\\u") {
                                    let lo_at = self.i + 3;
                                    let lo = u32::from_str_radix(&self.src[lo_at..lo_at + 4], 16)
                                        .map_err(|_| JsonError {
                                        offset: lo_at,
                                        message: "invalid low surrogate".into(),
                                    })?;
                                    self.i = lo_at + 3;
                                    let cp = 0x10000 + ((hi - 0xD800) << 10) + (lo & 0x3FF);
                                    match char::from_u32(cp) {
                                        Some(c) => c,
                                        None => return self.err("invalid surrogate pair"),
                                    }
                                } else {
                                    return self.err("lone high surrogate");
                                }
                            } else {
                                match char::from_u32(hi) {
                                    Some(c) => c,
                                    None => return self.err("invalid code point"),
                                }
                            }
                        }
                        _ => return self.err("unknown escape sequence"),
                    };
                    out.push(c);
                    self.i += 1;
                }
                Some(_) => {
                    let start = self.i;
                    while self
                        .b
                        .get(self.i)
                        .is_some_and(|c| *c != b'"' && *c != b'\\')
                    {
                        self.i += 1;
                    }
                    out.push_str(&self.src[start..self.i]);
                }
            }
        }
    }

    fn array(&mut self) -> Result<Value, JsonError> {
        self.i += 1;
        self.depth += 1;
        let mut items = Vec::new();
        self.ws();
        if self.b.get(self.i) == Some(&b']') {
            self.i += 1;
            self.depth -= 1;
            return Ok(Value::Array(items));
        }
        loop {
            self.ws();
            items.push(self.value()?);
            self.ws();
            match self.b.get(self.i) {
                Some(b',') => self.i += 1,
                Some(b']') => {
                    self.i += 1;
                    self.depth -= 1;
                    return Ok(Value::Array(items));
                }
                _ => return self.err("expected ',' or ']'"),
            }
        }
    }

    fn object(&mut self) -> Result<Value, JsonError> {
        self.i += 1;
        self.depth += 1;
        let mut members: Vec<(String, Value)> = Vec::new();
        self.ws();
        if self.b.get(self.i) == Some(&b'}') {
            self.i += 1;
            self.depth -= 1;
            return Ok(Value::Object(members));
        }
        loop {
            self.ws();
            if self.b.get(self.i) != Some(&b'"') {
                return self.err("expected a member name");
            }
            let key = self.string()?;
            self.ws();
            if self.b.get(self.i) != Some(&b':') {
                return self.err("expected ':'");
            }
            self.i += 1;
            self.ws();
            let v = self.value()?;
            members.push((key, v));
            self.ws();
            match self.b.get(self.i) {
                Some(b',') => self.i += 1,
                Some(b'}') => {
                    self.i += 1;
                    self.depth -= 1;
                    return Ok(Value::Object(members));
                }
                _ => return self.err("expected ',' or '}'"),
            }
        }
    }
}
