//! `evidence collect` — the controller executes declared `kind: command`
//! evidence itself and records exit code, stdout, stderr, and a content hash
//! (DESIGN § Acceptance Ledger). Because the controller is the collector,
//! `evidence record` refuses agent-pasted command output.
//!
//! Commands run through the platform shell in the workspace root under timeout
//! and output-byte caps, serialized on the workspace command lock.

use crate::config::ProjectConfig;
use crate::error::Result;
use crate::error::SpeccyError;
use crate::event::Event;
use crate::event::EvidenceRecord;
use crate::gitx;
use crate::ids;
use crate::model::EvidenceKind;
use crate::store::Store;
use crate::store::write_atomic;
use camino::Utf8Path;
use serde_json::Value;
use serde_json::json;
use std::io::Read;
use std::process::Child;
use std::process::Command;
use std::process::Stdio;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::time::Instant;

/// Collect command evidence for the named requirements (optionally narrowed to
/// specific qualified request IDs `R.E`).
///
/// # Errors
///
/// Returns an error if the run/spec/revision cannot be resolved, no matching
/// `kind: command` evidence requests are found, a command fails the
/// configured allow policy, or writing the evidence artifact/event fails.
pub fn collect(
    store: &Store,
    run_id: &str,
    requirements: &[String],
    requests: &[String],
) -> Result<Value> {
    let (spec_id, run) = store.run_by_id(run_id)?;
    let draft = store.run_draft(&run)?;
    let config = ProjectConfig::load(&store.workspace_root)?;

    // Resolve the set of (requirement, request) command targets.
    let targets = resolve_targets(&draft, requirements, requests)?;
    if targets.is_empty() {
        return Err(SpeccyError::validation(
            "no kind: command evidence requests found for the given selectors",
        ));
    }

    // Command allow policy: when configured, refuse a command matching no
    // pattern (DESIGN § Acceptance Ledger). The harness sandbox remains the
    // security boundary; this is a drift guardrail.
    let allow = &config.evidence.command_policy.allow;
    if !allow.is_empty() {
        for (req_id, ev_id, command) in &targets {
            if !crate::lint::command_allowed(command, allow) {
                return Err(SpeccyError::validation(format!(
                    "command for {req_id}.{ev_id} matches no allow pattern: {command}"
                )));
            }
        }
    }

    // Byte cap; fall back to usize::MAX on platforms where usize is narrower
    // than u64 rather than failing evidence collection over a config value.
    let cap = usize::try_from(config.evidence.command_output_max_bytes).unwrap_or(usize::MAX);
    let timeout = Duration::from_secs(config.evidence.command_timeout_seconds);
    // Known-secret env values are scrubbed from stored output before hashing
    // (env-scrubbing stub; full redaction model is Q18 in OPEN-ITEMS.md).
    let secrets = secret_env_values();

    // Serialize all command execution on the workspace command lock.
    let records = store.with_command_lock(|| {
        let mut out = Vec::new();
        for (req_id, ev_id, command) in &targets {
            let id = ids::short_id("ev");
            let dirty_before = gitx::dirty_files(&store.git_root).unwrap_or_default().len();
            let mut run = run_shell(command, &store.workspace_root, timeout, cap);
            let dirty_after = gitx::dirty_files(&store.git_root).unwrap_or_default().len();

            run.stdout = scrub_secrets(&run.stdout, &secrets);
            run.stderr = scrub_secrets(&run.stderr, &secrets);

            let stdout_hash = crate::hash::sha256_prefixed(&run.stdout);

            let artifact_rel = format!("evidence/{id}.txt");
            let artifact_body = render_artifact(command, &run, dirty_before, dirty_after);
            let artifact_hash = crate::hash::sha256_prefixed(artifact_body.as_bytes());
            write_atomic(
                &store.run_dir(&spec_id, run_id).join(&artifact_rel),
                artifact_body.as_bytes(),
            )?;

            let mut notes = Vec::new();
            if run.timed_out {
                notes.push(format!(
                    "timed out after {}s",
                    config.evidence.command_timeout_seconds
                ));
            }
            if run.truncated {
                notes.push(format!("output truncated at {cap} bytes"));
            }
            if run.reader_abandoned {
                notes.push("reader abandoned: descendant process still holds the pipe".to_string());
            }
            let note = (!notes.is_empty()).then(|| notes.join("; "));
            let record = EvidenceRecord {
                id: id.clone(),
                requirement: req_id.clone(),
                request: Some(ev_id.clone()),
                kind: EvidenceKind::Command,
                collected_by: "controller".into(),
                note: note.clone(),
                artifact: Some(artifact_rel),
                artifact_hash: Some(artifact_hash.clone()),
                command: Some(command.clone()),
                exit_code: Some(run.exit_code),
                stdout_hash: Some(stdout_hash.clone()),
            };
            store.append_run_event(
                &spec_id,
                run_id,
                Event::EvidenceRecorded { evidence: record },
            )?;
            out.push(json!({
                "id": id,
                "requirement": req_id,
                "request": ev_id,
                "kind": "command",
                "command": command,
                "exit_code": run.exit_code,
                "stdout_hash": stdout_hash,
                "artifact_hash": artifact_hash,
                "artifact": format!("evidence/{id}.txt"),
                "collected_by": "controller",
                "note": note,
            }));
        }
        Ok(out)
    })?;

    Ok(json!({ "evidence": records }))
}

/// (`requirement_id`, `request_id`, command) tuples to execute.
fn resolve_targets(
    draft: &crate::model::SpecDraft,
    requirements: &[String],
    requests: &[String],
) -> Result<Vec<(String, String, String)>> {
    let mut targets = Vec::new();

    if !requests.is_empty() {
        for qualified in requests {
            let (req_id, ev_id) = qualified.split_once('.').ok_or_else(|| {
                SpeccyError::validation(format!(
                    "malformed request selector `{qualified}`; expected <requirement>.<request>"
                ))
            })?;
            let req = draft
                .requirement(req_id)
                .ok_or_else(|| SpeccyError::not_found(format!("no requirement {req_id}")))?;
            let ev = req.evidence.iter().find(|e| e.id == ev_id).ok_or_else(|| {
                SpeccyError::not_found(format!("no evidence request {qualified}"))
            })?;
            if ev.kind_enum() != Some(EvidenceKind::Command) {
                return Err(SpeccyError::validation(format!(
                    "{qualified} is not a kind: command request"
                )));
            }
            let command = ev.command.clone().ok_or_else(|| {
                SpeccyError::validation(format!("{qualified} has no command string"))
            })?;
            targets.push((req_id.to_string(), ev_id.to_string(), command));
        }
        return Ok(targets);
    }

    for req_id in requirements {
        let req = draft
            .requirement(req_id)
            .ok_or_else(|| SpeccyError::not_found(format!("no requirement {req_id}")))?;
        for ev in &req.evidence {
            if ev.kind_enum() == Some(EvidenceKind::Command)
                && let Some(command) = &ev.command
            {
                targets.push((req_id.clone(), ev.id.clone(), command.clone()));
            }
        }
    }
    Ok(targets)
}

struct ShellRun {
    exit_code: i32,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    timed_out: bool,
    /// stdout or stderr exceeded `command_output_max_bytes` and was clamped.
    truncated: bool,
    /// A reader thread was still blocked on a pipe past the grace window (a
    /// killed command's descendant still holds the write end); its stream is
    /// recorded empty and the thread is leaked rather than blocking forever.
    reader_abandoned: bool,
}

/// Grace after the process exits (or is killed) for the reader threads to drain
/// the pipes before a still-blocked reader is abandoned.
const READER_GRACE: Duration = Duration::from_secs(2);

/// Run a command through the platform shell with a timeout and output cap.
fn run_shell(command: &str, cwd: &Utf8Path, timeout: Duration, max_bytes: usize) -> ShellRun {
    let mut cmd = if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.arg("/c").arg(command);
        c
    } else {
        let mut c = Command::new("sh");
        c.arg("-c").arg(command);
        c
    };
    cmd.current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            return ShellRun {
                exit_code: -1,
                stdout: Vec::new(),
                stderr: format!("failed to spawn command: {e}").into_bytes(),
                timed_out: false,
                truncated: false,
                reader_abandoned: false,
            };
        }
    };

    // Drain pipes on threads so a chatty command cannot deadlock on a full
    // pipe. The threads report over channels (rather than a joined handle) so
    // that a reader blocked on a pipe a killed descendant still holds open can
    // be abandoned instead of blocking this thread forever.
    let stdout_pipe = child.stdout.take();
    let stderr_pipe = child.stderr.take();
    let (out_tx, out_rx) = mpsc::channel();
    let (err_tx, err_rx) = mpsc::channel();
    thread::spawn(move || {
        // Send failing means the receiver was abandoned; drop the output.
        _ = out_tx.send(read_capped(stdout_pipe, max_bytes));
    });
    thread::spawn(move || {
        _ = err_tx.send(read_capped(stderr_pipe, max_bytes));
    });

    let deadline = Instant::now() + timeout;
    let mut timed_out = false;
    let exit_code = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code().unwrap_or(-1),
            Ok(None) => {
                if Instant::now() >= deadline {
                    // Best-effort tree kill: descendants that inherited the
                    // pipes must die too, or the readers never see EOF.
                    kill_tree(&mut child);
                    // Reap the killed child; nothing to do with the result.
                    _ = child.wait();
                    timed_out = true;
                    break -1;
                }
                thread::sleep(Duration::from_millis(20));
            }
            Err(_) => break -1,
        }
    };

    // One shared grace deadline across both streams so a single stuck reader
    // cannot double the wait.
    let grace_deadline = Instant::now() + READER_GRACE;
    let (stdout, out_trunc, out_lost) = recv_stream(&out_rx, grace_deadline);
    let (stderr, err_trunc, err_lost) = recv_stream(&err_rx, grace_deadline);
    ShellRun {
        exit_code,
        stdout,
        stderr,
        timed_out,
        truncated: out_trunc || err_trunc,
        reader_abandoned: out_lost || err_lost,
    }
}

/// Await a reader thread's `(bytes, truncated)` result until `deadline`.
/// Returns `(bytes, truncated, abandoned)`; on timeout the reader is abandoned
/// (`abandoned = true`) and its stream is recorded empty.
fn recv_stream(rx: &mpsc::Receiver<(Vec<u8>, bool)>, deadline: Instant) -> (Vec<u8>, bool, bool) {
    let remaining = deadline.saturating_duration_since(Instant::now());
    match rx.recv_timeout(remaining) {
        Ok((buf, truncated)) => (buf, truncated, false),
        Err(_) => (Vec::new(), false, true),
    }
}

/// Best-effort tree kill of a timed-out command. On Windows, `taskkill /T`
/// terminates the whole process tree so descendants that inherited the pipes
/// die too; elsewhere (and if `taskkill` cannot be spawned) fall back to
/// killing the direct child.
fn kill_tree(child: &mut Child) {
    #[cfg(windows)]
    {
        // Only trust `taskkill` when it actually reports success: a nonzero
        // exit (process already gone, access denied) means the tree may still
        // be alive, so fall through to the direct child kill.
        let killed = Command::new("taskkill")
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success());
        if killed {
            return;
        }
    }
    _ = child.kill();
}

/// Read a pipe up to `max_bytes`, returning the clamped bytes and whether the
/// output exceeded the cap (so the caller can note truncation).
fn read_capped(pipe: Option<impl Read>, max_bytes: usize) -> (Vec<u8>, bool) {
    let Some(mut pipe) = pipe else {
        return (Vec::new(), false);
    };
    let mut buf = Vec::new();
    // Read one byte past the cap so we can tell truncation from an exact fit.
    // Read errors are ignored: whatever was read before the error is still
    // used, capped and reported as truncated below.
    _ = pipe
        .by_ref()
        .take((max_bytes as u64).saturating_add(1))
        .read_to_end(&mut buf);
    let truncated = buf.len() > max_bytes;
    buf.truncate(max_bytes);
    (buf, truncated)
}

fn render_artifact(
    command: &str,
    run: &ShellRun,
    dirty_before: usize,
    dirty_after: usize,
) -> String {
    format!(
        "command: {command}\nexit_code: {}\ntimed_out: {}\ntruncated: {}\ndirty_before: {dirty_before}\ndirty_after: {dirty_after}\n\n--- stdout ---\n{}\n--- stderr ---\n{}\n",
        run.exit_code,
        run.timed_out,
        run.truncated,
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr),
    )
}

/// Env vars whose values are treated as secrets and scrubbed from stored
/// command output. This is the MVP env-scrubbing stub; the full redaction
/// model is Open Question 18 (`OPEN-ITEMS.md`).
fn secret_env_values() -> Vec<(String, String)> {
    std::env::vars()
        // Skip trivially short values: they would match innocuous substrings.
        .filter(|(name, value)| is_secret_name(name) && value.trim().len() >= 4)
        .collect()
}

fn is_secret_name(name: &str) -> bool {
    let n = name.to_ascii_uppercase();
    [
        "SECRET",
        "TOKEN",
        "PASSWORD",
        "PASSWD",
        "CREDENTIAL",
        "PRIVATE_KEY",
        "API_KEY",
        "ACCESS_KEY",
        "AUTH",
    ]
    .iter()
    .any(|needle| n.contains(needle))
}

/// Replace occurrences of each known-secret value with `[REDACTED:<NAME>]`.
/// No-op (and byte-preserving) when there are no secrets to scrub.
fn scrub_secrets(data: &[u8], secrets: &[(String, String)]) -> Vec<u8> {
    if secrets.is_empty() {
        return data.to_vec();
    }
    let mut text = String::from_utf8_lossy(data).into_owned();
    for (name, value) in secrets {
        if text.contains(value.as_str()) {
            text = text.replace(value.as_str(), &format!("[REDACTED:{name}]"));
        }
    }
    text.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::is_secret_name;
    use super::scrub_secrets;

    #[test]
    fn scrubs_known_secret_values() {
        let secrets = vec![("API_KEY".to_string(), "sk-live-abc123".to_string())];
        let out = scrub_secrets(b"leaked sk-live-abc123 here", &secrets);
        assert_eq!(
            String::from_utf8(out).expect("valid utf8"),
            "leaked [REDACTED:API_KEY] here"
        );
    }

    #[test]
    fn no_secrets_is_byte_preserving() {
        let raw = vec![0u8, 159, 146, 150]; // invalid UTF-8
        assert_eq!(scrub_secrets(&raw, &[]), raw);
    }

    #[test]
    fn secret_name_matching() {
        assert!(is_secret_name("GITHUB_TOKEN"));
        assert!(is_secret_name("aws_access_key_id"));
        assert!(is_secret_name("DB_PASSWORD"));
        assert!(!is_secret_name("PATH"));
        assert!(!is_secret_name("HOME"));
    }

    #[test]
    fn read_capped_does_not_overflow_at_usize_max() {
        // `max_bytes + 1` would overflow at usize::MAX; saturating_add avoids it.
        let (buf, truncated) = super::read_capped(Some(&b"data"[..]), usize::MAX);
        assert_eq!(buf, b"data");
        assert!(!truncated);
    }

    // A timed-out command whose backgrounded descendant still holds the output
    // pipe must not block the collector forever: the reader is abandoned after
    // the bounded grace window. (Unix-only: relies on `sleep` + `&` semantics.)
    #[cfg(unix)]
    #[test]
    fn timeout_abandons_a_reader_held_by_a_descendant() {
        use camino::Utf8Path;
        use std::time::Duration;
        use std::time::Instant;

        let start = Instant::now();
        let run = super::run_shell(
            "echo hi; sleep 30",
            Utf8Path::new("."),
            Duration::from_secs(1),
            4096,
        );
        let elapsed = start.elapsed();
        assert!(run.timed_out, "command should have hit the 1s timeout");
        assert!(
            run.reader_abandoned,
            "reader held by the orphaned sleep should be abandoned"
        );
        // 1s timeout + 2s grace; a generous bound proves it did not block on
        // the 30s sleep.
        assert!(
            elapsed < Duration::from_secs(15),
            "collector blocked too long: {elapsed:?}"
        );
    }
}
