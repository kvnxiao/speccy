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
use serde::Serialize;

/// Selected host pack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostChoice {
    /// Claude Code skill pack; destination `.claude/skills/<name>/SKILL.md`
    /// per SPEC-0015 (was `.claude/commands/` pre-v1).
    ClaudeCode,
    /// Codex skill pack; destination `.agents/skills/<name>/SKILL.md` per
    /// SPEC-0015 (the path `OpenAI`'s Codex docs list as the project-local
    /// scan location; was `.codex/skills/` pre-v1).
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

    /// Project-relative skill install directory split into its two
    /// path segments. SPEC-0015 documents the destination paths; the
    /// bundle layout now mirrors the install destination 1:1 via
    /// [`Self::install_roots`], so a separate sub-path constant is
    /// redundant.
    ///
    /// This split form is retained for callers that classify
    /// `--force` user-file safety: tests in `speccy-cli/tests/init.rs`
    /// (and the SPEC-0015 invariant preserved across SPEC-0016) join
    /// these segments onto the project root when reasoning about which
    /// shipped files the renderer will overwrite. Claude Code:
    /// `.claude/skills/`. Codex: `.agents/skills/` per `OpenAI`'s docs
    /// at `developers.openai.com/codex/skills`.
    #[must_use = "the destination path is where the copy lands on disk"]
    pub const fn destination_segments(self) -> [&'static str; 2] {
        match self {
            HostChoice::ClaudeCode => [".claude", "skills"],
            HostChoice::Codex => [".agents", "skills"],
        }
    }

    /// Project-relative install roots this host writes to.
    ///
    /// Claude Code writes only to `.claude/`. Codex writes to two
    /// siblings: `.agents/` for skills (per SPEC-0015 and `OpenAI`'s
    /// project-local skill scan path) and `.codex/` for subagents
    /// (per `OpenAI`'s Codex subagents docs, which list
    /// `.codex/agents/` as the project-local subagent scan path).
    ///
    /// The SPEC-0016 renderer iterates these to walk the matching
    /// `resources/agents/<root>/` subtrees in the embedded bundle.
    #[must_use = "the install roots drive which resources/agents/ subtrees are rendered"]
    pub const fn install_roots(self) -> &'static [&'static str] {
        match self {
            HostChoice::ClaudeCode => &[".claude"],
            HostChoice::Codex => &[".agents", ".codex"],
        }
    }

    /// `MiniJinja` template context for this host.
    ///
    /// The returned [`minijinja::Value`] carries four string-typed
    /// keys used by every `resources/agents/<host>/*.tmpl` wrapper
    /// and by the host-divergent blocks inside
    /// `resources/modules/skills/speccy-*.md`:
    ///
    /// - `host`: lowercase host name (`"claude-code"` or `"codex"`). The skill
    ///   bodies' `{% if host == "claude-code" %}` blocks key off this value.
    /// - `cmd_prefix`: `"/"` for Claude Code's slash-prefixed skill
    ///   invocations, `""` for Codex's bare skill names.
    /// - `host_display_name`: human-facing host name (`"Claude Code"` or
    ///   `"Codex"`) used in skill prose.
    /// - `skill_install_path`: project-relative skill install directory
    ///   (`".claude/skills"` or `".agents/skills"`), used by `speccy-init`'s
    ///   skill body to tell users where the pack lands.
    #[must_use = "the template context drives every substitution in resources/agents/<host>/*.tmpl"]
    pub fn template_context(self) -> minijinja::Value {
        minijinja::Value::from_serialize(self.template_context_raw())
    }

    fn template_context_raw(self) -> TemplateContext {
        match self {
            HostChoice::ClaudeCode => TemplateContext {
                host: "claude-code",
                cmd_prefix: "/",
                host_display_name: "Claude Code",
                skill_install_path: ".claude/skills",
            },
            HostChoice::Codex => TemplateContext {
                host: "codex",
                cmd_prefix: "",
                host_display_name: "Codex",
                skill_install_path: ".agents/skills",
            },
        }
    }
}

/// Internal serializable representation of [`HostChoice::template_context`].
///
/// Kept private: external callers receive a [`minijinja::Value`] so the
/// keys are stable but the carrier type can evolve. `Serialize` is the
/// only trait `MiniJinja` needs to materialise this into a `Value` via
/// `Value::from_serialize`.
#[derive(Debug, Clone, Copy, Serialize)]
struct TemplateContext {
    host: &'static str,
    cmd_prefix: &'static str,
    host_display_name: &'static str,
    skill_install_path: &'static str,
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

    #[test]
    fn install_roots_claude_code_is_dot_claude() {
        assert_eq!(HostChoice::ClaudeCode.install_roots(), &[".claude"]);
    }

    #[test]
    fn install_roots_codex_is_dot_agents_and_dot_codex() {
        assert_eq!(HostChoice::Codex.install_roots(), &[".agents", ".codex"]);
    }

    /// Render a probe template against the host's template context and
    /// return the result. Exercising the context through a real
    /// `MiniJinja` render keeps the test focused on what the renderer
    /// actually consumes — the four string keys, surfaced via the
    /// `{{ var }}` substitution path — rather than poking at the
    /// internal shape of [`minijinja::Value`].
    fn render_probe(host: HostChoice) -> String {
        let mut env = minijinja::Environment::new();
        env.set_undefined_behavior(minijinja::UndefinedBehavior::Strict);
        env.add_template(
            "probe",
            "{{ host }}|{{ cmd_prefix }}|{{ host_display_name }}|{{ skill_install_path }}",
        )
        .expect("probe template should register cleanly");
        let tmpl = env
            .get_template("probe")
            .expect("probe template should be retrievable after registration");
        tmpl.render(host.template_context())
            .expect("probe template should render with the host context")
    }

    #[test]
    fn template_context_claude_code_renders_expected_keys() {
        assert_eq!(
            render_probe(HostChoice::ClaudeCode),
            "claude-code|/|Claude Code|.claude/skills",
        );
    }

    #[test]
    fn template_context_codex_renders_expected_keys() {
        assert_eq!(
            render_probe(HostChoice::Codex),
            "codex||Codex|.agents/skills",
        );
    }
}
