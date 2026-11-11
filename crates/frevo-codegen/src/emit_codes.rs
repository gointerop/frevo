//! Writing `codes.rs`: the closed enumerations R4's `required` bindings
//! produce.
//!
//! Task 12 of `add-fhirpath-and-codegen-r4`. [`crate::bindings`] decides which
//! bindings close and what their variants are called; this module writes the
//! decision out.
//!
//! # Why a table and not 220 Rust `enum`s
//!
//! `design.md` section 8 budgets a cold `cargo build -p frevo-model` at 120 s
//! and the generated source at 40 MB, and names "generate the codec as data
//! tables interpreted at runtime rather than as per-type code" as the lever if
//! that is missed. Tasks 8 to 10 already took that shape for the type and
//! element tables, and 220 enumerations with 2,066 variants, each needing a
//! `FromStr`, a `Display` and a round-trip test, would be the largest single
//! block of per-case code in the crate for no behaviour a table does not
//! already give: the only operations anything performs on a required binding
//! are "is this code in it" and "what is this code's variant called", both of
//! which are a lookup.
//!
//! A variant's `rust_name` is therefore emitted as data **and is still a valid
//! Rust identifier**, because a later change that does want a real `enum` per
//! value set must be able to generate it from this table without renaming
//! anything. `codegen.naming.005` asserts exactly that property.

use crate::emit::{GeneratedFile, header};
use crate::model::Model;
use crate::pin::PackagePin;
use crate::rustsrc::{Item, array_statement, quote};

/// The required-binding enumerations: `codes.rs`.
pub fn codes_file(pin: &PackagePin, model: &Model) -> GeneratedFile {
    let mut source = header(pin);
    source.push_str(&doc(model));
    source.push_str(PREAMBLE);
    source.push_str(&tables(model));
    source.push_str(LOOKUP);

    GeneratedFile::new("codes.rs", &source)
}

fn doc(model: &Model) -> String {
    let variants: usize = model
        .enumerations
        .iter()
        .map(|generated| generated.variants.len())
        .sum();
    let paths: usize = model
        .enumerations
        .iter()
        .map(|generated| generated.element_paths.len())
        .sum();
    format!(
        "\n//! The closed enumerations R4's `required` bindings produce.\n\
         //!\n\
         //! {enums} enumerations over {variants} codes, named by {paths} element\n\
         //! paths. There is one entry per value set that some element binds\n\
         //! `required` **and** whose codes expand from the input package alone;\n\
         //! {unresolved} required binding(s) do not expand, and are listed in\n\
         //! `UNRESOLVED_REQUIRED_BINDINGS` rather than dropped or guessed at. The\n\
         //! elements they are on stay open.\n\
         //!\n\
         //! A binding weaker than `required` -- extensible, preferred, example --\n\
         //! produces nothing here, and the element it is on accepts any code string.\n\
         //! Every element's declared binding, whatever its strength, is in\n\
         //! `elements.rs`.\n\
         //!\n\
         //! Each variant carries the code it came from, so serialising one returns\n\
         //! the original code string verbatim, and each `rust_name` is a plain Rust\n\
         //! identifier so that a real `enum` per value set could be generated from\n\
         //! this table without renaming anything.\n",
        enums = model.enumerations.len(),
        unresolved = model.unresolved_bindings.len(),
    )
}

const PREAMBLE: &str = "\n\
/// One variant of a closed enumeration, and the code it came from.\n\
#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n\
pub struct CodeVariant {\n\
\x20   /// The code string, which is what serialisation emits.\n\
\x20   pub code: &'static str,\n\
\x20   /// The Rust variant name: a plain identifier, never a raw one.\n\
\x20   pub rust_name: &'static str,\n\
}\n\
\n\
/// A closed enumeration over one value set's codes.\n\
#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n\
pub struct CodeEnum {\n\
\x20   pub rust_name: &'static str,\n\
\x20   /// The value-set string the binding declares, version suffix included.\n\
\x20   pub value_set: &'static str,\n\
\x20   /// Every variant, in value-set order.\n\
\x20   pub variants: &'static [CodeVariant],\n\
\x20   /// Every element path bound to it.\n\
\x20   pub element_paths: &'static [&'static str],\n\
}\n\
\n\
impl CodeEnum {\n\
\x20   /// The variant one code names, or `None` when the code is outside the\n\
\x20   /// binding -- which is a diagnostic, not a reason to drop the value.\n\
\x20   pub fn variant(&self, code: &str) -> Option<&'static CodeVariant> {\n\
\x20       self.variants.iter().find(|variant| variant.code == code)\n\
\x20   }\n\
\n\
\x20   /// The codes this enumeration admits, in value-set order.\n\
\x20   pub fn codes(&self) -> impl Iterator<Item = &'static str> {\n\
\x20       self.variants.iter().map(|variant| variant.code)\n\
\x20   }\n\
}\n\
\n\
/// One variant.\n\
const fn v(code: &'static str, rust_name: &'static str) -> CodeVariant {\n\
\x20   CodeVariant { code, rust_name }\n\
}\n\
\n\
/// One enumeration.\n\
const fn e(\n\
\x20   rust_name: &'static str,\n\
\x20   value_set: &'static str,\n\
\x20   variants: &'static [CodeVariant],\n\
\x20   element_paths: &'static [&'static str],\n\
) -> CodeEnum {\n\
\x20   CodeEnum {\n\
\x20       rust_name,\n\
\x20       value_set,\n\
\x20       variants,\n\
\x20       element_paths,\n\
\x20   }\n\
}\n\
\n\
/// A required binding whose value set does not expand from the input package.\n\
const fn u(\n\
\x20   element_path: &'static str,\n\
\x20   value_set: &'static str,\n\
\x20   strength: &'static str,\n\
) -> (&'static str, &'static str, &'static str) {\n\
\x20   (element_path, value_set, strength)\n\
}\n";

const LOOKUP: &str = "\n\
/// The enumeration one value-set string closes, or `None` when nothing does.\n\
pub fn code_enum(value_set: &str) -> Option<&'static CodeEnum> {\n\
\x20   CODE_ENUMS\n\
\x20       .iter()\n\
\x20       .find(|generated| generated.value_set == value_set)\n\
}\n\
\n\
/// The enumeration one element path is bound to, or `None` when the element is\n\
/// open -- which is every element whose binding is weaker than `required`, and\n\
/// every one whose required value set does not expand from the package.\n\
pub fn code_enum_of_element(element_path: &str) -> Option<&'static CodeEnum> {\n\
\x20   CODE_ENUMS.iter().find(|generated| {\n\
\x20       generated\n\
\x20           .element_paths\n\
\x20           .iter()\n\
\x20           .copied()\n\
\x20           .any(|path| path == element_path)\n\
\x20   })\n\
}\n";

fn tables(model: &Model) -> String {
    let mut out = String::new();
    if !model.enumerations.is_empty() {
        out.push_str(
            "\n// The variants and the bound element paths of each enumeration, hoisted\n\
             // so that every row of CODE_ENUMS below is one line.\n",
        );
    }
    for (index, generated) in model.enumerations.iter().enumerate() {
        out.push_str(&format!("\n// {}\n", generated.value_set));
        let variants: Vec<Item> = generated
            .variants
            .iter()
            .map(|variant| Item::call("v", vec![quote(&variant.code), quote(&variant.rust_name)]))
            .collect();
        out.push_str(&array_statement(
            0,
            &format!("const V_{index:04}: &[CodeVariant] = "),
            &variants,
            ";",
        ));
        let paths: Vec<Item> = generated
            .element_paths
            .iter()
            .map(|path| Item::string(path))
            .collect();
        out.push_str(&array_statement(
            0,
            &format!("const P_{index:04}: &[&str] = "),
            &paths,
            ";",
        ));
    }

    let rows: Vec<Item> = model
        .enumerations
        .iter()
        .enumerate()
        .map(|(index, generated)| {
            Item::call(
                "e",
                vec![
                    quote(&generated.rust_name),
                    quote(&generated.value_set),
                    format!("V_{index:04}"),
                    format!("P_{index:04}"),
                ],
            )
        })
        .collect();
    out.push_str(&format!(
        "\n/// Every closed enumeration, in value-set order. There are {}.\n",
        rows.len()
    ));
    out.push_str(&array_statement(
        0,
        "pub const CODE_ENUMS: &[CodeEnum] = ",
        &rows,
        ";",
    ));

    let unresolved: Vec<Item> = model
        .unresolved_bindings
        .iter()
        .map(|binding| {
            Item::call(
                "u",
                vec![
                    quote(&binding.element_path),
                    quote(&binding.value_set_url),
                    quote(&binding.strength),
                ],
            )
        })
        .collect();
    out.push_str(
        "\n/// Every required binding the generator could not close: the element path,\n\
         /// the value-set URL that could not be resolved, and the strength it was\n\
         /// declared with. The element stays open.\n",
    );
    out.push_str(&array_statement(
        0,
        "pub const UNRESOLVED_REQUIRED_BINDINGS: &[(&str, &str, &str)] = ",
        &unresolved,
        ";",
    ));
    out
}
