//! Unit tests for `xhtml.rs`: the narrative's XML-mode recomposition
//! (`path.navigation.025`).

use super::xhtml::{JavaHashMap, XhtmlError, compose_narrative, java_hash};

const NS: &str = r#"xmlns="http://www.w3.org/1999/xhtml""#;

fn compose(s: &str) -> String {
    compose_narrative(s).unwrap_or_else(|e| panic!("{s:?}: {e:?}"))
}

// covers: path.navigation.025
#[test]
fn a_plain_narrative_round_trips() {
    let div = format!("<div {NS}>hi</div>");
    assert_eq!(compose(&div), div);
    let list = format!("<div {NS}>x<ul><li>a</li><li>b</li></ul></div>");
    assert_eq!(compose(&list), list);
}

// covers: path.navigation.025
#[test]
fn empty_elements_self_close_and_quotes_are_escaped() {
    assert_eq!(
        compose(&format!(r#"<div {NS}>say "a"<br></br>b</div>"#)),
        format!(r#"<div {NS}>say &quot;a&quot;<br/>b</div>"#)
    );
}

// covers: path.navigation.025
#[test]
fn a_missing_namespace_is_added_and_bare_text_is_wrapped() {
    assert_eq!(compose("<div>x</div>"), format!("<div {NS}>x</div>"));
    assert_eq!(compose("  x  "), format!("<div {NS}>x</div>"));
}

// covers: path.navigation.025
#[test]
fn entities_are_decoded_and_re_escaped() {
    assert_eq!(
        compose(&format!(
            "<div {NS}>a&amp;b &lt; &#65;&#x42;&nbsp;&copy;</div>"
        )),
        format!("<div {NS}>a&amp;b &lt; AB\u{a0}\u{a9}</div>")
    );
    assert_eq!(
        compose(&format!("<div {NS}>\u{1F600}</div>")),
        format!("<div {NS}>&#x1F600;</div>")
    );
}

// covers: path.navigation.025
#[test]
fn an_attribute_with_no_value_is_written_as_null() {
    assert_eq!(
        compose(&format!(
            r#"<div {NS}><input disabled/><a title="">x</a></div>"#
        )),
        format!(r#"<div {NS}><input disabled="null"/><a title="null">x</a></div>"#)
    );
}

// covers: path.navigation.025
#[test]
fn attributes_follow_java_hash_map_order() {
    // Spread hashes and 16-bucket indexes: xmlns 2, style 10, id 11,
    // href 11, title 12. `href` and `id` share a bucket, in insertion order.
    assert_eq!(
        compose(&format!(
            r#"<div {NS} title="t" id="i" style="s" href="h">x</div>"#
        )),
        format!(r#"<div {NS} style="s" id="i" href="h" title="t">x</div>"#)
    );
}

// covers: path.navigation.025
#[test]
fn comments_and_cdata_are_kept() {
    assert_eq!(
        compose(&format!("<div {NS}>a<!--  c  --><![CDATA[<x>]]></div>")),
        format!("<div {NS}>a<!-- c --><![CDATA[<x>]]></div>")
    );
}

// covers: path.navigation.025
#[test]
fn a_mismatched_end_tag_and_an_empty_value_are_errors() {
    assert!(matches!(
        compose_narrative(&format!("<div {NS}><p>x</b></div>")),
        Err(XhtmlError::Format(_))
    ));
    assert_eq!(compose_narrative(""), Err(XhtmlError::NullNodeType));
    assert_eq!(
        compose_narrative("<?xml version=\"1.0\"?>"),
        Err(XhtmlError::NullNodeType)
    );
}

// covers: path.navigation.025
#[test]
fn a_prefixed_root_is_renamed_and_its_namespace_stated() {
    assert_eq!(
        compose(r#"<h:div xmlns:h="http://www.w3.org/1999/xhtml">x</h:div>"#),
        format!("<div {NS}>x</div>")
    );
}

// covers: path.navigation.025
#[test]
fn java_hash_map_resizes_as_java_does() {
    let mut m = JavaHashMap::new();
    for i in 0..13 {
        m.put(format!("k{i}"), None);
    }
    assert_eq!(m.capacity(), 32);
    let copy = JavaHashMap::copy_of(&m);
    // tableSizeFor((int)(13 / 0.75 + 1)) = tableSizeFor(18) = 32.
    assert_eq!(copy.capacity(), 32);
    assert_eq!(java_hash("xmlns"), 114_176_722);
    assert_eq!(java_hash("id"), 3355);
}
