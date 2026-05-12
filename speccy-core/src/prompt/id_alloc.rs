//! Spec ID allocator (`max + 1`; no gap recycling).
//!
//! Per SPEC-0005 DEC-005 and `.speccy/DESIGN.md` "Spec ID allocation":
//! gaps left by dropped specs are not recycled so historical SPEC IDs
//! retain unambiguous meaning in commits and PR descriptions.

use camino::Utf8Path;
use regex::Regex;
use std::sync::OnceLock;

/// Scan `specs_dir` for immediate subdirectories whose name matches
/// `^(\d{4,})-` and return the next available numeric prefix as a
/// zero-padded 4+ digit string (`"0001"`, `"0014"`, `"10000"` ...).
///
/// Returns `"0001"` when `specs_dir` is absent, empty, or contains no
/// matching directories. Non-matching subdirectories (e.g. `_scratch`,
/// `00ab-foo`) are silently ignored.
#[must_use = "the allocated ID identifies the next spec to be created"]
pub fn allocate_next_spec_id(specs_dir: &Utf8Path) -> String {
    let pattern = dir_prefix_regex();
    let Ok(entries) = fs_err::read_dir(specs_dir.as_std_path()) else {
        return "0001".to_owned();
    };
    let mut max: Option<u64> = None;
    for entry in entries.flatten() {
        let Ok(meta) = entry.metadata() else { continue };
        if !meta.is_dir() {
            continue;
        }
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let Some(caps) = pattern.captures(name) else {
            continue;
        };
        let Some(digits) = caps.get(1) else { continue };
        let Ok(n) = digits.as_str().parse::<u64>() else {
            continue;
        };
        max = Some(max.map_or(n, |m| m.max(n)));
    }
    let next = max.map_or(1_u64, |m| m.saturating_add(1));
    format!("{next:04}")
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
}
