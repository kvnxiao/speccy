//! Shared path-formatting helpers for the speccy CLI.

use camino::Utf8Path;

/// Convert an absolute path to a repo-relative forward-slash string.
///
/// Strips the `project_root` prefix and normalises path separators to forward
/// slashes so paths are consistent across platforms. Returns the original path
/// string if stripping fails (should be unreachable for workspace-discovered
/// paths).
#[must_use]
pub(crate) fn to_repo_relative(abs: &Utf8Path, project_root: &Utf8Path) -> String {
    abs.strip_prefix(project_root)
        .unwrap_or(abs)
        .as_str()
        .replace('\\', "/")
}
