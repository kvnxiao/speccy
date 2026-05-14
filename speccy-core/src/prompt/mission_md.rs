//! Nearest-parent `MISSION.md` walker.
//!
//! Mission folders (`specs/[focus]/`) optionally carry a `MISSION.md`
//! describing the parent-context shared by a group of specs. Phase
//! commands that operate on one spec (plan amend, implement, review,
//! report) walk upward from the spec directory looking for the nearest
//! enclosing `MISSION.md` and inline it into the rendered prompt.
//!
//! A missing MISSION.md is not an error: most specs in a flat
//! single-focus project will not have one. The function returns a
//! marker string the rendered prompt surfaces to the agent so the
//! absence is legible.
//!
//! See SPEC-0005 REQ-007.

use camino::Utf8Path;
use std::io::Write;

const MARKER_UNGROUPED: &str = "<!-- no parent MISSION.md; spec is ungrouped -->";

/// Walk upward from `spec_dir` toward `specs_root` looking for the
/// nearest enclosing `MISSION.md`. Warnings go to stderr.
///
/// `specs_root` is the boundary: walking stops once it is checked
/// (inclusive), regardless of whether `MISSION.md` was found.
#[must_use = "the loaded content (or marker) is inlined into the rendered prompt"]
pub fn find_nearest_mission_md(spec_dir: &Utf8Path, specs_root: &Utf8Path) -> String {
    let stderr = std::io::stderr();
    let mut lock = stderr.lock();
    find_nearest_mission_md_with_warn(spec_dir, specs_root, &mut lock)
}

/// Same as [`find_nearest_mission_md`] but routes warnings to an
/// injected sink (used in tests).
#[must_use = "the loaded content (or marker) is inlined into the rendered prompt"]
pub fn find_nearest_mission_md_with_warn<W: Write>(
    spec_dir: &Utf8Path,
    specs_root: &Utf8Path,
    warn_out: &mut W,
) -> String {
    let Some(mut cursor) = spec_dir.parent() else {
        return MARKER_UNGROUPED.to_owned();
    };
    loop {
        let candidate = cursor.join("MISSION.md");
        match fs_err::read_to_string(candidate.as_std_path()) {
            Ok(content) => return content,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                // Fall through to ascend.
            }
            Err(err) => {
                if writeln!(
                    warn_out,
                    "speccy prompt: MISSION.md at {candidate} could not be read: {err}",
                )
                .is_err()
                {
                    // Warning sink is closed; nothing actionable.
                }
                return format!("<!-- MISSION.md unreadable at {candidate}: {err} -->");
            }
        }
        if cursor == specs_root {
            break;
        }
        let Some(next) = cursor.parent() else { break };
        cursor = next;
    }
    MARKER_UNGROUPED.to_owned()
}

#[cfg(test)]
mod tests {
    use super::find_nearest_mission_md_with_warn;
    use camino::Utf8PathBuf;
    use tempfile::TempDir;

    fn make_tmp_root() -> (TempDir, Utf8PathBuf) {
        let dir = tempfile::tempdir().expect("tmpdir creation must succeed");
        let path = Utf8PathBuf::from_path_buf(dir.path().to_path_buf())
            .expect("tempdir path must be UTF-8");
        (dir, path)
    }

    #[test]
    fn returns_ungrouped_marker_when_no_mission_md_present() {
        let (_tmp, root) = make_tmp_root();
        let specs_root = root.join("specs");
        let spec_dir = specs_root.join("0001-foo");
        fs_err::create_dir_all(spec_dir.as_std_path()).expect("mkdir spec_dir");
        let mut warns = Vec::new();
        let out = find_nearest_mission_md_with_warn(&spec_dir, &specs_root, &mut warns);
        assert!(
            out.contains("no parent MISSION.md"),
            "expected ungrouped marker, got: {out}",
        );
        assert!(
            warns.is_empty(),
            "no warning expected when MISSION.md is simply absent, got: {warns:?}",
        );
    }

    #[test]
    fn returns_parent_mission_md_content_when_present() {
        let (_tmp, root) = make_tmp_root();
        let specs_root = root.join("specs");
        let mission_folder = specs_root.join("auth");
        let spec_dir = mission_folder.join("0042-signup");
        fs_err::create_dir_all(spec_dir.as_std_path()).expect("mkdir spec_dir");
        fs_err::write(
            mission_folder.join("MISSION.md").as_std_path(),
            "# Mission: auth\n\n## Scope\nAuthentication for the app.\n",
        )
        .expect("write MISSION.md");

        let mut warns = Vec::new();
        let out = find_nearest_mission_md_with_warn(&spec_dir, &specs_root, &mut warns);
        assert!(out.contains("# Mission: auth"));
        assert!(out.contains("Authentication for the app."));
        assert!(warns.is_empty());
    }

    #[test]
    fn walks_only_up_to_specs_root_not_beyond() {
        let (_tmp, root) = make_tmp_root();
        let specs_root = root.join("specs");
        let spec_dir = specs_root.join("0001-foo");
        fs_err::create_dir_all(spec_dir.as_std_path()).expect("mkdir");
        // A MISSION.md *outside* specs_root must not be picked up.
        fs_err::write(
            root.join("MISSION.md").as_std_path(),
            "should-not-be-loaded",
        )
        .expect("write outside-root MISSION.md");

        let mut warns = Vec::new();
        let out = find_nearest_mission_md_with_warn(&spec_dir, &specs_root, &mut warns);
        assert!(
            out.contains("no parent MISSION.md"),
            "MISSION.md outside specs_root must be ignored; got: {out}",
        );
    }

    #[test]
    fn returns_nearest_mission_md_when_multiple_ancestors_have_one() {
        // Deeply nested: specs/auth/v2/0099-rotate/. Both `auth/` and
        // `auth/v2/` carry MISSION.md; the deeper one wins.
        let (_tmp, root) = make_tmp_root();
        let specs_root = root.join("specs");
        let auth = specs_root.join("auth");
        let auth_v2 = auth.join("v2");
        let spec_dir = auth_v2.join("0099-rotate");
        fs_err::create_dir_all(spec_dir.as_std_path()).expect("mkdir");
        fs_err::write(auth.join("MISSION.md").as_std_path(), "outer-mission").expect("write outer");
        fs_err::write(auth_v2.join("MISSION.md").as_std_path(), "inner-mission")
            .expect("write inner");

        let mut warns = Vec::new();
        let out = find_nearest_mission_md_with_warn(&spec_dir, &specs_root, &mut warns);
        assert_eq!(
            out, "inner-mission",
            "the closest enclosing MISSION.md must win, not the outermost",
        );
    }
}
