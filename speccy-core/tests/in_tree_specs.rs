#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]

//! SPEC-0020 T-004 corpus integration tests.
//!
//! After the SPEC-0019 → SPEC-0020 migration runs across every in-tree
//! `.speccy/specs/NNNN-*/SPEC.md`, two invariants must hold:
//!
//! 1. Every spec file parses cleanly with the SPEC-0020 raw XML element parser
//!    ([`speccy_core::parse::parse_spec_xml`]).
//! 2. The per-spec requirement / scenario / decision id sets equal the
//!    pre-migration id sets captured in
//!    `tests/fixtures/in_tree_id_snapshot.json`. That fixture was generated
//!    before the bulk migration by running the SPEC-0019 marker parser over
//!    every in-tree SPEC.md (and the SPEC-0020 XML parser over SPEC-0020 /
//!    SPEC-0022, which were already authored in raw XML before T-004 started).
//!    Equality between the pre-migration and post-migration id sets is the
//!    structural guarantee that the migration was mechanical: tags swapped,
//!    identifiers preserved.
//!
//! `no_spec_toml_files_remain_under_speccy_specs` survives from the
//! SPEC-0019 T-004 invariant and continues to assert no stray
//! `spec.toml` files leak back in.

use camino::Utf8Path;
use camino::Utf8PathBuf;
use serde_json::Value;
use speccy_core::parse::parse_spec_xml;
use std::collections::BTreeMap;

fn workspace_root() -> Utf8PathBuf {
    // CARGO_MANIFEST_DIR is `speccy-core`; parent is the workspace root.
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set by cargo");
    let manifest = Utf8PathBuf::from(manifest_dir);
    manifest
        .parent()
        .expect("speccy-core has a parent")
        .to_path_buf()
}

fn spec_dirs(root: &Utf8Path) -> Vec<Utf8PathBuf> {
    let specs_dir = root.join(".speccy").join("specs");
    let mut out = Vec::new();
    for entry in fs_err::read_dir(specs_dir.as_std_path()).expect("read .speccy/specs") {
        let entry = entry.expect("read dir entry");
        let path = entry.path();
        let utf8 =
            Utf8PathBuf::from_path_buf(path).expect("non-utf8 spec dir name should not exist");
        if utf8.is_dir() && utf8.join("SPEC.md").is_file() {
            out.push(utf8);
        }
    }
    out.sort();
    out
}

fn load_snapshot() -> BTreeMap<String, IdSet> {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set by cargo");
    let path = Utf8PathBuf::from(manifest_dir)
        .join("tests")
        .join("fixtures")
        .join("in_tree_id_snapshot.json");
    let source = fs_err::read_to_string(path.as_std_path()).expect("snapshot fixture is readable");
    let value: Value = serde_json::from_str(&source).expect("snapshot is valid JSON");
    let mut out: BTreeMap<String, IdSet> = BTreeMap::new();
    let map = value
        .as_object()
        .expect("snapshot root is a JSON object keyed by spec dir name");
    for (k, v) in map {
        out.insert(k.clone(), IdSet::from_json(v));
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IdSet {
    requirements: Vec<String>,
    scenarios: Vec<String>,
    decisions: Vec<String>,
}

impl IdSet {
    fn from_json(v: &Value) -> Self {
        let obj = v.as_object().expect("id-set entry is a JSON object");
        let extract = |k: &str| -> Vec<String> {
            obj.get(k)
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|s| s.as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default()
        };
        let mut me = Self {
            requirements: extract("requirements"),
            scenarios: extract("scenarios"),
            decisions: extract("decisions"),
        };
        me.requirements.sort();
        me.scenarios.sort();
        me.decisions.sort();
        me
    }
}

#[test]
fn every_in_tree_spec_md_parses_with_xml_parser_and_matches_snapshot() {
    let root = workspace_root();
    let dirs = spec_dirs(&root);
    assert!(
        !dirs.is_empty(),
        "expected at least one spec under .speccy/specs/",
    );
    let snapshot = load_snapshot();
    let mut parse_failures: Vec<String> = Vec::new();
    let mut mismatches: Vec<String> = Vec::new();
    let mut covered: Vec<String> = Vec::new();
    for d in &dirs {
        let spec_md_path = d.join("SPEC.md");
        let source = fs_err::read_to_string(spec_md_path.as_std_path())
            .expect("reading SPEC.md should succeed");
        let doc = match parse_spec_xml(&source, &spec_md_path) {
            Ok(doc) => doc,
            Err(e) => {
                parse_failures.push(format!("{spec_md_path}: {e}"));
                continue;
            }
        };
        let name = d.file_name().expect("spec dir has a name").to_owned();
        covered.push(name.clone());
        let actual = IdSet {
            requirements: {
                let mut v: Vec<String> = doc.requirements.iter().map(|r| r.id.clone()).collect();
                v.sort();
                v
            },
            scenarios: {
                let mut v: Vec<String> = doc
                    .requirements
                    .iter()
                    .flat_map(|r| r.scenarios.iter().map(|s| s.id.clone()))
                    .collect();
                v.sort();
                v
            },
            decisions: {
                let mut v: Vec<String> = doc.decisions.iter().map(|d| d.id.clone()).collect();
                v.sort();
                v
            },
        };
        match snapshot.get(&name) {
            Some(expected) if expected == &actual => {}
            Some(expected) => {
                mismatches.push(format!(
                    "{name}: id-set drift from pre-migration snapshot\n  expected: {expected:?}\n  actual:   {actual:?}",
                ));
            }
            None => {
                mismatches.push(format!(
                    "{name}: missing from pre-migration snapshot fixture",
                ));
            }
        }
    }
    assert!(
        parse_failures.is_empty(),
        "SPEC.md files failed to parse with parse_spec_xml:\n{}",
        parse_failures.join("\n"),
    );
    assert!(
        mismatches.is_empty(),
        "id-set drift between pre-migration snapshot and post-migration XML parse:\n{}",
        mismatches.join("\n"),
    );
    // Sanity: every spec listed in the snapshot must have been touched
    // by the loop above, otherwise a spec dir was deleted out from
    // under the fixture.
    for k in snapshot.keys() {
        assert!(
            covered.contains(k),
            "snapshot lists `{k}` but no matching spec dir was scanned",
        );
    }
}

#[test]
fn no_spec_toml_files_remain_under_speccy_specs() {
    let root = workspace_root();
    let dirs = spec_dirs(&root);
    let mut stray: Vec<Utf8PathBuf> = Vec::new();
    for d in &dirs {
        let candidate = d.join("spec.toml");
        if candidate.exists() {
            stray.push(candidate);
        }
    }
    assert!(
        stray.is_empty(),
        "stray spec.toml files remain after migration: {stray:?}",
    );
}

/// Whitelist of Speccy structure element names per SPEC-0020 DEC-002.
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
fn every_migrated_spec_md_has_blank_line_after_each_close_tag() {
    // SPEC-0020 T-002 pinned "every closing element tag is followed by a
    // blank line" as the renderer's canonical convention
    // (`render_emits_blank_line_after_every_closing_element_tag` in
    // `tests/spec_xml_roundtrip.rs`). T-004 normalises every in-tree
    // SPEC.md to that same convention. This test pins it so a future
    // migration rerun (or a stray hand-edit) that drops the blank line
    // fails CI loudly, instead of hiding behind manual `git diff`
    // inspection as the previous reviewer flagged.
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
        "SPEC-0020 blank-line-after-close convention violated:\n{}",
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
fn spec_0019_fenced_example_preserves_legacy_marker_form() {
    // REQ-004's third Done-when bullet: fenced examples that document
    // the SPEC-0019 marker form must survive migration byte-for-byte.
    // SPEC-0019 carries the canonical marker-form example inside a
    // ```markdown fence; pin it against a small inline fixture string
    // so a future rerun of the migration (or a stray hand-edit) cannot
    // silently rewrite the documentation.
    let root = workspace_root();
    let path = root
        .join(".speccy")
        .join("specs")
        .join("0019-xml-canonical-spec-md")
        .join("SPEC.md");
    let source = fs_err::read_to_string(path.as_std_path()).expect("SPEC-0019 SPEC.md is readable");
    let expected = "```markdown\n\
<!-- speccy:requirement id=\"REQ-001\" -->\n\
### REQ-001: Render selected scenarios\n\
\n\
Plain Markdown prose remains plain Markdown.\n\
\n\
<!-- speccy:scenario id=\"CHK-001\" -->\n\
Given a task covers REQ-001,\n\
when `speccy check SPEC-0019/T-001` runs,\n\
then only REQ-001's scenarios are rendered.\n\
<!-- /speccy:scenario -->\n\
<!-- /speccy:requirement -->\n\
```";
    assert!(
        source.contains(expected),
        "SPEC-0019 fenced marker-form example drift: expected substring\n\
        ---\n{expected}\n---\n\
        not found in SPEC.md (path: {path})",
    );
}

#[test]
fn spec_0020_fenced_example_preserves_raw_xml_form() {
    // Companion to the SPEC-0019 fence pin above: SPEC-0020's authored
    // raw-XML example block must also survive migration unchanged.
    // SPEC-0020 documents the raw-tag carrier inside a ```markdown
    // fence; pin the body byte-for-byte so the migration normaliser
    // cannot silently mutate documentation that describes Speccy's own
    // grammar.
    let root = workspace_root();
    let path = root
        .join(".speccy")
        .join("specs")
        .join("0020-raw-xml-spec-carrier")
        .join("SPEC.md");
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
