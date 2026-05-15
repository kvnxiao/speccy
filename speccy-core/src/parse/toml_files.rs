//! Parsers for Speccy's two TOML files: `speccy.toml` and `spec.toml`.
//!
//! See `.speccy/specs/0001-artifact-parsers/SPEC.md` REQ-001 for the
//! complete contract.

use crate::error::ParseError;
use camino::Utf8Path;
use serde::Deserialize;

const SUPPORTED_SCHEMA_VERSION: i64 = 1;

/// Parsed `speccy.toml` workspace config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeccyConfig {
    /// The `[project]` table.
    pub project: ProjectConfig,
}

/// The `[project]` block inside `speccy.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectConfig {
    /// Project name (free-form string).
    pub name: String,
}

/// Parsed `spec.toml` for one spec folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecToml {
    /// Requirements in declared order.
    pub requirements: Vec<RequirementEntry>,
    /// Checks in declared order.
    pub checks: Vec<CheckEntry>,
}

/// One `[[requirements]]` row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequirementEntry {
    /// Stable `REQ-NNN` identifier.
    pub id: String,
    /// IDs of checks that prove this requirement.
    pub checks: Vec<String>,
}

/// One `[[checks]]` row. Post-SPEC-0018 the schema collapses to exactly
/// two fields: an identifier and an English validation scenario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckEntry {
    /// Stable `CHK-NNN` identifier.
    pub id: String,
    /// English validation scenario. Non-empty after trim.
    pub scenario: String,
}

#[derive(Debug, Deserialize)]
struct RawSpeccyConfig {
    schema_version: i64,
    project: RawProject,
}

#[derive(Debug, Deserialize)]
struct RawProject {
    name: String,
}

#[derive(Debug, Deserialize)]
struct RawSpecToml {
    schema_version: i64,
    #[serde(default)]
    requirements: Vec<RawRequirement>,
    #[serde(default)]
    checks: Vec<RawCheck>,
}

#[derive(Debug, Deserialize)]
struct RawRequirement {
    id: String,
    #[serde(default)]
    checks: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCheck {
    id: String,
    scenario: String,
}

/// Parse a `speccy.toml` file.
///
/// # Errors
///
/// Returns any [`ParseError`] variant relevant to TOML parsing: I/O,
/// non-UTF-8 file content, unsupported `schema_version`, or missing
/// required fields.
pub fn speccy_toml(path: &Utf8Path) -> Result<SpeccyConfig, ParseError> {
    let content = read_to_string(path)?;
    let raw: RawSpeccyConfig = toml::from_str(&content).map_err(|e| ParseError::Toml {
        path: path.to_path_buf(),
        message: e.to_string(),
    })?;

    guard_schema_version(raw.schema_version, path)?;

    Ok(SpeccyConfig {
        project: ProjectConfig {
            name: raw.project.name,
        },
    })
}

/// Parse a `spec.toml` file.
///
/// # Errors
///
/// Returns any [`ParseError`] variant relevant to TOML parsing: I/O,
/// non-UTF-8 file content, unsupported `schema_version`, an empty or
/// whitespace-only `scenario` on a check row, an unknown field on any
/// `[[checks]]` row, or missing required fields.
pub fn spec_toml(path: &Utf8Path) -> Result<SpecToml, ParseError> {
    let content = read_to_string(path)?;
    let raw: RawSpecToml = toml::from_str(&content).map_err(|e| ParseError::Toml {
        path: path.to_path_buf(),
        message: e.to_string(),
    })?;

    guard_schema_version(raw.schema_version, path)?;

    let requirements = raw
        .requirements
        .into_iter()
        .map(|row| RequirementEntry {
            id: row.id,
            checks: row.checks,
        })
        .collect();

    let mut checks = Vec::with_capacity(raw.checks.len());
    for row in raw.checks {
        let entry = lower_check(row, path)?;
        checks.push(entry);
    }

    Ok(SpecToml {
        requirements,
        checks,
    })
}

fn lower_check(row: RawCheck, path: &Utf8Path) -> Result<CheckEntry, ParseError> {
    if row.scenario.trim().is_empty() {
        return Err(ParseError::InvalidCheckEntry {
            path: path.to_path_buf(),
            check_id: row.id,
            reason: "`scenario` is empty or whitespace-only".to_owned(),
        });
    }
    Ok(CheckEntry {
        id: row.id,
        scenario: row.scenario,
    })
}

fn guard_schema_version(value: i64, path: &Utf8Path) -> Result<(), ParseError> {
    if value == SUPPORTED_SCHEMA_VERSION {
        Ok(())
    } else {
        Err(ParseError::UnsupportedSchemaVersion {
            path: path.to_path_buf(),
            value,
        })
    }
}

pub(crate) fn read_to_string(path: &Utf8Path) -> Result<String, ParseError> {
    fs_err::read_to_string(path.as_std_path()).map_err(|e| ParseError::Io {
        path: path.to_path_buf(),
        source: e,
    })
}

#[cfg(test)]
mod tests {
    use super::spec_toml;
    use super::speccy_toml;
    use crate::error::ParseError;
    use camino::Utf8Path;
    use camino::Utf8PathBuf;
    use indoc::indoc;
    use tempfile::TempDir;

    struct Fixture {
        _dir: TempDir,
        path: Utf8PathBuf,
    }

    fn write_tmp(name: &str, content: &str) -> Fixture {
        let dir = tempfile::tempdir().expect("tempdir creation should succeed");
        let std_path = dir.path().join(name);
        fs_err::write(&std_path, content).expect("writing fixture should succeed");
        let path = Utf8PathBuf::from_path_buf(std_path).expect("tempdir path should be UTF-8");
        Fixture { _dir: dir, path }
    }

    #[test]
    fn parses_valid_speccy_toml() {
        let src = indoc! {r#"
            schema_version = 1

            [project]
            name = "demo"
        "#};
        let fx = write_tmp("speccy.toml", src);
        let parsed = speccy_toml(&fx.path).expect("parse should succeed");
        assert_eq!(parsed.project.name, "demo");
    }

    #[test]
    fn rejects_unknown_schema_version() {
        let src = indoc! {r#"
            schema_version = 2

            [project]
            name = "demo"
        "#};
        let fx = write_tmp("speccy.toml", src);
        let err = speccy_toml(&fx.path).expect_err("schema_version = 2 must fail");
        assert!(
            matches!(err, ParseError::UnsupportedSchemaVersion { value: 2, .. }),
            "got: {err:?}",
        );
    }

    #[test]
    fn parses_valid_spec_toml_preserves_order() {
        let src = indoc! {r#"
            schema_version = 1

            [[requirements]]
            id = "REQ-001"
            checks = ["CHK-001"]

            [[requirements]]
            id = "REQ-002"
            checks = ["CHK-002"]

            [[checks]]
            id = "CHK-001"
            scenario = "Given the parser, when fed a row, then it returns CHK-001."

            [[checks]]
            id = "CHK-002"
            scenario = "Given the parser, when fed a row, then it returns CHK-002."
        "#};
        let fx = write_tmp("spec.toml", src);
        let parsed = spec_toml(&fx.path).expect("parse should succeed");

        let req_ids: Vec<&str> = parsed.requirements.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(req_ids, vec!["REQ-001", "REQ-002"]);

        let check_ids: Vec<&str> = parsed.checks.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(check_ids, vec!["CHK-001", "CHK-002"]);

        let first_scenario = parsed
            .checks
            .first()
            .map(|c| c.scenario.clone())
            .expect("at least one check");
        assert_eq!(
            first_scenario,
            "Given the parser, when fed a row, then it returns CHK-001."
        );
    }

    #[test]
    fn rejects_check_with_legacy_command_field() {
        let src = indoc! {r#"
            schema_version = 1

            [[checks]]
            id = "CHK-001"
            scenario = "Given X, when Y, then Z."
            command = "cargo test"
        "#};
        let fx = write_tmp("spec.toml", src);
        let err = spec_toml(&fx.path).expect_err("legacy `command` must fail");
        assert!(matches!(err, ParseError::Toml { .. }), "got: {err:?}");
    }

    #[test]
    fn rejects_check_with_legacy_prompt_field() {
        let src = indoc! {r#"
            schema_version = 1

            [[checks]]
            id = "CHK-001"
            scenario = "Given X, when Y, then Z."
            prompt = "verify"
        "#};
        let fx = write_tmp("spec.toml", src);
        let err = spec_toml(&fx.path).expect_err("legacy `prompt` must fail");
        assert!(matches!(err, ParseError::Toml { .. }), "got: {err:?}");
    }

    #[test]
    fn rejects_check_with_legacy_kind_field() {
        let src = indoc! {r#"
            schema_version = 1

            [[checks]]
            id = "CHK-001"
            scenario = "Given X, when Y, then Z."
            kind = "test"
        "#};
        let fx = write_tmp("spec.toml", src);
        let err = spec_toml(&fx.path).expect_err("legacy `kind` must fail");
        assert!(matches!(err, ParseError::Toml { .. }), "got: {err:?}");
    }

    #[test]
    fn rejects_check_with_legacy_proves_field() {
        let src = indoc! {r#"
            schema_version = 1

            [[checks]]
            id = "CHK-001"
            scenario = "Given X, when Y, then Z."
            proves = "z"
        "#};
        let fx = write_tmp("spec.toml", src);
        let err = spec_toml(&fx.path).expect_err("legacy `proves` must fail");
        assert!(matches!(err, ParseError::Toml { .. }), "got: {err:?}");
    }

    #[test]
    fn rejects_missing_required_field() {
        let src = indoc! {r#"
            schema_version = 1

            [[checks]]
            scenario = "Given X, when Y, then Z."
        "#};
        let fx = write_tmp("spec.toml", src);
        let err = spec_toml(&fx.path).expect_err("missing id must fail");
        assert!(matches!(err, ParseError::Toml { .. }), "got: {err:?}");
    }

    #[test]
    fn parses_scenario_field_verbatim() {
        let src = indoc! {r#"
            schema_version = 1

            [[checks]]
            id = "CHK-001"
            scenario = "Given a workspace, when verify runs, then it exits 0."
        "#};
        let fx = write_tmp("spec.toml", src);
        let parsed = spec_toml(&fx.path).expect("parse should succeed");
        let entry = parsed.checks.first().expect("one check");
        assert_eq!(entry.id, "CHK-001");
        assert_eq!(
            entry.scenario,
            "Given a workspace, when verify runs, then it exits 0."
        );
    }

    #[test]
    fn parses_multiline_scenario_verbatim() {
        let src = indoc! {r#"
            schema_version = 1

            [[checks]]
            id = "CHK-001"
            scenario = """
            Given a workspace,
            when verify runs,
            then it exits 0.
            """
        "#};
        let fx = write_tmp("spec.toml", src);
        let parsed = spec_toml(&fx.path).expect("parse should succeed");
        let entry = parsed.checks.first().expect("one check");
        assert!(entry.scenario.contains("Given a workspace"));
        assert!(entry.scenario.contains("then it exits 0."));
    }

    #[test]
    fn rejects_empty_scenario() {
        let src = indoc! {r#"
            schema_version = 1

            [[checks]]
            id = "CHK-001"
            scenario = "   "
        "#};
        let fx = write_tmp("spec.toml", src);
        let err = spec_toml(&fx.path).expect_err("empty scenario must fail");
        assert!(
            matches!(
                &err,
                ParseError::InvalidCheckEntry { check_id, reason, .. }
                    if check_id == "CHK-001" && reason.contains("scenario")
            ),
            "got: {err:?}",
        );
    }

    #[test]
    fn io_error_names_the_path() {
        let path = Utf8Path::new("definitely/does/not/exist.toml");
        let err = speccy_toml(path).expect_err("missing file must error");
        assert!(
            matches!(
                &err,
                ParseError::Io { path: errpath, .. } if errpath == path
            ),
            "got: {err:?}",
        );
    }
}
