#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation"
)]
//! Integration tests for the XML-001 balance lint, across both the
//! parsed-document artifacts (SPEC.md / TASKS.md / REPORT.md) and the
//! on-demand per-task `journal/T-NNN.md` files.
//!
//! Fixtures live under tempdirs — never the real `.speccy/specs/` tree.
//! The verify-exit scenario lives in `speccy-cli/tests/verify.rs` because
//! it exercises the CLI exit-code path and the in-progress demotion gate.

use camino::Utf8PathBuf;
use speccy_core::lint::Diagnostic;
use speccy_core::lint::run;
use speccy_core::parse::supersession::supersession_index;
use speccy_core::workspace::scan;
use tempfile::TempDir;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// SPEC.md whose requirement body slot can carry an extra fixture line.
/// `__EXTRA__` is replaced with body content (or removed) per test.
const SPEC_MD: &str = "---\nid: SPEC-0042\nslug: example\ntitle: Example\nstatus: in-progress\ncreated: 2026-05-21\nsupersedes: []\n---\n\n# SPEC-0042: Example\n\n<goals>\nGoals.\n</goals>\n\n<non-goals>\nNon-goals.\n</non-goals>\n\n<user-stories>\n- Story.\n</user-stories>\n\n<requirement id=\"REQ-001\">\n### REQ-001\n\n__EXTRA__\n<done-when>\n- thing.\n</done-when>\n\n<behavior>\n- thing.\n</behavior>\n\n<scenario id=\"CHK-001\">\nWhen X then Y.\n</scenario>\n</requirement>\n\n## Changelog\n\n<changelog>\n| Date | Author | Summary |\n|------|--------|---------|\n| 2026-05-21 | t | init |\n</changelog>\n";

const CLEAN_TASK: &str = "<task id=\"T-001\" state=\"pending\" covers=\"REQ-001\">\nwork\n<task-scenarios>\n- placeholder.\n</task-scenarios>\n</task>\n";

fn utf8(dir: &TempDir) -> TestResult<Utf8PathBuf> {
    Utf8PathBuf::from_path_buf(dir.path().to_path_buf())
        .map_err(|p| format!("tempdir path must be UTF-8: {}", p.display()).into())
}

/// Build a tempdir workspace holding one spec. `spec_extra` is spliced
/// into the SPEC.md requirement body; `tasks_xml` is the TASKS.md task
/// block; `report_body`, when `Some`, writes a REPORT.md with that body
/// inside the `<report>` root.
fn make_workspace(
    spec_extra: &str,
    tasks_xml: &str,
    report_body: Option<&str>,
) -> TestResult<TempDir> {
    let dir = tempfile::tempdir()?;
    let root = utf8(&dir)?;
    let spec_dir = root.join(".speccy").join("specs").join("0042-example");
    fs_err::create_dir_all(spec_dir.as_std_path())?;

    let spec_body = SPEC_MD.replace("__EXTRA__", spec_extra);
    fs_err::write(spec_dir.join("SPEC.md").as_std_path(), spec_body)?;

    let tasks_md = format!(
        "---\nspec: SPEC-0042\nspec_hash_at_generation: bootstrap-pending\ngenerated_at: 2026-05-21T18:00:00Z\n---\n\n# Tasks: SPEC-0042\n\n{tasks_xml}\n"
    );
    fs_err::write(spec_dir.join("TASKS.md").as_std_path(), tasks_md)?;

    if let Some(body) = report_body {
        let report_md = format!(
            "---\nspec: SPEC-0042\noutcome: satisfied\ngenerated_at: 2026-05-21T18:00:00Z\n---\n\n# REPORT: SPEC-0042\n\n<report spec=\"SPEC-0042\">\n{body}\n</report>\n"
        );
        fs_err::write(spec_dir.join("REPORT.md").as_std_path(), report_md)?;
    }

    Ok(dir)
}

/// Write a `journal/<name>` file under the spec dir of an existing
/// workspace, creating the `journal/` subdir as `lint_jnl.rs` does. The
/// caller supplies the full journal file body (frontmatter + elements).
fn write_journal(dir: &TempDir, name: &str, body: &str) -> TestResult {
    let root = utf8(dir)?;
    let journal_dir = root
        .join(".speccy")
        .join("specs")
        .join("0042-example")
        .join("journal");
    fs_err::create_dir_all(journal_dir.as_std_path())?;
    fs_err::write(journal_dir.join(name).as_std_path(), body)?;
    Ok(())
}

fn run_lint(dir: &TempDir) -> TestResult<Vec<Diagnostic>> {
    let root = utf8(dir)?;
    let scanned = scan(root.as_path());
    let parsed_specs: Vec<_> = scanned.specs;
    let spec_md_refs: Vec<&_> = parsed_specs
        .iter()
        .filter_map(|s| s.spec_md.as_ref().ok())
        .collect();
    let supersession = supersession_index(&spec_md_refs);
    let workspace = speccy_core::lint::types::Workspace {
        specs: &parsed_specs,
        supersession: &supersession,
    };
    Ok(run(&workspace))
}

fn xml_001(diags: &[Diagnostic]) -> Vec<&Diagnostic> {
    diags.iter().filter(|d| d.code == "XML-001").collect()
}

/// A TASKS.md ending in two bare orphan closes (`</content>` then
/// `</invoke>`) with no matching opens fires exactly two XML-001
/// diagnostics, one per orphan close line, each naming that TASKS.md.
#[test]
fn xml_001_fires_per_orphan_close_in_tasks_md() -> TestResult {
    let tasks = format!("{CLEAN_TASK}\n</content>\n</invoke>\n");
    let dir = make_workspace("", &tasks, None)?;
    let diags = run_lint(&dir)?;
    let hits = xml_001(&diags);
    assert_eq!(hits.len(), 2, "two orphan closes -> two XML-001: {diags:?}");
    for hit in &hits {
        assert!(
            hit.file
                .as_ref()
                .is_some_and(|p| p.as_str().ends_with("TASKS.md")),
            "XML-001 must name the TASKS.md file: {:?}",
            hit.file
        );
        assert!(hit.line.is_some(), "XML-001 must carry a line: {hit:?}");
    }
    // The two distinct orphan close lines are reported separately.
    let lines: Vec<u32> = hits.iter().filter_map(|d| d.line).collect();
    assert_eq!(lines.len(), 2);
    assert_ne!(lines.first(), lines.get(1), "distinct lines: {lines:?}");
    Ok(())
}

/// A foreign non-void open on its own line with no matching close
/// fires exactly one XML-001 naming that open tag's line.
#[test]
fn xml_001_fires_for_dangling_non_void_open() -> TestResult {
    // Place the dangling open in the SPEC.md requirement body.
    let dir = make_workspace("<custom>\n", CLEAN_TASK, None)?;
    let diags = run_lint(&dir)?;
    let hits = xml_001(&diags);
    assert_eq!(hits.len(), 1, "one dangling open -> one XML-001: {diags:?}");
    let hit = hits.first().ok_or("one XML-001")?;
    assert!(
        hit.message.contains("custom"),
        "message must name the offending tag: {}",
        hit.message
    );
    assert!(
        hit.file
            .as_ref()
            .is_some_and(|p| p.as_str().ends_with("SPEC.md")),
        "XML-001 must name the SPEC.md file: {:?}",
        hit.file
    );
    Ok(())
}

/// A balanced foreign pair (`<details>` … `</details>`) produces
/// no XML-001.
#[test]
fn xml_001_silent_on_balanced_foreign_pair() -> TestResult {
    let dir = make_workspace("<details>\nsome prose\n</details>\n", CLEAN_TASK, None)?;
    let diags = run_lint(&dir)?;
    assert!(
        xml_001(&diags).is_empty(),
        "balanced foreign pair must not fire XML-001: {diags:?}"
    );
    Ok(())
}

/// A lone void-element open (`<br>`) fires no XML-001, while a
/// lone non-void foreign open fires exactly one — the exemption is scoped
/// to the void set.
#[test]
fn xml_001_exempts_void_open_but_not_non_void() -> TestResult {
    let void_dir = make_workspace("<br>\n", CLEAN_TASK, None)?;
    let void_diags = run_lint(&void_dir)?;
    assert!(
        xml_001(&void_diags).is_empty(),
        "lone void <br> must not fire XML-001: {void_diags:?}"
    );

    let non_void_dir = make_workspace("<widget>\n", CLEAN_TASK, None)?;
    let non_void_diags = run_lint(&non_void_dir)?;
    assert_eq!(
        xml_001(&non_void_diags).len(),
        1,
        "lone non-void open must fire exactly one XML-001: {non_void_diags:?}"
    );
    Ok(())
}

/// An orphan foreign close that sits only inside a fenced code
/// block fires no XML-001; a foreign close outside any fence still fires
/// regardless of fenced occurrences of the same name.
#[test]
fn xml_001_exempts_fenced_orphan_close() -> TestResult {
    // Orphan close lives only inside a fence -> no fire.
    let fenced = "```\n</fenced>\n```\n";
    let dir = make_workspace(fenced, CLEAN_TASK, None)?;
    let diags = run_lint(&dir)?;
    assert!(
        xml_001(&diags).is_empty(),
        "fenced orphan close must be exempt: {diags:?}"
    );

    // Same name appears both inside a fence and as a real orphan outside:
    // the outside one still fires, the fenced one does not pair with it.
    let mixed = "```\n</leak>\n```\n\n</leak>\n";
    let dir2 = make_workspace(mixed, CLEAN_TASK, None)?;
    let diags2 = run_lint(&dir2)?;
    let hits2 = xml_001(&diags2);
    assert_eq!(
        hits2.len(),
        1,
        "the unfenced orphan close fires regardless of fenced occurrences: {diags2:?}"
    );
    Ok(())
}

/// A spec whose SPEC.md, TASKS.md, and REPORT.md each carry
/// exactly one dangling foreign tag fires exactly three XML-001
/// diagnostics, one per artifact, each with the correct file path.
#[test]
fn xml_001_covers_all_three_parsed_artifacts() -> TestResult {
    let tasks = format!("{CLEAN_TASK}\n</orphantask>\n");
    let dir = make_workspace("<orphanspec>\n", &tasks, Some("</orphanreport>"))?;
    let diags = run_lint(&dir)?;
    let hits = xml_001(&diags);
    assert_eq!(
        hits.len(),
        3,
        "one orphan per artifact -> three XML-001: {diags:?}"
    );

    let names: Vec<&str> = hits
        .iter()
        .filter_map(|d| d.file.as_deref().map(camino::Utf8Path::as_str))
        .collect();
    for artifact in ["SPEC.md", "TASKS.md", "REPORT.md"] {
        assert!(
            names.iter().any(|n| n.ends_with(artifact)),
            "XML-001 must fire for {artifact}; files: {names:?}"
        );
    }
    Ok(())
}

/// A well-formed journal frontmatter + one `<implementer>` block whose
/// inner body slot (`__BODY__`) is filled per test. `T-001` matches the
/// `CLEAN_TASK` id, so the lint derives `journal/T-001.md`.
const JOURNAL_MD: &str = "---\nspec: SPEC-0042\ntask: T-001\ngenerated_at: 2026-05-21T18:00:00Z\n---\n\n<implementer date=\"2026-05-21T18:00:00Z\" model=\"m\" round=\"1\">\n__BODY__\n</implementer>\n";

/// A task whose `journal/T-001.md` exists and carries a dangling
/// foreign close fires exactly one XML-001 whose file is that journal file
/// and whose line is the orphan tag's line.
#[test]
fn xml_001_fires_for_dangling_tag_in_journal() -> TestResult {
    let dir = make_workspace("", CLEAN_TASK, None)?;
    let journal = JOURNAL_MD.replace("__BODY__", "completed work\n</journalorphan>");
    write_journal(&dir, "T-001.md", &journal)?;
    let diags = run_lint(&dir)?;
    let hits = xml_001(&diags);
    assert_eq!(
        hits.len(),
        1,
        "one dangling tag in journal -> one XML-001: {diags:?}"
    );
    let hit = hits.first().ok_or("one XML-001")?;
    assert!(
        hit.file
            .as_ref()
            .is_some_and(|p| p.as_str().ends_with("T-001.md")),
        "XML-001 must name the journal file: {:?}",
        hit.file
    );
    assert!(
        hit.line.is_some(),
        "XML-001 must carry the orphan tag's line: {hit:?}"
    );
    assert!(
        hit.message.contains("journalorphan"),
        "message must name the offending tag: {}",
        hit.message
    );
    Ok(())
}

/// The journal half of the silent case: a journal file with only
/// balanced foreign tags (and a missing journal for the next task) fires
/// no XML-001 for any journal file.
#[test]
fn xml_001_silent_on_balanced_journal() -> TestResult {
    let dir = make_workspace("", CLEAN_TASK, None)?;
    let journal = JOURNAL_MD.replace("__BODY__", "<details>\nnotes\n</details>");
    write_journal(&dir, "T-001.md", &journal)?;
    let diags = run_lint(&dir)?;
    assert!(
        xml_001(&diags).is_empty(),
        "balanced foreign tags in a journal must not fire XML-001: {diags:?}"
    );
    Ok(())
}
