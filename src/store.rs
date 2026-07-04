//! Runtime storage, external to the target repo (DESIGN § Storage Model).
//!
//! Everything operational lives under `~/.speccy/` (override `SPECCY_HOME`).
//! The canonical source of truth is the append-only `events.jsonl` per spec
//! and per run; projections are rebuilt by replay. Appends use verified
//! read-back so a crash never leaves a half-written transition.

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use fs4::FileExt;
use sha2::{Digest, Sha256};

use crate::error::{Result, SpeccyError};
use crate::event::{Event, LoggedEvent};
use crate::gitx;
use crate::lease::LeaseState;
use crate::projection::{RunProjection, SpecState};

/// A resolved workspace bound to the current git repository.
#[derive(Debug, Clone)]
pub struct Store {
    pub home: PathBuf,
    pub workspace_id: String,
    pub workspace_root: PathBuf,
    pub git_root: PathBuf,
}

impl Store {
    /// Resolve the workspace from the current directory. Requires a git repo
    /// (non-git workspaces are unsupported — DESIGN § Non-Goals).
    pub fn open() -> Result<Store> {
        let cwd = std::env::current_dir()
            .map_err(|e| SpeccyError::io(format!("cannot read current directory: {e}")))?;
        Self::open_at(&cwd)
    }

    /// Resolve the workspace for an explicit directory (used by tests).
    pub fn open_at(dir: &Path) -> Result<Store> {
        let git_root = gitx::toplevel(dir)?;
        let workspace_root = fs::canonicalize(dir)
            .map_err(|e| SpeccyError::io(format!("cannot canonicalize {}: {e}", dir.display())))?;
        let git_root = fs::canonicalize(&git_root).unwrap_or(git_root);
        let workspace_id = workspace_id(&workspace_root, &git_root);
        let home = home_dir()?;
        let store = Store {
            home,
            workspace_id,
            workspace_root,
            git_root,
        };
        store.ensure_workspace()?;
        Ok(store)
    }

    fn ensure_workspace(&self) -> Result<()> {
        let dir = self.workspace_dir();
        fs::create_dir_all(dir.join("specs"))?;
        let meta = dir.join("workspace.json");
        if !meta.exists() {
            let value = serde_json::json!({
                "workspace_id": self.workspace_id,
                "workspace_root": self.workspace_root.to_string_lossy(),
                "git_root": self.git_root.to_string_lossy(),
            });
            write_atomic(&meta, format!("{value}\n").as_bytes())?;
        }
        Ok(())
    }

    // --- paths ---

    pub fn workspace_dir(&self) -> PathBuf {
        self.home.join("workspaces").join(&self.workspace_id)
    }
    fn specs_dir(&self) -> PathBuf {
        self.workspace_dir().join("specs")
    }
    pub fn spec_dir(&self, spec_id: &str) -> PathBuf {
        self.specs_dir().join(spec_id)
    }
    pub fn run_dir(&self, spec_id: &str, run_id: &str) -> PathBuf {
        self.spec_dir(spec_id).join("runs").join(run_id)
    }

    // --- spec lifecycle ---

    /// Create a spec's directory and pin its public reference.
    pub fn create_spec(&self, spec_id: &str, spec_ref: &str) -> Result<()> {
        let dir = self.spec_dir(spec_id);
        fs::create_dir_all(dir.join("runs"))?;
        write_atomic(
            &dir.join("spec-ref.txt"),
            format!("{spec_ref}\n").as_bytes(),
        )?;
        Ok(())
    }

    /// All `(spec_id, spec_ref)` pairs in this workspace.
    pub fn list_specs(&self) -> Result<Vec<(String, String)>> {
        let mut out = Vec::new();
        let dir = self.specs_dir();
        if !dir.exists() {
            return Ok(out);
        }
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            if !entry.path().is_dir() {
                continue;
            }
            let spec_id = entry.file_name().to_string_lossy().to_string();
            let ref_path = entry.path().join("spec-ref.txt");
            if let Ok(text) = fs::read_to_string(&ref_path) {
                out.push((spec_id, text.trim().to_string()));
            }
        }
        out.sort();
        Ok(out)
    }

    /// Resolve a `SPEC-...` reference to its internal spec ID.
    pub fn resolve_spec_id(&self, spec_ref: &str) -> Result<String> {
        self.list_specs()?
            .into_iter()
            .find(|(_, r)| r == spec_ref)
            .map(|(id, _)| id)
            .ok_or_else(|| SpeccyError::not_found(format!("no spec with reference {spec_ref}")))
    }

    /// Run IDs for a spec, sorted (ULIDs sort chronologically).
    pub fn list_runs(&self, spec_id: &str) -> Result<Vec<String>> {
        let dir = self.spec_dir(spec_id).join("runs");
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut ids = Vec::new();
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            if entry.path().is_dir() {
                ids.push(entry.file_name().to_string_lossy().to_string());
            }
        }
        ids.sort();
        Ok(ids)
    }

    /// Locate the `(spec_id, run_id)` for a run ID by scanning runs.
    pub fn find_run(&self, run_id: &str) -> Result<(String, String)> {
        for (spec_id, _) in self.list_specs()? {
            let runs = self.spec_dir(&spec_id).join("runs").join(run_id);
            if runs.is_dir() {
                return Ok((spec_id, run_id.to_string()));
            }
        }
        Err(SpeccyError::not_found(format!("no run {run_id}")))
    }

    // --- event log ---

    fn spec_events_path(&self, spec_id: &str) -> PathBuf {
        self.spec_dir(spec_id).join("events.jsonl")
    }
    fn run_events_path(&self, spec_id: &str, run_id: &str) -> PathBuf {
        self.run_dir(spec_id, run_id).join("events.jsonl")
    }

    pub fn append_spec_event(&self, spec_id: &str, event: Event) -> Result<()> {
        let path = self.spec_events_path(spec_id);
        self.with_store_lock(|| append_event(&path, event))
    }

    pub fn append_run_event(&self, spec_id: &str, run_id: &str, event: Event) -> Result<()> {
        let dir = self.run_dir(spec_id, run_id);
        fs::create_dir_all(&dir)?;
        let path = self.run_events_path(spec_id, run_id);
        self.with_store_lock(|| append_event(&path, event))
    }

    // --- locks (DESIGN § Storage Model, § Run Lease and Concurrent Writers) ---

    /// Serialize concurrent event appends on a per-workspace store lock, held
    /// only for the duration of the append. Artifact files are per-ID and
    /// never contend, so they are written outside this lock.
    pub fn with_store_lock<T>(&self, f: impl FnOnce() -> Result<T>) -> Result<T> {
        self.with_lock(self.workspace_dir().join(".store.lock"), f)
    }

    /// The workspace command lock (separate from the run lease): only one
    /// `kind: command` evidence execution runs at a time, even for lease-free
    /// reviewer personas.
    pub fn with_command_lock<T>(&self, f: impl FnOnce() -> Result<T>) -> Result<T> {
        self.with_lock(self.workspace_dir().join(".command.lock"), f)
    }

    fn with_lock<T>(&self, path: PathBuf, f: impl FnOnce() -> Result<T>) -> Result<T> {
        fs::create_dir_all(self.workspace_dir())?;
        let file = File::create(&path)?;
        FileExt::lock(&file)
            .map_err(|e| SpeccyError::io(format!("failed to acquire {}: {e}", path.display())))?;
        let result = f();
        let _ = FileExt::unlock(&file);
        result
    }

    // --- run lease ---

    fn lease_path(&self, spec_id: &str, run_id: &str) -> PathBuf {
        self.run_dir(spec_id, run_id).join("lease.json")
    }

    pub fn read_lease(&self, spec_id: &str, run_id: &str) -> Result<Option<LeaseState>> {
        let path = self.lease_path(spec_id, run_id);
        match fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text)
                .map(Some)
                .map_err(|e| SpeccyError::io(format!("corrupt lease {}: {e}", path.display()))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn write_lease(&self, spec_id: &str, run_id: &str, lease: &LeaseState) -> Result<()> {
        let path = self.lease_path(spec_id, run_id);
        let bytes = serde_json::to_vec(lease)
            .map_err(|e| SpeccyError::io(format!("failed to serialize lease: {e}")))?;
        write_atomic(&path, &bytes)
    }

    /// Confirm `token` matches the current live (non-expired) lease. Used to
    /// gate state-mutating operations.
    pub fn verify_lease(&self, spec_id: &str, run_id: &str, token: Option<&str>) -> Result<()> {
        let lease = self.read_lease(spec_id, run_id)?.ok_or_else(|| {
            SpeccyError::lease_held("no active lease on this run; call run next --agent first")
        })?;
        let now = jiff::Timestamp::now();
        if lease.is_expired(now) {
            return Err(SpeccyError::lease_held(
                "the run lease has expired; call run next to reacquire it",
            ));
        }
        match token {
            Some(t) if t == lease.token => Ok(()),
            _ => Err(SpeccyError::lease_held(format!(
                "run lease held by {} until {}",
                lease.agent, lease.expires_at
            ))),
        }
    }

    pub fn read_spec_events(&self, spec_id: &str) -> Result<Vec<LoggedEvent>> {
        read_events(&self.spec_events_path(spec_id))
    }

    pub fn read_run_events(&self, spec_id: &str, run_id: &str) -> Result<Vec<LoggedEvent>> {
        read_events(&self.run_events_path(spec_id, run_id))
    }

    // --- projections ---

    pub fn spec_state(&self, spec_id: &str) -> Result<SpecState> {
        let events = self.read_spec_events(spec_id)?;
        SpecState::replay(&events)
            .ok_or_else(|| SpeccyError::not_found(format!("spec {spec_id} has no events")))
    }

    /// Load spec state by public reference.
    pub fn spec_state_by_ref(&self, spec_ref: &str) -> Result<SpecState> {
        let spec_id = self.resolve_spec_id(spec_ref)?;
        self.spec_state(&spec_id)
    }

    pub fn run_projection(&self, spec_id: &str, run_id: &str) -> Result<RunProjection> {
        let events = self.read_run_events(spec_id, run_id)?;
        RunProjection::replay(&events)
            .ok_or_else(|| SpeccyError::not_found(format!("run {run_id} has no events")))
    }

    /// Load a run projection by run ID alone (scans to find its spec).
    pub fn run_by_id(&self, run_id: &str) -> Result<(String, RunProjection)> {
        let (spec_id, _) = self.find_run(run_id)?;
        let run = self.run_projection(&spec_id, run_id)?;
        Ok((spec_id, run))
    }
}

/// `ws_<hash>` from the canonical workspace root plus git root.
fn workspace_id(workspace_root: &Path, git_root: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(workspace_root.to_string_lossy().as_bytes());
    hasher.update([0]);
    hasher.update(git_root.to_string_lossy().as_bytes());
    let digest = hasher.finalize();
    let hex: String = digest.iter().take(3).map(|b| format!("{b:02x}")).collect();
    format!("ws_{hex}")
}

/// The store root: `SPECCY_HOME`, else `~/.speccy`.
fn home_dir() -> Result<PathBuf> {
    if let Ok(dir) = std::env::var("SPECCY_HOME") {
        return Ok(PathBuf::from(dir));
    }
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| SpeccyError::io("cannot locate home directory; set SPECCY_HOME"))?;
    Ok(PathBuf::from(home).join(".speccy"))
}

/// Append one event as a JSONL line with verified read-back.
fn append_event(path: &Path, event: Event) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let logged = LoggedEvent::now(event);
    let line = serde_json::to_string(&logged)
        .map_err(|e| SpeccyError::io(format!("failed to serialize event: {e}")))?;
    {
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        file.write_all(line.as_bytes())?;
        file.write_all(b"\n")?;
        file.flush()?;
        file.sync_all()?;
    }
    // Verified read-back: the last line must be exactly what we wrote.
    let last = last_line(path)?;
    if last.as_deref() != Some(line.as_str()) {
        return Err(SpeccyError::io(format!(
            "append verification failed for {}: read-back mismatch",
            path.display()
        )));
    }
    Ok(())
}

fn last_line(path: &Path) -> Result<Option<String>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut last = None;
    for line in reader.lines() {
        let line = line?;
        if !line.trim().is_empty() {
            last = Some(line);
        }
    }
    Ok(last)
}

/// Read and parse a JSONL event log, failing closed on a corrupt or truncated
/// line and naming the byte offset (DESIGN § Storage Model: fail-closed
/// truncated-tail detection).
fn read_events(path: &Path) -> Result<Vec<LoggedEvent>> {
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    let mut events = Vec::new();
    let mut offset = 0usize;
    for raw in bytes.split_inclusive(|&b| b == b'\n') {
        let line_start = offset;
        offset += raw.len();
        let text = std::str::from_utf8(raw)
            .map_err(|_| {
                SpeccyError::io(format!(
                    "non-UTF8 bytes in {} at byte offset {line_start}",
                    path.display()
                ))
            })?
            .trim();
        if text.is_empty() {
            continue;
        }
        // A well-formed record ends in a newline; a tail without one is a
        // half-written (truncated) append.
        if !raw.ends_with(b"\n") {
            return Err(SpeccyError::io(format!(
                "truncated final record in {} at byte offset {line_start}",
                path.display()
            )));
        }
        let logged: LoggedEvent = serde_json::from_str(text).map_err(|e| {
            SpeccyError::io(format!(
                "corrupt event in {} at byte offset {line_start}: {e}",
                path.display()
            ))
        })?;
        events.push(logged);
    }
    Ok(events)
}

/// Atomic whole-file write: temp file, fsync, rename over target.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension(format!(
        "tmp.{}",
        ulid::Ulid::new().to_string().to_lowercase()
    ));
    {
        let mut file = File::create(&tmp)?;
        file.write_all(bytes)?;
        file.flush()?;
        file.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SpecStatus;

    fn valid_line() -> String {
        let ev = LoggedEvent::now(Event::SpecStatusChanged {
            to: SpecStatus::Accepted,
        });
        serde_json::to_string(&ev).unwrap()
    }

    #[test]
    fn reads_valid_jsonl() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        let content = format!("{}\n{}\n", valid_line(), valid_line());
        fs::write(&path, content).unwrap();
        assert_eq!(read_events(&path).unwrap().len(), 2);
    }

    #[test]
    fn truncated_tail_fails_closed_with_offset() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        let good = valid_line();
        // Second record is half-written (no trailing newline).
        let content = format!("{good}\n{}", &good[..good.len() / 2]);
        fs::write(&path, &content).unwrap();
        let err = read_events(&path).unwrap_err();
        assert!(
            err.message.contains("truncated final record"),
            "{}",
            err.message
        );
        assert!(
            err.message
                .contains(&format!("byte offset {}", good.len() + 1)),
            "{}",
            err.message
        );
    }

    #[test]
    fn corrupt_line_fails_closed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        fs::write(&path, "{not valid json}\n").unwrap();
        let err = read_events(&path).unwrap_err();
        assert!(err.message.contains("corrupt event"), "{}", err.message);
    }
}
