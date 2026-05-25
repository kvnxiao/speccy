#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]

//! In-tree corpus integration tests.
//!
//! Invariants that should hold for every spec that lives under
//! `.speccy/specs/NNNN-*/`:
//!
//! 1. Every `SPEC.md` parses cleanly with the raw XML element parser
//!    ([`speccy_core::parse::parse_spec_xml`]).
//! 2. Renderer convention: every whitelisted closing element tag is followed by
//!    a blank line.
//! 3. Fenced documentation examples in the specs that document Speccy's own
//!    grammar survive byte-for-byte, so silent rewrites of their fenced bodies
//!    cannot corrupt the canonical reference.

mod corpus;

use corpus::find_spec_dir;
use corpus::spec_dirs;
use corpus::workspace_root;
use speccy_core::parse::parse_spec_xml;

#[test]
fn every_in_tree_spec_md_parses_with_xml_parser() {
    let root = workspace_root();
    let dirs = spec_dirs(&root);
    assert!(
        !dirs.is_empty(),
        "expected at least one spec under .speccy/specs/",
    );
    let mut parse_failures: Vec<String> = Vec::new();
    for d in &dirs {
        let spec_md_path = d.join("SPEC.md");
        let source = fs_err::read_to_string(spec_md_path.as_std_path())
            .expect("reading SPEC.md should succeed");
        if let Err(e) = parse_spec_xml(&source, &spec_md_path) {
            parse_failures.push(format!("{spec_md_path}: {e}"));
        }
    }
    assert!(
        parse_failures.is_empty(),
        "SPEC.md files failed to parse with parse_spec_xml:\n{}",
        parse_failures.join("\n"),
    );
}

/// Whitelist of Speccy structure element names.
/// Mirrored from the renderer's emission set so the convention assertion
/// below stays anchored to the renderer's contract.
const WHITELIST_NAMES: &[&str] = &[
    "requirement",
    "scenario",
    "decision",
    "changelog",
    "open-question",
    "overview",
    "spec",
];

/// Track `CommonMark` fenced code block state while iterating lines.
///
/// Recognises a fence opener as a line whose leading content (after up to
/// three spaces of indent) is a run of >=3 backticks or >=3 tildes,
/// optionally followed by an info string. The matching close has the same
/// fence character with a run length >= the opener and no info string.
/// This is the same line-aware rule the SPEC-0020 raw XML element scanner
/// already uses to decide which lines carry structure.
#[derive(Default)]
struct FenceTracker {
    in_fence: bool,
    fence_char: char,
    fence_run: usize,
}

impl FenceTracker {
    /// Update fence state from `line`. Returns `true` if `line` is itself
    /// a fence delimiter (open or close).
    fn observe(&mut self, line: &str) -> bool {
        let trimmed = line.trim_start_matches(' ');
        // `CommonMark` caps indent at 3 spaces for fences.
        if line.len() - trimmed.len() > 3 {
            return false;
        }
        let first = trimmed.chars().next();
        let Some(c) = first else { return false };
        if c != '`' && c != '~' {
            return false;
        }
        let run: String = trimmed.chars().take_while(|x| *x == c).collect();
        if run.len() < 3 {
            return false;
        }
        let info = trimmed.get(run.len()..).unwrap_or("").trim();
        if self.in_fence {
            if c == self.fence_char && run.len() >= self.fence_run && info.is_empty() {
                self.in_fence = false;
                self.fence_char = ' ';
                self.fence_run = 0;
                return true;
            }
            false
        } else {
            self.in_fence = true;
            self.fence_char = c;
            self.fence_run = run.len();
            true
        }
    }
}

#[test]
fn every_in_tree_spec_md_has_blank_line_after_each_close_tag() {
    // "Every closing element tag is followed by a blank line" is the
    // renderer's canonical convention
    // (`render_emits_blank_line_after_every_closing_element_tag` in
    // `tests/spec_xml_roundtrip.rs`). This test pins the convention so
    // a stray hand-edit that drops the blank line fails CI loudly
    // instead of hiding behind manual `git diff` inspection.
    let root = workspace_root();
    let dirs = spec_dirs(&root);
    let mut drift: Vec<String> = Vec::new();
    for d in &dirs {
        let spec_md_path = d.join("SPEC.md");
        let source = fs_err::read_to_string(spec_md_path.as_std_path())
            .expect("reading SPEC.md should succeed");
        let lines: Vec<&str> = source.lines().collect();
        let mut fence = FenceTracker::default();
        for (idx, line) in lines.iter().enumerate() {
            let was_fence = fence.observe(line);
            if was_fence || fence.in_fence {
                continue;
            }
            if !is_whitelist_close_line(line) {
                continue;
            }
            // Allowed: end-of-file, or next line is blank (empty).
            let Some(next) = lines.get(idx + 1) else {
                continue;
            };
            if !next.is_empty() {
                drift.push(format!(
                    "{spec_md_path}:{}: close tag `{}` not followed by blank line (next: `{}`)",
                    idx + 1,
                    line,
                    next,
                ));
            }
        }
    }
    assert!(
        drift.is_empty(),
        "blank-line-after-close convention violated:\n{}",
        drift.join("\n"),
    );
}

fn is_whitelist_close_line(line: &str) -> bool {
    let trimmed = line.trim();
    if !trimmed.starts_with("</") || !trimmed.ends_with('>') {
        return false;
    }
    let inner = trimmed
        .strip_prefix("</")
        .and_then(|s| s.strip_suffix('>'))
        .unwrap_or("");
    WHITELIST_NAMES.contains(&inner)
}

#[test]
fn spec_0020_fenced_example_preserves_raw_xml_form() {
    // SPEC-0020 documents the raw-tag carrier inside a ```markdown
    // fence; pin the body byte-for-byte so any future bulk-rewrite
    // cannot silently mutate documentation that describes Speccy's own
    // grammar.
    let root = workspace_root();
    let spec_dir = find_spec_dir(&root, "0020-raw-xml-spec-carrier")
        .expect("SPEC-0020 exists under .speccy/specs/ or .speccy/archive/");
    let path = spec_dir.join("SPEC.md");
    let source = fs_err::read_to_string(path.as_std_path()).expect("SPEC-0020 SPEC.md is readable");
    let expected = "```markdown\n\
<requirement id=\"REQ-001\">\n\
### REQ-001: Render selected scenarios\n\
\n\
Plain Markdown prose remains plain Markdown.\n\
\n\
<scenario id=\"CHK-001\">\n\
Given a task covers REQ-001,\n\
when `speccy check SPEC-0019/T-001` runs,\n\
then only REQ-001's scenarios are rendered.\n\
</scenario>\n\
</requirement>\n\
```";
    assert!(
        source.contains(expected),
        "SPEC-0020 fenced raw-XML example drift: expected substring\n\
        ---\n{expected}\n---\n\
        not found in SPEC.md (path: {path})",
    );
}
