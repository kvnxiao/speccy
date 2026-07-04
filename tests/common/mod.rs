//! Shared test harness: a temp git repo + isolated `SPECCY_HOME`, and helpers
//! to drive the real `speccy` binary the way an install-pack skill would.

#![allow(dead_code)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;
use tempfile::TempDir;

pub struct Harness {
    pub repo: TempDir,
    pub home: TempDir,
    pub env: Vec<(String, String)>,
}

impl Harness {
    /// A fresh workspace: `git init`, identity, one committed file.
    pub fn new() -> Harness {
        let repo = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let h = Harness {
            repo,
            home,
            env: Vec::new(),
        };
        h.git(&["-c", "init.defaultBranch=main", "init"]);
        h.git(&["config", "user.email", "test@speccy.local"]);
        h.git(&["config", "user.name", "Test"]);
        h.write_file("README.md", "# test repo\n");
        h.git(&["add", "-A"]);
        h.git(&["commit", "-m", "initial"]);
        h
    }

    pub fn repo_path(&self) -> &Path {
        self.repo.path()
    }

    pub fn write_file(&self, rel: &str, contents: &str) {
        let path = self.repo.path().join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, contents).unwrap();
    }

    pub fn git(&self, args: &[&str]) -> String {
        let out = Command::new("git")
            .args(args)
            .current_dir(self.repo.path())
            .output()
            .expect("git runs");
        assert!(
            out.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).to_string()
    }

    fn bin() -> PathBuf {
        PathBuf::from(env!("CARGO_BIN_EXE_speccy"))
    }

    /// Set an env var applied to every subsequent `speccy` invocation.
    pub fn set_env(&mut self, key: &str, value: &str) {
        self.env.push((key.to_string(), value.to_string()));
    }

    fn base_command(&self) -> Command {
        let mut cmd = Command::new(Self::bin());
        cmd.current_dir(self.repo.path())
            .env("SPECCY_HOME", self.home.path());
        for (k, v) in &self.env {
            cmd.env(k, v);
        }
        cmd
    }

    /// Run a ctl command with no stdin; return the full envelope.
    pub fn ctl_raw(&self, args: &[&str]) -> Value {
        let out = self
            .base_command()
            .args(args)
            .output()
            .expect("speccy runs");
        parse_envelope(&out.stdout, args)
    }

    /// Run a ctl command; assert `ok: true`; return `data`.
    pub fn ctl(&self, args: &[&str]) -> Value {
        let env = self.ctl_raw(args);
        assert_eq!(
            env["ok"],
            Value::Bool(true),
            "expected ok for {args:?}: {env}"
        );
        env["data"].clone()
    }

    /// Run a ctl command with a JSON payload on stdin; assert ok; return data.
    pub fn ctl_in(&self, args: &[&str], payload: &Value) -> Value {
        let env = self.ctl_in_raw(args, payload);
        assert_eq!(
            env["ok"],
            Value::Bool(true),
            "expected ok for {args:?}: {env}"
        );
        env["data"].clone()
    }

    /// Run a ctl command with stdin payload; return the full envelope.
    pub fn ctl_in_raw(&self, args: &[&str], payload: &Value) -> Value {
        let mut child = self
            .base_command()
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("speccy spawns");
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(payload.to_string().as_bytes())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        parse_envelope(&out.stdout, args)
    }

    /// Create a directory under the repo (e.g. a harness config dir).
    pub fn mkdir(&self, rel: &str) {
        std::fs::create_dir_all(self.repo.path().join(rel)).unwrap();
    }

    /// Run a command; return `(stdout, success)`.
    pub fn output(&self, args: &[&str]) -> (String, bool) {
        let out = self
            .base_command()
            .args(args)
            .output()
            .expect("speccy runs");
        (
            String::from_utf8_lossy(&out.stdout).to_string(),
            out.status.success(),
        )
    }

    /// True if a repo-relative path exists.
    pub fn exists(&self, rel: &str) -> bool {
        self.repo.path().join(rel).exists()
    }

    /// Read a repo-relative file.
    pub fn read(&self, rel: &str) -> String {
        std::fs::read_to_string(self.repo.path().join(rel)).unwrap()
    }

    /// Recursively find and read the first file under `SPECCY_HOME` whose path
    /// contains `needle` — used to inspect stored artifacts without hardcoding
    /// opaque store paths.
    pub fn read_home_file_containing(&self, needle: &str) -> Option<String> {
        fn walk(dir: &Path, needle: &str) -> Option<String> {
            for entry in std::fs::read_dir(dir).ok()?.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(found) = walk(&path, needle) {
                        return Some(found);
                    }
                } else if path.to_string_lossy().replace('\\', "/").contains(needle) {
                    return std::fs::read_to_string(&path).ok();
                }
            }
            None
        }
        walk(self.home.path(), needle)
    }

    /// Run a human command; return stdout text.
    pub fn human(&self, args: &[&str]) -> String {
        let out = self
            .base_command()
            .args(args)
            .output()
            .expect("speccy runs");
        String::from_utf8_lossy(&out.stdout).to_string()
    }
}

fn parse_envelope(stdout: &[u8], args: &[&str]) -> Value {
    let text = String::from_utf8_lossy(stdout);
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("non-JSON output for {args:?}: {e}\n---\n{text}"))
}

use serde_json::json;

/// Create + approve a minimal single-task, single-review-requirement spec.
/// Returns `(spec_ref, revision)`.
pub fn approve_minimal(h: &Harness, title: &str) -> (String, String) {
    let start = h.ctl_in(
        &["ctl", "spec", "start", "--input", "-", "--json"],
        &json!({ "request": "do the thing", "title": title }),
    );
    let spec_ref = start["spec_ref"].as_str().unwrap().to_string();
    let drafted = h.ctl_in(
        &[
            "ctl",
            "spec",
            "record-draft",
            "--spec",
            &spec_ref,
            "--input",
            "-",
            "--json",
        ],
        &json!({
            "goal": "do the thing", "scope": { "in": ["x"] }, "risk": "standard",
            "requirements": [{ "id": "R1", "statement": "it works",
                "evidence": [{ "id": "E1", "kind": "review", "note": "review" }] }],
            "tasks": [{ "id": "T1", "title": "the task", "requirements": ["R1"] }]
        }),
    );
    assert_eq!(drafted["lint"]["clean"], json!(true), "{drafted}");
    let approved = h.ctl_in(
        &[
            "ctl",
            "spec",
            "record-decision",
            "--spec",
            &spec_ref,
            "--input",
            "-",
            "--json",
        ],
        &json!({ "type": "approve", "revision": "spec_rev_001-draft", "approved_in_prose": "go" }),
    );
    let revision = approved["approved_revision"].as_str().unwrap().to_string();
    (spec_ref, revision)
}

/// Drive the happy-path loop (review evidence, all requirements pass) until a
/// terminal directive, and return it.
pub fn drive_to_gate(h: &Harness, run: &str) -> Value {
    for _ in 0..80 {
        let d = h.ctl(&["ctl", "run", "next", "--run", run, "--agent", "a", "--json"]);
        let lease = d["lease"]["token"].as_str().unwrap().to_string();
        match d["action"].as_str().unwrap() {
            "claim_task" => {
                let task = d["subject"]["task"].as_str().unwrap();
                h.ctl(&[
                    "ctl", "task", "claim", "--run", run, "--task", task, "--agent", "a",
                    "--lease", &lease, "--json",
                ]);
            }
            "dispatch_worker" => {
                let task = d["subject"]["task"].as_str().unwrap().to_string();
                let round = d["round"]["current"].as_u64().unwrap();
                h.write_file(&format!("src/{task}_r{round}.txt"), "work\n");
                let reqs = d["subject"]["requirements"].clone();
                h.ctl_in(
                    &[
                        "ctl",
                        "task",
                        "record-handoff",
                        "--run",
                        run,
                        "--lease",
                        &lease,
                        "--input",
                        "-",
                        "--json",
                    ],
                    &json!({ "task": task, "round": round, "summary": "did it",
                             "requirements_claimed": reqs }),
                );
            }
            "dispatch_verifier" => {
                let reqs: Vec<String> = d["subject"]["requirements"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_str().unwrap().to_string())
                    .collect();
                let mut updates = Vec::new();
                for r in &reqs {
                    let ev = h.ctl_in(
                        &[
                            "ctl", "evidence", "record", "--run", run, "--input", "-", "--json",
                        ],
                        &json!({ "requirement": r, "kind": "review", "collected_by": "v" }),
                    );
                    updates.push(json!({ "requirement": r, "status": "passed",
                                         "evidence": [ev["id"]] }));
                }
                h.ctl_in(
                    &[
                        "ctl",
                        "requirement",
                        "set-status",
                        "--run",
                        run,
                        "--lease",
                        &lease,
                        "--input",
                        "-",
                        "--json",
                    ],
                    &json!({ "updates": updates }),
                );
            }
            _ => return d,
        }
    }
    panic!("loop did not terminate");
}
