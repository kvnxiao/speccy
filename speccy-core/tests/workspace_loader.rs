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
//! Workspace loader acceptance tests against the `SpecDoc` element tree.

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

    <goals>
    Example goals.
    </goals>

    <non-goals>
    Example non-goals.
    </non-goals>

    <user-stories>
    - Example user story.
    </user-stories>

    <requirement id="REQ-001">
    ### REQ-001: First
    body

    <done-when>
    - placeholder.
    </done-when>

    <behavior>
    - placeholder.
    </behavior>

    <scenario id="CHK-001">
    covers REQ-001
    </scenario>
    </requirement>

    ## Changelog

    <changelog>
    | Date | Author | Summary |
    |------|--------|---------|
    | 2026-05-11 | t | init |
    </changelog>
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

/// Duplicate `<scenario id="CHK-NNN">` ids inside one
/// spec surface as `ParseError::DuplicateMarkerId` from the XML parser
/// and are reported as SPC-001 by the lint engine.
#[test]
fn duplicate_chk_ids_surface_as_duplicate_marker_id_via_spc_001() -> TestResult {
    use speccy_core::lint::Level;
    use speccy_core::lint::run as lint_run;
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

        <goals>
        Example goals.
        </goals>

        <non-goals>
        Example non-goals.
        </non-goals>

        <user-stories>
        - Example user story.
        </user-stories>

        <requirement id="REQ-001">
        ### REQ-001: First
        body

        <done-when>
        - placeholder.
        </done-when>

        <behavior>
        - placeholder.
        </behavior>

        <scenario id="CHK-001">
        first
        </scenario>
        </requirement>
        <requirement id="REQ-002">
        ### REQ-002: Second
        body

        <done-when>
        - placeholder.
        </done-when>

        <behavior>
        - placeholder.
        </behavior>

        <scenario id="CHK-001">
        duplicate id
        </scenario>
        </requirement>

        ## Changelog

        <changelog>
        | Date | Author | Summary |
        |------|--------|---------|
        | 2026-05-11 | t | init |
        </changelog>
    "#};
    write_spec_md(&root, "0001-dup", spec_md)?;

    let ws = scan(&root);
    let only = ws.specs.first().expect("one spec");
    let err = only
        .spec_doc
        .as_ref()
        .expect_err("duplicate CHK ids must reject the parse");
    assert!(
        matches!(err.as_ref(), ParseError::DuplicateMarkerId { .. }),
        "expected ParseError::DuplicateMarkerId, got {err:?}",
    );

    let lint_ws = ws.as_lint_workspace();
    let diagnostics = lint_run(&lint_ws);
    let spc_001_dup: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "SPC-001" && matches!(d.level, Level::Error))
        .filter(|d| d.message.contains("CHK-001"))
        .collect();
    assert!(
        !spc_001_dup.is_empty(),
        "SPC-001 must surface the duplicate-id error and name CHK-001; got: {:?}",
        diagnostics
            .iter()
            .map(|d| (d.code, d.message.as_str()))
            .collect::<Vec<_>>(),
    );
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

        <goals>
        Example goals.
        </goals>

        <non-goals>
        Example non-goals.
        </non-goals>

        <user-stories>
        - Example user story.
        </user-stories>

        <requirement id="REQ-001">
        ### REQ-001: First
        body

        <done-when>
        - placeholder.
        </done-when>

        <behavior>
        - placeholder.
        </behavior>

        <scenario id="CHK-001">
        first
        </scenario>
        <scenario id="CHK-002">
        second
        </scenario>
        </requirement>

        ## Changelog

        <changelog>
        | Date | Author | Summary |
        |------|--------|---------|
        | 2026-05-11 | t | init |
        </changelog>
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
