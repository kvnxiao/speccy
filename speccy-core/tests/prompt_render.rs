#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
//! Integration tests for `speccy_core::prompt::render`.
//! Covers SPEC-0005 REQ-005 via the public API (the wrapper sinks to
//! process stderr; inline unit tests use the injected-sink form to
//! assert warning text).

use speccy_core::prompt::render;
use std::collections::BTreeMap;

fn vars(pairs: &[(&'static str, &str)]) -> BTreeMap<&'static str, String> {
    pairs.iter().map(|(k, v)| (*k, (*v).to_owned())).collect()
}

#[test]
fn happy_path_substitutes_single_placeholder() {
    let out = render("hello {{name}}", &vars(&[("name", "world")]));
    assert_eq!(out, "hello world");
}

#[test]
fn happy_path_substitutes_multiple_placeholders() {
    let out = render(
        "{{greeting}} {{name}}!",
        &vars(&[("greeting", "hi"), ("name", "kev")]),
    );
    assert_eq!(out, "hi kev!");
}

#[test]
fn happy_path_single_pass_does_not_rescan_substituted_text() {
    let out = render("{{a}} {{b}}", &vars(&[("a", "{{b}}"), ("b", "x")]));
    assert_eq!(
        out, "{{b}} x",
        "substituted {{{{b}}}} from a must NOT be re-scanned",
    );
}

#[test]
fn unknown_placeholders_are_emitted_verbatim() {
    let out = render("{{unknown}}", &vars(&[]));
    assert_eq!(out, "{{unknown}}");
}

#[test]
fn unknown_placeholders_amid_known_placeholders_emit_verbatim() {
    let out = render("known={{a}} unknown={{nope}}", &vars(&[("a", "yes")]));
    assert_eq!(out, "known=yes unknown={{nope}}");
}
