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
    tasks_md: Option<&str>,
) -> TestResult<Utf8PathBuf> {
    let dir = root.join(".speccy").join("specs").join(dir_name);
    fs_err::create_dir_all(dir.as_std_path())?;
    fs_err::write(dir.join("SPEC.md").as_std_path(), spec_md)?;
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

        <goals>
        Example goals.
        </goals>

        <non-goals>
        Example non-goals.
        </non-goals>

        <user-stories>
        - Example user story.
        </user-stories>

        <requirement id="REQ-001">
        ### REQ-001: First
        Body.

        <done-when>
        - placeholder.
        </done-when>

        <behavior>
        - placeholder.
        </behavior>

        <scenario id="CHK-001">
        Given REQ-001, when the suite runs, then it covers REQ-001.
        </scenario>
        </requirement>

        ## Changelog

        <changelog>
        | Date | Author | Summary |
        |------|--------|---------|
        | 2026-05-11 | t | init |
        </changelog>
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

pub fn bootstrap_tasks_md(spec_id: &str) -> String {
    format!(
        "---\nspec: {spec_id}\nspec_hash_at_generation: bootstrap-pending\ngenerated_at: 2026-05-11T00:00:00Z\n---\n\n# Tasks: {spec_id}\n\n\n\n<task id=\"T-001\" state=\"pending\" covers=\"REQ-001\">\nfirst\n\n<task-scenarios>\n- placeholder.\n</task-scenarios>\n</task>\n",
    )
}

/// Lowercase hex SHA-256 of the given bytes. Mirrors the encoding the
/// production code uses for `tasks_hash` in VET.md gate blocks.
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest as _;
    const_hex::encode(sha2::Sha256::digest(bytes))
}

/// Wrap a `<task>` element body in the TASKS.md frontmatter shape with
/// a bootstrap-pending spec hash. Used by every `next_*` integration
/// test that needs a minimum-viable TASKS.md.
pub fn tasks_md_xml(spec_id: &str, tasks_xml: &str) -> String {
    format!(
        "---\nspec: {spec_id}\nspec_hash_at_generation: bootstrap-pending\ngenerated_at: 2026-05-11T00:00:00Z\n---\n\n# Tasks: {spec_id}\n\n\n\n{tasks_xml}\n\n",
    )
}

/// Render a single `<task>` element body covering REQ-001 with a
/// placeholder scenarios block.
pub fn task_xml(id: &str, state: &str) -> String {
    format!(
        "<task id=\"{id}\" state=\"{state}\" covers=\"REQ-001\">\ndo the thing\n\n<task-scenarios>\n- placeholder.\n</task-scenarios>\n</task>\n\n",
    )
}

/// Write a fresh, passing `journal/VET.md` whose `tasks_hash` matches
/// the supplied TASKS.md bytes. Drives the SPEC-0041 fresh-pass gate
/// branch in `speccy next`.
pub fn write_fresh_pass_vet_md(spec_dir: &Utf8Path, tasks_md: &str) -> TestResult {
    let hash = sha256_hex(tasks_md.as_bytes());
    let journal = spec_dir.join("journal");
    fs_err::create_dir_all(journal.as_std_path())?;
    let body = format!(
        "## Invocation 1\n\n<gate verdict=\"passed\" tasks_hash=\"{hash}\" date=\"2026-05-22T00:00:00Z\">\nstub.\n</gate>\n",
    );
    fs_err::write(journal.join("VET.md").as_std_path(), body)?;
    Ok(())
}
