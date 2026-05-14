//! Spec ID allocator (`max + 1`; no gap recycling).
//!
//! Per SPEC-0005 DEC-005 and `.speccy/ARCHITECTURE.md` "Spec ID allocation":
//! gaps left by dropped specs are not recycled so historical SPEC IDs
//! retain unambiguous meaning in commits and PR descriptions.
//!
//! Walks `specs_dir` recursively so flat specs (`specs/NNNN-slug/`) and
//! mission-grouped specs (`specs/[focus]/NNNN-slug/`) share one ID
//! space. Mission folders themselves are descended into but contribute
//! no numeric prefix.

use camino::Utf8Path;
use regex::Regex;
use std::sync::OnceLock;

/// Walk `specs_dir` recursively, find every directory whose name
/// matches `^(\d{4,})-`, and return the next available numeric prefix
/// as a zero-padded 4+ digit string (`"0001"`, `"0014"`, `"10000"` ...).
///
/// Directories whose names do **not** match the `NNNN-slug` pattern
/// (e.g. `auth/`, `billing/`) are treated as mission folders: descended
/// into but not counted. Names that look like a malformed numeric
/// prefix (`_scratch`, `00ab-foo`) are ignored entirely.
///
/// Returns `"0001"` when `specs_dir` is absent, empty, or contains no
/// matching directories anywhere in its tree.
#[must_use = "the allocated ID identifies the next spec to be created"]
pub fn allocate_next_spec_id(specs_dir: &Utf8Path) -> String {
    let max = scan_max_prefix(specs_dir);
    let next = max.map_or(1_u64, |m| m.saturating_add(1));
    format!("{next:04}")
}

/// Recursive helper: returns the maximum numeric prefix found anywhere
/// under `dir`. Stops descending into a subdirectory once it matches the
/// `NNNN-slug` pattern (specs are leaves; nothing meaningful lives
/// inside them for ID allocation).
fn scan_max_prefix(dir: &Utf8Path) -> Option<u64> {
    let entries = fs_err::read_dir(dir.as_std_path()).ok()?;
    let pattern = dir_prefix_regex();
    let mut max: Option<u64> = None;
    for entry in entries.flatten() {
        let Ok(meta) = entry.metadata() else { continue };
        if !meta.is_dir() {
            continue;
        }
        let Ok(child) = camino::Utf8PathBuf::from_path_buf(entry.path()) else {
            continue;
        };
        let Some(name) = child.file_name() else {
            continue;
        };
        if let Some(caps) = pattern.captures(name)
            && let Some(digits) = caps.get(1)
            && let Ok(n) = digits.as_str().parse::<u64>()
        {
            max = Some(max.map_or(n, |m| m.max(n)));
            continue;
        }
        if let Some(child_max) = scan_max_prefix(&child) {
            max = Some(max.map_or(child_max, |m| m.max(child_max)));
        }
    }
    max
}

#[expect(
    clippy::unwrap_used,
    reason = "compile-time literal regex; covered by unit tests"
)]
fn dir_prefix_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"^(\d{4,})-").unwrap())
}

#[cfg(test)]
mod tests {
    use super::allocate_next_spec_id;
    use camino::Utf8PathBuf;
    use tempfile::TempDir;

    fn make_tmp_root() -> (TempDir, Utf8PathBuf) {
        let dir = tempfile::tempdir().expect("tmpdir creation must succeed");
        let path = Utf8PathBuf::from_path_buf(dir.path().to_path_buf())
            .expect("tempdir path must be UTF-8");
        (dir, path)
    }

    fn mkdir(root: &Utf8PathBuf, name: &str) {
        fs_err::create_dir_all(root.join(name).as_std_path()).expect("mkdir must succeed");
    }

    #[test]
    fn absent_specs_directory_returns_0001() {
        let (_tmp, root) = make_tmp_root();
        let specs = root.join("does-not-exist");
        assert_eq!(allocate_next_spec_id(&specs), "0001");
    }

    #[test]
    fn empty_specs_directory_returns_0001() {
        let (_tmp, root) = make_tmp_root();
        let specs = root.join("specs");
        fs_err::create_dir_all(specs.as_std_path()).expect("mkdir");
        assert_eq!(allocate_next_spec_id(&specs), "0001");
    }

    #[test]
    fn gap_is_not_recycled() {
        let (_tmp, root) = make_tmp_root();
        let specs = root.join("specs");
        fs_err::create_dir_all(specs.as_std_path()).expect("mkdir");
        mkdir(&specs, "0001-foo");
        mkdir(&specs, "0003-bar");
        assert_eq!(allocate_next_spec_id(&specs), "0004");
    }

    #[test]
    fn high_id_is_max_plus_one() {
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
        mkdir(&specs, "README");
        assert_eq!(allocate_next_spec_id(&specs), "0002");
    }

    #[test]
    fn malformed_numeric_prefix_is_ignored() {
        let (_tmp, root) = make_tmp_root();
        let specs = root.join("specs");
        fs_err::create_dir_all(specs.as_std_path()).expect("mkdir");
        mkdir(&specs, "00ab-foo");
        mkdir(&specs, "0001-bar");
        assert_eq!(allocate_next_spec_id(&specs), "0002");
    }

    #[test]
    fn five_digit_ids_are_supported() {
        let (_tmp, root) = make_tmp_root();
        let specs = root.join("specs");
        fs_err::create_dir_all(specs.as_std_path()).expect("mkdir");
        mkdir(&specs, "9999-last");
        mkdir(&specs, "10000-rollover");
        assert_eq!(allocate_next_spec_id(&specs), "10001");
    }

    #[test]
    fn nested_mission_folders_contribute_to_id_space() {
        let (_tmp, root) = make_tmp_root();
        let specs = root.join("specs");
        fs_err::create_dir_all(specs.as_std_path()).expect("mkdir");
        mkdir(&specs, "auth/0001-signup");
        mkdir(&specs, "billing/0002-invoice");
        assert_eq!(allocate_next_spec_id(&specs), "0003");
    }

    #[test]
    fn flat_and_nested_share_id_space() {
        let (_tmp, root) = make_tmp_root();
        let specs = root.join("specs");
        fs_err::create_dir_all(specs.as_std_path()).expect("mkdir");
        mkdir(&specs, "0001-foo");
        mkdir(&specs, "auth/0002-signup");
        mkdir(&specs, "billing/0010-invoice");
        assert_eq!(allocate_next_spec_id(&specs), "0011");
    }

    #[test]
    fn descent_does_not_double_count_inside_specs() {
        // `auth/0042-signup/` should be counted once at depth 2.
        // Anything inside `0042-signup/` (e.g. an accidentally-named
        // `0099-stale/`) must NOT be descended into.
        let (_tmp, root) = make_tmp_root();
        let specs = root.join("specs");
        fs_err::create_dir_all(specs.as_std_path()).expect("mkdir");
        mkdir(&specs, "auth/0042-signup");
        mkdir(&specs, "auth/0042-signup/0099-stale");
        assert_eq!(allocate_next_spec_id(&specs), "0043");
    }
}
