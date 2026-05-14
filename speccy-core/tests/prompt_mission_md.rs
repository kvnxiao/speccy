#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
//! Integration tests for `speccy_core::prompt::find_nearest_mission_md`.
//! Covers SPEC-0005 REQ-007 via the public API.

use camino::Utf8PathBuf;
use speccy_core::prompt::find_nearest_mission_md;
use tempfile::TempDir;

fn make_tmp_root() -> (TempDir, Utf8PathBuf) {
    let dir = tempfile::tempdir().expect("tmpdir creation must succeed");
    let path =
        Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).expect("tempdir path must be UTF-8");
    (dir, path)
}

#[test]
fn flat_spec_returns_ungrouped_marker() {
    let (_tmp, root) = make_tmp_root();
    let specs = root.join("specs");
    let spec_dir = specs.join("0001-foo");
    fs_err::create_dir_all(spec_dir.as_std_path()).expect("mkdir");

    let out = find_nearest_mission_md(&spec_dir, &specs);
    assert!(
        out.contains("no parent MISSION.md"),
        "expected ungrouped marker for flat spec, got: {out}",
    );
}

#[test]
fn mission_grouped_spec_inlines_parent_mission_md() {
    let (_tmp, root) = make_tmp_root();
    let specs = root.join("specs");
    let mission_folder = specs.join("auth");
    let spec_dir = mission_folder.join("0042-signup");
    fs_err::create_dir_all(spec_dir.as_std_path()).expect("mkdir spec_dir");
    fs_err::write(
        mission_folder.join("MISSION.md").as_std_path(),
        "# Mission: auth\n",
    )
    .expect("write MISSION.md");

    let out = find_nearest_mission_md(&spec_dir, &specs);
    assert!(out.contains("# Mission: auth"), "got: {out}");
}

#[test]
fn mission_grouped_spec_without_mission_md_in_folder_returns_ungrouped() {
    // Someone created a focus folder + spec directory but never wrote
    // a `MISSION.md` inside the focus folder. The walker should still
    // return the ungrouped marker rather than failing or treating the
    // focus folder as a mission.
    let (_tmp, root) = make_tmp_root();
    let specs = root.join("specs");
    let mission_folder = specs.join("auth");
    let spec_dir = mission_folder.join("0042-signup");
    fs_err::create_dir_all(spec_dir.as_std_path()).expect("mkdir spec_dir");
    // Note: no MISSION.md anywhere — neither in `auth/` nor in `specs/`.

    let out = find_nearest_mission_md(&spec_dir, &specs);
    assert!(
        out.contains("no parent MISSION.md"),
        "mission folder without MISSION.md must still yield the ungrouped marker; got: {out}",
    );
}

#[test]
fn walking_stops_at_specs_root() {
    let (_tmp, root) = make_tmp_root();
    let specs = root.join("specs");
    let spec_dir = specs.join("0001-foo");
    fs_err::create_dir_all(spec_dir.as_std_path()).expect("mkdir");
    // A MISSION.md *outside* specs_root must not be loaded.
    fs_err::write(root.join("MISSION.md").as_std_path(), "must-not-be-loaded")
        .expect("write outside-root MISSION.md");

    let out = find_nearest_mission_md(&spec_dir, &specs);
    assert!(
        out.contains("no parent MISSION.md"),
        "MISSION.md outside specs_root must be ignored; got: {out}",
    );
}
