#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! Tests for `workspace::find_root`.

use camino::Utf8PathBuf;
use speccy_core::workspace::WorkspaceError;
use speccy_core::workspace::find_root;
use tempfile::TempDir;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn utf8_dir(dir: &TempDir) -> TestResult<Utf8PathBuf> {
    Utf8PathBuf::from_path_buf(dir.path().to_path_buf())
        .map_err(|p| format!("tempdir path must be UTF-8: {}", p.display()).into())
}

#[test]
fn finds_root_when_speccy_dir_exists() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let root = utf8_dir(&tmp)?;
    fs_err::create_dir_all(root.join(".speccy").as_std_path())?;

    let discovered = find_root(&root)?;
    assert_eq!(discovered, root);
    Ok(())
}

#[test]
fn walks_up_from_nested_directory() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let root = utf8_dir(&tmp)?;
    fs_err::create_dir_all(root.join(".speccy").as_std_path())?;

    let nested = root.join("a").join("b").join("c");
    fs_err::create_dir_all(nested.as_std_path())?;

    let discovered = find_root(&nested)?;
    assert_eq!(discovered, root);
    Ok(())
}

#[test]
fn returns_no_speccy_dir_when_no_workspace() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let root = utf8_dir(&tmp)?;
    let err = find_root(&root).expect_err("find_root must fail outside a workspace");
    assert!(
        matches!(err, WorkspaceError::NoSpeccyDir { .. }),
        "expected NoSpeccyDir, got: {err:?}",
    );
    Ok(())
}

#[test]
fn ignores_regular_file_named_speccy() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let root = utf8_dir(&tmp)?;
    // A regular file named `.speccy` (not a directory) should not be
    // treated as a project root.
    fs_err::write(root.join(".speccy").as_std_path(), "not a directory")?;

    let err = find_root(&root).expect_err("regular file `.speccy` should not satisfy the search");
    assert!(
        matches!(err, WorkspaceError::NoSpeccyDir { .. }),
        "expected NoSpeccyDir, got: {err:?}",
    );
    Ok(())
}
