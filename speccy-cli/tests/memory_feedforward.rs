#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
//! The implementer feed-forward read step pulls
//! the memory-ledger *summary* once from the canonical work-phase module body,
//! and no host wrapper inlines a shadowing copy of that include directive. The
//! hot work path carries only the terse read protocol; the full entry shape and
//! authoring discipline stay in `memory-ledger.md`, included by the ship phase.
//!
//! This keys on the `{% include %}` structural surface — not on curated prose —
//! so it does not assert that specific sentences appear in any skill or
//! subagent body.

use std::path::Path;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf(),
        Path::to_path_buf,
    )
}

/// The include directive that pulls the memory-ledger summary into a body — the
/// read-side digest the implementer feed-forward step carries on the hot path.
const MEMORY_LEDGER_SUMMARY_INCLUDE: &str =
    r#"{% include "modules/references/memory-ledger-summary.md" %}"#;

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

/// The canonical work-phase module body carries the memory-ledger summary
/// include exactly once — the feed-forward read step points at the shared
/// read-protocol digest rather than inlining the full entry shape.
#[test]
fn work_phase_body_includes_memory_ledger_summary_once() {
    let body_path = workspace_root()
        .join("resources")
        .join("modules")
        .join("phases")
        .join("speccy-work.md");
    let body = fs_err::read_to_string(&body_path)
        .expect("resources/modules/phases/speccy-work.md must be readable");

    let count = body.matches(MEMORY_LEDGER_SUMMARY_INCLUDE).count();
    assert_eq!(
        count, 1,
        "`{MEMORY_LEDGER_SUMMARY_INCLUDE}` must appear exactly once in \
         resources/modules/phases/speccy-work.md (the feed-forward read step; \
         SPEC-0064 REQ-003 CHK-004); found {count}",
    );
}

/// No host wrapper under `resources/agents/` inlines the memory-ledger summary
/// include. The summary reaches every host transitively through the phase-body
/// include (`{% include "modules/phases/speccy-work.md" %}`), never as a
/// shadowing copy — the no-duplicate-snippet invariant.
#[test]
fn no_host_wrapper_inlines_memory_ledger_summary_include() {
    let agents_root = workspace_root().join("resources").join("agents");
    let mut wrappers = Vec::new();
    collect_files(&agents_root, &mut wrappers);

    for wrapper in &wrappers {
        let display = wrapper.display();
        let contents = fs_err::read_to_string(wrapper)
            .expect("host wrapper under resources/agents/ must be readable");
        assert!(
            !contents.contains(MEMORY_LEDGER_SUMMARY_INCLUDE),
            "host wrapper `{display}` must not inline `{MEMORY_LEDGER_SUMMARY_INCLUDE}`; \
             the memory-ledger summary reaches every host transitively via the \
             phase-body include, never as a shadowing copy (SPEC-0064 REQ-003 CHK-004)",
        );
    }
}
