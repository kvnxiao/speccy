#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! Tests for `workspace::scan`'s supersession integration. Covers
//! SPEC-0004 CHK-005.

use camino::Utf8Path;
use camino::Utf8PathBuf;
use indoc::indoc;
use speccy_core::workspace::scan;
use tempfile::TempDir;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const SPEC_TOML: &str = indoc! {r#"
    schema_version = 1

    [[requirements]]
    id = "REQ-001"
    checks = ["CHK-001"]

    [[checks]]
    id = "CHK-001"
    kind = "test"
    command = "cargo test"
    proves = "covers REQ-001"
"#};

fn utf8(dir: &TempDir) -> TestResult<Utf8PathBuf> {
    Utf8PathBuf::from_path_buf(dir.path().to_path_buf())
        .map_err(|p| format!("tempdir path must be UTF-8: {}", p.display()).into())
}

fn write_spec_with_frontmatter(
    project_root: &Utf8Path,
    dir_name: &str,
    id: &str,
    supersedes: &[&str],
) -> TestResult<()> {
    let dir = project_root.join(".speccy").join("specs").join(dir_name);
    fs_err::create_dir_all(dir.as_std_path())?;
    let supersedes_yaml = if supersedes.is_empty() {
        String::from("[]")
    } else {
        let inner = supersedes
            .iter()
            .map(|s| format!("\"{s}\""))
            .collect::<Vec<_>>()
            .join(", ");
        format!("[{inner}]")
    };
    let spec_md = format!(
        "---\nid: {id}\nslug: x\ntitle: y\nstatus: in-progress\ncreated: 2026-05-11\nsupersedes: {supersedes_yaml}\n---\n\n# {id}\n\n### REQ-001: First\n",
    );
    fs_err::write(dir.join("SPEC.md").as_std_path(), spec_md)?;
    fs_err::write(dir.join("spec.toml").as_std_path(), SPEC_TOML)?;
    Ok(())
}

#[test]
fn supersession_inverse_is_computed() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let root = utf8(&tmp)?;

    write_spec_with_frontmatter(&root, "0017-original", "SPEC-0017", &[])?;
    write_spec_with_frontmatter(&root, "0042-replacement", "SPEC-0042", &["SPEC-0017"])?;

    let ws = scan(&root);
    let by_target = ws.supersession.superseded_by("SPEC-0017");
    assert_eq!(by_target, &["SPEC-0042".to_owned()]);
    assert!(ws.supersession.superseded_by("SPEC-0042").is_empty());
    Ok(())
}

#[test]
fn dangling_supersedes_reference_is_exposed() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let root = utf8(&tmp)?;

    write_spec_with_frontmatter(&root, "0001-only", "SPEC-0001", &["SPEC-9999"])?;

    let ws = scan(&root);
    let dangling = ws.supersession.dangling_references();
    assert_eq!(dangling, &["SPEC-9999".to_owned()]);
    Ok(())
}

#[test]
fn parse_failures_excluded_from_supersession_input() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let root = utf8(&tmp)?;

    // Spec 0001 is malformed (no frontmatter); spec 0002 declares
    // supersedes: [SPEC-0001]. Since 0001 can't be parsed, the
    // supersession index would see SPEC-0001 as dangling (not in input).
    let dir1 = root.join(".speccy").join("specs").join("0001-broken");
    fs_err::create_dir_all(dir1.as_std_path())?;
    fs_err::write(dir1.join("SPEC.md").as_std_path(), "# malformed\n")?;
    fs_err::write(dir1.join("spec.toml").as_std_path(), SPEC_TOML)?;

    write_spec_with_frontmatter(&root, "0002-newer", "SPEC-0002", &["SPEC-0001"])?;

    let ws = scan(&root);
    // SPEC-0001 should appear as dangling because we couldn't parse it.
    let dangling = ws.supersession.dangling_references();
    assert!(
        dangling.contains(&"SPEC-0001".to_owned()),
        "expected SPEC-0001 in dangling, got: {dangling:?}",
    );
    Ok(())
}
