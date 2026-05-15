#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! Tests for `workspace::scan` covering discovery, ignored
//! subdirectories, per-spec parse failure, missing directories, and
//! ascending spec-ID ordering. Covers SPEC-0004 CHK-001 and CHK-002.

use camino::Utf8Path;
use camino::Utf8PathBuf;
use indoc::indoc;
use speccy_core::workspace::scan;
use tempfile::TempDir;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const VALID_SPEC_MD: &str = indoc! {r"
    ---
    id: SPEC-0001
    slug: example
    title: Example
    status: in-progress
    created: 2026-05-11
    ---

    # Example

    ### REQ-001: First
"};

const VALID_SPEC_TOML: &str = indoc! {r#"
    schema_version = 1

    [[requirements]]
    id = "REQ-001"
    checks = ["CHK-001"]

    [[checks]]
    id = "CHK-001"
    scenario = "covers REQ-001"
"#};

fn utf8(dir: &TempDir) -> TestResult<Utf8PathBuf> {
    Utf8PathBuf::from_path_buf(dir.path().to_path_buf())
        .map_err(|p| format!("tempdir path must be UTF-8: {}", p.display()).into())
}

fn write_spec(
    project_root: &Utf8Path,
    dir_name: &str,
    spec_md: &str,
    spec_toml: &str,
) -> TestResult<Utf8PathBuf> {
    let dir = project_root.join(".speccy").join("specs").join(dir_name);
    fs_err::create_dir_all(dir.as_std_path())?;
    fs_err::write(dir.join("SPEC.md").as_std_path(), spec_md)?;
    fs_err::write(dir.join("spec.toml").as_std_path(), spec_toml)?;
    Ok(dir)
}

#[test]
fn discovers_specs_and_ignores_non_matching_subdirs() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let root = utf8(&tmp)?;

    write_spec(&root, "0001-foo", VALID_SPEC_MD, VALID_SPEC_TOML)?;
    write_spec(
        &root,
        "0002-bar",
        &VALID_SPEC_MD.replace("SPEC-0001", "SPEC-0002"),
        VALID_SPEC_TOML,
    )?;

    // Non-matching subdirectories that must be ignored.
    fs_err::create_dir_all(
        root.join(".speccy")
            .join("specs")
            .join("_scratch")
            .as_std_path(),
    )?;
    fs_err::create_dir_all(
        root.join(".speccy")
            .join("specs")
            .join("notes")
            .as_std_path(),
    )?;

    let ws = scan(&root);
    assert_eq!(ws.specs.len(), 2);
    let ids: Vec<_> = ws.specs.iter().map(|s| s.spec_id.clone()).collect();
    assert_eq!(
        ids,
        vec![Some("SPEC-0001".to_owned()), Some("SPEC-0002".to_owned()),],
    );
    Ok(())
}

#[test]
fn parse_failure_on_one_spec_is_non_fatal() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let root = utf8(&tmp)?;

    // Malformed SPEC.md: missing frontmatter.
    write_spec(&root, "0001-broken", "# Just a heading\n", VALID_SPEC_TOML)?;
    // Well-formed second spec.
    write_spec(
        &root,
        "0002-good",
        &VALID_SPEC_MD.replace("SPEC-0001", "SPEC-0002"),
        VALID_SPEC_TOML,
    )?;

    let ws = scan(&root);
    assert_eq!(ws.specs.len(), 2);

    let first = ws.specs.first().expect("first spec");
    assert!(
        first.spec_md.is_err(),
        "first spec should carry parse error"
    );
    // The spec_id should fall back to the dir-derived form when the
    // frontmatter cannot be parsed.
    assert_eq!(first.spec_id.as_deref(), Some("SPEC-0001"));

    let second = ws.specs.get(1).expect("second spec");
    assert!(
        second.spec_md.is_ok(),
        "second spec must parse successfully"
    );
    assert_eq!(second.spec_id.as_deref(), Some("SPEC-0002"));
    Ok(())
}

#[test]
fn missing_specs_dir_yields_empty_workspace() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let root = utf8(&tmp)?;
    fs_err::create_dir_all(root.join(".speccy").as_std_path())?;
    // No specs/ subdir.

    let ws = scan(&root);
    assert!(ws.specs.is_empty(), "expected empty specs vec");
    Ok(())
}

#[test]
fn empty_specs_dir_yields_empty_workspace() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let root = utf8(&tmp)?;
    fs_err::create_dir_all(root.join(".speccy").join("specs").as_std_path())?;

    let ws = scan(&root);
    assert!(ws.specs.is_empty());
    Ok(())
}

#[test]
fn specs_are_returned_in_ascending_id_order() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let root = utf8(&tmp)?;

    // Write in non-ascending order.
    write_spec(
        &root,
        "0003-c",
        &VALID_SPEC_MD.replace("SPEC-0001", "SPEC-0003"),
        VALID_SPEC_TOML,
    )?;
    write_spec(&root, "0001-a", VALID_SPEC_MD, VALID_SPEC_TOML)?;
    write_spec(
        &root,
        "0002-b",
        &VALID_SPEC_MD.replace("SPEC-0001", "SPEC-0002"),
        VALID_SPEC_TOML,
    )?;

    let ws = scan(&root);
    let ids: Vec<_> = ws.specs.iter().filter_map(|s| s.spec_id.clone()).collect();
    assert_eq!(ids, vec!["SPEC-0001", "SPEC-0002", "SPEC-0003"]);
    Ok(())
}

#[test]
fn discovers_specs_inside_mission_focus_folders() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let root = utf8(&tmp)?;

    // Ungrouped flat spec.
    write_spec(&root, "0001-flat", VALID_SPEC_MD, VALID_SPEC_TOML)?;

    // Mission-grouped specs under `auth/`.
    let auth = root.join(".speccy").join("specs").join("auth");
    fs_err::create_dir_all(auth.join("0002-signup").as_std_path())?;
    fs_err::write(
        auth.join("0002-signup").join("SPEC.md").as_std_path(),
        VALID_SPEC_MD.replace("SPEC-0001", "SPEC-0002"),
    )?;
    fs_err::write(
        auth.join("0002-signup").join("spec.toml").as_std_path(),
        VALID_SPEC_TOML,
    )?;
    fs_err::create_dir_all(auth.join("0003-reset").as_std_path())?;
    fs_err::write(
        auth.join("0003-reset").join("SPEC.md").as_std_path(),
        VALID_SPEC_MD.replace("SPEC-0001", "SPEC-0003"),
    )?;
    fs_err::write(
        auth.join("0003-reset").join("spec.toml").as_std_path(),
        VALID_SPEC_TOML,
    )?;
    // Optional MISSION.md alongside the specs — not a spec dir, must
    // not be picked up.
    fs_err::write(auth.join("MISSION.md").as_std_path(), "# Mission: auth\n")?;

    let ws = scan(&root);
    let ids: Vec<_> = ws.specs.iter().filter_map(|s| s.spec_id.clone()).collect();
    assert_eq!(
        ids,
        vec!["SPEC-0001", "SPEC-0002", "SPEC-0003"],
        "mission-grouped specs under .speccy/specs/auth/ must be discovered alongside flat ones",
    );
    Ok(())
}

#[test]
fn tasks_md_optional() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let root = utf8(&tmp)?;
    let dir = write_spec(&root, "0001-foo", VALID_SPEC_MD, VALID_SPEC_TOML)?;
    // No TASKS.md written.

    let ws = scan(&root);
    let only = ws.specs.first().expect("one spec");
    assert!(only.tasks_md.is_none(), "TASKS.md absent should yield None");
    assert!(only.tasks_md_path.is_none());
    assert_eq!(only.dir, dir);
    Ok(())
}
