//! `speccy check` command logic.
//!
//! Discovers the project root, scans `.speccy/specs/`, collects every
//! `[[checks]]` entry from successfully-parsed spec.toml files, and
//! executes them through the host shell. Manual checks render their
//! prompt and never spawn a subprocess. Executable checks inherit
//! stdio so child output streams live.
//!
//! See `.speccy/specs/0010-check-command/SPEC.md`.

use camino::Utf8Path;
use camino::Utf8PathBuf;
use speccy_core::exec::shell_command;
use speccy_core::parse::CheckEntry;
use speccy_core::parse::CheckPayload;
use speccy_core::parse::SpecStatus;
use speccy_core::workspace::Workspace;
use speccy_core::workspace::WorkspaceError;
use speccy_core::workspace::find_root;
use speccy_core::workspace::scan;
use std::io::Write;
use thiserror::Error;

/// CLI-level error returned by [`run`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum CheckError {
    /// The CHK-ID argument did not match the `CHK-NNN` (>= 3 digits) shape.
    #[error("invalid check ID format `{arg}`; expected CHK- followed by 3 or more digits")]
    InvalidCheckIdFormat {
        /// Raw argument the user supplied.
        arg: String,
    },
    /// No spec.toml across the workspace contained a `[[checks]]` entry
    /// with the requested ID.
    #[error("no check with id `{id}` found in workspace; run `speccy status` to list specs")]
    NoCheckMatching {
        /// Check ID that produced no match.
        id: String,
    },
    /// Walked up from cwd without locating a `.speccy/` directory.
    #[error(".speccy/ directory not found walking up from current directory")]
    ProjectRootNotFound,
    /// I/O failure during discovery or while writing framing output.
    #[error("I/O error during check execution")]
    Io(#[from] std::io::Error),
    /// `std::process::Command::status` failed to spawn the shell.
    #[error("failed to spawn shell process for {check_id}")]
    ChildSpawn {
        /// Check whose command could not be spawned.
        check_id: String,
        /// Underlying spawn error.
        #[source]
        source: std::io::Error,
    },
}

/// `speccy check` arguments.
#[derive(Debug, Clone, Default)]
pub struct CheckArgs {
    /// Optional `CHK-NNN` filter. When `None`, every discovered check runs.
    pub id: Option<String>,
}

/// One check enriched with the `spec_id` and parent-spec lifecycle
/// status (drives in-flight categorisation and header lines).
#[derive(Debug, Clone)]
struct CollectedCheck {
    spec_id: String,
    spec_status: SpecStatus,
    entry: CheckEntry,
}

/// Resolve current working directory as a `Utf8PathBuf`.
///
/// # Errors
///
/// Returns [`CheckError::Io`] if `std::env::current_dir` fails, or if
/// the path isn't valid UTF-8.
pub fn resolve_cwd() -> Result<Utf8PathBuf, CheckError> {
    let std_path = std::env::current_dir()?;
    Utf8PathBuf::from_path_buf(std_path).map_err(|path| {
        CheckError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "current working directory is not valid UTF-8: {}",
                path.display()
            ),
        ))
    })
}

/// Run `speccy check` from `cwd`. Returns the intended process exit code
/// (per REQ-004: first non-zero from any executable check, or 1 when at
/// least one spec.toml failed to parse, or 0 otherwise).
///
/// `out` receives framing lines (`==>`, `<--`, summary, manual prompts).
/// `err` receives malformed-spec warnings. Child stdout/stderr streams
/// live via inherited stdio (bypassing both writers).
///
/// # Errors
///
/// See [`CheckError`] variants. CLI exit-code mapping lives in the
/// dispatcher.
pub fn run(
    args: CheckArgs,
    cwd: &Utf8Path,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<i32, CheckError> {
    let CheckArgs { id } = args;

    let project_root = match find_root(cwd) {
        Ok(p) => p,
        Err(WorkspaceError::NoSpeccyDir { .. }) => return Err(CheckError::ProjectRootNotFound),
        Err(WorkspaceError::Io(e)) => return Err(CheckError::Io(e)),
        Err(other) => {
            return Err(CheckError::Io(std::io::Error::other(other.to_string())));
        }
    };

    if let Some(arg) = id.as_deref() {
        validate_chk_id_format(arg)?;
    }

    let ws = scan(&project_root);
    let (all_checks, malformed) = collect_checks(&ws, err)?;

    let filtered: Vec<CollectedCheck> = match id.as_deref() {
        Some(arg) => all_checks
            .into_iter()
            .filter(|c| c.entry.id == arg)
            .collect(),
        None => all_checks,
    };

    if filtered.is_empty() {
        if let Some(arg) = id {
            return Err(CheckError::NoCheckMatching { id: arg });
        }
        writeln!(out, "No checks defined.")?;
        return Ok(i32::from(malformed > 0));
    }

    execute_checks(&filtered, &project_root, out, malformed)
}

fn execute_checks(
    checks: &[CollectedCheck],
    project_root: &Utf8Path,
    out: &mut dyn Write,
    malformed: u32,
) -> Result<i32, CheckError> {
    let mut passed: u32 = 0;
    let mut failed: u32 = 0;
    let mut in_flight: u32 = 0;
    let mut manual: u32 = 0;
    let mut first_gating_nonzero: Option<i32> = None;

    for c in checks {
        match &c.entry.payload {
            CheckPayload::Prompt(prompt) => {
                render_manual(c, prompt, out)?;
                manual = manual.saturating_add(1);
            }
            CheckPayload::Command(command) => {
                let code = run_executable(c, command, project_root, out)?;
                if code == 0 {
                    passed = passed.saturating_add(1);
                } else if matches!(c.spec_status, SpecStatus::InProgress) {
                    in_flight = in_flight.saturating_add(1);
                } else {
                    failed = failed.saturating_add(1);
                    if first_gating_nonzero.is_none() {
                        first_gating_nonzero = Some(code);
                    }
                }
            }
        }
    }

    writeln!(
        out,
        "{passed} passed, {failed} failed, {in_flight} in-flight, {manual} manual",
    )?;

    let exit = first_gating_nonzero.unwrap_or(i32::from(malformed > 0));
    Ok(exit)
}

fn render_manual(c: &CollectedCheck, prompt: &str, out: &mut dyn Write) -> Result<(), CheckError> {
    writeln!(out, "==> {} ({}, manual):", c.entry.id, c.spec_id)?;
    if prompt.ends_with('\n') {
        out.write_all(prompt.as_bytes())?;
    } else {
        writeln!(out, "{prompt}")?;
    }
    writeln!(out, "<-- {} MANUAL (verify and proceed)", c.entry.id)?;
    Ok(())
}

fn run_executable(
    c: &CollectedCheck,
    command: &str,
    project_root: &Utf8Path,
    out: &mut dyn Write,
) -> Result<i32, CheckError> {
    writeln!(
        out,
        "==> {} ({}): {}",
        c.entry.id, c.spec_id, c.entry.proves,
    )?;
    out.flush()?;

    let mut cmd = shell_command(command, project_root);
    let status = cmd.status().map_err(|source| CheckError::ChildSpawn {
        check_id: c.entry.id.clone(),
        source,
    })?;
    let code = status.code().unwrap_or(-1);

    if code == 0 {
        writeln!(out, "<-- {} PASS", c.entry.id)?;
    } else if matches!(c.spec_status, SpecStatus::InProgress) {
        writeln!(
            out,
            "<-- {} IN-FLIGHT (in-progress spec, exit {code})",
            c.entry.id,
        )?;
    } else {
        writeln!(out, "<-- {} FAIL (exit {code})", c.entry.id)?;
    }
    Ok(code)
}

fn collect_checks(
    ws: &Workspace,
    err: &mut dyn Write,
) -> Result<(Vec<CollectedCheck>, u32), CheckError> {
    let mut out = Vec::new();
    let mut malformed: u32 = 0;
    for parsed in &ws.specs {
        let label = parsed
            .spec_id
            .clone()
            .unwrap_or_else(|| display_spec_label(&parsed.dir));
        let spec_status = parsed
            .spec_md
            .as_ref()
            .map_or(SpecStatus::InProgress, |s| s.frontmatter.status);
        // Skip defunct specs entirely: their checks should never run.
        if matches!(spec_status, SpecStatus::Dropped | SpecStatus::Superseded) {
            continue;
        }
        match &parsed.spec_toml {
            Ok(toml) => {
                for check in &toml.checks {
                    out.push(CollectedCheck {
                        spec_id: label.clone(),
                        spec_status,
                        entry: check.clone(),
                    });
                }
            }
            Err(e) => {
                writeln!(
                    err,
                    "speccy check: warning: {label} spec.toml failed to parse: {e}; skipping",
                )?;
                malformed = malformed.saturating_add(1);
            }
        }
    }
    Ok((out, malformed))
}

fn display_spec_label(dir: &Utf8Path) -> String {
    dir.file_name()
        .map_or_else(|| dir.to_string(), ToOwned::to_owned)
}

fn validate_chk_id_format(arg: &str) -> Result<(), CheckError> {
    let Some(suffix) = arg.strip_prefix("CHK-") else {
        return Err(CheckError::InvalidCheckIdFormat {
            arg: arg.to_owned(),
        });
    };
    if suffix.len() < 3 || !suffix.chars().all(|c| c.is_ascii_digit()) {
        return Err(CheckError::InvalidCheckIdFormat {
            arg: arg.to_owned(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::CheckError;
    use super::validate_chk_id_format;

    #[test]
    fn validate_accepts_three_digit_id() {
        validate_chk_id_format("CHK-001").expect("CHK-001 should be valid");
    }

    #[test]
    fn validate_accepts_more_than_three_digits() {
        validate_chk_id_format("CHK-1234").expect("CHK-1234 should be valid");
    }

    #[test]
    fn validate_rejects_too_few_digits() {
        let err =
            validate_chk_id_format("CHK-12").expect_err("CHK-12 should be rejected (need >=3)");
        assert!(matches!(
            err,
            CheckError::InvalidCheckIdFormat { ref arg } if arg == "CHK-12"
        ));
    }

    #[test]
    fn validate_rejects_missing_prefix() {
        let err = validate_chk_id_format("FOO").expect_err("FOO should be rejected");
        assert!(matches!(err, CheckError::InvalidCheckIdFormat { .. }));
    }

    #[test]
    fn validate_rejects_lowercase_prefix() {
        let err = validate_chk_id_format("chk-001").expect_err("chk-001 should be rejected (case)");
        assert!(matches!(err, CheckError::InvalidCheckIdFormat { .. }));
    }

    #[test]
    fn validate_rejects_non_digit_suffix() {
        let err = validate_chk_id_format("CHK-00A").expect_err("CHK-00A should be rejected");
        assert!(matches!(err, CheckError::InvalidCheckIdFormat { .. }));
    }
}
