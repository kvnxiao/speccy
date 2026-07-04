//! Runtime storage, external to the target repo (DESIGN § Storage Model).
//!
//! Everything operational lives under `~/.speccy/` (override `SPECCY_HOME`).
//! The canonical source of truth is the append-only `events.jsonl` per spec
//! and per run; projections are rebuilt by replay. Appends use verified
//! read-back so a crash never leaves a half-written transition.

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::{Result, SpeccyError};
use crate::event::{Event, LoggedEvent};
use crate::gitx;
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
        append_event(&self.spec_events_path(spec_id), event)
    }

    pub fn append_run_event(&self, spec_id: &str, run_id: &str, event: Event) -> Result<()> {
        let dir = self.run_dir(spec_id, run_id);
        fs::create_dir_all(&dir)?;
        append_event(&self.run_events_path(spec_id, run_id), event)
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

fn read_events(path: &Path) -> Result<Vec<LoggedEvent>> {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    let reader = BufReader::new(file);
    let mut events = Vec::new();
    for (idx, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let logged: LoggedEvent = serde_json::from_str(&line).map_err(|e| {
            SpeccyError::io(format!(
                "corrupt event at {}:{}: {e}",
                path.display(),
                idx + 1
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
