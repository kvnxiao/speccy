//! Compile-time embedded resource bundle.
//!
//! `include_dir!` snapshots the workspace `resources/` tree into the
//! binary at build time (per SPEC-0002 DEC-001, re-targeted by
//! SPEC-0016 T-007). The bundle is structured into two top-level
//! subtrees:
//!
//! - `agents/.<install_root>/...` -- host-specific wrapper templates whose
//!   folder structure mirrors the install destination 1:1. For `.claude/`
//!   Claude Code wrappers land under
//!   `agents/.claude/skills/speccy-<verb>/SKILL.md.tmpl`; Codex wrappers split
//!   between `agents/.agents/` (skill packs, per SPEC-0015) and
//!   `agents/.codex/` (subagents, per `OpenAI`'s Codex subagents docs).
//! - `modules/...` -- host-neutral content, single-source for every wrapper to
//!   `{% include %}`. Personas live at
//!   `modules/personas/reviewer-<persona>.md`, prompts at
//!   `modules/prompts/<name>.md`, skill bodies at
//!   `modules/skills/speccy-<verb>.md`.
//!
//! [`crate::render`] consumes both subtrees at init time: it walks
//! `agents/.<install_root>/` for each install root the chosen host
//! writes to, then renders each `.tmpl` file through `MiniJinja` with a
//! loader rooted at `modules/`.

use include_dir::Dir;
use include_dir::include_dir;

/// Embedded copy of the workspace `resources/` directory.
///
/// Layout invariants asserted by the in-module tests: at least the
/// `agents/` and `modules/` top-level subtrees exist, each non-empty,
/// and key per-host wrapper / module-body files resolve to a present
/// file.
pub static RESOURCES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../resources");

#[cfg(test)]
mod tests {
    use super::RESOURCES;

    #[test]
    fn claude_code_pack_contains_init_recipe() {
        assert!(
            RESOURCES
                .get_file("agents/.claude/skills/speccy-init/SKILL.md.tmpl")
                .is_some(),
            "bundle should contain agents/.claude/skills/speccy-init/SKILL.md.tmpl",
        );
    }

    #[test]
    fn codex_pack_contains_init_recipe() {
        assert!(
            RESOURCES
                .get_file("agents/.agents/skills/speccy-init/SKILL.md.tmpl")
                .is_some(),
            "bundle should contain agents/.agents/skills/speccy-init/SKILL.md.tmpl",
        );
    }

    #[test]
    fn root_bundle_is_non_empty() {
        let top_level: Vec<&str> = RESOURCES
            .dirs()
            .filter_map(|d| d.path().file_name().and_then(|n| n.to_str()))
            .collect();
        assert!(
            top_level.contains(&"agents") && top_level.contains(&"modules"),
            "RESOURCES should contain `agents/` and `modules/` top-level subtrees; got: {top_level:?}",
        );
        let agents = RESOURCES
            .get_dir("agents")
            .expect("RESOURCES.get_dir(\"agents\") should resolve");
        assert!(
            agents.dirs().count() >= 1,
            "agents/ subtree should be non-empty",
        );
        let modules = RESOURCES
            .get_dir("modules")
            .expect("RESOURCES.get_dir(\"modules\") should resolve");
        assert!(
            modules.dirs().count() >= 1,
            "modules/ subtree should be non-empty",
        );
    }
}
