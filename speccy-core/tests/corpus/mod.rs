//! Helpers for tests that walk the in-tree spec corpus.
//!
//! Specs live under either `.speccy/specs/NNNN-*/` (active) or
//! `.speccy/archive/NNNN-*/` (archived via `speccy archive`). Tests
//! that pin invariants over the corpus must see both locations so
//! archiving does not silently shrink the corpus the test exercises.
//!
//! Each integration test binary compiles this module independently
//! and may use only a subset of the helpers. The module-level
//! `expect(dead_code)` accommodates that.

#![expect(
    dead_code,
    reason = "shared test helpers; each test binary uses only a subset"
)]

use camino::Utf8Path;
use camino::Utf8PathBuf;

/// Resolve the workspace root from `CARGO_MANIFEST_DIR`.
///
/// Integration tests run with `CARGO_MANIFEST_DIR` pointing at the
/// member crate (e.g. `speccy-core/`); the workspace root is its
/// parent.
pub fn workspace_root() -> Utf8PathBuf {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set by cargo");
    let manifest = Utf8PathBuf::from(manifest_dir);
    manifest
        .parent()
        .expect("crate manifest dir has a parent")
        .to_path_buf()
}

/// Every spec directory under `.speccy/specs/` and `.speccy/archive/`
/// that contains a `SPEC.md`, sorted ascending by path.
///
/// Missing top-level subdirectories are tolerated; only the existing
/// ones contribute. This lets fresh-init repos and post-archive
/// repos both pass through without special-casing.
pub fn spec_dirs(root: &Utf8Path) -> Vec<Utf8PathBuf> {
    let mut out = Vec::new();
    for sub in ["specs", "archive"] {
        let dir = root.join(".speccy").join(sub);
        let Ok(entries) = fs_err::read_dir(dir.as_std_path()) else {
            continue;
        };
        for entry in entries {
            let entry = entry.expect("read dir entry");
            let path = entry.path();
            let utf8 =
                Utf8PathBuf::from_path_buf(path).expect("non-utf8 spec dir name should not exist");
            if utf8.is_dir() && utf8.join("SPEC.md").is_file() {
                out.push(utf8);
            }
        }
    }
    out.sort();
    out
}

/// Deliberately-unused helper. Guarantees the module-level
/// `expect(dead_code)` is always fulfilled, even in test binaries
/// that use every public helper above.
pub fn touch_for_dead_code_expect() {}
