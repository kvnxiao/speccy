//! `speccy lock SPEC-NNNN` command logic.
//!
//! Records the SPEC.md content hash and a UTC timestamp into the
//! corresponding TASKS.md frontmatter. Delegates the rewrite to
//! [`speccy_core::tasks::commit_frontmatter`], preserving body bytes
//! and managed-field declared order. On precondition failure (missing
//! workspace, missing SPEC.md or TASKS.md, parse errors, ID
//! disagreement) the command exits non-zero and TASKS.md is untouched.
//!
//! See `.speccy/specs/0033-eject-prompt-bodies/SPEC.md` REQ-002.
//! DEC-006 holds the precondition surface to exactly what
//! `tasks --commit` validated pre-SPEC; no new checks are added here.
//!
//! See `.speccy/specs/0006-tasks-command/SPEC.md` for the original
//! `--commit` contract this command inherits.

use camino::Utf8Path;
use camino::Utf8PathBuf;
use jiff::Timestamp;
use regex::Regex;
use speccy_core::ParseError;
use speccy_core::parse::spec_md;
use speccy_core::tasks::CommitError;
use speccy_core::tasks::commit_frontmatter;
use speccy_core::workspace::WorkspaceError;
use speccy_core::workspace::find_root;
use std::sync::OnceLock;
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
    /// Working directory could not be resolved.
    #[error("failed to resolve current working directory")]
    Cwd(#[source] std::io::Error),
    /// Cwd path is not valid UTF-8.
    #[error("current working directory is not valid UTF-8")]
    CwdNotUtf8,
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
    let project_root = match find_root(cwd) {
        Ok(p) => p,
        Err(WorkspaceError::NoSpeccyDir { .. }) => return Err(LockError::ProjectRootNotFound),
        Err(other) => return Err(LockError::Workspace(other)),
    };

    let canonical_id = validate_spec_id(&spec_id)?;
    let spec_dir = locate_spec_dir(&project_root, &canonical_id)?;
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
    if !spec_id_regex().is_match(raw) {
        return Err(LockError::InvalidSpecIdFormat {
            arg: raw.to_owned(),
        });
    }
    Ok(raw.to_owned())
}

fn locate_spec_dir(project_root: &Utf8Path, canonical_id: &str) -> Result<Utf8PathBuf, LockError> {
    let digits =
        canonical_id
            .strip_prefix("SPEC-")
            .ok_or_else(|| LockError::InvalidSpecIdFormat {
                arg: canonical_id.to_owned(),
            })?;
    let specs_dir = project_root.join(".speccy").join("specs");
    let prefix = format!("{digits}-");

    if let Some(dir) = find_spec_dir_in(&specs_dir, &prefix) {
        return Ok(dir);
    }
    // Mission-folder layer: one level of grouping per the workspace
    // scanner's contract (`enumerate_focus_folder`).
    if let Some(dir) = find_spec_dir_in_mission_folders(&specs_dir, &prefix) {
        return Ok(dir);
    }
    Err(LockError::SpecNotFound {
        id: canonical_id.to_owned(),
    })
}

fn find_spec_dir_in(parent: &Utf8Path, prefix: &str) -> Option<Utf8PathBuf> {
    let entries = fs_err::read_dir(parent.as_std_path()).ok()?;
    for entry in entries.flatten() {
        let meta = entry.metadata().ok()?;
        if !meta.is_dir() {
            continue;
        }
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.starts_with(prefix) {
            return Utf8PathBuf::from_path_buf(path).ok();
        }
    }
    None
}

fn find_spec_dir_in_mission_folders(specs_dir: &Utf8Path, prefix: &str) -> Option<Utf8PathBuf> {
    let entries = fs_err::read_dir(specs_dir.as_std_path()).ok()?;
    for entry in entries.flatten() {
        let meta = entry.metadata().ok()?;
        if !meta.is_dir() {
            continue;
        }
        let path = entry.path();
        let Ok(utf8) = Utf8PathBuf::from_path_buf(path) else {
            continue;
        };
        if let Some(found) = find_spec_dir_in(&utf8, prefix) {
            return Some(found);
        }
    }
    None
}

#[expect(
    clippy::unwrap_used,
    reason = "compile-time literal regex; covered by unit tests"
)]
fn spec_id_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"^SPEC-\d{4,}$").unwrap())
}

/// Resolve current working directory as a `Utf8PathBuf`.
///
/// # Errors
///
/// Returns [`LockError::Cwd`] if `std::env::current_dir` fails, or
/// [`LockError::CwdNotUtf8`] if the path isn't valid UTF-8.
pub fn resolve_cwd() -> Result<Utf8PathBuf, LockError> {
    let std_path = std::env::current_dir().map_err(LockError::Cwd)?;
    Utf8PathBuf::from_path_buf(std_path).map_err(|_path| LockError::CwdNotUtf8)
}

#[cfg(test)]
mod tests {
    use super::spec_id_regex;
    use super::validate_spec_id;

    #[test]
    fn valid_spec_ids_pass_regex() {
        assert!(spec_id_regex().is_match("SPEC-0006"));
        assert!(spec_id_regex().is_match("SPEC-1234"));
        assert!(spec_id_regex().is_match("SPEC-10000"));
    }

    #[test]
    fn invalid_spec_ids_rejected() {
        validate_spec_id("FOO").expect_err("`FOO` must fail format validation");
        validate_spec_id("SPEC-1").expect_err("`SPEC-1` has fewer than 4 digits");
        validate_spec_id("spec-0001").expect_err("lowercase prefix must be rejected");
        validate_spec_id("SPEC-").expect_err("missing digits must be rejected");
    }
}
