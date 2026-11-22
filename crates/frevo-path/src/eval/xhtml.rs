//! `Narrative.div` as the R4 object model hands it to the engine: the stored
//! XHTML text parsed by `XhtmlParser` and recomposed by `XhtmlComposer` in XML
//! mode (`path.navigation.025`).
//!
//! Derived from org.hl7.fhir.core/org.hl7.fhir.utilities/src/main/java/org/hl7/fhir/utilities/xhtml/XhtmlParser.java
//! (`parse` at `:494-533`, `normalizeNamespaces` at `:541-594`,
//! `addTextNode` and `parseElementInner` at `:596-677`, `parseScriptInner`,
//! `parseElement`, `parseAttributes` and `parseAttributeValue` at `:680-777`,
//! `skipWhiteSpaceAndComments` at `:780-807`, the character, tag-end, comment,
//! CDATA, doctype-entity, name and literal readers at `:824-1344`),
//! org.hl7.fhir.core/org.hl7.fhir.utilities/src/main/java/org/hl7/fhir/utilities/xhtml/XhtmlComposer.java
//! (`compose(XhtmlNode)` at `:105-116`, `writeNode`, `writeText`,
//! `escapeHtml`, `writeComment`, `writeCData`, `writeDocType`,
//! `writeInstruction`, `attributes` and `writeElement` at `:150-353`,
//! `composeEx` at `:486-488`),
//! org.hl7.fhir.core/org.hl7.fhir.utilities/src/main/java/org/hl7/fhir/utilities/xhtml/XhtmlNode.java
//! (`setValueAsString` at `:592-627`) and
//! org.hl7.fhir.core/org.hl7.fhir.r4/src/main/java/org/hl7/fhir/r4/model/Narrative.java
//! (`getProperty` at `:364-366`), at 73c432b5f43ddbce979978a760ec4aa81e929094;
//! and from hapi-fhir/hapi-fhir-base/src/main/java/ca/uhn/fhir/model/primitive/XhtmlDt.java
//! (`setValueAsString` at `:122-132`, `preprocessXhtmlNamespaceDeclaration`
//! at `:134-154`) at 7ab4c7c7c49281496faa1b3c2498a652457669ae.
//! Original work Copyright (c) 2011+, HL7, Inc. (the org.hl7.fhir.core
//! files, under HL7's BSD-style licence below) and Copyright (C) 2014 - 2026
//! Smile CDR, Inc. (`XhtmlDt.java`, under the Apache License, Version 2.0).
//! Modified by the Frevo authors: only the path the JSON parser reaches is
//! ported -- the default `XhtmlParser` (policy `Accept`, whitespace kept,
//! well-formedness required) and the XML-mode, non-pretty composer -- over
//! UTF-16 code units as Java reads them; the stored text is never edited; and
//! `java.util.HashMap`'s iteration order, which decides the order attributes
//! are written in, is reproduced by [`JavaHashMap`].
//!
//! Where this differs from the Java, and why each is unreachable from a
//! narrative HAPI would have accepted:
//!
//! - a character reference that reaches the end of the text with no
//!   terminator makes `readUntil` loop forever in Java; here it is an error;
//! - `Integer.parseInt` in a numeric character reference accepts any Unicode
//!   decimal digit; here only ASCII digits are read, and anything else is "not
//!   an integer", as an out-of-range number is;
//! - a numeric reference to a lone surrogate yields a Java string a Rust
//!   string cannot hold; it is written as U+FFFD;
//! - messages omit `descLoc()`'s line and column;
//! - a `HashMap` bin that Java would turn into a tree (nine or more attribute
//!   names of one element colliding in one bucket of a table of 64 or more)
//!   keeps list order here.
//!
//! The ingest path guarantees more than this module relies on: HAPI's
//! `XhtmlDt.parse` also runs the text through a StAX parser, so a stored
//! narrative is well-formed XML by the time anything navigates it.
//!
//! The org.hl7.fhir.core sources named above carry this notice, reproduced as
//! their licence requires:
//!
//! > Copyright (c) 2011+, HL7, Inc.
//! > All rights reserved.
//! >
//! > Redistribution and use in source and binary forms, with or without
//! > modification, are permitted provided that the following conditions are
//! > met:
//! >
//! > * Redistributions of source code must retain the above copyright notice,
//! >   this list of conditions and the following disclaimer.
//! > * Redistributions in binary form must reproduce the above copyright
//! >   notice, this list of conditions and the following disclaimer in the
//! >   documentation and/or other materials provided with the distribution.
//! > * Neither the name of HL7 nor the names of its contributors may be used
//! >   to endorse or promote products derived from this software without
//! >   specific prior written permission.
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

use std::collections::HashMap;
use std::sync::OnceLock;

use super::xhtml_entities::ENTITIES;

const DECL_XMLNS: &str = " xmlns=\"http://www.w3.org/1999/xhtml\"";
/// `XhtmlParser.END_OF_CHARS`: `(char) -1`.
const EOC: u16 = 0xFFFF;
/// `XhtmlParser.MAX_XHTML_DEPTH`.
const MAX_XHTML_DEPTH: usize = 500;

/// Why a stored narrative has no composition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum XhtmlError {
    /// `XhtmlParser` rejects the text: a `FHIRFormatError`, or a Java runtime
    /// exception on the same path. HAPI fails the resource's parse with it.
    Format(String),
    /// The text leaves `XhtmlNode` with no node type (it is empty, or only a
    /// processing instruction), and `XhtmlComposer.writeNode` throws
    /// `new Error("Null node type")`.
    NullNodeType,
}

impl XhtmlError {
    pub(crate) fn message(&self) -> String {
        match self {
            XhtmlError::Format(m) => format!("the narrative is not XHTML HAPI can parse: {m}"),
            XhtmlError::NullNodeType => "Null node type".to_string(),
        }
    }
}

fn format_error(message: impl Into<String>) -> XhtmlError {
    XhtmlError::Format(message.into())
}

/// `new XhtmlComposer(true).composeEx(div)` for the `div` that
/// `XhtmlNode.setValueAsString(stored)` builds, as the JSON parser sets it
/// (`XhtmlDt.setValueAsString` first, then the R4 node's).
pub(crate) fn compose_narrative(stored: &str) -> Result<String, XhtmlError> {
    // `XhtmlDt.setValueAsString`: an empty value is no value.
    if stored.is_empty() {
        return Err(XhtmlError::NullNodeType);
    }
    let value = java_trim(stored);
    if value.is_empty() {
        // `preprocessXhtmlNamespaceDeclaration("")`: `charAt(0)` throws.
        return Err(format_error("String index out of range: 0"));
    }
    let value = preprocess_xhtml_namespace_declaration(value);
    // `XhtmlNode.setValueAsString`.
    let mut val = java_trim(&value).to_string();
    if !val.starts_with('<') {
        val = format!("<div{DECL_XMLNS}>{val}</div>");
    }
    if val.starts_with("<?") && val.ends_with("?>") {
        return Err(XhtmlError::NullNodeType);
    }
    let val = preprocess_xhtml_namespace_declaration(&val);
    let document = Parser::new(&val).parse("div")?;
    let skip = usize::from(matches!(document.first(), Some(XNode::Instruction(_))));
    let root = document
        .get(skip)
        .ok_or_else(|| format_error("Index out of range"))?;
    let mut out = Vec::new();
    write_node(root, &mut out);
    Ok(String::from_utf16_lossy(&out))
}

/// `String.trim()`: every leading and trailing character at or below U+0020.
fn java_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c <= ' ')
}

/// `XhtmlDt.preprocessXhtmlNamespaceDeclaration`. The value is never empty
/// here; the characters searched for are ASCII, so byte and `char` indexes
/// agree on them.
fn preprocess_xhtml_namespace_declaration(value: &str) -> String {
    let mut value = if value.starts_with('<') {
        value.to_string()
    } else {
        format!("<div{DECL_XMLNS}>{value}</div>")
    };
    let from = usize::from(value.starts_with("<?"));
    if let Some(first_tag_index) = value[from..].find('<').map(|i| i + from) {
        let first_tag_end = value[first_tag_index..]
            .find('>')
            .map(|i| i + first_tag_index);
        let first_slash = value[first_tag_index..]
            .find('/')
            .map(|i| i + first_tag_index);
        if let (Some(end), Some(slash)) = (first_tag_end, first_slash) {
            if slash > end && !value[first_tag_index..end].contains(" xmlns") {
                value.insert_str(end, DECL_XMLNS);
            }
        }
    }
    value
}

// ---------------------------------------------------------------------------
// The node tree
// ---------------------------------------------------------------------------

/// Text, attribute values and comment content are kept as UTF-16 code units,
/// which is what the Java reads and writes.
type Units = Vec<u16>;

#[derive(Debug)]
enum XNode {
    Element(Element),
    Text(Units),
    Comment(Units),
    CData(Units),
    DocType(Units),
    Instruction(Units),
}

#[derive(Debug)]
struct Element {
    name: String,
    attributes: JavaHashMap,
    children: Vec<XNode>,
}

impl Element {
    fn new(name: String) -> Self {
        Element {
            name,
            attributes: JavaHashMap::new(),
            children: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// java.util.HashMap's iteration order
// ---------------------------------------------------------------------------

/// The parts of `java.util.HashMap<String, String>` that decide iteration
/// order: the table's capacity, which grows as Java's does, and each bucket's
/// list order. Values are `None` for Java's `null`.
#[derive(Debug, Clone)]
pub(super) struct JavaHashMap {
    table: Vec<Vec<(String, Option<Units>)>>,
    threshold: usize,
    size: usize,
}

const DEFAULT_INITIAL_CAPACITY: usize = 16;
const TREEIFY_THRESHOLD: usize = 8;
const MIN_TREEIFY_CAPACITY: usize = 64;

/// `String.hashCode()` spread as `HashMap.hash` spreads it.
pub(super) fn java_hash(key: &str) -> u32 {
    let mut h: u32 = 0;
    for unit in key.encode_utf16() {
        h = h.wrapping_mul(31).wrapping_add(u32::from(unit));
    }
    h ^ (h >> 16)
}

impl JavaHashMap {
    /// `new HashMap<>()`.
    pub(super) fn new() -> Self {
        JavaHashMap {
            table: Vec::new(),
            threshold: 0,
            size: 0,
        }
    }

    /// `new HashMap<>()` followed by `putAll(source)`
    /// (`HashMap.putMapEntries`): the table is sized for `source` up front.
    pub(super) fn copy_of(source: &JavaHashMap) -> Self {
        let mut map = JavaHashMap::new();
        let s = source.size;
        if s > 0 {
            // `(int) ((float) s / loadFactor + 1.0F)`, in integers:
            // floor(s * 4 / 3) + 1.
            let t = s * 4 / 3 + 1;
            if t > map.threshold {
                map.threshold = t.next_power_of_two();
            }
        }
        for (k, v) in source.iter() {
            map.put(k.to_string(), v.cloned());
        }
        map
    }

    #[cfg(test)]
    pub(super) fn capacity(&self) -> usize {
        self.table.len()
    }

    fn index(&self, hash: u32) -> usize {
        (hash as usize) & (self.table.len() - 1)
    }

    /// `HashMap.resize`.
    fn resize(&mut self) {
        let old_cap = self.table.len();
        let old_thr = self.threshold;
        let (new_cap, new_thr) = if old_cap > 0 {
            let new_cap = old_cap << 1;
            let new_thr = if old_cap >= DEFAULT_INITIAL_CAPACITY {
                old_thr << 1
            } else {
                new_cap * 3 / 4
            };
            (new_cap, new_thr)
        } else if old_thr > 0 {
            (old_thr, old_thr * 3 / 4)
        } else {
            (DEFAULT_INITIAL_CAPACITY, DEFAULT_INITIAL_CAPACITY * 3 / 4)
        };
        let old = std::mem::take(&mut self.table);
        self.table = vec![Vec::new(); new_cap];
        self.threshold = new_thr;
        // Splitting each bucket into its low and high halves keeps the
        // entries' relative order, which re-inserting in iteration order does.
        for bucket in old {
            for (k, v) in bucket {
                let i = self.index(java_hash(&k));
                self.table[i].push((k, v));
            }
        }
    }

    /// `HashMap.put`: a key already present keeps its place.
    pub(super) fn put(&mut self, key: String, value: Option<Units>) {
        if self.table.is_empty() {
            self.resize();
        }
        let i = self.index(java_hash(&key));
        if let Some(entry) = self.table[i].iter_mut().find(|(k, _)| *k == key) {
            entry.1 = value;
            return;
        }
        let bin_count = self.table[i].len();
        self.table[i].push((key, value));
        // `treeifyBin` on a table smaller than 64 resizes instead.
        if bin_count >= TREEIFY_THRESHOLD && self.table.len() < MIN_TREEIFY_CAPACITY {
            self.resize();
        }
        self.size += 1;
        if self.size > self.threshold {
            self.resize();
        }
    }

    fn get(&self, key: &str) -> Option<&Option<Units>> {
        if self.table.is_empty() {
            return None;
        }
        self.table[self.index(java_hash(key))]
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }

    fn contains_key(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    fn remove(&mut self, key: &str) {
        if self.table.is_empty() {
            return;
        }
        let i = self.index(java_hash(key));
        if let Some(at) = self.table[i].iter().position(|(k, _)| k == key) {
            self.table[i].remove(at);
            self.size -= 1;
        }
    }

    fn iter(&self) -> impl Iterator<Item = (&str, Option<&Units>)> {
        self.table
            .iter()
            .flatten()
            .map(|(k, v)| (k.as_str(), v.as_ref()))
    }
}

// ---------------------------------------------------------------------------
// Character classes, as `java.lang.Character` answers them for one `char`
// ---------------------------------------------------------------------------

/// `Character.isWhitespace(char)`.
fn is_whitespace(u: u16) -> bool {
    matches!(
        u,
        0x09..=0x0D
            | 0x1C..=0x20
            | 0x1680
            | 0x2000..=0x2006
            | 0x2008..=0x200A
            | 0x2028
            | 0x2029
            | 0x205F
            | 0x3000
    )
}

/// `Character.isLetterOrDigit(char)`: a letter (`L*`) or a decimal digit
/// (`Nd`). A surrogate is neither.
fn is_letter_or_digit(u: u16) -> bool {
    if u < 0x80 {
        return u8::try_from(u).is_ok_and(|b| b.is_ascii_alphanumeric());
    }
    let Some(c) = char::from_u32(u32::from(u)) else {
        return false;
    };
    static LETTER_OR_DIGIT: OnceLock<Option<regex::Regex>> = OnceLock::new();
    let mut buf = [0u8; 4];
    LETTER_OR_DIGIT
        .get_or_init(|| regex::Regex::new(r"^[\p{L}\p{Nd}]$").ok())
        .as_ref()
        .is_some_and(|re| re.is_match(c.encode_utf8(&mut buf)))
}

/// `XhtmlParser.isNameChar`.
fn is_name_char(u: u16) -> bool {
    is_letter_or_digit(u) || matches!(u, 0x5F | 0x2D | 0x3A | 0x2E)
}

fn units(s: &str) -> Units {
    s.encode_utf16().collect()
}

fn string(u: &[u16]) -> String {
    String::from_utf16_lossy(u)
}

const LT: u16 = 0x3C;
const GT: u16 = 0x3E;
const SLASH: u16 = 0x2F;
const BANG: u16 = 0x21;
const QUESTION: u16 = 0x3F;
const AMP: u16 = 0x26;
const DASH: u16 = 0x2D;
const LBRACKET: u16 = 0x5B;
const RBRACKET: u16 = 0x5D;
const EQUALS: u16 = 0x3D;
const DQUOTE: u16 = 0x22;
const SQUOTE: u16 = 0x27;
const SPACE: u16 = 0x20;
const HASH: u16 = 0x23;
const LOWER_X: u16 = 0x78;

// ---------------------------------------------------------------------------
// XhtmlParser
// ---------------------------------------------------------------------------

/// `XhtmlParser.ElementName`: a `prefix:name`, split at the first `:`.
struct ElementName {
    namespace: Option<String>,
    name: String,
}

impl ElementName {
    fn new(src: &str) -> Self {
        match src.split_once(':') {
            Some((ns, name)) => ElementName {
                namespace: Some(ns.to_string()),
                name: name.to_string(),
            },
            None => ElementName {
                namespace: None,
                name: src.to_string(),
            },
        }
    }
}

/// `XhtmlParser.NamespaceNormalizationMap`.
struct NamespaceMap {
    default_namespace: Option<Units>,
    original_namespace_prefix: Option<String>,
    map: JavaHashMap,
}

impl NamespaceMap {
    fn new(parent: Option<&NamespaceMap>) -> Self {
        match parent {
            Some(p) => NamespaceMap {
                default_namespace: p.default_namespace.clone(),
                original_namespace_prefix: p.original_namespace_prefix.clone(),
                map: JavaHashMap::copy_of(&p.map),
            },
            None => NamespaceMap {
                default_namespace: None,
                original_namespace_prefix: None,
                map: JavaHashMap::new(),
            },
        }
    }

    /// `getNamespaceForPrefix`: `None` is a Java `null` value.
    fn namespace_for_prefix(&self, prefix: &str) -> Option<Units> {
        if self.original_namespace_prefix.as_deref() == Some(prefix) {
            return self.default_namespace.clone();
        }
        match self.map.get(prefix) {
            Some(v) => v.clone(),
            None => Some(units("http://error/undefined-namespace")),
        }
    }
}

struct Parser {
    src: Units,
    pos: usize,
    /// `pushChar`'s cache, in front of the unread source.
    cache: Vec<u16>,
    declared_entities: HashMap<String, Units>,
}

impl Parser {
    fn new(source: &str) -> Self {
        Parser {
            src: units(source),
            pos: 0,
            cache: Vec::new(),
            declared_entities: HashMap::new(),
        }
    }

    fn peek(&self) -> u16 {
        match self.cache.first() {
            Some(c) => *c,
            None => self.src.get(self.pos).copied().unwrap_or(EOC),
        }
    }

    fn read(&mut self) -> u16 {
        if !self.cache.is_empty() {
            return self.cache.remove(0);
        }
        match self.src.get(self.pos) {
            Some(c) => {
                self.pos += 1;
                *c
            }
            None => EOC,
        }
    }

    fn push_char(&mut self, c: u16) {
        self.cache.insert(0, c);
    }

    /// `parse(String entryName)`: the document's nodes -- leading comments,
    /// document types and instructions, then the root element.
    fn parse(mut self, entry_name: &str) -> Result<Vec<XNode>, XhtmlError> {
        let mut document = Vec::new();
        self.skip_white_space_and_comments(&mut document)?;
        if self.peek() != LT {
            return Err(format_error(
                "Unable to Parse HTML - does not start with tag",
            ));
        }
        self.read();
        let n = ElementName::new(&self.read_name().to_lowercase());
        if n.name != entry_name {
            return Err(format_error(format!(
                "Unable to Parse HTML - starts with '{}' not '{entry_name}'",
                n.name
            )));
        }
        let mut root = Element::new(n.name.clone());
        self.parse_attributes(&mut root)?;
        let nsm = normalize_namespaces(&n, &mut root, None, true)?;
        if self.read() == SLASH {
            if self.peek() != GT {
                return Err(format_error("unexpected non-end of element"));
            }
            self.read();
        } else {
            self.parse_element_inner(&mut root, 0, &nsm)?;
        }
        document.push(XNode::Element(root));
        Ok(document)
    }

    /// `addTextNode`, with whitespace kept.
    fn add_text_node(node: &mut Element, s: &mut Units) {
        if !s.is_empty() {
            node.children.push(XNode::Text(std::mem::take(s)));
        }
    }

    /// `parseElementInner`. With `mustBeWellFormed` set, a mismatched end tag
    /// is an error, so the unwind recovery never runs and is not ported.
    /// `depth` is `parents.size()`.
    fn parse_element_inner(
        &mut self,
        node: &mut Element,
        depth: usize,
        nsm: &NamespaceMap,
    ) -> Result<(), XhtmlError> {
        let mut s = Units::new();
        while self.peek() != EOC {
            if self.peek() == LT {
                Self::add_text_node(node, &mut s);
                self.read();
                if self.peek() == BANG {
                    self.read();
                    if self.peek() == LBRACKET {
                        let sc = self.read_cdata()?;
                        node.children.push(XNode::CData(sc));
                    } else {
                        self.push_char(BANG);
                        let sc = self.read_to_comment_end(true)?;
                        node.children.push(XNode::Comment(sc));
                    }
                } else if self.peek() == QUESTION {
                    let sc = self.read_to_tag_end()?;
                    node.children.push(XNode::Comment(sc));
                } else if self.peek() == SLASH {
                    self.read();
                    let end = self.read_to_tag_end()?;
                    let n = ElementName::new(&string(&end));
                    if node.name == n.name {
                        return Ok(());
                    }
                    return Err(format_error(format!(
                        "Malformed XHTML: Found \"</{}>\" expecting \"</{}>\"",
                        n.name, node.name
                    )));
                } else if is_letter_or_digit(self.peek()) {
                    self.parse_element(node, depth, nsm)?;
                } else {
                    return Err(format_error(format!(
                        "Unable to Parse HTML - node '{}' has unexpected content",
                        node.name
                    )));
                }
            } else if self.peek() == AMP {
                self.parse_literal(&mut s)?;
            } else {
                let c = self.read();
                s.push(c);
            }
        }
        Self::add_text_node(node, &mut s);
        Ok(())
    }

    /// `parseScriptInner`: the text up to `</script>`, whose last nine
    /// characters are cut whatever they are.
    fn parse_script_inner(&mut self, node: &mut Element) {
        let end = units("</script>");
        let mut s = Units::new();
        while self.peek() != EOC && !s.ends_with(&end) {
            let c = self.read();
            s.push(c);
        }
        if s.len() >= end.len() {
            s.truncate(s.len() - end.len());
        }
        if !s.is_empty() {
            node.children.push(XNode::Text(s));
        }
    }

    /// `parseElement`; `depth` is `parents.size()`.
    fn parse_element(
        &mut self,
        parent: &mut Element,
        depth: usize,
        nsm: &NamespaceMap,
    ) -> Result<(), XhtmlError> {
        if depth > MAX_XHTML_DEPTH {
            return Err(format_error(format!(
                "XHTML nesting depth exceeds maximum of {MAX_XHTML_DEPTH}"
            )));
        }
        let name = ElementName::new(&self.read_name());
        let mut node = Element::new(name.name.clone());
        self.parse_attributes(&mut node)?;
        let nsm = normalize_namespaces(&name, &mut node, Some(nsm), false)?;
        if self.read() == SLASH {
            if self.peek() != GT {
                return Err(format_error("unexpected non-end of element"));
            }
            self.read();
        } else if name.name == "script" {
            self.parse_script_inner(&mut node);
        } else {
            self.parse_element_inner(&mut node, depth + 1, &nsm)?;
        }
        parent.children.push(XNode::Element(node));
        Ok(())
    }

    fn skip_whitespace(&mut self) {
        while is_whitespace(self.peek()) {
            self.read();
        }
    }

    fn parse_attributes(&mut self, node: &mut Element) -> Result<(), XhtmlError> {
        self.skip_whitespace();
        while !matches!(self.peek(), GT | SLASH | EOC) {
            let name = self.read_name();
            if name.is_empty() {
                return Err(format_error(format!(
                    "Unable to read attribute on <{}>",
                    node.name
                )));
            }
            self.skip_whitespace();
            let p = self.peek();
            if is_name_char(p) || p == GT || p == SLASH {
                node.attributes.put(name, None);
            } else if p != EQUALS {
                return Err(format_error(format!(
                    "Unable to read attribute '{name}' value on <{}>",
                    node.name
                )));
            } else {
                self.read();
                self.skip_whitespace();
                let value = if matches!(self.peek(), DQUOTE | SQUOTE) {
                    let term = self.read();
                    self.parse_attribute_value(term)?
                } else {
                    self.parse_attribute_value(EOC)?
                };
                node.attributes.put(name, Some(value));
            }
            self.skip_whitespace();
        }
        Ok(())
    }

    fn parse_attribute_value(&mut self, term: u16) -> Result<Units, XhtmlError> {
        let mut b = Units::new();
        loop {
            let p = self.peek();
            if p == EOC || p == GT || (term == EOC && p == SLASH) || p == term {
                break;
            }
            if p == AMP {
                self.parse_literal(&mut b)?;
            } else {
                let c = self.read();
                b.push(c);
            }
        }
        if self.peek() == term {
            self.read();
        }
        Ok(b)
    }

    fn skip_white_space_and_comments(&mut self, focus: &mut Vec<XNode>) -> Result<(), XhtmlError> {
        loop {
            while is_whitespace(self.peek()) || self.peek() == 0xFEFF {
                self.read();
            }
            if self.peek() != LT {
                return Ok(());
            }
            let c = self.read();
            if self.peek() == BANG {
                self.read();
                if self.peek() == DASH {
                    self.read();
                    if self.peek() != DASH {
                        return Err(format_error("unrecognised element type <!"));
                    }
                    self.read();
                    if self.peek() == SPACE {
                        self.read();
                    }
                    let comment = self.read_to_comment_end(false)?;
                    focus.push(XNode::Comment(comment));
                } else {
                    let doctype = self.read_to_doc_type_end()?;
                    focus.push(XNode::DocType(doctype));
                }
            } else if self.peek() == QUESTION {
                let r = self.read_to_tag_end()?;
                if r.len() < 2 {
                    return Err(format_error("String index out of range"));
                }
                focus.push(XNode::Instruction(r[1..r.len() - 1].to_vec()));
            } else {
                self.push_char(c);
                return Ok(());
            }
        }
    }

    fn read_to_tag_end(&mut self) -> Result<Units, XhtmlError> {
        let mut s = Units::new();
        while self.peek() != GT && self.peek() != EOC {
            let c = self.read();
            s.push(c);
        }
        if self.peek() == EOC {
            return Err(format_error("Unexpected termination of html source"));
        }
        self.read();
        Ok(s)
    }

    fn read_to_doc_type_end(&mut self) -> Result<Units, XhtmlError> {
        let mut s = Units::new();
        loop {
            let c = self.peek();
            if c == GT {
                self.read();
                return Ok(s);
            }
            if c == EOC {
                return Err(format_error("Unexpected termination of html source"));
            }
            let c = self.read();
            s.push(c);
        }
    }

    fn read_to_comment_end(&mut self, simple: bool) -> Result<Units, XhtmlError> {
        let mut simple = simple;
        if self.peek() == BANG {
            self.read();
        }
        let mut s = Units::new();
        if self.peek() == DASH {
            self.read();
            simple = self.peek() != DASH;
            if simple {
                s.push(DASH);
            } else {
                self.read();
            }
        }
        let doctype_prefix = units("DOCTYPE ");
        let mut doctype_entities = false;
        loop {
            let c = self.peek();
            if c == DASH {
                self.read();
                if self.peek() == DASH {
                    self.read();
                    if self.peek() == GT {
                        break;
                    }
                    self.push_char(DASH);
                }
                s.push(DASH);
            } else if doctype_entities && c == RBRACKET {
                let c = self.read();
                s.push(c);
                if self.peek() == GT {
                    break;
                }
            } else if simple && c == GT && !doctype_entities {
                break;
            } else if c == LBRACKET && s.starts_with(&doctype_prefix) {
                doctype_entities = true;
                let c = self.read();
                s.push(c);
            } else if c != EOC {
                let c = self.read();
                s.push(c);
            } else {
                return Err(format_error("Unexpected termination of html source"));
            }
        }
        if self.peek() != EOC {
            self.read();
        }
        if doctype_entities {
            self.parse_doctype_entities(&string(&s))?;
        }
        Ok(s)
    }

    fn read_cdata(&mut self) -> Result<Units, XhtmlError> {
        let mut b = Units::new();
        loop {
            let c = self.read();
            if c == EOC {
                return Err(format_error("Stream ended before finding ']]>' terminator"));
            }
            b.push(c);
            let n = b.len();
            if c == GT && n >= 3 && b[n - 2] == RBRACKET && b[n - 3] == RBRACKET {
                break;
            }
        }
        // `b.substring(7, b.length() - 3)`.
        if b.len() < 10 {
            return Err(format_error("String index out of range"));
        }
        Ok(b[7..b.len() - 3].to_vec())
    }

    /// `parseDoctypeEntities`, with its `substring` and `indexOf` failures
    /// as errors.
    fn parse_doctype_entities(&mut self, s: &str) -> Result<(), XhtmlError> {
        let fail = || format_error("String index out of range");
        let mut s = s.to_string();
        while let Some(at) = s.find("<!ENTITY") {
            s = s[at..].to_string();
            let e = s.find('>').ok_or_else(fail)?;
            let ed = s[..=e].to_string();
            s = s[e + 1..].to_string();
            let ed = java_trim(&ed[8..]).to_string();
            let e = ed.find(' ').ok_or_else(fail)?;
            let n = java_trim(&ed[..e]).to_string();
            let ed = java_trim(&ed[e..]).to_string();
            let e = ed.find(' ').ok_or_else(fail)?;
            let ed = java_trim(&ed[e..]).to_string();
            let mut v = units(&ed);
            if v.pop().is_none() {
                return Err(fail());
            }
            self.declared_entities.insert(n, v);
        }
        Ok(())
    }

    fn read_name(&mut self) -> String {
        let mut s = Units::new();
        while is_name_char(self.peek()) {
            let c = self.read();
            s.push(c);
        }
        string(&s)
    }

    /// `readUntil(";&'\"><")`: the terminator, or a NUL, is consumed.
    fn read_until_literal_end(&mut self) -> Result<Units, XhtmlError> {
        let mut s = Units::new();
        loop {
            let p = self.peek();
            if p == 0 || matches!(p, 0x3B | AMP | SQUOTE | DQUOTE | GT | LT) {
                break;
            }
            if p == EOC {
                return Err(format_error("unterminated character reference"));
            }
            let c = self.read();
            s.push(c);
        }
        self.read();
        Ok(s)
    }

    /// `parseLiteral`: the `&` has not been read yet.
    fn parse_literal(&mut self, s: &mut Units) -> Result<(), XhtmlError> {
        self.read();
        let c = self.read_until_literal_end()?;
        if c.is_empty() {
            return Err(format_error(format!(
                "Invalid literal declaration following text: {}",
                string(s)
            )));
        }
        if c[0] == HASH {
            if let Some(cp) = java_parse_int(&c[1..], 10) {
                push_code_point(s, cp)?;
            } else {
                match c.get(1) {
                    None => return Err(format_error("String index out of range: 1")),
                    Some(&LOWER_X) => {
                        if let Some(cp) = java_parse_int(&c[2..], 16) {
                            push_code_point(s, cp)?;
                        }
                    }
                    // Neither form: upstream appends nothing.
                    Some(_) => {}
                }
            }
            return Ok(());
        }
        let name = string(&c);
        if let Some(v) = self.declared_entities.get(&name) {
            s.extend_from_slice(v);
            return Ok(());
        }
        match ENTITIES.binary_search_by(|(n, _)| (*n).cmp(name.as_str())) {
            Ok(i) => {
                s.extend(ENTITIES[i].1.encode_utf16());
                Ok(())
            }
            Err(_) => Err(format_error(format!(
                "unable to parse character reference '{name}''"
            ))),
        }
    }
}

/// `Integer.parseInt(s, radix)`, ASCII digits only.
fn java_parse_int(s: &[u16], radix: u32) -> Option<i32> {
    let text = String::from_utf16(s).ok()?;
    if !text.is_ascii() {
        return None;
    }
    let digits = text.strip_prefix(['+', '-']).unwrap_or(&text);
    if digits.is_empty() {
        return None;
    }
    i32::from_str_radix(&text, radix).ok()
}

/// `s.append(Character.toString(cp))`.
fn push_code_point(s: &mut Units, cp: i32) -> Result<(), XhtmlError> {
    let invalid = || format_error(format!("Not a valid Unicode code point: 0x{cp:X}"));
    let cp = u32::try_from(cp).map_err(|_| invalid())?;
    if cp > 0x10_FFFF {
        return Err(invalid());
    }
    match char::from_u32(cp) {
        Some(c) => {
            let mut buf = [0u16; 2];
            s.extend_from_slice(c.encode_utf16(&mut buf));
        }
        // A lone surrogate: the Java string holds it; the final conversion
        // to a Rust string writes U+FFFD for it.
        None => s.push(u16::try_from(cp).unwrap_or(0xFFFD)),
    }
    Ok(())
}

/// `normalizeNamespaces`.
fn normalize_namespaces(
    element_name: &ElementName,
    node: &mut Element,
    parent: Option<&NamespaceMap>,
    is_root: bool,
) -> Result<NamespaceMap, XhtmlError> {
    let mut m = NamespaceMap::new(parent);
    let mut namespace_attributes = Vec::new();
    for (an, v) in node.attributes.iter() {
        if an == "xmlns" {
            m.default_namespace = v.cloned();
            namespace_attributes.push(an.to_string());
        }
        if let Some(prefix) = an.strip_prefix("xmlns:") {
            m.map.put(prefix.to_string(), v.cloned());
            namespace_attributes.push(an.to_string());
        }
    }
    for an in &namespace_attributes {
        node.attributes.remove(an);
    }
    if let Some(prefix) = &element_name.namespace {
        // A `null` namespace is a `NullPointerException` on `equals`.
        let Some(element_namespace) = m.namespace_for_prefix(prefix) else {
            return Err(format_error("NullPointerException"));
        };
        if m.default_namespace.as_ref() != Some(&element_namespace) {
            node.attributes
                .put("xmlns".to_string(), Some(element_namespace.clone()));
            m.default_namespace = Some(element_namespace);
            m.original_namespace_prefix = Some(prefix.clone());
            m.map.remove(prefix);
        }
    }
    let add_default = if is_root {
        m.default_namespace.is_some()
    } else {
        m.default_namespace.is_some()
            && parent.is_none_or(|p| m.default_namespace != p.default_namespace)
    };
    if add_default {
        node.attributes
            .put("xmlns".to_string(), m.default_namespace.clone());
    }
    let prefixed: Vec<(String, Option<Units>)> = m
        .map
        .iter()
        .map(|(k, v)| (k.to_string(), v.cloned()))
        .collect();
    for (k, v) in prefixed {
        if is_root || parent.is_none_or(|p| !p.map.contains_key(&k)) {
            node.attributes.put(format!("xmlns:{k}"), v);
        }
    }
    Ok(m)
}

// ---------------------------------------------------------------------------
// XhtmlComposer, XML mode, not pretty
// ---------------------------------------------------------------------------

fn push_str(out: &mut Units, s: &str) {
    out.extend(s.encode_utf16());
}

fn write_node(node: &XNode, out: &mut Units) {
    match node {
        XNode::Comment(content) => {
            push_str(out, "<!-- ");
            push_str(out, java_trim(&string(content)));
            push_str(out, " -->");
        }
        XNode::DocType(content) => {
            push_str(out, "<!");
            out.extend_from_slice(content);
            push_str(out, ">\r\n");
        }
        XNode::Instruction(content) => {
            push_str(out, "<?");
            out.extend_from_slice(content);
            push_str(out, "?>\r\n");
        }
        XNode::Element(element) => write_element(element, out),
        XNode::Text(content) => write_text(content, out),
        XNode::CData(content) => {
            push_str(out, "<![CDATA[");
            out.extend_from_slice(content);
            push_str(out, "]]>");
        }
    }
}

/// `writeText`, XML mode, not canonical, no auto-links: a code point beyond
/// U+FFFF is written as a hexadecimal character reference.
fn write_text(src: &[u16], out: &mut Units) {
    let mut i = 0;
    while i < src.len() {
        let u = src[i];
        if let Some(&low) = src.get(i + 1) {
            if (0xD800..0xDC00).contains(&u) && (0xDC00..0xE000).contains(&low) {
                let cp = 0x10000 + ((u32::from(u) - 0xD800) << 10) + (u32::from(low) - 0xDC00);
                push_str(out, &format!("&#x{cp:X};"));
                i += 2;
                continue;
            }
        }
        i += 1;
        match u {
            AMP => push_str(out, "&amp;"),
            LT => push_str(out, "&lt;"),
            GT => push_str(out, "&gt;"),
            DQUOTE => push_str(out, "&quot;"),
            _ => out.push(u),
        }
    }
}

/// `escapeHtml`, not canonical: an empty or absent value is Java's `null`,
/// which string concatenation writes as `null`.
fn escape_html(value: Option<&Units>, out: &mut Units) {
    match value {
        Some(v) if !v.is_empty() => {
            for &u in v {
                match u {
                    LT => push_str(out, "&lt;"),
                    GT => push_str(out, "&gt;"),
                    DQUOTE => push_str(out, "&quot;"),
                    AMP => push_str(out, "&amp;"),
                    _ => out.push(u),
                }
            }
        }
        _ => push_str(out, "null"),
    }
}

/// `attributes`: in the map's iteration order.
fn write_attributes(element: &Element, out: &mut Units) {
    for (n, v) in element.attributes.iter() {
        push_str(out, " ");
        push_str(out, n);
        push_str(out, "=\"");
        escape_html(v, out);
        push_str(out, "\"");
    }
}

/// `writeElement`, XML mode, not pretty. An element with no children is
/// written self-closed. (The `head` special case compares names by
/// reference, which a parsed name never matches.)
fn write_element(element: &Element, out: &mut Units) {
    push_str(out, "<");
    push_str(out, &element.name);
    write_attributes(element, out);
    if element.children.is_empty() {
        push_str(out, "/>");
        return;
    }
    push_str(out, ">");
    let all_text = element.children.iter().all(|c| matches!(c, XNode::Text(_)));
    if all_text && matches!(element.name.as_str(), "script" | "style") {
        // `allText()`: the texts as they are, unescaped.
        for c in &element.children {
            if let XNode::Text(t) = c {
                out.extend_from_slice(t);
            }
        }
    } else {
        for c in &element.children {
            write_node(c, out);
        }
    }
    push_str(out, "</");
    push_str(out, &element.name);
    push_str(out, ">");
}
