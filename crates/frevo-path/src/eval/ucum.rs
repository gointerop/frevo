//! The UCUM essence data: `ucum-essence.xml` read into the prefixes, base units
//! and defined units the unit algebra works from (`design.md` section 5,
//! `path.quantity.016`).
//!
//! Derived from Ucum-java/src/main/java/org/fhir/ucum/definitions/XmlDefinitionsParser.java
//! (`parse` at `:64-106`, `parseUnit`, `parseValue`, `parseBaseUnit` and
//! `parsePrefix` at `:109-160`), Ucum-java/src/main/java/org/fhir/ucum/utils/XmlUtils.java
//! (`parseDOM`, `getNamedChild`, `getNamedChildText`, `getFirstChild`,
//! `getNextSibling`, `getNamedChildren`) and
//! Ucum-java/src/main/java/org/fhir/ucum/UcumModel.java (`getUnit`,
//! `getBaseUnit`), at db9d22ade3f0f4a667b45817b85a9ac4534e4224 (tag
//! `1.0.10`). Original work Copyright (c) 2006+, Health Intersections Pty Ltd,
//! licensed under the BSD 3-Clause License, whose text is reproduced in
//! `ucum_decimal.rs` and in `NOTICE`.
//! Modified by the Frevo authors: the DOM is this module's own reader, which
//! refuses a document type declaration as `parseDOM` does; a load failure is a
//! [`UcumLoadError`] naming the file rather than a wrapped `UcumException`;
//! and the lenient `SimpleDateFormat` that reads `revision-date` is reproduced
//! for the forms listed at `parse_revision_date` only.
//!
//! What the parser accepts is what upstream accepts: no namespace and no
//! `version` is required (a missing attribute reads as `""`), a code may be
//! defined twice (a lookup takes the first, base units before defined units,
//! `UcumModel.java:127-137`), any of the three lists may be empty, and a flag
//! is set only by exactly `yes`. What it refuses is upstream's list
//! (`path.quantity.016`): an unreadable or not well-formed file, a DOCTYPE, a
//! root element other than `root`, a missing or unparseable `revision-date`,
//! an unknown child element of the root, a prefix or unit with no `value`
//! element or a malformed `value/@value`, and a base unit with no `dim`
//! (`"".charAt(0)`).
//!
//! The data is never embedded in the binary or committed: it is loaded at run
//! time from a path the operator or CI supplies
//! (`oracles/fhir-test-cases/fetched/ucum/ucum-essence.xml` in the oracle run).
//! An absent or unparseable file is a [`UcumLoadError`], never an empty table:
//! treating units as unknown would silently move every quantity comparison onto
//! the no-UCUM branch (`path.quantity.015`).

use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

use super::ucum_decimal::{Failure, UcumDecimal};

/// The namespace of the essence file's root element.
pub const UCUM_ESSENCE_NAMESPACE: &str = "http://unitsofmeasure.org/ucum-essence";

/// A UCUM essence file could not be loaded.
///
/// Carries the path (when the data came from a file) and a message naming what
/// was wrong, so a failed run says which file and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UcumLoadError {
    path: Option<PathBuf>,
    message: String,
}

impl UcumLoadError {
    fn new(message: impl Into<String>) -> Self {
        UcumLoadError {
            path: None,
            message: message.into(),
        }
    }

    fn at(mut self, path: &Path) -> Self {
        self.path = Some(path.to_path_buf());
        self
    }

    /// The file that failed to load, if the data came from a file.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// What was wrong, without the path.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for UcumLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.path {
            Some(path) => write!(
                f,
                "UCUM essence data {} could not be loaded: {}",
                path.display(),
                self.message
            ),
            None => write!(f, "UCUM essence data could not be loaded: {}", self.message),
        }
    }
}

impl Error for UcumLoadError {}

/// A `prefix` element: a scale factor a metric unit may carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UcumPrefix {
    /// The case-sensitive code (`Code`), as `k`.
    pub code: String,
    /// The case-insensitive code (`CODE`), as `K`.
    pub code_upper: String,
    /// The `name` text, as `kilo`.
    pub name: String,
    /// The `printSymbol` text with its markup removed, if present.
    pub print_symbol: Option<String>,
    /// The factor, exactly as the `value` element's `value` attribute states
    /// it (`1e3`).
    pub value: String,
}

/// A `base-unit` element: one of the dimensions every unit reduces to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UcumBaseUnit {
    /// The case-sensitive code (`Code`), as `m`.
    pub code: String,
    /// The case-insensitive code (`CODE`), as `M`.
    pub code_upper: String,
    /// The dimension symbol (`dim`), as `L`.
    pub dim: String,
    /// The `name` text, as `meter`.
    pub name: String,
    /// The `property` text, as `length`.
    pub property: Option<String>,
}

/// The `function` of a special unit: a non-linear conversion upstream
/// implements in code, named here as the file names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UcumFunction {
    /// The function's name (`name`), as `Cel`.
    pub name: String,
    /// Its magnitude (`value`), as the file states it.
    pub value: String,
    /// Its unit (`Unit`), as `K`.
    pub unit: String,
}

/// A `unit` element: a unit defined in terms of other units.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UcumUnit {
    /// The case-sensitive code (`Code`), as `[in_i]`.
    pub code: String,
    /// The case-insensitive code (`CODE`); one unit in the 2.0.1 file (`L`)
    /// has none.
    pub code_upper: Option<String>,
    /// `isMetric="yes"`: the unit may take a prefix.
    pub is_metric: bool,
    /// `isSpecial="yes"`: the conversion is a function, not a factor.
    pub is_special: bool,
    /// `isArbitrary="yes"`: the unit is not commensurable with anything else.
    pub is_arbitrary: bool,
    /// The `class` attribute, as `si` or `dimless`.
    pub class: Option<String>,
    /// Every `name` child, in document order.
    pub names: Vec<String>,
    /// The `printSymbol` text with its markup removed, if present.
    pub print_symbol: Option<String>,
    /// The `property` text, as `length`.
    pub property: Option<String>,
    /// The defining unit term (`value/@Unit`), as `cm`.
    pub value_unit: String,
    /// The defining term in case-insensitive form (`value/@UNIT`).
    pub value_unit_upper: Option<String>,
    /// The defining magnitude (`value/@value`), exactly as stated; absent for
    /// most special units.
    pub value: Option<String>,
    /// The conversion function of a special unit.
    pub function: Option<UcumFunction>,
}

/// The UCUM essence data, loaded from `ucum-essence.xml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UcumEssence {
    version: String,
    revision: Option<String>,
    revision_date: Option<String>,
    prefixes: Vec<UcumPrefix>,
    base_units: Vec<UcumBaseUnit>,
    units: Vec<UcumUnit>,
    /// `Prefix.getValue()`, by prefix index: `new Decimal(value, 24)`.
    prefix_values: Vec<UcumDecimal>,
    /// `Value.getValue()`, by unit index.
    unit_values: Vec<UcumDecimal>,
    /// The first index of each code, which is what upstream's linear scans
    /// find.
    prefix_index: HashMap<String, usize>,
    base_unit_index: HashMap<String, usize>,
    unit_index: HashMap<String, usize>,
}

/// `UcumModel.getUnit`'s answer: a base unit or a defined unit, by index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum UnitRef {
    Base(usize),
    Defined(usize),
}

impl UcumEssence {
    /// Load the essence data from `path`.
    ///
    /// Every condition under which `XmlDefinitionsParser.parse` raises is an
    /// error naming the file (`path.quantity.016`).
    pub fn load(path: &Path) -> Result<UcumEssence, UcumLoadError> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| UcumLoadError::new(format!("cannot read the file: {e}")).at(path))?;
        UcumEssence::parse(&text).map_err(|e| e.at(path))
    }

    /// Read the essence data from the text of `ucum-essence.xml`.
    pub fn parse(text: &str) -> Result<UcumEssence, UcumLoadError> {
        let root = xml::parse_document(text).map_err(UcumLoadError::new)?;
        UcumEssence::from_root(&root)
    }

    /// The root element's `version` attribute, as `2.0.1` (`""` when
    /// absent, as `getAttribute` reads it). This is what a run reports beside
    /// its results (`path.quantity.016`).
    pub fn version(&self) -> &str {
        &self.version
    }

    /// The root element's `revision` attribute, as the file states it.
    pub fn revision(&self) -> Option<&str> {
        self.revision.as_deref()
    }

    /// The root element's `revision-date` attribute, as the file states it.
    pub fn revision_date(&self) -> Option<&str> {
        self.revision_date.as_deref()
    }

    /// Every prefix, in document order.
    pub fn prefixes(&self) -> &[UcumPrefix] {
        &self.prefixes
    }

    /// Every base unit, in document order.
    pub fn base_units(&self) -> &[UcumBaseUnit] {
        &self.base_units
    }

    /// Every defined unit, in document order.
    pub fn units(&self) -> &[UcumUnit] {
        &self.units
    }

    /// The first prefix with case-sensitive code `code`.
    pub fn prefix(&self, code: &str) -> Option<&UcumPrefix> {
        self.prefix_index.get(code).map(|&i| &self.prefixes[i])
    }

    /// The first base unit with case-sensitive code `code`.
    pub fn base_unit(&self, code: &str) -> Option<&UcumBaseUnit> {
        self.base_unit_index.get(code).map(|&i| &self.base_units[i])
    }

    /// The first defined unit with case-sensitive code `code`.
    pub fn unit(&self, code: &str) -> Option<&UcumUnit> {
        self.unit_index.get(code).map(|&i| &self.units[i])
    }

    /// `UcumModel.getUnit`: the first base unit with the code, else the first
    /// defined unit.
    pub(super) fn find_unit(&self, code: &str) -> Option<UnitRef> {
        if let Some(&i) = self.base_unit_index.get(code) {
            return Some(UnitRef::Base(i));
        }
        self.unit_index.get(code).map(|&i| UnitRef::Defined(i))
    }

    /// `Prefix.getValue()` of prefix `i`.
    pub(super) fn prefix_value(&self, i: usize) -> &UcumDecimal {
        &self.prefix_values[i]
    }

    /// `Value.getValue()` of defined unit `i`.
    pub(super) fn unit_value(&self, i: usize) -> &UcumDecimal {
        &self.unit_values[i]
    }

    /// `XmlDefinitionsParser.parse(InputStream)` after `parseDOM`.
    fn from_root(root: &xml::Element) -> Result<UcumEssence, UcumLoadError> {
        if root.name != "root" {
            return Err(UcumLoadError::new(format!(
                "Unable to process XML document: expected 'root' but found '{}'",
                root.name
            )));
        }
        let revision_date = root.attr("revision-date");
        parse_revision_date(revision_date.unwrap_or("")).map_err(|e| {
            UcumLoadError::new(format!(
                "the revision-date {:?} does not parse: {e}",
                revision_date.unwrap_or("")
            ))
        })?;

        let mut essence = UcumEssence {
            version: root.attr("version").unwrap_or("").to_owned(),
            revision: root.attr("revision").map(str::to_owned),
            revision_date: revision_date.map(str::to_owned),
            prefixes: Vec::new(),
            base_units: Vec::new(),
            units: Vec::new(),
            prefix_values: Vec::new(),
            unit_values: Vec::new(),
            prefix_index: HashMap::new(),
            base_unit_index: HashMap::new(),
            unit_index: HashMap::new(),
        };

        for child in root.elements() {
            match child.name.as_str() {
                "prefix" => {
                    let (prefix, value) = read_prefix(child)?;
                    let at = essence.prefixes.len();
                    essence
                        .prefix_index
                        .entry(prefix.code.clone())
                        .or_insert(at);
                    essence.prefixes.push(prefix);
                    essence.prefix_values.push(value);
                }
                "base-unit" => {
                    let base = read_base_unit(child)?;
                    let at = essence.base_units.len();
                    essence
                        .base_unit_index
                        .entry(base.code.clone())
                        .or_insert(at);
                    essence.base_units.push(base);
                }
                "unit" => {
                    let (unit, value) = read_unit(child)?;
                    let at = essence.units.len();
                    essence.unit_index.entry(unit.code.clone()).or_insert(at);
                    essence.units.push(unit);
                    essence.unit_values.push(value);
                }
                other => {
                    return Err(UcumLoadError::new(format!("unknown element name {other}")));
                }
            }
        }
        Ok(essence)
    }
}

/// `SimpleDateFormat.parse` of `revision-date` (`XmlDefinitionsParser.java:79-88`):
/// longer than 25 characters, characters 7 to 31 as `yyyy-MM-dd HH:mm:ss Z`
/// (a value of 26 to 31 characters makes `substring(7, 32)` throw); otherwise
/// the whole value as `yyyy-MM-dd`.
///
/// The format is lenient, as a `SimpleDateFormat` is by default: a numeric
/// field is one or more ASCII digits after optional spaces or tabs, with no
/// range check, a pattern space matches any run of whitespace, and text after
/// the last field is ignored. The zone `Z` is read in its RFC 822 form
/// (`-0500`) or as text opening with `GMT`. That is the part of
/// `SimpleDateFormat` reproduced: a zone written as another name (`EST`), or
/// a signed or grouped number in a field, which the JDK may accept, is a load
/// failure here; the pinned file uses neither.
fn parse_revision_date(dt: &str) -> Result<(), String> {
    let units: Vec<u16> = dt.encode_utf16().collect();
    if units.len() > 25 {
        let Some(window) = units.get(7..32) else {
            return Err(format!("begin 7, end 32, length {}", units.len()));
        };
        let text = String::from_utf16_lossy(window);
        let b = text.as_bytes();
        let mut at = 0;
        for (field, sep) in [
            ("yyyy", b'-'),
            ("MM", b'-'),
            ("dd", b' '),
            ("HH", b':'),
            ("mm", b':'),
            ("ss", b' '),
        ] {
            date_number(b, &mut at, field)?;
            date_literal(b, &mut at, sep)?;
        }
        date_zone(b, at)
    } else {
        let b = dt.as_bytes();
        let mut at = 0;
        date_number(b, &mut at, "yyyy")?;
        date_literal(b, &mut at, b'-')?;
        date_number(b, &mut at, "MM")?;
        date_literal(b, &mut at, b'-')?;
        date_number(b, &mut at, "dd")
    }
}

fn date_number(b: &[u8], at: &mut usize, field: &str) -> Result<(), String> {
    while matches!(b.get(*at), Some(b' ' | b'\t')) {
        *at += 1;
    }
    let start = *at;
    while matches!(b.get(*at), Some(c) if c.is_ascii_digit()) {
        *at += 1;
    }
    if *at == start {
        return Err(format!("no number for {field} at {start}"));
    }
    Ok(())
}

fn date_literal(b: &[u8], at: &mut usize, sep: u8) -> Result<(), String> {
    if sep == b' ' {
        while matches!(b.get(*at), Some(c) if c.is_ascii_whitespace()) {
            *at += 1;
        }
        return Ok(());
    }
    if b.get(*at) == Some(&sep) {
        *at += 1;
        Ok(())
    } else {
        Err(format!("expected {:?} at {}", char::from(sep), *at))
    }
}

fn date_zone(b: &[u8], at: usize) -> Result<(), String> {
    let rest = b.get(at..).unwrap_or(&[]);
    let rfc822 = rest.len() >= 5
        && matches!(rest[0], b'+' | b'-')
        && rest[1..5].iter().all(u8::is_ascii_digit);
    if rfc822 || rest.starts_with(b"GMT") {
        return Ok(());
    }
    Err(format!("no time zone at {at}"))
}

/// `getAttribute`, which is `""` for an absent attribute.
fn attr<'a>(element: &'a xml::Element, name: &str) -> &'a str {
    element.attr(name).unwrap_or("")
}

/// `"yes".equals(x.getAttribute(name))`.
fn yes(element: &xml::Element, name: &str) -> bool {
    element.attr(name) == Some("yes")
}

/// `XmlUtils.getNamedChildText`, trimmed.
fn child_text(element: &xml::Element, name: &str) -> Option<String> {
    element
        .elements()
        .find(|c| c.name == name)
        .map(|c| c.text().trim().to_owned())
}

/// `XmlUtils.getNamedChild(x, "value")`, which upstream dereferences: a
/// missing `value` is a `NullPointerException` that fails the load.
fn value_element(element: &xml::Element) -> Result<&xml::Element, UcumLoadError> {
    element
        .elements()
        .find(|c| c.name == "value")
        .ok_or_else(|| {
            UcumLoadError::new(format!(
                "<{} Code={:?}> has no <value>",
                element.name,
                attr(element, "Code")
            ))
        })
}

fn bad_value(element: &xml::Element, text: &str, why: &Failure) -> UcumLoadError {
    let why = match why {
        Failure::Ucum(m) => m.clone(),
        Failure::Error(m) => (*m).to_owned(),
    };
    UcumLoadError::new(format!(
        "<{} Code={:?}> has the value {text:?}: {why}",
        element.name,
        attr(element, "Code")
    ))
}

/// `parsePrefix`: `new Decimal(value/@value, 24)`.
fn read_prefix(element: &xml::Element) -> Result<(UcumPrefix, UcumDecimal), UcumLoadError> {
    let value = value_element(element)?;
    let text = attr(value, "value");
    let decimal =
        UcumDecimal::with_precision(text, 24).map_err(|e| bad_value(element, text, &e))?;
    Ok((
        UcumPrefix {
            code: attr(element, "Code").to_owned(),
            code_upper: attr(element, "CODE").to_owned(),
            name: child_text(element, "name").unwrap_or_default(),
            print_symbol: child_text(element, "printSymbol"),
            value: text.to_owned(),
        },
        decimal,
    ))
}

/// `parseBaseUnit`: `getAttribute("dim").charAt(0)` fails on an absent or
/// empty `dim`.
fn read_base_unit(element: &xml::Element) -> Result<UcumBaseUnit, UcumLoadError> {
    let dim = attr(element, "dim");
    if dim.is_empty() {
        return Err(UcumLoadError::new(format!(
            "<base-unit Code={:?}> has no dim",
            attr(element, "Code")
        )));
    }
    Ok(UcumBaseUnit {
        code: attr(element, "Code").to_owned(),
        code_upper: attr(element, "CODE").to_owned(),
        dim: dim.to_owned(),
        name: child_text(element, "name").unwrap_or_default(),
        property: child_text(element, "property"),
    })
}

/// `parseUnit` and `parseValue`: a value with a `.` is read with precision
/// 24, any other with its own.
fn read_unit(element: &xml::Element) -> Result<(UcumUnit, UcumDecimal), UcumLoadError> {
    let value = value_element(element)?;
    let text = attr(value, "value");
    let decimal = if text.contains('.') {
        UcumDecimal::with_precision(text, 24)
    } else {
        UcumDecimal::new(text)
    }
    .map_err(|e| bad_value(element, text, &e))?;
    let function = value
        .elements()
        .find(|c| c.name == "function")
        .map(|f| UcumFunction {
            name: attr(f, "name").to_owned(),
            value: attr(f, "value").to_owned(),
            unit: attr(f, "Unit").to_owned(),
        });
    Ok((
        UcumUnit {
            code: attr(element, "Code").to_owned(),
            code_upper: element.attr("CODE").map(str::to_owned),
            is_metric: yes(element, "isMetric"),
            is_special: yes(element, "isSpecial"),
            is_arbitrary: yes(element, "isArbitrary"),
            class: element.attr("class").map(str::to_owned),
            names: element
                .elements()
                .filter(|c| c.name == "name")
                .map(|c| c.text().trim().to_owned())
                .collect(),
            print_symbol: child_text(element, "printSymbol"),
            property: child_text(element, "property"),
            value_unit: attr(value, "Unit").to_owned(),
            value_unit_upper: value.attr("UNIT").map(str::to_owned),
            value: value.attr("value").map(str::to_owned),
            function,
        },
        decimal,
    ))
}

/// A minimal, strict XML reader: enough for the essence file and strict
/// enough that a file which is not well-formed is refused rather than read in
/// part. No DTD, no external entities, only the five predefined entities and
/// character references.
mod xml {
    #[derive(Debug)]
    pub(super) struct Element {
        pub(super) name: String,
        attrs: Vec<(String, String)>,
        children: Vec<Node>,
    }

    #[derive(Debug)]
    enum Node {
        Element(Element),
        Text(String),
    }

    impl Element {
        pub(super) fn attr(&self, name: &str) -> Option<&str> {
            self.attrs
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, v)| v.as_str())
        }

        pub(super) fn elements(&self) -> impl Iterator<Item = &Element> {
            self.children.iter().filter_map(|c| match c {
                Node::Element(e) => Some(e),
                Node::Text(_) => None,
            })
        }

        /// The text content, markup removed.
        pub(super) fn text(&self) -> String {
            let mut out = String::new();
            self.collect_text(&mut out);
            out
        }

        fn collect_text(&self, out: &mut String) {
            for child in &self.children {
                match child {
                    Node::Text(t) => out.push_str(t),
                    Node::Element(e) => e.collect_text(out),
                }
            }
        }
    }

    struct Reader<'a> {
        src: &'a str,
        pos: usize,
    }

    /// Parse a whole document and return its root element.
    pub(super) fn parse_document(src: &str) -> Result<Element, String> {
        let src = src.strip_prefix('\u{feff}').unwrap_or(src);
        let mut r = Reader { src, pos: 0 };
        r.skip_misc()?;
        if r.rest().starts_with("<!DOCTYPE") {
            return Err(r.error("a DOCTYPE is not accepted"));
        }
        if !r.rest().starts_with('<') {
            return Err(r.error("the file is not XML: no root element"));
        }
        let root = r.element()?;
        r.skip_misc()?;
        if !r.rest().is_empty() {
            return Err(r.error("content after the root element"));
        }
        Ok(root)
    }

    impl<'a> Reader<'a> {
        fn rest(&self) -> &'a str {
            &self.src[self.pos..]
        }

        fn error(&self, what: &str) -> String {
            let line = self.src[..self.pos].matches('\n').count() + 1;
            format!("not well-formed XML at line {line}: {what}")
        }

        fn eat(&mut self, s: &str) -> bool {
            if self.rest().starts_with(s) {
                self.pos += s.len();
                true
            } else {
                false
            }
        }

        fn skip_ws(&mut self) {
            let trimmed = self.rest().trim_start_matches([' ', '\t', '\r', '\n']);
            self.pos = self.src.len() - trimmed.len();
        }

        fn skip_until(&mut self, end: &str, what: &str) -> Result<(), String> {
            match self.rest().find(end) {
                Some(i) => {
                    self.pos += i + end.len();
                    Ok(())
                }
                None => Err(self.error(&format!("unterminated {what}"))),
            }
        }

        /// Whitespace, comments and processing instructions (including the
        /// XML declaration) outside the root element.
        fn skip_misc(&mut self) -> Result<(), String> {
            loop {
                self.skip_ws();
                if self.eat("<?") {
                    self.skip_until("?>", "processing instruction")?;
                } else if self.eat("<!--") {
                    self.skip_until("-->", "comment")?;
                } else {
                    return Ok(());
                }
            }
        }

        fn name(&mut self) -> Result<String, String> {
            let rest = self.rest();
            let end = rest
                .find(|c: char| !(c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | ':')))
                .unwrap_or(rest.len());
            let name = &rest[..end];
            if name.is_empty() || name.starts_with(['-', '.']) || name.starts_with(char::is_numeric)
            {
                return Err(self.error("expected a name"));
            }
            self.pos += end;
            Ok(name.to_owned())
        }

        fn element(&mut self) -> Result<Element, String> {
            if !self.eat("<") {
                return Err(self.error("expected '<'"));
            }
            let name = self.name()?;
            let mut attrs: Vec<(String, String)> = Vec::new();
            loop {
                let before = self.pos;
                self.skip_ws();
                if self.eat("/>") {
                    return Ok(Element {
                        name,
                        attrs,
                        children: Vec::new(),
                    });
                }
                if self.eat(">") {
                    break;
                }
                if self.pos == before {
                    return Err(self.error("expected whitespace before an attribute"));
                }
                let attr = self.name()?;
                self.skip_ws();
                if !self.eat("=") {
                    return Err(self.error(&format!("attribute {attr} has no '='")));
                }
                self.skip_ws();
                let quote = match self.rest().chars().next() {
                    Some(q @ ('"' | '\'')) => q,
                    _ => return Err(self.error(&format!("attribute {attr} is not quoted"))),
                };
                self.pos += 1;
                let end = self
                    .rest()
                    .find(quote)
                    .ok_or_else(|| self.error(&format!("attribute {attr} is not closed")))?;
                let raw = &self.rest()[..end];
                if raw.contains('<') {
                    return Err(self.error(&format!("'<' in the value of attribute {attr}")));
                }
                let value = self.unescape(raw)?;
                self.pos += end + 1;
                if attrs.iter().any(|(n, _)| *n == attr) {
                    return Err(self.error(&format!("attribute {attr} is repeated")));
                }
                attrs.push((attr, value));
            }

            let mut children = Vec::new();
            loop {
                if self.eat("</") {
                    let close = self.name()?;
                    self.skip_ws();
                    if !self.eat(">") {
                        return Err(self.error("expected '>' after an end tag"));
                    }
                    if close != name {
                        return Err(self.error(&format!("</{close}> closes <{name}>")));
                    }
                    return Ok(Element {
                        name,
                        attrs,
                        children,
                    });
                }
                if self.eat("<!--") {
                    self.skip_until("-->", "comment")?;
                } else if self.eat("<![CDATA[") {
                    let end = self
                        .rest()
                        .find("]]>")
                        .ok_or_else(|| self.error("unterminated CDATA section"))?;
                    children.push(Node::Text(self.rest()[..end].to_owned()));
                    self.pos += end + 3;
                } else if self.eat("<?") {
                    self.skip_until("?>", "processing instruction")?;
                } else if self.rest().starts_with('<') {
                    children.push(Node::Element(self.element()?));
                } else if self.rest().is_empty() {
                    return Err(self.error(&format!("<{name}> is not closed")));
                } else {
                    let end = self.rest().find('<').unwrap_or(self.rest().len());
                    let raw = &self.rest()[..end];
                    let text = self.unescape(raw)?;
                    self.pos += end;
                    children.push(Node::Text(text));
                }
            }
        }

        fn unescape(&self, raw: &str) -> Result<String, String> {
            if !raw.contains('&') {
                return Ok(raw.to_owned());
            }
            let mut out = String::with_capacity(raw.len());
            let mut rest = raw;
            while let Some(i) = rest.find('&') {
                out.push_str(&rest[..i]);
                rest = &rest[i + 1..];
                let end = rest
                    .find(';')
                    .ok_or_else(|| self.error("an entity reference has no ';'"))?;
                let entity = &rest[..end];
                rest = &rest[end + 1..];
                let ch = match entity {
                    "lt" => '<',
                    "gt" => '>',
                    "amp" => '&',
                    "quot" => '"',
                    "apos" => '\'',
                    _ => {
                        let code = if let Some(hex) = entity
                            .strip_prefix("#x")
                            .or_else(|| entity.strip_prefix("#X"))
                        {
                            u32::from_str_radix(hex, 16).ok()
                        } else if let Some(dec) = entity.strip_prefix('#') {
                            dec.parse::<u32>().ok()
                        } else {
                            return Err(self.error(&format!("unknown entity &{entity};")));
                        };
                        code.and_then(char::from_u32).ok_or_else(|| {
                            self.error(&format!("invalid character reference &{entity};"))
                        })?
                    }
                };
                out.push(ch);
            }
            out.push_str(rest);
            Ok(out)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL: &str = r#"<?xml version="1.0" encoding="ascii"?>
<root xmlns="http://unitsofmeasure.org/ucum-essence" version="2.0.1"
      revision="$Revision: 439 $">
   <!-- a comment -->
   <prefix Code="k" CODE="K">
      <name>kilo</name>
      <printSymbol>k</printSymbol>
      <value value="1e3">1 &#215; 10<sup>3</sup>
      </value>
   </prefix>
   <base-unit Code="m" CODE="M" dim="L">
      <name>meter</name>
      <printSymbol>m</printSymbol>
      <property>length</property>
   </base-unit>
   <base-unit Code="K" CODE="K" dim="C">
      <name>kelvin</name>
      <property>temperature</property>
   </base-unit>
   <unit Code="[in_i]" CODE="[IN_I]" isMetric="no" class="intcust">
      <name>inch</name>
      <printSymbol>in</printSymbol>
      <property>length</property>
      <value Unit="cm" UNIT="CM" value="254e-2">2.54</value>
   </unit>
   <unit Code="Cel" CODE="CEL" isMetric="yes" isSpecial="yes" class="si">
      <name>degree Celsius</name>
      <printSymbol>&#176;C</printSymbol>
      <property>temperature</property>
      <value Unit="cel(1 K)" UNIT="CEL(1 K)">
         <function name="Cel" value="1" Unit="K"/>
      </value>
   </unit>
</root>"#;

    // covers: path.quantity.016
    #[test]
    fn reads_the_version_and_the_three_tables() {
        let essence = UcumEssence::parse(MINIMAL).unwrap();
        assert_eq!(essence.version(), "2.0.1");
        assert_eq!(essence.revision(), Some("$Revision: 439 $"));
        assert_eq!(essence.revision_date(), None);

        let kilo = essence.prefix("k").unwrap();
        assert_eq!(kilo.code_upper, "K");
        assert_eq!(kilo.name, "kilo");
        assert_eq!(kilo.value, "1e3");

        let metre = essence.base_unit("m").unwrap();
        assert_eq!(metre.dim, "L");
        assert_eq!(metre.property.as_deref(), Some("length"));
        assert!(essence.base_unit("s").is_none());

        let inch = essence.unit("[in_i]").unwrap();
        assert!(!inch.is_metric && !inch.is_special && !inch.is_arbitrary);
        assert_eq!(inch.value_unit, "cm");
        assert_eq!(inch.value.as_deref(), Some("254e-2"));
        assert_eq!(inch.names, ["inch"]);

        let celsius = essence.unit("Cel").unwrap();
        assert!(celsius.is_metric && celsius.is_special);
        assert_eq!(celsius.print_symbol.as_deref(), Some("\u{b0}C"));
        assert_eq!(celsius.value, None);
        assert_eq!(
            celsius.function,
            Some(UcumFunction {
                name: "Cel".into(),
                value: "1".into(),
                unit: "K".into()
            })
        );
    }

    // covers: path.quantity.016
    #[test]
    fn the_pinned_essence_file_loads_completely() {
        // Fetched, not vendored (`design.md` section 5): `bash oracles/fetch.sh`.
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../oracles/fhir-test-cases/fetched/ucum/ucum-essence.xml");
        let essence = UcumEssence::load(&path).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(essence.version(), "2.0.1");
        assert_eq!(essence.prefixes().len(), 24);
        assert_eq!(essence.base_units().len(), 7);
        assert_eq!(essence.units().len(), 303);
        assert_eq!(essence.unit("[pi]").unwrap().value_unit, "1");
        assert_eq!(essence.unit("L").unwrap().code_upper, None);
        assert!(essence.units().iter().filter(|u| u.is_special).count() >= 20);
        assert!(essence.unit("[m/s2/Hz^(1/2)]").unwrap().is_special);
        assert_eq!(
            essence.prefix("u").unwrap().print_symbol.as_deref(),
            Some("\u{3bc}")
        );
    }

    // covers: path.quantity.016
    #[test]
    fn a_missing_file_names_the_path() {
        let path = Path::new("does/not/exist/ucum-essence.xml");
        let err = UcumEssence::load(path).unwrap_err();
        assert_eq!(err.path(), Some(path));
        assert!(err.to_string().contains("ucum-essence.xml"), "{err}");
        assert!(!err.message().is_empty());
    }

    // covers: path.quantity.016
    #[test]
    fn xml_that_is_not_ucum_is_refused() {
        for (text, needle) in [
            ("<tests name=\"x\"/>", "not the UCUM essence"),
            ("<root version=\"1\"/>", "namespace"),
            (
                "<root xmlns=\"http://example.org\" version=\"1\"/>",
                "namespace",
            ),
            (
                "<root xmlns=\"http://unitsofmeasure.org/ucum-essence\"/>",
                "version",
            ),
            (
                "<root xmlns=\"http://unitsofmeasure.org/ucum-essence\" version=\"2\"/>",
                "no base units",
            ),
        ] {
            let err = UcumEssence::parse(text).unwrap_err();
            assert!(err.message().contains(needle), "{text}: {err}");
        }
    }

    // covers: path.quantity.016
    #[test]
    fn malformed_xml_is_refused() {
        for text in [
            "",
            "not xml",
            "<root>",
            "<root></toor>",
            "<root a=\"1\" a=\"2\"/>",
            "<root a=1/>",
            "<root>&nbsp;</root>",
            "<root/><root/>",
            "<!DOCTYPE root><root/>",
        ] {
            let err = UcumEssence::parse(text).unwrap_err();
            assert!(err.message().contains("XML"), "{text:?}: {err}");
        }
    }

    // covers: path.quantity.016
    #[test]
    fn a_unit_with_a_bad_flag_or_no_value_is_refused() {
        let bad_flag = MINIMAL.replace("isMetric=\"no\"", "isMetric=\"maybe\"");
        assert!(
            UcumEssence::parse(&bad_flag)
                .unwrap_err()
                .message()
                .contains("isMetric")
        );
        let no_value = MINIMAL.replace(
            "<value Unit=\"cm\" UNIT=\"CM\" value=\"254e-2\">2.54</value>",
            "",
        );
        assert!(
            UcumEssence::parse(&no_value)
                .unwrap_err()
                .message()
                .contains("no <value>")
        );
        let twice = MINIMAL.replace("Code=\"Cel\"", "Code=\"[in_i]\"");
        assert!(
            UcumEssence::parse(&twice)
                .unwrap_err()
                .message()
                .contains("twice")
        );
    }
}
