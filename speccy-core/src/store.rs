//! Runtime storage, external to the target repo (DESIGN § Storage Model).
//!
//! Everything operational lives under `~/.speccy/` (override `SPECCY_HOME`).
//! The canonical source of truth is the append-only `events.jsonl` per spec
//! and per run; projections are rebuilt by replay. Appends use verified
//! read-back so a crash never leaves a half-written transition.

use crate::error::Result;
use crate::error::SpeccyError;
use crate::event::Event;
use crate::event::LoggedEvent;
use crate::gitx;
use crate::lease::LeaseState;
use crate::projection::RunProjection;
use crate::projection::SpecState;
use camino::Utf8Path;
use camino::Utf8PathBuf;
use fs_err::File;
use fs_err::OpenOptions;
use fs_err::{self as fs};
use fs4::FileExt;
use sha2::Digest;
use sha2::Sha256;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;

/// Proof that the per-workspace store lock is held. Minted only inside
/// [`Store::with_store_lock`] and required by `append_*_with`, so those appends
/// are unreachable without holding the lock. Zero-sized; the field is private
/// so no other module can forge one.
#[derive(Debug)]
pub struct StoreLockGuard(());

/// A resolved workspace bound to the current git repository.
#[derive(Debug, Clone)]
pub struct Store {
    pub home: Utf8PathBuf,
    pub workspace_id: String,
    pub workspace_root: Utf8PathBuf,
    pub git_root: Utf8PathBuf,
}

impl Store {
    /// Resolve the workspace from the current directory. Requires a git repo
    /// (non-git workspaces are unsupported — DESIGN § Non-Goals).
    ///
    /// # Errors
    ///
    /// Returns an error if the current directory cannot be read, is not
    /// UTF-8, or is not inside a git repository.
    pub fn open() -> Result<Store> {
        let cwd = std::env::current_dir()
            .map_err(|e| SpeccyError::io(format!("cannot read current directory: {e}")))?;
        let cwd = Utf8PathBuf::from_path_buf(cwd).map_err(|p| {
            SpeccyError::io(format!("current directory {} is not UTF-8", p.display()))
        })?;
        Self::open_at(&cwd)
    }

    /// Resolve the workspace for an explicit directory (used by tests).
    ///
    /// # Errors
    ///
    /// Returns an error if `dir` is not inside a git repository, cannot be
    /// canonicalized, or the workspace directory cannot be created.
    pub fn open_at(dir: &Utf8Path) -> Result<Store> {
        let git_root = gitx::toplevel(dir)?;
        let workspace_root = dir
            .canonicalize_utf8()
            .map_err(|e| SpeccyError::io(format!("cannot canonicalize {dir}: {e}")))?;
        let git_root = git_root.canonicalize_utf8().unwrap_or(git_root);
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
                "workspace_root": self.workspace_root.as_str(),
                "git_root": self.git_root.as_str(),
            });
            write_atomic(&meta, format!("{value}\n").as_bytes())?;
        }
        Ok(())
    }

    // --- paths ---

    /// This workspace's directory under the store root.
    #[must_use = "the computed path is useless if discarded"]
    pub fn workspace_dir(&self) -> Utf8PathBuf {
        self.home.join("workspaces").join(&self.workspace_id)
    }
    fn specs_dir(&self) -> Utf8PathBuf {
        self.workspace_dir().join("specs")
    }
    /// A spec's directory within this workspace.
    #[must_use = "the computed path is useless if discarded"]
    pub fn spec_dir(&self, spec_id: &str) -> Utf8PathBuf {
        self.specs_dir().join(spec_id)
    }
    /// A run's directory within its spec.
    #[must_use = "the computed path is useless if discarded"]
    pub fn run_dir(&self, spec_id: &str, run_id: &str) -> Utf8PathBuf {
        self.spec_dir(spec_id).join("runs").join(run_id)
    }

    // --- spec lifecycle ---

    /// Create a spec's directory and pin its public reference.
    ///
    /// # Errors
    ///
    /// Returns an error if the spec directory or reference file cannot be
    /// written.
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
    ///
    /// # Errors
    ///
    /// Returns an error if the specs directory cannot be read.
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
    ///
    /// # Errors
    ///
    /// Returns an error if the specs directory cannot be read, or if no spec
    /// has the given reference.
    pub fn resolve_spec_id(&self, spec_ref: &str) -> Result<String> {
        self.list_specs()?
            .into_iter()
            .find(|(_, r)| r == spec_ref)
            .map(|(id, _)| id)
            .ok_or_else(|| SpeccyError::not_found(format!("no spec with reference {spec_ref}")))
    }

    /// Run IDs for a spec, sorted (ULIDs sort chronologically).
    ///
    /// # Errors
    ///
    /// Returns an error if the runs directory cannot be read.
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
    ///
    /// # Errors
    ///
    /// Returns an error if the specs cannot be listed, or if no run with the
    /// given ID exists.
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

    fn spec_events_path(&self, spec_id: &str) -> Utf8PathBuf {
        self.spec_dir(spec_id).join("events.jsonl")
    }
    fn run_events_path(&self, spec_id: &str, run_id: &str) -> Utf8PathBuf {
        self.run_dir(spec_id, run_id).join("events.jsonl")
    }

    /// Append an event to the spec's event log, acquiring the store lock for
    /// the duration of the append.
    ///
    /// # Errors
    ///
    /// Returns an error if the event cannot be serialized, written, or its
    /// read-back verification fails.
    pub fn append_spec_event(&self, spec_id: &str, event: Event) -> Result<()> {
        self.with_store_lock(|guard| self.append_spec_event_with(guard, spec_id, event))
    }

    /// Append an event to the spec's event log while the caller already holds
    /// the store lock (proven by `_guard`). Used inside a multi-append cycle
    /// like `run next` that wraps the whole cycle in one `with_store_lock`.
    ///
    /// # Errors
    ///
    /// Returns an error if the event cannot be serialized, written, or its
    /// read-back verification fails.
    pub fn append_spec_event_with(
        &self,
        _guard: &StoreLockGuard,
        spec_id: &str,
        event: Event,
    ) -> Result<()> {
        append_event(&self.spec_events_path(spec_id), event)
    }

    /// Append an event to the run's event log, acquiring the store lock for the
    /// duration of the append.
    ///
    /// # Errors
    ///
    /// Returns an error if the run directory cannot be created, or if the
    /// event cannot be serialized, written, or its read-back verification
    /// fails.
    pub fn append_run_event(&self, spec_id: &str, run_id: &str, event: Event) -> Result<()> {
        self.with_store_lock(|guard| self.append_run_event_with(guard, spec_id, run_id, event))
    }

    /// Append an event to the run's event log while the caller already holds
    /// the store lock (proven by `_guard`).
    ///
    /// # Errors
    ///
    /// Returns an error if the run directory cannot be created, or if the
    /// event cannot be serialized, written, or its read-back verification
    /// fails.
    pub fn append_run_event_with(
        &self,
        _guard: &StoreLockGuard,
        spec_id: &str,
        run_id: &str,
        event: Event,
    ) -> Result<()> {
        let dir = self.run_dir(spec_id, run_id);
        fs::create_dir_all(&dir)?;
        append_event(&self.run_events_path(spec_id, run_id), event)
    }

    // --- locks (DESIGN § Storage Model, § Run Lease and Concurrent Writers) ---

    /// Serialize concurrent event appends on a per-workspace store lock. The
    /// closure receives a [`StoreLockGuard`] proving the lock is held, which it
    /// passes to `append_*_with` for every append it makes. `run next` wraps
    /// its whole cycle in one call so a concurrent cycle cannot read the
    /// same pre-transition state and apply a derived transition twice
    /// (DESIGN § Storage Model). Artifact files are per-ID and never
    /// contend, so they are written outside this lock.
    ///
    /// # Errors
    ///
    /// Returns an error if the lock file cannot be created or acquired, or if
    /// `f` returns an error.
    pub fn with_store_lock<T>(&self, f: impl FnOnce(&StoreLockGuard) -> Result<T>) -> Result<T> {
        self.with_lock(&self.workspace_dir().join(".store.lock"), || {
            // Minted only here, while the lock is held; a `&StoreLockGuard` is
            // therefore proof at compile time that the store lock is held.
            let guard = StoreLockGuard(());
            f(&guard)
        })
    }

    /// The workspace command lock (separate from the run lease): only one
    /// `kind: command` evidence execution runs at a time, even for lease-free
    /// reviewer personas.
    ///
    /// # Errors
    ///
    /// Returns an error if the lock file cannot be created or acquired, or if
    /// `f` returns an error.
    pub fn with_command_lock<T>(&self, f: impl FnOnce() -> Result<T>) -> Result<T> {
        self.with_lock(&self.workspace_dir().join(".command.lock"), f)
    }

    fn with_lock<T>(&self, path: &Utf8Path, f: impl FnOnce() -> Result<T>) -> Result<T> {
        fs::create_dir_all(self.workspace_dir())?;
        // The lock handle must be a `std::fs::File`: `fs4::FileExt` is
        // implemented for it, not for `fs_err::File`.
        let file = std::fs::File::create(path)
            .map_err(|e| SpeccyError::io(format!("failed to open lock {path}: {e}")))?;
        FileExt::lock(&file)
            .map_err(|e| SpeccyError::io(format!("failed to acquire {path}: {e}")))?;
        let result = f();
        // best-effort: the OS releases the lock on drop regardless of this result
        _ = FileExt::unlock(&file);
        result
    }

    // --- run lease ---

    fn lease_path(&self, spec_id: &str, run_id: &str) -> Utf8PathBuf {
        self.run_dir(spec_id, run_id).join("lease.json")
    }

    /// Read the current lease for a run, if any.
    ///
    /// # Errors
    ///
    /// Returns an error if the lease file exists but cannot be read or
    /// parsed.
    pub fn read_lease(&self, spec_id: &str, run_id: &str) -> Result<Option<LeaseState>> {
        let path = self.lease_path(spec_id, run_id);
        match fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text)
                .map(Some)
                .map_err(|e| SpeccyError::io(format!("corrupt lease {path}: {e}"))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Write the lease for a run, replacing any existing one.
    ///
    /// # Errors
    ///
    /// Returns an error if the lease cannot be serialized or written.
    pub fn write_lease(&self, spec_id: &str, run_id: &str, lease: &LeaseState) -> Result<()> {
        let path = self.lease_path(spec_id, run_id);
        let bytes = serde_json::to_vec(lease)
            .map_err(|e| SpeccyError::io(format!("failed to serialize lease: {e}")))?;
        write_atomic(&path, &bytes)
    }

    /// Confirm `token` matches the current live (non-expired) lease. Used to
    /// gate state-mutating operations.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no active lease, the lease has expired,
    /// or `token` does not match the current lease.
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

    /// Read all events for a spec's event log.
    ///
    /// # Errors
    ///
    /// Returns an error if the log cannot be read, or contains a truncated or
    /// corrupt record.
    pub fn read_spec_events(&self, spec_id: &str) -> Result<Vec<LoggedEvent>> {
        read_events(&self.spec_events_path(spec_id))
    }

    /// Read all events for a run's event log.
    ///
    /// # Errors
    ///
    /// Returns an error if the log cannot be read, or contains a truncated or
    /// corrupt record.
    pub fn read_run_events(&self, spec_id: &str, run_id: &str) -> Result<Vec<LoggedEvent>> {
        read_events(&self.run_events_path(spec_id, run_id))
    }

    // --- projections ---

    /// Replay a spec's event log into its current state.
    ///
    /// # Errors
    ///
    /// Returns an error if the events cannot be read, or if the spec has no
    /// events.
    pub fn spec_state(&self, spec_id: &str) -> Result<SpecState> {
        let events = self.read_spec_events(spec_id)?;
        SpecState::replay(&events)
            .ok_or_else(|| SpeccyError::not_found(format!("spec {spec_id} has no events")))
    }

    /// Load spec state by public reference.
    ///
    /// # Errors
    ///
    /// Returns an error if the reference cannot be resolved, or the spec's
    /// state cannot be loaded.
    pub fn spec_state_by_ref(&self, spec_ref: &str) -> Result<SpecState> {
        let spec_id = self.resolve_spec_id(spec_ref)?;
        self.spec_state(&spec_id)
    }

    /// Replay a run's event log into its current state.
    ///
    /// # Errors
    ///
    /// Returns an error if the events cannot be read, or if the run has no
    /// events.
    pub fn run_projection(&self, spec_id: &str, run_id: &str) -> Result<RunProjection> {
        let events = self.read_run_events(spec_id, run_id)?;
        RunProjection::replay(&events)
            .ok_or_else(|| SpeccyError::not_found(format!("run {run_id} has no events")))
    }

    /// Load a run projection by run ID alone (scans to find its spec).
    ///
    /// # Errors
    ///
    /// Returns an error if no run with the given ID exists, or its
    /// projection cannot be loaded.
    pub fn run_by_id(&self, run_id: &str) -> Result<(String, RunProjection)> {
        let (spec_id, _) = self.find_run(run_id)?;
        let run = self.run_projection(&spec_id, run_id)?;
        Ok((spec_id, run))
    }
}

/// `ws_<hash>` from the canonical workspace root plus git root.
fn workspace_id(workspace_root: &Utf8Path, git_root: &Utf8Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(workspace_root.as_str().as_bytes());
    hasher.update([0]);
    hasher.update(git_root.as_str().as_bytes());
    let digest = hasher.finalize();
    let prefix: Vec<u8> = digest.iter().take(3).copied().collect();
    format!("ws_{}", crate::hash::to_hex(&prefix))
}

/// The store root: `SPECCY_HOME`, else `~/.speccy`.
pub(crate) fn home_dir() -> Result<Utf8PathBuf> {
    if let Ok(dir) = std::env::var("SPECCY_HOME") {
        return Ok(Utf8PathBuf::from(dir));
    }
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|e| {
            SpeccyError::io(format!(
                "cannot locate home directory; set SPECCY_HOME: {e}"
            ))
        })?;
    Ok(Utf8PathBuf::from(home).join(".speccy"))
}

/// Append one event as a JSONL line with verified read-back.
fn append_event(path: &Utf8Path, event: Event) -> Result<()> {
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
            "append verification failed for {path}: read-back mismatch"
        )));
    }
    Ok(())
}

fn last_line(path: &Utf8Path) -> Result<Option<String>> {
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
fn read_events(path: &Utf8Path) -> Result<Vec<LoggedEvent>> {
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
            .map_err(|e| {
                SpeccyError::io(format!(
                    "non-UTF8 bytes in {path} at byte offset {line_start}: {e}"
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
                "truncated final record in {path} at byte offset {line_start}"
            )));
        }
        let logged: LoggedEvent = serde_json::from_str(text).map_err(|e| {
            SpeccyError::io(format!(
                "corrupt event in {path} at byte offset {line_start}: {e}"
            ))
        })?;
        events.push(logged);
    }
    Ok(events)
}

/// Atomic whole-file write: temp file, fsync, rename over target.
///
/// # Errors
///
/// Returns an error if the temp file cannot be written, or the rename
/// over `path` fails.
pub fn write_atomic(path: &Utf8Path, bytes: &[u8]) -> Result<()> {
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
    // Best-effort: fsync the parent directory so the rename is itself durable,
    // not just the file's data (DESIGN § Storage Model). A no-op on platforms
    // that cannot open a directory handle.
    if let Some(parent) = path.parent()
        && let Ok(dir) = File::open(parent)
    {
        // best-effort: a failed fsync here does not affect data durability
        _ = dir.sync_all();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SpecStatus;
    use camino::Utf8Path;

    fn valid_line() -> String {
        let ev = LoggedEvent::now(Event::SpecStatusChanged {
            to: SpecStatus::Accepted,
        });
        serde_json::to_string(&ev).expect("serialize event")
    }

    fn utf8_tempdir() -> (tempfile::TempDir, Utf8PathBuf) {
        let dir = tempfile::tempdir().expect("create tempdir");
        let path = Utf8Path::from_path(dir.path())
            .expect("tempdir path is UTF-8")
            .to_owned();
        (dir, path)
    }

    #[test]
    fn reads_valid_jsonl() {
        let (_dir, base) = utf8_tempdir();
        let path = base.join("events.jsonl");
        let content = format!("{}\n{}\n", valid_line(), valid_line());
        fs::write(&path, content).expect("write events");
        assert_eq!(read_events(&path).expect("read events").len(), 2);
    }

    #[test]
    fn truncated_tail_fails_closed_with_offset() {
        let (_dir, base) = utf8_tempdir();
        let path = base.join("events.jsonl");
        let good = valid_line();
        // Second record is half-written (no trailing newline).
        let half = good.get(..good.len() / 2).expect("split on char boundary");
        let content = format!("{good}\n{half}");
        fs::write(&path, &content).expect("write events");
        let err = read_events(&path).expect_err("truncated tail must fail");
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
        let (_dir, base) = utf8_tempdir();
        let path = base.join("events.jsonl");
        fs::write(&path, "{not valid json}\n").expect("write events");
        let err = read_events(&path).expect_err("corrupt line must fail");
        assert!(err.message.contains("corrupt event"), "{}", err.message);
    }
}
