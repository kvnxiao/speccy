//! `evidence collect` — the controller executes declared `kind: command`
//! evidence itself and records exit code, stdout, stderr, and a content hash
//! (DESIGN § Acceptance Ledger). Because the controller is the collector,
//! `evidence record` refuses agent-pasted command output.
//!
//! Commands run through the platform shell in the workspace root under timeout
//! and output-byte caps, serialized on the workspace command lock.

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::config::ProjectConfig;
use crate::error::{Result, SpeccyError};
use crate::event::{Event, EvidenceRecord};
use crate::gitx;
use crate::ids;
use crate::model::EvidenceKind;
use crate::store::{write_atomic, Store};

/// Collect command evidence for the named requirements (optionally narrowed to
/// specific qualified request IDs `R.E`).
pub fn collect(
    store: &Store,
    run_id: &str,
    requirements: &[String],
    requests: &[String],
) -> Result<Value> {
    let (spec_id, run) = store.run_by_id(run_id)?;
    let spec = store.spec_state(&spec_id)?;
    let draft = spec
        .revision(&run.revision_id)
        .map(|r| r.draft.clone())
        .ok_or_else(|| SpeccyError::not_found(format!("revision {} not found", run.revision_id)))?;
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

    let cap = config.evidence.command_output_max_bytes as usize;
    let timeout = Duration::from_secs(config.evidence.command_timeout_seconds);

    // Serialize all command execution on the workspace command lock.
    let records = store.with_command_lock(|| {
        let mut out = Vec::new();
        for (req_id, ev_id, command) in &targets {
            let id = ids::short_id("ev");
            let dirty_before = gitx::dirty_files(&store.git_root).unwrap_or_default().len();
            let run = run_shell(command, &store.workspace_root, timeout, cap);
            let dirty_after = gitx::dirty_files(&store.git_root).unwrap_or_default().len();

            let mut hasher = Sha256::new();
            hasher.update(&run.stdout);
            let stdout_hash = format!("sha256:{:x}", hasher.finalize());

            let artifact_rel = format!("evidence/{id}.txt");
            let artifact_body = render_artifact(command, &run, dirty_before, dirty_after);
            write_atomic(
                &store.run_dir(&spec_id, run_id).join(&artifact_rel),
                artifact_body.as_bytes(),
            )?;

            let note = if run.timed_out {
                Some(format!(
                    "timed out after {}s",
                    config.evidence.command_timeout_seconds
                ))
            } else {
                None
            };
            let record = EvidenceRecord {
                id: id.clone(),
                requirement: req_id.clone(),
                request: Some(ev_id.clone()),
                kind: "command".into(),
                collected_by: "controller".into(),
                note,
                artifact: Some(artifact_rel),
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
                "artifact": format!("evidence/{id}.txt"),
                "collected_by": "controller",
            }));
        }
        Ok(out)
    })?;

    Ok(json!({ "evidence": records }))
}

/// (requirement_id, request_id, command) tuples to execute.
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
            if ev.kind_enum() == Some(EvidenceKind::Command) {
                if let Some(command) = &ev.command {
                    targets.push((req_id.clone(), ev.id.clone(), command.clone()));
                }
            }
        }
    }
    Ok(targets)
}

struct CommandRun {
    exit_code: i32,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    timed_out: bool,
}

/// Run a command through the platform shell with a timeout and output cap.
fn run_shell(command: &str, cwd: &Path, timeout: Duration, max_bytes: usize) -> CommandRun {
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
            return CommandRun {
                exit_code: -1,
                stdout: Vec::new(),
                stderr: format!("failed to spawn command: {e}").into_bytes(),
                timed_out: false,
            }
        }
    };

    // Drain pipes on threads so a chatty command cannot deadlock on a full pipe.
    let stdout_pipe = child.stdout.take();
    let stderr_pipe = child.stderr.take();
    let out_handle = thread::spawn(move || read_capped(stdout_pipe, max_bytes));
    let err_handle = thread::spawn(move || read_capped(stderr_pipe, max_bytes));

    let deadline = Instant::now() + timeout;
    let mut timed_out = false;
    let exit_code = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code().unwrap_or(-1),
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    timed_out = true;
                    break -1;
                }
                thread::sleep(Duration::from_millis(20));
            }
            Err(_) => break -1,
        }
    };

    let stdout = out_handle.join().unwrap_or_default();
    let stderr = err_handle.join().unwrap_or_default();
    CommandRun {
        exit_code,
        stdout,
        stderr,
        timed_out,
    }
}

fn read_capped(pipe: Option<impl Read>, max_bytes: usize) -> Vec<u8> {
    let Some(mut pipe) = pipe else {
        return Vec::new();
    };
    let mut buf = Vec::new();
    // Read a bit past the cap so we can note truncation, then clamp.
    let _ = pipe
        .by_ref()
        .take((max_bytes as u64) + 1)
        .read_to_end(&mut buf);
    buf.truncate(max_bytes);
    buf
}

fn render_artifact(
    command: &str,
    run: &CommandRun,
    dirty_before: usize,
    dirty_after: usize,
) -> String {
    format!(
        "command: {command}\nexit_code: {}\ntimed_out: {}\ndirty_before: {dirty_before}\ndirty_after: {dirty_after}\n\n--- stdout ---\n{}\n--- stderr ---\n{}\n",
        run.exit_code,
        run.timed_out,
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr),
    )
}
