//! `speccy archive SPEC-NNNN` command logic.
//!
//! Relocates a spec directory from `.speccy/specs/NNNN-slug/` to
//! `.speccy/archive/NNNN-slug/` via `git mv`, preserving the canonical
//! directory name. Before moving, edits SPEC.md's YAML frontmatter to
//! append `archived_at: <UTC date>` (unconditional) and, when `--reason`
//! is passed, `archived_reason: "<value>"`.

use crate::check_selector::bare_spec_regex;
use camino::Utf8Path;
use camino::Utf8PathBuf;
use jiff::Zoned;
use serde::Serialize;
use speccy_core::ParseError;
use speccy_core::parse::SpecMd;
use speccy_core::parse::spec_md;
use speccy_core::parse::spec_md::SpecStatus;
use speccy_core::parse::supersession::orphan_candidates_on_archive;
use speccy_core::workspace::WorkspaceError;
use speccy_core::workspace::scan as scan_workspace;
use std::process::Command;
use std::process::Stdio;
use thiserror::Error;

/// Statuses that may be archived without `--force`.
const ARCHIVABLE_STATUSES: &str = "`implemented`, `dropped`, `superseded`";

/// CLI-level error returned by [`run`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ArchiveError {
    /// Walked up from cwd without locating a `.speccy/` directory.
    #[error(".speccy/ directory not found walking up from current directory")]
    ProjectRootNotFound,
    /// I/O failure during workspace discovery.
    #[error("workspace discovery failed")]
    Workspace(#[from] WorkspaceError),
    /// Argument did not match the `SPEC-\d{4,}` shape.
    #[error("invalid SPEC-ID `{arg}`; expected format SPEC-NNNN (4+ digits)")]
    InvalidSpecIdFormat {
        /// The string supplied by the user.
        arg: String,
    },
    /// No spec directory matched the supplied ID under `.speccy/specs/`.
    #[error("spec `{id}` not found under .speccy/specs/")]
    SpecNotFound {
        /// The canonical SPEC ID that was looked up.
        id: String,
    },
    /// SPEC.md parse failure.
    #[error("failed to parse SPEC.md for {id}")]
    SpecMdParse {
        /// The SPEC ID we were trying to parse.
        id: String,
        /// Underlying parser error.
        #[source]
        source: Box<ParseError>,
    },
    /// Status gate refused the archive (status is `in-progress` and
    /// `--force` was not passed).
    #[error(
        "refusing to archive {id}: status is `in-progress`; pass --force to override, or archive only specs with status {allowed}"
    )]
    StatusGate {
        /// The SPEC ID under consideration.
        id: String,
        /// Human-readable list of archivable statuses.
        allowed: &'static str,
    },
    /// `git mv` shell-out failed.
    #[error("git mv failed: {stderr}")]
    GitMvFailed {
        /// Captured stderr from the failing `git mv` invocation.
        stderr: String,
    },
    /// I/O failure while reading or writing SPEC.md.
    #[error("I/O error during archive: {message}")]
    Io {
        /// Human-readable I/O context.
        message: String,
    },
}

/// `speccy archive` arguments.
#[derive(Debug, Clone)]
pub struct ArchiveArgs {
    /// The `SPEC-NNNN` argument (required).
    pub spec_id: String,
    /// Optional `--reason "..."`. Newlines are rejected at clap parse time.
    pub reason: Option<String>,
    /// `--force`: bypass the status gate.
    pub force: bool,
}

/// Successful outcome of an archive run.
#[derive(Debug, Clone)]
pub struct ArchiveOutcome {
    /// Canonical SPEC ID archived.
    pub spec_id: String,
    /// Slug (= original directory name, e.g. `0001-artifact-parsers`).
    pub slug: String,
    /// Original spec directory path, relative to project root.
    pub from: Utf8PathBuf,
    /// New (archive) directory path, relative to project root.
    pub to: Utf8PathBuf,
    /// `YYYY-MM-DD` UTC date recorded in `archived_at`.
    pub archived_at: String,
    /// Reason text, if `--reason` was passed.
    pub archived_reason: Option<String>,
    /// SPEC-NNNN IDs of supersession-chain orphan candidates surfaced
    /// by this archive (sorted). Empty when no orphans fired.
    pub orphan_warnings: Vec<String>,
}

/// Run `speccy archive` from `cwd`.
///
/// # Errors
///
/// Returns any [`ArchiveError`] variant if discovery, ID validation,
/// SPEC.md parsing, the status gate, frontmatter rewrite, or the
/// `git mv` shell-out fails.
pub fn run(args: ArchiveArgs, cwd: &Utf8Path) -> Result<ArchiveOutcome, ArchiveError> {
    let ArchiveArgs {
        spec_id,
        reason,
        force,
    } = args;

    let project_root = crate::cwd::resolve_root(cwd, ArchiveError::ProjectRootNotFound)?;

    let canonical_id = validate_spec_id(&spec_id)?;
    // Scan once, before any file mutation: the supersession-chain orphan
    // detector below needs the source SPEC.md in its pre-move location
    // and pre-mutation state. The scan tolerates parse failures on
    // unrelated specs (ParsedSpec carries Result fields).
    let workspace = scan_workspace(&project_root);
    let spec_dir = workspace
        .spec_dir_by_id(&canonical_id)
        .ok_or_else(|| ArchiveError::SpecNotFound {
            id: canonical_id.clone(),
        })?
        .to_path_buf();
    let spec_md_path = spec_dir.join("SPEC.md");

    let parsed = spec_md(&spec_md_path).map_err(|source| ArchiveError::SpecMdParse {
        id: canonical_id.clone(),
        source,
    })?;

    if matches!(parsed.frontmatter.status, SpecStatus::InProgress) && !force {
        return Err(ArchiveError::StatusGate {
            id: canonical_id,
            allowed: ARCHIVABLE_STATUSES,
        });
    }

    // Read raw bytes (parsed.raw is also available, but we read again to
    // be explicit about the rollback surface — we mutate this file and
    // must be able to restore the original on `git mv` failure).
    let original_bytes =
        fs_err::read_to_string(spec_md_path.as_std_path()).map_err(|e| ArchiveError::Io {
            message: format!("read {spec_md_path}: {e}"),
        })?;

    let active_specs: Vec<&SpecMd> = workspace
        .specs
        .iter()
        .filter_map(|s| s.spec_md.as_ref().ok())
        .collect();
    let orphan_warnings = orphan_candidates_on_archive(&active_specs, &canonical_id);

    let archived_at = today_utc_iso_date();
    let new_bytes = insert_archive_fields(&original_bytes, &archived_at, reason.as_deref());

    // Write mutated SPEC.md back to source path before `git mv`, so the
    // moved file already carries the archive metadata.
    fs_err::write(spec_md_path.as_std_path(), &new_bytes).map_err(|e| ArchiveError::Io {
        message: format!("write {spec_md_path}: {e}"),
    })?;

    // Compute destination: .speccy/archive/<dir-name>/
    let slug = match spec_dir.file_name() {
        Some(name) => name.to_owned(),
        None => canonical_id.clone(),
    };
    let archive_root = project_root.join(".speccy").join("archive");
    let archive_dir = archive_root.join(&slug);

    if let Err(e) = fs_err::create_dir_all(archive_root.as_std_path()) {
        // Roll back frontmatter mutation.
        let _write = fs_err::write(spec_md_path.as_std_path(), original_bytes.as_bytes());
        return Err(ArchiveError::Io {
            message: format!("create {archive_root}: {e}"),
        });
    }

    // `git mv <src> <dst>` run with project_root as cwd.
    let output = Command::new("git")
        .arg("mv")
        .arg(spec_dir.as_std_path())
        .arg(archive_dir.as_std_path())
        .current_dir(project_root.as_std_path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    let output = match output {
        Ok(o) => o,
        Err(e) => {
            let _write = fs_err::write(spec_md_path.as_std_path(), original_bytes.as_bytes());
            return Err(ArchiveError::GitMvFailed {
                stderr: format!("failed to spawn git: {e}"),
            });
        }
    };

    if !output.status.success() {
        // Roll back frontmatter mutation so source SPEC.md returns to
        // pre-archive bytes.
        let _write = fs_err::write(spec_md_path.as_std_path(), original_bytes.as_bytes());
        let stderr_text = String::from_utf8_lossy(&output.stderr).into_owned();
        return Err(ArchiveError::GitMvFailed {
            stderr: stderr_text,
        });
    }

    let from_rel = relativize(&spec_dir, &project_root);
    let to_rel = relativize(&archive_dir, &project_root);

    Ok(ArchiveOutcome {
        spec_id: canonical_id,
        slug,
        from: from_rel,
        to: to_rel,
        archived_at,
        archived_reason: reason,
        orphan_warnings,
    })
}

/// `speccy archive --json` receipt envelope (`schema_version` = 1).
///
/// Stable JSON shape returned on stdout when `--json` is passed and the
/// archive succeeds.
#[derive(Debug, Clone, Serialize)]
pub struct ArchiveReceipt {
    /// Envelope schema version. Always `1` for this revision.
    pub schema_version: u32,
    /// The single archived spec.
    pub archived: ArchivedSpec,
    /// Warnings raised during the archive. Always present; empty when
    /// no warnings fired.
    pub warnings: Vec<ArchiveWarning>,
}

/// Per-spec record inside [`ArchiveReceipt`].
#[derive(Debug, Clone, Serialize)]
pub struct ArchivedSpec {
    /// Canonical `SPEC-NNNN` identifier.
    pub id: String,
    /// Directory slug (the `NNNN-slug` filename component).
    pub slug: String,
    /// Source path relative to the project root (forward slashes).
    pub from: String,
    /// Destination path relative to the project root (forward slashes).
    pub to: String,
    /// `YYYY-MM-DD` date recorded into SPEC.md frontmatter.
    pub archived_at: String,
    /// Reason text from `--reason`, or `null` when omitted. Serialized
    /// as JSON `null` rather than omitted — the key is always present.
    pub archived_reason: Option<String>,
}

/// One warning entry inside [`ArchiveReceipt::warnings`].
///
/// The struct gives the receipt's `warnings` field a concrete element
/// type; the orphan-supersession path is filled in at archive time.
#[derive(Debug, Clone, Serialize)]
pub struct ArchiveWarning {
    /// SPEC-NNNN that the warning is about.
    pub spec: String,
    /// Machine-readable reason code (e.g. `"orphaned-supersession"`).
    pub reason: String,
}

impl ArchiveReceipt {
    /// Build a receipt from a successful [`ArchiveOutcome`].
    ///
    /// Populates [`ArchiveReceipt::warnings`] with one
    /// `orphaned-supersession` entry per SPEC ID in
    /// [`ArchiveOutcome::orphan_warnings`] (already sorted by the
    /// detector). When `orphan_warnings` is empty, `warnings` serializes
    /// as the empty array `[]`.
    #[must_use = "the receipt is the JSON payload to emit on stdout"]
    pub fn from_outcome(outcome: &ArchiveOutcome) -> Self {
        let warnings = outcome
            .orphan_warnings
            .iter()
            .map(|spec| ArchiveWarning {
                spec: spec.clone(),
                reason: "orphaned-supersession".to_owned(),
            })
            .collect();
        Self {
            schema_version: 1,
            archived: ArchivedSpec {
                id: outcome.spec_id.clone(),
                slug: outcome.slug.clone(),
                from: to_forward_slash(outcome.from.as_str()),
                to: to_forward_slash(outcome.to.as_str()),
                archived_at: outcome.archived_at.clone(),
                archived_reason: outcome.archived_reason.clone(),
            },
            warnings,
        }
    }
}

/// Normalize a path string to forward slashes regardless of host OS.
///
/// `Utf8Path` preserves native separators (backslashes on Windows); the
/// `--json` receipt contract requires forward
/// slashes on all platforms.
fn to_forward_slash(s: &str) -> String {
    if std::path::MAIN_SEPARATOR == '/' || !s.contains('\\') {
        s.to_owned()
    } else {
        s.replace('\\', "/")
    }
}

/// Render `path` relative to `root`, falling back to `path` as-is on
/// any decomposition failure. Forward-slash output by `camino` contract.
fn relativize(path: &Utf8Path, root: &Utf8Path) -> Utf8PathBuf {
    match path.strip_prefix(root) {
        Ok(rel) => rel.to_path_buf(),
        Err(_) => path.to_path_buf(),
    }
}

fn validate_spec_id(raw: &str) -> Result<String, ArchiveError> {
    if !bare_spec_regex().is_match(raw) {
        return Err(ArchiveError::InvalidSpecIdFormat {
            arg: raw.to_owned(),
        });
    }
    Ok(raw.to_owned())
}

/// Today's UTC date as `YYYY-MM-DD`.
fn today_utc_iso_date() -> String {
    let now = Zoned::now().with_time_zone(jiff::tz::TimeZone::UTC);
    let d = now.date();
    format!("{:04}-{:02}-{:02}", d.year(), d.month(), d.day())
}

/// Insert `archived_at` and (optionally) `archived_reason` into the YAML
/// frontmatter of a SPEC.md source.
///
/// The new keys are appended after the existing frontmatter keys,
/// immediately before the closing `---` fence. When the source has no
/// frontmatter (defensive only — production callers parse SPEC.md
/// first), the input is returned unchanged.
#[must_use = "the returned string is the mutated SPEC.md bytes"]
pub fn insert_archive_fields(source: &str, archived_at: &str, reason: Option<&str>) -> String {
    // Locate opening fence (start of file) and the closing fence after
    // the frontmatter. We work line-wise to preserve original line
    // endings exactly outside the inserted region.
    let bytes = source.as_bytes();
    if !source.starts_with("---") {
        return source.to_owned();
    }
    // Find first newline after opening `---`.
    let Some(first_nl) = source.find('\n') else {
        return source.to_owned();
    };
    // Find closing `---` line. Scan line by line starting after first_nl.
    let after_open = first_nl.saturating_add(1);
    let mut cursor = after_open;
    let mut close_line_start: Option<usize> = None;
    while cursor < bytes.len() {
        let Some(rest) = source.get(cursor..) else {
            return source.to_owned();
        };
        // Check if this line is the closing fence.
        let line_end = rest
            .find('\n')
            .map_or(rest.len(), |i| cursor.saturating_add(i));
        let Some(line) = source.get(cursor..line_end) else {
            return source.to_owned();
        };
        let trimmed_end = line.trim_end_matches(['\r', ' ', '\t']);
        if trimmed_end == "---" {
            close_line_start = Some(cursor);
            break;
        }
        cursor = line_end.saturating_add(1);
    }
    let Some(close_start) = close_line_start else {
        return source.to_owned();
    };

    // Determine the line ending used right before the closing fence:
    // look at the byte before close_start.
    let line_ending = if close_start >= 2
        && source.get(close_start.saturating_sub(2)..close_start) == Some("\r\n")
    {
        "\r\n"
    } else {
        "\n"
    };

    // Build the inserted lines.
    let mut insertion = String::new();
    insertion.push_str("archived_at: ");
    insertion.push_str(archived_at);
    insertion.push_str(line_ending);
    if let Some(r) = reason {
        insertion.push_str("archived_reason: ");
        insertion.push_str(&yaml_double_quoted(r));
        insertion.push_str(line_ending);
    }

    let mut out = String::with_capacity(source.len().saturating_add(insertion.len()));
    out.push_str(source.get(..close_start).unwrap_or(""));
    out.push_str(&insertion);
    out.push_str(source.get(close_start..).unwrap_or(""));
    out
}

/// Render `value` as a YAML double-quoted scalar with backslash escapes
/// for `"` and `\`. Other ASCII printable chars pass through.
///
/// Callers must reject newlines at argument-parse time; this function
/// does not handle multi-line strings.
fn yaml_double_quoted(value: &str) -> String {
    let mut out = String::with_capacity(value.len().saturating_add(2));
    out.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

/// clap value parser for `--reason`: rejects newline characters.
///
/// # Errors
///
/// Returns a string error suitable for clap's `value_parser` when the
/// supplied string contains `\n` or `\r`.
pub fn parse_reason(s: &str) -> Result<String, String> {
    if s.contains('\n') || s.contains('\r') {
        return Err("--reason must not contain newline characters".to_owned());
    }
    Ok(s.to_owned())
}

#[cfg(test)]
mod tests {
    use super::insert_archive_fields;
    use super::parse_reason;
    use super::yaml_double_quoted;

    const SRC_NO_TRAILING: &str = "---\nid: SPEC-0001\nslug: x\ntitle: T\nstatus: implemented\ncreated: 2026-01-01\nsupersedes: []\n---\n\n# body\n";

    #[test]
    fn insert_archive_fields_appends_unconditional_archived_at() {
        let out = insert_archive_fields(SRC_NO_TRAILING, "2026-05-23", None);
        assert!(
            out.contains("archived_at: 2026-05-23\n"),
            "missing archived_at line: {out}"
        );
        assert!(
            !out.contains("archived_reason:"),
            "reason should be absent when None: {out}"
        );
        // Inserted before closing fence: the closing `---` should still
        // immediately follow the last frontmatter field.
        assert!(out.contains("archived_at: 2026-05-23\n---\n"));
    }

    #[test]
    fn insert_archive_fields_with_reason_appends_both_in_order() {
        let out = insert_archive_fields(SRC_NO_TRAILING, "2026-05-23", Some("shipped 2025-12-15"));
        let archived_at_idx = out.find("archived_at:").expect("archived_at present");
        let reason_idx = out
            .find("archived_reason:")
            .expect("archived_reason present");
        assert!(archived_at_idx < reason_idx);
        assert!(out.contains("archived_reason: \"shipped 2025-12-15\"\n"));
    }

    #[test]
    fn insert_archive_fields_preserves_crlf_line_endings() {
        let crlf = SRC_NO_TRAILING.replace('\n', "\r\n");
        let out = insert_archive_fields(&crlf, "2026-05-23", None);
        assert!(out.contains("archived_at: 2026-05-23\r\n"));
        assert!(out.contains("archived_at: 2026-05-23\r\n---\r\n"));
    }

    #[test]
    fn yaml_double_quoted_escapes_quotes_and_backslashes() {
        assert_eq!(yaml_double_quoted("plain"), "\"plain\"");
        assert_eq!(yaml_double_quoted("a\"b"), "\"a\\\"b\"");
        assert_eq!(yaml_double_quoted("c\\d"), "\"c\\\\d\"");
    }

    #[test]
    fn parse_reason_rejects_newline() {
        parse_reason("ok line").expect("plain text accepted");
        parse_reason("two\nlines").expect_err("LF rejected");
        parse_reason("cr\rok").expect_err("CR rejected");
    }
}
