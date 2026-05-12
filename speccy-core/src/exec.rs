//! Captured shell execution for checks.
//!
//! [`crate::parse::CheckEntry`] entries from `spec.toml` are executable
//! claims of behaviour; this module spawns them through the host shell
//! with stdio piped, tees the output to a caller-supplied writer, and
//! returns structured [`CheckResult`] records.
//!
//! `speccy check` (SPEC-0010) uses inherited stdio for its CLI command;
//! this module is the captured-output variant introduced by SPEC-0012
//! (`speccy verify`), where stdout is reserved for the summary or JSON
//! and live output must reach the user via stderr.
//!
//! See `.speccy/specs/0012-verify-command/SPEC.md` DEC-001.

use crate::parse::CheckEntry;
use crate::parse::CheckPayload;
use camino::Utf8Path;
use std::io::Read;
use std::io::Write;
use std::process::Command;
use std::process::Stdio;
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

/// Build a [`Command`] that invokes `command` through the host shell with
/// `cwd` as the working directory.
///
/// Picks the host shell at compile time (Unix: `sh -c`, Windows:
/// `cmd /c`). Stdio is left at the default (inherited) so the child
/// writes directly to the parent's stdout/stderr unless the caller
/// reconfigures it.
#[must_use = "the configured Command must be spawned"]
pub fn shell_command(command: &str, cwd: &Utf8Path) -> Command {
    let mut cmd = if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.arg("/c").arg(command);
        c
    } else {
        let mut c = Command::new("sh");
        c.arg("-c").arg(command);
        c
    };
    cmd.current_dir(cwd.as_std_path());
    cmd
}

/// Outcome bucket of a check after execution (or non-execution, for
/// manual checks).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckOutcome {
    /// Executable check exited zero.
    Pass,
    /// Executable check exited non-zero, or failed to spawn / pipe.
    Fail,
    /// Manual check; never spawned. Does not affect verify's exit code.
    Manual,
}

impl CheckOutcome {
    /// Render the outcome as a short string for diagnostics and JSON.
    #[must_use = "the rendered outcome is the on-wire form"]
    pub const fn as_str(self) -> &'static str {
        match self {
            CheckOutcome::Pass => "Pass",
            CheckOutcome::Fail => "Fail",
            CheckOutcome::Manual => "Manual",
        }
    }
}

/// One check tagged with the spec it came from, ready for execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckSpec {
    /// Spec the check belongs to (e.g. `SPEC-0001`).
    pub spec_id: String,
    /// Stable `CHK-NNN` identifier.
    pub check_id: String,
    /// Free-form kind label.
    pub kind: String,
    /// Executable command, if any. Mutually exclusive with `prompt`.
    pub command: Option<String>,
    /// Manual prompt, if any. Mutually exclusive with `command`.
    pub prompt: Option<String>,
    /// Human-readable claim of what the check proves.
    pub proves: String,
}

impl CheckSpec {
    /// Construct from a parsed [`CheckEntry`] plus the spec it came from.
    #[must_use = "the returned CheckSpec is the input to run_checks_captured"]
    pub fn from_entry(spec_id: impl Into<String>, entry: &CheckEntry) -> Self {
        let (command, prompt) = match &entry.payload {
            CheckPayload::Command(c) => (Some(c.clone()), None),
            CheckPayload::Prompt(p) => (None, Some(p.clone())),
        };
        Self {
            spec_id: spec_id.into(),
            check_id: entry.id.clone(),
            kind: entry.kind.clone(),
            command,
            prompt,
            proves: entry.proves.clone(),
        }
    }
}

/// Structured per-check outcome from [`run_checks_captured`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckResult {
    /// Spec the check belongs to.
    pub spec_id: String,
    /// Stable `CHK-NNN` identifier.
    pub check_id: String,
    /// Free-form kind label.
    pub kind: String,
    /// Outcome bucket.
    pub outcome: CheckOutcome,
    /// Child exit code, if the check spawned a process and exited.
    pub exit_code: Option<i32>,
    /// Wall-clock duration in milliseconds. Not serialised to JSON in
    /// v1 (omitted for byte-determinism across runs).
    pub duration_ms: Option<u64>,
}

/// Execute every check in `checks` serially, tee'ing each child's stdio
/// to `err` as the child writes, and return one [`CheckResult`] per
/// input.
///
/// Manual checks (`command.is_none() && prompt.is_some()`) are not
/// spawned; their prompt is written to `err` and a `Manual` result is
/// returned. Checks with both `command` and `prompt` unset are
/// defensively reported as `Fail` (the upstream `spec.toml` parser
/// rejects them, so this branch is unreachable in practice).
///
/// Per-check headers (`==> CHK-NNN ...`) and footers (`<-- CHK-NNN
/// PASS|FAIL|MANUAL ...`) are written to `err` for consistency with
/// `speccy check`'s output conventions.
///
/// # Errors
///
/// Returns the underlying [`std::io::Error`] if writing to `err`
/// fails (broken pipe, etc.). Child spawn failures do not error;
/// they are recorded as `Fail` results with `exit_code: None`.
pub fn run_checks_captured(
    checks: &[CheckSpec],
    project_root: &Utf8Path,
    err: &mut dyn Write,
) -> std::io::Result<Vec<CheckResult>> {
    let mut out = Vec::with_capacity(checks.len());
    for c in checks {
        let result = run_one(c, project_root, err)?;
        out.push(result);
    }
    Ok(out)
}

fn run_one(
    c: &CheckSpec,
    project_root: &Utf8Path,
    err: &mut dyn Write,
) -> std::io::Result<CheckResult> {
    match (c.command.as_deref(), c.prompt.as_deref()) {
        (None, Some(prompt)) => render_manual(c, prompt, err),
        (Some(command), _) => run_executable(c, command, project_root, err),
        (None, None) => {
            writeln!(
                err,
                "==> {} ({}): malformed check (neither command nor prompt)",
                c.check_id, c.spec_id,
            )?;
            writeln!(err, "<-- {} FAIL (malformed)", c.check_id)?;
            Ok(CheckResult {
                spec_id: c.spec_id.clone(),
                check_id: c.check_id.clone(),
                kind: c.kind.clone(),
                outcome: CheckOutcome::Fail,
                exit_code: None,
                duration_ms: None,
            })
        }
    }
}

fn render_manual(c: &CheckSpec, prompt: &str, err: &mut dyn Write) -> std::io::Result<CheckResult> {
    writeln!(err, "==> {} ({}, manual):", c.check_id, c.spec_id)?;
    if prompt.ends_with('\n') {
        err.write_all(prompt.as_bytes())?;
    } else {
        writeln!(err, "{prompt}")?;
    }
    writeln!(err, "<-- {} MANUAL (verify and proceed)", c.check_id)?;
    Ok(CheckResult {
        spec_id: c.spec_id.clone(),
        check_id: c.check_id.clone(),
        kind: c.kind.clone(),
        outcome: CheckOutcome::Manual,
        exit_code: None,
        duration_ms: None,
    })
}

fn run_executable(
    c: &CheckSpec,
    command: &str,
    project_root: &Utf8Path,
    err: &mut dyn Write,
) -> std::io::Result<CheckResult> {
    writeln!(err, "==> {} ({}): {}", c.check_id, c.spec_id, c.proves)?;
    err.flush()?;

    let mut cmd = shell_command(command, project_root);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let start = Instant::now();
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(spawn_err) => {
            writeln!(err, "<-- {} FAIL (spawn error: {spawn_err})", c.check_id)?;
            return Ok(CheckResult {
                spec_id: c.spec_id.clone(),
                check_id: c.check_id.clone(),
                kind: c.kind.clone(),
                outcome: CheckOutcome::Fail,
                exit_code: None,
                duration_ms: None,
            });
        }
    };

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let (Some(stdout), Some(stderr)) = (stdout, stderr) else {
        if child.kill().is_err() {
            // Already exited or kill() unavailable; nothing else to do.
        }
        writeln!(err, "<-- {} FAIL (missing child pipes)", c.check_id)?;
        return Ok(CheckResult {
            spec_id: c.spec_id.clone(),
            check_id: c.check_id.clone(),
            kind: c.kind.clone(),
            outcome: CheckOutcome::Fail,
            exit_code: None,
            duration_ms: None,
        });
    };

    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    let tx_stdout = tx.clone();
    let h_stdout = thread::spawn(move || pump(stdout, &tx_stdout));
    let h_stderr = thread::spawn(move || pump(stderr, &tx));

    let mut write_failure: Option<std::io::Error> = None;
    while let Ok(chunk) = rx.recv() {
        if write_failure.is_some() {
            continue;
        }
        if let Err(write_err) = err.write_all(&chunk) {
            write_failure = Some(write_err);
            continue;
        }
        if let Err(flush_err) = err.flush() {
            write_failure = Some(flush_err);
        }
    }

    if h_stdout.join().is_err() {
        // Reader thread panicked; we still want to reap the child.
    }
    if h_stderr.join().is_err() {
        // Reader thread panicked; we still want to reap the child.
    }

    if let Some(write_err) = write_failure {
        return Err(write_err);
    }

    let status = child.wait()?;
    let exit_code = status.code();
    let outcome = if status.success() {
        CheckOutcome::Pass
    } else {
        CheckOutcome::Fail
    };
    let duration_ms = u64::try_from(start.elapsed().as_millis()).ok();

    match outcome {
        CheckOutcome::Pass => writeln!(err, "<-- {} PASS", c.check_id)?,
        CheckOutcome::Fail => writeln!(
            err,
            "<-- {} FAIL (exit {code})",
            c.check_id,
            code = exit_code.unwrap_or(-1),
        )?,
        CheckOutcome::Manual => {}
    }

    Ok(CheckResult {
        spec_id: c.spec_id.clone(),
        check_id: c.check_id.clone(),
        kind: c.kind.clone(),
        outcome,
        exit_code,
        duration_ms,
    })
}

fn pump<R: Read>(mut reader: R, tx: &mpsc::Sender<Vec<u8>>) {
    let mut buf = [0u8; 4096];
    loop {
        match reader.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                let Some(chunk) = buf.get(..n) else { break };
                if tx.send(chunk.to_vec()).is_err() {
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CheckOutcome;
    use super::CheckSpec;
    use super::shell_command;
    use crate::parse::CheckEntry;
    use crate::parse::CheckPayload;
    use camino::Utf8PathBuf;

    #[test]
    fn shell_command_targets_expected_program() {
        let cwd = Utf8PathBuf::from(".");
        let cmd = shell_command("echo hello", &cwd);
        let program = cmd.get_program().to_string_lossy().to_string();
        if cfg!(windows) {
            assert_eq!(program, "cmd");
        } else {
            assert_eq!(program, "sh");
        }
    }

    #[test]
    fn shell_command_passes_command_string_verbatim() {
        let cwd = Utf8PathBuf::from(".");
        let cmd = shell_command("echo 'hi'", &cwd);
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        let flag = if cfg!(windows) { "/c" } else { "-c" };
        assert_eq!(args, vec![flag.to_owned(), "echo 'hi'".to_owned()]);
    }

    #[test]
    fn check_spec_from_entry_command_populates_command_branch() {
        let entry = CheckEntry {
            id: "CHK-001".into(),
            kind: "test".into(),
            proves: "x".into(),
            payload: CheckPayload::Command("echo".into()),
        };
        let spec = CheckSpec::from_entry("SPEC-0001", &entry);
        assert_eq!(spec.command.as_deref(), Some("echo"));
        assert!(spec.prompt.is_none());
    }

    #[test]
    fn check_spec_from_entry_prompt_populates_prompt_branch() {
        let entry = CheckEntry {
            id: "CHK-001".into(),
            kind: "manual".into(),
            proves: "x".into(),
            payload: CheckPayload::Prompt("verify".into()),
        };
        let spec = CheckSpec::from_entry("SPEC-0001", &entry);
        assert!(spec.command.is_none());
        assert_eq!(spec.prompt.as_deref(), Some("verify"));
    }

    #[test]
    fn check_outcome_as_str_renders_stable_tokens() {
        assert_eq!(CheckOutcome::Pass.as_str(), "Pass");
        assert_eq!(CheckOutcome::Fail.as_str(), "Fail");
        assert_eq!(CheckOutcome::Manual.as_str(), "Manual");
    }
}
