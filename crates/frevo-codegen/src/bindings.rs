//! Required-binding enumerations, and the naming rule that produces their
//! variants.
//!
//! Tasks 11 and 12 of `add-fhirpath-and-codegen-r4`. Two things live here
//! because they are one decision seen from two sides: which coded elements get
//! a closed enumeration (`codegen.binding.001` to `codegen.binding.004`), and
//! what the variants of one are called (`codegen.naming.005`).
//!
//! # Only a `required` binding, and only a resolvable one
//!
//! R4 binds 224 distinct value sets `required` across the 212 core type
//! definitions. 220 of them expand from the pinned package alone; the other
//! four cannot, and are **reported rather than guessed**:
//!
//! | Value set | Why it does not expand |
//! |---|---|
//! | `currencies` | its code system is `urn:iso:std:iso:4217`, which the package does not ship |
//! | `mimetypes` | its code system is `urn:ietf:bcp:13` |
//! | `ucum-units` | its code system is `http://unitsofmeasure.org` |
//! | `http://loinc.org/vs/LL379-9` | the value set itself is not in the package — LOINC content is licensed and not redistributable |
//!
//! Those elements stay **open**, which is what `codegen.binding.004`
//! requires, and each appears in [`Resolution::unresolved`] with its element
//! path and the URL that could not be resolved. A binding weaker than
//! `required` — extensible, preferred, example — never generates an
//! enumeration at all (`codegen.binding.003`), and the element's declared
//! binding is still recorded so that a consumer can see what it was.
//!
//! # What "resolvable from the package" means exactly
//!
//! An expansion here is deliberately narrow: no terminology server, no
//! transitive closure, no filter evaluation. A value set expands when its
//! `compose` has no `exclude` and every `include` either lists its `concept`
//! codes outright or names one `system` the package ships as a `CodeSystem`
//! with `content = complete`. Anything else — a `filter`, an imported
//! `valueSet`, an incomplete code system, a code system that is not there —
//! leaves the binding unresolved. Guessing at a partial expansion would
//! produce a *closed* enumeration missing codes, which is strictly worse than
//! leaving the element open.
//!
//! # The variant naming rule
//!
//! Total, deterministic and collision-free, and the code string is carried
//! beside the variant so that serialisation returns it verbatim
//! (`codegen.naming.005`). Four cases, measured over all 2,066 codes behind
//! R4's required bindings:
//!
//! | Code | Variant | Rule |
//! |---|---|---|
//! | `entered-in-error` | `EnteredInError` | each alphanumeric run upper-cased on its first character and concatenated |
//! | `_special` | `Special` | a leading separator is a boundary like any other |
//! | `>=` | `GreaterEqual` | a code with no alphanumeric character is spelled from [`SYMBOL_NAMES`] |
//! | `0.5.0` | `N050` | a name that would start with a digit takes an `N` |
//!
//! Only the punctuation-only codes need the symbol table: measured over the
//! 220 resolvable value sets, the alphanumeric rule alone is already injective
//! on every one of them except `quantity-comparator` and
//! `questionnaire-enable-operator`, whose codes are `<`, `<=`, `>`, `>=`, `=`
//! and `!=`. [`code_variants`] still carries a deterministic disambiguator,
//! because a rule that is collision-free only by measurement is not
//! collision-free.

use std::collections::{BTreeMap, BTreeSet};

use crate::package::Package;
use crate::synthetic;

/// One variant of a generated enumeration, with the code it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    pub code: String,
    pub rust_name: String,
}

/// A closed enumeration, generated only for a resolvable `required` binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Enumeration {
    pub rust_name: String,
    /// The value-set string the binding declares, version suffix included —
    /// which is the key a consumer looks it up by, because that is what the
    /// snapshot wrote.
    pub value_set: String,
    /// Every element path bound to it, in model order, each once.
    pub element_paths: Vec<String>,
    pub variants: Vec<Variant>,
}

/// A `required` binding the generator could not resolve, which it reports
/// rather than silently leaving out (`codegen.binding.004`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedBinding {
    pub element_path: String,
    pub value_set_url: String,
    pub strength: String,
}

/// The strength that closes an enumeration. The other three never do.
pub const REQUIRED: &str = "required";

/// What one resolution pass over a model produced.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Resolution {
    pub enumerations: Vec<Enumeration>,
    pub unresolved: Vec<UnresolvedBinding>,
}

// ---------------------------------------------------------------------------
// Expanding the value sets
// ---------------------------------------------------------------------------

/// The codes behind every value set an expansion from one input can reach.
///
/// Keyed by the value set's canonical URL **without** a version suffix, which
/// is how a `ValueSet` declares itself; a binding names it with one, so
/// [`Codes::of_binding`] strips it.
#[derive(Debug, Clone, Default)]
pub struct Codes {
    by_url: BTreeMap<String, Vec<String>>,
}

impl Codes {
    /// Expand the package's value sets against its code systems.
    pub fn of_package(package: &Package) -> Codes {
        let systems: BTreeMap<&str, &crate::package::CodeSystemDefinition> = package
            .code_systems
            .iter()
            .map(|system| (system.url.as_str(), system))
            .collect();

        let mut by_url = BTreeMap::new();
        for value_set in &package.value_sets {
            if !value_set.has_compose || value_set.has_exclude {
                continue;
            }
            let mut codes = Vec::new();
            let mut expandable = !value_set.includes.is_empty();
            for include in &value_set.includes {
                if include.has_filter || !include.imports.is_empty() {
                    expandable = false;
                    break;
                }
                if !include.concepts.is_empty() {
                    codes.extend(include.concepts.iter().cloned());
                    continue;
                }
                let Some(system) = include
                    .system
                    .as_deref()
                    .and_then(|url| systems.get(url))
                    .filter(|system| system.content.as_deref() == Some("complete"))
                else {
                    expandable = false;
                    break;
                };
                codes.extend(system.codes.iter().cloned());
            }
            if expandable {
                by_url.insert(value_set.url.clone(), codes);
            }
        }
        Codes { by_url }
    }

    /// The code lists a caller wrote, used where the pinned package contains
    /// no instance of the condition a scenario describes.
    pub fn of_synthetic(value_sets: &[synthetic::ValueSet]) -> Codes {
        Codes {
            by_url: value_sets
                .iter()
                .map(|value_set| (value_set.url.clone(), value_set.codes.clone()))
                .collect(),
        }
    }

    /// The codes a binding's value-set string names, with any `|version`
    /// suffix stripped. All 224 of R4's `required` bindings carry one.
    pub fn of_binding(&self, value_set: &str) -> Option<&[String]> {
        let url = value_set.split('|').next().unwrap_or(value_set);
        self.by_url.get(url).map(Vec::as_slice)
    }
}

// ---------------------------------------------------------------------------
// The naming rule
// ---------------------------------------------------------------------------

/// The variants a code list produces, in the list's own order, each distinct.
pub fn code_variants(codes: &[&str]) -> Vec<Variant> {
    let mut taken: BTreeSet<String> = BTreeSet::new();
    codes
        .iter()
        .map(|code| Variant {
            code: (*code).to_string(),
            rust_name: unique(&variant_name(code), &mut taken),
        })
        .collect()
}

/// The Rust variant name one code produces, before disambiguation.
pub fn variant_name(code: &str) -> String {
    let mut name = String::with_capacity(code.len() + 2);
    let mut word = String::new();
    for c in code.chars() {
        if c.is_ascii_alphanumeric() {
            word.push(c);
        } else if !word.is_empty() {
            push_word(&mut name, &word);
            word.clear();
        }
    }
    if !word.is_empty() {
        push_word(&mut name, &word);
    }

    if name.is_empty() {
        // A code with no alphanumeric character at all: `=`, `<=`, `>=`.
        for c in code.chars() {
            name.push_str(&symbol_name(c));
        }
    }
    if name.is_empty() {
        // The empty code string, which no R4 code system declares and which a
        // synthetic one could.
        name.push_str("Empty");
    }
    if name.starts_with(|c: char| c.is_ascii_digit()) {
        name.insert(0, 'N');
    }
    if RUST_KEYWORDS.contains(&name.as_str()) {
        name.push('_');
    }
    name
}

fn push_word(out: &mut String, word: &str) {
    let mut chars = word.chars();
    if let Some(first) = chars.next() {
        out.extend(first.to_uppercase());
        out.push_str(chars.as_str());
    }
}

/// What a non-alphanumeric character is called, for a code that is nothing
/// else. Every symbol R4's required-binding codes contain is here; anything
/// outside the table is spelled by its code point, so the rule stays total.
pub const SYMBOL_NAMES: [(char, &str); 29] = [
    (' ', "Space"),
    ('!', "Not"),
    ('"', "Quote"),
    ('#', "Hash"),
    ('$', "Dollar"),
    ('%', "Percent"),
    ('&', "And"),
    ('\'', "Apostrophe"),
    ('(', "Open"),
    (')', "Close"),
    ('*', "Star"),
    ('+', "Plus"),
    (',', "Comma"),
    ('-', "Minus"),
    ('.', "Dot"),
    ('/', "Slash"),
    (':', "Colon"),
    (';', "Semicolon"),
    ('<', "Less"),
    ('=', "Equal"),
    ('>', "Greater"),
    ('?', "Question"),
    ('@', "At"),
    ('[', "OpenBracket"),
    ('\\', "Backslash"),
    (']', "CloseBracket"),
    ('^', "Caret"),
    ('_', "Underscore"),
    ('~', "Tilde"),
];

fn symbol_name(c: char) -> String {
    SYMBOL_NAMES
        .iter()
        .find(|(symbol, _)| *symbol == c)
        .map(|(_, name)| (*name).to_string())
        .unwrap_or_else(|| format!("U{:04X}", c as u32))
}

/// A value set's own Rust name, from the last segment of its URL:
/// `administrative-gender` is `AdministrativeGender`. Measured over R4's 224
/// required-binding value sets, the 224 stems are distinct.
pub fn enum_name(value_set: &str) -> String {
    let stem = value_set
        .split('|')
        .next()
        .unwrap_or(value_set)
        .rsplit('/')
        .next()
        .unwrap_or(value_set);
    variant_name(stem)
}

/// `stem`, or `stem2`, `stem3` … if it is taken.
fn unique(stem: &str, taken: &mut BTreeSet<String>) -> String {
    if taken.insert(stem.to_string()) {
        return stem.to_string();
    }
    for suffix in 2..u32::MAX {
        let candidate = format!("{stem}{suffix}");
        if taken.insert(candidate.clone()) {
            return candidate;
        }
    }
    unreachable!("a name cannot be taken four billion times")
}

/// Rust's keywords, reserved words included. A camel-case variant name can
/// only ever hit `Self`, but the check is over the whole list because a rule
/// that is safe only for the input it was measured on is not a rule.
const RUST_KEYWORDS: [&str; 51] = [
    "as", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "false", "fn",
    "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
    "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe",
    "use", "where", "while", "async", "await", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "try", "typeof", "unsized", "virtual", "yield",
];

// ---------------------------------------------------------------------------
// Resolving one model's bindings
// ---------------------------------------------------------------------------

/// What one element contributes to the resolution: its path, and the binding
/// the snapshot declares for it.
#[derive(Debug, Clone, Copy)]
pub struct BoundElement<'a> {
    pub path: &'a str,
    pub strength: &'a str,
    pub value_set: &'a str,
}

/// Decide which `required` bindings close, in one pass over the bound
/// elements.
///
/// The enumerations come out in value-set order and their names are assigned
/// in that order, so the output does not depend on which element happened to
/// be visited first (`codegen.output.001`). The unresolved bindings stay in
/// model order, because that is the order a reader of the report would look
/// for them in.
pub fn resolve(elements: &[BoundElement<'_>], codes: &Codes) -> Resolution {
    let mut resolvable: BTreeSet<&str> = BTreeSet::new();
    for element in elements {
        if element.strength == REQUIRED && codes.of_binding(element.value_set).is_some() {
            resolvable.insert(element.value_set);
        }
    }

    let mut taken: BTreeSet<String> = BTreeSet::new();
    let mut names: BTreeMap<&str, String> = BTreeMap::new();
    for value_set in &resolvable {
        names.insert(value_set, unique(&enum_name(value_set), &mut taken));
    }

    let mut paths: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    let mut unresolved = Vec::new();
    for element in elements {
        if element.strength != REQUIRED {
            continue;
        }
        if names.contains_key(element.value_set) {
            let bound = paths.entry(element.value_set).or_default();
            if !bound.iter().any(|path| path == element.path) {
                bound.push(element.path.to_string());
            }
        } else {
            unresolved.push(UnresolvedBinding {
                element_path: element.path.to_string(),
                value_set_url: element.value_set.to_string(),
                strength: element.strength.to_string(),
            });
        }
    }

    let enumerations = resolvable
        .iter()
        .map(|value_set| {
            let expanded = codes
                .of_binding(value_set)
                .expect("a resolvable value set expands");
            let as_str: Vec<&str> = expanded.iter().map(String::as_str).collect();
            Enumeration {
                rust_name: names[*value_set].clone(),
                value_set: (*value_set).to_string(),
                element_paths: paths.remove(*value_set).unwrap_or_default(),
                variants: code_variants(&as_str),
            }
        })
        .collect();

    Resolution {
        enumerations,
        unresolved,
    }
}

/// The enumeration chosen for one element, by strength and value-set string.
pub fn enumeration_of<'a>(
    resolution: &'a Resolution,
    strength: &str,
    value_set: &str,
) -> Option<&'a str> {
    if strength != REQUIRED {
        return None;
    }
    resolution
        .enumerations
        .iter()
        .find(|generated| generated.value_set == value_set)
        .map(|generated| generated.rust_name.as_str())
}
