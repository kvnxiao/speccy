//! SPEC.md parser.
//!
//! Returns frontmatter (validated against the closed `status` set), the
//! list of REQ headings (extracted from comrak's heading nodes so fenced
//! code blocks never poison the result), the `## Changelog` table (if
//! present), and a sha256 over the raw file bytes for staleness
//! detection. See `.speccy/specs/0001-artifact-parsers/SPEC.md` REQ-003.

use crate::error::ParseError;
use crate::parse::frontmatter::Split;
use crate::parse::frontmatter::split as split_frontmatter;
use crate::parse::markdown::inline_text;
use crate::parse::markdown::parse_markdown;
use crate::parse::toml_files::read_to_string;
use camino::Utf8Path;
use comrak::Arena;
use comrak::nodes::AstNode;
use comrak::nodes::NodeValue;
use jiff::civil::Date;
use regex::Regex;
use serde::Deserialize;
use sha2::Digest;
use sha2::Sha256;
use std::sync::OnceLock;

/// Parsed SPEC.md.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecMd {
    /// Deserialised YAML frontmatter.
    pub frontmatter: SpecFrontmatter,
    /// REQ headings extracted from the markdown body, in declared order.
    pub requirements: Vec<ReqHeading>,
    /// Rows of the `## Changelog` table, in declared order. Empty if no
    /// Changelog section is present.
    pub changelog: Vec<ChangelogRow>,
    /// Raw file content as read from disk.
    pub raw: String,
    /// sha256 of the raw file bytes (frontmatter inclusive). Stable across
    /// identical content; differs after any byte edit.
    pub sha256: [u8; 32],
}

/// SPEC.md YAML frontmatter, validated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecFrontmatter {
    /// Stable spec ID (`SPEC-NNNN`).
    pub id: String,
    /// Folder-name slug.
    pub slug: String,
    /// Human-readable title.
    pub title: String,
    /// Lifecycle status (closed set).
    pub status: SpecStatus,
    /// ISO date the spec was first drafted.
    pub created: Date,
    /// IDs of prior specs this one replaces; empty if the field was
    /// omitted in the source.
    pub supersedes: Vec<String>,
}

/// Closed set of spec lifecycle states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecStatus {
    /// Actively being worked on.
    InProgress,
    /// All tasks completed and PR merged.
    Implemented,
    /// Intent abandoned mid-loop.
    Dropped,
    /// A later spec declared `supersedes` pointing here.
    Superseded,
}

impl SpecStatus {
    /// Render back to the on-disk string form (e.g. `in-progress`).
    #[must_use = "the rendered status is the on-disk form"]
    pub const fn as_str(self) -> &'static str {
        match self {
            SpecStatus::InProgress => "in-progress",
            SpecStatus::Implemented => "implemented",
            SpecStatus::Dropped => "dropped",
            SpecStatus::Superseded => "superseded",
        }
    }
}

/// One extracted REQ heading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReqHeading {
    /// `REQ-NNN` identifier.
    pub id: String,
    /// Heading title text after the `REQ-NNN: ` prefix.
    pub title: String,
    /// 1-indexed line number where the heading appears in the source.
    pub line: usize,
}

/// One row of the `## Changelog` table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangelogRow {
    /// `Date` column content (verbatim string; not parsed as a date).
    pub date: String,
    /// `Author` column content.
    pub author: String,
    /// `Summary` column content.
    pub summary: String,
}

#[derive(Debug, Deserialize)]
struct RawFrontmatter {
    id: String,
    slug: String,
    title: String,
    status: String,
    created: Date,
    #[serde(default)]
    supersedes: Vec<String>,
}

const ALLOWED_STATUSES: &[&str] = &["in-progress", "implemented", "dropped", "superseded"];

#[expect(
    clippy::unwrap_used,
    reason = "compile-time literal regex; covered by unit tests"
)]
fn req_id_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"^(REQ-\d{3}):\s*(.*)$").unwrap())
}

/// Parse a SPEC.md file.
///
/// # Errors
///
/// Returns any [`ParseError`] variant relevant to SPEC.md parsing: I/O,
/// non-UTF-8 file content, malformed or absent frontmatter, invalid
/// `status` value, or YAML deserialisation failures.
pub fn spec_md(path: &Utf8Path) -> Result<SpecMd, ParseError> {
    let raw = read_to_string(path)?;
    let sha256 = compute_sha256(raw.as_bytes());

    let frontmatter = parse_frontmatter(&raw, path)?;
    let (requirements, changelog) = parse_body(&raw);

    Ok(SpecMd {
        frontmatter,
        requirements,
        changelog,
        raw,
        sha256,
    })
}

fn compute_sha256(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}

fn parse_frontmatter(raw: &str, path: &Utf8Path) -> Result<SpecFrontmatter, ParseError> {
    let split_result = split_frontmatter(raw, path)?;
    let yaml = match split_result {
        Split::Some { yaml, .. } => yaml,
        Split::None => {
            return Err(ParseError::MissingField {
                field: "frontmatter".to_owned(),
                context: format!("SPEC.md at {path}"),
            });
        }
    };

    let raw_fm: RawFrontmatter = serde_saphyr::from_str(yaml).map_err(|e| ParseError::Yaml {
        label: Some(path.to_string()),
        message: e.to_string(),
    })?;

    let status = parse_status(&raw_fm.status, path)?;

    Ok(SpecFrontmatter {
        id: raw_fm.id,
        slug: raw_fm.slug,
        title: raw_fm.title,
        status,
        created: raw_fm.created,
        supersedes: raw_fm.supersedes,
    })
}

fn parse_status(value: &str, path: &Utf8Path) -> Result<SpecStatus, ParseError> {
    match value {
        "in-progress" => Ok(SpecStatus::InProgress),
        "implemented" => Ok(SpecStatus::Implemented),
        "dropped" => Ok(SpecStatus::Dropped),
        "superseded" => Ok(SpecStatus::Superseded),
        other => Err(ParseError::InvalidEnumValue {
            path: path.to_path_buf(),
            field: "status".to_owned(),
            value: other.to_owned(),
            allowed: ALLOWED_STATUSES.join(", "),
        }),
    }
}

fn parse_body(raw: &str) -> (Vec<ReqHeading>, Vec<ChangelogRow>) {
    let arena = Arena::new();
    let root = parse_markdown(&arena, raw);

    let mut requirements = Vec::new();
    collect_req_headings(root, &mut requirements);

    let changelog = collect_changelog(root);

    (requirements, changelog)
}

fn collect_req_headings<'a>(root: &'a AstNode<'a>, out: &mut Vec<ReqHeading>) {
    for node in root.descendants() {
        let ast = node.data.borrow();
        if matches!(ast.value, NodeValue::Heading(_)) {
            let text = inline_text(node);
            if let Some(caps) = req_id_regex().captures(text.trim()) {
                let id = caps
                    .get(1)
                    .map(|m| m.as_str().to_owned())
                    .unwrap_or_default();
                let title = caps
                    .get(2)
                    .map(|m| m.as_str().trim().to_owned())
                    .unwrap_or_default();
                let line = ast.sourcepos.start.line;
                out.push(ReqHeading { id, title, line });
            }
        }
    }
}

fn collect_changelog<'a>(root: &'a AstNode<'a>) -> Vec<ChangelogRow> {
    let mut found_heading = false;
    let mut rows = Vec::new();

    for node in root.children() {
        let ast = node.data.borrow();
        match &ast.value {
            NodeValue::Heading(h) if h.level == 2 => {
                let text = inline_text(node);
                if text.trim().eq_ignore_ascii_case("Changelog") {
                    found_heading = true;
                } else if found_heading {
                    // A new level-2 heading after Changelog terminates
                    // its scope.
                    break;
                }
            }
            NodeValue::Table(_) if found_heading => {
                drop(ast);
                rows = extract_table_rows(node);
                break;
            }
            _ => {}
        }
    }

    rows
}

fn extract_table_rows<'a>(table: &'a AstNode<'a>) -> Vec<ChangelogRow> {
    let mut rows = Vec::new();
    for row_node in table.children() {
        let row_ast = row_node.data.borrow();
        if let NodeValue::TableRow(is_header) = row_ast.value {
            if is_header {
                continue;
            }
            let cells: Vec<String> = row_node
                .children()
                .map(|cell| inline_text(cell).trim().to_owned())
                .collect();
            let date = cells.first().cloned().unwrap_or_default();
            let author = cells.get(1).cloned().unwrap_or_default();
            let summary = cells.get(2).cloned().unwrap_or_default();
            rows.push(ChangelogRow {
                date,
                author,
                summary,
            });
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::SpecStatus;
    use super::spec_md;
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
        let std_path = dir.path().join("SPEC.md");
        fs_err::write(&std_path, content).expect("writing fixture should succeed");
        let path = Utf8PathBuf::from_path_buf(std_path).expect("tempdir path should be UTF-8");
        Fixture { _dir: dir, path }
    }

    #[test]
    fn parses_frontmatter_and_headings() {
        let src = indoc! {r"
            ---
            id: SPEC-0001
            slug: artifact-parsers
            title: Test
            status: in-progress
            created: 2026-05-11
            ---

            # SPEC-0001: Test

            ## Requirements

            ### REQ-001: First requirement
            Body.

            ### REQ-002: Second
            More body.
        "};
        let fx = write_tmp(src);
        let parsed = spec_md(&fx.path).expect("parse should succeed");
        assert_eq!(parsed.frontmatter.id, "SPEC-0001");
        assert_eq!(parsed.frontmatter.slug, "artifact-parsers");
        assert_eq!(parsed.frontmatter.status, SpecStatus::InProgress);
        assert!(parsed.frontmatter.supersedes.is_empty());
        let req_ids: Vec<&str> = parsed.requirements.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(req_ids, vec!["REQ-001", "REQ-002"]);

        let first_title = parsed
            .requirements
            .first()
            .map(|r| r.title.clone())
            .expect("first requirement");
        assert_eq!(first_title, "First requirement");
    }

    #[test]
    fn skips_req_headings_inside_fenced_code() {
        let src = indoc! {r"
            ---
            id: SPEC-0001
            slug: test
            title: Test
            status: in-progress
            created: 2026-05-11
            ---

            # Test

            Example markdown inside a fenced block:

            ```markdown
            ### REQ-999: Bogus heading that must not be extracted
            ```

            ### REQ-001: Real one
        "};
        let fx = write_tmp(src);
        let parsed = spec_md(&fx.path).expect("parse should succeed");
        let req_ids: Vec<&str> = parsed.requirements.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(req_ids, vec!["REQ-001"]);
    }

    #[test]
    fn missing_frontmatter_errors() {
        let src = "# No frontmatter at all\n";
        let fx = write_tmp(src);
        let err = spec_md(&fx.path).expect_err("missing frontmatter must fail");
        assert!(
            matches!(err, ParseError::MissingField { .. }),
            "got: {err:?}"
        );
    }

    #[test]
    fn invalid_status_errors() {
        let src = indoc! {r"
            ---
            id: SPEC-0001
            slug: x
            title: y
            status: garbage
            created: 2026-05-11
            ---

            body
        "};
        let fx = write_tmp(src);
        let err = spec_md(&fx.path).expect_err("invalid status must fail");
        assert!(
            matches!(
                &err,
                ParseError::InvalidEnumValue { field, value, .. }
                    if field == "status" && value == "garbage"
            ),
            "got: {err:?}",
        );
    }

    #[test]
    fn changelog_table_parses() {
        let src = indoc! {r"
            ---
            id: SPEC-0001
            slug: x
            title: y
            status: in-progress
            created: 2026-05-11
            ---

            ## Changelog

            | Date       | Author       | Summary           |
            |------------|--------------|-------------------|
            | 2026-05-11 | human/kevin  | Initial draft     |
            | 2026-05-12 | agent/claude | bcrypt cost bumped |
        "};
        let fx = write_tmp(src);
        let parsed = spec_md(&fx.path).expect("parse should succeed");
        assert_eq!(parsed.changelog.len(), 2);
        let first = parsed
            .changelog
            .first()
            .expect("at least one changelog row");
        assert_eq!(first.date, "2026-05-11");
        assert_eq!(first.author, "human/kevin");
        assert_eq!(first.summary, "Initial draft");
    }

    #[test]
    fn missing_changelog_yields_empty_vec() {
        let src = indoc! {r"
            ---
            id: SPEC-0001
            slug: x
            title: y
            status: in-progress
            created: 2026-05-11
            ---

            no changelog here
        "};
        let fx = write_tmp(src);
        let parsed = spec_md(&fx.path).expect("parse should succeed");
        assert!(parsed.changelog.is_empty());
    }

    #[test]
    fn sha256_changes_on_one_byte_edit() {
        let src_a = indoc! {r"
            ---
            id: SPEC-0001
            slug: x
            title: y
            status: in-progress
            created: 2026-05-11
            ---

            body content
        "};
        let src_b = indoc! {r"
            ---
            id: SPEC-0001
            slug: x
            title: y
            status: in-progress
            created: 2026-05-11
            ---

            body content!
        "};
        let fx_a = write_tmp(src_a);
        let fx_b = write_tmp(src_b);
        let a = spec_md(&fx_a.path).expect("parse a should succeed");
        let b = spec_md(&fx_b.path).expect("parse b should succeed");
        assert_ne!(a.sha256, b.sha256);
    }

    #[test]
    fn sha256_stable_for_identical_content() {
        let src = indoc! {r"
            ---
            id: SPEC-0001
            slug: x
            title: y
            status: in-progress
            created: 2026-05-11
            ---

            body
        "};
        let fx_a = write_tmp(src);
        let fx_b = write_tmp(src);
        let a = spec_md(&fx_a.path).expect("parse a should succeed");
        let b = spec_md(&fx_b.path).expect("parse b should succeed");
        assert_eq!(a.sha256, b.sha256);
    }
}
