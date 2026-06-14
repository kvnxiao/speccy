#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
//! Persona-name registry tests. Host-native files
//! (`.claude/agents/reviewer-<persona>.md` and the Codex equivalent)
//! are the canonical persona surface; this test pins the registry
//! contents that the renderer derives those files from.

use speccy_core::personas::ALL;

#[test]
fn registry_default_personas_is_first_five_prefix() {
    let default = ALL.get(..5).expect("ALL must have at least 5 elements");
    assert_eq!(
        default,
        &["business", "tests", "security", "style", "correctness"],
        "DEFAULT_PERSONAS is mechanically derived as &ALL[..5]",
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
