//! The public evaluation surface over [`Evaluator`]: evaluate from text,
//! evaluate-first, and evaluation that requires a result type.
//!
//! Derived from hapi-fhir/hapi-fhir-structures-r4/src/main/java/org/hl7/fhir/r4/hapi/fluentpath/FhirPathR4.java
//! (`evaluate(IBase, String, Class)`, `evaluate(IBase, IParsedExpression,
//! Class)`, the private `evaluate` whose `for (IBase next : result)` loop
//! raises `Msg.code(256)`, `evaluateFirst` and `parse`) at the pin in
//! `reference/PINS.md`.
//! Original work Copyright University Health Network, licensed under the
//! Apache License, Version 2.0.
//! Modified by the Frevo authors: the requested type is a FHIR type name
//! rather than a Java class, tested against the model's type hierarchy rather
//! than with `Class.isAssignableFrom`; a parse failure and an evaluation
//! failure are two variants of one error rather than one exception class with
//! two message codes.
//!
//! Task 54 of `add-fhirpath-and-codegen-r4` (`path.api.001` to
//! `path.api.006`). Every operation here is synchronous (`path.api.006`).

use std::error::Error;
use std::fmt;

use super::classes;
use super::navigate::fhir_type;
use super::{EvalError, Evaluator};
use crate::seam::value::Value;
use crate::syntax::ast::Expression;
use crate::syntax::error::ParseError;
use crate::syntax::parser::ParseOptions;

/// The requested type that accepts every result: `IBase`, which is what a
/// caller that does not care passes.
const ANY_TYPE: &str = "Base";

/// A failure of an operation that starts from expression text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathError {
    /// The text did not parse. `FhirPathR4` wraps this as `Msg.code(2409)`.
    Parse(ParseError),
    /// The parsed expression did not evaluate, or returned an item of a type
    /// other than the one asked for. See [`Evaluator::evaluate_as`].
    Evaluate(EvalError),
}

impl PathError {
    /// Whether this is the not-implemented kind rather than a real failure:
    /// a caller that grades results must report it as not having run.
    pub fn is_unsupported(&self) -> bool {
        matches!(self, PathError::Evaluate(e) if e.is_unsupported())
    }
}

impl fmt::Display for PathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PathError::Parse(e) => e.fmt(f),
            PathError::Evaluate(e) => e.fmt(f),
        }
    }
}

impl Error for PathError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            PathError::Parse(e) => Some(e),
            PathError::Evaluate(e) => Some(e),
        }
    }
}

impl From<ParseError> for PathError {
    fn from(e: ParseError) -> Self {
        PathError::Parse(e)
    }
}

impl From<EvalError> for PathError {
    fn from(e: EvalError) -> Self {
        PathError::Evaluate(e)
    }
}

impl Evaluator<'_> {
    /// `FHIRPathEngine.parse(String)`, with this evaluator's host service
    /// consulted for a function name the R4 table does not have.
    ///
    /// Liquid mode is not implemented (`path.strictness.010`): with it on,
    /// this answers [`EvalError::Unsupported`] rather than parsing as if it
    /// were off.
    pub fn parse(&self, text: &str) -> Result<Expression, PathError> {
        if self.liquid_mode {
            return Err(PathError::Evaluate(EvalError::unsupported(
                "liquid mode",
                0,
            )));
        }
        Ok(Expression::parse_with(
            text,
            ParseOptions::default(),
            self.host,
        )?)
    }

    /// `IFhirPath.evaluate(IBase, String, Class)` with `IBase.class`: parse
    /// `text` and evaluate it over `focus` (`path.api.001`).
    pub fn evaluate_text(
        &self,
        text: &str,
        focus: Option<&Value>,
    ) -> Result<Vec<Value>, PathError> {
        let expr = self.parse(text)?;
        Ok(self.evaluate(&expr, focus)?)
    }

    /// `IFhirPath.evaluate(IBase, String, Class)`: parse `text`, evaluate it
    /// and require every result to be of `expected_type`.
    pub fn evaluate_text_as(
        &self,
        text: &str,
        focus: Option<&Value>,
        expected_type: &str,
    ) -> Result<Vec<Value>, PathError> {
        let expr = self.parse(text)?;
        Ok(self.evaluate_as(&expr, focus, expected_type)?)
    }

    /// `IFhirPath.evaluateFirst(IBase, String, Class)` with `IBase.class`.
    pub fn evaluate_first_text(
        &self,
        text: &str,
        focus: Option<&Value>,
    ) -> Result<Option<Value>, PathError> {
        let expr = self.parse(text)?;
        Ok(self.evaluate_first(&expr, focus)?)
    }

    /// `FhirPathR4.evaluateFirst`, which is `evaluate(...).stream().findFirst()`:
    /// the first result in order, or `None` when there is none
    /// (`path.api.003`, `path.api.004`). `None` is no result at all; a result
    /// that is an empty string is `Some`.
    pub fn evaluate_first(
        &self,
        expr: &Expression,
        focus: Option<&Value>,
    ) -> Result<Option<Value>, EvalError> {
        Ok(self.evaluate(expr, focus)?.into_iter().next())
    }

    /// [`Evaluator::evaluate_first`] with every result required to be of
    /// `expected_type`, as `FhirPathR4.evaluateFirst` checks the whole result
    /// before taking the first.
    pub fn evaluate_first_as(
        &self,
        expr: &Expression,
        focus: Option<&Value>,
        expected_type: &str,
    ) -> Result<Option<Value>, EvalError> {
        Ok(self
            .evaluate_as(expr, focus, expected_type)?
            .into_iter()
            .next())
    }

    /// Evaluate and require every result to be of `expected_type`, a FHIR type
    /// name (`path.api.005`). A result whose type is not `expected_type` and
    /// does not derive from it fails the whole evaluation with
    /// `FhirPathR4`'s `Msg.code(256)` text, naming both types; it is never
    /// dropped. An empty result satisfies any type. `Base` accepts everything.
    ///
    /// The failure is an [`EvalError::Execution`] with no offset, because the
    /// check is made after the evaluation and is about no sub-expression.
    pub fn evaluate_as(
        &self,
        expr: &Expression,
        focus: Option<&Value>,
        expected_type: &str,
    ) -> Result<Vec<Value>, EvalError> {
        // Where each navigated node came from decides its class
        // (`Patient.gender` is an `Enumeration`), so it is recorded for the
        // length of this evaluation.
        let _recording = classes::Recording::begin();
        let result = self.evaluate(expr, focus)?;
        if expected_type == ANY_TYPE {
            return Ok(result);
        }
        for item in &result {
            if let Some(message) = self.mismatch(item, expected_type) {
                return Err(EvalError::plain(format!("HAPI-0256: {message}")));
            }
        }
        Ok(result)
    }

    /// `theReturnType.isAssignableFrom(next.getClass())` over the generated R4
    /// model class table (`path.api.005`, `codegen.ancestry.005`): the message
    /// for an item that is not accepted, naming the item's class's simple name
    /// and the requested type's Java name.
    ///
    /// An item the table has no class for -- a node of a type the R4 package
    /// does not define -- falls back to the model's type hierarchy and FHIR
    /// type names.
    fn mismatch(&self, item: &Value, expected_type: &str) -> Option<String> {
        let message = |item_class: &str, expected: &str| {
            format!(
                "FhirPath expression returned unexpected type {item_class} - Expected {expected}"
            )
        };
        match classes::class_of(item) {
            Some(row) => {
                let (ok, expected) = classes::accepts(row, expected_type);
                (!ok).then(|| message(&row.simple, &expected))
            }
            None => {
                let own = fhir_type(item);
                let ok = own == expected_type || self.model.derives_from(own, expected_type);
                (!ok).then(|| message(own, expected_type))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::EngineSettings;
    use crate::seam::model::Model;

    struct StubModel;

    /// `Value` has no `PartialEq` (a node is compared by identity nowhere),
    /// so results are compared by their debug rendering.
    fn shown<T: std::fmt::Debug>(v: T) -> String {
        format!("{v:?}")
    }

    impl Model for StubModel {
        fn is_type(&self, t: &str) -> bool {
            matches!(t, "string" | "code" | "integer" | "Element")
        }
        fn base_type(&self, t: &str) -> Option<&str> {
            match t {
                "code" => Some("string"),
                "string" | "integer" => Some("Element"),
                _ => None,
            }
        }
        fn element_types(&self, _: &str, _: &str) -> Option<Vec<String>> {
            None
        }
        fn is_primitive(&self, t: &str) -> bool {
            matches!(t, "string" | "code" | "integer")
        }
        fn is_resource_type(&self, _: &str) -> bool {
            false
        }
    }

    // covers: path.api.001
    #[test]
    fn text_evaluates_and_a_parse_failure_is_a_parse_error() {
        let e = Evaluator::new(&StubModel);
        let values = e.evaluate_text("1 + 1", None).expect("evaluates");
        assert_eq!(shown(values), shown(vec![Value::Integer(2)]));
        assert!(matches!(
            e.evaluate_text("1 +", None),
            Err(PathError::Parse(_))
        ));
    }

    // covers: path.api.003, path.api.004
    #[test]
    fn evaluate_first_is_the_first_or_none() {
        let e = Evaluator::new(&StubModel);
        assert_eq!(
            shown(e.evaluate_first_text("(3 | 4)", None).expect("evaluates")),
            shown(Some(Value::Integer(3)))
        );
        assert!(
            e.evaluate_first_text("{}", None)
                .expect("evaluates")
                .is_none()
        );
        assert_eq!(
            shown(e.evaluate_first_text("''", None).expect("evaluates")),
            shown(Some(Value::String(String::new())))
        );
    }

    // covers: path.api.005
    #[test]
    fn an_unexpected_type_names_both_types_and_empty_satisfies_any() {
        let e = Evaluator::new(&StubModel);
        match e.evaluate_text_as("1", None, "string") {
            Err(PathError::Evaluate(EvalError::Execution { message, offset })) => {
                // path.api.005: the item's Java simple name and the requested
                // type's fully qualified name, read from the generated class
                // table; an integer the engine builds is an `IntegerType`.
                assert_eq!(
                    message,
                    "HAPI-0256: FhirPath expression returned unexpected type IntegerType - \
                     Expected org.hl7.fhir.r4.model.StringType"
                );
                assert_eq!(offset, None);
            }
            other => panic!("expected the unexpected-type error, got {other:?}"),
        }
        assert!(e.evaluate_text_as("'a'", None, "string").is_ok());
        assert!(e.evaluate_text_as("'a'", None, "Element").is_ok());
        assert!(e.evaluate_text_as("'a'", None, "Base").is_ok());
        assert!(
            e.evaluate_text_as("{}", None, "HumanName")
                .expect("empty")
                .is_empty()
        );
    }

    // covers: path.strictness.007
    #[test]
    fn legacy_mode_makes_not_equals_with_an_empty_operand_true() {
        let off = Evaluator::new(&StubModel).with_settings(EngineSettings::PRE_R5);
        assert!(off.evaluate_text("{} != 1", None).expect("ok").is_empty());
        let on = off.clone().with_legacy_mode(true);
        for text in ["{} != 1", "1 != {}", "{} != {}"] {
            assert_eq!(
                shown(on.evaluate_text(text, None).expect("ok")),
                shown(vec![Value::Boolean(true)]),
                "{text}"
            );
        }
        // `=` is not affected by legacy mode.
        assert!(on.evaluate_text("{} = 1", None).expect("ok").is_empty());
        assert!(on.settings().legacy_mode);
    }

    // covers: path.strictness.010
    #[test]
    fn liquid_mode_is_not_implemented_and_off_means_ordinary_parsing() {
        let e = Evaluator::for_version(&StubModel, "4.0.1").expect("valid");
        assert!(!e.settings().liquid_mode);
        assert!(matches!(e.parse("1 || 2"), Err(PathError::Parse(_))));
        let liquid = e.with_settings(EngineSettings {
            liquid_mode: true,
            ..EngineSettings::PRE_R5
        });
        assert!(liquid.parse("1").expect_err("refused").is_unsupported());
    }

    // covers: path.strictness.001
    #[test]
    fn an_evaluator_for_r4_reports_the_pre_r5_settings() {
        let e = Evaluator::for_version(&StubModel, "4.0.1").expect("valid");
        assert_eq!(e.settings(), EngineSettings::PRE_R5);
        assert_eq!(e.settings(), EngineSettings::server_facing());
        assert_eq!(
            Evaluator::new(&StubModel).settings(),
            EngineSettings::DECLARED
        );
    }

    // covers: path.strictness.005
    #[test]
    fn an_upper_case_type_name_is_known_and_matches_only_when_case_is_not_enforced() {
        let lenient = Evaluator::new(&StubModel).with_settings(EngineSettings::PRE_R5);
        let strict = Evaluator::new(&StubModel);
        assert_eq!(
            shown(lenient.evaluate_text("'x'.as(String)", None).expect("ok")),
            shown(vec![Value::String("x".to_string())])
        );
        assert!(
            strict
                .evaluate_text("'x'.as(String)", None)
                .expect("known, so empty rather than an error")
                .is_empty()
        );
        // Only an upper-case initial is retried lower-cased, as upstream's
        // worker does: `strinG` names nothing.
        assert!(lenient.evaluate_text("'x'.as(strinG)", None).is_err());
    }

    mod polymorphic {
        use std::sync::Arc;

        use super::*;
        use crate::seam::model::ElementInfo;
        use crate::seam::node::{Node, NodeError};
        use crate::seam::value::{Decimal, Primitive, Quantity};

        struct ObsModel;

        impl Model for ObsModel {
            fn is_type(&self, t: &str) -> bool {
                matches!(t, "Observation" | "Quantity" | "string")
            }
            fn base_type(&self, _: &str) -> Option<&str> {
                None
            }
            fn element_types(&self, _: &str, _: &str) -> Option<Vec<String>> {
                None
            }
            fn is_primitive(&self, t: &str) -> bool {
                t == "string"
            }
            fn is_resource_type(&self, t: &str) -> bool {
                t == "Observation"
            }
            fn elements(&self, t: &str) -> Option<Vec<ElementInfo>> {
                (t == "Observation").then(|| {
                    vec![ElementInfo {
                        name: "value".to_string(),
                        path: "Observation.value".to_string(),
                        types: vec!["Quantity".to_string(), "string".to_string()],
                        codes: vec!["Quantity".to_string(), "string".to_string()],
                        choice: true,
                        repeating: false,
                        content_reference: None,
                    }]
                })
            }
        }

        #[derive(Debug)]
        struct Obs;

        impl Node for Obs {
            fn type_name(&self) -> &str {
                "Observation"
            }
            fn is_resource(&self) -> bool {
                true
            }
            fn children(&self, name: &str) -> Result<Vec<Value>, NodeError> {
                Ok(if name == "value" {
                    vec![Value::Quantity(Quantity {
                        value: Decimal::parse("185").expect("decimal"),
                        unit: Some("lbs".to_string()),
                        system: None,
                        code: None,
                    })]
                } else {
                    Vec::new()
                })
            }
            fn all_children(&self) -> Result<Vec<(String, Value)>, NodeError> {
                Ok(vec![(
                    "value".to_string(),
                    self.children("value")?.remove(0),
                )])
            }
            fn primitive(&self) -> Option<Primitive> {
                None
            }
        }

        // covers: path.strictness.006
        #[test]
        fn a_typed_choice_name_navigates_only_when_allowed() {
            let obs = Value::Node(Arc::new(Obs));
            let off = Evaluator::new(&ObsModel).with_settings(EngineSettings::PRE_R5);
            let on = off.clone().with_polymorphic_names(true);
            assert!(on.settings().allow_polymorphic_names);
            let exists = |e: &Evaluator<'_>, text: &str| {
                shown(e.evaluate_text(text, Some(&obs)).expect("evaluates"))
            };
            let t = shown(vec![Value::Boolean(true)]);
            let f = shown(vec![Value::Boolean(false)]);
            assert_eq!(exists(&off, "Observation.valueQuantity.exists()"), f);
            assert_eq!(exists(&on, "Observation.valueQuantity.exists()"), t);
            assert_eq!(exists(&on, "Observation.valuequantity.exists()"), t);
            assert_eq!(exists(&on, "Observation.valueString.exists()"), f);
            // `name.startsWith(n)` matches the choice element's own name with
            // an empty rest, which no type equals.
            assert_eq!(exists(&on, "Observation.value.exists()"), f);
            assert_eq!(exists(&off, "Observation.value.exists()"), t);
        }
    }
}
