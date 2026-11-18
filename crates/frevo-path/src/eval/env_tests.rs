//! Unit tests for the environment variables and the constant-resolution sites,
//! over an in-memory node and a stub model.
//!
//! The host-service scenarios in `tests/path_env.rs` are driven through a
//! host-aware seam the committed test file leaves unbound, so these cover the
//! same behaviour at the crate's own surface: [`Evaluator::with_host_services`],
//! [`Evaluator::with_application_context`] and [`Evaluator::evaluate_in`].

use std::sync::{Arc, Mutex};

use crate::seam::host::{ConstantMode, FunctionDetails, HostError, HostServices};
use crate::seam::model::{Model, TypeSet};
use crate::seam::node::{Node, NodeError};
use crate::seam::server::ServerHostServices;
use crate::seam::value::{Primitive, Value};
use crate::syntax::ast::Expression;

use super::{EvalError, Evaluator};

#[derive(Debug)]
struct TestNode {
    type_name: String,
    is_resource: bool,
    children: Vec<(String, Value)>,
    primitive: Option<Primitive>,
}

impl Node for TestNode {
    fn type_name(&self) -> &str {
        &self.type_name
    }
    fn is_resource(&self) -> bool {
        self.is_resource
    }
    fn children(&self, name: &str) -> Result<Vec<Value>, NodeError> {
        Ok(self
            .children
            .iter()
            .filter(|(n, _)| n == name)
            .map(|(_, v)| v.clone())
            .collect())
    }
    fn all_children(&self) -> Result<Vec<(String, Value)>, NodeError> {
        Ok(self.children.clone())
    }
    fn primitive(&self) -> Option<Primitive> {
        self.primitive.clone()
    }
}

fn node(type_name: &str, is_resource: bool, children: Vec<(&str, Value)>) -> Value {
    Value::Node(Arc::new(TestNode {
        type_name: type_name.to_string(),
        is_resource,
        children: children
            .into_iter()
            .map(|(n, v)| (n.to_string(), v))
            .collect(),
        primitive: None,
    }))
}

fn prim(type_name: &str, lexical: &str) -> Value {
    Value::Node(Arc::new(TestNode {
        type_name: type_name.to_string(),
        is_resource: false,
        children: Vec::new(),
        primitive: Some(Primitive {
            type_name: type_name.to_string(),
            lexical: lexical.to_string(),
            id: None,
            extensions: Vec::new(),
        }),
    }))
}

struct StubModel;

impl Model for StubModel {
    fn is_type(&self, type_name: &str) -> bool {
        matches!(
            type_name,
            "Patient" | "Organization" | "HumanName" | "string"
        )
    }
    fn base_type(&self, _type_name: &str) -> Option<&str> {
        None
    }
    fn element_types(&self, _type_name: &str, _element: &str) -> Option<Vec<String>> {
        None
    }
    fn is_primitive(&self, type_name: &str) -> bool {
        type_name.chars().next().is_some_and(char::is_lowercase)
    }
    fn is_resource_type(&self, type_name: &str) -> bool {
        matches!(type_name, "Patient" | "Organization")
    }
}

/// A `Patient` containing an `Organization`, with two names.
fn patient() -> Value {
    node(
        "Patient",
        true,
        vec![
            ("id", prim("id", "p1")),
            (
                "contained",
                node("Organization", true, vec![("id", prim("id", "o1"))]),
            ),
            (
                "name",
                node("HumanName", false, vec![("family", prim("string", "A"))]),
            ),
            (
                "name",
                node("HumanName", false, vec![("family", prim("string", "B"))]),
            ),
        ],
    )
}

/// Answers from a table and records every question with its mode.
#[derive(Default)]
struct Host {
    constants: Vec<(String, Vec<String>)>,
    /// When set, the constants are answered in this mode only.
    only_mode: Option<ConstantMode>,
    asked: Mutex<Vec<(String, ConstantMode)>>,
}

impl Host {
    fn owning(name: &str, values: &[&str]) -> Self {
        Host {
            constants: vec![(
                name.to_string(),
                values.iter().map(|v| (*v).to_string()).collect(),
            )],
            only_mode: None,
            asked: Mutex::new(Vec::new()),
        }
    }

    fn owning_in(mode: ConstantMode, name: &str, values: &[&str]) -> Self {
        Host {
            only_mode: Some(mode),
            ..Host::owning(name, values)
        }
    }

    fn asked(&self) -> Vec<(String, ConstantMode)> {
        self.asked.lock().expect("not poisoned").clone()
    }
}

impl HostServices for Host {
    fn resolve_constant(&self, name: &str, mode: ConstantMode) -> Result<Vec<Value>, HostError> {
        self.asked
            .lock()
            .expect("not poisoned")
            .push((name.to_string(), mode));
        if self.only_mode.is_some_and(|m| m != mode) {
            return Ok(Vec::new());
        }
        Ok(self
            .constants
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, vs)| vs.iter().map(|v| Value::String(v.clone())).collect())
            .unwrap_or_default())
    }
    fn resolve_constant_type(
        &self,
        _name: &str,
        _mode: ConstantMode,
    ) -> Result<Option<TypeSet>, HostError> {
        Ok(None)
    }
    fn resolve_reference(
        &self,
        _url: &str,
        _identifier: Option<&Value>,
        _context: Option<&Value>,
    ) -> Result<Option<Value>, HostError> {
        Ok(None)
    }
    fn conforms_to_profile(&self, _item: &Value, _url: &str) -> Result<bool, HostError> {
        Ok(false)
    }
    fn resolve_value_set(&self, _url: &str) -> Result<Option<Value>, HostError> {
        Ok(None)
    }
    fn resolve_function(&self, _name: &str) -> Option<FunctionDetails> {
        None
    }
    fn check_function(
        &self,
        name: &str,
        _focus: &TypeSet,
        _params: &[TypeSet],
    ) -> Result<TypeSet, HostError> {
        Err(HostError::new(name))
    }
    fn execute_function(
        &self,
        name: &str,
        _focus: &[Value],
        _params: &[Vec<Value>],
    ) -> Result<Vec<Value>, HostError> {
        Err(HostError::new(name))
    }
    fn param_is_type(&self, _name: &str, _index: usize) -> bool {
        false
    }
    fn log(&self, _name: &str, _focus: &[Value]) -> bool {
        false
    }
}

fn describe(value: &Value) -> String {
    match value {
        Value::String(s) => format!("string:{s}"),
        Value::Integer(i) => format!("integer:{i}"),
        Value::Boolean(b) => format!("boolean:{b}"),
        Value::Node(n) => match n.primitive() {
            Some(p) => format!("{}:{}", p.type_name, p.lexical),
            None => format!("{}:<element>", n.type_name()),
        },
        _ => "other".to_string(),
    }
}

fn run(
    evaluator: Evaluator<'_>,
    text: &str,
    focus: Option<&Value>,
) -> Result<Vec<String>, EvalError> {
    let expr = Expression::parse(text).unwrap_or_else(|e| panic!("{text:?} must parse: {e}"));
    evaluator
        .evaluate(&expr, focus)
        .map(|v| v.iter().map(describe).collect())
}

fn failure(result: Result<Vec<String>, EvalError>) -> String {
    match result {
        Err(EvalError::Execution { message, .. }) => message,
        other => panic!("expected an execution error, got {other:?}"),
    }
}

// covers: path.env.012
#[test]
fn a_host_service_answers_a_collection_for_an_explicit_name() {
    let host = Host::owning("dummyConstant", &["one", "two"]);
    let e = Evaluator::new(&StubModel).with_host_services(&host);
    assert_eq!(
        run(e, "%dummyConstant", None).expect("evaluates"),
        ["string:one", "string:two"]
    );
    assert_eq!(
        run(e, "%dummyConstant.count()", None).expect("evaluates"),
        ["integer:2"]
    );
    assert_eq!(
        host.asked()[0],
        ("dummyConstant".to_string(), ConstantMode::Explicit)
    );
}

// covers: path.env.011
#[test]
fn an_unknown_name_with_no_host_service_fails_and_with_one_that_owns_nothing_is_empty() {
    let p = patient();
    let message = failure(run(Evaluator::new(&StubModel), "%nosuchvariable", Some(&p)));
    assert!(message.contains("nosuchvariable"), "{message}");

    let host = Host::default();
    let e = Evaluator::new(&StubModel).with_host_services(&host);
    assert!(run(e, "%nosuchvariable", Some(&p)).expect("ok").is_empty());
    assert_eq!(
        host.asked(),
        [("nosuchvariable".into(), ConstantMode::Explicit)]
    );
}

// covers: path.env.006, path.env.007, path.env.008, path.env.009, path.env.010
#[test]
fn resource_root_resource_and_context_come_from_the_evaluation() {
    let p = patient();
    let e = Evaluator::new(&StubModel);

    // Started from the resource, all three are the resource.
    assert_eq!(run(e, "%resource.id", Some(&p)).expect("ok"), ["id:p1"]);
    assert_eq!(run(e, "%context.id", Some(&p)).expect("ok"), ["id:p1"]);

    // Started from a non-resource with nothing supplied: the guards fire, and
    // name the constant.
    let name = {
        let Value::Node(n) = &p else { unreachable!() };
        n.children("name").expect("ok").remove(0)
    };
    assert!(failure(run(e, "%resource", Some(&name))).contains("%resource"));
    assert!(failure(run(e, "%rootResource", Some(&name))).contains("%rootResource"));
    assert!(failure(run(e, "%resource", None)).contains("%resource"));

    // With the enclosing resource supplied, `%resource` and `%rootResource`
    // can differ: the contained one and the outer one.
    let contained = {
        let Value::Node(n) = &p else { unreachable!() };
        n.children("contained").expect("ok").remove(0)
    };
    let expr = Expression::parse("%resource.id").expect("parses");
    let got = e
        .evaluate_in(&expr, Some(&contained), Some(&contained), Some(&p))
        .expect("ok");
    assert_eq!(got.iter().map(describe).collect::<Vec<_>>(), ["id:o1"]);
    let expr = Expression::parse("%rootResource.id").expect("parses");
    let got = e
        .evaluate_in(&expr, Some(&contained), Some(&contained), Some(&p))
        .expect("ok");
    assert_eq!(got.iter().map(describe).collect::<Vec<_>>(), ["id:p1"]);

    // `%context` is the node evaluation started from, not the resource.
    let expr = Expression::parse("%context.family").expect("parses");
    let got = e
        .evaluate_in(&expr, Some(&name), Some(&p), Some(&p))
        .expect("ok");
    assert_eq!(got.iter().map(describe).collect::<Vec<_>>(), ["string:A"]);
}

// covers: path.env.013, path.env.014
#[test]
fn a_defined_variable_is_visible_to_the_rest_of_its_chain_and_nowhere_else() {
    let p = patient();
    let e = Evaluator::new(&StubModel);
    assert_eq!(
        run(
            e,
            "name.first().defineVariable('n', family).select(%n)",
            Some(&p)
        )
        .expect("ok"),
        ["string:A"]
    );
    assert_eq!(
        run(e, "name.defineVariable('x', 'v').count()", Some(&p)).expect("ok"),
        ["integer:2"]
    );
    // Defined inside one argument, used outside it.
    let message = failure(run(
        e,
        "name.where(defineVariable('v', family).exists()).select(%v)",
        Some(&p),
    ));
    assert!(message.contains("%v"), "{message}");
    // A system name, and a name already defined, cannot be defined again.
    assert!(failure(run(e, "defineVariable('sct', 1)", None)).contains("cannot be redefined"));
    assert!(
        failure(run(
            e,
            "defineVariable('a', 1).defineVariable('a', 2)",
            None
        ))
        .contains("cannot be redefined")
    );
}

// covers: path.env.016
#[test]
fn a_host_constant_resolved_before_navigation_shadows_a_real_element() {
    let p = patient();
    let host = Host::owning("name", &["shadow"]);
    let on = Evaluator::new(&StubModel)
        .with_host_services(&host)
        .with_application_context(true);
    assert_eq!(run(on, "name", Some(&p)).expect("ok"), ["string:shadow"]);
    assert_eq!(
        host.asked()[0],
        ("name".into(), ConstantMode::ImplicitBefore)
    );

    // Without an application context the host service is never asked and the
    // real element is reached.
    let host = Host::owning("name", &["shadow"]);
    let off = Evaluator::new(&StubModel).with_host_services(&host);
    assert_eq!(
        run(off, "name.family", Some(&p)).expect("ok"),
        ["string:A", "string:B"]
    );
    assert!(host.asked().is_empty());
}

// covers: path.env.016
#[test]
fn the_leading_type_name_is_asked_about_before_it_is_matched() {
    let p = patient();
    let host = Host::owning("Patient", &["shadow"]);
    let on = Evaluator::new(&StubModel)
        .with_host_services(&host)
        .with_application_context(true);
    assert_eq!(run(on, "Patient", Some(&p)).expect("ok"), ["string:shadow"]);
}

// covers: path.env.017
#[test]
fn implicit_after_is_a_fall_through_gated_on_the_application_context() {
    let p = patient();
    let host = Host::owning_in(ConstantMode::ImplicitAfter, "nosuchelement", &["fallback"]);
    let on = Evaluator::new(&StubModel)
        .with_host_services(&host)
        .with_application_context(true);
    assert_eq!(
        run(on, "nosuchelement", Some(&p)).expect("ok"),
        ["string:fallback"]
    );
    assert_eq!(
        host.asked(),
        [
            ("nosuchelement".into(), ConstantMode::ImplicitBefore),
            ("nosuchelement".into(), ConstantMode::ImplicitAfter),
        ]
    );

    // A real element is found by navigation, so the fall-through is not asked.
    let host = Host::owning("name", &[]);
    let on = Evaluator::new(&StubModel)
        .with_host_services(&host)
        .with_application_context(true);
    run(on, "name", Some(&p)).expect("ok");
    assert_eq!(
        host.asked(),
        [("name".into(), ConstantMode::ImplicitBefore)]
    );

    // No application context: neither implicit mode is asked, whatever the
    // host service owns.
    let host = Host::owning("nosuchelement", &["fallback"]);
    let off = Evaluator::new(&StubModel).with_host_services(&host);
    assert!(run(off, "nosuchelement", Some(&p)).expect("ok").is_empty());
    assert!(host.asked().is_empty());
}

// covers: path.env.016
#[test]
fn the_server_host_service_leaves_a_resources_own_elements_reachable() {
    let p = patient();
    let host = ServerHostServices;
    let e = Evaluator::new(&StubModel)
        .with_host_services(&host)
        .with_application_context(true);
    assert_eq!(
        run(e, "name.family", Some(&p)).expect("ok"),
        ["string:A", "string:B"]
    );
    assert_eq!(run(e, "id", Some(&p)).expect("ok"), ["id:p1"]);
    assert!(run(e, "%unowned", Some(&p)).expect("ok").is_empty());
}
