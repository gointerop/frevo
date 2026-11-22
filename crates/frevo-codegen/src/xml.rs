//! A pull reader for the subset of XML that FHIR definition bundles use.
//!
//! This exists for exactly one caller: the `reference/`-conditional cross-check
//! of `codegen.inputs.005`, which reads HAPI's
//! `profile/{profiles-types,profiles-resources,profiles-others}.xml`. The
//! authoritative input is JSON (`design.md` section 2), so XML is a
//! cross-check-only concern and adding an XML crate to the dependency graph for
//! it would need an ADR under the constitution's dependency rule.
//!
//! What it handles, because the pinned files use it: elements, attributes with
//! single or double quotes, the five predefined entities and numeric character
//! references, self-closing elements, comments, processing instructions, CDATA
//! sections and a `DOCTYPE` declaration. Namespace prefixes are stripped: FHIR
//! bundles put everything in one default namespace, and the cross-check
//! addresses elements by local name.
//!
//! What it deliberately does not handle: entity *declarations*, and therefore
//! custom entities. A file that declares one is rejected rather than read
//! wrongly — see [`Error::UnknownEntity`].

use std::fmt;

/// One parse event. Text nodes are not reported: FHIR XML carries every value
/// in a `value` attribute, and `Narrative.div` is the only text-bearing element
/// the cross-check does not read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// `<name a="1">`, and the opening half of `<name a="1"/>`.
    Start {
        name: String,
        attributes: Vec<(String, String)>,
    },
    /// `</name>`, and the closing half of `<name a="1"/>`.
    End { name: String },
}

impl Event {
    /// The element's local name, whichever kind of event this is.
    pub fn name(&self) -> &str {
        match self {
            Event::Start { name, .. } | Event::End { name } => name,
        }
    }

    /// One attribute's value, by local name.
    pub fn attribute(&self, key: &str) -> Option<&str> {
        match self {
            Event::Start { attributes, .. } => attributes
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.as_str()),
            Event::End { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// A construct ended without its terminator.
    Unterminated { at: usize, what: &'static str },
    /// `</b>` closing `<a>`.
    Mismatched {
        at: usize,
        expected: String,
        found: String,
    },
    /// A tag with no name, or an attribute with no `=`.
    Malformed { at: usize, what: &'static str },
    /// An entity reference the reader does not define. Silently passing it
    /// through would change the text being compared.
    UnknownEntity { at: usize, reference: String },
    /// The document ended with elements still open.
    UnclosedElements { open: Vec<String> },
    /// The bytes are not UTF-8.
    NotUtf8 { at: usize },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Unterminated { at, what } => write!(f, "unterminated {what} at byte {at}"),
            Error::Mismatched {
                at,
                expected,
                found,
            } => write!(f, "</{found}> closes <{expected}> at byte {at}"),
            Error::Malformed { at, what } => write!(f, "malformed {what} at byte {at}"),
            Error::UnknownEntity { at, reference } => {
                write!(f, "unknown entity reference {reference} at byte {at}")
            }
            Error::UnclosedElements { open } => {
                write!(f, "the document ends with {open:?} still open")
            }
            Error::NotUtf8 { at } => write!(f, "the bytes at {at} are not UTF-8"),
        }
    }
}

impl std::error::Error for Error {}

/// A pull reader over one document.
pub struct Reader<'a> {
    source: &'a [u8],
    at: usize,
    /// Set when a self-closing tag's `End` is still owed.
    pending_end: Option<String>,
    open: Vec<String>,
}

impl<'a> Reader<'a> {
    pub fn new(source: &'a [u8]) -> Self {
        Reader {
            source,
            at: 0,
            pending_end: None,
            open: Vec::new(),
        }
    }

    /// The next event, or `None` at the end of a well-formed document.
    pub fn next_event(&mut self) -> Result<Option<Event>, Error> {
        if let Some(name) = self.pending_end.take() {
            self.open.pop();
            return Ok(Some(Event::End { name }));
        }
        loop {
            // Skip to the next `<`.
            match self.source[self.at..].iter().position(|b| *b == b'<') {
                Some(offset) => self.at += offset,
                None => {
                    self.at = self.source.len();
                    if self.open.is_empty() {
                        return Ok(None);
                    }
                    return Err(Error::UnclosedElements {
                        open: self.open.clone(),
                    });
                }
            }
            let rest = &self.source[self.at..];
            if rest.starts_with(b"<!--") {
                self.skip_past(b"-->", "comment")?;
                continue;
            }
            if rest.starts_with(b"<![CDATA[") {
                self.skip_past(b"]]>", "CDATA section")?;
                continue;
            }
            if rest.starts_with(b"<?") {
                self.skip_past(b"?>", "processing instruction")?;
                continue;
            }
            if rest.starts_with(b"<!") {
                // A DOCTYPE. Internal subsets are not supported, and an
                // entity declaration inside one would be a silent change of
                // meaning, so reject the declaration rather than skip it.
                let end = self.find(b">", "declaration")?;
                let text = self.text(self.at..end)?;
                if text.contains("<!ENTITY") || text.contains('[') {
                    return Err(Error::Malformed {
                        at: self.at,
                        what: "DOCTYPE with an internal subset",
                    });
                }
                self.at = end + 1;
                continue;
            }
            if rest.starts_with(b"</") {
                return self.read_end_tag().map(Some);
            }
            return self.read_start_tag().map(Some);
        }
    }

    fn read_end_tag(&mut self) -> Result<Event, Error> {
        let start = self.at;
        let close = self.find(b">", "end tag")?;
        let name = local_name(self.text(start + 2..close)?.trim());
        if name.is_empty() {
            return Err(Error::Malformed {
                at: start,
                what: "end tag",
            });
        }
        self.at = close + 1;
        match self.open.pop() {
            Some(expected) if expected == name => Ok(Event::End { name }),
            Some(expected) => Err(Error::Mismatched {
                at: start,
                expected,
                found: name,
            }),
            None => Err(Error::Mismatched {
                at: start,
                expected: String::new(),
                found: name,
            }),
        }
    }

    fn read_start_tag(&mut self) -> Result<Event, Error> {
        let start = self.at;
        let close = self.tag_end(start)?;
        let self_closing = self.source[close - 1] == b'/';
        let inner_end = if self_closing { close - 1 } else { close };
        let inner = self.text(start + 1..inner_end)?;

        let mut cursor = inner.char_indices().peekable();
        let mut name = String::new();
        for (_, c) in cursor.by_ref() {
            if c.is_whitespace() {
                break;
            }
            name.push(c);
        }
        let name = local_name(&name);
        if name.is_empty() {
            return Err(Error::Malformed {
                at: start,
                what: "start tag",
            });
        }
        let attribute_text: String = cursor.map(|(_, c)| c).collect();
        let attributes = self.read_attributes(&attribute_text, start)?;

        self.at = close + 1;
        self.open.push(name.clone());
        if self_closing {
            self.pending_end = Some(name.clone());
        }
        Ok(Event::Start { name, attributes })
    }

    fn read_attributes(&self, text: &str, at: usize) -> Result<Vec<(String, String)>, Error> {
        let mut attributes = Vec::new();
        let bytes = text.as_bytes();
        let mut i = 0usize;
        while i < bytes.len() {
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i >= bytes.len() {
                break;
            }
            let key_start = i;
            while i < bytes.len() && bytes[i] != b'=' && !bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            let key = text[key_start..i].trim();
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i >= bytes.len() || bytes[i] != b'=' {
                return Err(Error::Malformed {
                    at,
                    what: "attribute with no value",
                });
            }
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i >= bytes.len() || (bytes[i] != b'"' && bytes[i] != b'\'') {
                return Err(Error::Malformed {
                    at,
                    what: "unquoted attribute value",
                });
            }
            let quote = bytes[i];
            i += 1;
            let value_start = i;
            while i < bytes.len() && bytes[i] != quote {
                i += 1;
            }
            if i >= bytes.len() {
                return Err(Error::Unterminated {
                    at,
                    what: "attribute value",
                });
            }
            let raw = &text[value_start..i];
            i += 1;
            attributes.push((local_name(key), decode_entities(raw, at)?));
        }
        Ok(attributes)
    }

    /// The index of the `>` that closes the tag starting at `from`, skipping
    /// any `>` inside a quoted attribute value.
    fn tag_end(&self, from: usize) -> Result<usize, Error> {
        let mut i = from + 1;
        let mut quote = 0u8;
        while i < self.source.len() {
            let byte = self.source[i];
            if quote != 0 {
                if byte == quote {
                    quote = 0;
                }
            } else if byte == b'"' || byte == b'\'' {
                quote = byte;
            } else if byte == b'>' {
                return Ok(i);
            }
            i += 1;
        }
        Err(Error::Unterminated {
            at: from,
            what: "tag",
        })
    }

    fn find(&self, needle: &[u8], what: &'static str) -> Result<usize, Error> {
        find_from(self.source, self.at, needle).ok_or(Error::Unterminated { at: self.at, what })
    }

    fn skip_past(&mut self, needle: &[u8], what: &'static str) -> Result<(), Error> {
        let at = self.find(needle, what)?;
        self.at = at + needle.len();
        Ok(())
    }

    fn text(&self, range: std::ops::Range<usize>) -> Result<&'a str, Error> {
        let start = range.start;
        std::str::from_utf8(&self.source[range]).map_err(|_| Error::NotUtf8 { at: start })
    }
}

fn find_from(haystack: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    (from..=haystack.len() - needle.len()).find(|i| &haystack[*i..*i + needle.len()] == needle)
}

/// `a:b` becomes `b`. FHIR bundles use one default namespace, and the
/// cross-check addresses elements by local name.
fn local_name(name: &str) -> String {
    match name.rsplit_once(':') {
        Some((_, local)) => local.to_string(),
        None => name.to_string(),
    }
}

/// The five predefined entities and numeric character references. Anything
/// else is an error rather than a silent pass-through.
fn decode_entities(text: &str, at: usize) -> Result<String, Error> {
    if !text.contains('&') {
        return Ok(text.to_string());
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let tail = &rest[amp..];
        let end = tail.find(';').ok_or_else(|| Error::UnknownEntity {
            at,
            reference: tail.chars().take(12).collect(),
        })?;
        let reference = &tail[1..end];
        let decoded = match reference {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "quot" => '"',
            "apos" => '\'',
            numeric if numeric.starts_with('#') => {
                let (digits, radix) =
                    match numeric.strip_prefix("#x").or(numeric.strip_prefix("#X")) {
                        Some(hex) => (hex, 16),
                        None => (&numeric[1..], 10),
                    };
                u32::from_str_radix(digits, radix)
                    .ok()
                    .and_then(char::from_u32)
                    .ok_or_else(|| Error::UnknownEntity {
                        at,
                        reference: format!("&{reference};"),
                    })?
            }
            _ => {
                return Err(Error::UnknownEntity {
                    at,
                    reference: format!("&{reference};"),
                });
            }
        };
        out.push(decoded);
        rest = &tail[end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}
