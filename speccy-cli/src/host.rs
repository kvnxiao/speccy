//! Host detection for `speccy init`.
//!
//! Pure decision logic: takes an optional `--host` flag and a project
//! root path, returns either the chosen [`HostChoice`] (optionally with
//! a stderr-bound warning) or an [`InitError`] that the CLI maps to an
//! exit code.
//!
//! Precedence (SPEC-0002 REQ-003 + DEC-004):
//! 1. Explicit `--host <name>` always wins.
//! 2. Probe in declared order: `.claude/`, `.codex/`, `.cursor/`.
//! 3. `.cursor/` (without `--host`) refuses with [`InitError::CursorDetected`].
//! 4. No host directories: fall back to `claude-code` with a warning.

use crate::init::InitError;
use camino::Utf8Path;

/// Selected host pack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostChoice {
    /// Claude Code skill pack; destination `.claude/commands/`.
    ClaudeCode,
    /// Codex skill pack; destination `.codex/skills/`.
    Codex,
}

impl HostChoice {
    /// Canonical lowercase flag name (`"claude-code"` or `"codex"`).
    #[must_use = "the flag name is what the user typed and what error messages report"]
    pub const fn flag_name(self) -> &'static str {
        match self {
            HostChoice::ClaudeCode => "claude-code",
            HostChoice::Codex => "codex",
        }
    }

    /// Sub-path inside the embedded `skills/` bundle to copy from.
    #[must_use = "the sub-path drives the embedded-bundle source for the copy"]
    pub const fn bundle_subpath(self) -> &'static str {
        match self {
            HostChoice::ClaudeCode => "claude-code",
            HostChoice::Codex => "codex",
        }
    }

    /// Destination directory relative to the project root.
    #[must_use = "the destination path is where the copy lands on disk"]
    pub const fn destination_segments(self) -> [&'static str; 2] {
        match self {
            HostChoice::ClaudeCode => [".claude", "commands"],
            HostChoice::Codex => [".codex", "skills"],
        }
    }
}

/// Supported `--host` values, in the order they're listed in error
/// messages.
pub const SUPPORTED_HOSTS: &[&str] = &["claude-code", "codex"];

/// Outcome of [`detect_host`] when a choice was made.
#[derive(Debug, Clone)]
#[must_use = "the detection outcome carries the chosen host and any warning"]
pub struct Detected {
    /// Selected host pack.
    pub host: HostChoice,
    /// Warning message destined for stderr, or `None` if the choice was
    /// unambiguous.
    pub warning: Option<String>,
}

/// Decide which host pack to install.
///
/// Returns [`Detected`] on success (optionally with a warning) or an
/// [`InitError`] variant the CLI maps to exit code 1.
///
/// # Errors
///
/// - [`InitError::UnknownHost`] when `flag` is set to a value outside
///   [`SUPPORTED_HOSTS`].
/// - [`InitError::CursorDetected`] when only `.cursor/` is present and no
///   `--host` override was provided.
pub fn detect_host(flag: Option<&str>, project_root: &Utf8Path) -> Result<Detected, InitError> {
    if let Some(name) = flag {
        return parse_host_flag(name).map(|host| Detected {
            host,
            warning: None,
        });
    }

    if exists_dir(project_root, ".claude") {
        return Ok(Detected {
            host: HostChoice::ClaudeCode,
            warning: None,
        });
    }
    if exists_dir(project_root, ".codex") {
        return Ok(Detected {
            host: HostChoice::Codex,
            warning: None,
        });
    }
    if exists_dir(project_root, ".cursor") {
        return Err(InitError::CursorDetected);
    }

    Ok(Detected {
        host: HostChoice::ClaudeCode,
        warning: Some(
            "no host directory detected (.claude/, .codex/); defaulting to claude-code".to_owned(),
        ),
    })
}

fn parse_host_flag(name: &str) -> Result<HostChoice, InitError> {
    match name {
        "claude-code" => Ok(HostChoice::ClaudeCode),
        "codex" => Ok(HostChoice::Codex),
        other => Err(InitError::UnknownHost {
            name: other.to_owned(),
            supported: SUPPORTED_HOSTS,
        }),
    }
}

fn exists_dir(root: &Utf8Path, name: &str) -> bool {
    fs_err::metadata(root.join(name).as_std_path()).is_ok_and(|m| m.is_dir())
}

#[cfg(test)]
mod tests {
    use super::HostChoice;
    use super::detect_host;
    use crate::init::InitError;
    use camino::Utf8PathBuf;
    use tempfile::TempDir;

    fn tmp_root() -> (TempDir, Utf8PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir should succeed in tests");
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf())
            .expect("tempdir path should be UTF-8");
        (dir, root)
    }

    fn mkdir(root: &Utf8PathBuf, name: &str) {
        fs_err::create_dir_all(root.join(name).as_std_path())
            .expect("create_dir_all should succeed in tests");
    }

    #[test]
    fn flag_wins_over_filesystem_signals() {
        let (_dir, root) = tmp_root();
        mkdir(&root, ".claude");
        mkdir(&root, ".codex");
        let detected = detect_host(Some("codex"), &root).expect("explicit --host should succeed");
        assert_eq!(detected.host, HostChoice::Codex);
        assert!(detected.warning.is_none());
    }

    #[test]
    fn claude_wins_when_both_present() {
        let (_dir, root) = tmp_root();
        mkdir(&root, ".claude");
        mkdir(&root, ".codex");
        let detected = detect_host(None, &root).expect("autodetect should succeed");
        assert_eq!(detected.host, HostChoice::ClaudeCode);
    }

    #[test]
    fn codex_picked_when_only_codex_present() {
        let (_dir, root) = tmp_root();
        mkdir(&root, ".codex");
        let detected = detect_host(None, &root).expect("autodetect should succeed");
        assert_eq!(detected.host, HostChoice::Codex);
    }

    #[test]
    fn cursor_only_refuses() {
        let (_dir, root) = tmp_root();
        mkdir(&root, ".cursor");
        let err = detect_host(None, &root).expect_err("cursor-only must refuse");
        assert!(matches!(err, InitError::CursorDetected));
    }

    #[test]
    fn unknown_flag_value_rejected() {
        let (_dir, root) = tmp_root();
        let err = detect_host(Some("cursor"), &root).expect_err("--host cursor must reject in v1");
        assert!(matches!(
            err,
            InitError::UnknownHost { ref name, .. } if name == "cursor"
        ));
    }

    #[test]
    fn no_signals_falls_back_to_claude_with_warning() {
        let (_dir, root) = tmp_root();
        let detected = detect_host(None, &root).expect("fallback must succeed");
        assert_eq!(detected.host, HostChoice::ClaudeCode);
        let warning = detected
            .warning
            .as_ref()
            .expect("fallback should carry a warning");
        assert!(warning.contains("claude-code"));
    }
}
