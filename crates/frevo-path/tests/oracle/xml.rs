//! A minimal, strict XML reader for the oracle harness.
//!
//! Test-only. It exists because the harness must read two kinds of XML — the
//! pinned FHIRPath suite `oracles/fhirpath/tests-fhir-r4.xml` and the suite's
//! XML input files under `oracles/fhir-test-cases/fetched/r4/` — and
//! `design.md` section 6 adds no XML crate to the workspace. Nothing here is
//! compiled into a shipped crate.
//!
//! Strictness is the point. Every construct the pinned content uses is handled;
//! anything else is an error carrying a byte offset, so a silently
//! mis-interpreted suite file can never be mistaken for a suite that parsed.
//! An oracle that shrugged off malformed input would be an oracle that grades
//! fewer cases than it claims to.

use std::fmt;

/// One node of element content.
#[derive(Debug, Clone)]
pub enum Node {
    Element(Element),
    /// Character data, with entities already decoded.
    Text(String),
}

/// An XML element.
#[derive(Debug, Clone)]
pub struct Element {
    /// The tag name exactly as written, prefix included.
    pub name: String,
    /// Attributes in document order. Names are as written, values decoded.
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Node>,
    /// Byte offset of the `<` that opens this element.
    pub offset: usize,
}

impl Element {
    /// The attribute's decoded value, or `""` when it is absent.
    ///
    /// Absent and empty collapse deliberately: `org.w3c.dom.Element`'s
    /// `getAttribute` returns `""` for an absent attribute and
    /// `Utilities.noString` treats `""` as "not stated", which is the rule
    /// `FHIRPathTests` compares by.
    pub fn attr(&self, name: &str) -> &str {
        self.attrs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
            .unwrap_or("")
    }

    /// Whether the attribute is present at all, empty value included.
    pub fn has_attr(&self, name: &str) -> bool {
        self.attrs.iter().any(|(k, _)| k == name)
    }

    /// Child elements with this tag name, in document order.
    pub fn children_named(&self, name: &str) -> Vec<&Element> {
        self.children
            .iter()
            .filter_map(|n| match n {
                Node::Element(e) if e.name == name => Some(e),
                _ => None,
            })
            .collect()
    }

    /// Every child element, in document order.
    pub fn child_elements(&self) -> Vec<&Element> {
        self.children
            .iter()
            .filter_map(|n| match n {
                Node::Element(e) => Some(e),
                _ => None,
            })
            .collect()
    }

    /// Concatenated descendant character data, as `Node.getTextContent()`.
    pub fn text_content(&self) -> String {
        let mut out = String::new();
        self.append_text(&mut out);
        out
    }

    fn append_text(&self, out: &mut String) {
        for n in &self.children {
            match n {
                Node::Text(t) => out.push_str(t),
                Node::Element(e) => e.append_text(out),
            }
        }
    }
}

/// A parsed document.
#[derive(Debug, Clone)]
pub struct Document {
    pub root: Element,
}

/// A parse failure, with the byte offset at which it was detected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlError {
    pub offset: usize,
    pub message: String,
}

impl fmt::Display for XmlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "XML error at byte {}: {}", self.offset, self.message)
    }
}

/// Parse a whole document. The input must be UTF-8.
pub fn parse(src: &str) -> Result<Document, XmlError> {
    Parser {
        b: src.as_bytes(),
        i: 0,
        src,
    }
    .document()
}

struct Parser<'a> {
    b: &'a [u8],
    i: usize,
    src: &'a str,
}

impl<'a> Parser<'a> {
    fn err<T>(&self, message: impl Into<String>) -> Result<T, XmlError> {
        Err(XmlError {
            offset: self.i,
            message: message.into(),
        })
    }

    fn at(&self, s: &str) -> bool {
        self.b[self.i..].starts_with(s.as_bytes())
    }

    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }

    fn skip_space(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\r' | b'\n')) {
            self.i += 1;
        }
    }

    /// Consume a `<!-- ... -->`, `<?...?>` or `<!DOCTYPE ...>` prologue item.
    /// Returns false when the next thing is not one of those.
    fn skip_misc(&mut self) -> Result<bool, XmlError> {
        if self.at("<!--") {
            match self.src[self.i + 4..].find("-->") {
                Some(rel) => {
                    self.i = self.i + 4 + rel + 3;
                    Ok(true)
                }
                None => self.err("unterminated comment"),
            }
        } else if self.at("<?") {
            match self.src[self.i + 2..].find("?>") {
                Some(rel) => {
                    self.i = self.i + 2 + rel + 2;
                    Ok(true)
                }
                None => self.err("unterminated processing instruction"),
            }
        } else if self.at("<!DOCTYPE") {
            // Only the flat form is accepted: an internal subset could carry
            // entity declarations, and silently ignoring those would change
            // what the document means.
            match self.src[self.i..].find('>') {
                Some(rel) => {
                    if self.src[self.i..self.i + rel].contains('[') {
                        return self.err("DOCTYPE with an internal subset is not supported");
                    }
                    self.i = self.i + rel + 1;
                    Ok(true)
                }
                None => self.err("unterminated DOCTYPE"),
            }
        } else {
            Ok(false)
        }
    }

    fn document(&mut self) -> Result<Document, XmlError> {
        loop {
            self.skip_space();
            if !self.skip_misc()? {
                break;
            }
        }
        if !self.at("<") {
            return self.err("expected the root element");
        }
        let root = self.element()?;
        loop {
            self.skip_space();
            if !self.skip_misc()? {
                break;
            }
        }
        if self.i != self.b.len() {
            return self.err("trailing content after the root element");
        }
        Ok(Document { root })
    }

    fn name(&mut self) -> Result<String, XmlError> {
        let start = self.i;
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.' | b':') {
                self.i += 1;
            } else {
                break;
            }
        }
        if self.i == start {
            return self.err("expected a name");
        }
        Ok(self.src[start..self.i].to_string())
    }

    fn element(&mut self) -> Result<Element, XmlError> {
        let offset = self.i;
        self.i += 1; // '<'
        let name = self.name()?;
        let mut attrs: Vec<(String, String)> = Vec::new();
        loop {
            self.skip_space();
            match self.peek() {
                Some(b'/') => {
                    self.i += 1;
                    if self.peek() != Some(b'>') {
                        return self.err("expected '>' after '/'");
                    }
                    self.i += 1;
                    return Ok(Element {
                        name,
                        attrs,
                        children: Vec::new(),
                        offset,
                    });
                }
                Some(b'>') => {
                    self.i += 1;
                    break;
                }
                Some(_) => {
                    let at_name = self.name()?;
                    self.skip_space();
                    if self.peek() != Some(b'=') {
                        return self.err(format!("expected '=' after attribute '{at_name}'"));
                    }
                    self.i += 1;
                    self.skip_space();
                    let quote = match self.peek() {
                        Some(q @ (b'"' | b'\'')) => q,
                        _ => return self.err("expected a quoted attribute value"),
                    };
                    self.i += 1;
                    let start = self.i;
                    while self.peek().is_some_and(|c| c != quote) {
                        self.i += 1;
                    }
                    if self.peek().is_none() {
                        return self.err("unterminated attribute value");
                    }
                    let raw = &self.src[start..self.i];
                    self.i += 1;
                    if attrs.iter().any(|(k, _)| *k == at_name) {
                        return self.err(format!("duplicate attribute '{at_name}'"));
                    }
                    let value = decode_entities(raw, start)?;
                    attrs.push((at_name, value));
                }
                None => return self.err("unterminated start tag"),
            }
        }

        let mut children: Vec<Node> = Vec::new();
        loop {
            if self.i >= self.b.len() {
                return self.err(format!("unterminated element '{name}'"));
            }
            if self.at("</") {
                self.i += 2;
                let close = self.name()?;
                if close != name {
                    return self.err(format!("'</{close}>' closes '<{name}>'"));
                }
                self.skip_space();
                if self.peek() != Some(b'>') {
                    return self.err("expected '>' in the end tag");
                }
                self.i += 1;
                return Ok(Element {
                    name,
                    attrs,
                    children,
                    offset,
                });
            }
            if self.at("<![CDATA[") {
                let start = self.i + 9;
                match self.src[start..].find("]]>") {
                    Some(rel) => {
                        children.push(Node::Text(self.src[start..start + rel].to_string()));
                        self.i = start + rel + 3;
                    }
                    None => return self.err("unterminated CDATA section"),
                }
                continue;
            }
            if self.at("<!--") || self.at("<?") {
                self.skip_misc()?;
                continue;
            }
            if self.at("<!") {
                return self.err("unsupported '<!' construct inside an element");
            }
            if self.at("<") {
                children.push(Node::Element(self.element()?));
                continue;
            }
            let start = self.i;
            while self.peek().is_some_and(|c| c != b'<') {
                self.i += 1;
            }
            let decoded = decode_entities(&self.src[start..self.i], start)?;
            children.push(Node::Text(decoded));
        }
    }
}

/// Decode the five predefined entities and numeric character references.
///
/// An unknown entity is an error rather than a pass-through: the pinned content
/// uses only `&quot;` and `&amp;`, so an unrecognised one means either a
/// re-pinned suite or a bug, and both must be visible.
fn decode_entities(raw: &str, base: usize) -> Result<String, XmlError> {
    if !raw.contains('&') {
        return Ok(raw.to_string());
    }
    let mut out = String::with_capacity(raw.len());
    let bytes = raw.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'&' {
            let start = i;
            while i < bytes.len() && bytes[i] != b'&' {
                i += 1;
            }
            out.push_str(&raw[start..i]);
            continue;
        }
        let end = match raw[i..].find(';') {
            Some(rel) => i + rel,
            None => {
                return Err(XmlError {
                    offset: base + i,
                    message: "unterminated entity reference".into(),
                });
            }
        };
        let body = &raw[i + 1..end];
        let ch = match body {
            "lt" => '<',
            "gt" => '>',
            "amp" => '&',
            "quot" => '"',
            "apos" => '\'',
            _ => {
                let code = if let Some(hex) = body.strip_prefix("#x").or(body.strip_prefix("#X")) {
                    u32::from_str_radix(hex, 16).ok()
                } else if let Some(dec) = body.strip_prefix('#') {
                    dec.parse::<u32>().ok()
                } else {
                    None
                };
                match code.and_then(char::from_u32) {
                    Some(c) => c,
                    None => {
                        return Err(XmlError {
                            offset: base + i,
                            message: format!("unknown entity reference '&{body};'"),
                        });
                    }
                }
            }
        };
        out.push(ch);
        i = end + 1;
    }
    Ok(out)
}
