#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
//! Integration tests for `speccy_core::prompt::allocate_next_spec_id`.
//! Covers SPEC-0005 REQ-003 via the public API.

use camino::Utf8PathBuf;
use speccy_core::prompt::allocate_next_spec_id;
use tempfile::TempDir;

fn make_tmp_root() -> (TempDir, Utf8PathBuf) {
    let dir = tempfile::tempdir().expect("tmpdir creation must succeed");
    let path =
        Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).expect("tempdir path must be UTF-8");
    (dir, path)
}

fn mkdir(root: &Utf8PathBuf, name: &str) {
    fs_err::create_dir_all(root.join(name).as_std_path()).expect("mkdir must succeed");
}

#[test]
fn empty_specs_dir_returns_0001() {
    let (_tmp, root) = make_tmp_root();
    let specs = root.join("specs");
    fs_err::create_dir_all(specs.as_std_path()).expect("mkdir specs");
    assert_eq!(allocate_next_spec_id(&specs), "0001");
}

#[test]
fn absent_specs_dir_returns_0001() {
    let (_tmp, root) = make_tmp_root();
    let specs = root.join("does-not-exist");
    assert_eq!(allocate_next_spec_id(&specs), "0001");
}

#[test]
fn gaps_are_not_recycled() {
    let (_tmp, root) = make_tmp_root();
    let specs = root.join("specs");
    fs_err::create_dir_all(specs.as_std_path()).expect("mkdir");
    mkdir(&specs, "0001-foo");
    mkdir(&specs, "0003-bar");
    assert_eq!(allocate_next_spec_id(&specs), "0004");
}

#[test]
fn high_id_returns_max_plus_one() {
    let (_tmp, root) = make_tmp_root();
    let specs = root.join("specs");
    fs_err::create_dir_all(specs.as_std_path()).expect("mkdir");
    mkdir(&specs, "0042-foo");
    assert_eq!(allocate_next_spec_id(&specs), "0043");
}

#[test]
fn non_matching_directories_are_ignored() {
    let (_tmp, root) = make_tmp_root();
    let specs = root.join("specs");
    fs_err::create_dir_all(specs.as_std_path()).expect("mkdir");
    mkdir(&specs, "0001-foo");
    mkdir(&specs, "_scratch");
    assert_eq!(allocate_next_spec_id(&specs), "0002");
}
