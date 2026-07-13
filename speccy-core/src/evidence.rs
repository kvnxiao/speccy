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
use crate::event::ControlBaselineRecord;
use crate::event::ControlCleanup;
use crate::event::ControlIsolation;
use crate::event::ControlStatus;
use crate::event::Event;
use crate::event::EvidenceControlRecord;
use crate::event::EvidenceRecord;
use crate::event::EvidenceRepoIdentity;
use crate::gitx;
use crate::ids;
use crate::model::EvidenceControl;
use crate::model::EvidenceKind;
use crate::model::EvidenceRequest;
use crate::store::Store;
use crate::store::write_atomic;
use camino::Utf8Path;
use serde_json::Value;
use serde_json::json;
use std::io::Read;
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
        for t in &targets {
            if !crate::lint::command_allowed(&t.command, allow) {
                return Err(SpeccyError::validation(format!(
                    "command for {}.{} matches no allow pattern: {}",
                    t.requirement, t.request, t.command
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

    // Serialize all command execution on the workspace command lock, held
    // through process cleanup and identity capture (DESIGN § Acceptance
    // Ledger).
    let ctx = CollectCtx {
        store,
        spec_id: &spec_id,
        run_id,
        base_commit: &run.base_commit,
        config: &config,
        secrets: &secrets,
        cap,
        timeout,
    };
    let records = store.with_command_lock(|| {
        let mut out = Vec::new();
        for target in &targets {
            out.push(collect_one(&ctx, target)?);
        }
        Ok(out)
    })?;

    Ok(json!({ "evidence": records }))
}

/// Shared context for one `evidence collect` invocation.
struct CollectCtx<'a> {
    store: &'a Store,
    spec_id: &'a str,
    run_id: &'a str,
    base_commit: &'a str,
    config: &'a ProjectConfig,
    secrets: &'a [(String, String)],
    cap: usize,
    timeout: Duration,
}

/// One declared command target to execute.
struct Target {
    requirement: String,
    request: String,
    command: String,
    control: Option<EvidenceControl>,
}

/// Execute one declared command under identity capture and containment, store
/// its artifact, and append its evidence record.
fn collect_one(ctx: &CollectCtx<'_>, target: &Target) -> Result<Value> {
    let (req_id, ev_id, command) = (&target.requirement, &target.request, &target.command);
    let id = ids::short_id("ev");
    // Baseline side of a declared control runs first, in an isolated
    // worktree, so the negative is proven before the candidate executes.
    let control_draft = match target.control {
        Some(kind) => Some(run_baseline(ctx, &id, command, kind)?),
        None => None,
    };
    let before = capture_repo_identity(&ctx.store.git_root).map_err(|e| {
        SpeccyError::io(format!(
            "evidence collection failed: cannot capture the pre-command repository identity: {}",
            e.message
        ))
    })?;
    let mut run = run_shell(command, &ctx.store.workspace_root, ctx.timeout, ctx.cap);
    let after = capture_repo_identity(&ctx.store.git_root).map_err(|e| {
        SpeccyError::io(format!(
            "evidence collection failed: cannot capture the post-command repository identity: {}",
            e.message
        ))
    })?;
    let newly_dirty: Vec<String> = after
        .dirty
        .iter()
        .filter(|p| !before.dirty.contains(p))
        .cloned()
        .collect();
    let repo = EvidenceRepoIdentity {
        head_before: before.head.clone(),
        head_after: after.head.clone(),
        head_changed: before.head != after.head,
        diff_hash_before: before.diff_hash.clone(),
        diff_hash_after: after.diff_hash.clone(),
        newly_dirty,
    };

    run.stdout = scrub_secrets(&run.stdout, ctx.secrets);
    run.stderr = scrub_secrets(&run.stderr, ctx.secrets);

    let stdout_hash = crate::hash::sha256_prefixed(&run.stdout);

    let artifact_rel = format!("evidence/{id}.txt");
    let artifact_body = render_artifact(command, &run, &before, &after, &repo);
    let artifact_hash = crate::hash::sha256_prefixed(artifact_body.as_bytes());
    write_atomic(
        &ctx.store
            .run_dir(ctx.spec_id, ctx.run_id)
            .join(&artifact_rel),
        artifact_body.as_bytes(),
    )?;

    let (note, exit_code) = execution_verdict(ctx, &run, &repo);
    let control = control_draft.map(|draft| Box::new(control_verdict(draft, exit_code)));
    let record = EvidenceRecord {
        id: id.clone(),
        requirement: req_id.clone(),
        request: Some(ev_id.clone()),
        kind: EvidenceKind::Command,
        collected_by: "controller".into(),
        note: note.clone(),
        artifact: Some(artifact_rel.clone()),
        artifact_hash: Some(artifact_hash.clone()),
        command: Some(command.clone()),
        exit_code: Some(exit_code),
        stdout_hash: Some(stdout_hash.clone()),
        repo: Some(repo.clone()),
        control: control.clone(),
    };
    ctx.store.append_run_event(
        ctx.spec_id,
        ctx.run_id,
        Event::EvidenceRecorded { evidence: record },
    )?;
    let mut response = json!({
        "id": id,
        "requirement": req_id,
        "request": ev_id,
        "kind": "command",
        "command": command,
        "exit_code": exit_code,
        "stdout_hash": stdout_hash,
        "artifact_hash": artifact_hash,
        "artifact": artifact_rel,
        "collected_by": "controller",
        "note": note,
        "repo": repo,
        "contained": run.contained,
    });
    if let Some(control) = control {
        let value = serde_json::to_value(control)
            .map_err(|e| SpeccyError::io(format!("cannot serialize control record: {e}")))?;
        if let Some(fields) = response.as_object_mut() {
            fields.insert("control".into(), value);
        }
    }
    Ok(response)
}

/// Baseline-side observations of a declared control, gathered before the
/// candidate command has run.
struct BaselineDraft {
    kind: EvidenceControl,
    baseline: Option<ControlBaselineRecord>,
    isolation: ControlIsolation,
    /// Environment problems that block the control regardless of exit codes:
    /// worktree setup, identity capture, spawn, or containment failure.
    blocked: Vec<String>,
}

/// Execute the declared command against the pinned run baseline in an
/// isolated temporary worktree. Environment problems become a blocked
/// control, never a synthesized failure; only artifact-write failures error.
fn run_baseline(
    ctx: &CollectCtx<'_>,
    id: &str,
    command: &str,
    kind: EvidenceControl,
) -> Result<BaselineDraft> {
    let worktree = ctx
        .store
        .run_dir(ctx.spec_id, ctx.run_id)
        .join(format!("control-wt-{id}"));
    if let Err(e) = gitx::worktree_add(&ctx.store.git_root, &worktree, ctx.base_commit) {
        let cleanup = cleanup_worktree(ctx, &worktree);
        return Ok(BaselineDraft {
            kind,
            baseline: None,
            isolation: ControlIsolation {
                path: worktree.to_string(),
                cleanup,
            },
            blocked: vec![format!("baseline worktree setup failed: {}", e.message)],
        });
    }
    let executed = baseline_execution(ctx, id, command, &worktree);
    // The isolation path is torn down and verified before any error can
    // propagate, and a leak is surfaced either way.
    let cleanup = cleanup_worktree(ctx, &worktree);
    let (baseline, blocked) = executed.map_err(|e| {
        if cleanup == ControlCleanup::Leaked {
            SpeccyError::io(format!(
                "{}; isolation worktree leaked at {worktree}",
                e.message
            ))
        } else {
            e
        }
    })?;
    Ok(BaselineDraft {
        kind,
        baseline,
        isolation: ControlIsolation {
            path: worktree.to_string(),
            cleanup,
        },
        blocked: blocked.into_iter().collect(),
    })
}

/// Run the baseline command inside the isolation worktree and store its
/// artifact. Returns the baseline record plus an optional blocking reason;
/// only the artifact write can error.
fn baseline_execution(
    ctx: &CollectCtx<'_>,
    id: &str,
    command: &str,
    worktree: &Utf8Path,
) -> Result<(Option<ControlBaselineRecord>, Option<String>)> {
    let Ok(before) = capture_repo_identity(worktree) else {
        return Ok((
            None,
            Some("baseline identity capture failed before execution".into()),
        ));
    };
    let mut run = run_shell(command, worktree, ctx.timeout, ctx.cap);
    if !run.spawned {
        return Ok((
            None,
            Some(format!(
                "baseline command could not be spawned: {}",
                String::from_utf8_lossy(&run.stderr)
            )),
        ));
    }
    let Ok(after) = capture_repo_identity(worktree) else {
        return Ok((
            None,
            Some("baseline identity capture failed after execution".into()),
        ));
    };
    let repo = EvidenceRepoIdentity {
        head_before: before.head.clone(),
        head_after: after.head.clone(),
        head_changed: before.head != after.head,
        diff_hash_before: before.diff_hash.clone(),
        diff_hash_after: after.diff_hash.clone(),
        newly_dirty: after
            .dirty
            .iter()
            .filter(|p| !before.dirty.contains(p))
            .cloned()
            .collect(),
    };
    run.stdout = scrub_secrets(&run.stdout, ctx.secrets);
    run.stderr = scrub_secrets(&run.stderr, ctx.secrets);
    let stdout_hash = crate::hash::sha256_prefixed(&run.stdout);
    let artifact_rel = format!("evidence/{id}.baseline.txt");
    let body = render_artifact(command, &run, &before, &after, &repo);
    let artifact_hash = crate::hash::sha256_prefixed(body.as_bytes());
    write_atomic(
        &ctx.store
            .run_dir(ctx.spec_id, ctx.run_id)
            .join(&artifact_rel),
        body.as_bytes(),
    )?;
    let blocked = (!run.contained)
        .then(|| "baseline containment failed: descendants survived teardown".to_string());
    Ok((
        Some(ControlBaselineRecord {
            commit: ctx.base_commit.to_string(),
            exit_code: run.exit_code,
            stdout_hash,
            artifact: artifact_rel,
            artifact_hash,
            contained: run.contained,
            repo,
        }),
        blocked,
    ))
}

/// Remove the isolation worktree and verify it is gone. `Leaked` means the
/// path still exists after removal, deletion, and prune were all attempted.
fn cleanup_worktree(ctx: &CollectCtx<'_>, worktree: &Utf8Path) -> ControlCleanup {
    _ = gitx::worktree_remove(&ctx.store.git_root, worktree);
    if worktree.as_std_path().exists() {
        _ = fs_err::remove_dir_all(worktree.as_std_path());
    }
    _ = gitx::worktree_prune(&ctx.store.git_root);
    if worktree.as_std_path().exists() {
        ControlCleanup::Leaked
    } else {
        ControlCleanup::Removed
    }
}

/// Combine the baseline draft with the candidate's recorded exit code into
/// the stored control verdict. A blocked environment is never a synthesized
/// failure, and a leaked isolation path never reports `passed`.
fn control_verdict(draft: BaselineDraft, candidate_exit: i32) -> EvidenceControlRecord {
    let mut notes = draft.blocked;
    let mut status = if !notes.is_empty() {
        ControlStatus::Blocked
    } else if let Some(baseline) = &draft.baseline {
        if baseline.exit_code == 0 {
            notes.push(
                "baseline command passed: the evidence does not distinguish before from after"
                    .into(),
            );
            ControlStatus::Failed
        } else if candidate_exit == 0 {
            ControlStatus::Passed
        } else {
            notes.push("candidate command failed".into());
            ControlStatus::Failed
        }
    } else {
        notes.push("baseline result missing".into());
        ControlStatus::Blocked
    };
    if draft.isolation.cleanup == ControlCleanup::Leaked {
        notes.push(format!(
            "isolation worktree leaked at {}",
            draft.isolation.path
        ));
        if status == ControlStatus::Passed {
            status = ControlStatus::Blocked;
        }
    }
    EvidenceControlRecord {
        kind: draft.kind,
        status,
        baseline: draft.baseline,
        isolation: draft.isolation,
        note: (!notes.is_empty()).then(|| notes.join("; ")),
    }
}

/// The recorded note and exit code for one execution. Containment failure is
/// failed evidence, never a successful command with a warning: the recorded
/// exit code fails closed and the artifact keeps the observed one.
fn execution_verdict(
    ctx: &CollectCtx<'_>,
    run: &ShellRun,
    repo: &EvidenceRepoIdentity,
) -> (Option<String>, i32) {
    let mut notes = Vec::new();
    if run.timed_out {
        notes.push(format!(
            "timed out after {}s",
            ctx.config.evidence.command_timeout_seconds
        ));
    }
    if run.truncated {
        notes.push(format!("output truncated at {} bytes", ctx.cap));
    }
    if run.reader_abandoned {
        notes.push("reader abandoned: descendant process still holds the pipe".to_string());
    }
    let exit_code = if run.contained {
        run.exit_code
    } else {
        notes.push(format!(
            "process containment failed: descendants survived teardown; evidence fails closed (observed exit {})",
            run.exit_code
        ));
        -1
    };
    if repo.head_changed {
        notes.push(format!(
            "command changed HEAD: {} -> {}",
            repo.head_before, repo.head_after
        ));
    }
    ((!notes.is_empty()).then(|| notes.join("; ")), exit_code)
}

/// The exact repository state on one side of a command execution: HEAD, the
/// sorted dirty paths (untracked included), and a hash of the complete
/// worktree diff against HEAD.
struct RepoIdentity {
    head: String,
    dirty: Vec<String>,
    diff_hash: String,
}

fn capture_repo_identity(git_root: &Utf8Path) -> Result<RepoIdentity> {
    let head = gitx::head(git_root)?;
    let mut dirty = gitx::dirty_files(git_root)?;
    dirty.sort();
    let diff = gitx::worktree_diff(git_root, "HEAD")?;
    Ok(RepoIdentity {
        head,
        dirty,
        diff_hash: crate::hash::sha256_prefixed(diff.as_bytes()),
    })
}

/// The declared control of a request, or `validation_failed` when the stored
/// value is outside the closed vocabulary (fail closed rather than silently
/// running the command uncontrolled).
fn declared_control(req_id: &str, ev: &EvidenceRequest) -> Result<Option<EvidenceControl>> {
    match (&ev.control, ev.control_enum()) {
        (Some(raw), None) => Err(SpeccyError::validation(format!(
            "{req_id}.{} declares unknown control \"{raw}\"",
            ev.id
        ))),
        (_, parsed) => Ok(parsed),
    }
}

/// The declared command targets to execute.
fn resolve_targets(
    draft: &crate::model::SpecDraft,
    requirements: &[String],
    requests: &[String],
) -> Result<Vec<Target>> {
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
            targets.push(Target {
                requirement: req_id.to_string(),
                request: ev_id.to_string(),
                command,
                control: declared_control(req_id, ev)?,
            });
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
                targets.push(Target {
                    requirement: req_id.clone(),
                    request: ev.id.clone(),
                    command: command.clone(),
                    control: declared_control(req_id, ev)?,
                });
            }
        }
    }
    Ok(targets)
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "five independent observations of one execution (spawn, timeout, \
              truncation, reader loss, containment), not an encoded state \
              machine; an enum would misrepresent that they combine freely"
)]
struct ShellRun {
    exit_code: i32,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// The shell process started. `false` distinguishes "could not execute"
    /// from a command that ran and failed — a control baseline must treat the
    /// former as blocked, never as the expected failure.
    spawned: bool,
    timed_out: bool,
    /// stdout or stderr exceeded `command_output_max_bytes` and was clamped.
    truncated: bool,
    /// A reader thread was still blocked on a pipe past the grace window (a
    /// descendant that survived teardown still holds the write end); its
    /// stream is recorded empty and the thread is leaked rather than blocking
    /// forever.
    reader_abandoned: bool,
    /// The full process tree was torn down and confirmed gone before this run
    /// was recorded. `false` is a containment failure and fails the evidence
    /// closed (DESIGN § Acceptance Ledger).
    contained: bool,
}

/// Grace after the process exits (or is killed) for the reader threads to drain
/// the pipes before a still-blocked reader is abandoned.
const READER_GRACE: Duration = Duration::from_secs(2);

/// Run a command through the platform shell with a timeout, an output cap,
/// and process-tree containment: descendants are torn down and reaped after
/// normal exit or timeout, before the run is returned for recording.
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
    containment::prepare(&mut cmd);

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            return ShellRun {
                exit_code: -1,
                stdout: Vec::new(),
                stderr: format!("failed to spawn command: {e}").into_bytes(),
                spawned: false,
                timed_out: false,
                truncated: false,
                reader_abandoned: false,
                contained: true,
            };
        }
    };
    let guard = containment::attach(&child);

    // Drain pipes on threads so a chatty command cannot deadlock on a full
    // pipe. The threads report over channels (rather than a joined handle) so
    // that a reader blocked on a pipe an uncontained descendant still holds
    // open can be abandoned instead of blocking this thread forever.
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
                    timed_out = true;
                    break -1;
                }
                thread::sleep(Duration::from_millis(20));
            }
            Err(_) => break -1,
        }
    };

    // Tear down the whole process tree — after normal exit as well as on
    // timeout — and reap before draining the pipes, so surviving descendants
    // die (readers then see EOF) and nothing keeps mutating after this
    // function returns.
    let contained = guard.teardown(&mut child);

    // One shared grace deadline across both streams so a single stuck reader
    // cannot double the wait.
    let grace_deadline = Instant::now() + READER_GRACE;
    let (stdout, out_trunc, out_lost) = recv_stream(&out_rx, grace_deadline);
    let (stderr, err_trunc, err_lost) = recv_stream(&err_rx, grace_deadline);
    ShellRun {
        exit_code,
        stdout,
        stderr,
        spawned: true,
        timed_out,
        truncated: out_trunc || err_trunc,
        reader_abandoned: out_lost || err_lost,
        contained,
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

/// Process-tree containment (DESIGN § Acceptance Ledger): the command's whole
/// tree is addressable, torn down after normal exit or timeout, and reaped
/// before evidence is recorded. Unix uses a per-command process group and the
/// `kill` utility (no repository-owned `unsafe`); Windows uses a kill-on-close
/// job object via the `win32job` dependency.
#[cfg(unix)]
mod containment {
    use std::process::Child;
    use std::process::Command;
    use std::process::Stdio;
    use std::thread;
    use std::time::Duration;

    pub struct Guard {
        pgid: u32,
    }

    /// Give the command its own process group so the whole tree shares one id.
    pub fn prepare(cmd: &mut Command) {
        use std::os::unix::process::CommandExt as _;
        cmd.process_group(0);
    }

    pub fn attach(child: &Child) -> Guard {
        // `process_group(0)` makes the child the leader, so pgid == pid.
        Guard { pgid: child.id() }
    }

    /// Send `sig` to the whole group via the `kill` utility; `true` when the
    /// signal was delivered to at least one member.
    fn signal_group(pgid: u32, sig: &str) -> bool {
        Command::new("kill")
            .args([sig, "--", &format!("-{pgid}")])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    }

    impl Guard {
        /// SIGKILL the group, reap the direct child, then confirm no member
        /// survives (signal 0 fails once the group is empty). Returns whether
        /// containment was confirmed.
        pub fn teardown(self, child: &mut Child) -> bool {
            _ = signal_group(self.pgid, "-KILL");
            _ = child.wait();
            for _ in 0..40 {
                if !signal_group(self.pgid, "-0") {
                    return true;
                }
                thread::sleep(Duration::from_millis(25));
            }
            false
        }
    }
}

#[cfg(windows)]
mod containment {
    use std::os::windows::io::AsRawHandle as _;
    use std::process::Child;
    use std::process::Command;

    pub struct Guard {
        /// Kill-on-close job object holding the command's whole tree; `None`
        /// when the job could not be created or the child could not be
        /// assigned, which is a containment failure.
        job: Option<win32job::Job>,
    }

    pub fn prepare(_cmd: &mut Command) {}

    pub fn attach(child: &Child) -> Guard {
        let mut info = win32job::ExtendedLimitInfo::new();
        info.limit_kill_on_job_close();
        let job = win32job::Job::create_with_limit_info(&info)
            .and_then(|job| {
                job.assign_process(child.as_raw_handle() as isize)?;
                Ok(job)
            })
            .ok();
        Guard { job }
    }

    impl Guard {
        /// Reap the direct child, then close the job handle: kill-on-close
        /// terminates every remaining member. Containment holds iff the tree
        /// was in the job from the start.
        pub fn teardown(self, child: &mut Child) -> bool {
            _ = child.wait();
            let established = self.job.is_some();
            drop(self.job);
            established
        }
    }
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
    before: &RepoIdentity,
    after: &RepoIdentity,
    repo: &EvidenceRepoIdentity,
) -> String {
    format!(
        "command: {command}\nexit_code: {}\ntimed_out: {}\ntruncated: {}\ncontained: {}\n\
         head_before: {}\nhead_after: {}\nhead_changed: {}\n\
         diff_hash_before: {}\ndiff_hash_after: {}\n\
         dirty_before ({}): {}\ndirty_after ({}): {}\nnewly_dirty ({}): {}\n\
         \n--- stdout ---\n{}\n--- stderr ---\n{}\n",
        run.exit_code,
        run.timed_out,
        run.truncated,
        run.contained,
        repo.head_before,
        repo.head_after,
        repo.head_changed,
        repo.diff_hash_before,
        repo.diff_hash_after,
        before.dirty.len(),
        before.dirty.join(", "),
        after.dirty.len(),
        after.dirty.join(", "),
        repo.newly_dirty.len(),
        repo.newly_dirty.join(", "),
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr),
    )
}

/// Env vars whose values are treated as secrets and scrubbed from stored
/// command output. This is the MVP env-scrubbing stub; the full redaction
/// model is Open Question 18 (`OPEN-ITEMS.md`). Shared with the run receipt,
/// whose included notes pass the same scrubbing.
pub(crate) fn secret_env_values() -> Vec<(String, String)> {
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
pub(crate) fn scrub_secrets(data: &[u8], secrets: &[(String, String)]) -> Vec<u8> {
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
    use super::BaselineDraft;
    use super::ControlBaselineRecord;
    use super::ControlCleanup;
    use super::ControlIsolation;
    use super::ControlStatus;
    use super::EvidenceControl;
    use super::EvidenceRepoIdentity;
    use super::control_verdict;
    use super::is_secret_name;
    use super::scrub_secrets;

    fn baseline(exit_code: i32) -> ControlBaselineRecord {
        ControlBaselineRecord {
            commit: "base".into(),
            exit_code,
            stdout_hash: "sha256:x".into(),
            artifact: "evidence/ev.baseline.txt".into(),
            artifact_hash: "sha256:y".into(),
            contained: true,
            repo: EvidenceRepoIdentity {
                head_before: "base".into(),
                head_after: "base".into(),
                head_changed: false,
                diff_hash_before: "sha256:d".into(),
                diff_hash_after: "sha256:d".into(),
                newly_dirty: vec![],
            },
        }
    }

    fn draft(exit_code: i32, cleanup: ControlCleanup, blocked: Vec<String>) -> BaselineDraft {
        BaselineDraft {
            kind: EvidenceControl::FailBeforePassAfter,
            baseline: Some(baseline(exit_code)),
            isolation: ControlIsolation {
                path: "/tmp/control-wt".into(),
                cleanup,
            },
            blocked,
        }
    }

    // A leaked isolation path never reports passed, and the leak is surfaced.
    #[test]
    fn leaked_isolation_never_reports_passed() {
        let verdict = control_verdict(draft(1, ControlCleanup::Leaked, vec![]), 0);
        assert_eq!(verdict.status, ControlStatus::Blocked);
        let note = verdict.note.expect("leak is surfaced");
        assert!(note.contains("leaked at /tmp/control-wt"), "{note}");
    }

    // A genuine semantic failure stays failed when the path also leaked.
    #[test]
    fn leaked_isolation_keeps_a_semantic_failure() {
        let verdict = control_verdict(draft(0, ControlCleanup::Leaked, vec![]), 0);
        assert_eq!(verdict.status, ControlStatus::Failed);
    }

    // Environment problems block the control even when the exit codes would
    // otherwise read as a pass — never a synthesized failure or pass.
    #[test]
    fn blocked_environment_overrides_exit_codes() {
        let verdict = control_verdict(
            draft(1, ControlCleanup::Removed, vec!["baseline env gone".into()]),
            0,
        );
        assert_eq!(verdict.status, ControlStatus::Blocked);
    }

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

    // A timed-out command is contained: the whole process group is torn down,
    // so the reader sees EOF instead of blocking on a pipe an orphaned
    // descendant still holds. (Unix-only: relies on `sleep` semantics.)
    #[cfg(unix)]
    #[test]
    fn timeout_kills_descendants_and_frees_the_readers() {
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
            run.contained,
            "the process group teardown should be confirmed"
        );
        assert!(
            !run.reader_abandoned,
            "group teardown should free the readers via EOF"
        );
        assert_eq!(run.stdout, b"hi\n", "pre-timeout output is kept");
        // 1s timeout + bounded teardown probe; a generous bound proves it did
        // not block on the 30s sleep.
        assert!(
            elapsed < Duration::from_secs(15),
            "collector blocked too long: {elapsed:?}"
        );
    }
}
