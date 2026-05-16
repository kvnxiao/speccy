//! Parser for Speccy's workspace-level `speccy.toml`.
//!
//! Per-spec `spec.toml` files were removed by SPEC-0019: requirement
//! and scenario data now lives in the raw-XML-element SPEC.md
//! (see [`crate::parse::spec_xml`]). Only the workspace
//! `speccy.toml` carrier remains.
//!
//! See `.speccy/specs/0001-artifact-parsers/SPEC.md` REQ-001 for the
//! original two-file contract; SPEC-0019 narrowed it to one file and
//! SPEC-0020 swapped the carrier shape from HTML-comment markers to
//! raw XML element tags.

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

#[derive(Debug, Deserialize)]
struct RawSpeccyConfig {
    schema_version: i64,
    project: RawProject,
}

#[derive(Debug, Deserialize)]
struct RawProject {
    name: String,
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
