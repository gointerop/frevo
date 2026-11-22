//! Unit tests for the utility functions -- `trace` and its log, `check`,
//! `aggregate`, `sort` and `extension(url)` -- at the crate's own surface.
//!
//! `tests/path_utility.rs` reads the log through the oracle harness, whose
//! `has_log` and `take_log` are bound to an unbuilt stub, and checks every
//! expression statically first; HAPI's check pass rejects `check(given.exists(),
//! ...)` on a `Patient`, because `check`'s arguments are typed against the
//! context and not the focus. These tests evaluate directly.

use super::env_tests::{Host, StubModel, describe, failure, node, prim};
use super::{EvalError, Evaluator};
use crate::seam::value::Value;
use crate::syntax::ast::Expression;

fn name(use_: &str, family: Option<&str>, given: &[&str]) -> Value {
    let mut children = vec![("use", prim("code", use_))];
    if let Some(f) = family {
        children.push(("family", prim("string", f)));
    }
    for g in given {
        children.push(("given", prim("string", g)));
    }
    node("HumanName", false, children)
}

/// Three names carrying five given names, as `patient-example.xml` does, and
/// a birth date with one extension reachable from the primitive.
fn patient() -> Value {
    let ext = node(
        "Extension",
        false,
        vec![
            (
                "url",
                prim(
                    "uri",
                    "http://hl7.org/fhir/StructureDefinition/patient-birthTime",
                ),
            ),
            ("value", prim("dateTime", "1974-12-25T14:35:45-05:00")),
        ],
    );
    let birth = Value::Node(std::sync::Arc::new(Sibling {
        inner: prim("date", "1974-12-25"),
        extensions: vec![ext],
    }));
    node(
        "Patient",
        true,
        vec![
            ("name", name("usual", Some("Chalmers"), &["Peter", "James"])),
            ("name", name("official", Some("Windsor"), &["Jim"])),
            ("name", name("maiden", None, &["Peter", "James"])),
            ("birthDate", birth),
        ],
    )
}

/// A date primitive whose `extension` child is listed, as the model lists it.
#[derive(Debug)]
struct Sibling {
    inner: Value,
    extensions: Vec<Value>,
}

impl crate::seam::node::Node for Sibling {
    fn type_name(&self) -> &str {
        "date"
    }
    fn is_resource(&self) -> bool {
        false
    }
    fn children(&self, name: &str) -> Result<Vec<Value>, crate::seam::node::NodeError> {
        Ok(if name == "extension" {
            self.extensions.clone()
        } else {
            Vec::new()
        })
    }
    fn all_children(&self) -> Result<Vec<(String, Value)>, crate::seam::node::NodeError> {
        Ok(self
            .extensions
            .iter()
            .map(|e| ("extension".to_string(), e.clone()))
            .collect())
    }
    fn primitive(&self) -> Option<crate::seam::value::Primitive> {
        match &self.inner {
            Value::Node(n) => n.primitive(),
            _ => None,
        }
    }
}

fn run(e: &Evaluator<'_>, text: &str, focus: Option<&Value>) -> Result<Vec<String>, EvalError> {
    let expr = Expression::parse(text).unwrap_or_else(|x| panic!("{text:?} must parse: {x}"));
    e.evaluate(&expr, focus)
        .map(|v| v.iter().map(describe).collect())
}

// covers: path.utility.001
#[test]
fn trace_passes_its_input_through_and_records_a_log_line() {
    let p = patient();
    let e = Evaluator::new(&StubModel);
    assert_eq!(
        run(&e, "name.given.trace('test').count()", Some(&p)).expect("ok"),
        ["integer:5"]
    );
    assert!(e.has_log());
    assert_eq!(e.take_log(), "test: Peter,James,Jim,Peter,James");
    // Taking clears: a second take is empty and the predicate is false.
    assert!(!e.has_log());
    assert_eq!(e.take_log(), "");

    // Two traces are joined by `; ` in evaluation order.
    run(&e, "name.given.trace('a').count().trace('b')", Some(&p)).expect("ok");
    assert_eq!(e.take_log(), "a: Peter,James,Jim,Peter,James; b: 5");

    // No trace, no log.
    run(&e, "name.given.count()", Some(&p)).expect("ok");
    assert!(!e.has_log());
    assert_eq!(e.take_log(), "");
}

// covers: path.utility.001
#[test]
fn an_evaluation_starts_a_fresh_log_and_a_clone_shares_it() {
    let p = patient();
    let e = Evaluator::new(&StubModel);
    run(&e, "name.trace('one')", Some(&p)).expect("ok");
    // `evaluate` starts `log = new StringBuilder()`.
    run(&e, "name.trace('two')", Some(&p)).expect("ok");
    assert_eq!(e.take_log(), "two: HumanName,HumanName,HumanName");
    let c = e.clone();
    run(&c, "'x'.trace('shared')", None).expect("ok");
    assert_eq!(e.take_log(), "shared: x");
}

// covers: path.utility.002
#[test]
fn trace_with_a_projection_logs_the_projection_and_returns_the_focus() {
    let p = patient();
    let e = Evaluator::new(&StubModel);
    assert_eq!(
        run(&e, "name.trace('test', given).count()", Some(&p)).expect("ok"),
        ["integer:3"]
    );
    assert_eq!(e.take_log(), "test: Peter,James,Jim,Peter,James");

    run(
        &e,
        "'[stop]ONE[stop][stop]TWO[stop]'.split('[stop]').trace('n').count()",
        None,
    )
    .expect("ok");
    assert!(e.take_log().starts_with("n: "));
}

// covers: path.utility.028
#[test]
fn a_host_service_that_answers_true_suppresses_the_engines_log() {
    let p = patient();
    let host = Host::logging(true);
    let e = Evaluator::new(&StubModel).with_host_services(&host);
    run(&e, "name.trace('t')", Some(&p)).expect("ok");
    assert!(!e.has_log());
    assert_eq!(host.logged.lock().expect("ok").as_slice(), ["t/3"]);

    let host = Host::default();
    let e = Evaluator::new(&StubModel).with_host_services(&host);
    run(&e, "name.trace('t')", Some(&p)).expect("ok");
    assert!(e.has_log());
}

// covers: path.utility.003, path.utility.004
#[test]
fn check_passes_its_input_through_and_fails_with_the_callers_text() {
    let p = patient();
    let e = Evaluator::new(&StubModel);
    assert_eq!(
        run(
            &e,
            "name.check(given.exists(), 'given must exist').count()",
            Some(&p)
        )
        .expect("ok"),
        ["integer:3"]
    );
    assert_eq!(
        run(&e, "name.first().check(true, 'always').family", Some(&p)).expect("ok"),
        ["string:Chalmers"]
    );
    let message = failure(run(
        &e,
        "name.check(prefix.exists(), 'every name needs a prefix')",
        Some(&p),
    ));
    assert!(message.contains("every name needs a prefix"), "{message}");
    failure(run(&e, "name.check(false, 'never')", Some(&p)));
    failure(run(&e, "{}.check(false, 'never').empty()", None));
}

// covers: path.utility.005, path.utility.006
#[test]
fn aggregate_folds_with_this_and_total() {
    let e = Evaluator::new(&StubModel);
    assert_eq!(
        run(&e, "(1|2|3|4|5|6|7|8|9).aggregate($this+$total, 0)", None).expect("ok"),
        ["integer:45"]
    );
    assert_eq!(
        run(&e, "(1|2|3).aggregate($this+$total, 10)", None).expect("ok"),
        ["integer:16"]
    );
    // No initial value: `$total` is empty on the first item.
    assert_eq!(
        run(
            &e,
            "(1|2|3|4|5|6|7|8|9).aggregate(iif($total.empty(), $this, iif($this < $total, $this, $total)))",
            None
        )
        .expect("ok"),
        ["integer:1"]
    );
    assert_eq!(
        run(
            &e,
            "(1|2|3|4|5|6|7|8|9).aggregate(iif($total.empty(), $this, iif($this > $total, $this, $total)))",
            None
        )
        .expect("ok"),
        ["integer:9"]
    );
    // An empty focus folds to the empty initial total.
    assert!(
        run(&e, "{}.aggregate($this+$total)", None)
            .expect("ok")
            .is_empty()
    );
    assert_eq!(
        run(&e, "{}.aggregate($this+$total, 7)", None).expect("ok"),
        ["integer:7"]
    );
}

// covers: path.utility.007, path.utility.008
#[test]
fn sort_orders_by_natural_order_one_key_or_several_with_descending() {
    let p = patient();
    let e = Evaluator::new(&StubModel);
    assert_eq!(
        run(&e, "(3 | 2 | 1).sort()", None).expect("ok"),
        ["integer:1", "integer:2", "integer:3"]
    );
    assert_eq!(
        run(&e, "('c' | 'b' | 'a').sort($this)", None).expect("ok"),
        ["string:a", "string:b", "string:c"]
    );
    assert_eq!(
        run(&e, "(1 | 2 | 3).sort(-$this)", None).expect("ok"),
        ["integer:3", "integer:2", "integer:1"]
    );
    // A descending key reverses the whole comparison, so the name with no
    // `family` -- which sorts last ascending -- sorts first.
    assert_eq!(
        run(&e, "name.sort(-family).first().use", Some(&p)).expect("ok"),
        ["code:maiden"]
    );
    // Several keys: the second decides only where the first ties.
    assert_eq!(
        run(
            &e,
            "name.sort(-family.length(), -given.first()).use",
            Some(&p)
        )
        .expect("ok"),
        ["code:maiden", "code:usual", "code:official"]
    );
    // A name with no `family` sorts last ascending.
    assert_eq!(
        run(&e, "name.sort(family).last().use", Some(&p)).expect("ok"),
        ["code:maiden"]
    );
    // The sort is stable.
    assert_eq!(
        run(&e, "name.sort(given.first()).use", Some(&p)).expect("ok"),
        ["code:official", "code:usual", "code:maiden"]
    );
    // A key that yields several values, and a mixed-type natural sort, fail.
    failure(run(&e, "name.sort(given)", Some(&p)));
    failure(run(&e, "(1 | 'a').sort()", None));
}

// covers: path.utility.009, path.utility.010, path.utility.011
#[test]
fn extension_selects_by_url_including_on_a_primitive() {
    let p = patient();
    let e = Evaluator::new(&StubModel);
    const URL: &str = "http://hl7.org/fhir/StructureDefinition/patient-birthTime";
    assert_eq!(
        run(
            &e,
            &format!("birthDate.extension('{URL}').count()"),
            Some(&p)
        )
        .expect("ok"),
        ["integer:1"]
    );
    assert_eq!(
        run(&e, &format!("birthDate.extension('{URL}').url"), Some(&p)).expect("ok"),
        [format!("uri:{URL}")]
    );
    assert_eq!(
        run(&e, &format!("birthDate.extension('{URL}').value"), Some(&p)).expect("ok"),
        ["dateTime:1974-12-25T14:35:45-05:00"]
    );
    assert!(
        run(
            &e,
            "birthDate.extension('http://example.org/nothing')",
            Some(&p)
        )
        .expect("ok")
        .is_empty()
    );
    // An element with no extensions is empty, not a failure.
    assert!(
        run(&e, "name.extension('http://x')", Some(&p))
            .expect("ok")
            .is_empty()
    );
    // The shorthand reaches the same extension.
    assert_eq!(
        run(
            &e,
            "birthDate.extension(%`ext-patient-birthTime`).count()",
            Some(&p)
        )
        .expect("ok"),
        ["integer:1"]
    );
}
