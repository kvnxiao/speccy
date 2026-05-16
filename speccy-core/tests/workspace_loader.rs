#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![allow(
    clippy::unwrap_in_result,
    reason = "test code may .expect() inside TestResult fns"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! T-005 acceptance tests: the workspace loader speaks `SpecDoc`, not
//! the deleted per-spec `spec.toml` shape.
//!
//! Bullets mirrored from `.speccy/specs/0019-xml-canonical-spec-md/TASKS.md`.

use camino::Utf8Path;
use camino::Utf8PathBuf;
use indoc::indoc;
use speccy_core::ParseError;
use speccy_core::workspace::scan;
use tempfile::TempDir;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn utf8(dir: &TempDir) -> TestResult<Utf8PathBuf> {
    Utf8PathBuf::from_path_buf(dir.path().to_path_buf())
        .map_err(|p| format!("tempdir path must be UTF-8: {}", p.display()).into())
}

fn write_spec_md(
    project_root: &Utf8Path,
    dir_name: &str,
    contents: &str,
) -> TestResult<Utf8PathBuf> {
    let dir = project_root.join(".speccy").join("specs").join(dir_name);
    fs_err::create_dir_all(dir.as_std_path())?;
    fs_err::write(dir.join("SPEC.md").as_std_path(), contents)?;
    Ok(dir)
}

const VALID_SPEC_MD: &str = indoc! {r#"
    ---
    id: SPEC-0001
    slug: x
    title: Example
    status: in-progress
    created: 2026-05-11
    ---

    # Example

    <!-- speccy:requirement id="REQ-001" -->
    ### REQ-001: First
    body
    <!-- speccy:scenario id="CHK-001" -->
    covers REQ-001
    <!-- /speccy:scenario -->
    <!-- /speccy:requirement -->

    ## Changelog

    <!-- speccy:changelog -->
    | Date | Author | Summary |
    |------|--------|---------|
    | 2026-05-11 | t | init |
    <!-- /speccy:changelog -->
"#};

/// Bullet 1: loader loads each spec as a `SpecDoc`, with the
/// requirement-to-scenario edge sourced from `Scenario.parent_requirement_id`.
#[test]
fn loader_links_scenarios_to_requirements_via_marker_containment() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let root = utf8(&tmp)?;
    write_spec_md(&root, "0001-foo", VALID_SPEC_MD)?;

    let ws = scan(&root);
    let only = ws.specs.first().expect("one spec");
    let doc = only
        .spec_doc
        .as_ref()
        .expect("spec_doc should parse against migrated workspace");

    let req = doc
        .requirements
        .first()
        .expect("requirement parsed from marker tree");
    assert_eq!(req.id, "REQ-001");
    let scenario = req
        .scenarios
        .first()
        .expect("scenario parsed from nested marker");
    assert_eq!(scenario.id, "CHK-001");
    assert_eq!(
        scenario.parent_requirement_id, "REQ-001",
        "parent_requirement_id is the canonical source for REQ→CHK linkage, not any TOML table",
    );
    Ok(())
}

/// Bullet 2: stray `spec.toml` makes the loader record
/// `ParseError::StraySpecToml` whose `Display` names the file path.
#[test]
fn stray_spec_toml_surfaces_as_loader_parse_error() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let root = utf8(&tmp)?;
    let dir = write_spec_md(&root, "0001-foo", VALID_SPEC_MD)?;
    let stray = dir.join("spec.toml");
    fs_err::write(stray.as_std_path(), "schema_version = 1\n")?;

    let ws = scan(&root);
    let only = ws.specs.first().expect("one spec");
    let err = only
        .spec_doc
        .as_ref()
        .expect_err("spec_doc must be Err when spec.toml is stray");
    match err {
        ParseError::StraySpecToml { path } => {
            assert_eq!(
                path, &stray,
                "StraySpecToml path must be the actual file path"
            );
        }
        other => {
            return Err(format!("expected StraySpecToml, got {other:?}").into());
        }
    }
    let rendered = format!("{err}");
    assert!(
        rendered.contains(stray.as_str()),
        "Display impl must name the stray file path; got: {rendered}",
    );
    Ok(())
}

/// Bullet 3: SPEC-0019 deleted types are gone from
/// `speccy_core::parse`. Grep-style assertion over `parse/mod.rs`.
#[test]
fn deleted_symbols_are_not_re_exported_from_parse_module() -> TestResult {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = std::path::Path::new(manifest_dir).join("src/parse/mod.rs");
    let contents = fs_err::read_to_string(&path)?;
    for forbidden in &[
        "pub use toml_files::SpecToml",
        "pub use toml_files::RequirementEntry",
        "pub use toml_files::CheckEntry",
        "pub use toml_files::spec_toml",
    ] {
        assert!(
            !contents.contains(forbidden),
            "`{forbidden}` must not be re-exported from speccy_core::parse after T-005",
        );
    }
    Ok(())
}

/// Bullet 4: workspace-level `speccy.toml` still parses cleanly via
/// the surviving `ProjectConfig` parser.
#[test]
fn workspace_speccy_toml_still_parses() -> TestResult {
    use speccy_core::parse::speccy_toml;
    let tmp = tempfile::tempdir()?;
    let root = utf8(&tmp)?;
    let path = root.join("speccy.toml");
    fs_err::write(
        path.as_std_path(),
        indoc! {r#"
            schema_version = 1

            [project]
            name = "demo"
        "#},
    )?;
    let parsed = speccy_toml(&path).expect("workspace speccy.toml must still parse");
    assert_eq!(parsed.project.name, "demo");
    Ok(())
}

/// Bullet 5: a requirement marker with two nested scenarios surfaces
/// as a requirement proved by two scenarios in the loader output.
#[test]
fn requirement_with_two_scenarios_reports_two_proofs() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let root = utf8(&tmp)?;
    let spec_md = indoc! {r#"
        ---
        id: SPEC-0001
        slug: x
        title: Example
        status: in-progress
        created: 2026-05-11
        ---

        # Example

        <!-- speccy:requirement id="REQ-001" -->
        ### REQ-001: First
        body
        <!-- speccy:scenario id="CHK-001" -->
        first
        <!-- /speccy:scenario -->
        <!-- speccy:scenario id="CHK-002" -->
        second
        <!-- /speccy:scenario -->
        <!-- /speccy:requirement -->

        ## Changelog

        <!-- speccy:changelog -->
        | Date | Author | Summary |
        |------|--------|---------|
        | 2026-05-11 | t | init |
        <!-- /speccy:changelog -->
    "#};
    write_spec_md(&root, "0001-foo", spec_md)?;

    let ws = scan(&root);
    let only = ws.specs.first().expect("one spec");
    let doc = only.spec_doc.as_ref().expect("spec_doc must parse");
    let req = doc.requirements.first().expect("requirement present");
    assert_eq!(
        req.scenarios.len(),
        2,
        "two nested scenarios must surface as two proofs",
    );
    let scenario_ids: Vec<&str> = req.scenarios.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(scenario_ids, vec!["CHK-001", "CHK-002"]);
    Ok(())
}
