//! Compile-time embedded skill bundle.
//!
//! `include_dir!` snapshots the workspace `skills/` tree into the binary
//! at build time (per SPEC-0002 DEC-001). Layout under [`SKILLS`],
//! per SPEC-0015:
//!
//! - `claude-code/speccy-<verb>/SKILL.md` -- Claude Code skills; copied to
//!   `.claude/skills/speccy-<verb>/SKILL.md` at init time so the pack is
//!   discoverable as host-native skills (not slash commands).
//! - `codex/speccy-<verb>/SKILL.md` -- Codex skills; copied to
//!   `.agents/skills/speccy-<verb>/SKILL.md` (the project-local scan
//!   path OpenAI's Codex docs list). Layout mirrors the Claude Code pack
//!   1:1.
//! - `shared/personas/*.md` -- persona definitions; copied to
//!   `.speccy/skills/personas/` so SPEC-0009's reviewer-persona resolver can
//!   find them as project-local overrides.
//! - `shared/prompts/*.md` -- prompt templates; copied to
//!   `.speccy/skills/prompts/` so future overrides have a documented home (the
//!   prompts are also loaded directly from the embedded bundle by
//!   `speccy-core::prompt::template`).

use include_dir::Dir;
use include_dir::include_dir;

/// Embedded copy of the workspace `skills/` directory.
pub static SKILLS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../skills");

#[cfg(test)]
mod tests {
    use super::SKILLS;

    #[test]
    fn claude_code_pack_contains_init_recipe() {
        assert!(
            SKILLS
                .get_file("claude-code/speccy-init/SKILL.md")
                .is_some(),
            "bundle should contain claude-code/speccy-init/SKILL.md",
        );
    }

    #[test]
    fn codex_pack_contains_init_recipe() {
        assert!(
            SKILLS.get_file("codex/speccy-init/SKILL.md").is_some(),
            "bundle should contain codex/speccy-init/SKILL.md",
        );
    }

    #[test]
    fn shared_personas_contain_security_reviewer() {
        assert!(
            SKILLS
                .get_file("shared/personas/reviewer-security.md")
                .is_some(),
            "bundle should contain shared/personas/reviewer-security.md",
        );
    }

    #[test]
    fn root_bundle_is_non_empty() {
        assert!(
            SKILLS.dirs().count() >= 3,
            "SKILLS should contain at least claude-code/, codex/, shared/",
        );
    }
}
