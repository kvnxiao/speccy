#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert!/assert_eq! macros and return Result for ? propagation in setup"
)]
//! TSK-* lint diagnostics.

mod lint_common;

use camino::Utf8PathBuf;
use indoc::indoc;
use lint_common::Fixture;
use lint_common::TestResult;
use lint_common::lint_fixture;
use lint_common::valid_spec_md;
use lint_common::write_spec_fixture;
use speccy_core::lint::types::Diagnostic;
use speccy_core::lint::types::Level;

fn assert_has_code(diags: &[Diagnostic], code: &str) {
    assert!(
        diags.iter().any(|d| d.code == code),
        "expected {code}, got: {:?}",
        diags.iter().map(|d| d.code).collect::<Vec<_>>(),
    );
}

fn tasks_md_xml(state: &str, covers: &str) -> String {
    format!(
        "---\nspec: SPEC-0001\nspec_hash_at_generation: bootstrap-pending\ngenerated_at: 2026-05-11T00:00:00Z\n---\n\n# Tasks: SPEC-0001\n\n<tasks spec=\"SPEC-0001\">\n\n<task id=\"T-001\" state=\"{state}\" covers=\"{covers}\">\nt\n\n<task-scenarios>\n- placeholder.\n</task-scenarios>\n</task>\n\n</tasks>\n",
    )
}

#[test]
fn tsk_001_fires_for_unknown_covered_req() -> TestResult {
    let fx = write_spec_fixture(
        &valid_spec_md("SPEC-0001"),
        Some(&tasks_md_xml("pending", "REQ-099")),
    )?;
    let diags = lint_fixture(&fx);
    assert_has_code(&diags, "TSK-001");
    Ok(())
}

#[test]
fn tsk_004_fires_when_frontmatter_missing_generated_at() -> TestResult {
    let tasks_md = indoc! {r#"
        ---
        spec: SPEC-0001
        spec_hash_at_generation: bootstrap-pending
        ---

        # Tasks: SPEC-0001

        <tasks spec="SPEC-0001">
        </tasks>
    "#};
    let fx = write_spec_fixture(&valid_spec_md("SPEC-0001"), Some(tasks_md))?;
    let diags = lint_fixture(&fx);
    assert_has_code(&diags, "TSK-004");
    Ok(())
}

#[test]
fn tsk_003_fires_at_info_for_bootstrap_pending() -> TestResult {
    let fx = write_spec_fixture(
        &valid_spec_md("SPEC-0001"),
        Some(&tasks_md_xml("pending", "REQ-001")),
    )?;
    let diags = lint_fixture(&fx);
    let tsk_003 = diags
        .iter()
        .find(|d| d.code == "TSK-003")
        .ok_or("TSK-003 expected")?;
    assert_eq!(tsk_003.level, Level::Info);
    assert!(
        tsk_003.message.contains("speccy tasks"),
        "message should advise speccy tasks --commit; got: {}",
        tsk_003.message,
    );
    Ok(())
}

/// Write a fixture under a properly-named `NNNN-slug` subdirectory so
/// `derive_spec_id_from_dir` resolves a folder-derived ID. Used by the
/// TSK-005 tests where folder digits are a load-bearing input.
fn write_named_fixture(
    folder_name: &str,
    spec_md: &str,
    tasks_md: Option<&str>,
) -> TestResult<Fixture> {
    let dir = tempfile::tempdir()?;
    let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf())
        .map_err(|p| format!("tempdir path must be UTF-8: {}", p.display()))?;
    let spec_dir = root.join(folder_name);
    fs_err::create_dir(spec_dir.as_std_path())?;

    let spec_md_path = spec_dir.join("SPEC.md");
    fs_err::write(spec_md_path.as_std_path(), spec_md)?;

    let tasks_md_path = match tasks_md {
        Some(content) => {
            let p = spec_dir.join("TASKS.md");
            fs_err::write(p.as_std_path(), content)?;
            Some(p)
        }
        None => None,
    };

    Ok(Fixture {
        _dir: dir,
        spec_md_path,
        tasks_md_path,
        dir_path: spec_dir,
    })
}

fn tasks_md_with_spec(spec_id: &str) -> String {
    format!(
        "---\nspec: {spec_id}\nspec_hash_at_generation: bootstrap-pending\ngenerated_at: 2026-05-11T00:00:00Z\n---\n\n# Tasks: {spec_id}\n\n<tasks spec=\"{spec_id}\">\n\n<task id=\"T-001\" state=\"pending\" covers=\"REQ-001\">\nt\n\n<task-scenarios>\n- placeholder.\n</task-scenarios>\n</task>\n\n</tasks>\n",
    )
}

#[test]
fn tsk_005_silent_when_all_three_ids_agree() -> TestResult {
    let fx = write_named_fixture(
        "0001-test-slug",
        &valid_spec_md("SPEC-0001"),
        Some(&tasks_md_with_spec("SPEC-0001")),
    )?;
    let diags = lint_fixture(&fx);
    assert!(
        !diags.iter().any(|d| d.code == "TSK-005"),
        "no TSK-005 expected when folder, SPEC.md id, and TASKS.md spec all agree; got: {:?}",
        diags.iter().map(|d| d.code).collect::<Vec<_>>(),
    );
    Ok(())
}

#[test]
fn tsk_005_fires_when_three_ids_disagree() -> TestResult {
    let fx = write_named_fixture(
        "0024-meaningful-hash-semantics",
        &valid_spec_md("SPEC-1234"),
        Some(&tasks_md_with_spec("SPEC-0024")),
    )?;
    let diags = lint_fixture(&fx);
    let tsk_005: Vec<&Diagnostic> = diags.iter().filter(|d| d.code == "TSK-005").collect();
    assert_eq!(
        tsk_005.len(),
        1,
        "expected exactly one TSK-005; got {} (all diags: {:?})",
        tsk_005.len(),
        diags.iter().map(|d| d.code).collect::<Vec<_>>(),
    );
    let diag = tsk_005
        .first()
        .expect("TSK-005 diagnostic should be present");
    assert_eq!(diag.level, Level::Error);
    assert!(
        diag.message.contains("SPEC-0024"),
        "message should name folder ID `SPEC-0024`; got: {}",
        diag.message,
    );
    assert!(
        diag.message.contains("SPEC-1234"),
        "message should name SPEC.md id `SPEC-1234`; got: {}",
        diag.message,
    );
    assert!(
        diag.message.contains("TASKS.md.spec"),
        "message should label the TASKS.md observation; got: {}",
        diag.message,
    );
    Ok(())
}

#[test]
fn tsk_005_silent_when_tasks_md_absent() -> TestResult {
    let fx = write_named_fixture("0024-no-tasks", &valid_spec_md("SPEC-1234"), None)?;
    let diags = lint_fixture(&fx);
    assert!(
        !diags.iter().any(|d| d.code == "TSK-005"),
        "TSK-005 must not fire without TASKS.md (no third observation)",
    );
    Ok(())
}

#[test]
fn tsk_005_silent_when_spec_md_unparseable() -> TestResult {
    let busted_spec_md = "---\nthis is not: valid: yaml:\n---\n\n# busted\n";
    let fx = write_named_fixture(
        "0024-busted",
        busted_spec_md,
        Some(&tasks_md_with_spec("SPEC-0024")),
    )?;
    let diags = lint_fixture(&fx);
    assert!(
        !diags.iter().any(|d| d.code == "TSK-005"),
        "TSK-005 must not fire when SPEC.md is unparseable (upstream parse-error rules surface that)",
    );
    Ok(())
}

#[test]
fn tsk_003_fires_at_warn_for_hash_mismatch() -> TestResult {
    let body = "---\nspec: SPEC-0001\nspec_hash_at_generation: sha256:deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef\ngenerated_at: 2026-05-11T00:00:00Z\n---\n\n# Tasks: SPEC-0001\n\n<tasks spec=\"SPEC-0001\">\n\n<task id=\"T-001\" state=\"pending\" covers=\"REQ-001\">\nt\n\n<task-scenarios>\n- placeholder.\n</task-scenarios>\n</task>\n\n</tasks>\n".to_owned();
    let fx = write_spec_fixture(&valid_spec_md("SPEC-0001"), Some(&body))?;
    let diags = lint_fixture(&fx);
    let tsk_003 = diags
        .iter()
        .find(|d| d.code == "TSK-003")
        .ok_or("TSK-003 expected")?;
    assert_eq!(tsk_003.level, Level::Warn);
    Ok(())
}
