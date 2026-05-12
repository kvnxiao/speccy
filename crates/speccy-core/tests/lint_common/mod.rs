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
use speccy_core::parse::report_md;
use speccy_core::parse::spec_md;
use speccy_core::parse::spec_toml;
use speccy_core::parse::supersession::SupersessionIndex;
use speccy_core::parse::supersession::supersession_index;
use speccy_core::parse::tasks_md;
use tempfile::TempDir;

/// Shared boxed-error result type so helpers can propagate failures
/// with `?` while staying inside the test-code expect/unwrap policy.
pub type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// One on-disk spec fixture rooted at a `TempDir`.
pub struct Fixture {
    pub _dir: TempDir,
    pub spec_md_path: Utf8PathBuf,
    pub spec_toml_path: Utf8PathBuf,
    pub tasks_md_path: Option<Utf8PathBuf>,
    pub dir_path: Utf8PathBuf,
}

/// Write a single spec fixture into a tempdir.
pub fn write_spec_fixture(
    spec_md: &str,
    spec_toml: &str,
    tasks_md: Option<&str>,
) -> TestResult<Fixture> {
    let dir = tempfile::tempdir()?;
    let dir_path = Utf8PathBuf::from_path_buf(dir.path().to_path_buf())
        .map_err(|p| format!("tempdir path must be UTF-8: {}", p.display()))?;

    let spec_md_path = dir_path.join("SPEC.md");
    fs_err::write(spec_md_path.as_std_path(), spec_md)?;

    let spec_toml_path = dir_path.join("spec.toml");
    fs_err::write(spec_toml_path.as_std_path(), spec_toml)?;

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
        spec_toml_path,
        tasks_md_path,
        dir_path,
    })
}

/// Build a `ParsedSpec` by parsing each artifact via SPEC-0001's
/// parsers.
pub fn parse_fixture(fx: &Fixture) -> ParsedSpec {
    let spec_md_result = spec_md(&fx.spec_md_path);
    let spec_id = spec_md_result
        .as_ref()
        .ok()
        .map(|s| s.frontmatter.id.clone());
    let spec_toml_result = spec_toml(&fx.spec_toml_path);
    let tasks_md_result = fx.tasks_md_path.as_ref().map(|p| tasks_md(p));

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
        spec_toml_path: fx.spec_toml_path.clone(),
        tasks_md_path: fx.tasks_md_path.clone(),
        spec_md: spec_md_result,
        spec_toml: spec_toml_result,
        tasks_md: tasks_md_result,
        spec_md_mtime,
        tasks_md_mtime,
    }
}

/// Run lint against a set of parsed specs.
pub fn run_lint(specs: Vec<ParsedSpec>) -> Vec<Diagnostic> {
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
    run_lint(vec![parsed])
}

/// Minimal valid SPEC.md for tests that need a backdrop. Replace
/// `__ID__` in the returned string to inject a spec id.
pub fn valid_spec_md(id: &str) -> String {
    let template = indoc! {r"
        ---
        id: __ID__
        slug: x
        title: y
        status: in-progress
        created: 2026-05-11
        ---

        # Test spec

        ### REQ-001: First
        Body.
    "};
    template.replace("__ID__", id)
}

/// Minimal valid spec.toml matching `valid_spec_md`.
pub fn valid_spec_toml() -> String {
    indoc! {r#"
        schema_version = 1

        [[requirements]]
        id = "REQ-001"
        checks = ["CHK-001"]

        [[checks]]
        id = "CHK-001"
        kind = "test"
        command = "cargo test"
        proves = "covers REQ-001"
    "#}
    .to_owned()
}

/// Deliberately-unused helper that keeps the `report_md` import alive
/// for future integration tests. Renamed without a leading underscore so
/// `dead_code` fires on it in every test binary; this makes the
/// module-level `#![expect(dead_code, ...)]` fulfilled regardless of
/// which subset of helpers a given test target uses.
pub fn touch_report_md_for_future_tests() {
    let _ = report_md;
}
