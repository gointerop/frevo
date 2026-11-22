//! Writing `searchparam.rs`: the generated R4 search-parameter table.
//!
//! Task 13 of `add-fhirpath-and-codegen-r4`. [`crate::searchparam`] builds the
//! rows; this module writes them out.
//!
//! # Why everything a row points at is hoisted
//!
//! A row has fourteen fields, four of them lists and one of them a string that
//! reaches 1,386 characters. Hoisting each list and each expression into its
//! own `const` does three things at once: the longest value is written once
//! rather than once per base, a row's field value is always a single short
//! token, and `crate::rustsrc` never has to reproduce how `rustfmt` breaks a
//! nested literal apart — which it would otherwise have to, because
//! `codegen.output.004` compares the emitted bytes with `rustfmt`'s own output.
//!
//! The row itself is a **struct literal** rather than a constructor call for a
//! related reason: fourteen arguments is past
//! `clippy::too_many_arguments`, and `codegen.output.004` forbids an
//! `#[allow]` added to accommodate the generator's output style. A struct
//! literal wider than `rustfmt`'s `struct_lit_width` is always broken one
//! field per line, and every one of these is, which is the same rule
//! `types.rs` already relies on for `TypeDef`.

use std::collections::{BTreeMap, BTreeSet};

use crate::emit::{GeneratedFile, header};
use crate::pin::PackagePin;
use crate::rustsrc::{INDENT, Item, array_statement, pad, quote, scalar_statement};
use crate::searchparam::Row;

/// The search-parameter table: `searchparam.rs`.
pub fn searchparam_file(pin: &PackagePin, rows: &[Row]) -> GeneratedFile {
    let hoisted = Hoisted::of(rows);
    let mut source = header(pin);
    source.push_str(&doc(rows));
    source.push_str(PREAMBLE);
    source.push_str(&hoisted.emit());
    source.push_str(&table(rows, &hoisted));
    source.push_str(LOOKUP);

    GeneratedFile::new("searchparam.rs", &source)
}

fn doc(rows: &[Row]) -> String {
    let parameters: BTreeSet<&str> = rows.iter().map(|row| row.id.as_str()).collect();
    let composites = rows
        .iter()
        .filter(|row| row.search_type == "composite")
        .count();
    let without: BTreeSet<&str> = rows
        .iter()
        .filter(|row| row.expression.is_none())
        .map(|row| row.id.as_str())
        .collect();
    let stated = |count: fn(&Row) -> bool| -> usize {
        rows.iter()
            .filter(|row| count(row))
            .map(|row| row.id.as_str())
            .collect::<BTreeSet<&str>>()
            .len()
    };
    format!(
        "\n//! The generated R4 search-parameter table.\n\
         //!\n\
         //! {rows} rows over {parameters} SearchParameter resources: **one row per\n\
         //! declared `base`**, because the index is keyed by resource type. Every row\n\
         //! also carries the whole declared base list, so the parameters are\n\
         //! recoverable from the rows.\n\
         //!\n\
         //! {composites} rows are `composite` and name their component parameter\n\
         //! definitions with each component's own expression. {without} parameter(s)\n\
         //! declare no `expression` at all -- {names} -- recorded as `None`,\n\
         //! distinctly from an empty expression.\n\
         //!\n\
         //! Every modifier flag is an `Option`: absent means **not stated**, not\n\
         //! `false`. In R4 the distinction is observable rather than theoretical --\n\
         //! {multiple_or} parameters state `multipleOr` and every one of them states\n\
         //! it `false`, {comparator} state `comparator`, and `multipleAnd`,\n\
         //! `modifier` and `chain` are stated by none.\n\
         //!\n\
         //! A row whose `base` is `Resource` or `DomainResource` is stored as\n\
         //! declared, and resolving it onto a concrete type is `applicable_to`'s\n\
         //! job. The distinction matters: `_id` is declared on `Resource` and so\n\
         //! applies to every resource, while `_text` is declared on\n\
         //! `DomainResource` and so does not apply to `Binary`, `Bundle` or\n\
         //! `Parameters`.\n\
         //!\n\
         //! `SearchParameter.status` is deliberately not a column. The package and\n\
         //! HAPI's `sp/search-parameters.json` disagree about it -- every one of the\n\
         //! parameters reads `draft` in the package and `active` in HAPI, and it is\n\
         //! the only field that differs across all of them -- and nothing downstream\n\
         //! of this table consults it, because a parameter is enabled by\n\
         //! configuration rather than by its publication state.\n",
        rows = rows.len(),
        parameters = parameters.len(),
        composites = composites,
        without = without.len(),
        names = without.into_iter().collect::<Vec<_>>().join(", "),
        multiple_or = stated(|row| row.multiple_or.is_some()),
        comparator = stated(|row| row.comparator.is_some()),
    )
}

const PREAMBLE: &str = "\n\
/// One composite parameter's component.\n\
#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n\
pub struct Component {\n\
\x20   /// The canonical URL of the component's own SearchParameter.\n\
\x20   pub definition: &'static str,\n\
\x20   /// The component's expression, relative to the composite's.\n\
\x20   pub expression: &'static str,\n\
}\n\
\n\
/// One search parameter as it applies to one base resource type.\n\
#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n\
pub struct SearchParamRow {\n\
\x20   /// `SearchParameter.id`.\n\
\x20   pub id: &'static str,\n\
\x20   /// The code a query names the parameter by.\n\
\x20   pub code: &'static str,\n\
\x20   /// The canonical URL.\n\
\x20   pub url: &'static str,\n\
\x20   /// `SearchParameter.type`: `token`, `reference`, `composite` and so on.\n\
\x20   pub search_type: &'static str,\n\
\x20   /// The one base this row is for.\n\
\x20   pub base: &'static str,\n\
\x20   /// Every base the parameter declares.\n\
\x20   pub declared_bases: &'static [&'static str],\n\
\x20   /// `None` when the parameter declares no expression at all, which is not\n\
\x20   /// the same as an empty one.\n\
\x20   pub expression: Option<&'static str>,\n\
\x20   /// Empty unless the parameter is `composite`.\n\
\x20   pub components: &'static [Component],\n\
\x20   /// Empty unless the parameter is a `reference`.\n\
\x20   pub targets: &'static [&'static str],\n\
\x20   /// `None` means the parameter does not state the flag, which is not the\n\
\x20   /// same as stating it `false`.\n\
\x20   pub multiple_or: Option<bool>,\n\
\x20   pub multiple_and: Option<bool>,\n\
\x20   pub comparator: Option<&'static [&'static str]>,\n\
\x20   pub modifier: Option<&'static [&'static str]>,\n\
\x20   pub chain: Option<&'static [&'static str]>,\n\
}\n\
\n\
/// One component of a composite parameter.\n\
const fn k(definition: &'static str, expression: &'static str) -> Component {\n\
\x20   Component {\n\
\x20       definition,\n\
\x20       expression,\n\
\x20   }\n\
}\n";

const LOOKUP: &str = "\n\
/// Every row declared on one base, exactly as declared: an abstract base is\n\
/// not resolved here.\n\
///\n\
/// A scan, because this is a convenience over the table rather than the index\n\
/// path: an extractor that needs the rows per type builds its own map from\n\
/// `SEARCH_PARAMETERS` once at startup rather than calling this per request.\n\
pub fn rows_for_base(base: &str) -> Vec<&'static SearchParamRow> {\n\
\x20   SEARCH_PARAMETERS\n\
\x20       .iter()\n\
\x20       .filter(|row| row.base == base)\n\
\x20       .collect()\n\
}\n\
\n\
/// Every row that applies to one concrete resource type: its own first, then\n\
/// each abstract base's, outermost last.\n\
///\n\
/// The base chain is the caller's, because this module holds no type\n\
/// hierarchy and `types.rs` does. Pass the queried type followed by every\n\
/// type above it.\n\
pub fn applicable_to(base_chain: &[&str]) -> Vec<&'static SearchParamRow> {\n\
\x20   base_chain\n\
\x20       .iter()\n\
\x20       .flat_map(|base| rows_for_base(base))\n\
\x20       .collect()\n\
}\n\
\n\
/// One row, by parameter id and base.\n\
pub fn row(id: &str, base: &str) -> Option<&'static SearchParamRow> {\n\
\x20   SEARCH_PARAMETERS\n\
\x20       .iter()\n\
\x20       .find(|row| row.id == id && row.base == base)\n\
}\n";

/// One composite's components, as `(definition, expression)` pairs in
/// declared order.
type Components = Vec<(String, String)>;

/// The arrays and strings every row points at, hoisted and deduplicated.
struct Hoisted {
    bases: BTreeMap<Vec<String>, String>,
    targets: BTreeMap<Vec<String>, String>,
    code_lists: BTreeMap<Vec<String>, String>,
    expressions: BTreeMap<String, String>,
    /// Keyed by `SearchParameter.id`, because a composite's components belong
    /// to the parameter rather than to one of its rows.
    components: BTreeMap<String, (String, Components)>,
}

impl Hoisted {
    fn of(rows: &[Row]) -> Hoisted {
        let mut bases: BTreeSet<Vec<String>> = BTreeSet::new();
        let mut targets: BTreeSet<Vec<String>> = BTreeSet::new();
        let mut code_lists: BTreeSet<Vec<String>> = BTreeSet::new();
        let mut expressions: BTreeSet<String> = BTreeSet::new();
        let mut components: BTreeMap<String, Components> = BTreeMap::new();

        for row in rows {
            bases.insert(row.declared_bases.clone());
            if !row.targets.is_empty() {
                targets.insert(row.targets.clone());
            }
            for codes in [&row.comparator, &row.modifier, &row.chain]
                .into_iter()
                .flatten()
            {
                code_lists.insert(codes.clone());
            }
            if let Some(expression) = &row.expression {
                expressions.insert(expression.clone());
            }
            if !row.components.is_empty() {
                components.insert(
                    row.id.clone(),
                    row.components
                        .iter()
                        .map(|component| {
                            (component.definition.clone(), component.expression.clone())
                        })
                        .collect(),
                );
            }
        }

        Hoisted {
            bases: numbered(bases, "B"),
            targets: numbered(targets, "G"),
            code_lists: numbered(code_lists, "C"),
            expressions: numbered(expressions, "X"),
            components: components
                .into_iter()
                .enumerate()
                .map(|(index, (id, parts))| (id, (format!("K_{index:04}"), parts)))
                .collect(),
        }
    }

    fn emit(&self) -> String {
        let mut out = String::new();

        out.push_str(
            "\n// The base lists parameters declare, in lexical order of their contents.\n\
             // The longest -- clinical-patient's -- has 32 entries, and R4's longest\n\
             // base list is why the table has more rows than parameters.\n",
        );
        for (value, name) in &self.bases {
            out.push_str(&string_array(name, value));
        }

        if !self.targets.is_empty() {
            out.push_str("\n// The target lists reference parameters declare.\n");
            for (value, name) in &self.targets {
                out.push_str(&string_array(name, value));
            }
        }

        if !self.code_lists.is_empty() {
            out.push_str(
                "\n// The code lists a stated comparator, modifier or chain carries. R4\n\
                 // states only comparators, and every one of them states the same nine\n\
                 // codes.\n",
            );
            for (value, name) in &self.code_lists {
                out.push_str(&string_array(name, value));
            }
        }

        if !self.components.is_empty() {
            out.push_str("\n// The components of each composite parameter, in declared order.\n");
            let mut by_name: BTreeMap<&str, (&str, &Components)> = BTreeMap::new();
            for (id, (name, parts)) in &self.components {
                by_name.insert(name.as_str(), (id.as_str(), parts));
            }
            for (name, (id, parts)) in by_name {
                let items: Vec<Item> = parts
                    .iter()
                    .map(|(definition, expression)| {
                        Item::call("k", vec![quote(definition), quote(expression)])
                    })
                    .collect();
                out.push_str(&format!("\n// {id}\n"));
                out.push_str(&array_statement(
                    0,
                    &format!("const {name}: &[Component] = "),
                    &items,
                    ";",
                ));
            }
        }

        out.push_str(
            "\n// Every distinct expression, written once. The longest is 1,386\n\
             // characters, which no struct literal can carry inline.\n",
        );
        let mut by_name: BTreeMap<&str, &str> = BTreeMap::new();
        for (expression, name) in &self.expressions {
            by_name.insert(name.as_str(), expression.as_str());
        }
        for (name, expression) in by_name {
            out.push_str(&scalar_statement(
                0,
                &format!("const {name}: &str = "),
                &quote(expression),
                ";",
            ));
        }
        out
    }

    /// One row as a struct literal, one field per line.
    fn row_literal(&self, row: &Row, indent: usize) -> String {
        let list = |value: &Vec<String>, table: &BTreeMap<Vec<String>, String>| -> String {
            table
                .get(value)
                .cloned()
                .expect("every list a row names is hoisted")
        };
        let optional = |value: &Option<Vec<String>>| -> String {
            match value {
                Some(codes) => format!("Some({})", list(codes, &self.code_lists)),
                None => "None".to_string(),
            }
        };
        let fields: [(&str, String); 14] = [
            ("id", quote(&row.id)),
            ("code", quote(&row.code)),
            ("url", quote(&row.url)),
            ("search_type", quote(&row.search_type)),
            ("base", quote(&row.base)),
            ("declared_bases", list(&row.declared_bases, &self.bases)),
            (
                "expression",
                match &row.expression {
                    Some(expression) => format!(
                        "Some({})",
                        self.expressions
                            .get(expression)
                            .expect("every expression is hoisted")
                    ),
                    None => "None".to_string(),
                },
            ),
            (
                "components",
                match self.components.get(&row.id) {
                    Some((name, _)) => name.clone(),
                    None => "&[]".to_string(),
                },
            ),
            (
                "targets",
                if row.targets.is_empty() {
                    "&[]".to_string()
                } else {
                    list(&row.targets, &self.targets)
                },
            ),
            ("multiple_or", optional_bool(row.multiple_or)),
            ("multiple_and", optional_bool(row.multiple_and)),
            ("comparator", optional(&row.comparator)),
            ("modifier", optional(&row.modifier)),
            ("chain", optional(&row.chain)),
        ];

        let mut out = format!("{}SearchParamRow {{\n", pad(indent));
        for (name, value) in fields {
            out.push_str(&scalar_statement(
                indent + INDENT,
                &format!("{name}: "),
                &value,
                ",",
            ));
        }
        out.push_str(&format!("{}}},\n", pad(indent)));
        out
    }
}

fn table(rows: &[Row], hoisted: &Hoisted) -> String {
    let mut out = format!(
        "\n/// Every search-parameter row, in canonical-URL order and then in each\n\
         /// parameter's own declared-base order. There are {}.\n\
         pub const SEARCH_PARAMETERS: &[SearchParamRow] = &[\n",
        rows.len()
    );
    for row in rows {
        out.push_str(&hoisted.row_literal(row, INDENT));
    }
    out.push_str("];\n");
    out
}

fn optional_bool(value: Option<bool>) -> String {
    match value {
        Some(value) => format!("Some({value})"),
        None => "None".to_string(),
    }
}

fn string_array(name: &str, values: &[String]) -> String {
    let items: Vec<Item> = values.iter().map(|value| Item::string(value)).collect();
    array_statement(0, &format!("const {name}: &[&str] = "), &items, ";")
}

/// Assign `PREFIX_0000`-style names to a collection, in its sorted order.
fn numbered<T: Ord>(values: impl IntoIterator<Item = T>, prefix: &str) -> BTreeMap<T, String> {
    values
        .into_iter()
        .collect::<BTreeSet<T>>()
        .into_iter()
        .enumerate()
        .map(|(index, value)| (value, format!("{prefix}_{index:04}")))
        .collect()
}
