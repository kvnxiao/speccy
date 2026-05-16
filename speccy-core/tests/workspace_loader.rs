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

    <requirement id="REQ-001">
    ### REQ-001: First
    body
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
/// `speccy_core::parse`. Grep-style assertion scoped to `pub use` /
/// `pub mod` lines in `parse/mod.rs`. Doc comments and the SPEC-0020
/// `compile_fail` doctest may mention deleted symbols by name; the
/// invariant is that none of them are actually re-exported.
#[test]
fn deleted_symbols_are_not_re_exported_from_parse_module() -> TestResult {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = std::path::Path::new(manifest_dir).join("src/parse/mod.rs");
    let contents = fs_err::read_to_string(&path)?;
    let export_lines: Vec<&str> = contents
        .lines()
        .filter(|l| {
            let t = l.trim_start();
            t.starts_with("pub use ") || t.starts_with("pub mod ")
        })
        .collect();
    let exports = export_lines.join("\n");
    for forbidden in &[
        // SPEC-0019 deletions — per-spec TOML.
        "SpecToml",
        "RequirementEntry",
        "CheckEntry",
        "spec_toml",
        // SPEC-0020 T-005: marker parser symbols.
        "spec_markers",
        "parse_spec_markers",
        "render_spec_markers",
        "MarkerSpan",
    ] {
        assert!(
            !exports.contains(forbidden),
            "`{forbidden}` must not be re-exported from speccy_core::parse after T-005; export block:\n{exports}",
        );
    }
    Ok(())
}

/// SPEC-0020 T-005 extra coverage: the marker parser module file is
/// gone from disk (REQ-002 "deleted, not feature-flagged"). A
/// build-time grep guards against accidental resurrection.
#[test]
fn spec_markers_module_file_is_gone() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = std::path::Path::new(manifest_dir).join("src/parse/spec_markers.rs");
    assert!(
        !path.exists(),
        "speccy-core/src/parse/spec_markers.rs must be deleted after T-005; found at {}",
        path.display(),
    );
    let dir = std::path::Path::new(manifest_dir).join("src/parse/spec_markers");
    assert!(
        !dir.exists(),
        "speccy-core/src/parse/spec_markers/ directory must be deleted after T-005; found at {}",
        dir.display(),
    );
}

/// SPEC-0020 T-005: a stray SPEC.md that still contains the SPEC-0019
/// HTML-comment marker form surfaces as `ParseError::LegacyMarker`,
/// and `Display` carries the suggested raw-XML element form.
#[test]
fn stray_legacy_marker_spec_md_surfaces_as_legacy_marker_error() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let root = utf8(&tmp)?;
    // Hand-authored marker-form SPEC.md (the form SPEC-0019 shipped).
    let legacy = indoc! {r#"
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
    write_spec_md(&root, "0001-legacy", legacy)?;

    let ws = scan(&root);
    let only = ws.specs.first().expect("one spec");
    let err = only
        .spec_doc
        .as_ref()
        .expect_err("legacy marker form must be rejected as a parse error");
    assert!(
        matches!(err, ParseError::LegacyMarker { .. }),
        "expected ParseError::LegacyMarker, got {err:?}",
    );
    let rendered = format!("{err}");
    assert!(
        rendered.contains("<!-- speccy:requirement"),
        "diagnostic must name the legacy marker form; got: {rendered}",
    );
    assert!(
        rendered.contains("<requirement"),
        "diagnostic must suggest the equivalent raw XML element form; got: {rendered}",
    );
    Ok(())
}

/// SPEC-0020 T-005: a stray legacy-marker SPEC.md surfaces as SPC-001
/// through the lint engine (the workspace-level diagnostic channel
/// users see via `speccy verify` / `speccy status`).
#[test]
fn legacy_marker_spec_md_surfaces_as_spc_001_diagnostic() -> TestResult {
    use speccy_core::lint::Level;
    use speccy_core::lint::run as lint_run;
    let tmp = tempfile::tempdir()?;
    let root = utf8(&tmp)?;
    let legacy = indoc! {r#"
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
    write_spec_md(&root, "0001-legacy", legacy)?;

    let ws = scan(&root);
    let lint_ws = ws.as_lint_workspace();
    let diagnostics = lint_run(&lint_ws);
    let spc_001: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "SPC-001" && matches!(d.level, Level::Error))
        .collect();
    assert!(
        !spc_001.is_empty(),
        "SPC-001 must fire for a legacy-marker SPEC.md; got diagnostics: {:?}",
        diagnostics.iter().map(|d| d.code).collect::<Vec<_>>(),
    );
    let message = &spc_001
        .first()
        .expect("at least one SPC-001 diagnostic was asserted above")
        .message;
    assert!(
        message.contains("<requirement"),
        "SPC-001 message must carry the suggested raw XML element form from LegacyMarker `Display`; got: {message}",
    );
    Ok(())
}

/// SPEC-0020 T-005: duplicate `<scenario id="CHK-NNN">` ids inside one
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

        <requirement id="REQ-001">
        ### REQ-001: First
        body
        <scenario id="CHK-001">
        first
        </scenario>
        </requirement>
        <requirement id="REQ-002">
        ### REQ-002: Second
        body
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
        matches!(err, ParseError::DuplicateMarkerId { .. }),
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

        <requirement id="REQ-001">
        ### REQ-001: First
        body
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
