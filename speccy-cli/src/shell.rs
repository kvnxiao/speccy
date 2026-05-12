//! Cross-platform shell-command builder for `speccy check`.
//!
//! Picks the host shell at compile time (Unix: `sh -c`, Windows:
//! `cmd /c`) and configures the working directory. Stdio inheritance
//! is the caller's responsibility (the default for [`std::process::Command`]
//! is inherit when no `stdout`/`stderr` is set, so callers just spawn).
//!
//! Runtime detection via `std::env::consts::OS` was rejected per
//! SPEC-0010 DEC-001: equivalent behaviour with extra cost.

use camino::Utf8Path;
use std::process::Command;

/// Build a [`Command`] that invokes `command` through the host shell
/// with `cwd` as the working directory.
///
/// Stdio is left at the default (inherited) so the child writes
/// directly to the parent's stdout/stderr.
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

#[cfg(test)]
mod tests {
    use super::shell_command;
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
        let cmd = shell_command("echo 'hi there'", &cwd);
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        let flag = if cfg!(windows) { "/c" } else { "-c" };
        assert_eq!(args, vec![flag.to_owned(), "echo 'hi there'".to_owned()]);
    }

    #[test]
    fn shell_command_sets_cwd() {
        let cwd = Utf8PathBuf::from("/tmp/speccy-test");
        let cmd = shell_command("true", &cwd);
        let got = cmd
            .get_current_dir()
            .expect("current_dir should be set on the Command");
        assert_eq!(got, cwd.as_std_path());
    }
}
