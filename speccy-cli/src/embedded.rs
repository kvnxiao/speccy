//! Compile-time embedded skill bundle.
//!
//! `include_dir!` snapshots the workspace `skills/` tree into the binary
//! at build time (per SPEC-0002 DEC-001). Layout under [`SKILLS`]:
//!
//! - `claude-code/*.md` -- Claude Code recipes; copied to `.claude/commands/`
//!   at init time.
//! - `codex/*.md` -- Codex recipes; copied to `.codex/skills/`.
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

fn has_md_extension(name: &str) -> bool {
    std::path::Path::new(name)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("md"))
}

/// File names of every `*.md` entry inside `SKILLS/<sub_path>`.
///
/// Returns an empty slice if `sub_path` is not present in the bundle.
/// Used by the init command to compute the set of "shipped" file names
/// it may overwrite under `--force`.
#[must_use = "the returned list drives the shipped-vs-user file decision"]
pub fn shipped_file_names(sub_path: &str) -> Vec<&'static str> {
    let Some(dir) = SKILLS.get_dir(sub_path) else {
        return Vec::new();
    };
    let mut names: Vec<&'static str> = dir
        .files()
        .filter_map(|f| f.path().file_name().and_then(|n| n.to_str()))
        .filter(|n| has_md_extension(n))
        .collect();
    names.sort_unstable();
    names
}

#[cfg(test)]
mod tests {
    use super::SKILLS;
    use super::shipped_file_names;

    #[test]
    fn claude_code_pack_contains_speccy_init() {
        let names = shipped_file_names("claude-code");
        assert!(
            names.contains(&"speccy-init.md"),
            "claude-code pack should contain speccy-init.md, got: {names:?}",
        );
    }

    #[test]
    fn codex_pack_contains_speccy_init() {
        let names = shipped_file_names("codex");
        assert!(
            names.contains(&"speccy-init.md"),
            "codex pack should contain speccy-init.md, got: {names:?}",
        );
    }

    #[test]
    fn shared_personas_contain_security_reviewer() {
        let names = shipped_file_names("shared/personas");
        assert!(
            names.contains(&"reviewer-security.md"),
            "shared/personas should contain reviewer-security.md, got: {names:?}",
        );
    }

    #[test]
    fn unknown_subpath_returns_empty() {
        assert!(shipped_file_names("does-not-exist").is_empty());
    }

    #[test]
    fn root_bundle_is_non_empty() {
        assert!(
            SKILLS.dirs().count() >= 3,
            "SKILLS should contain at least claude-code/, codex/, shared/",
        );
    }
}
