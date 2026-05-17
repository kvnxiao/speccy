//! SPEC.md parser.
//!
//! Returns frontmatter (validated against the closed `status` set), the
//! list of REQ headings (line-based scan that skips fenced code blocks
//! so embedded `### REQ-NNN:` examples never poison the result), the
//! `## Changelog` table (if present), and a sha256 over the SPEC.md's
//! canonical content (frontmatter minus `status`, plus body) for
//! staleness detection. See `.speccy/specs/0001-artifact-parsers/SPEC.md`
//! REQ-003 and `.speccy/specs/0024-meaningful-hash-semantics/SPEC.md`
//! REQ-001.
//!
//! The line-based heading scan was introduced as part of SPEC-0020
//! T-005: after the carrier switched to raw XML element tags
//! (`<requirement>` etc.), comrak parses the body of each element as
//! an opaque raw-HTML block, so headings nested inside the element
//! never surface via `AstNode::descendants`. Line-based scanning over
//! the raw bytes — with code-fence awareness — keeps SPC-002/SPC-003
//! cross-reference working without coupling `spec_md` to `spec_xml`.

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
    /// sha256 of canonical(frontmatter \ {status}) ++ body bytes. Stable
    /// across status flips and frontmatter cosmetics (key order, whitespace,
    /// comments inside the fence); changes on any body byte edit or
    /// non-`status` frontmatter field change. See SPEC-0024 REQ-001.
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

/// Frontmatter fields excluded from the SPEC.md content hash.
///
/// Per SPEC-0024 DEC-002, the default is include-all-fields: adding a
/// new entry here is the only way to make a frontmatter field
/// hash-neutral, and doing so requires a SPEC amendment.
const HASH_EXCLUDED_FRONTMATTER_FIELDS: &[&str] = &["status"];

#[expect(
    clippy::unwrap_used,
    reason = "compile-time literal regex; covered by unit tests"
)]
fn req_heading_line_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"^#{1,6}\s+(REQ-\d{3}):\s*(.*?)\s*$").unwrap())
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
    let frontmatter = parse_frontmatter(&raw, path)?;
    let (requirements, changelog) = parse_body(&raw);
    let sha256 = canonical_content_sha256(&raw, &frontmatter, path)?;

    Ok(SpecMd {
        frontmatter,
        requirements,
        changelog,
        raw,
        sha256,
    })
}

/// Compute `sha256(canonical_frontmatter || body)` for [`SpecMd::sha256`].
///
/// `canonical_frontmatter` is [`canonical_frontmatter_for_hash`]'s output;
/// `body` is the source bytes immediately after the closing `---` fence
/// returned by [`split_frontmatter`]. The split is re-run here (cheap) so
/// the body slice doesn't have to be threaded through `parse_frontmatter`.
fn canonical_content_sha256(
    raw: &str,
    fm: &SpecFrontmatter,
    path: &Utf8Path,
) -> Result<[u8; 32], ParseError> {
    let body = match split_frontmatter(raw, path)? {
        Split::Some { body, .. } => body,
        Split::None => {
            return Err(ParseError::MissingField {
                field: "frontmatter".to_owned(),
                context: format!("SPEC.md at {path}"),
            });
        }
    };
    let canonical_fm = canonical_frontmatter_for_hash(fm);
    let mut hasher = Sha256::new();
    hasher.update(&canonical_fm);
    hasher.update(body.as_bytes());
    Ok(hasher.finalize().into())
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

/// Serialise the parts of [`SpecFrontmatter`] that contribute to the
/// SPEC.md content hash, in a canonical YAML-shaped byte sequence.
///
/// Keys appear in alphabetical order; strings are double-quoted with
/// backslash escapes for `"`, `\`, and ASCII control characters;
/// sequences are emitted in flow style. Fields named in
/// [`HASH_EXCLUDED_FRONTMATTER_FIELDS`] are omitted (`status` today).
/// Two equal [`SpecFrontmatter`] values always produce byte-identical
/// output, so source-file whitespace, key order, and comments are
/// erased by the parse-then-emit round-trip.
///
/// Hand-rolled for the bounded six-field schema rather than going
/// through a generic YAML emitter — keeps determinism a property of
/// this file rather than a dependency's patch-version behaviour. See
/// SPEC-0024 DEC-001.
fn canonical_frontmatter_for_hash(fm: &SpecFrontmatter) -> Vec<u8> {
    let mut out = String::new();
    let push_kv = |out: &mut String, key: &str, emit: &dyn Fn(&mut String)| {
        if HASH_EXCLUDED_FRONTMATTER_FIELDS.contains(&key) {
            return;
        }
        out.push_str(key);
        out.push_str(": ");
        emit(out);
        out.push('\n');
    };

    // Calls listed in alphabetical order. The exclusion check above
    // skips any key in HASH_EXCLUDED_FRONTMATTER_FIELDS, so adding a
    // new exclusion is a single-line edit to that constant.
    push_kv(&mut out, "created", &|out| {
        out.push_str(&fm.created.to_string());
    });
    push_kv(&mut out, "id", &|out| {
        write_yaml_dquoted(out, &fm.id);
    });
    push_kv(&mut out, "slug", &|out| {
        write_yaml_dquoted(out, &fm.slug);
    });
    push_kv(&mut out, "status", &|out| {
        write_yaml_dquoted(out, fm.status.as_str());
    });
    push_kv(&mut out, "supersedes", &|out| {
        out.push('[');
        for (idx, item) in fm.supersedes.iter().enumerate() {
            if idx > 0 {
                out.push_str(", ");
            }
            write_yaml_dquoted(out, item);
        }
        out.push(']');
    });
    push_kv(&mut out, "title", &|out| {
        write_yaml_dquoted(out, &fm.title);
    });

    out.into_bytes()
}

/// Append `s` to `out` as a YAML double-quoted scalar, escaping `"`,
/// `\`, and ASCII control characters so the output is a function of
/// the string's logical content alone.
fn write_yaml_dquoted(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let n = u32::from(c);
                out.push_str("\\u00");
                out.push(hex_nibble((n >> 4) & 0xF));
                out.push(hex_nibble(n & 0xF));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Render the low nibble of `n` as a lowercase hex digit.
fn hex_nibble(n: u32) -> char {
    char::from_digit(n & 0xF, 16).unwrap_or('0')
}

fn parse_body(raw: &str) -> (Vec<ReqHeading>, Vec<ChangelogRow>) {
    let arena = Arena::new();
    let root = parse_markdown(&arena, raw);

    let code_fence_lines = collect_code_fence_line_ranges(root);
    let mut requirements = Vec::new();
    collect_req_headings_line_based(raw, &code_fence_lines, &mut requirements);

    let changelog = collect_changelog(root);

    (requirements, changelog)
}

/// Collect 1-indexed `(start_line, end_line)` ranges for every fenced
/// code block in the source. Inclusive on both ends. Used by
/// [`collect_req_headings_line_based`] to skip example REQ headings
/// embedded inside code blocks.
fn collect_code_fence_line_ranges<'a>(root: &'a AstNode<'a>) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for node in root.descendants() {
        let ast = node.data.borrow();
        if let NodeValue::CodeBlock(_) = &ast.value {
            let start = ast.sourcepos.start.line;
            let end = ast.sourcepos.end.line;
            out.push((start, end));
        }
    }
    out
}

/// Scan `raw` line-by-line for `### REQ-NNN: title` headings (any
/// heading level 1..6). Skip lines that fall inside any fenced code
/// block listed in `code_fence_lines`. This decouples REQ-heading
/// discovery from comrak's HTML-block opacity, which was introduced
/// when SPEC-0020 swapped the SPEC.md carrier from HTML-comment
/// markers (transparent to comrak) to raw XML element tags (treated
/// as opaque raw-HTML blocks).
fn collect_req_headings_line_based(
    raw: &str,
    code_fence_lines: &[(usize, usize)],
    out: &mut Vec<ReqHeading>,
) {
    let in_code_fence = |line_1: usize| -> bool {
        code_fence_lines
            .iter()
            .any(|&(s, e)| line_1 >= s && line_1 <= e)
    };
    for (idx, line) in raw.lines().enumerate() {
        let line_1 = idx.saturating_add(1);
        if in_code_fence(line_1) {
            continue;
        }
        let Some(caps) = req_heading_line_regex().captures(line) else {
            continue;
        };
        let id = caps
            .get(1)
            .map(|m| m.as_str().to_owned())
            .unwrap_or_default();
        let title = caps
            .get(2)
            .map(|m| m.as_str().trim().to_owned())
            .unwrap_or_default();
        out.push(ReqHeading {
            id,
            title,
            line: line_1,
        });
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
    use super::HASH_EXCLUDED_FRONTMATTER_FIELDS;
    use super::SpecFrontmatter;
    use super::SpecStatus;
    use super::canonical_frontmatter_for_hash;
    use super::spec_md;
    use crate::error::ParseError;
    use camino::Utf8PathBuf;
    use indoc::indoc;
    use jiff::civil::Date;
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

    fn sample_frontmatter() -> SpecFrontmatter {
        SpecFrontmatter {
            id: "SPEC-0001".to_owned(),
            slug: "artifact-parsers".to_owned(),
            title: "Test".to_owned(),
            status: SpecStatus::InProgress,
            created: Date::new(2026, 5, 11).expect("valid date"),
            supersedes: vec![],
        }
    }

    #[test]
    fn hash_excluded_frontmatter_fields_contains_only_status() {
        assert_eq!(HASH_EXCLUDED_FRONTMATTER_FIELDS, &["status"]);
    }

    #[test]
    fn canonical_frontmatter_is_deterministic() {
        let fm = sample_frontmatter();
        let a = canonical_frontmatter_for_hash(&fm);
        let b = canonical_frontmatter_for_hash(&fm);
        assert_eq!(a, b);
    }

    #[test]
    fn canonical_frontmatter_omits_status_field() {
        let fm = sample_frontmatter();
        let bytes = canonical_frontmatter_for_hash(&fm);
        let text = String::from_utf8(bytes).expect("canonical output is UTF-8");
        assert!(
            !text.contains("status:"),
            "canonical output must not emit a `status:` key, got:\n{text}",
        );
        assert!(
            !text.contains("in-progress"),
            "canonical output must not leak the status value, got:\n{text}",
        );
    }

    #[test]
    fn canonical_frontmatter_invariant_under_status_flip() {
        let mut in_progress = sample_frontmatter();
        in_progress.status = SpecStatus::InProgress;
        let mut implemented = sample_frontmatter();
        implemented.status = SpecStatus::Implemented;
        let a = canonical_frontmatter_for_hash(&in_progress);
        let b = canonical_frontmatter_for_hash(&implemented);
        assert_eq!(
            a, b,
            "status-only difference must not perturb the canonical output"
        );
    }

    #[test]
    fn canonical_frontmatter_keys_in_alphabetical_order() {
        let fm = sample_frontmatter();
        let bytes = canonical_frontmatter_for_hash(&fm);
        let text = String::from_utf8(bytes).expect("canonical output is UTF-8");
        let created_pos = text
            .find("created:")
            .expect("canonical output should contain `created:`");
        let id_pos = text
            .find("id:")
            .expect("canonical output should contain `id:`");
        let slug_pos = text
            .find("slug:")
            .expect("canonical output should contain `slug:`");
        let supersedes_pos = text
            .find("supersedes:")
            .expect("canonical output should contain `supersedes:`");
        let title_pos = text
            .find("title:")
            .expect("canonical output should contain `title:`");
        assert!(
            created_pos < id_pos
                && id_pos < slug_pos
                && slug_pos < supersedes_pos
                && supersedes_pos < title_pos,
            "keys must appear in alphabetical order, got:\n{text}",
        );
    }

    #[test]
    fn canonical_frontmatter_changes_with_non_status_fields() {
        let baseline = sample_frontmatter();
        let baseline_bytes = canonical_frontmatter_for_hash(&baseline);

        let mut alt_id = sample_frontmatter();
        alt_id.id = "SPEC-9999".to_owned();
        assert_ne!(
            baseline_bytes,
            canonical_frontmatter_for_hash(&alt_id),
            "changing `id` must perturb the canonical output",
        );

        let mut alt_slug = sample_frontmatter();
        alt_slug.slug = "different".to_owned();
        assert_ne!(
            baseline_bytes,
            canonical_frontmatter_for_hash(&alt_slug),
            "changing `slug` must perturb the canonical output",
        );

        let mut alt_title = sample_frontmatter();
        alt_title.title = "Different title".to_owned();
        assert_ne!(
            baseline_bytes,
            canonical_frontmatter_for_hash(&alt_title),
            "changing `title` must perturb the canonical output",
        );

        let mut alt_created = sample_frontmatter();
        alt_created.created = Date::new(2026, 5, 12).expect("valid date");
        assert_ne!(
            baseline_bytes,
            canonical_frontmatter_for_hash(&alt_created),
            "changing `created` must perturb the canonical output",
        );

        let mut alt_supersedes = sample_frontmatter();
        alt_supersedes.supersedes = vec!["SPEC-0000".to_owned()];
        assert_ne!(
            baseline_bytes,
            canonical_frontmatter_for_hash(&alt_supersedes),
            "changing `supersedes` must perturb the canonical output",
        );
    }

    #[test]
    fn canonical_frontmatter_equates_explicit_and_default_empty_supersedes() {
        let src_with_explicit = indoc! {r"
            ---
            id: SPEC-0001
            slug: x
            title: y
            status: in-progress
            created: 2026-05-11
            supersedes: []
            ---

            body
        "};
        let src_default = indoc! {r"
            ---
            id: SPEC-0001
            slug: x
            title: y
            status: in-progress
            created: 2026-05-11
            ---

            body
        "};
        let fx_a = write_tmp(src_with_explicit);
        let fx_b = write_tmp(src_default);
        let parsed_a = spec_md(&fx_a.path).expect("parse a should succeed");
        let parsed_b = spec_md(&fx_b.path).expect("parse b should succeed");
        assert_eq!(
            parsed_a.frontmatter, parsed_b.frontmatter,
            "default and explicit empty `supersedes:` must parse equal",
        );
        let a = canonical_frontmatter_for_hash(&parsed_a.frontmatter);
        let b = canonical_frontmatter_for_hash(&parsed_b.frontmatter);
        assert_eq!(
            a, b,
            "canonical output must be identical for default vs explicit empty `supersedes:`",
        );
    }

    #[test]
    fn canonical_frontmatter_invariant_under_source_key_reordering() {
        let src_original = indoc! {r"
            ---
            id: SPEC-0001
            slug: x
            title: y
            status: in-progress
            created: 2026-05-11
            ---

            body
        "};
        let src_reordered = indoc! {r"
            ---
            slug: x
            created: 2026-05-11
            title: y
            id: SPEC-0001
            status: in-progress
            ---

            body
        "};
        let fx_a = write_tmp(src_original);
        let fx_b = write_tmp(src_reordered);
        let parsed_a = spec_md(&fx_a.path).expect("parse original should succeed");
        let parsed_b = spec_md(&fx_b.path).expect("parse reordered should succeed");
        let a = canonical_frontmatter_for_hash(&parsed_a.frontmatter);
        let b = canonical_frontmatter_for_hash(&parsed_b.frontmatter);
        assert_eq!(
            a, b,
            "source-file key reordering must not perturb the canonical output",
        );
    }

    #[test]
    fn spec_md_sha256_invariant_under_status_flip() {
        let in_progress = indoc! {r"
            ---
            id: SPEC-0001
            slug: x
            title: y
            status: in-progress
            created: 2026-05-11
            ---

            body
        "};
        let implemented = indoc! {r"
            ---
            id: SPEC-0001
            slug: x
            title: y
            status: implemented
            created: 2026-05-11
            ---

            body
        "};
        let fx_a = write_tmp(in_progress);
        let fx_b = write_tmp(implemented);
        let a = spec_md(&fx_a.path).expect("parse in-progress should succeed");
        let b = spec_md(&fx_b.path).expect("parse implemented should succeed");
        assert_eq!(
            a.sha256, b.sha256,
            "flipping `status:` must not perturb SpecMd.sha256",
        );
    }

    #[test]
    fn spec_md_sha256_invariant_under_source_key_reordering() {
        let original = indoc! {r"
            ---
            id: SPEC-0001
            slug: x
            title: y
            status: in-progress
            created: 2026-05-11
            ---

            body
        "};
        let reordered = indoc! {r"
            ---
            slug: x
            created: 2026-05-11
            title: y
            id: SPEC-0001
            status: in-progress
            ---

            body
        "};
        let fx_a = write_tmp(original);
        let fx_b = write_tmp(reordered);
        let a = spec_md(&fx_a.path).expect("parse original should succeed");
        let b = spec_md(&fx_b.path).expect("parse reordered should succeed");
        assert_eq!(
            a.sha256, b.sha256,
            "source-file key reordering must not perturb SpecMd.sha256",
        );
    }

    #[test]
    fn spec_md_sha256_changes_when_id_changes() {
        let base = indoc! {r"
            ---
            id: SPEC-0001
            slug: x
            title: y
            status: in-progress
            created: 2026-05-11
            ---

            body
        "};
        let alt = indoc! {r"
            ---
            id: SPEC-9999
            slug: x
            title: y
            status: in-progress
            created: 2026-05-11
            ---

            body
        "};
        let fx_a = write_tmp(base);
        let fx_b = write_tmp(alt);
        let a = spec_md(&fx_a.path).expect("parse base should succeed");
        let b = spec_md(&fx_b.path).expect("parse alt should succeed");
        assert_ne!(
            a.sha256, b.sha256,
            "changing `id:` must perturb SpecMd.sha256"
        );
    }

    #[test]
    fn spec_md_sha256_changes_when_slug_changes() {
        let base = indoc! {r"
            ---
            id: SPEC-0001
            slug: original
            title: y
            status: in-progress
            created: 2026-05-11
            ---

            body
        "};
        let alt = indoc! {r"
            ---
            id: SPEC-0001
            slug: different
            title: y
            status: in-progress
            created: 2026-05-11
            ---

            body
        "};
        let fx_a = write_tmp(base);
        let fx_b = write_tmp(alt);
        let a = spec_md(&fx_a.path).expect("parse base should succeed");
        let b = spec_md(&fx_b.path).expect("parse alt should succeed");
        assert_ne!(
            a.sha256, b.sha256,
            "changing `slug:` must perturb SpecMd.sha256"
        );
    }

    #[test]
    fn spec_md_sha256_changes_when_title_changes() {
        let base = indoc! {r"
            ---
            id: SPEC-0001
            slug: x
            title: First title
            status: in-progress
            created: 2026-05-11
            ---

            body
        "};
        let alt = indoc! {r"
            ---
            id: SPEC-0001
            slug: x
            title: Second title
            status: in-progress
            created: 2026-05-11
            ---

            body
        "};
        let fx_a = write_tmp(base);
        let fx_b = write_tmp(alt);
        let a = spec_md(&fx_a.path).expect("parse base should succeed");
        let b = spec_md(&fx_b.path).expect("parse alt should succeed");
        assert_ne!(
            a.sha256, b.sha256,
            "changing `title:` must perturb SpecMd.sha256"
        );
    }

    #[test]
    fn spec_md_sha256_changes_when_created_changes() {
        let base = indoc! {r"
            ---
            id: SPEC-0001
            slug: x
            title: y
            status: in-progress
            created: 2026-05-11
            ---

            body
        "};
        let alt = indoc! {r"
            ---
            id: SPEC-0001
            slug: x
            title: y
            status: in-progress
            created: 2026-05-12
            ---

            body
        "};
        let fx_a = write_tmp(base);
        let fx_b = write_tmp(alt);
        let a = spec_md(&fx_a.path).expect("parse base should succeed");
        let b = spec_md(&fx_b.path).expect("parse alt should succeed");
        assert_ne!(
            a.sha256, b.sha256,
            "changing `created:` must perturb SpecMd.sha256"
        );
    }

    #[test]
    fn spec_md_sha256_changes_when_supersedes_changes() {
        let base = indoc! {r"
            ---
            id: SPEC-0001
            slug: x
            title: y
            status: in-progress
            created: 2026-05-11
            ---

            body
        "};
        let alt = indoc! {r"
            ---
            id: SPEC-0001
            slug: x
            title: y
            status: in-progress
            created: 2026-05-11
            supersedes: [SPEC-0000]
            ---

            body
        "};
        let fx_a = write_tmp(base);
        let fx_b = write_tmp(alt);
        let a = spec_md(&fx_a.path).expect("parse base should succeed");
        let b = spec_md(&fx_b.path).expect("parse alt should succeed");
        assert_ne!(
            a.sha256, b.sha256,
            "changing `supersedes:` must perturb SpecMd.sha256",
        );
    }
}
