#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! Priority logic tests for `speccy_core::next::compute`. Covers
//! SPEC-0007 CHK-001 through CHK-006.

use camino::Utf8Path;
use camino::Utf8PathBuf;
use indoc::indoc;
use speccy_core::next::BlockedReason;
use speccy_core::next::KindFilter;
use speccy_core::next::NextResult;
use speccy_core::next::compute;
use speccy_core::workspace::scan;
use std::fmt::Write as _;
use tempfile::TempDir;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn utf8(dir: &TempDir) -> TestResult<Utf8PathBuf> {
    Utf8PathBuf::from_path_buf(dir.path().to_path_buf())
        .map_err(|p| format!("tempdir path must be UTF-8: {}", p.display()).into())
}

fn valid_spec_md(id: &str) -> String {
    let template = indoc! {r"
        ---
        id: __ID__
        slug: x
        title: Example __ID__
        status: in-progress
        created: 2026-05-11
        ---

        # __ID__

        ### REQ-001: First
    "};
    template.replace("__ID__", id)
}

fn valid_spec_toml() -> &'static str {
    indoc! {r#"
        schema_version = 1

        [[requirements]]
        id = "REQ-001"
        checks = ["CHK-001"]

        [[checks]]
        id = "CHK-001"
        scenario = "covers REQ-001"
    "#}
}

/// Build a TASKS.md with the given list of `(state, id, title)` rows.
fn tasks_md(spec_id: &str, rows: &[(char, &str, &str)]) -> String {
    let mut body = String::new();
    write!(
        body,
        "---\nspec: {spec_id}\nspec_hash_at_generation: bootstrap-pending\ngenerated_at: 2026-05-11T00:00:00Z\n---\n\n# Tasks\n\n## Phase\n",
    )
    .expect("writes to String are infallible");
    for (state, id, title) in rows {
        let glyph = glyph_for(*state);
        writeln!(body, "- {glyph} **{id}**: {title}").expect("writes to String are infallible");
        writeln!(body, "  - Covers: REQ-001").expect("writes to String are infallible");
    }
    body
}

fn glyph_for(state: char) -> &'static str {
    match state {
        '~' => "[~]",
        '?' => "[?]",
        'x' => "[x]",
        _ => "[ ]",
    }
}

fn write_spec(
    project_root: &Utf8Path,
    dir_name: &str,
    spec_id: &str,
    tasks_rows: Option<&[(char, &str, &str)]>,
) -> TestResult<Utf8PathBuf> {
    let dir = project_root.join(".speccy").join("specs").join(dir_name);
    fs_err::create_dir_all(dir.as_std_path())?;
    fs_err::write(dir.join("SPEC.md").as_std_path(), valid_spec_md(spec_id))?;
    fs_err::write(dir.join("spec.toml").as_std_path(), valid_spec_toml())?;
    if let Some(rows) = tasks_rows {
        fs_err::write(dir.join("TASKS.md").as_std_path(), tasks_md(spec_id, rows))?;
    }
    Ok(dir)
}

// -- CHK-001 ----------------------------------------------------------------

#[test]
fn default_within_spec_preference() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let root = utf8(&tmp)?;
    write_spec(
        &root,
        "0001-foo",
        "SPEC-0001",
        Some(&[
            (' ', "T-001", "open one"),
            ('?', "T-002", "awaiting review"),
            (' ', "T-003", "open two"),
        ]),
    )?;
    let ws = scan(&root);
    let result = compute(&ws, None);
    assert!(
        matches!(
            &result,
            NextResult::Review { spec, task, .. }
                if spec == "SPEC-0001" && task == "T-002",
        ),
        "expected review T-002 to beat opens, got {result:?}",
    );

    // [~] tasks should never win over an [ ] task even within the same spec.
    let tmp2 = tempfile::tempdir()?;
    let root2 = utf8(&tmp2)?;
    write_spec(
        &root2,
        "0001-foo",
        "SPEC-0001",
        Some(&[('~', "T-001", "claimed"), (' ', "T-002", "open")]),
    )?;
    let ws2 = scan(&root2);
    let r2 = compute(&ws2, None);
    assert!(
        matches!(&r2, NextResult::Implement { task, .. } if task == "T-002"),
        "[~] task must be skipped, got {r2:?}",
    );
    Ok(())
}

// -- CHK-002 ----------------------------------------------------------------

#[test]
fn lowest_spec_id_wins() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let root = utf8(&tmp)?;
    write_spec(
        &root,
        "0001-foo",
        "SPEC-0001",
        Some(&[(' ', "T-001", "open in SPEC-0001")]),
    )?;
    write_spec(
        &root,
        "0002-bar",
        "SPEC-0002",
        Some(&[('?', "T-002", "review in SPEC-0002")]),
    )?;
    let ws = scan(&root);
    let result = compute(&ws, None);
    assert!(
        matches!(
            &result,
            NextResult::Implement { spec, task, .. }
                if spec == "SPEC-0001" && task == "T-001",
        ),
        "lowest spec ID must win across specs, got {result:?}",
    );

    // SPEC-0001 has only [~]; SPEC-0002 has an [ ] task; the implement
    // should come from SPEC-0002.
    let tmp2 = tempfile::tempdir()?;
    let root2 = utf8(&tmp2)?;
    write_spec(
        &root2,
        "0001-foo",
        "SPEC-0001",
        Some(&[('~', "T-001", "claimed")]),
    )?;
    write_spec(
        &root2,
        "0002-bar",
        "SPEC-0002",
        Some(&[(' ', "T-002", "open")]),
    )?;
    let ws2 = scan(&root2);
    let r2 = compute(&ws2, None);
    assert!(
        matches!(
            &r2,
            NextResult::Implement { spec, task, .. }
                if spec == "SPEC-0002" && task == "T-002",
        ),
        "should fall through to SPEC-0002 when SPEC-0001 has no actionable work, got {r2:?}",
    );
    Ok(())
}

// -- CHK-003 ----------------------------------------------------------------

#[test]
fn kind_implement_filter() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let root = utf8(&tmp)?;
    write_spec(
        &root,
        "0001-foo",
        "SPEC-0001",
        Some(&[('?', "T-001", "review-ready")]),
    )?;
    let ws = scan(&root);
    let result = compute(&ws, Some(KindFilter::Implement));
    assert!(
        matches!(&result, NextResult::Blocked { reason } if reason == BlockedReason::NO_OPEN_TASKS),
        "--kind implement with only [?] must return Blocked(NO_OPEN_TASKS), got {result:?}",
    );

    // Mixed: [ ] in SPEC-0002 wins despite SPEC-0001 having [?].
    let tmp2 = tempfile::tempdir()?;
    let root2 = utf8(&tmp2)?;
    write_spec(
        &root2,
        "0001-foo",
        "SPEC-0001",
        Some(&[('?', "T-001", "review")]),
    )?;
    write_spec(
        &root2,
        "0002-bar",
        "SPEC-0002",
        Some(&[(' ', "T-002", "open")]),
    )?;
    let ws2 = scan(&root2);
    let r2 = compute(&ws2, Some(KindFilter::Implement));
    assert!(
        matches!(
            &r2,
            NextResult::Implement { spec, task, .. }
                if spec == "SPEC-0002" && task == "T-002",
        ),
        "--kind implement must skip [?] and find the next [ ], got {r2:?}",
    );
    Ok(())
}

// -- CHK-004 ----------------------------------------------------------------

#[test]
fn kind_review_filter() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let root = utf8(&tmp)?;
    write_spec(
        &root,
        "0001-foo",
        "SPEC-0001",
        Some(&[(' ', "T-001", "open")]),
    )?;
    write_spec(
        &root,
        "0002-bar",
        "SPEC-0002",
        Some(&[('?', "T-002", "review-ready")]),
    )?;
    let ws = scan(&root);
    let result = compute(&ws, Some(KindFilter::Review));
    assert!(
        matches!(
            &result,
            NextResult::Review { spec, task, personas, .. }
                if spec == "SPEC-0002"
                && task == "T-002"
                && *personas == ["business", "tests", "security", "style"],
        ),
        "--kind review must return [?] task with the default fan-out, got {result:?}",
    );

    // No [?] anywhere -> Blocked(NO_REVIEWS_PENDING).
    let tmp2 = tempfile::tempdir()?;
    let root2 = utf8(&tmp2)?;
    write_spec(
        &root2,
        "0001-foo",
        "SPEC-0001",
        Some(&[(' ', "T-001", "open")]),
    )?;
    let ws2 = scan(&root2);
    let r2 = compute(&ws2, Some(KindFilter::Review));
    assert!(
        matches!(
            &r2,
            NextResult::Blocked { reason }
                if reason == BlockedReason::NO_REVIEWS_PENDING,
        ),
        "--kind review with no [?] must return Blocked(NO_REVIEWS_PENDING), got {r2:?}",
    );
    Ok(())
}

// -- CHK-005 ----------------------------------------------------------------

#[test]
fn report_kind() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let root = utf8(&tmp)?;
    write_spec(
        &root,
        "0001-foo",
        "SPEC-0001",
        Some(&[('x', "T-001", "done")]),
    )?;
    let ws = scan(&root);
    let result = compute(&ws, None);
    assert!(
        matches!(&result, NextResult::Report { spec } if spec == "SPEC-0001"),
        "all-[x] spec without REPORT.md must return Report, got {result:?}",
    );

    // Two all-[x] specs; SPEC-0001 has REPORT.md, SPEC-0002 does not.
    let tmp2 = tempfile::tempdir()?;
    let root2 = utf8(&tmp2)?;
    let dir1 = write_spec(
        &root2,
        "0001-foo",
        "SPEC-0001",
        Some(&[('x', "T-001", "done")]),
    )?;
    fs_err::write(
        dir1.join("REPORT.md").as_std_path(),
        "---\nspec: SPEC-0001\n---\n",
    )?;
    write_spec(
        &root2,
        "0002-bar",
        "SPEC-0002",
        Some(&[('x', "T-002", "done")]),
    )?;
    let ws2 = scan(&root2);
    let r2 = compute(&ws2, None);
    assert!(
        matches!(&r2, NextResult::Report { spec } if spec == "SPEC-0002"),
        "second-lowest unreported spec must be returned, got {r2:?}",
    );

    // All done + all REPORT.md present -> falls through to blocked.
    let dir2 = root2.join(".speccy").join("specs").join("0002-bar");
    fs_err::write(
        dir2.join("REPORT.md").as_std_path(),
        "---\nspec: SPEC-0002\n---\n",
    )?;
    let ws3 = scan(&root2);
    let r3 = compute(&ws3, None);
    assert!(
        matches!(&r3, NextResult::Blocked { reason } if reason == BlockedReason::ALL_DONE),
        "all reported should fall through to Blocked(ALL_DONE), got {r3:?}",
    );
    Ok(())
}

// -- CHK-006 ----------------------------------------------------------------

#[test]
fn blocked_kind_reasons() -> TestResult {
    // Empty workspace.
    let tmp = tempfile::tempdir()?;
    let root = utf8(&tmp)?;
    fs_err::create_dir_all(root.join(".speccy").as_std_path())?;
    let ws = scan(&root);
    let r = compute(&ws, None);
    assert!(
        matches!(&r, NextResult::Blocked { reason } if reason == BlockedReason::NO_SPECS),
        "empty workspace must yield NO_SPECS, got {r:?}",
    );

    // All [~] (claimed).
    let tmp2 = tempfile::tempdir()?;
    let root2 = utf8(&tmp2)?;
    write_spec(
        &root2,
        "0001-foo",
        "SPEC-0001",
        Some(&[('~', "T-001", "claimed")]),
    )?;
    let ws2 = scan(&root2);
    let r2 = compute(&ws2, None);
    assert!(
        matches!(&r2, NextResult::Blocked { reason } if reason == BlockedReason::ALL_CLAIMED),
        "all-claimed default-kind must yield ALL_CLAIMED, got {r2:?}",
    );

    // --kind implement with only [?] tasks.
    let tmp3 = tempfile::tempdir()?;
    let root3 = utf8(&tmp3)?;
    write_spec(
        &root3,
        "0001-foo",
        "SPEC-0001",
        Some(&[('?', "T-001", "review")]),
    )?;
    let ws3 = scan(&root3);
    let r3 = compute(&ws3, Some(KindFilter::Implement));
    assert!(
        matches!(&r3, NextResult::Blocked { reason } if reason == BlockedReason::NO_OPEN_TASKS),
        "--kind implement with only [?] must yield NO_OPEN_TASKS, got {r3:?}",
    );

    // --kind review with only [ ] tasks.
    let tmp4 = tempfile::tempdir()?;
    let root4 = utf8(&tmp4)?;
    write_spec(
        &root4,
        "0001-foo",
        "SPEC-0001",
        Some(&[(' ', "T-001", "open")]),
    )?;
    let ws4 = scan(&root4);
    let r4 = compute(&ws4, Some(KindFilter::Review));
    assert!(
        matches!(
            &r4,
            NextResult::Blocked { reason } if reason == BlockedReason::NO_REVIEWS_PENDING,
        ),
        "--kind review with only [ ] must yield NO_REVIEWS_PENDING, got {r4:?}",
    );

    // --kind implement with all [~]: ALL_CLAIMED, not NO_OPEN_TASKS.
    let tmp5 = tempfile::tempdir()?;
    let root5 = utf8(&tmp5)?;
    write_spec(
        &root5,
        "0001-foo",
        "SPEC-0001",
        Some(&[('~', "T-001", "claimed")]),
    )?;
    let ws5 = scan(&root5);
    let r5 = compute(&ws5, Some(KindFilter::Implement));
    assert!(
        matches!(&r5, NextResult::Blocked { reason } if reason == BlockedReason::ALL_CLAIMED),
        "--kind implement with only [~] must yield ALL_CLAIMED, got {r5:?}",
    );
    Ok(())
}
