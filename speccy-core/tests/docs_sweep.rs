#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]

//! Docs sweep integration tests.
//!
//! Asserts that:
//! - `.speccy/ARCHITECTURE.md` documents the raw XML element grammar by
//!   containing the canonical element names currently in the whitelist.
//! - The ephemeral migration `xtask/` directories have been deleted.
//! - No active instruction in `resources/modules/` (the source-of-truth skill
//!   pack) or any rendered host mirror (`.claude/skills/`, `.agents/skills/`,
//!   `.codex/agents/`, `.speccy/skills/`) tells an agent to read or edit a
//!   per-spec `spec.toml`; matches are allowed only inside lines or files that
//!   flag themselves as migration / historical notes.

use camino::Utf8Path;
use camino::Utf8PathBuf;

fn workspace_root() -> Utf8PathBuf {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set by cargo");
    let manifest = Utf8PathBuf::from(manifest_dir);
    manifest
        .parent()
        .expect("speccy-core has a parent")
        .to_path_buf()
}

/// A `spec.toml` mention is allowed only if its line contains a
/// migration / historical marker. Match is case-insensitive.
fn line_is_historical(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("migration")
        || lower.contains("spec-0019")
        || lower.contains("history")
        || lower.contains("historical")
}

/// `line_is_historical` extended over a small line window so wrapped
/// prose that splits the marker across consecutive lines still counts
/// as historical context.
fn mention_is_historical(lines: &[&str], idx: usize) -> bool {
    let start = idx.saturating_sub(1);
    let end = (idx + 1).min(lines.len().saturating_sub(1));
    (start..=end).any(|i| lines.get(i).is_some_and(|l| line_is_historical(l)))
}

#[test]
fn architecture_md_documents_xml_element_grammar() {
    let root = workspace_root();
    let arch = root.join(".speccy").join("ARCHITECTURE.md");
    let body = fs_err::read_to_string(arch.as_std_path()).expect("read .speccy/ARCHITECTURE.md");

    // ARCHITECTURE.md must teach the raw XML element grammar: every
    // element name in the live whitelist, plus the HTML5-disjointness
    // invariant.
    for needle in [
        "<requirement",
        "<scenario",
        "<decision",
        "<changelog",
        "<open-question",
        "HTML5",
    ] {
        assert!(
            body.contains(needle),
            "ARCHITECTURE.md must document the raw XML element grammar by \
             mentioning `{needle}`"
        );
    }
}

#[test]
fn architecture_md_pins_no_public_speccy_fmt() {
    let root = workspace_root();
    let arch = root.join(".speccy").join("ARCHITECTURE.md");
    let body = fs_err::read_to_string(arch.as_std_path()).expect("read .speccy/ARCHITECTURE.md");

    let has_pinning_line = body.lines().any(|line| line.contains("speccy fmt"));

    assert!(
        has_pinning_line,
        "ARCHITECTURE.md must contain at least one line that mentions \
         `speccy fmt`, pinning the no-public-formatter contract so a future \
         deletion of the \"What We Deliberately Don't Do\" row regresses loudly"
    );
}

#[test]
fn migration_xtask_directories_are_deleted() {
    let root = workspace_root();
    let spec_0019_xtask = root.join("xtask").join("migrate-spec-markers-0019");
    assert!(
        !spec_0019_xtask.as_std_path().exists(),
        "{spec_0019_xtask} must be deleted at the end of SPEC-0019"
    );
    let spec_0020_xtask = root.join("xtask").join("migrate-spec-xml-0020");
    assert!(
        !spec_0020_xtask.as_std_path().exists(),
        "{spec_0020_xtask} must be deleted at the end of SPEC-0020 T-007"
    );
}

/// Walk `dir` recursively and collect `.md` (and `SKILL.md`) files.
fn collect_md_files(dir: &Utf8Path, out: &mut Vec<Utf8PathBuf>) {
    let std_dir = dir.as_std_path();
    if !std_dir.exists() {
        return;
    }
    let read = fs_err::read_dir(std_dir).expect("read skill dir");
    for entry in read {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        let utf8 = Utf8PathBuf::from_path_buf(path).expect("utf8 path under skill mirror");
        let file_type = entry.file_type().expect("file type");
        if file_type.is_dir() {
            collect_md_files(&utf8, out);
        } else if file_type.is_file()
            && utf8
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
        {
            out.push(utf8);
        }
    }
}

#[test]
fn shipped_skills_do_not_instruct_editing_per_spec_spec_toml() {
    let root = workspace_root();
    let mut skill_files: Vec<Utf8PathBuf> = Vec::new();
    for rel in [
        "resources/modules",
        ".claude/skills",
        ".agents/skills",
        ".codex/agents",
        ".speccy/skills",
    ] {
        collect_md_files(&root.join(rel), &mut skill_files);
    }

    let mut offenders: Vec<(Utf8PathBuf, usize, String)> = Vec::new();
    for file in &skill_files {
        let body = fs_err::read_to_string(file.as_std_path()).expect("read skill file");
        // A file may flag itself as a migration/historical note in its
        // header; if so, every `spec.toml` mention is allowed.
        let file_lower = body.to_ascii_lowercase();
        let file_is_historical =
            file_lower.contains("migration note") || file_lower.contains("historical note");
        if file_is_historical {
            continue;
        }
        let lines: Vec<&str> = body.lines().collect();
        for (idx, line) in lines.iter().enumerate() {
            if line.contains("spec.toml") && !mention_is_historical(&lines, idx) {
                offenders.push((file.clone(), idx + 1, (*line).to_string()));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "No active skill instruction may tell an agent to edit a per-spec \
         spec.toml; offending lines: {offenders:#?}"
    );
}
