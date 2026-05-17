//! Reviewer persona registry and project-local-first file resolver.
//!
//! Six personas ship: the four default fan-out personas
//! ([`ALL`][`ALL`]`[..4]` = `business`, `tests`, `security`, `style`) plus
//! two off-by-default personas (`architecture`, `docs`). Adding a new
//! persona is a single-line change to [`ALL`]; SPEC-0007 consumes
//! `&ALL[..4]` as its `DEFAULT_PERSONAS`, so the two lists are
//! mechanically derived from one source.
//!
//! Persona content is markdown shipped under
//! `resources/modules/personas/reviewer-<name>.md`. Resolution order:
//!
//! 1. **Project-local override.**
//!    `<project_root>/.speccy/skills/personas/reviewer-<name>.md`.
//! 2. **Embedded bundle.** Same file name, shipped inside the `speccy-core`
//!    binary via [`include_dir!`].
//!
//! Host-native locations (`.claude/commands/`, `.codex/skills/`) are
//! **not** in the chain (SPEC-0009 DEC-002): the host files carry
//! host-specific frontmatter and are not suitable for direct inlining as
//! persona content.
//!
//! Empty or unreadable overrides emit a stderr warning and fall through
//! to the embedded version; absent overrides fall through silently.
//!
//! See `.speccy/specs/0009-review-command/SPEC.md` REQ-001 / REQ-002.

use camino::Utf8Path;
use include_dir::Dir;
use include_dir::include_dir;
use std::io::Write;
use thiserror::Error;

/// Embedded copy of every shipped reviewer-persona markdown file.
///
/// Mirrors [`crate::prompt::template`]'s embedded prompts directory, but
/// scoped to `resources/modules/personas/` so persona resolution does
/// not depend on the binary crate's separate `SKILLS` bundle. SPEC-0016
/// T-002 moved the embedded source from `skills/shared/personas/` to
/// `resources/modules/personas/`; the resolver chain (project-local
/// override → embedded bundle) and external API are unchanged.
pub static PERSONAS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../resources/modules/personas");

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

/// Failure mode of [`resolve_file`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PersonaError {
    /// `name` is not a member of [`ALL`].
    #[error("unknown persona `{name}`; valid: {valid}", valid = valid.join(", "))]
    UnknownName {
        /// Verbatim user input.
        name: String,
        /// The six valid registry names.
        valid: &'static [&'static str],
    },
    /// Neither the project-local override nor the embedded bundle
    /// contains a persona file for `name`. Reachable only if the binary
    /// was built with an incomplete bundle.
    #[error("persona `{name}` not found in project override or embedded bundle")]
    NotFound {
        /// The persona that was looked up.
        name: String,
    },
}

/// Resolve the markdown body for reviewer persona `name`.
///
/// Lookup order is documented at the module level. The returned string
/// is the persona content verbatim, ready to substitute into the
/// reviewer prompt at `{{persona_content}}`.
///
/// # Errors
///
/// - [`PersonaError::UnknownName`] if `name` is not in [`ALL`].
/// - [`PersonaError::NotFound`] if neither the project-local override nor the
///   embedded bundle has the persona file (only reachable if the bundle is
///   missing the entry; the test suite asserts all six are present).
pub fn resolve_file(name: &str, project_root: &Utf8Path) -> Result<String, PersonaError> {
    if !ALL.contains(&name) {
        return Err(PersonaError::UnknownName {
            name: name.to_owned(),
            valid: ALL,
        });
    }
    let mut stderr = std::io::stderr();
    resolve_file_with_warn(name, project_root, &mut stderr)
}

/// Variant of [`resolve_file`] that writes the fall-through warning to
/// `warn_out` instead of process stderr. Used by tests.
///
/// # Errors
///
/// Same as [`resolve_file`].
pub fn resolve_file_with_warn<W: Write>(
    name: &str,
    project_root: &Utf8Path,
    warn_out: &mut W,
) -> Result<String, PersonaError> {
    if !ALL.contains(&name) {
        return Err(PersonaError::UnknownName {
            name: name.to_owned(),
            valid: ALL,
        });
    }
    let file_name = persona_file_name(name);
    let override_path = project_root
        .join(".speccy")
        .join("skills")
        .join("personas")
        .join(&file_name);

    if override_path.exists() {
        match fs_err::read_to_string(override_path.as_std_path()) {
            Ok(content) if !content.trim().is_empty() => return Ok(content),
            Ok(_empty) => {
                if writeln!(
                    warn_out,
                    "speccy: persona override at {override_path} is empty; falling back to the embedded bundle",
                )
                .is_err()
                {
                    // Warning sink is closed; nothing actionable.
                }
            }
            Err(err) => {
                if writeln!(
                    warn_out,
                    "speccy: failed to read persona override at {override_path}: {err}; falling back to the embedded bundle",
                )
                .is_err()
                {
                    // Warning sink is closed; nothing actionable.
                }
            }
        }
    }

    let entry = PERSONAS
        .get_file(file_name.as_str())
        .ok_or_else(|| PersonaError::NotFound {
            name: name.to_owned(),
        })?;
    let body = entry
        .contents_utf8()
        .ok_or_else(|| PersonaError::NotFound {
            name: name.to_owned(),
        })?;
    Ok(body.to_owned())
}

fn persona_file_name(name: &str) -> String {
    format!("reviewer-{name}.md")
}

#[cfg(test)]
mod tests {
    use super::ALL;
    use super::PersonaError;
    use super::persona_file_name;
    use super::resolve_file_with_warn;
    use camino::Utf8PathBuf;

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

    #[test]
    fn persona_file_name_formats_correctly() {
        assert_eq!(persona_file_name("security"), "reviewer-security.md");
        assert_eq!(
            persona_file_name("architecture"),
            "reviewer-architecture.md"
        );
    }

    #[test]
    fn resolve_unknown_name_returns_unknown_name_error() {
        let tmp = tempfile::tempdir().expect("tempdir creation should succeed");
        let root = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf())
            .expect("tempdir path should be UTF-8");
        let mut warns: Vec<u8> = Vec::new();
        let err = resolve_file_with_warn("nope", &root, &mut warns)
            .expect_err("unknown persona name must be rejected");
        assert!(
            matches!(&err, PersonaError::UnknownName { name, valid } if name == "nope" && *valid == ALL),
            "expected UnknownName{{nope, ALL}}, got {err:?}",
        );
    }
}
