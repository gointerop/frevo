//! The generated R4 search-parameter table.
//!
//! Task 13 of `add-fhirpath-and-codegen-r4`. One row per *(parameter, base)*
//! pair over the 1,375 `SearchParameter` resources the specification stamps —
//! **1,706 rows**, because 203 parameters declare more than one base and
//! `clinical-patient` alone declares 32 (`codegen.searchparam.002`).
//!
//! # Why a row per base rather than a row per parameter
//!
//! Because the index is keyed by resource type. A parameter row is read when a
//! search names a type and a code, and the table that answers that lookup
//! without a scan is one keyed by `(base, code)`. Carrying
//! [`Row::declared_bases`] on every row is what keeps the two views
//! reconcilable: a row knows its siblings, so the 1,375 parameters are
//! recoverable from the 1,706 rows.
//!
//! # Base resolution, which is not a row
//!
//! Nine of the 1,375 declare an **abstract** base: eight on `Resource`
//! (`_id`, `_lastUpdated`, `_profile`, `_security`, `_source`, `_tag`,
//! `_content`, `_query`) and one on `DomainResource` (`_text`). They are
//! stored as declared — a row whose base is `Resource` says `Resource`,
//! because inventing 146 rows per whole-system parameter would make the table
//! 1,300 rows longer and lose the fact that the specification declared one.
//!
//! The resolution happens at **query** time, in [`applicable`], by walking the
//! queried type's `baseDefinition` chain. That distinction is observable and is
//! what `codegen.searchparam.002` turns on: `_id` applies to `Binary`, whose
//! base is `Resource`, and `_text` does not, because `Binary` is not a
//! `DomainResource`. `Binary`, `Bundle` and `Parameters` are the only three
//! concrete R4 resources for which the two answers differ.
//!
//! # What is not in a row
//!
//! `SearchParameter.status`. The field is not in the table, so the generator
//! does not have to choose between the two sources that disagree about it:
//! every one of the 1,375 parameters reads `"draft"` in
//! `hl7.fhir.r4.core#4.0.1` and `"active"` in HAPI's
//! `sp/search-parameters.json`, and it is the only field that differs across
//! all 1,375. Nothing downstream of a search-parameter table reads `status` —
//! a parameter is enabled by configuration, not by its publication state — so
//! emitting it would be recording a disagreement nobody consults. `tasks.md`
//! records the decision.

use std::collections::BTreeSet;

use crate::error::{CodegenError, Result};
use crate::inventory;
use crate::package::Package;

/// One composite parameter's component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    /// The canonical URL of the component's own `SearchParameter`.
    pub definition: String,
    /// The component's expression, relative to the composite's.
    pub expression: String,
}

/// One row of the generated table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// `SearchParameter.id`, which is the table's case selector.
    pub id: String,
    pub code: String,
    pub url: String,
    /// `SearchParameter.type`.
    pub search_type: String,
    /// The one base this row is for.
    pub base: String,
    /// Every base the parameter declares, so a row knows its siblings.
    pub declared_bases: Vec<String>,
    /// `None` when the parameter declares no `expression` at all, which is
    /// distinct from `Some(String::new())` (`codegen.searchparam.005`). R4 has
    /// exactly three: `DomainResource-text`, `Resource-content` and
    /// `Resource-query`.
    pub expression: Option<String>,
    pub components: Vec<Component>,
    pub targets: Vec<String>,
    /// Every flag is `Option`, and absent means "not stated" rather than
    /// `false` (`codegen.searchparam.006`). In R4, 46 parameters state
    /// `multipleOr` and 142 state `comparator`; `multipleAnd`, `modifier` and
    /// `chain` are stated by none, which is what makes the distinction
    /// observable.
    pub multiple_or: Option<bool>,
    pub multiple_and: Option<bool>,
    pub comparator: Option<Vec<String>>,
    pub modifier: Option<Vec<String>>,
    pub chain: Option<Vec<String>>,
}

/// The whole table, in canonical-URL order and then declared-base order.
///
/// Scoped the way [`crate::inventory`] scopes the count: only the parameters
/// carrying `structuredefinition-standards-status`, which is exactly the 1,375
/// of `sp/search-parameters.json`. The 25 the package additionally ships —
/// five extension-registry packs plus `example` and `filter` — are not the
/// specification's own and are excluded, so the table and the reported
/// inventory cannot disagree (`codegen.searchparam.001`).
pub fn table(package: &Package) -> Vec<Row> {
    let mut rows = Vec::new();
    for parameter in &package.search_parameters {
        if !parameter.has_standards_status() {
            continue;
        }
        for base in &parameter.bases {
            rows.push(Row {
                id: parameter.id.clone(),
                code: parameter.code.clone(),
                url: parameter.url.clone(),
                search_type: parameter.search_type.clone(),
                base: base.clone(),
                declared_bases: parameter.bases.clone(),
                expression: parameter.expression.clone(),
                components: parameter
                    .components
                    .iter()
                    .map(|component| Component {
                        definition: component.definition.clone(),
                        expression: component.expression.clone(),
                    })
                    .collect(),
                targets: parameter.targets.clone(),
                multiple_or: parameter.multiple_or,
                multiple_and: parameter.multiple_and,
                comparator: parameter.comparator.clone(),
                modifier: parameter.modifier.clone(),
                chain: parameter.chain.clone(),
            });
        }
    }
    rows
}

/// The bases a search on `resource_type` matches: the type itself and every
/// abstract type above it, outermost last.
///
/// Read from `StructureDefinition.baseDefinition`, never from a list written
/// here: `Patient` gives `["Patient", "DomainResource", "Resource"]` and
/// `Binary` gives `["Binary", "Resource"]` because that is what the snapshots
/// say.
pub fn base_chain(package: &Package, resource_type: &str) -> Result<Vec<String>> {
    let in_scope: Vec<&crate::package::Definition> = package
        .definitions
        .iter()
        .filter(|definition| {
            inventory::in_core_scope(definition) && inventory::names_itself(definition)
        })
        .collect();
    let by_name = |name: &str| in_scope.iter().find(|d| d.name == name).copied();

    let Some(start) = by_name(resource_type) else {
        return Err(CodegenError::other(format!(
            "{resource_type} is not a type the pinned package defines, so the \
             search-parameter table cannot be queried for it"
        )));
    };

    let mut chain = vec![start.name.clone()];
    let mut current = start;
    // 212 in-scope definitions bound the walk; a cycle would be a corrupt
    // package rather than something to recurse into.
    for _ in 0..in_scope.len() {
        let Some(next) = current
            .base_definition
            .as_deref()
            .map(|url| url.rsplit('/').next().unwrap_or(url))
            .and_then(by_name)
        else {
            break;
        };
        if chain.iter().any(|name| name == &next.name) {
            break;
        }
        chain.push(next.name.clone());
        current = next;
    }
    Ok(chain)
}

/// Every row that applies to one resource type, with the abstract bases
/// resolved onto it (`codegen.searchparam.002`).
///
/// Order: the type's own rows first, then each abstract base's, so a lookup
/// that stops at the first match prefers the most specific declaration. Within
/// one base the table's own order is kept.
pub fn applicable(package: &Package, resource_type: &str) -> Result<Vec<Row>> {
    let chain = base_chain(package, resource_type)?;
    let rows = table(package);
    let mut out = Vec::new();
    for base in &chain {
        out.extend(rows.iter().filter(|row| &row.base == base).cloned());
    }
    Ok(out)
}

/// Every distinct expression in the table, so a caller can hand each one to a
/// parser without walking 1,706 rows for 1,344 answers
/// (`codegen.searchparam.005`).
pub fn distinct_expressions(rows: &[Row]) -> Vec<&str> {
    rows.iter()
        .filter_map(|row| row.expression.as_deref())
        .collect::<BTreeSet<&str>>()
        .into_iter()
        .collect()
}
