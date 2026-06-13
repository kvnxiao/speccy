//! `speccy lock SPEC-NNNN` command logic.
//!
//! Records the SPEC.md content hash and a UTC timestamp into the
//! corresponding TASKS.md frontmatter. Delegates the rewrite to
//! [`speccy_core::tasks::commit_frontmatter`], preserving body bytes
//! and managed-field declared order. On precondition failure (missing
//! workspace, missing SPEC.md or TASKS.md, parse errors, ID
//! disagreement) the command exits non-zero and TASKS.md is untouched.
//!
//! The precondition surface is held to exactly what `tasks --commit`
//! validated; no new checks are added here.

use crate::check_selector::bare_spec_regex;
use camino::Utf8Path;
use jiff::Timestamp;
use speccy_core::ParseError;
use speccy_core::parse::spec_md;
use speccy_core::tasks::CommitError;
use speccy_core::tasks::commit_frontmatter;
use speccy_core::workspace::WorkspaceError;
use speccy_core::workspace::scan;
use thiserror::Error;

/// CLI-level error returned by [`run`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum LockError {
    /// Walked up from cwd without locating a `.speccy/` directory.
    #[error(".speccy/ directory not found walking up from current directory")]
    ProjectRootNotFound,
    /// I/O failure during workspace discovery.
    #[error("workspace discovery failed")]
    Workspace(#[from] WorkspaceError),
    /// Argument did not match the `SPEC-\d{4,}` shape.
    #[error("invalid SPEC-ID `{arg}`; expected format SPEC-NNNN (4+ digits)")]
    InvalidSpecIdFormat {
        /// The string supplied by the user.
        arg: String,
    },
    /// No spec directory matched the supplied ID.
    #[error("spec `{id}` not found under .speccy/specs/")]
    SpecNotFound {
        /// The canonical SPEC ID that was looked up.
        id: String,
    },
    /// SPEC.md parse failure.
    #[error("failed to parse SPEC.md for {id}")]
    SpecMdParse {
        /// The SPEC ID we were trying to parse.
        id: String,
        /// Underlying parser error.
        #[source]
        source: Box<ParseError>,
    },
    /// Frontmatter-rewrite failure (missing TASKS.md, ID disagreement,
    /// or I/O).
    #[error(transparent)]
    Commit(#[from] CommitError),
}

/// `speccy lock` arguments.
#[derive(Debug, Clone)]
pub struct LockArgs {
    /// The `SPEC-NNNN` argument (required).
    pub spec_id: String,
}

/// Run `speccy lock` from `cwd`.
///
/// Resolves the spec directory, parses SPEC.md to capture its
/// canonical content hash, then delegates the TASKS.md frontmatter
/// rewrite to [`commit_frontmatter`]. On any precondition failure the
/// TASKS.md file is left byte-identical to its pre-invocation state.
///
/// # Errors
///
/// Returns any [`LockError`] variant if discovery, ID validation,
/// SPEC.md parsing, or the frontmatter rewrite fails.
pub fn run(args: LockArgs, cwd: &Utf8Path) -> Result<(), LockError> {
    let LockArgs { spec_id } = args;
    let project_root = crate::cwd::resolve_root(cwd, LockError::ProjectRootNotFound)?;

    let canonical_id = validate_spec_id(&spec_id)?;
    let workspace = scan(&project_root);
    let spec_dir =
        workspace
            .spec_dir_by_id(&canonical_id)
            .ok_or_else(|| LockError::SpecNotFound {
                id: canonical_id.clone(),
            })?;
    let spec_md_path = spec_dir.join("SPEC.md");
    let tasks_md_path = spec_dir.join("TASKS.md");

    let parsed_spec = spec_md(&spec_md_path).map_err(|source| LockError::SpecMdParse {
        id: canonical_id.clone(),
        source,
    })?;

    commit_frontmatter(
        &tasks_md_path,
        &canonical_id,
        &parsed_spec.frontmatter.id,
        &parsed_spec.sha256,
        Timestamp::now(),
    )?;

    Ok(())
}

fn validate_spec_id(raw: &str) -> Result<String, LockError> {
    if !bare_spec_regex().is_match(raw) {
        return Err(LockError::InvalidSpecIdFormat {
            arg: raw.to_owned(),
        });
    }
    Ok(raw.to_owned())
}

#[cfg(test)]
mod tests {
    use super::validate_spec_id;

    #[test]
    fn valid_spec_ids_pass_format_validation() {
        validate_spec_id("SPEC-0006").expect("4-digit id accepted");
        validate_spec_id("SPEC-1234").expect("4-digit id accepted");
        validate_spec_id("SPEC-10000").expect("5-digit id accepted");
    }

    #[test]
    fn invalid_spec_ids_rejected() {
        validate_spec_id("FOO").expect_err("`FOO` must fail format validation");
        validate_spec_id("SPEC-1").expect_err("`SPEC-1` has fewer than 4 digits");
        validate_spec_id("spec-0001").expect_err("lowercase prefix must be rejected");
        validate_spec_id("SPEC-").expect_err("missing digits must be rejected");
    }
}
