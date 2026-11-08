//! Comparing an evaluation outcome with a case's `<output>` expectations.
//!
//! Two branches, kept separate on purpose (`path.oracle.007`):
//!
//! - **ordered** — every case without `ordered="false"`. Position by position,
//!   through the FHIRPath string value, which carries `@` or `@T` for a
//!   temporal type.
//! - **unordered** — the one `ordered="false"` case in the pinned file,
//!   `testLiterals/testExpressions` at `tests-fhir-r4.xml:321`. Each outcome
//!   item is matched against **any** expectation, through the plain string
//!   value with no prefix. An expectation may be matched by more than one item,
//!   because the match is existential per item and consumes nothing.
//!
//! The one `ordered="false"` case in the pinned file yields strings, so the
//! divergence between the two renderers is latent today. Unifying them would
//! surface as a whole-suite failure the first time a temporal result appears in
//! an unordered case, which is why they are two functions and not one.

use super::suite::Output;
use super::value::{self, ResultItem};

/// Why a comparison failed. `Ok(())` is a match.
pub type Comparison = Result<(), String>;

/// The count check both branches share.
///
/// `FHIRPathTests` asserts `outcome.size() == expected.size()` before either
/// branch, so a count mismatch fails the case whatever the items are
/// (`path.oracle.008`).
fn compare_count(expected: &[Output], items: &[ResultItem]) -> Comparison {
    if expected.len() == items.len() {
        Ok(())
    } else {
        Err(format!(
            "expected {} output(s) but the outcome has {} item(s)",
            expected.len(),
            items.len()
        ))
    }
}

/// The ordered branch.
pub fn compare_ordered(expected: &[Output], items: &[ResultItem]) -> Comparison {
    compare_count(expected, items)?;
    for (i, (out, item)) in expected.iter().zip(items.iter()).enumerate() {
        if out.checks_type() && out.type_name != item.fhir_type() {
            return Err(format!(
                "outcome {i}: type should be {} but was {}",
                out.type_name,
                item.fhir_type()
            ));
        }
        if !out.checks_value() {
            continue;
        }
        match item {
            // The branch is chosen by the *result item's* runtime type and not
            // by the expectation's `type` attribute: the suite has exactly one
            // `<output type="Quantity">`, while `LowBoundary/LowBoundaryQuantity`
            // and `HighBoundary/HighBoundaryQuantity` expect `1.58650000 'cm'`
            // and `1.58750000 'm'` from a bare `<output>` (`path.oracle.009`).
            ResultItem::Quantity(actual) => {
                let want = value::parse_quantity_string(&out.body).ok_or_else(|| {
                    format!(
                        "outcome {i}: expected value {:?} is not a parseable quantity string",
                        out.body
                    )
                })?;
                if !actual.equals_deep(&want) {
                    return Err(format!(
                        "outcome {i}: value should be {:?} but was {actual:?}",
                        out.body
                    ));
                }
            }
            ResultItem::Primitive { .. } => {
                let got = value::fhirpath_string(item).expect("a primitive renders");
                if got != out.body {
                    return Err(format!(
                        "outcome {i}: value should be {:?} but was {got:?}",
                        out.body
                    ));
                }
            }
            ResultItem::Complex { fhir_type } => {
                return Err(format!(
                    "outcome {i}: value should be a primitive type but was {fhir_type}"
                ));
            }
        }
    }
    Ok(())
}

/// The unordered branch, for `ordered="false"`.
pub fn compare_unordered(expected: &[Output], items: &[ResultItem]) -> Comparison {
    compare_count(expected, items)?;
    for (i, item) in items.iter().enumerate() {
        let rendered = value::plain_string(item).ok_or_else(|| {
            format!(
                "outcome {i}: a {} has no plain string value to compare on the unordered branch",
                item.fhir_type()
            )
        })?;
        let found = expected.iter().any(|out| {
            (!out.checks_type() || out.type_name == item.fhir_type())
                && (!out.checks_value() || out.body == rendered)
        });
        if !found {
            return Err(format!(
                "outcome {i}: value {rendered:?} of type {} not expected",
                item.fhir_type()
            ));
        }
    }
    Ok(())
}
