#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
//! SPEC-0064 REQ-003 (CHK-004): the implementer feed-forward read step pulls
//! the memory-ledger reference once from the canonical work-phase module body,
//! and no host wrapper inlines a shadowing copy of that include directive.
//!
//! This keys on the `{% include %}` structural surface — not on curated prose —
//! so it complies with DEC-009 (no scenario asserts specific sentences appear
//! in any skill or subagent body).

use std::path::Path;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf(),
        Path::to_path_buf,
    )
}

/// The include directive that pulls the memory-ledger reference into a body.
const MEMORY_LEDGER_INCLUDE: &str = r#"{% include "modules/references/memory-ledger.md" %}"#;

/// Recursively collect every file path under `dir`.
fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs_err::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, out);
        } else if path.is_file() {
            out.push(path);
        }
    }
}

/// The canonical work-phase module body carries the memory-ledger include
/// exactly once — the feed-forward read step points at the shared reference
/// rather than restating the entry shape.
#[test]
fn work_phase_body_includes_memory_ledger_reference_once() {
    let body_path = workspace_root()
        .join("resources")
        .join("modules")
        .join("phases")
        .join("speccy-work.md");
    let body = fs_err::read_to_string(&body_path)
        .expect("resources/modules/phases/speccy-work.md must be readable");

    let count = body.matches(MEMORY_LEDGER_INCLUDE).count();
    assert_eq!(
        count, 1,
        "`{MEMORY_LEDGER_INCLUDE}` must appear exactly once in \
         resources/modules/phases/speccy-work.md (the feed-forward read step; \
         SPEC-0064 REQ-003 CHK-004); found {count}",
    );
}

/// No host wrapper under `resources/agents/` inlines the memory-ledger include.
/// The reference reaches every host transitively through the phase-body include
/// (`{% include "modules/phases/speccy-work.md" %}`), never as a shadowing copy
/// — the no-duplicate-snippet invariant (SPEC-0064 REQ-003 CHK-004).
#[test]
fn no_host_wrapper_inlines_memory_ledger_include() {
    let agents_root = workspace_root().join("resources").join("agents");
    let mut wrappers = Vec::new();
    collect_files(&agents_root, &mut wrappers);

    for wrapper in &wrappers {
        let display = wrapper.display();
        let contents = fs_err::read_to_string(wrapper)
            .expect("host wrapper under resources/agents/ must be readable");
        assert!(
            !contents.contains(MEMORY_LEDGER_INCLUDE),
            "host wrapper `{display}` must not inline `{MEMORY_LEDGER_INCLUDE}`; \
             the memory-ledger reference reaches every host transitively via the \
             phase-body include, never as a shadowing copy (SPEC-0064 REQ-003 CHK-004)",
        );
    }
}
