#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
//! Integration tests for the `speccy check` selector parser.
//!
//! Covers SPEC-0017 CHK-001 (REQ-001: Selector parsing). Test names start
//! with `parser_` so the invocation
//! `cargo test -p speccy-cli --test check_selector -- parser` runs
//! exactly these tests.

use speccy_cli::check_selector::CheckSelector;
use speccy_cli::check_selector::SelectorError;
use speccy_cli::check_selector::parse_selector;
use speccy_core::task_lookup::TaskRef;

#[test]
fn parser_none_returns_all() {
    let result = parse_selector(None).expect("None must parse to CheckSelector::All");
    assert_eq!(result, CheckSelector::All);
}

#[test]
fn parser_qualified_task_uses_task_lookup_parse_ref() {
    let result = parse_selector(Some("SPEC-0010/T-002"))
        .expect("SPEC-0010/T-002 must parse to a qualified task");
    let expected = CheckSelector::Task(TaskRef::Qualified {
        spec_id: "SPEC-0010".to_owned(),
        task_id: "T-002".to_owned(),
    });
    assert_eq!(result, expected);
}

#[test]
fn parser_qualified_check() {
    let result = parse_selector(Some("SPEC-0010/CHK-001"))
        .expect("SPEC-0010/CHK-001 must parse to a qualified check");
    let expected = CheckSelector::QualifiedCheck {
        spec_id: "SPEC-0010".to_owned(),
        check_id: "CHK-001".to_owned(),
    };
    assert_eq!(result, expected);
}

#[test]
fn parser_bare_spec() {
    let result = parse_selector(Some("SPEC-0010")).expect("SPEC-0010 must parse to a bare spec");
    let expected = CheckSelector::Spec {
        spec_id: "SPEC-0010".to_owned(),
    };
    assert_eq!(result, expected);
}

#[test]
fn parser_unqualified_task() {
    let result = parse_selector(Some("T-002")).expect("T-002 must parse to an unqualified task");
    let expected = CheckSelector::Task(TaskRef::Unqualified {
        id: "T-002".to_owned(),
    });
    assert_eq!(result, expected);
}

#[test]
fn parser_unqualified_check() {
    let result =
        parse_selector(Some("CHK-001")).expect("CHK-001 must parse to an unqualified check");
    let expected = CheckSelector::UnqualifiedCheck {
        check_id: "CHK-001".to_owned(),
    };
    assert_eq!(result, expected);
}

#[test]
fn parser_unknown_input_errors_and_display_lists_five_shapes() {
    let err = parse_selector(Some("FOO")).expect_err("FOO must be rejected as InvalidFormat");
    assert!(
        matches!(&err, SelectorError::InvalidFormat { arg } if arg == "FOO"),
        "expected InvalidFormat{{FOO}}, got {err:?}",
    );

    let display = format!("{err}");
    assert!(
        display.contains("FOO"),
        "Display must name the offending input verbatim, got: {display}",
    );
    for shape in &[
        "SPEC-NNNN",
        "SPEC-NNNN/CHK-NNN",
        "SPEC-NNNN/T-NNN",
        "CHK-NNN",
        "T-NNN",
    ] {
        assert!(
            display.contains(shape),
            "Display must mention shape `{shape}`, got: {display}",
        );
    }
}

#[test]
fn parser_lowercase_chk_not_normalised() {
    let err =
        parse_selector(Some("chk-001")).expect_err("chk-001 must be rejected (case mismatch)");
    assert!(
        matches!(&err, SelectorError::InvalidFormat { arg } if arg == "chk-001"),
        "expected InvalidFormat{{chk-001}}, got {err:?}",
    );
}

#[test]
fn parser_rejects_malformed_inputs_carrying_exact_arg() {
    for bad in &["", "SPEC-", "SPEC-001", "SPEC-0001/", "/T-001", "T- 001"] {
        let err = parse_selector(Some(bad))
            .expect_err("malformed input must surface SelectorError::InvalidFormat");
        assert!(
            matches!(&err, SelectorError::InvalidFormat { arg } if arg == bad),
            "expected InvalidFormat carrying `{bad}` verbatim, got {err:?}",
        );
    }
}

#[test]
fn parser_dispatch_order_picks_most_specific_shape() {
    // Each accepted form maps to exactly one variant; walks the dispatch
    // ladder so a future regression that swaps two arms shows up here.
    let cases: &[(&str, CheckSelector)] = &[
        (
            "SPEC-0010/T-002",
            CheckSelector::Task(TaskRef::Qualified {
                spec_id: "SPEC-0010".to_owned(),
                task_id: "T-002".to_owned(),
            }),
        ),
        (
            "SPEC-0010/CHK-001",
            CheckSelector::QualifiedCheck {
                spec_id: "SPEC-0010".to_owned(),
                check_id: "CHK-001".to_owned(),
            },
        ),
        (
            "SPEC-0010",
            CheckSelector::Spec {
                spec_id: "SPEC-0010".to_owned(),
            },
        ),
        (
            "T-002",
            CheckSelector::Task(TaskRef::Unqualified {
                id: "T-002".to_owned(),
            }),
        ),
        (
            "CHK-001",
            CheckSelector::UnqualifiedCheck {
                check_id: "CHK-001".to_owned(),
            },
        ),
    ];
    for (input, expected) in cases {
        let got = parse_selector(Some(input)).expect("each accepted form must parse without error");
        assert_eq!(
            &got, expected,
            "dispatch order regression for input `{input}`: got {got:?}, want {expected:?}",
        );
    }
}
