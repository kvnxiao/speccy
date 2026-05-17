//! Helpers shared by every `tests/lint_*.rs` integration test.
//!
//! Each integration test binary compiles this module independently and
//! may use only a subset of the helpers. The module-level expect below
//! silences dead-code warnings in test binaries that exercise only a
//! subset of the helpers.

#![expect(
    dead_code,
    reason = "shared test helpers; each test binary uses only a subset"
)]

use camino::Utf8PathBuf;
use indoc::indoc;
use speccy_core::lint::ParsedSpec;
use speccy_core::lint::Workspace;
use speccy_core::lint::run;
use speccy_core::lint::types::Diagnostic;
use speccy_core::parse::parse_report_xml;
use speccy_core::parse::parse_spec_xml;
use speccy_core::parse::parse_task_xml;
use speccy_core::parse::spec_md;
use speccy_core::parse::supersession::SupersessionIndex;
use speccy_core::parse::supersession::supersession_index;
use tempfile::TempDir;

/// Shared boxed-error result type so helpers can propagate failures
/// with `?` while staying inside the test-code expect/unwrap policy.
pub type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// One on-disk spec fixture rooted at a `TempDir`. The legacy
/// `spec_toml` field was removed by SPEC-0019; per-spec data lives in
/// `SPEC.md` raw XML elements (SPEC-0020).
pub struct Fixture {
    pub _dir: TempDir,
    pub spec_md_path: Utf8PathBuf,
    pub tasks_md_path: Option<Utf8PathBuf>,
    pub dir_path: Utf8PathBuf,
}

/// Write a single spec fixture into a tempdir. `spec_md` should be a
/// canonical raw-XML-element-structured SPEC.md (see [`valid_spec_md`]).
pub fn write_spec_fixture(spec_md: &str, tasks_md: Option<&str>) -> TestResult<Fixture> {
    let dir = tempfile::tempdir()?;
    let dir_path = Utf8PathBuf::from_path_buf(dir.path().to_path_buf())
        .map_err(|p| format!("tempdir path must be UTF-8: {}", p.display()))?;

    let spec_md_path = dir_path.join("SPEC.md");
    fs_err::write(spec_md_path.as_std_path(), spec_md)?;

    let tasks_md_path = match tasks_md {
        Some(content) => {
            let p = dir_path.join("TASKS.md");
            fs_err::write(p.as_std_path(), content)?;
            Some(p)
        }
        None => None,
    };

    Ok(Fixture {
        _dir: dir,
        spec_md_path,
        tasks_md_path,
        dir_path,
    })
}

/// Build a `ParsedSpec` by parsing each artifact via SPEC-0001's
/// parsers and SPEC-0020's raw XML element parser.
pub fn parse_fixture(fx: &Fixture) -> ParsedSpec {
    let spec_md_result = spec_md(&fx.spec_md_path);
    let spec_id = spec_md_result
        .as_ref()
        .ok()
        .map(|s| s.frontmatter.id.clone());
    // Mirror the workspace loader's SPEC-0019 stray-spec.toml check so
    // SPC-001 fires on the lint side of the test harness too.
    let stray_path = fx.dir_path.join("spec.toml");
    let spec_doc_result = if fs_err::metadata(stray_path.as_std_path()).is_ok() {
        Err(speccy_core::ParseError::StraySpecToml { path: stray_path })
    } else {
        fs_err::read_to_string(fx.spec_md_path.as_std_path())
            .map_err(|e| speccy_core::ParseError::Io {
                path: fx.spec_md_path.clone(),
                source: e,
            })
            .and_then(|src| parse_spec_xml(&src, &fx.spec_md_path))
    };
    let tasks_md_result = fx.tasks_md_path.as_ref().map(|p| {
        fs_err::read_to_string(p.as_std_path())
            .map_err(|e| speccy_core::ParseError::Io {
                path: p.clone(),
                source: e,
            })
            .and_then(|src| parse_task_xml(&src, p))
    });
    let report_md_path = fx.dir_path.join("REPORT.md");
    let report_md_result =
        if fs_err::metadata(report_md_path.as_std_path()).is_ok_and(|m| m.is_file()) {
            Some(
                fs_err::read_to_string(report_md_path.as_std_path())
                    .map_err(|e| speccy_core::ParseError::Io {
                        path: report_md_path.clone(),
                        source: e,
                    })
                    .and_then(|src| parse_report_xml(&src, &report_md_path)),
            )
        } else {
            None
        };

    let spec_md_mtime = fs_err::metadata(fx.spec_md_path.as_std_path())
        .ok()
        .and_then(|m| m.modified().ok());
    let tasks_md_mtime = fx.tasks_md_path.as_ref().and_then(|p| {
        fs_err::metadata(p.as_std_path())
            .ok()
            .and_then(|m| m.modified().ok())
    });

    ParsedSpec {
        spec_id,
        dir: fx.dir_path.clone(),
        spec_md_path: fx.spec_md_path.clone(),
        tasks_md_path: fx.tasks_md_path.clone(),
        spec_md: spec_md_result,
        spec_doc: spec_doc_result,
        tasks_md: tasks_md_result,
        report_md: report_md_result,
        spec_md_mtime,
        tasks_md_mtime,
    }
}

/// Run lint against a set of parsed specs.
pub fn run_lint(specs: &[ParsedSpec]) -> Vec<Diagnostic> {
    let spec_md_refs: Vec<&_> = specs
        .iter()
        .filter_map(|s| s.spec_md.as_ref().ok())
        .collect();
    let index: SupersessionIndex = supersession_index(&spec_md_refs);
    let workspace = Workspace {
        specs,
        supersession: &index,
    };
    run(&workspace)
}

/// Convenience: parse + lint a single fixture.
pub fn lint_fixture(fx: &Fixture) -> Vec<Diagnostic> {
    let parsed = parse_fixture(fx);
    run_lint(&[parsed])
}

/// Minimal valid raw-XML-element-structured SPEC.md for tests that
/// need a backdrop. Replace `__ID__` in the returned string to inject a
/// spec id.
pub fn valid_spec_md(id: &str) -> String {
    let template = indoc! {r#"
        ---
        id: __ID__
        slug: x
        title: y
        status: in-progress
        created: 2026-05-11
        ---

        # Test spec

        <goals>
        Test goals.
        </goals>

        <non-goals>
        Test non-goals.
        </non-goals>

        <user-stories>
        - Test user story.
        </user-stories>

        <requirement id="REQ-001">
        ### REQ-001: First

        Body.

        <done-when>
        - placeholder.
        </done-when>

        <behavior>
        - placeholder.
        </behavior>

        <scenario id="CHK-001">
        Given REQ-001, when the suite runs, then it covers REQ-001.
        </scenario>
        </requirement>

        ## Changelog

        <changelog>
        | Date       | Author | Summary |
        |------------|--------|---------|
        | 2026-05-11 | tester | Initial. |
        </changelog>
    "#};
    template.replace("__ID__", id)
}

/// Minimal valid raw-XML-element-structured SPEC.md, kept as a separate
/// helper name so older test bodies that referenced `valid_spec_toml`
/// for pairing can be adapted incrementally.
pub fn valid_spec_md_default() -> String {
    valid_spec_md("SPEC-0001")
}

/// Deliberately-unused helper that keeps the `parse_report_xml` import
/// alive for future integration tests. Renamed without a leading
/// underscore so `dead_code` fires on it in every test binary; this
/// makes the module-level `#![expect(dead_code, ...)]` fulfilled
/// regardless of which subset of helpers a given test target uses.
pub fn touch_report_md_for_future_tests() {
    let _ = parse_report_xml;
}
