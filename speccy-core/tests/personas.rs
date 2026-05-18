#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
//! Persona registry tests (SPEC-0009 CHK-001/CHK-002).
//!
//! SPEC-0027 retired the project-local override resolver chain
//! (`resolve_file`, `resolve_file_with_warn`, `persona_file_name`,
//! `PersonaError`, the speccy-core-side `PERSONAS` static); host-native
//! files (`.claude/agents/reviewer-<persona>.md` and the Codex
//! equivalent) are now the sole canonical persona surface. The seven
//! resolver-chain tests this file used to host have been deleted along
//! with the resolver itself. What remains is the persona-name registry
//! — the only public surface of `speccy_core::personas` that survives.

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
