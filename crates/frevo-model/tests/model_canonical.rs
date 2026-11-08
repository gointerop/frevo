//! The `model-canonical` in-process oracle: the canonical JSON form the
//! round-trip oracle compares with.
//!
//! Task 5 of `add-fhirpath-and-codegen-r4`. Run it with:
//!
//! ```text
//! cargo test -p frevo-model --test model_canonical
//! ```
//!
//! Scenario coverage: `model.canonical.001` .. `model.canonical.006`.
//! `model.canonical.007` is the encode-side dangling-reference condition and is
//! graded by the committed `model_parser_errors.rs` oracle; it is not repeated
//! here.
//!
//! ## What "canonical" means, and where it comes from
//!
//! Not an invention of this project: it is what
//! `org.hl7.fhir.r4.formats.JsonCreatorCanonical` produces at pin `73c432b5`.
//! That class collects each object's property names, sorts them, and refuses to
//! pretty-print. Using HAPI's own canonical form is what makes
//! `model.canonical.005` a meaningful comparison rather than a comparison
//! against a convention of our own (`design.md` section 3, "Two serialisation
//! orders, deliberately").
//!
//! ## Why this file carries its own JSON reader
//!
//! `frevo-model` has no dependencies, and this change adds none. The reader
//! below is about 150 lines and exists only so the oracle can state "every
//! object's property names are in ascending lexicographic order" and "there is
//! no whitespace between tokens" as structural facts about the codec's output
//! rather than as a string comparison against an expected blob. It keeps every
//! number as its **lexical** text, which is what lets `model.canonical.003`
//! tell `1.58700` from `1.587`.
//!
//! ## The seam
//!
//! The oracle runs against a trait object. The unbound binding returns
//! [`CanonicalError::NotImplemented`] from every surface, and every helper here
//! turns that into an explicit "nothing is bound yet" failure rather than
//! letting it read as a pass.

use std::fmt;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// The seam
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalError {
    /// Serialisation failed, with the codec's own message.
    Failed(String),
    /// No codec is bound to this oracle yet.
    NotImplemented,
}

impl fmt::Display for CanonicalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CanonicalError::Failed(m) => write!(f, "serialisation failed: {m}"),
            CanonicalError::NotImplemented => f.write_str(NOT_BOUND),
        }
    }
}

const NOT_BOUND: &str = "no codec is bound to the model-canonical oracle yet \
     (crates/frevo-model/tests/model_canonical.rs::codec_under_test)";

pub trait CanonicalOracle {
    /// Decode `bytes` and serialise the result in canonical form.
    fn canonicalise(&self, bytes: &[u8]) -> Result<Vec<u8>, CanonicalError>;

    /// Decode `bytes`, re-encode in the **stored** form (generated element
    /// order), decode that, and serialise canonically. This is the right-hand
    /// side of the `r4-example-roundtrip` comparison.
    fn roundtrip_then_canonicalise(&self, bytes: &[u8]) -> Result<Vec<u8>, CanonicalError>;

    /// Decode `bytes` and serialise in the stored form **with indentation**.
    fn pretty(&self, bytes: &[u8]) -> Result<Vec<u8>, CanonicalError>;

    /// Decode both inputs and report whether every element of the two
    /// resources is equal.
    fn elements_equal(&self, a: &[u8], b: &[u8]) -> Result<bool, CanonicalError>;
}

/// The codec this oracle grades. Phase C replaces this body with the real one.
fn codec_under_test() -> Box<dyn CanonicalOracle> {
    Box::new(UnboundCodec)
}

struct UnboundCodec;

impl CanonicalOracle for UnboundCodec {
    fn canonicalise(&self, _bytes: &[u8]) -> Result<Vec<u8>, CanonicalError> {
        Err(CanonicalError::NotImplemented)
    }

    fn roundtrip_then_canonicalise(&self, _bytes: &[u8]) -> Result<Vec<u8>, CanonicalError> {
        Err(CanonicalError::NotImplemented)
    }

    fn pretty(&self, _bytes: &[u8]) -> Result<Vec<u8>, CanonicalError> {
        Err(CanonicalError::NotImplemented)
    }

    fn elements_equal(&self, _a: &[u8], _b: &[u8]) -> Result<bool, CanonicalError> {
        Err(CanonicalError::NotImplemented)
    }
}

fn must_canonicalise(bytes: &[u8], what: &str) -> Vec<u8> {
    match codec_under_test().canonicalise(bytes) {
        Ok(v) => v,
        Err(e) => panic!("{what}: canonical serialisation must succeed, but: {e}"),
    }
}

fn must_roundtrip_then_canonicalise(bytes: &[u8], what: &str) -> Vec<u8> {
    match codec_under_test().roundtrip_then_canonicalise(bytes) {
        Ok(v) => v,
        Err(e) => panic!("{what}: the round trip must succeed, but: {e}"),
    }
}

fn must_pretty(bytes: &[u8], what: &str) -> Vec<u8> {
    match codec_under_test().pretty(bytes) {
        Ok(v) => v,
        Err(e) => panic!("{what}: indented serialisation must succeed, but: {e}"),
    }
}

fn as_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

// ---------------------------------------------------------------------------
// A minimal JSON reader, so the structural claims can be asserted structurally
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
enum Json {
    Null,
    Bool(bool),
    /// The number's **lexical** text, never a binary float.
    Number(String),
    Str(String),
    Array(Vec<Json>),
    /// Properties in the order they appeared.
    Object(Vec<(String, Json)>),
}

struct Reader<'a> {
    src: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn new(src: &'a [u8]) -> Self {
        Reader { src, at: 0 }
    }

    fn fail(&self, why: &str) -> String {
        format!("at byte {}: {why}", self.at)
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.src.get(self.at) {
            if matches!(c, b' ' | b'\t' | b'\r' | b'\n') {
                self.at += 1;
            } else {
                break;
            }
        }
    }

    fn eat(&mut self, c: u8) -> Result<(), String> {
        if self.src.get(self.at) == Some(&c) {
            self.at += 1;
            Ok(())
        } else {
            Err(self.fail(&format!("expected {}", c as char)))
        }
    }

    fn value(&mut self) -> Result<Json, String> {
        self.skip_ws();
        match self.src.get(self.at) {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => self.string().map(Json::Str),
            Some(b't') => self.literal("true").map(|()| Json::Bool(true)),
            Some(b'f') => self.literal("false").map(|()| Json::Bool(false)),
            Some(b'n') => self.literal("null").map(|()| Json::Null),
            Some(_) => self.number(),
            None => Err(self.fail("unexpected end of input")),
        }
    }

    fn literal(&mut self, text: &str) -> Result<(), String> {
        if self.src[self.at..].starts_with(text.as_bytes()) {
            self.at += text.len();
            Ok(())
        } else {
            Err(self.fail(&format!("expected {text}")))
        }
    }

    fn number(&mut self) -> Result<Json, String> {
        let start = self.at;
        while let Some(c) = self.src.get(self.at) {
            if c.is_ascii_digit() || matches!(c, b'-' | b'+' | b'.' | b'e' | b'E') {
                self.at += 1;
            } else {
                break;
            }
        }
        if self.at == start {
            return Err(self.fail("expected a value"));
        }
        Ok(Json::Number(
            String::from_utf8_lossy(&self.src[start..self.at]).into_owned(),
        ))
    }

    fn string(&mut self) -> Result<String, String> {
        self.eat(b'"')?;
        let mut out = String::new();
        loop {
            match self.src.get(self.at) {
                None => return Err(self.fail("unterminated string")),
                Some(b'"') => {
                    self.at += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.at += 1;
                    let esc = *self
                        .src
                        .get(self.at)
                        .ok_or_else(|| self.fail("bad escape"))?;
                    self.at += 1;
                    match esc {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let hex = self
                                .src
                                .get(self.at..self.at + 4)
                                .ok_or_else(|| self.fail("truncated \\u escape"))?;
                            let code = u32::from_str_radix(&String::from_utf8_lossy(hex), 16)
                                .map_err(|_| self.fail("bad \\u escape"))?;
                            self.at += 4;
                            // Surrogate halves are kept as the replacement
                            // character: no fixture here carries one, and a
                            // lossless reader is not what this file is for.
                            out.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                        }
                        other => {
                            return Err(self.fail(&format!("unknown escape \\{}", other as char)));
                        }
                    }
                }
                Some(_) => {
                    let start = self.at;
                    while let Some(c) = self.src.get(self.at) {
                        if matches!(c, b'"' | b'\\') {
                            break;
                        }
                        self.at += 1;
                    }
                    out.push_str(&String::from_utf8_lossy(&self.src[start..self.at]));
                }
            }
        }
    }

    fn array(&mut self) -> Result<Json, String> {
        self.eat(b'[')?;
        let mut items = Vec::new();
        self.skip_ws();
        if self.src.get(self.at) == Some(&b']') {
            self.at += 1;
            return Ok(Json::Array(items));
        }
        loop {
            items.push(self.value()?);
            self.skip_ws();
            match self.src.get(self.at) {
                Some(b',') => self.at += 1,
                Some(b']') => {
                    self.at += 1;
                    return Ok(Json::Array(items));
                }
                _ => return Err(self.fail("expected , or ] in an array")),
            }
        }
    }

    fn object(&mut self) -> Result<Json, String> {
        self.eat(b'{')?;
        let mut props = Vec::new();
        self.skip_ws();
        if self.src.get(self.at) == Some(&b'}') {
            self.at += 1;
            return Ok(Json::Object(props));
        }
        loop {
            self.skip_ws();
            let key = self.string()?;
            self.skip_ws();
            self.eat(b':')?;
            let value = self.value()?;
            props.push((key, value));
            self.skip_ws();
            match self.src.get(self.at) {
                Some(b',') => self.at += 1,
                Some(b'}') => {
                    self.at += 1;
                    return Ok(Json::Object(props));
                }
                _ => return Err(self.fail("expected , or } in an object")),
            }
        }
    }
}

fn parse(bytes: &[u8], what: &str) -> Json {
    let mut r = Reader::new(bytes);
    let v = r
        .value()
        .unwrap_or_else(|e| panic!("{what}: the output is not JSON: {e}\n{}", as_text(bytes)));
    r.skip_ws();
    assert_eq!(
        r.at,
        bytes.len(),
        "{what}: trailing bytes after the JSON value: {:?}",
        as_text(&bytes[r.at..])
    );
    v
}

/// Every object path whose property names are not in ascending lexicographic
/// order, with the order found.
fn unsorted_objects(v: &Json, path: &str, out: &mut Vec<(String, Vec<String>)>) {
    match v {
        Json::Object(props) => {
            let keys: Vec<String> = props.iter().map(|(k, _)| k.clone()).collect();
            let mut sorted = keys.clone();
            sorted.sort();
            if keys != sorted {
                out.push((path.to_string(), keys));
            }
            for (k, child) in props {
                unsorted_objects(child, &format!("{path}.{k}"), out);
            }
        }
        Json::Array(items) => {
            for (i, child) in items.iter().enumerate() {
                unsorted_objects(child, &format!("{path}[{i}]"), out);
            }
        }
        _ => {}
    }
}

/// The byte offsets at which whitespace occurs outside a string literal.
fn whitespace_outside_strings(bytes: &[u8]) -> Vec<usize> {
    let mut out = Vec::new();
    let mut in_string = false;
    let mut escaped = false;
    for (i, &c) in bytes.iter().enumerate() {
        if in_string {
            if escaped {
                escaped = false;
            } else if c == b'\\' {
                escaped = true;
            } else if c == b'"' {
                in_string = false;
            }
            continue;
        }
        match c {
            b'"' => in_string = true,
            b' ' | b'\t' | b'\r' | b'\n' => out.push(i),
            _ => {}
        }
    }
    out
}

/// Render with every object's properties sorted and no whitespace. This is the
/// oracle's own notion of "same up to whitespace and key order", used to grade
/// `model.canonical.006` independently of the codec's own comparison.
fn render_key_sorted(v: &Json) -> String {
    match v {
        Json::Null => "null".to_string(),
        Json::Bool(b) => b.to_string(),
        Json::Number(n) => n.clone(),
        Json::Str(s) => format!("{s:?}"),
        Json::Array(items) => {
            let inner: Vec<String> = items.iter().map(render_key_sorted).collect();
            format!("[{}]", inner.join(","))
        }
        Json::Object(props) => {
            let mut sorted: Vec<&(String, Json)> = props.iter().collect();
            sorted.sort_by(|a, b| a.0.cmp(&b.0));
            let inner: Vec<String> = sorted
                .iter()
                .map(|(k, v)| format!("{k:?}:{}", render_key_sorted(v)))
                .collect();
            format!("{{{}}}", inner.join(","))
        }
    }
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// A pretty-printed `Patient` whose property names are deliberately **not** in
/// lexicographic order and whose repeating elements are deliberately not in
/// sorted order either, so that sorting the one and leaving the other alone are
/// separable observations.
const PRETTY_PATIENT: &str = r#"{
  "resourceType": "Patient",
  "id": "canonical-example",
  "text": {
    "status": "generated",
    "div": "<div xmlns=\"http://www.w3.org/1999/xhtml\">Windsor</div>"
  },
  "active": true,
  "name": [
    {
      "use": "official",
      "family": "Windsor",
      "given": [
        "Peter",
        "James",
        "Duncan"
      ]
    },
    {
      "use": "maiden",
      "family": "Chalmers"
    }
  ],
  "gender": "male",
  "birthDate": "1974-12-25",
  "_birthDate": {
    "extension": [
      {
        "url": "https://example.org/birth-time",
        "valueString": "08:00"
      }
    ]
  },
  "managingOrganization": {
    "reference": "Organization/org1"
  }
}"#;

/// The three decimal forms, pretty-printed. `1.58700` is the one
/// `model.canonical.003` names.
const PRETTY_DECIMALS: &str = r#"{
  "resourceType": "Parameters",
  "id": "example-decimals",
  "parameter": [
    {
      "name": "trailing-zeros",
      "valueDecimal": 1.58700
    },
    {
      "name": "exponent",
      "valueDecimal": 1.00e3
    },
    {
      "name": "small",
      "valueDecimal": 0.0014
    }
  ]
}"#;

/// A `Bundle` with entries of three types, so the round trip has contained
/// resources and nested objects to get wrong.
const PRETTY_BUNDLE: &str = r#"{
  "resourceType": "Bundle",
  "id": "canonical-bundle",
  "type": "collection",
  "entry": [
    {
      "fullUrl": "urn:uuid:1",
      "resource": {
        "resourceType": "Patient",
        "id": "p1",
        "name": [
          {
            "family": "Zeta"
          },
          {
            "family": "Alpha"
          }
        ]
      }
    },
    {
      "fullUrl": "urn:uuid:2",
      "resource": {
        "resourceType": "Observation",
        "id": "o1",
        "status": "final",
        "code": {
          "text": "Glucose"
        },
        "valueQuantity": {
          "value": 6.30,
          "unit": "mmol/L"
        }
      }
    }
  ]
}"#;

/// A `Patient` carrying a contained resource and a `null` hole in a primitive
/// array, which are the two shapes most likely to be reordered or lost.
const PRETTY_CONTAINED: &str = r##"{
  "resourceType": "Patient",
  "id": "with-contained",
  "contained": [
    {
      "resourceType": "Organization",
      "id": "org1",
      "name": "Acme Healthcare"
    }
  ],
  "name": [
    {
      "given": [
        null,
        "James"
      ],
      "_given": [
        {
          "extension": [
            {
              "url": "https://example.org/syllable-count",
              "valueString": "five"
            }
          ]
        },
        null
      ]
    }
  ],
  "managingOrganization": {
    "reference": "#org1"
  }
}"##;

const FIXTURES: &[(&str, &str)] = &[
    ("a pretty-printed Patient", PRETTY_PATIENT),
    ("the three decimal forms", PRETTY_DECIMALS),
    ("a Bundle of mixed entry types", PRETTY_BUNDLE),
    ("a Patient with a contained resource", PRETTY_CONTAINED),
];

/// The interim round-trip selector of task 56: the 103 R4 examples under
/// `oracles/fhir-test-cases/fetched/r4/examples/`. The directory is
/// git-ignored, so it is additional evidence rather than the only evidence --
/// every scenario here is graded by the embedded fixtures whether or not
/// `oracles/fetch.sh` has run.
fn interim_example_corpus() -> Option<(PathBuf, Vec<PathBuf>)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../oracles/fhir-test-cases/fetched/r4/examples");
    let entries = std::fs::read_dir(&dir).ok()?;
    let mut files: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    if files.is_empty() {
        return None;
    }
    Some((dir, files))
}

// ---------------------------------------------------------------------------
// model.canonical.001 .. model.canonical.006
// ---------------------------------------------------------------------------

// covers: model.canonical.001
#[test]
fn canonical_form_sorts_property_names_and_emits_no_insignificant_whitespace() {
    for (what, fixture) in FIXTURES {
        let out = must_canonicalise(fixture.as_bytes(), what);
        let parsed = parse(&out, what);

        let mut unsorted = Vec::new();
        unsorted_objects(&parsed, "$", &mut unsorted);
        assert!(
            unsorted.is_empty(),
            "{what}: every object's property names must be in ascending \
             lexicographic order, as JsonCreatorCanonical sorts them. These are \
             not: {unsorted:?}"
        );

        let ws = whitespace_outside_strings(&out);
        assert!(
            ws.is_empty(),
            "{what}: there must be no whitespace between tokens, but there is \
             some at byte offsets {ws:?}:\n{}",
            as_text(&out)
        );
    }
}

// covers: model.canonical.002
#[test]
fn canonical_form_preserves_array_order() {
    let out = must_canonicalise(PRETTY_PATIENT.as_bytes(), "a pretty-printed Patient");
    let parsed = parse(&out, "the canonical Patient");

    // `given` is Peter, James, Duncan -- which is neither sorted nor reverse
    // sorted, so only preserving document order produces it. Arrays are
    // ordered in FHIR and only object properties are sorted.
    let given = select(&parsed, &["name", "0", "given"], "Patient.name[0].given");
    assert_eq!(
        strings(given, "Patient.name[0].given"),
        ["Peter", "James", "Duncan"],
        "array element order must be unchanged, not sorted"
    );

    // And the two `name` repetitions keep their order too, even though sorting
    // them would put Chalmers first.
    let names = select(&parsed, &["name"], "Patient.name");
    let families: Vec<String> = match names {
        Json::Array(items) => items
            .iter()
            .map(|n| match select(n, &["family"], "HumanName.family") {
                Json::Str(s) => s.clone(),
                other => panic!("family must be a string, got {other:?}"),
            })
            .collect(),
        other => panic!("Patient.name must be an array, got {other:?}"),
    };
    assert_eq!(
        families,
        ["Windsor", "Chalmers"],
        "repeating complex elements keep document order as well"
    );
}

/// Walk a path of object keys and array indices.
fn select<'a>(v: &'a Json, path: &[&str], what: &str) -> &'a Json {
    let mut cur = v;
    for step in path {
        cur = match cur {
            Json::Object(props) => props
                .iter()
                .find(|(k, _)| k == step)
                .map(|(_, v)| v)
                .unwrap_or_else(|| panic!("{what}: no property {step:?} in {cur:?}")),
            Json::Array(items) => {
                let i: usize = step
                    .parse()
                    .unwrap_or_else(|_| panic!("{what}: {step:?} is not an array index"));
                items
                    .get(i)
                    .unwrap_or_else(|| panic!("{what}: no index {i} in {cur:?}"))
            }
            other => panic!("{what}: cannot enter {step:?} of {other:?}"),
        };
    }
    cur
}

fn strings(v: &Json, what: &str) -> Vec<String> {
    match v {
        Json::Array(items) => items
            .iter()
            .map(|i| match i {
                Json::Str(s) => s.clone(),
                other => panic!("{what}: expected a string, got {other:?}"),
            })
            .collect(),
        other => panic!("{what}: expected an array, got {other:?}"),
    }
}

// covers: model.canonical.003
#[test]
fn canonical_form_preserves_a_decimals_presented_form() {
    let out = must_canonicalise(PRETTY_DECIMALS.as_bytes(), "the three decimal forms");
    let parsed = parse(&out, "the canonical decimals");

    // The reader keeps a number's lexical text, so this distinguishes 1.58700
    // from 1.587 -- which a reader that went through an f64 could not.
    let value = select(
        &parsed,
        &["parameter", "0", "valueDecimal"],
        "Parameters.parameter[0].value",
    );
    assert_eq!(
        *value,
        Json::Number("1.58700".to_string()),
        "the output is 1.58700, not 1.587"
    );

    // The other two forms are not normalised either.
    assert_eq!(
        *select(
            &parsed,
            &["parameter", "1", "valueDecimal"],
            "Parameters.parameter[1].value"
        ),
        Json::Number("1.00e3".to_string()),
        "the exponent form survives canonicalisation"
    );
    assert_eq!(
        *select(
            &parsed,
            &["parameter", "2", "valueDecimal"],
            "Parameters.parameter[2].value"
        ),
        Json::Number("0.0014".to_string())
    );
}

// covers: model.canonical.004
#[test]
fn canonical_form_is_idempotent() {
    for (what, fixture) in FIXTURES {
        let once = must_canonicalise(fixture.as_bytes(), what);
        let twice = must_canonicalise(&once, &format!("{what}, canonicalised twice"));
        assert_eq!(
            as_text(&twice),
            as_text(&once),
            "{what}: decoding canonical output and canonically serialising it \
             again must be byte-identical to the first output"
        );
    }
}

// covers: model.canonical.005
#[test]
fn canonicalising_an_example_equals_canonicalising_its_round_trip() {
    // This is the comparison the r4-example-roundtrip oracle performs: the file
    // canonicalised directly against the file decoded, re-encoded in the stored
    // element order, and then canonicalised.
    for (what, fixture) in FIXTURES {
        let direct = must_canonicalise(fixture.as_bytes(), what);
        let via_roundtrip = must_roundtrip_then_canonicalise(fixture.as_bytes(), what);
        assert_eq!(
            as_text(&via_roundtrip),
            as_text(&direct),
            "{what}: both byte sequences must be identical"
        );
    }

    // And over the interim selector of task 56 when it has been fetched. The
    // directory is git-ignored, so its absence is reported rather than treated
    // as a pass -- the embedded fixtures above are what make this scenario fail
    // in a clean checkout.
    match interim_example_corpus() {
        None => eprintln!(
            "note: the interim round-trip corpus is not present. Run \
             `bash oracles/fetch.sh` to extend this scenario to the 103 R4 \
             examples under oracles/fhir-test-cases/fetched/r4/examples/; the \
             embedded fixtures above grade it either way."
        ),
        Some((dir, files)) => {
            eprintln!(
                "note: grading model.canonical.005 over the INTERIM selector -- \
                 {} example files in {}. Task 59 replaces this with the full \
                 hl7.fhir.r4.examples#4.0.1 package.",
                files.len(),
                dir.display()
            );
            assert!(
                files.len() >= 100,
                "the interim selector is the 103 examples in {}; only {} were \
                 found, so the corpus is incomplete rather than absent",
                dir.display(),
                files.len()
            );
            let mut mismatched = Vec::new();
            for file in &files {
                let bytes = std::fs::read(file)
                    .unwrap_or_else(|e| panic!("{} must be readable: {e}", file.display()));
                let name = file
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let direct = must_canonicalise(&bytes, &name);
                let via_roundtrip = must_roundtrip_then_canonicalise(&bytes, &name);
                if direct != via_roundtrip {
                    mismatched.push(name);
                }
            }
            assert!(
                mismatched.is_empty(),
                "canonicalising the file and canonicalising its round trip must \
                 agree for every example; these disagree: {mismatched:?}"
            );
        }
    }
}

// covers: model.canonical.006
#[test]
fn canonical_and_pretty_forms_differ_only_in_whitespace_and_key_order() {
    let codec = codec_under_test();

    for (what, fixture) in FIXTURES {
        let canonical = must_canonicalise(fixture.as_bytes(), what);
        let indented = must_pretty(fixture.as_bytes(), what);

        // The two forms really are different forms, or the scenario is vacuous.
        assert_ne!(
            as_text(&indented),
            as_text(&canonical),
            "{what}: the indented form must actually be indented, otherwise \
             there is nothing to compare"
        );
        assert!(
            !whitespace_outside_strings(&indented).is_empty(),
            "{what}: the indented form carries whitespace between tokens"
        );

        // Decoding both yields resources whose every element is equal.
        match codec.elements_equal(&canonical, &indented) {
            Ok(true) => {}
            Ok(false) => panic!(
                "{what}: decoding the canonical and the indented form must yield \
                 resources whose every element is equal\ncanonical: {}\nindented: {}",
                as_text(&canonical),
                as_text(&indented)
            ),
            Err(e) => panic!("{what}: comparing the two decodes must succeed, but: {e}"),
        }

        // Graded independently of the codec's own comparison: strip the
        // whitespace and sort the keys of each, and the two must be the same
        // document. A difference here is a difference in content, not in form.
        let a = render_key_sorted(&parse(&canonical, &format!("{what}, canonical")));
        let b = render_key_sorted(&parse(&indented, &format!("{what}, indented")));
        assert_eq!(
            a, b,
            "{what}: the two forms must differ *only* in whitespace and key \
             order"
        );
    }
}
