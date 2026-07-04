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
