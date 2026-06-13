//! Reviewer persona name registry.
//!
//! Seven personas ship: the five default fan-out personas
//! ([`ALL`][`ALL`]`[..5]` = `business`, `tests`, `security`, `style`,
//! `correctness`) plus two off-by-default personas (`architecture`,
//! `docs`). Adding a new persona is a single-line change to [`ALL`];
//! the default persona set consumes `&ALL[..5]` as its `DEFAULT_PERSONAS`,
//! so the two lists are mechanically derived from one source.
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

/// All reviewer personas shipped with Speccy, in declared order.
///
/// The first five entries are the **default fan-out** consumed by
/// `speccy next --kind review`; the trailing two
/// (`architecture`, `docs`) are off-by-default and only run when a
/// reviewer explicitly passes `--persona`. The default fan-out must
/// reference `&ALL[..5]` so both lists evolve together.
pub const ALL: &[&str] = &[
    "business",
    "tests",
    "security",
    "style",
    "correctness",
    "architecture",
    "docs",
];
