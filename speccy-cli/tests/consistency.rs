#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! SPEC-0045 T-006: end-to-end fixture tests for the `consistency` block
//! in `speccy next --json`.
//!
//! Each test builds an isolated tempdir workspace containing a real
//! `.git/` repo plus a minimal `.speccy/specs/NNNN-slug/` tree, then
//! drives the production `speccy_cli::next::run` entry point and parses
//! the resulting JSON envelope. The fixtures exercise the four drift
//! kinds defined by REQ-006 plus the all-healthy path, and a final
//! source-grep test enforces CHK-011 (no mutating git commands in
//! `speccy-cli/src/` or `speccy-core/src/`).

mod common;

use camino::Utf8Path;
use camino::Utf8PathBuf;
use common::TestResult;
use indoc::indoc;
use serde_json::Value;
use speccy_cli::next::NextArgs;
use speccy_cli::next::run;
use std::process::Command;
use tempfile::TempDir;

const SPEC_ID: &str = "SPEC-0099";
const SLUG: &str = "0099-fixture";

const SPEC_MD: &str = indoc! {r#"
    ---
    id: SPEC-0099
    slug: fixture
    title: Fixture
    status: in-progress
    created: 2026-05-25
    supersedes: []
    ---

    # SPEC-0099: Fixture

    ## Summary

    Notes.

    ## Requirements

    <requirement id="REQ-001">
    ### REQ-001

    <done-when>
    - thing.
    </done-when>

    <behavior>
    - thing.
    </behavior>

    <scenario id="CHK-001">
    Given X when Y then Z.
    </scenario>
    </requirement>
"#};

struct GitFixture {
    _dir: TempDir,
    root: Utf8PathBuf,
    spec_dir: Utf8PathBuf,
}

fn git(root: &Utf8Path, args: &[&str]) -> TestResult<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(root.as_std_path())
        .output()?;
    if !out.status.success() {
        return Err(format!(
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(out.stdout)?.trim().to_owned())
}

fn tasks_md(task_xml: &str) -> String {
    format!(
        "---\nspec: {SPEC_ID}\nspec_hash_at_generation: bootstrap-pending\ngenerated_at: 2026-05-25T00:00:00Z\n---\n\n# Tasks: {SPEC_ID}\n\n{task_xml}\n"
    )
}

fn one_task(id: &str, state: &str) -> String {
    format!(
        "<task id=\"{id}\" state=\"{state}\" covers=\"REQ-001\">\ndo it\n<task-scenarios>\n- placeholder.\n</task-scenarios>\n</task>\n"
    )
}

/// Build a tempdir with `.git/` (one bootstrap commit so HEAD exists)
/// plus the `.speccy/specs/<SLUG>/` tree. Returns the fixture handle.
fn make_fixture(tasks_xml: &str, journal: Option<(&str, &str)>) -> TestResult<GitFixture> {
    let dir = tempfile::tempdir()?;
    let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf())
        .map_err(|p| format!("tempdir path must be UTF-8: {}", p.display()))?;

    // Initialise an isolated git repo (no global config bleed).
    git(&root, &["init", "--quiet", "--initial-branch=main"])?;
    git(&root, &["config", "user.email", "test@example.com"])?;
    git(&root, &["config", "user.name", "Test"])?;
    git(&root, &["config", "commit.gpgsign", "false"])?;

    let spec_dir = root.join(".speccy").join("specs").join(SLUG);
    fs_err::create_dir_all(spec_dir.as_std_path())?;
    fs_err::write(spec_dir.join("SPEC.md").as_std_path(), SPEC_MD)?;
    fs_err::write(spec_dir.join("TASKS.md").as_std_path(), tasks_md(tasks_xml))?;
    if let Some((task_id, body)) = journal {
        let journal_dir = spec_dir.join("journal");
        fs_err::create_dir_all(journal_dir.as_std_path())?;
        fs_err::write(
            journal_dir.join(format!("{task_id}.md")).as_std_path(),
            body,
        )?;
    }

    // Seed a bootstrap commit so HEAD exists and `git log --grep`
    // queries don't error on a no-HEAD repo.
    fs_err::write(root.join(".gitignore").as_std_path(), "")?;
    git(&root, &["add", "-A"])?;
    git(&root, &["commit", "--quiet", "-m", "bootstrap"])?;

    Ok(GitFixture {
        _dir: dir,
        root,
        spec_dir,
    })
}

/// Drop a tracked file edit into the working tree so `git status
/// --porcelain` reports it. Touches `.gitignore` (already tracked by
/// `make_fixture`).
fn dirty_tracked(root: &Utf8Path, files: &[&str]) -> TestResult {
    for (idx, name) in files.iter().enumerate() {
        let path = root.join(name);
        // Stage an initial empty version on first sight so subsequent
        // edits register as modifications instead of untracked adds.
        fs_err::write(path.as_std_path(), format!("seed-{idx}\n"))?;
    }
    git(root, &["add", "-A"])?;
    git(root, &["commit", "--quiet", "-m", "seed dirty files"])?;
    for name in files {
        let path = root.join(name);
        fs_err::write(path.as_std_path(), "modified\n")?;
    }
    Ok(())
}

/// Create a commit whose title carries the `[SPEC-NNNN/T-NNN]: …`
/// prefix and return the 40-char SHA.
fn task_commit(root: &Utf8Path, task_id: &str, summary: &str) -> TestResult<String> {
    let stub_path = root.join(format!("task_{task_id}.txt"));
    fs_err::write(stub_path.as_std_path(), summary)?;
    git(root, &["add", "-A"])?;
    let title = format!("[{SPEC_ID}/{task_id}]: {summary}");
    git(root, &["commit", "--quiet", "-m", &title])?;
    git(root, &["rev-parse", "HEAD"])
}

fn render_per_spec_json(root: &Utf8Path) -> TestResult<Value> {
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    run(
        &NextArgs {
            spec_id: Some(SPEC_ID.to_owned()),
            include_archive: false,
            json: true,
        },
        root,
        &mut out,
        &mut err,
    )?;
    let text = String::from_utf8(out)?;
    Ok(serde_json::from_str(&text)?)
}

fn consistency(envelope: &Value) -> &Value {
    envelope
        .get("consistency")
        .expect("envelope must have a top-level `consistency` field")
}

fn drifts(envelope: &Value) -> &Vec<Value> {
    consistency(envelope)
        .get("drifts")
        .and_then(Value::as_array)
        .expect("consistency.drifts must be a JSON array")
}

// --- CHK-012: state_completed_no_commit + dirty tree --------------------

#[test]
fn state_completed_no_commit_with_dirty_tree_is_blocking() -> TestResult {
    let fx = make_fixture(&one_task("T-001", "completed"), None)?;
    dirty_tracked(&fx.root, &["mod_a.txt", "mod_b.txt"])?;

    let env = render_per_spec_json(&fx.root)?;

    assert_eq!(
        consistency(&env).get("status").and_then(Value::as_str),
        Some("blocked"),
        "status must be blocked: {env}"
    );
    let drifts = drifts(&env);
    let scnc: Vec<&Value> = drifts
        .iter()
        .filter(|d| {
            d.get("kind").and_then(Value::as_str) == Some("state_completed_no_commit")
                && d.get("task_id").and_then(Value::as_str) == Some("T-001")
        })
        .collect();
    assert_eq!(
        scnc.len(),
        1,
        "exactly one state_completed_no_commit entry expected: {drifts:?}",
    );
    let entry = scnc.first().expect("one entry");
    assert_eq!(
        entry.get("severity").and_then(Value::as_str),
        Some("blocking")
    );
    let dirty = entry
        .pointer("/details/working_tree_dirty")
        .and_then(Value::as_bool);
    assert_eq!(
        dirty,
        Some(true),
        "working_tree_dirty must be true: {entry}"
    );
    assert_eq!(
        env.pointer("/next_action/kind").and_then(Value::as_str),
        Some("reconcile"),
        "next_action.kind must be reconcile: {env}"
    );
    Ok(())
}

// --- CHK-013: journal_xml_malformed -------------------------------------

#[test]
fn journal_xml_malformed_reports_kind_path_and_offset() -> TestResult {
    // Valid frontmatter, a well-formed implementer block, then a trailing
    // whitelisted open tag with no matching close -> strict parse fails
    // (unbalanced element) while the tolerant `scan_tags` recovery still
    // pairs the first block. Frontmatter is required: the recovery helper
    // reuses `scan_tags` behind `split_required`, so a frontmatter-less
    // journal recovers to offset 0 (SPEC-0062 CHK-003). The expected
    // offset is computed relative to this same `body`, so the frontmatter
    // prefix shifts the actual and expected close offsets together.
    let body = "---\nspec: SPEC-0099\ntask: T-001\ngenerated_at: 2026-05-25T00:00:00Z\n---\n\n<implementer date=\"2026-05-25T00:00:00Z\" model=\"m\" round=\"1\">\nbody\n</implementer>\n<implementer date=\"2026-05-25T01:00:00Z\" model=\"m\" round=\"2\">";
    let fx = make_fixture(&one_task("T-001", "completed"), Some(("T-001", body)))?;
    // Land a matching commit so we isolate the journal-malformed drift
    // from a parallel state_completed_no_commit entry.
    let _ = task_commit(&fx.root, "T-001", "land it")?;

    let env = render_per_spec_json(&fx.root)?;

    let entry = drifts(&env)
        .iter()
        .find(|d| d.get("kind").and_then(Value::as_str) == Some("journal_xml_malformed"))
        .ok_or("journal_xml_malformed drift expected")?;
    assert_eq!(entry.get("task_id").and_then(Value::as_str), Some("T-001"));
    assert_eq!(
        entry.get("severity").and_then(Value::as_str),
        Some("blocking")
    );
    let journal_path = entry
        .pointer("/details/journal_path")
        .and_then(Value::as_str)
        .ok_or("details.journal_path string")?;
    let on_disk = fx.spec_dir.join("journal").join("T-001.md");
    let expected_suffix = "journal/T-001.md";
    assert!(
        journal_path.ends_with(expected_suffix),
        "journal_path must end with `{expected_suffix}`, got `{journal_path}` (on-disk: {on_disk})",
    );
    let offset = entry
        .pointer("/details/last_well_formed_byte_offset")
        .and_then(Value::as_u64)
        .ok_or("details.last_well_formed_byte_offset must be a non-negative integer")?;
    let expected = body
        .find("</implementer>")
        .ok_or("close tag present in fixture body")?
        + "</implementer>".len();
    let expected_u64 = u64::try_from(expected)?;
    assert_eq!(
        offset, expected_u64,
        "byte offset must be the close of `</implementer>`",
    );
    Ok(())
}

// --- CHK-010: commit_without_state --------------------------------------

#[test]
fn commit_without_state_reports_40_hex_sha() -> TestResult {
    let fx = make_fixture(&one_task("T-002", "in-review"), None)?;
    let sha = task_commit(&fx.root, "T-002", "some task")?;

    let env = render_per_spec_json(&fx.root)?;

    assert_eq!(
        consistency(&env).get("status").and_then(Value::as_str),
        Some("drift"),
    );
    let entry = drifts(&env)
        .iter()
        .find(|d| d.get("kind").and_then(Value::as_str) == Some("commit_without_state"))
        .ok_or("commit_without_state drift expected")?;
    assert_eq!(entry.get("task_id").and_then(Value::as_str), Some("T-002"));
    assert_eq!(
        entry.get("severity").and_then(Value::as_str),
        Some("auto_fixable"),
    );
    let commit_sha = entry
        .pointer("/details/commit_sha")
        .and_then(Value::as_str)
        .ok_or("details.commit_sha string")?;
    assert_eq!(commit_sha.len(), 40, "commit_sha must be 40 hex chars");
    assert!(
        commit_sha.chars().all(|c| c.is_ascii_hexdigit()),
        "commit_sha must be hex: {commit_sha}",
    );
    assert_eq!(commit_sha, sha, "commit_sha must match the seeded HEAD");
    assert_eq!(
        env.pointer("/next_action/kind").and_then(Value::as_str),
        Some("reconcile"),
    );
    Ok(())
}

// --- state_in_progress_orphaned with four dirty files -------------------

#[test]
fn state_in_progress_orphaned_reports_dirty_files_count() -> TestResult {
    let fx = make_fixture(&one_task("T-003", "in-progress"), None)?;
    dirty_tracked(&fx.root, &["a.txt", "b.txt", "c.txt", "d.txt"])?;

    let env = render_per_spec_json(&fx.root)?;

    let entry = drifts(&env)
        .iter()
        .find(|d| d.get("kind").and_then(Value::as_str) == Some("state_in_progress_orphaned"))
        .ok_or("state_in_progress_orphaned drift expected")?;
    assert_eq!(entry.get("task_id").and_then(Value::as_str), Some("T-003"));
    assert_eq!(
        entry.get("severity").and_then(Value::as_str),
        Some("blocking")
    );
    let count = entry
        .pointer("/details/dirty_files_count")
        .and_then(Value::as_u64)
        .ok_or("details.dirty_files_count must be a non-negative integer")?;
    assert_eq!(count, 4, "expected four dirty files");
    Ok(())
}

// --- all-healthy: consistency.status == "ok", no reconcile override -----

#[test]
fn ok_status_when_completed_task_has_matching_commit() -> TestResult {
    let fx = make_fixture(&one_task("T-001", "completed"), None)?;
    let _ = task_commit(&fx.root, "T-001", "finish")?;

    let env = render_per_spec_json(&fx.root)?;

    assert_eq!(
        consistency(&env).get("status").and_then(Value::as_str),
        Some("ok"),
        "status must be ok: {env}",
    );
    let kind = env.pointer("/next_action/kind").and_then(Value::as_str);
    assert_ne!(
        kind,
        Some("reconcile"),
        "next_action.kind must not be reconcile on healthy state, got {kind:?}",
    );
    Ok(())
}

// --- CHK-011: no mutating git commands anywhere in src/ -----------------

#[test]
fn no_mutating_git_commands_in_source() -> TestResult {
    // Walk speccy-cli/src and speccy-core/src, strip line comments and
    // string-literal-looking contexts conservatively, and assert none of
    // the five mutating git command invocations appear. CHK-011 scopes
    // the check to "command invocations, not in comments or doc
    // strings", which we approximate by ignoring any line whose first
    // non-whitespace token is `//`, `//!`, `///`, `*`, or `/*`.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_root = Utf8PathBuf::from(manifest_dir)
        .parent()
        .ok_or("workspace root above speccy-cli")?
        .to_owned();
    let roots = [
        workspace_root.join("speccy-cli").join("src"),
        workspace_root.join("speccy-core").join("src"),
    ];
    let needles = [
        "git add",
        "git commit",
        "git restore",
        "git clean",
        "git stash",
    ];
    let mut offences: Vec<String> = Vec::new();
    for root in &roots {
        walk_rs_files(root, &mut |path, contents| {
            for (lineno, line) in contents.lines().enumerate() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("//")
                    || trimmed.starts_with("/*")
                    || trimmed.starts_with('*')
                {
                    continue;
                }
                for needle in &needles {
                    if line.contains(needle) {
                        offences.push(format!(
                            "{path}:{}: contains `{needle}`: {line}",
                            lineno + 1,
                        ));
                    }
                }
            }
        })?;
    }
    assert!(
        offences.is_empty(),
        "mutating git commands found in source tree:\n{}",
        offences.join("\n"),
    );
    Ok(())
}

fn walk_rs_files(root: &Utf8Path, visit: &mut dyn FnMut(&Utf8Path, &str)) -> TestResult {
    for entry in fs_err::read_dir(root.as_std_path())? {
        let entry = entry?;
        let path_buf = Utf8PathBuf::from_path_buf(entry.path())
            .map_err(|p| format!("non-UTF-8 path: {}", p.display()))?;
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            walk_rs_files(&path_buf, visit)?;
        } else if path_buf.extension() == Some("rs") {
            let contents = fs_err::read_to_string(path_buf.as_std_path())?;
            visit(&path_buf, &contents);
        }
    }
    Ok(())
}
