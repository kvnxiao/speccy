#![expect(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
//! Resource-prose hygiene lint: no internal artifact-ID provenance in the
//! agent-facing module bodies under `resources/modules/`.
//!
//! Every `*.md` body that `speccy init` ejects into a user's repo must use
//! only the generic placeholder ids (`SPEC-NNNN`, `REQ-NNN`, `DEC-NNN`,
//! `T-NNN`, `CHK-NNN`) or the single whitelisted concrete example `SPEC-0042`.
//! A real Speccy spec / requirement / decision / task / lint id cited as
//! provenance is pure noise in another repo — and an invitation to
//! hallucinate. See AGENTS.md → "Authoring resource prose".
//!
//! Source-only scan: the dogfood byte-identity test
//! (`tests/init.rs::dogfood_outputs_match_committed_tree`) already proves
//! eject == source, so scanning the ejected `.claude/` / `.codex/` / `.agents/`
//! trees would only double-count. `.speccy/specs/` is outside `resources/` and
//! is never walked — Speccy's own dogfood artifacts stay Speccy-specific.

use regex::Regex;
use std::path::Path;
use std::path::PathBuf;
use std::sync::OnceLock;

/// Workspace root, derived from `CARGO_MANIFEST_DIR` (the `speccy-cli` crate
/// dir) by walking one level up. Mirrors the helper in the sibling test
/// crates so this scan reads the on-disk canonical sources under
/// `resources/modules/`.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf(),
        Path::to_path_buf,
    )
}

/// `SPEC-NNNN`-shaped ids carrying a real digit run; every match except the
/// whitelisted `SPEC-0042` is a violation. The generic letter-form `SPEC-NNNN`
/// has no digit run after the dash, so it never matches.
fn spec_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"\bSPEC-\d{4,}\b").expect("valid SPEC id regex"))
}

/// Lint-family ids (`REQ` / `DEC` / `CHK` / `TSK` / `JNL`) carrying a real
/// digit run; any match is a violation (no exemptions). The `TSK` / `JNL` arms
/// also catch CLI lint codes cited by number rather than described by behavior.
/// The `\b` boundary keeps `TSK-003` out of [`task_regex`].
fn family_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| {
        Regex::new(r"\b(?:REQ|DEC|CHK|TSK|JNL)-\d{3,}\b").expect("valid family id regex")
    })
}

/// `T-NNN` task ids carrying a real digit run; any match is a violation. The
/// leading `\b` keeps `TSK-003` (matched by [`family_regex`]) and ISO
/// timestamps like `...T19:45:00Z` out of this regex.
fn task_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"\bT-\d{3,}\b").expect("valid task id regex"))
}

/// Every `resources/modules/**/*.md` source path under `root`, sorted.
fn module_md_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect_md(&root.join("resources").join("modules"), &mut out);
    out.sort();
    out
}

fn collect_md(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs_err::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect_md(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            out.push(path);
        }
    }
}

/// A single provenance violation: file (workspace-relative), 1-indexed line,
/// offending token, and the fix hint surfaced to the author.
struct Violation {
    rel_path: String,
    line_no: usize,
    token: String,
    fix: &'static str,
}

#[test]
fn module_prose_has_no_internal_artifact_id_provenance() {
    let root = workspace_root();
    let files = module_md_files(&root);

    // Floor guard: a path or layout change that returns near-zero files would
    // make the scan pass vacuously.
    assert!(
        files.len() >= 30,
        "resource-prose scan found only {} .md files under resources/modules/ — \
         the scan scope looks broken",
        files.len(),
    );

    let mut violations: Vec<Violation> = Vec::new();
    for path in &files {
        let rel_path = path
            .strip_prefix(&root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        let body = fs_err::read_to_string(path).expect("module body must be UTF-8 readable");
        for (idx, line) in body.lines().enumerate() {
            let line_no = idx + 1;
            for m in spec_regex().find_iter(line) {
                if m.as_str() == "SPEC-0042" {
                    continue;
                }
                violations.push(Violation {
                    rel_path: rel_path.clone(),
                    line_no,
                    token: m.as_str().to_owned(),
                    fix: "use the generic `SPEC-NNNN`, or the whitelisted example `SPEC-0042`",
                });
            }
            for m in family_regex().find_iter(line) {
                violations.push(Violation {
                    rel_path: rel_path.clone(),
                    line_no,
                    token: m.as_str().to_owned(),
                    fix: "use the generic `<PREFIX>-NNN` form; cite lint codes (TSK-/JNL-) by behavior, not by number",
                });
            }
            for m in task_regex().find_iter(line) {
                violations.push(Violation {
                    rel_path: rel_path.clone(),
                    line_no,
                    token: m.as_str().to_owned(),
                    fix: "use the generic `T-NNN` form",
                });
            }
        }
    }

    assert!(
        violations.is_empty(),
        "internal artifact-ID provenance found in resources/modules/ prose \
         (see AGENTS.md -> \"Authoring resource prose\"):\n{}",
        violations
            .iter()
            .map(|v| format!(
                "  {}:{} -- `{}` -> {}",
                v.rel_path, v.line_no, v.token, v.fix
            ))
            .collect::<Vec<_>>()
            .join("\n"),
    );
}

/// Guards the detection regexes themselves: a pattern that silently stopped
/// matching real ids would let the scan above pass vacuously. Asserts each
/// regex fires on a concrete id and stays quiet on the generic placeholder and
/// on lookalikes that belong to a different family.
#[test]
fn id_regexes_match_concrete_ids_and_skip_generic_placeholders() {
    assert!(
        spec_regex().is_match("SPEC-0045"),
        "concrete SPEC id matches"
    );
    assert!(
        spec_regex().is_match("SPEC-0042"),
        "the regex matches SPEC-0042; the scan exempts it by exact-string check, not by regex",
    );
    assert!(
        !spec_regex().is_match("SPEC-NNNN"),
        "generic SPEC placeholder has no digit run",
    );

    assert!(
        family_regex().is_match("REQ-001"),
        "concrete REQ id matches"
    );
    assert!(
        family_regex().is_match("TSK-003"),
        "numbered lint code matches the family regex",
    );
    assert!(
        !family_regex().is_match("REQ-NNN"),
        "generic REQ placeholder has no digit run",
    );
    assert!(
        !family_regex().is_match("the SPC-* lint family"),
        "SPC is not in the banned family set",
    );

    assert!(task_regex().is_match("T-001"), "concrete task id matches");
    assert!(
        !task_regex().is_match("T-NNN"),
        "generic task placeholder has no digit run",
    );
    assert!(
        !task_regex().is_match("TSK-003"),
        "the word boundary keeps TSK-003 out of the task regex (it is a family id)",
    );
    assert!(
        !task_regex().is_match("2026-05-21T19:45:00Z"),
        "an ISO timestamp carries no `T-<digits>` token",
    );
}
