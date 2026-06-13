#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! End-to-end tests for `speccy lock SPEC-NNNN`.
//!
//! Exercises the happy-path hash + timestamp rewrite and the SPEC.md
//! parse-failure precondition, plus the SPEC-not-found and
//! `--help` listing scenarios.

mod common;

use assert_cmd::Command;
use camino::Utf8Path;
use common::TestResult;
use common::Workspace;
use common::bootstrap_tasks_md;
use common::spec_md_template;
use common::write_spec;
use predicates::str::contains;
use serde_json::Value;

#[test]
fn lock_writes_hash_and_rfc3339_timestamp_into_tasks_md_frontmatter() -> TestResult {
    let ws = Workspace::new()?;
    let spec_dir = write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        Some(&bootstrap_tasks_md("SPEC-0001")),
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("lock")
        .arg("SPEC-0001")
        .current_dir(ws.root.as_std_path());
    cmd.assert().success();

    let tasks_md = fs_err::read_to_string(spec_dir.join("TASKS.md").as_std_path())?;
    assert!(
        !tasks_md.contains("bootstrap-pending"),
        "lock should replace the bootstrap sentinel: {tasks_md}",
    );
    let hash_line = tasks_md
        .lines()
        .find(|l| l.starts_with("spec_hash_at_generation:"))
        .expect("frontmatter must declare spec_hash_at_generation");
    let hash_value = hash_line
        .strip_prefix("spec_hash_at_generation:")
        .map(str::trim)
        .expect("prefix matched by find()");
    assert_eq!(
        hash_value.len(),
        64,
        "sha256 must render as 64 lowercase hex chars: {hash_value}",
    );
    assert!(
        hash_value
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()),
        "hash must be lowercase ASCII hex: {hash_value}",
    );

    let ts_line = tasks_md
        .lines()
        .find(|l| l.starts_with("generated_at:"))
        .expect("frontmatter must declare generated_at");
    let ts_value = ts_line
        .strip_prefix("generated_at:")
        .map(str::trim)
        .expect("prefix matched by find()");
    // RFC-3339 with trailing `Z`, e.g. `2026-05-19T15:30:42Z`.
    assert_eq!(ts_value.len(), 20, "RFC-3339 Z form: {ts_value}");
    assert!(ts_value.ends_with('Z'), "expected Z suffix: {ts_value}");
    assert!(
        ts_value.chars().nth(10) == Some('T') && ts_value.chars().nth(4) == Some('-'),
        "expected ISO date shape: {ts_value}",
    );
    Ok(())
}

#[test]
fn lock_missing_spec_exits_one_with_not_found_message() -> TestResult {
    let ws = Workspace::new()?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("lock")
        .arg("SPEC-9999")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .failure()
        .code(1)
        .stderr(contains("SPEC-9999"))
        .stderr(contains("not found"));
    Ok(())
}

#[test]
fn lock_spec_md_parse_failure_exits_one_and_tasks_md_unchanged() -> TestResult {
    let ws = Workspace::new()?;
    let malformed_spec_md = "no frontmatter\n";
    let tasks_before = bootstrap_tasks_md("SPEC-0001");
    let spec_dir = write_spec(&ws.root, "0001-foo", malformed_spec_md, Some(&tasks_before))?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("lock")
        .arg("SPEC-0001")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .failure()
        .code(1)
        .stderr(contains("failed to parse"));

    let tasks_after = fs_err::read_to_string(spec_dir.join("TASKS.md").as_std_path())?;
    assert_eq!(
        tasks_before, tasks_after,
        "TASKS.md must be byte-identical on SPEC.md parse failure",
    );
    Ok(())
}

#[test]
fn lock_appears_in_help_subcommands() -> TestResult {
    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("--help");
    cmd.assert().success().stdout(contains("lock"));
    Ok(())
}

#[test]
fn lock_invalid_spec_id_format_exits_two() -> TestResult {
    let ws = Workspace::new()?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("lock")
        .arg("FOO")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .failure()
        .code(2)
        .stderr(contains("invalid SPEC-ID"));
    Ok(())
}

#[test]
fn lock_outside_workspace_exits_one_with_clear_error() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let path = camino::Utf8PathBuf::from_path_buf(tmp.path().to_path_buf())
        .map_err(|p| format!("tempdir path must be UTF-8: {}", p.display()))?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("lock")
        .arg("SPEC-0001")
        .current_dir(path.as_std_path());
    cmd.assert()
        .failure()
        .code(1)
        .stderr(contains(".speccy/ directory not found"));
    Ok(())
}

#[test]
fn lock_missing_tasks_md_exits_one_without_creating_file() -> TestResult {
    let ws = Workspace::new()?;
    let spec_dir = write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        None,
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("lock")
        .arg("SPEC-0001")
        .current_dir(ws.root.as_std_path());
    cmd.assert()
        .failure()
        .code(1)
        .stderr(contains("TASKS.md not found"));

    assert!(
        !spec_dir.join("TASKS.md").as_std_path().exists(),
        "lock must not create a missing TASKS.md",
    );
    Ok(())
}

#[test]
fn lock_preserves_body_bytes_byte_identical() -> TestResult {
    let ws = Workspace::new()?;
    let body = "\n# Tasks\n\n\n\n<task id=\"T-001\" state=\"pending\" covers=\"REQ-001\">\nfirst\n\n<task-scenarios>\n- placeholder.\n</task-scenarios>\n</task>\n";
    let bootstrap = format!(
        "---\nspec: SPEC-0001\nspec_hash_at_generation: bootstrap-pending\ngenerated_at: 2026-05-11T00:00:00Z\n---{body}",
    );
    let spec_dir = write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        Some(&bootstrap),
    )?;

    let mut cmd = Command::cargo_bin("speccy")?;
    cmd.arg("lock")
        .arg("SPEC-0001")
        .current_dir(ws.root.as_std_path());
    cmd.assert().success();

    let after = fs_err::read_to_string(spec_dir.join("TASKS.md").as_std_path())?;
    assert!(
        after.ends_with(body),
        "body bytes (after the closing `---` fence) must remain byte-identical: {after}",
    );
    Ok(())
}

// --- Hash-value & resolution coverage ---
//
// The tests above gate the hash *shape* (64 lowercase hex chars). The four
// below gate that lock records the *right* hash and resolves the right
// spec: a cross-command staleness round-trip, invariance under a `status:`
// flip (canonical vs raw-byte hashing), mission-folder resolution, and the
// CLI's exit-1 mapping for a 3-way ID disagreement.

/// Run `speccy lock <id>` and assert it succeeds.
fn run_lock_ok(ws: &Workspace, id: &str) -> TestResult {
    Command::cargo_bin("speccy")?
        .args(["lock", id])
        .current_dir(ws.root.as_std_path())
        .assert()
        .success();
    Ok(())
}

/// Read `speccy status --json` and report whether spec `id` is stale.
fn spec_is_stale(ws: &Workspace, id: &str) -> TestResult<bool> {
    let stdout = Command::cargo_bin("speccy")?
        .args(["status", "--json"])
        .current_dir(ws.root.as_std_path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&stdout)?;
    let specs = json
        .get("specs")
        .and_then(Value::as_array)
        .ok_or("status --json output missing `specs` array")?;
    let spec = specs
        .iter()
        .find(|s| s.get("id").and_then(Value::as_str) == Some(id))
        .ok_or_else(|| format!("spec {id} absent from status --json output"))?;
    let stale = spec
        .get("stale")
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("spec {id} entry missing `stale` boolean"))?;
    Ok(stale)
}

/// Mutate the REQ-001 body so the SPEC.md canonical content hash changes.
fn dirty_requirement_body(spec_dir: &Utf8Path) -> TestResult {
    let path = spec_dir.join("SPEC.md");
    let before = fs_err::read_to_string(path.as_std_path())?;
    let after = before.replace("Body.", "Body. Amended requirement prose.");
    assert_ne!(
        before, after,
        "template must contain the REQ-001 body sentinel to mutate",
    );
    fs_err::write(path.as_std_path(), after)?;
    Ok(())
}

#[test]
fn lock_then_status_round_trips_staleness() -> TestResult {
    let ws = Workspace::new()?;
    let spec_dir = write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        Some(&bootstrap_tasks_md("SPEC-0001")),
    )?;

    // Locking records the canonical hash → the spec is fresh.
    run_lock_ok(&ws, "SPEC-0001")?;
    assert!(
        !spec_is_stale(&ws, "SPEC-0001")?,
        "freshly locked spec must report stale:false",
    );

    // Editing a Requirement body changes the canonical hash → drift.
    dirty_requirement_body(&spec_dir)?;
    assert!(
        spec_is_stale(&ws, "SPEC-0001")?,
        "editing a Requirement body must drive stale:true",
    );

    // Re-locking captures the new hash → fresh again.
    run_lock_ok(&ws, "SPEC-0001")?;
    assert!(
        !spec_is_stale(&ws, "SPEC-0001")?,
        "re-locking must clear staleness",
    );
    Ok(())
}

#[test]
fn lock_hash_invariant_under_status_flip() -> TestResult {
    let ws = Workspace::new()?;
    let spec_dir = write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        Some(&bootstrap_tasks_md("SPEC-0001")),
    )?;
    run_lock_ok(&ws, "SPEC-0001")?;
    assert!(!spec_is_stale(&ws, "SPEC-0001")?, "locked spec is fresh");

    // Flip ONLY the frontmatter `status:` value. The canonical content
    // hash excludes `status` (HASH_EXCLUDED_FRONTMATTER_FIELDS), so
    // staleness must not change — proving canonical, not raw-byte, hashing.
    let spec_md = spec_dir.join("SPEC.md");
    let before = fs_err::read_to_string(spec_md.as_std_path())?;
    let after = before.replace("status: in-progress", "status: implemented");
    assert_ne!(before, after, "status line must be present to flip");
    fs_err::write(spec_md.as_std_path(), after)?;

    assert!(
        !spec_is_stale(&ws, "SPEC-0001")?,
        "flipping status: must NOT flip staleness (hash excludes status)",
    );
    Ok(())
}

#[test]
fn lock_resolves_spec_in_mission_folder() -> TestResult {
    let ws = Workspace::new()?;
    // Spec grouped under a `platform/` mission (focus) folder.
    let spec_dir = write_spec(
        &ws.root,
        "platform/0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        Some(&bootstrap_tasks_md("SPEC-0001")),
    )?;

    Command::cargo_bin("speccy")?
        .args(["lock", "SPEC-0001"])
        .current_dir(ws.root.as_std_path())
        .assert()
        .success();

    let tasks_md = fs_err::read_to_string(spec_dir.join("TASKS.md").as_std_path())?;
    assert!(
        !tasks_md.contains("bootstrap-pending"),
        "lock must resolve and rewrite a mission-grouped spec's TASKS.md: {tasks_md}",
    );
    assert!(
        !spec_is_stale(&ws, "SPEC-0001")?,
        "mission-grouped spec must be non-stale after lock",
    );
    Ok(())
}

#[test]
fn lock_id_disagreement_exits_one_tasks_untouched() -> TestResult {
    let ws = Workspace::new()?;
    // folder=0001-foo (→ SPEC-0001), SPEC.md.id=SPEC-0002, TASKS.md.spec=SPEC-0001.
    // The 3-way ID guard fires. The error variant and byte-preservation are
    // covered in speccy-core/tests/tasks_commit.rs, so this asserts only the
    // CLI-level contract: exit 1 with TASKS.md byte-identical.
    let tasks_before = bootstrap_tasks_md("SPEC-0001");
    let spec_dir = write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0002", "in-progress"),
        Some(&tasks_before),
    )?;

    Command::cargo_bin("speccy")?
        .args(["lock", "SPEC-0001"])
        .current_dir(ws.root.as_std_path())
        .assert()
        .failure()
        .code(1)
        .stderr(contains("disagreement"));

    let tasks_after = fs_err::read_to_string(spec_dir.join("TASKS.md").as_std_path())?;
    assert_eq!(
        tasks_before, tasks_after,
        "TASKS.md must be byte-identical on ID disagreement",
    );
    Ok(())
}
