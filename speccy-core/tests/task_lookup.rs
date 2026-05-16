#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![allow(
    clippy::unwrap_in_result,
    reason = "test code may .expect() with descriptive messages inside TestResult fns"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! Integration tests for `speccy_core::task_lookup`.
//!
//! Covers SPEC-0008 REQ-001..REQ-003 and CHK-001..CHK-003.

mod lint_common;

use camino::Utf8PathBuf;
use indoc::indoc;
use lint_common::Fixture;
use lint_common::TestResult;
use lint_common::parse_fixture;
use lint_common::write_spec_fixture;
use speccy_core::parse::supersession::supersession_index;
use speccy_core::task_lookup::LookupError;
use speccy_core::task_lookup::TaskRef;
use speccy_core::task_lookup::find;
use speccy_core::task_lookup::parse_ref;
use speccy_core::workspace::Workspace;

// -- Helpers -----------------------------------------------------------------

fn spec_md_for(id: &str) -> String {
    let body = indoc! {r#"
        ---
        id: __ID__
        slug: x
        title: Example __ID__
        status: in-progress
        created: 2026-05-11
        ---

        # __ID__

        <!-- speccy:requirement id="REQ-001" -->
        ### REQ-001: First
        Body.
        <!-- speccy:scenario id="CHK-001" -->
        covers REQ-001
        <!-- /speccy:scenario -->
        <!-- /speccy:requirement -->

        ## Changelog

        <!-- speccy:changelog -->
        | Date | Author | Summary |
        |------|--------|---------|
        | 2026-05-11 | t | init |
        <!-- /speccy:changelog -->
    "#};
    body.replace("__ID__", id)
}

fn tasks_md_with(spec_id: &str, body: &str) -> String {
    format!(
        "---\nspec: {spec_id}\nspec_hash_at_generation: bootstrap-pending\ngenerated_at: 2026-05-11T00:00:00Z\n---\n\n# Tasks: {spec_id}\n\n{body}",
    )
}

fn make_fixture(spec_id: &str, tasks_body: &str) -> TestResult<Fixture> {
    let spec = spec_md_for(spec_id);
    let tasks = tasks_md_with(spec_id, tasks_body);
    write_spec_fixture(&spec, Some(&tasks))
}

fn make_workspace(fixtures: &[&Fixture]) -> Workspace {
    let specs: Vec<_> = fixtures.iter().map(|fx| parse_fixture(fx)).collect();
    let spec_md_refs: Vec<&_> = specs
        .iter()
        .filter_map(|s| s.spec_md.as_ref().ok())
        .collect();
    let supersession = supersession_index(&spec_md_refs);
    Workspace {
        project_root: Utf8PathBuf::from("/tmp/fake"),
        specs,
        supersession,
    }
}

// -- REQ-001 -- CHK-001: ref_parsing -----------------------------------------

#[test]
fn ref_parsing_accepts_unqualified_three_digits() {
    let parsed = parse_ref("T-001").expect("T-001 should parse");
    assert!(
        matches!(&parsed, TaskRef::Unqualified { id } if id == "T-001"),
        "expected Unqualified(T-001), got {parsed:?}",
    );
}

#[test]
fn ref_parsing_accepts_unqualified_four_digits() {
    let parsed = parse_ref("T-1234").expect("T-1234 should parse");
    assert!(
        matches!(&parsed, TaskRef::Unqualified { id } if id == "T-1234"),
        "expected Unqualified(T-1234), got {parsed:?}",
    );
}

#[test]
fn ref_parsing_accepts_qualified_form() {
    let parsed = parse_ref("SPEC-0001/T-001").expect("qualified should parse");
    assert!(
        matches!(
            &parsed,
            TaskRef::Qualified { spec_id, task_id }
                if spec_id == "SPEC-0001" && task_id == "T-001",
        ),
        "expected Qualified(SPEC-0001/T-001), got {parsed:?}",
    );
}

#[test]
fn ref_parsing_rejects_invalid_formats() {
    for bad in [
        "FOO",
        "T-",
        "T-AB",
        "T-12",
        "SPEC-0001/FOO",
        "/T-001",
        "SPEC-1/T-001",
        "spec-0001/T-001",
        "",
        " T-001",
    ] {
        let err = parse_ref(bad).expect_err("garbage input must fail format check");
        assert!(
            matches!(&err, LookupError::InvalidFormat { arg } if arg == bad),
            "expected InvalidFormat carrying `{bad}`, got {err:?}",
        );
    }
}

// -- REQ-002 -- CHK-002: workspace_lookup ------------------------------------

#[test]
fn workspace_lookup_finds_unique_unqualified_match() -> TestResult {
    let fx1 = make_fixture("SPEC-0001", "- [ ] **T-001**: first\n  - Covers: REQ-001\n")?;
    let fx2 = make_fixture(
        "SPEC-0002",
        "- [ ] **T-002**: second\n  - Covers: REQ-001\n",
    )?;
    let workspace = make_workspace(&[&fx1, &fx2]);

    let task_ref = parse_ref("T-001").expect("ref should parse");
    let location = find(&workspace, &task_ref).expect("unique match should resolve");
    assert_eq!(location.spec_id, "SPEC-0001");
    assert_eq!(location.task.id, "T-001");
    assert!(
        location.task_entry_raw.contains("**T-001**"),
        "task_entry_raw should contain the task line: {entry}",
        entry = location.task_entry_raw,
    );
    assert!(
        location.task_entry_raw.contains("Covers: REQ-001"),
        "task_entry_raw should contain sub-list bullets: {entry}",
        entry = location.task_entry_raw,
    );
    Ok(())
}

#[test]
fn workspace_lookup_finds_task_in_other_spec() -> TestResult {
    let fx1 = make_fixture("SPEC-0001", "- [ ] **T-001**: first\n  - Covers: REQ-001\n")?;
    let fx2 = make_fixture(
        "SPEC-0002",
        "- [ ] **T-002**: second\n  - Covers: REQ-001\n",
    )?;
    let workspace = make_workspace(&[&fx1, &fx2]);

    let task_ref = parse_ref("T-002").expect("ref should parse");
    let location = find(&workspace, &task_ref).expect("unique match should resolve");
    assert_eq!(location.spec_id, "SPEC-0002");
    assert_eq!(location.task.id, "T-002");
    Ok(())
}

#[test]
fn workspace_lookup_qualified_scopes_to_one_spec() -> TestResult {
    let fx1 = make_fixture(
        "SPEC-0001",
        "- [ ] **T-001**: in spec-1\n  - Covers: REQ-001\n",
    )?;
    let fx2 = make_fixture(
        "SPEC-0002",
        "- [ ] **T-001**: in spec-2\n  - Covers: REQ-001\n",
    )?;
    let workspace = make_workspace(&[&fx1, &fx2]);

    let qualified = parse_ref("SPEC-0001/T-001").expect("qualified must parse");
    let location =
        find(&workspace, &qualified).expect("qualified must bypass ambiguity for spec-1");
    assert_eq!(location.spec_id, "SPEC-0001");
    assert!(
        location.task_entry_raw.contains("in spec-1"),
        "qualified lookup should return spec-1's task body: {entry}",
        entry = location.task_entry_raw,
    );

    let qualified_b = parse_ref("SPEC-0002/T-001").expect("qualified must parse");
    let location_b = find(&workspace, &qualified_b).expect("qualified must resolve spec-2");
    assert_eq!(location_b.spec_id, "SPEC-0002");
    assert!(
        location_b.task_entry_raw.contains("in spec-2"),
        "qualified lookup should return spec-2's task body: {entry}",
        entry = location_b.task_entry_raw,
    );
    Ok(())
}

#[test]
fn workspace_lookup_missing_task_returns_not_found() -> TestResult {
    let fx = make_fixture("SPEC-0001", "- [ ] **T-001**: only\n")?;
    let workspace = make_workspace(&[&fx]);

    let task_ref = parse_ref("T-999").expect("ref should parse");
    let err = find(&workspace, &task_ref).expect_err("missing task must return error");
    assert!(
        matches!(&err, LookupError::NotFound { task_ref } if task_ref == "T-999"),
        "expected NotFound, got {err:?}",
    );
    Ok(())
}

#[test]
fn workspace_lookup_qualified_missing_in_named_spec_returns_not_found() -> TestResult {
    let fx1 = make_fixture("SPEC-0001", "- [ ] **T-001**: in spec-1\n")?;
    let fx2 = make_fixture("SPEC-0002", "- [ ] **T-002**: in spec-2\n")?;
    let workspace = make_workspace(&[&fx1, &fx2]);

    let qualified = parse_ref("SPEC-0001/T-002").expect("qualified parses");
    let err = find(&workspace, &qualified).expect_err("not present in scope spec");
    assert!(
        matches!(&err, LookupError::NotFound { task_ref } if task_ref == "SPEC-0001/T-002"),
        "expected NotFound, got {err:?}",
    );
    Ok(())
}

#[test]
fn workspace_lookup_skips_specs_with_failed_tasks_md_parse() -> TestResult {
    // Spec with valid TASKS.md
    let fx_valid = make_fixture("SPEC-0001", "- [ ] **T-001**: real\n  - Covers: REQ-001\n")?;

    // Spec with malformed TASKS.md (frontmatter missing).
    let dir = tempfile::tempdir()?;
    let dir_path = Utf8PathBuf::from_path_buf(dir.path().to_path_buf())
        .map_err(|p| format!("tempdir path must be UTF-8: {}", p.display()))?;
    let spec_md_path = dir_path.join("SPEC.md");
    fs_err::write(spec_md_path.as_std_path(), spec_md_for("SPEC-0002"))?;
    let tasks_md_path = dir_path.join("TASKS.md");
    fs_err::write(
        tasks_md_path.as_std_path(),
        "no frontmatter here, just plain text\n- [ ] **T-001**: ghost\n",
    )?;
    let fx_broken = Fixture {
        _dir: dir,
        spec_md_path,
        tasks_md_path: Some(tasks_md_path),
        dir_path,
    };

    let workspace = make_workspace(&[&fx_valid, &fx_broken]);
    let task_ref = parse_ref("T-001").expect("ref parses");
    // The broken spec's TASKS.md is silently skipped; the valid one wins.
    let location = find(&workspace, &task_ref).expect("valid spec resolves");
    assert_eq!(location.spec_id, "SPEC-0001");
    Ok(())
}

// -- REQ-003 -- CHK-003: ambiguity -------------------------------------------

#[test]
fn ambiguity_two_specs_returns_candidates_in_ascending_order() -> TestResult {
    let fx1 = make_fixture(
        "SPEC-0001",
        "- [ ] **T-001**: in spec-1\n  - Covers: REQ-001\n",
    )?;
    let fx2 = make_fixture(
        "SPEC-0002",
        "- [ ] **T-001**: in spec-2\n  - Covers: REQ-001\n",
    )?;
    let workspace = make_workspace(&[&fx1, &fx2]);

    let task_ref = parse_ref("T-001").expect("ref parses");
    let err = find(&workspace, &task_ref).expect_err("ambiguous must error");
    let LookupError::Ambiguous {
        task_id,
        candidate_specs,
    } = &err
    else {
        return Err(format!("expected Ambiguous, got {err:?}").into());
    };
    assert_eq!(task_id, "T-001");
    assert_eq!(
        candidate_specs,
        &vec!["SPEC-0001".to_owned(), "SPEC-0002".to_owned()],
    );
    Ok(())
}

#[test]
fn ambiguity_three_specs_returns_all_candidates() -> TestResult {
    let fx1 = make_fixture("SPEC-0001", "- [ ] **T-007**: a\n  - Covers: REQ-001\n")?;
    let fx2 = make_fixture("SPEC-0003", "- [ ] **T-007**: b\n  - Covers: REQ-001\n")?;
    let fx3 = make_fixture("SPEC-0005", "- [ ] **T-007**: c\n  - Covers: REQ-001\n")?;
    let workspace = make_workspace(&[&fx1, &fx2, &fx3]);

    let task_ref = parse_ref("T-007").expect("ref parses");
    let err = find(&workspace, &task_ref).expect_err("ambiguous must error");
    let LookupError::Ambiguous {
        task_id,
        candidate_specs,
    } = &err
    else {
        return Err(format!("expected Ambiguous, got {err:?}").into());
    };
    assert_eq!(task_id, "T-007");
    assert_eq!(
        candidate_specs,
        &vec![
            "SPEC-0001".to_owned(),
            "SPEC-0003".to_owned(),
            "SPEC-0005".to_owned(),
        ],
    );
    Ok(())
}

#[test]
fn ambiguity_bypassed_by_qualified_form() -> TestResult {
    let fx1 = make_fixture(
        "SPEC-0001",
        "- [ ] **T-001**: in spec-1\n  - Covers: REQ-001\n",
    )?;
    let fx2 = make_fixture(
        "SPEC-0002",
        "- [ ] **T-001**: in spec-2\n  - Covers: REQ-001\n",
    )?;
    let workspace = make_workspace(&[&fx1, &fx2]);

    let qualified = parse_ref("SPEC-0001/T-001").expect("qualified parses");
    let location = find(&workspace, &qualified).expect("qualified bypasses ambiguity");
    assert_eq!(location.spec_id, "SPEC-0001");
    Ok(())
}
