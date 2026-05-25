//! Reviewer persona name registry.
//!
//! Six personas ship: the four default fan-out personas
//! ([`ALL`][`ALL`]`[..4]` = `business`, `tests`, `security`, `style`) plus
//! two off-by-default personas (`architecture`, `docs`). Adding a new
//! persona is a single-line change to [`ALL`]; SPEC-0007 consumes
//! `&ALL[..4]` as its `DEFAULT_PERSONAS`, so the two lists are
//! mechanically derived from one source.
//!
//! Persona body content lives in
//! `resources/modules/personas/reviewer-<name>.md` and is shipped to
//! sub-agents via the host-pack renderer: the Jinja templates at
//! `resources/agents/.claude/agents/reviewer-<name>.md.tmpl` (and the
//! Codex twin under `resources/agents/.codex/agents/`) `{% include %}`
//! the persona body at render time, so on disk the persona body lands
//! at `.claude/agents/reviewer-<name>.md` (or the Codex equivalent).
//! The host loads that file as the sub-agent's system context when
//! `speccy-review` spawns it. Host-native files are the sole canonical
//! persona surface.
//!
//! See `.speccy/specs/0009-review-command/SPEC.md` REQ-001 / REQ-002.

/// All reviewer personas shipped with Speccy, in declared order.
///
/// The first four entries are the **default fan-out** consumed by
/// SPEC-0007 (`speccy next --kind review`); the trailing two
/// (`architecture`, `docs`) are off-by-default and only run when a
/// reviewer explicitly passes `--persona`. SPEC-0007 must reference
/// `&ALL[..4]` so both lists evolve together.
pub const ALL: &[&str] = &[
    "business",
    "tests",
    "security",
    "style",
    "architecture",
    "docs",
];

#[cfg(test)]
mod tests {
    use super::ALL;

    #[test]
    fn all_contains_exactly_six_names_in_declared_order() {
        assert_eq!(
            ALL,
            &[
                "business",
                "tests",
                "security",
                "style",
                "architecture",
                "docs"
            ]
        );
    }

    #[test]
    fn default_personas_is_prefix_of_all() {
        let default = ALL.get(..4).expect("ALL must have at least 4 elements");
        assert_eq!(default, &["business", "tests", "security", "style"]);
    }
}
