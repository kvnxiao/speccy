#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]

//! SPEC-0020 T-007 corpus contract: active shipped guidance must not
//! reference SPEC-0019's HTML-comment marker form.
//!
//! REQ-005 done-when bullet 5 names this property as the load-bearing
//! grep: "A grep for `<!-- speccy:` in active (non-historical) guidance
//! returns hits only inside migration-context documentation or this
//! spec's own summary/decisions."
//!
//! Scope: this test scans every Markdown / TOML file under
//! `resources/modules/prompts/`, `resources/modules/personas/`,
//! `resources/modules/skills/`, the rendered host mirrors
//! (`.claude/skills/`, `.agents/skills/`, `.codex/agents/`,
//! `.speccy/skills/`), and `.speccy/ARCHITECTURE.md`. It fails loudly
//! if any of those files contains the literal substring `<!-- speccy:`
//! outside the small allow-list below:
//!
//! 1. `.speccy/ARCHITECTURE.md`'s migration-history paragraph — the paragraph
//!    that documents SPEC-0019's superseded marker carrier and points readers
//!    at the equivalent SPEC-0020 element tag.
//!
//! No other file in active guidance is allowed to teach the legacy
//! marker form. If you legitimately need to mention it (e.g. a future
//! migration note), extend the allow-list explicitly so the contract
//! stays load-bearing.

use camino::Utf8Path;
use camino::Utf8PathBuf;

/// The legacy SPEC-0019 marker substring we are guarding against.
const LEGACY_MARKER: &str = "<!-- speccy:";

fn workspace_root() -> Utf8PathBuf {
    let manifest = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("speccy-cli has a parent")
        .to_path_buf()
}

/// Files where the legacy marker substring is allowed because they
/// teach it as historical / migration context. Paths are
/// workspace-root-relative.
const ALLOW_LIST: &[&str] = &[".speccy/ARCHITECTURE.md"];

/// File extensions to scan. Markdown and TOML cover every guidance
/// file under the scoped directories today.
const SCANNED_EXTENSIONS: &[&str] = &["md", "tmpl", "toml"];

/// Recursively collect every scanned file under `dir`.
fn collect_scanned_files(dir: &Utf8Path, out: &mut Vec<Utf8PathBuf>) {
    let std_dir = dir.as_std_path();
    if !std_dir.exists() {
        return;
    }
    let read = fs_err::read_dir(std_dir).expect("read scan dir");
    for entry in read {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        let utf8 = Utf8PathBuf::from_path_buf(path).expect("utf8 path under scan dir");
        let file_type = entry.file_type().expect("file type");
        if file_type.is_dir() {
            collect_scanned_files(&utf8, out);
        } else if file_type.is_file()
            && utf8.extension().is_some_and(|ext| {
                SCANNED_EXTENSIONS
                    .iter()
                    .any(|allowed| ext.eq_ignore_ascii_case(allowed))
            })
        {
            out.push(utf8);
        }
    }
}

#[test]
fn active_guidance_does_not_teach_legacy_html_comment_markers() {
    let root = workspace_root();

    let mut files: Vec<Utf8PathBuf> = Vec::new();
    for rel in [
        "resources/modules/personas",
        "resources/modules/prompts",
        "resources/modules/skills",
        ".claude/skills",
        ".agents/skills",
        ".codex/agents",
        ".speccy/skills",
    ] {
        collect_scanned_files(&root.join(rel), &mut files);
    }
    // Single architecture file outside any of the scanned trees above.
    let arch = root.join(".speccy").join("ARCHITECTURE.md");
    if arch.as_std_path().exists() {
        files.push(arch);
    }

    let allow_paths: Vec<Utf8PathBuf> = ALLOW_LIST.iter().map(|rel| root.join(rel)).collect();

    let mut offenders: Vec<(Utf8PathBuf, usize, String)> = Vec::new();
    for file in &files {
        if allow_paths.iter().any(|allowed| allowed == file) {
            continue;
        }
        let body = fs_err::read_to_string(file.as_std_path()).expect("read scan file");
        for (idx, line) in body.lines().enumerate() {
            if line.contains(LEGACY_MARKER) {
                offenders.push((file.clone(), idx + 1, line.to_owned()));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "active shipped guidance must not teach the legacy `<!-- speccy:` \
         marker form (SPEC-0020 T-007). Allowed only in {ALLOW_LIST:?}. \
         Offending hits:\n{offenders:#?}"
    );
}

#[test]
fn architecture_md_legacy_marker_mention_is_historical_only() {
    // Sanity-check the allow-list: ARCHITECTURE.md's mentions of
    // `<!-- speccy:` must sit inside the historical-note paragraph
    // that names SPEC-0019. If a future edit drops the historical
    // framing around the legacy substring, this test fails so the
    // allow-list can be re-validated rather than silently broadened.
    let root = workspace_root();
    let arch = root.join(".speccy").join("ARCHITECTURE.md");
    let body = fs_err::read_to_string(arch.as_std_path()).expect("read .speccy/ARCHITECTURE.md");

    let lines: Vec<&str> = body.lines().collect();
    for (idx, line) in lines.iter().enumerate() {
        if !line.contains(LEGACY_MARKER) {
            continue;
        }
        // The legacy marker must appear inside a contiguous block of
        // historical-note lines (lines that start with the Markdown
        // blockquote marker `>` and somewhere in the surrounding
        // window mention SPEC-0019). The historical note in
        // ARCHITECTURE.md uses `>` for its blockquote framing.
        let start = idx.saturating_sub(6);
        let end = (idx + 6).min(lines.len().saturating_sub(1));
        let window: Vec<&&str> = (start..=end).filter_map(|i| lines.get(i)).collect();
        let mentions_spec_0019 = window
            .iter()
            .any(|l| l.to_ascii_uppercase().contains("SPEC-0019"));
        let is_blockquote = line.trim_start().starts_with('>');
        assert!(
            mentions_spec_0019 && is_blockquote,
            "ARCHITECTURE.md line {} mentions `{}` outside a historical \
             blockquote window referencing SPEC-0019. Line: `{}`",
            idx + 1,
            LEGACY_MARKER,
            line,
        );
    }
}
