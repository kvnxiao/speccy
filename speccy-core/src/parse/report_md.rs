//! REPORT.md parser.
//!
//! Parses frontmatter only; the body is returned verbatim for downstream
//! consumers. See `.speccy/specs/0001-artifact-parsers/SPEC.md` REQ-005.

use crate::error::ParseError;
use crate::parse::frontmatter::Split;
use crate::parse::frontmatter::split as split_frontmatter;
use crate::parse::toml_files::read_to_string;
use camino::Utf8Path;
use serde::Deserialize;

/// Parsed REPORT.md.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportMd {
    /// YAML frontmatter.
    pub frontmatter: ReportFrontmatter,
    /// Body content immediately after the closing fence, verbatim.
    pub body: String,
}

/// REPORT.md YAML frontmatter, validated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportFrontmatter {
    /// Spec ID this report belongs to (`SPEC-NNNN`).
    pub spec: String,
    /// Outcome of the loop.
    pub outcome: ReportOutcome,
    /// UTC timestamp the report was produced. Stored verbatim.
    pub generated_at: String,
}

/// Closed set of report outcomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportOutcome {
    /// All requirements satisfied; PR landed.
    Delivered,
    /// Some requirements deferred or out-of-scope.
    Partial,
    /// Loop aborted without delivering.
    Abandoned,
}

impl ReportOutcome {
    /// On-disk string form.
    #[must_use = "the rendered outcome is the on-disk form"]
    pub const fn as_str(self) -> &'static str {
        match self {
            ReportOutcome::Delivered => "delivered",
            ReportOutcome::Partial => "partial",
            ReportOutcome::Abandoned => "abandoned",
        }
    }
}

const ALLOWED_OUTCOMES: &[&str] = &["delivered", "partial", "abandoned"];

#[derive(Debug, Deserialize)]
struct RawFrontmatter {
    spec: String,
    outcome: String,
    generated_at: String,
}

/// Parse a REPORT.md file.
///
/// # Errors
///
/// Returns any [`ParseError`] variant relevant to REPORT.md parsing: I/O,
/// missing/malformed frontmatter, invalid `outcome`, or missing required
/// fields.
pub fn report_md(path: &Utf8Path) -> Result<ReportMd, ParseError> {
    let raw = read_to_string(path)?;
    let (yaml, body) = match split_frontmatter(&raw, path)? {
        Split::Some { yaml, body } => (yaml.to_owned(), body.to_owned()),
        Split::None => {
            return Err(ParseError::MissingField {
                field: "frontmatter".to_owned(),
                context: format!("REPORT.md at {path}"),
            });
        }
    };

    let raw_fm: RawFrontmatter = serde_saphyr::from_str(&yaml).map_err(|e| ParseError::Yaml {
        label: Some(path.to_string()),
        message: e.to_string(),
    })?;

    let outcome = parse_outcome(&raw_fm.outcome, path)?;

    Ok(ReportMd {
        frontmatter: ReportFrontmatter {
            spec: raw_fm.spec,
            outcome,
            generated_at: raw_fm.generated_at,
        },
        body,
    })
}

fn parse_outcome(value: &str, path: &Utf8Path) -> Result<ReportOutcome, ParseError> {
    match value {
        "delivered" => Ok(ReportOutcome::Delivered),
        "partial" => Ok(ReportOutcome::Partial),
        "abandoned" => Ok(ReportOutcome::Abandoned),
        other => Err(ParseError::InvalidEnumValue {
            path: path.to_path_buf(),
            field: "outcome".to_owned(),
            value: other.to_owned(),
            allowed: ALLOWED_OUTCOMES.join(", "),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::ReportOutcome;
    use super::report_md;
    use crate::error::ParseError;
    use camino::Utf8PathBuf;
    use indoc::indoc;
    use tempfile::TempDir;

    struct Fixture {
        _dir: TempDir,
        path: Utf8PathBuf,
    }

    fn write_tmp(content: &str) -> Fixture {
        let dir = tempfile::tempdir().expect("tempdir creation should succeed");
        let std_path = dir.path().join("REPORT.md");
        fs_err::write(&std_path, content).expect("writing fixture should succeed");
        let path = Utf8PathBuf::from_path_buf(std_path).expect("tempdir path should be UTF-8");
        Fixture { _dir: dir, path }
    }

    #[test]
    fn parses_valid_report() {
        let src = indoc! {r"
            ---
            spec: SPEC-0001
            outcome: delivered
            generated_at: 2026-05-11T19:00:00Z
            ---

            # Report

            Body verbatim.
        "};
        let fx = write_tmp(src);
        let parsed = report_md(&fx.path).expect("parse should succeed");
        assert_eq!(parsed.frontmatter.spec, "SPEC-0001");
        assert_eq!(parsed.frontmatter.outcome, ReportOutcome::Delivered);
        assert!(parsed.body.contains("Body verbatim."));
    }

    #[test]
    fn rejects_invalid_outcome() {
        let src = indoc! {r"
            ---
            spec: SPEC-0001
            outcome: rejected
            generated_at: 2026-05-11T19:00:00Z
            ---

            body
        "};
        let fx = write_tmp(src);
        let err = report_md(&fx.path).expect_err("invalid outcome must fail");
        assert!(
            matches!(
                &err,
                ParseError::InvalidEnumValue { field, value, .. }
                    if field == "outcome" && value == "rejected"
            ),
            "got: {err:?}",
        );
    }

    #[test]
    fn rejects_missing_generated_at() {
        let src = indoc! {r"
            ---
            spec: SPEC-0001
            outcome: delivered
            ---

            body
        "};
        let fx = write_tmp(src);
        let err = report_md(&fx.path).expect_err("missing generated_at must fail");
        assert!(matches!(err, ParseError::Yaml { .. }), "got: {err:?}");
    }

    #[test]
    fn body_is_returned_verbatim() {
        let src = "---\nspec: SPEC-0001\noutcome: delivered\ngenerated_at: 2026-05-11T19:00:00Z\n---\nLine 1\nLine 2\n";
        let fx = write_tmp(src);
        let parsed = report_md(&fx.path).expect("parse should succeed");
        assert_eq!(parsed.body, "Line 1\nLine 2\n");
    }
}
