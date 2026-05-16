//! Shared fixtures for `speccy status` integration tests.
//!
//! Each integration test binary compiles this module independently and
//! uses only a subset of the helpers. The module-level expect below
//! silences dead-code warnings in test binaries that exercise only a
//! subset; a deliberately-unused [`touch_for_dead_code_expect`] function
//! guarantees the expectation is fulfilled in every binary.

#![expect(
    dead_code,
    reason = "shared test helpers; each test binary uses only a subset"
)]

use camino::Utf8Path;
use camino::Utf8PathBuf;
use indoc::indoc;
use std::fmt::Write as _;
use tempfile::TempDir;

pub type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub struct Workspace {
    pub _dir: TempDir,
    pub root: Utf8PathBuf,
}

impl Workspace {
    pub fn new() -> TestResult<Self> {
        let dir = tempfile::tempdir()?;
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf())
            .map_err(|p| format!("tempdir path must be UTF-8: {}", p.display()))?;
        fs_err::create_dir_all(root.join(".speccy").as_std_path())?;
        Ok(Workspace { _dir: dir, root })
    }
}

pub fn write_spec(
    root: &Utf8Path,
    dir_name: &str,
    spec_md: &str,
    // Legacy pre-SPEC-0019 spec.toml content. The third argument is
    // preserved so the many call sites don't all need touch-ups in this
    // task. When the string is non-empty it is written to disk, which
    // surfaces as `WorkspaceError::StraySpecToml` (recorded on the
    // parsed spec). T-006 will sweep the remaining call sites and drop
    // the parameter; for now writing it keeps tests that expected a
    // per-spec parse failure (`speccy check` warning, exit code 1)
    // working.
    legacy_spec_toml: &str,
    tasks_md: Option<&str>,
) -> TestResult<Utf8PathBuf> {
    let dir = root.join(".speccy").join("specs").join(dir_name);
    fs_err::create_dir_all(dir.as_std_path())?;
    fs_err::write(dir.join("SPEC.md").as_std_path(), spec_md)?;
    if !legacy_spec_toml.is_empty() {
        fs_err::write(dir.join("spec.toml").as_std_path(), legacy_spec_toml)?;
    }
    if let Some(tm) = tasks_md {
        fs_err::write(dir.join("TASKS.md").as_std_path(), tm)?;
    }
    Ok(dir)
}

pub fn spec_md_template(id: &str, status: &str) -> String {
    let template = indoc! {r#"
        ---
        id: __ID__
        slug: x
        title: Example __ID__
        status: __STATUS__
        created: 2026-05-11
        ---

        # __ID__

        <!-- speccy:requirement id="REQ-001" -->
        ### REQ-001: First
        Body.
        <!-- speccy:scenario id="CHK-001" -->
        Given REQ-001, when the suite runs, then it covers REQ-001.
        <!-- /speccy:scenario -->
        <!-- /speccy:requirement -->

        ## Changelog

        <!-- speccy:changelog -->
        | Date | Author | Summary |
        |------|--------|---------|
        | 2026-05-11 | t | init |
        <!-- /speccy:changelog -->
    "#};
    template.replace("__ID__", id).replace("__STATUS__", status)
}

pub fn spec_md_with_open_questions(id: &str, status: &str, questions: usize) -> String {
    let base = spec_md_template(id, status);
    // Inject the Open questions section before the changelog so the
    // marker parser still sees the required `speccy:changelog` block.
    let marker = "## Changelog";
    let split_idx = base.find(marker).unwrap_or(base.len());
    let (before, after) = base.split_at(split_idx);
    let mut s = String::from(before);
    s.push_str("## Open questions\n\n");
    for i in 0..questions {
        if writeln!(s, "- [ ] open question {i}").is_err() {
            break;
        }
    }
    s.push('\n');
    s.push_str(after);
    s
}

/// Deliberately-unused helper. Each integration test binary uses only
/// a subset of this module's helpers; this function is never called,
/// guaranteeing the module-level `expect(dead_code)` is fulfilled.
pub fn touch_for_dead_code_expect() {
    let _ = indoc! {""};
}

pub fn valid_spec_toml() -> String {
    indoc! {r#"
        schema_version = 1

        [[requirements]]
        id = "REQ-001"
        checks = ["CHK-001"]

        [[checks]]
        id = "CHK-001"
        scenario = "Given REQ-001, when the suite runs, then it covers REQ-001."
    "#}
    .to_owned()
}

pub fn bootstrap_tasks_md(spec_id: &str) -> String {
    format!(
        "---\nspec: {spec_id}\nspec_hash_at_generation: bootstrap-pending\ngenerated_at: 2026-05-11T00:00:00Z\n---\n\n# Tasks: {spec_id}\n\n- [ ] **T-001**: first\n  - Covers: REQ-001\n",
    )
}
