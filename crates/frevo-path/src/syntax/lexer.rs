//! The token layer.
//!
//! Derived from org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/fhirpath/FHIRLexer.java
//! at 73c432b5f43ddbce979978a760ec4aa81e929094.
//! Original work Copyright HL7, Inc. and contributors, licensed under the
//! Apache License, Version 2.0.
//! Modified by the Frevo authors: tokens are byte spans into the expression
//! rather than fresh `String`s, positions are byte offsets, with HAPI's own
//! line/column count kept beside them for the error suffix
//! (`currentLocation`, `path.errors.002`), and the four places where upstream raises an unchecked
//! Java exception on a truncated input return a `ParseError` instead
//! (`path.errors.003` forbids a panic; `DEV-0011`).
//!
//! What is **not** changed, deliberately, is upstream's behaviour on input it
//! accepts. Three upstream quirks are reproduced rather than corrected,
//! because `FHIRLexer`'s observable behaviour is what this port is measured
//! against:
//!
//! 1. an unfinished block comment is **not** an error. `skipWhitespaceAndComments`
//!    is declared without `throws`, so `FHIRLexer.java:315` builds the
//!    "Unfinished comment" exception and drops it; the scan then stops one
//!    character before the end of the source and that last character becomes
//!    the next token. `2 + 2 /* not finished` therefore fails as
//!    "Premature ExpressionNode termination at unexpected token \"d\"", which
//!    is still the syntax error `path.lexer.009` asks for, and
//!    `2 + /* c` parses as `2 + c`;
//! 2. [`Lexer::is_token`] inspects `charAt(1)` on every iteration of its loop
//!    rather than `charAt(i)` (`FHIRLexer.java:126-129`), so a name whose
//!    second character is `_` is not a token name;
//! 3. the two escape decoders differ: the one for string constants knows ten
//!    escapes and reads `\u` in base 16, while the one for backtick-delimited
//!    names knows nine (no `` \` ``) and reads `\u` in base **32**
//!    (`FHIRLexer.java:511`).

use super::ast::SourceLocation;
use super::error::ParseError;

/// A token: a byte range in the expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Span {
    pub start: usize,
    pub end: usize,
}

/// `Utilities.isWhitespace` (`Utilities.java:1825`).
fn is_whitespace(ch: char) -> bool {
    let c = ch as u32;
    if c < 0x80 {
        return c == 0x0020 || (0x0009..=0x000D).contains(&c);
    }
    matches!(
        c,
        0x0085 | 0x00A0 | 0x1680 | 0x2000..=0x200A | 0x2028 | 0x2029 | 0x202F | 0x205F | 0x3000
    )
}

fn is_ascii_letter(ch: char) -> bool {
    ch.is_ascii_uppercase() || ch.is_ascii_lowercase()
}

/// The lexer, positioned on one token at a time, exactly as upstream's is.
///
/// `source` has already had any byte-order marks removed by
/// [`super::source::Source`], which also holds the mapping back to the
/// caller's offsets; everything here is an offset into the stripped text.
pub(crate) struct Lexer<'a> {
    source: &'a str,
    cursor: usize,
    current_start: usize,
    current: Option<Span>,
    /// Where the token before the current one ended, which is what upstream's
    /// `getCurrentLocation()` amounts to once the whitespace it never counted
    /// is discounted. It is the end offset a node records.
    previous_end: usize,
    allow_double_quotes: bool,
    /// `FHIRLexer.currentLocation`: HAPI's line and column, advanced by
    /// HAPI's own rule (`path.errors.002`). It is not derived from the byte
    /// offset: a whitespace character advances it by two and a token by its
    /// length once (`FHIRLexer.java:285`, `:321-324`).
    location: SourceLocation,
    /// `FHIRLexer.currentStartLocation`: `location` as it was when the current
    /// token started (`FHIRLexer.java:147`).
    start_location: SourceLocation,
}

impl<'a> Lexer<'a> {
    /// `new FHIRLexer(source, name, false, allowDoubleQuotes)`
    /// (`FHIRLexer.java:73`), which reads the first token in its constructor.
    pub(crate) fn new(source: &'a str, allow_double_quotes: bool) -> Result<Self, ParseError> {
        let mut lexer = Lexer {
            source,
            cursor: 0,
            current_start: 0,
            current: None,
            previous_end: 0,
            allow_double_quotes,
            location: SourceLocation::START,
            start_location: SourceLocation::START,
        };
        lexer.next()?;
        Ok(lexer)
    }

    /// The current token's text, or `None` at the end of the source.
    pub(crate) fn current(&self) -> Option<&'a str> {
        self.current.map(|s| &self.source[s.start..s.end])
    }

    /// The current token's text, or `""` at the end of the source. Upstream
    /// interpolates a null `current` into its messages as the empty string, so
    /// "Found  expecting ..." is reproduced rather than reworded.
    pub(crate) fn current_or_empty(&self) -> &'a str {
        self.current().unwrap_or("")
    }

    /// Where the current token starts: the offset a diagnostic reports.
    pub(crate) fn current_start(&self) -> usize {
        self.current_start
    }

    /// `getCurrentStartLocation()`: HAPI's line and column of the current
    /// token's first character.
    pub(crate) fn current_start_location(&self) -> SourceLocation {
        self.start_location
    }

    /// `getCurrentLocation()`: HAPI's line and column just past the current
    /// token.
    pub(crate) fn current_location(&self) -> SourceLocation {
        self.location
    }

    /// One past the current token's last byte, or the end of the source when
    /// there is no current token.
    pub(crate) fn current_end(&self) -> usize {
        self.current.map_or(self.source.len(), |s| s.end)
    }

    /// One past the last byte of the token before the current one.
    pub(crate) fn previous_end(&self) -> usize {
        self.previous_end
    }

    /// `FHIRLexer.done()`: `currentStart >= source.length()`.
    pub(crate) fn done(&self) -> bool {
        self.current_start >= self.source.len()
    }

    /// `FHIRPathConstant.isFHIRPathConstant`.
    pub(crate) fn is_constant(&self) -> bool {
        match self.current() {
            None => false,
            Some(s) => match s.chars().next() {
                None => false,
                Some(c) => {
                    matches!(c, '\'' | '"' | '@' | '%' | '-' | '+' | '0'..='9')
                        || s == "true"
                        || s == "false"
                        || s == "{}"
                }
            },
        }
    }

    /// `FHIRPathConstant.isFHIRPathFixedName`: a backtick-delimited name.
    pub(crate) fn is_fixed_name(&self) -> bool {
        self.current().is_some_and(|s| s.starts_with('`'))
    }

    /// `FHIRPathConstant.isFHIRPathStringConstant`.
    pub(crate) fn is_string_constant(&self) -> bool {
        self.current()
            .is_some_and(|s| s.starts_with('\'') || s.starts_with('"') || s.starts_with('`'))
    }

    /// `FHIRLexer.isToken` (`FHIRLexer.java:115`), bug included: the loop
    /// tests `charAt(1)` on every pass instead of `charAt(i)`, so a name whose
    /// second character is not alphanumeric — `a_b` — is not a token name
    /// however long it is.
    pub(crate) fn is_token(&self) -> bool {
        let Some(s) = self.current() else {
            return false;
        };
        if s.is_empty() {
            return false;
        }
        if s.starts_with('$') {
            return true;
        }
        if s == "*" || s == "**" {
            return true;
        }
        let mut chars = s.chars();
        let first = chars.next().expect("not empty");
        if !is_ascii_letter(first) {
            return false;
        }
        let second = chars.next();
        for _ in 1..s.chars().count() {
            match second {
                Some(c) if is_ascii_letter(c) || c.is_ascii_digit() => {}
                _ => return false,
            }
        }
        true
    }

    /// `FHIRLexer.isOp`: the current token is an operator.
    pub(crate) fn is_op(&self) -> bool {
        !self.done()
            && self
                .current()
                .and_then(super::ast::Operation::from_code)
                .is_some()
    }

    /// `FHIRLexer.hasToken(String...)`.
    pub(crate) fn has_token_any(&self, names: &[&str]) -> bool {
        if self.done() {
            return false;
        }
        self.current().is_some_and(|c| names.contains(&c))
    }

    /// `FHIRLexer.take()`: the current token, then advance.
    pub(crate) fn take(&mut self) -> Result<&'a str, ParseError> {
        let s = self.current_or_empty();
        self.next()?;
        Ok(s)
    }

    /// `FHIRLexer.setCurrent(current.substring(1))`
    /// (`FHIRPathEngine.java:1101`): the parser peels a leading `-` or `+` off
    /// a token it has decided is a unary operator.
    pub(crate) fn drop_first_char_of_current(&mut self) {
        if let Some(span) = self.current {
            let text = &self.source[span.start..span.end];
            if let Some(c) = text.chars().next() {
                self.current = Some(Span {
                    start: span.start + c.len_utf8(),
                    end: span.end,
                });
            }
        }
    }

    /// `FHIRLexer.error(msg)`, reported at the current token's offset.
    pub(crate) fn error(&self, message: impl Into<String>) -> ParseError {
        ParseError::new(message, self.error_offset())
    }

    /// The offset a diagnostic about the current token reports: the token's
    /// first byte, or the end of the source when there is no current token.
    pub(crate) fn error_offset(&self) -> usize {
        self.current_start.min(self.source.len())
    }

    /// `FHIRLexer.readConstant(desc)`: a quoted run, decoded with the
    /// **lexer's** escape table. The only caller in FHIRPath is the unit of a
    /// quantity literal (`FHIRPathEngine.java:1130`).
    pub(crate) fn read_constant(&mut self, desc: &str) -> Result<String, ParseError> {
        if !self.is_string_constant() {
            return Err(self.error(format!(
                "Found {} expecting \"[{desc}]\"",
                self.current_or_empty()
            )));
        }
        let at = self.current_start;
        let text = self.take()?;
        process_constant(text, at)
    }

    /// `FHIRLexer.readFixedName(desc)`: a backtick-delimited name, decoded
    /// with the **fixed-name** escape table.
    pub(crate) fn read_fixed_name(&mut self, desc: &str) -> Result<String, ParseError> {
        if !self.is_fixed_name() {
            return Err(self.error(format!(
                "Found {} expecting \"[{desc}]\"",
                self.current_or_empty()
            )));
        }
        let at = self.current_start;
        let text = self.take()?;
        process_fixed_name(text, at)
    }

    // -- scanning ---------------------------------------------------------

    fn char_at(&self, at: usize) -> Option<char> {
        if at >= self.source.len() {
            return None;
        }
        self.source[at..].chars().next()
    }

    /// Advance the cursor by one character.
    fn bump(&mut self) {
        if let Some(c) = self.char_at(self.cursor) {
            self.cursor += c.len_utf8();
        } else {
            self.cursor = self.source.len();
        }
    }

    fn starts_with_at(&self, at: usize, pat: &str) -> bool {
        self.source[at.min(self.source.len())..].starts_with(pat)
    }

    /// `FHIRLexer.next()`: skip whitespace and comments, then read one token.
    pub(crate) fn next(&mut self) -> Result<(), ParseError> {
        self.previous_end = self.cursor;
        self.skip_whitespace_and_comments();
        self.current = None;
        self.current_start = self.cursor;
        self.start_location = self.location;
        let Some(ch) = self.char_at(self.cursor) else {
            return Ok(());
        };
        match ch {
            '!' | '>' | '<' | ':' | '-' | '=' => {
                self.bump();
                // `FHIRLexer.java:152`. Upstream evaluates
                // `source.charAt(cursor) == '>'` without a bounds check when
                // `ch == '-'`, so `2 -` raises StringIndexOutOfBoundsException
                // there; end of source is treated as "no second character"
                // here (`DEV-0011`).
                let second = self.char_at(self.cursor);
                let takes_second = match second {
                    Some('=') | Some('~') | Some('-') => true,
                    Some('>') => ch == '-',
                    _ => false,
                };
                if takes_second {
                    self.bump();
                }
            }
            '.' => {
                self.bump();
                if self.char_at(self.cursor) == Some('.') {
                    self.bump();
                }
            }
            '0'..='9' => {
                self.bump();
                let mut dotted = false;
                while let Some(c) = self.char_at(self.cursor) {
                    if c.is_ascii_digit() {
                        self.bump();
                    } else if c == '.' && !dotted {
                        dotted = true;
                        self.bump();
                    } else {
                        break;
                    }
                }
                if self.source[..self.cursor].ends_with('.') {
                    self.cursor -= 1;
                }
            }
            c if is_ascii_letter(c) => {
                while let Some(c) = self.char_at(self.cursor) {
                    if is_ascii_letter(c) || c.is_ascii_digit() || c == '_' {
                        self.bump();
                    } else {
                        break;
                    }
                }
            }
            '%' => {
                self.bump();
                if self.char_at(self.cursor) == Some('`') {
                    self.bump();
                    while let Some(c) = self.char_at(self.cursor) {
                        if c == '`' {
                            break;
                        }
                        self.bump();
                    }
                    // Upstream steps over the closing backtick
                    // unconditionally, which runs one past the end when the
                    // name is unterminated; the step is bounded here.
                    self.bump();
                } else {
                    while let Some(c) = self.char_at(self.cursor) {
                        if is_ascii_letter(c)
                            || c.is_ascii_digit()
                            || c == ':'
                            || c == '-'
                            || c == '_'
                        {
                            self.bump();
                        } else {
                            break;
                        }
                    }
                }
            }
            '/' => {
                self.bump();
                if self.char_at(self.cursor) == Some('/') {
                    // Metadata (`///`), which FHIRPath never reaches because
                    // `metadataFormat` is false and `skipWhitespaceAndComments`
                    // has already consumed any `//`. Upstream advances twice
                    // more here (`FHIRLexer.java:192-193`).
                    self.bump();
                    self.bump();
                }
            }
            '$' => {
                self.bump();
                while let Some(c) = self.char_at(self.cursor) {
                    if c.is_ascii_lowercase() {
                        self.bump();
                    } else {
                        break;
                    }
                }
            }
            '{' => {
                self.bump();
                // `FHIRLexer.java:205` reads the next character without a
                // bounds check, so a trailing `{` raises
                // StringIndexOutOfBoundsException upstream (`DEV-0011`).
                if self.char_at(self.cursor) == Some('}') {
                    self.bump();
                }
            }
            '"' if self.allow_double_quotes => {
                self.scan_delimited('"')?;
            }
            '`' => {
                self.scan_delimited('`')?;
            }
            '\'' => {
                self.scan_delimited('\'')?;
            }
            '@' => {
                let start = self.cursor;
                self.bump();
                while let Some(c) = self.char_at(self.cursor) {
                    if self.is_date_char(c, start) {
                        self.bump();
                    } else {
                        break;
                    }
                }
            }
            _ => {
                self.bump();
            }
        }
        self.current = Some(Span {
            start: self.current_start,
            end: self.cursor,
        });
        // `currentLocation.incColumn(cursor - currentStart)`: the token's
        // length in UTF-16 units, as Java's string offsets count it.
        let length = utf16_len(&self.source[self.current_start..self.cursor]);
        self.location.inc_column(length);
        Ok(())
    }

    /// The `'`, `` ` `` and `"` arms, which share their shape: scan to the
    /// matching delimiter, tracking backslash escapes, and raise
    /// "Unterminated string" if the source runs out first
    /// (`FHIRLexer.java:209-253`).
    fn scan_delimited(&mut self, delimiter: char) -> Result<(), ParseError> {
        self.bump();
        let mut escape = false;
        while let Some(c) = self.char_at(self.cursor) {
            if !escape && c == delimiter {
                break;
            }
            escape = if escape { false } else { c == '\\' };
            self.bump();
        }
        if self.cursor >= self.source.len() {
            return Err(ParseError::new("Unterminated string", self.current_start));
        }
        self.bump();
        Ok(())
    }

    /// `FHIRLexer.isDateChar` (`FHIRLexer.java:335`). `start` is the offset of
    /// the `@`; `eot` is where a fractional second may appear, and it depends
    /// on whether the literal is a time (`@T...`) or a date.
    fn is_date_char(&self, ch: char, start: usize) -> bool {
        let eot = if self.char_at(start + 1) == Some('T') {
            10
        } else {
            20
        };
        ch == '-'
            || ch == ':'
            || ch == 'T'
            || ch == '+'
            || ch == 'Z'
            || ch.is_ascii_digit()
            || (self.cursor - start == eot
                && ch == '.'
                && self
                    .char_at(self.cursor + 1)
                    .is_some_and(|c| c.is_ascii_digit()))
    }

    /// `FHIRLexer.skipWhitespaceAndComments` (`FHIRLexer.java:288`).
    ///
    /// The comment text itself is not retained: upstream collects it for the
    /// mapping language's metadata, which FHIRPath never reads.
    fn skip_whitespace_and_comments(&mut self) {
        let len = self.source.len();
        // `last13`: whether the character before was a CR, so that a CRLF
        // starts one new line and not two (`SourceLocation.checkChar`).
        let mut last13 = false;
        while self.cursor < len {
            if self.cursor + 1 < len && self.starts_with_at(self.cursor, "//") {
                while let Some(c) = self.char_at(self.cursor) {
                    if c == '\r' || c == '\n' {
                        break;
                    }
                    // Java steps once per UTF-16 unit, one column each.
                    self.cursor += c.len_utf8();
                    self.location.inc_column(utf16_len_char(c));
                }
            } else if self.cursor + 1 < len && self.starts_with_at(self.cursor, "/*") {
                while self.cursor + 1 < len && !self.starts_with_at(self.cursor, "*/") {
                    let Some(c) = self.char_at(self.cursor) else {
                        break;
                    };
                    // `checkChar` then `incColumn()`, once per UTF-16 unit.
                    for _ in 0..c.len_utf16() {
                        last13 = self.location.check_char(c, last13);
                        self.location.inc_column(1);
                    }
                    self.cursor += c.len_utf8();
                }
                if self.cursor + 1 >= len {
                    // "Unfinished comment": upstream builds the exception and
                    // discards it (`FHIRLexer.java:314-315`), leaving the
                    // cursor one character short of the end and falling back
                    // into this loop — which then consumes that last character
                    // if it is whitespace and stops on it if it is not.
                    // Reproduced; see this module's header.
                    continue;
                }
                self.bump();
                self.bump();
                self.location.inc_column(2);
            } else if let Some(c) = self.char_at(self.cursor).filter(|c| is_whitespace(*c)) {
                // Every whitespace character is in the Basic Multilingual
                // Plane, so this is one UTF-16 unit.
                last13 = self.location.check_char(c, last13);
                self.cursor += c.len_utf8();
                self.location.inc_column(1);
            } else {
                return;
            }
        }
    }
}

/// The length of `s` in UTF-16 code units, which is what Java's string
/// offsets, and so HAPI's columns, count.
fn utf16_len(s: &str) -> u32 {
    u32::try_from(s.encode_utf16().count()).unwrap_or(u32::MAX)
}

fn utf16_len_char(c: char) -> u32 {
    if c.len_utf16() == 2 { 2 } else { 1 }
}

/// `FHIRLexer.processConstant(String)` (`FHIRLexer.java:425`): the escape
/// table used for a quantity literal's unit.
///
/// Two upstream quirks are reproduced. The escape arms other than `\u` do not
/// advance past the escaped character, so the character is appended a second
/// time as itself — `'a\tb'` decodes to `a<TAB>tb` — and a `\\` therefore
/// starts a fresh escape, so `'a\\b'` is rejected as an unknown escape `\b`.
/// `at` is the offset of the opening quote, for the diagnostic.
pub(crate) fn process_constant(s: &str, at: usize) -> Result<String, ParseError> {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 1usize;
    while i + 1 < chars.len() {
        let ch = chars[i];
        if ch == '\\' {
            i += 1;
            let escape = *chars.get(i).ok_or_else(|| {
                ParseError::new("Unknown FHIRPath character escape at end of string", at)
            })?;
            match escape {
                't' => out.push('\t'),
                'r' => out.push('\r'),
                'n' => out.push('\n'),
                'f' => out.push('\u{000c}'),
                '\'' => out.push('\''),
                '"' => out.push('"'),
                '`' => out.push('`'),
                '\\' => out.push('\\'),
                '/' => out.push('/'),
                'u' => {
                    i += 1;
                    out.push(unicode_escape(&chars, i, 16, at)?);
                    i += 4;
                }
                other => {
                    return Err(ParseError::new(
                        format!("Unknown FHIRPath character escape \\{other}"),
                        at,
                    ));
                }
            }
        } else {
            out.push(ch);
            i += 1;
        }
    }
    Ok(out)
}

/// `FHIRLexer.processFixedName(String)` (`FHIRLexer.java:477`): the escape
/// table for a backtick-delimited name. It knows no `` \` `` escape and reads
/// `\u` in **base 32**, both of which are upstream's and both reproduced.
pub(crate) fn process_fixed_name(s: &str, at: usize) -> Result<String, ParseError> {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 1usize;
    while i + 1 < chars.len() {
        let ch = chars[i];
        if ch == '\\' {
            i += 1;
            let escape = *chars.get(i).ok_or_else(|| {
                ParseError::new("Unknown FHIRPath character escape at end of name", at)
            })?;
            match escape {
                't' => out.push('\t'),
                'r' => out.push('\r'),
                'n' => out.push('\n'),
                'f' => out.push('\u{000c}'),
                '\'' => out.push('\''),
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                '/' => out.push('/'),
                'u' => {
                    i += 1;
                    out.push(unicode_escape(&chars, i, 32, at)?);
                    i += 4;
                }
                other => {
                    return Err(ParseError::new(
                        format!("Unknown FHIRPath character escape \\{other}"),
                        at,
                    ));
                }
            }
            // Upstream's `processFixedName` has the same missing advance as
            // `processConstant`; the escaped character is re-read as itself.
        } else {
            out.push(ch);
            i += 1;
        }
    }
    Ok(out)
}

/// The engine's own string-constant decoder,
/// `FHIRPathEngine.processConstantString` (`FHIRPathEngine.java:1789`). It is
/// the one a string **literal** goes through, and unlike the lexer's two it
/// advances correctly past the escaped character, so `'a\tb'` is `a<TAB>b`.
///
/// `at` is the offset of the opening quote: a diagnostic about an escape
/// reports the offset of the escape itself, which is `at` plus the position
/// within the literal.
pub(crate) fn process_constant_string(s: &str, at: usize) -> Result<String, ParseError> {
    let chars: Vec<char> = s.chars().collect();
    // Byte offset of each character, for a diagnostic that names a position.
    let mut offsets = Vec::with_capacity(chars.len() + 1);
    let mut acc = at;
    for c in &chars {
        offsets.push(acc);
        acc += c.len_utf8();
    }
    offsets.push(acc);

    let mut out = String::new();
    let mut i = 1usize;
    while i + 1 < chars.len() {
        let ch = chars[i];
        if ch == '\\' {
            let escape_at = offsets[i];
            i += 1;
            let escape = *chars.get(i).ok_or_else(|| {
                ParseError::new(
                    "Unknown FHIRPath character escape at end of string",
                    escape_at,
                )
            })?;
            match escape {
                't' => out.push('\t'),
                'r' => out.push('\r'),
                'n' => out.push('\n'),
                'f' => out.push('\u{000c}'),
                '\'' => out.push('\''),
                '"' => out.push('"'),
                '`' => out.push('`'),
                '\\' => out.push('\\'),
                '/' => out.push('/'),
                'u' => {
                    i += 1;
                    out.push(unicode_escape(&chars, i, 16, escape_at)?);
                    i += 3;
                }
                other => {
                    return Err(ParseError::new(
                        format!("Unknown FHIRPath character escape \\{other}"),
                        escape_at,
                    ));
                }
            }
            i += 1;
        } else {
            out.push(ch);
            i += 1;
        }
    }
    Ok(out)
}

/// The four characters after `\u`, as a character.
///
/// Upstream is `Integer.parseInt(s.substring(i, i + 4), radix)` with no bounds
/// check and with `parseInt`'s tolerance of a leading sign, so `'\u00'` raises
/// StringIndexOutOfBoundsException, `'\u00zz'` raises NumberFormatException,
/// and `'\u+041'` decodes to `A`. All three are refused here with a
/// diagnostic: exactly four digits in the radix, and no sign
/// (`path.lexer.013`, `DEV-0011`).
fn unicode_escape(chars: &[char], i: usize, radix: u32, at: usize) -> Result<char, ParseError> {
    if i + 4 > chars.len() {
        return Err(ParseError::new(
            format!(
                "Unknown FHIRPath character escape \\u: a unicode escape needs four base-{radix} digits"
            ),
            at,
        ));
    }
    let mut value: u32 = 0;
    for k in 0..4 {
        let digit = chars[i + k].to_digit(radix).ok_or_else(|| {
            ParseError::new(
                format!(
                    "Unknown FHIRPath character escape \\u{}{}{}{}: {:?} is not a base-{radix} digit",
                    chars[i],
                    chars[i + 1],
                    chars[i + 2],
                    chars[i + 3],
                    chars[i + k]
                ),
                at,
            )
        })?;
        value = value * radix + digit;
    }
    char::from_u32(value).ok_or_else(|| {
        ParseError::new(
            format!("The unicode escape \\u in this literal is not a character: {value:#x}"),
            at,
        )
    })
}
