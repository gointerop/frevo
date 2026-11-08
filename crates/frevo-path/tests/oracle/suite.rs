//! The pinned FHIRPath suite, read by XML parse.
//!
//! Source: `oracles/fhirpath/tests-fhir-r4.xml`, vendored from
//! `FHIR/fhir-test-cases` at `c95adbee099c875de97d25ed7616aae7d66dd85b`
//! (`oracles/PINS.md`). Read-only: the harness never writes to it
//! (`path.oracle.015`).
//!
//! Counting is by XML parse and never by text match. A `grep -c '<group '`
//! over the pinned file answers 104 and a `grep -o '<test '` answers 1,193,
//! because the `testDollarResource` block at `tests-fhir-r4.xml:1576-1582`
//! sits inside an XML comment and its bodies are leftover Java text that still
//! contains `<test ...>`. The real figures are 103 and 1,191.
//!
//! Attribute handling is exhaustive by construction (`path.oracle.002`): each
//! element has a closed set of attributes it may carry, and an attribute
//! outside that set — or a value outside a recorded domain — fails the load.
//! An unread attribute is a silently weakened oracle.

use super::xml::{self, Element};

/// The suite's expected shape. A load that disagrees is a re-pin that has not
/// been reflected in the harness, and it fails rather than grading fewer cases
/// than it reports.
pub const EXPECTED_GROUPS: usize = 103;
pub const EXPECTED_CASES: usize = 1191;
pub const EXPECTED_OUTPUTS: usize = 1103;
pub const EXPECTED_SYNTAX_INVALID: usize = 7;
pub const EXPECTED_SEMANTIC_INVALID: usize = 29;
pub const EXPECTED_EXECUTION_INVALID: usize = 27;

/// Repo-relative path of the vendored suite, as `oracles/PINS.md` records it.
pub const SUITE_PATH: &str = "oracles/fhirpath/tests-fhir-r4.xml";

/// The directory `TestingUtilities.loadTestResource("r4", input)` resolves to
/// in `org.hl7.fhir.r4.test.FHIRPathTests` (`path.oracle.003`).
pub const INPUT_DIR: &str = "oracles/fhir-test-cases/fetched/r4";

// ---------------------------------------------------------------------------
// The closed attribute inventory
// ---------------------------------------------------------------------------

pub const TESTS_ATTRS: &[&str] = &["name", "description", "reference"];
pub const GROUP_ATTRS: &[&str] = &["name", "description", "mode", "reference"];
pub const TEST_ATTRS: &[&str] = &[
    "name",
    "description",
    "inputfile",
    "mode",
    "context",
    "contextType",
    "version",
    "testing",
    "predicate",
    "ordered",
    "checkOrderedFunctions",
];
pub const EXPRESSION_ATTRS: &[&str] = &["invalid", "mode"];
pub const OUTPUT_ATTRS: &[&str] = &["type"];
/// `<notes>` is the sixth element the pinned file carries — two of them, both
/// children of a `<group>` (`tests-fhir-r4.xml:1584` and `:1819`). It carries
/// no attributes, so its inventory is empty; it is listed rather than skipped
/// so that an attribute appearing on it later would fail the load.
pub const NOTES_ATTRS: &[&str] = &[];

// The value domains the pinned file uses. A value outside one of these fails
// the load, which the harness reports as `did-not-run` and never as `passed`.
pub const EXPRESSION_INVALID_DOMAIN: &[&str] = &["syntax", "semantic", "execution"];
pub const EXPRESSION_MODE_DOMAIN: &[&str] = &["strict"];
pub const TEST_MODE_DOMAIN: &[&str] = &["element", "strict"];
pub const GROUP_MODE_DOMAIN: &[&str] = &["custom", "semantic"];
pub const TEST_PREDICATE_DOMAIN: &[&str] = &["true", "false"];
pub const TEST_ORDERED_DOMAIN: &[&str] = &["false"];
pub const TEST_CHECK_ORDERED_FUNCTIONS_DOMAIN: &[&str] = &["true"];
pub const TEST_VERSION_DOMAIN: &[&str] = &["2.1.0"];

/// `test/@testing` is recorded and reported rather than domain-checked at load
/// time, because the spec delta states no domain for it; it is what drives
/// `path.oracle.014`'s out-of-scope reporting. The observed set is asserted by
/// a test instead, so a re-pin that adds a value is visible without aborting a
/// load the delta does not require to abort.
pub const OBSERVED_TESTING_VALUES: &[&str] = &[
    "entry",
    "entryIn",
    "hasCoding",
    "is",
    "resources",
    "resourcesIn",
    "sameSubject",
    "section",
];

/// The seven document functions the R4 `ExpressionNode.Function` enum does not
/// implement, which is why the `documentFunctions` group is out of scope
/// (`path.oracle.014`, `design.md` section 4 open question 1).
pub const UNIMPLEMENTED_DOCUMENT_FUNCTIONS: &[&str] = &[
    "section",
    "entry",
    "entryIn",
    "resources",
    "resourcesIn",
    "sameSubject",
    "hasCoding",
];

// ---------------------------------------------------------------------------
// The model
// ---------------------------------------------------------------------------

/// Which stage a case expects to fail at, from `expression/@invalid`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Parse,
    Check,
    Evaluate,
}

impl Stage {
    pub fn as_str(self) -> &'static str {
        match self {
            Stage::Parse => "parse",
            Stage::Check => "check",
            Stage::Evaluate => "evaluate",
        }
    }

    /// `expression/@invalid` spells the stages `syntax`, `semantic` and
    /// `execution`, exactly as `FHIRPathTests.TestResultType` reads them.
    fn from_invalid(v: &str) -> Option<Stage> {
        match v {
            "syntax" => Some(Stage::Parse),
            "semantic" => Some(Stage::Check),
            "execution" => Some(Stage::Evaluate),
            _ => None,
        }
    }
}

/// One expected `<output>`.
///
/// Both fields collapse absent with empty, because `Utilities.noString` does:
/// an empty `type` means "do not check the type" and an empty body means "do
/// not check the value" (`path.oracle.008`).
#[derive(Debug, Clone)]
pub struct Output {
    pub type_name: String,
    pub body: String,
}

impl Output {
    pub fn checks_type(&self) -> bool {
        !self.type_name.is_empty()
    }
    pub fn checks_value(&self) -> bool {
        !self.body.is_empty()
    }
}

/// A case's address. `group/name` is **not** unique in the pinned file —
/// `testEquivalent/testEquivalent23` appears twice, at `tests-fhir-r4.xml:986`
/// and `:987` — so the ordinal is part of the address. The pinned Java harness
/// disambiguates the same way, with the element's positional index inside its
/// group (`FHIRPathTests.getName`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CaseAddress {
    pub group: String,
    pub name: String,
    /// One-based position of the `<test>` among its group's child elements,
    /// matching `FHIRPathTests.getName`'s `[ndx + 1]`.
    pub ordinal: usize,
}

impl std::fmt::Display for CaseAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}[{}]", self.group, self.name, self.ordinal)
    }
}

#[derive(Debug, Clone)]
pub struct Case {
    pub address: CaseAddress,
    pub description: String,
    pub expression: String,
    /// `expression/@invalid`, mapped to the stage it names.
    pub expected_failure: Option<Stage>,
    /// `expression/@mode`. Recorded, and changes no grading
    /// (`path.oracle.017`).
    pub expression_mode: Option<String>,
    pub inputfile: Option<String>,
    /// `test/@mode`: `element` or `strict`.
    pub test_mode: Option<String>,
    pub context: Option<String>,
    /// `test/@contextType`, split on `|`; a case may name several types
    /// (`path.check.012`).
    pub context_types: Vec<String>,
    pub version: Option<String>,
    /// `test/@testing` — the function the case is testing, which is what marks
    /// the `documentFunctions` cases out of scope.
    pub testing: Option<String>,
    /// `test/@predicate == "true"` (`path.oracle.006`).
    pub predicate: bool,
    /// `test/@ordered == "false"` selects the unordered comparison branch
    /// (`path.oracle.007`).
    pub unordered: bool,
    /// `test/@checkOrderedFunctions == "true"`.
    pub check_ordered_functions: bool,
    pub outputs: Vec<Output>,
    /// `group/@mode` of the group this case belongs to.
    pub group_mode: Option<String>,
    /// Byte offset of the case's `<test>` element in the suite file, for
    /// reporting.
    pub offset: usize,
}

impl Case {
    /// Whether the case exercises a function the R4 engine does not implement.
    pub fn out_of_scope_function(&self) -> Option<&str> {
        let t = self.testing.as_deref()?;
        if UNIMPLEMENTED_DOCUMENT_FUNCTIONS.contains(&t) {
            Some(t)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct Group {
    pub name: String,
    pub description: Option<String>,
    pub mode: Option<String>,
    pub reference: Option<String>,
    pub notes: Vec<String>,
    pub cases: Vec<Case>,
}

#[derive(Debug, Clone)]
pub struct Suite {
    pub name: String,
    pub description: Option<String>,
    /// `tests/@reference` — recorded and reported, changes no grading.
    pub reference: Option<String>,
    pub groups: Vec<Group>,
    /// Every attribute seen, as `element/@name` with its occurrence count.
    /// Reported so a run can state that it read every attribute the file
    /// carries.
    pub attribute_inventory: Vec<(String, usize)>,
}

impl Suite {
    pub fn cases(&self) -> impl Iterator<Item = &Case> {
        self.groups.iter().flat_map(|g| g.cases.iter())
    }

    pub fn case_count(&self) -> usize {
        self.groups.iter().map(|g| g.cases.len()).sum()
    }

    pub fn output_count(&self) -> usize {
        self.cases().map(|c| c.outputs.len()).sum()
    }

    pub fn invalid_count(&self, stage: Stage) -> usize {
        self.cases()
            .filter(|c| c.expected_failure == Some(stage))
            .count()
    }

    pub fn case(&self, group: &str, name: &str) -> Option<&Case> {
        let mut found = self
            .cases()
            .filter(|c| c.address.group == group && c.address.name == name);
        let first = found.next();
        match found.next() {
            // An ambiguous address must not silently resolve to the first
            // match: `testEquivalent/testEquivalent23` is two different
            // expressions.
            Some(_) => None,
            None => first,
        }
    }

    /// Every `inputfile` the suite references, deduplicated, in first-seen
    /// order.
    pub fn referenced_inputs(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for c in self.cases() {
            if let Some(f) = &c.inputfile
                && !out.iter().any(|x| x == f)
            {
                out.push(f.clone());
            }
        }
        out
    }
}

/// A load failure. Reported as `did-not-run` for every case; never `passed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadError {
    pub message: String,
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

fn bad(message: impl Into<String>) -> LoadError {
    LoadError {
        message: message.into(),
    }
}

fn check_attrs(
    el: &Element,
    allowed: &[&str],
    inventory: &mut Vec<(String, usize)>,
) -> Result<(), LoadError> {
    for (k, _) in &el.attrs {
        if !allowed.contains(&k.as_str()) {
            return Err(bad(format!(
                "element <{}> at byte {} carries attribute '{}', which this element does not \
                 carry in the pinned file; allowed: {:?}. An unread attribute is a silently \
                 weakened oracle, so the run fails rather than ignoring it.",
                el.name, el.offset, k, allowed
            )));
        }
        let key = format!("{}/@{}", el.name, k);
        match inventory.iter_mut().find(|(n, _)| *n == key) {
            Some((_, n)) => *n += 1,
            None => inventory.push((key, 1)),
        }
    }
    Ok(())
}

fn check_domain(el: &Element, attr: &str, domain: &[&str]) -> Result<Option<String>, LoadError> {
    if !el.has_attr(attr) {
        return Ok(None);
    }
    let v = el.attr(attr);
    if !domain.contains(&v) {
        return Err(bad(format!(
            "element <{}> at byte {} has {attr}=\"{v}\", outside the recorded domain {domain:?}",
            el.name, el.offset
        )));
    }
    Ok(Some(v.to_string()))
}

fn opt(el: &Element, attr: &str) -> Option<String> {
    if el.has_attr(attr) {
        Some(el.attr(attr).to_string())
    } else {
        None
    }
}

/// Parse the suite from its XML text.
pub fn load(src: &str) -> Result<Suite, LoadError> {
    let doc = xml::parse(src).map_err(|e| bad(format!("{e}")))?;
    let root = doc.root;
    if root.name != "tests" {
        return Err(bad(format!(
            "root element is <{}>, expected <tests>",
            root.name
        )));
    }
    let mut inventory: Vec<(String, usize)> = Vec::new();
    check_attrs(&root, TESTS_ATTRS, &mut inventory)?;

    let mut groups = Vec::new();
    for child in root.child_elements() {
        if child.name != "group" {
            return Err(bad(format!(
                "unexpected element <{}> at byte {} directly inside <tests>",
                child.name, child.offset
            )));
        }
        check_attrs(child, GROUP_ATTRS, &mut inventory)?;
        let group_mode = check_domain(child, "mode", GROUP_MODE_DOMAIN)?;
        let group_name = child.attr("name").to_string();
        if group_name.is_empty() {
            return Err(bad(format!("<group> at byte {} has no name", child.offset)));
        }

        let mut notes = Vec::new();
        let mut cases = Vec::new();
        for (idx, member) in child.child_elements().iter().enumerate() {
            match member.name.as_str() {
                "notes" => {
                    check_attrs(member, NOTES_ATTRS, &mut inventory)?;
                    notes.push(member.text_content());
                }
                "test" => {
                    cases.push(load_case(
                        member,
                        idx + 1,
                        &group_name,
                        group_mode.as_deref(),
                        &mut inventory,
                    )?);
                }
                other => {
                    return Err(bad(format!(
                        "unexpected element <{other}> at byte {} inside <group name=\"{group_name}\">",
                        member.offset
                    )));
                }
            }
        }

        groups.push(Group {
            name: group_name,
            description: opt(child, "description"),
            mode: group_mode,
            reference: opt(child, "reference"),
            notes,
            cases,
        });
    }

    inventory.sort();
    Ok(Suite {
        name: root.attr("name").to_string(),
        description: opt(&root, "description"),
        reference: opt(&root, "reference"),
        groups,
        attribute_inventory: inventory,
    })
}

fn load_case(
    el: &Element,
    ordinal: usize,
    group: &str,
    group_mode: Option<&str>,
    inventory: &mut Vec<(String, usize)>,
) -> Result<Case, LoadError> {
    check_attrs(el, TEST_ATTRS, inventory)?;
    let name = el.attr("name").to_string();

    let exprs = el.children_named("expression");
    if exprs.len() != 1 {
        return Err(bad(format!(
            "<test name=\"{name}\"> at byte {} has {} <expression> children, expected exactly 1",
            el.offset,
            exprs.len()
        )));
    }
    let expr_el = exprs[0];
    check_attrs(expr_el, EXPRESSION_ATTRS, inventory)?;
    let invalid = check_domain(expr_el, "invalid", EXPRESSION_INVALID_DOMAIN)?;
    let expression_mode = check_domain(expr_el, "mode", EXPRESSION_MODE_DOMAIN)?;

    let mut outputs = Vec::new();
    for out_el in el.children_named("output") {
        check_attrs(out_el, OUTPUT_ATTRS, inventory)?;
        outputs.push(Output {
            type_name: out_el.attr("type").to_string(),
            body: out_el.text_content(),
        });
    }

    for member in el.child_elements() {
        if member.name != "expression" && member.name != "output" {
            return Err(bad(format!(
                "unexpected element <{}> at byte {} inside <test name=\"{name}\">",
                member.name, member.offset
            )));
        }
    }

    let predicate = check_domain(el, "predicate", TEST_PREDICATE_DOMAIN)?;
    let ordered = check_domain(el, "ordered", TEST_ORDERED_DOMAIN)?;
    let cof = check_domain(
        el,
        "checkOrderedFunctions",
        TEST_CHECK_ORDERED_FUNCTIONS_DOMAIN,
    )?;
    let test_mode = check_domain(el, "mode", TEST_MODE_DOMAIN)?;
    let version = check_domain(el, "version", TEST_VERSION_DOMAIN)?;

    let context_types = match opt(el, "contextType") {
        Some(v) => v.split('|').map(|s| s.to_string()).collect(),
        None => Vec::new(),
    };

    Ok(Case {
        address: CaseAddress {
            group: group.to_string(),
            name,
            ordinal,
        },
        description: el.attr("description").to_string(),
        expression: expr_el.text_content(),
        expected_failure: invalid.as_deref().and_then(Stage::from_invalid),
        expression_mode,
        inputfile: opt(el, "inputfile"),
        test_mode,
        context: opt(el, "context"),
        context_types,
        version,
        testing: opt(el, "testing"),
        predicate: predicate.as_deref() == Some("true"),
        unordered: ordered.as_deref() == Some("false"),
        check_ordered_functions: cof.as_deref() == Some("true"),
        outputs,
        group_mode: group_mode.map(|s| s.to_string()),
        offset: el.offset,
    })
}
