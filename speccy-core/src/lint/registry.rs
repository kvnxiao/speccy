//! Stability registry: every lint code the engine can emit, with its
//! severity. A snapshot test pins this list so that removing or renaming
//! a code breaks the build (SPEC-0003 REQ-007).

use crate::lint::types::Level;

/// Every (code, severity) pair the lint engine can emit.
///
/// This list is **append-only across minor versions**. Removing or
/// renaming an entry is a breaking change; changing a severity is a
/// breaking change. The snapshot test in
/// `speccy-core/tests/lint_registry.rs` pins this list.
pub const REGISTRY: &[(&str, Level)] = &[
    ("SPC-001", Level::Error),
    ("SPC-002", Level::Error),
    ("SPC-003", Level::Error),
    ("SPC-004", Level::Error),
    ("SPC-005", Level::Error),
    ("SPC-006", Level::Error),
    ("SPC-007", Level::Info),
    ("REQ-001", Level::Error),
    ("REQ-002", Level::Error),
    ("REQ-003", Level::Error),
    ("TSK-001", Level::Error),
    ("TSK-002", Level::Error),
    ("TSK-003", Level::Warn),
    ("TSK-004", Level::Error),
    ("QST-001", Level::Info),
];

/// Look up the registered severity for `code`. Returns `None` if `code`
/// is not in the registry.
#[must_use = "the looked-up severity describes how callers should react"]
pub fn lookup_severity(code: &str) -> Option<Level> {
    REGISTRY
        .iter()
        .find_map(|(c, level)| if *c == code { Some(*level) } else { None })
}

/// Render the registry into the snapshot text format
/// (`<code>\t<severity>\n` per line, ascending by code).
#[must_use = "the rendered registry is the on-disk snapshot form"]
pub fn render_snapshot() -> String {
    let mut entries: Vec<(&str, Level)> = REGISTRY.to_vec();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    let mut out = String::with_capacity(entries.len() * 16);
    for (code, level) in entries {
        out.push_str(code);
        out.push('\t');
        out.push_str(level.as_str());
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::REGISTRY;
    use super::lookup_severity;
    use super::render_snapshot;
    use crate::lint::types::Level;

    #[test]
    fn registry_has_no_duplicates() {
        let mut codes: Vec<&str> = REGISTRY.iter().map(|(c, _)| *c).collect();
        codes.sort_unstable();
        let unique = codes.iter().zip(codes.iter().skip(1)).all(|(a, b)| a != b);
        assert!(unique, "REGISTRY contains duplicate codes: {codes:?}");
    }

    #[test]
    fn lookup_severity_known_code() {
        assert_eq!(lookup_severity("TSK-003"), Some(Level::Warn));
        assert_eq!(lookup_severity("SPC-001"), Some(Level::Error));
        assert_eq!(lookup_severity("QST-001"), Some(Level::Info));
    }

    #[test]
    fn lookup_severity_unknown_code() {
        assert_eq!(lookup_severity("DOES-NOT-EXIST"), None);
    }

    #[test]
    fn snapshot_is_sorted() {
        let snap = render_snapshot();
        let lines: Vec<&str> = snap.lines().collect();
        let mut sorted = lines.clone();
        sorted.sort_unstable();
        assert_eq!(lines, sorted, "render_snapshot output must be sorted");
    }
}
