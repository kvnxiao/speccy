#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
//! Persona-name registry tests. Host-native files
//! (`.claude/agents/reviewer-<persona>.md` and the Codex equivalent)
//! are the canonical persona surface; this test pins the registry
//! contents that the renderer derives those files from.

use speccy_core::personas::ALL;

const EXPECTED: &[&str] = &[
    "business",
    "tests",
    "security",
    "style",
    "architecture",
    "docs",
];

#[test]
fn registry_contains_six_personas_in_declared_order() {
    assert_eq!(
        ALL, EXPECTED,
        "ALL must list the six personas in the order business, tests, security, style, architecture, docs",
    );
    assert_eq!(ALL.len(), 6, "registry must contain exactly six entries");
}

#[test]
fn registry_default_personas_is_first_four_prefix() {
    let default = ALL.get(..4).expect("ALL must have at least 4 elements");
    assert_eq!(
        default,
        &["business", "tests", "security", "style"],
        "DEFAULT_PERSONAS (SPEC-0007) is mechanically derived as &ALL[..4]",
    );
}

#[test]
fn registry_personas_are_unique() {
    let mut seen: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for name in ALL {
        assert!(
            seen.insert(name),
            "persona {name} appears more than once in ALL",
        );
    }
}
