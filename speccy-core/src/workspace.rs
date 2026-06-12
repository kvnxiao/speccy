//! Workspace scanner.
//!
//! Discovers the speccy project root by walking up from a starting path,
//! enumerates every `NNNN-slug` directory under `.speccy/specs/`, parses
//! each artifact, and assembles a [`Workspace`] that bundles parsed
//! specs with a [`SupersessionIndex`].
//!
//! Per-spec parse failures are non-fatal: each [`ParsedSpec`] carries
//! `Result` fields so a single malformed spec doesn't blind callers to
//! the rest of the workspace.
//!
//! See `.speccy/specs/0004-status-command/SPEC.md` REQ-001..REQ-004.

use crate::error::ParseError;
use crate::error::ParseResult;
use crate::lint::ParsedSpec;
use crate::parse::ReportDoc;
use crate::parse::SpecDoc;
use crate::parse::SpecMd;
use crate::parse::TasksDoc;
use crate::parse::cross_ref::validate_workspace_xml as cross_ref_validate_workspace_xml;
use crate::parse::report_xml;
use crate::parse::spec_md;
use crate::parse::spec_xml;
use crate::parse::supersession::SupersessionIndex;
use crate::parse::supersession::supersession_index;
use crate::parse::task_xml;
use camino::Utf8Path;
use camino::Utf8PathBuf;
use regex::Regex;
use std::sync::OnceLock;
use thiserror::Error;

/// Sentinel string used in TASKS.md frontmatter before SPEC-0006's
/// `speccy tasks --commit` records a real SPEC.md hash.
pub const BOOTSTRAP_PENDING: &str = "bootstrap-pending";

/// Aggregate result of [`scan`].
///
/// Owns every [`ParsedSpec`] discovered under `.speccy/specs/` plus a
/// computed [`SupersessionIndex`]. Reusable by callers that need both
/// rendering data and a borrowed [`crate::lint::Workspace`].
#[derive(Debug)]
pub struct Workspace {
    /// Absolute path to the project root (the directory containing
    /// `.speccy/`).
    pub project_root: Utf8PathBuf,
    /// Every spec directory found under `.speccy/specs/`, in ascending
    /// spec-ID order.
    pub specs: Vec<ParsedSpec>,
    /// Inverse `supersedes` relation computed over successfully-parsed
    /// SPEC.md files.
    pub supersession: SupersessionIndex,
}

impl Workspace {
    /// Borrow this workspace as a [`crate::lint::Workspace`] for lint
    /// integration.
    #[must_use = "the returned view borrows from self for the lint pass"]
    pub fn as_lint_workspace(&self) -> crate::lint::Workspace<'_> {
        crate::lint::Workspace {
            specs: &self.specs,
            supersession: &self.supersession,
        }
    }

    /// Locate the spec directory whose `NNNN-slug` name matches the
    /// digits of `canonical_id` (a `SPEC-NNNN` string). Matches on the
    /// directory name rather than the parsed frontmatter id so a spec
    /// with an unparseable SPEC.md is still located (the caller then
    /// surfaces its own parse diagnostic).
    #[must_use = "callers must handle the not-found case"]
    pub fn spec_dir_by_id(&self, canonical_id: &str) -> Option<&Utf8Path> {
        let digits = canonical_id.strip_prefix("SPEC-")?;
        let prefix = format!("{digits}-");
        self.specs
            .iter()
            .find(|s| {
                s.dir
                    .file_name()
                    .is_some_and(|name| name.starts_with(prefix.as_str()))
            })
            .map(|s| s.dir.as_path())
    }
}

/// Per-spec staleness result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Staleness {
    /// Whether any staleness reason fired.
    pub stale: bool,
    /// Reasons for staleness, in declared order. Empty when `stale` is
    /// `false`.
    pub reasons: Vec<StaleReason>,
}

impl Staleness {
    /// Construct a staleness result reporting no drift.
    #[must_use = "the constructed Staleness value carries the result"]
    pub const fn fresh() -> Self {
        Self {
            stale: false,
            reasons: Vec::new(),
        }
    }
}

/// Reason TASKS.md is considered stale relative to SPEC.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StaleReason {
    /// TASKS.md frontmatter `spec_hash_at_generation` does not equal the
    /// current SPEC.md sha256.
    HashDrift,
    /// TASKS.md frontmatter contains the [`BOOTSTRAP_PENDING`] sentinel.
    BootstrapPending,
}

impl StaleReason {
    /// Render the reason as a short kebab-case string for diagnostics
    /// and JSON output.
    #[must_use = "the rendered name is the on-wire form"]
    pub const fn as_str(self) -> &'static str {
        match self {
            StaleReason::HashDrift => "hash-drift",
            StaleReason::BootstrapPending => "bootstrap-pending",
        }
    }
}

/// Failure mode of [`find_root`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum WorkspaceError {
    /// Walked up to the filesystem root without finding `.speccy/`.
    #[error(".speccy/ directory not found walking up from {start}")]
    NoSpeccyDir {
        /// Starting path the search was launched from.
        start: Utf8PathBuf,
    },
    /// I/O error encountered while inspecting a candidate directory.
    #[error("I/O error during workspace discovery")]
    Io(#[from] std::io::Error),
}

/// Discover the project root by walking up parent directories from
/// `start` until a directory containing `.speccy/` is found.
///
/// # Errors
///
/// Returns [`WorkspaceError::NoSpeccyDir`] if the walk reaches the
/// filesystem root without finding a `.speccy/` directory, or
/// [`WorkspaceError::Io`] if a metadata read fails.
pub fn find_root(start: &Utf8Path) -> Result<Utf8PathBuf, WorkspaceError> {
    let initial = start.to_path_buf();
    let mut current = initial.clone();
    loop {
        let candidate = current.join(".speccy");
        match fs_err::metadata(candidate.as_std_path()) {
            Ok(meta) if meta.is_dir() => return Ok(current),
            Ok(_) => {
                // `.speccy` exists but is a regular file; treat as if
                // absent and keep walking up.
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                // No `.speccy` at this level; continue walking.
            }
            Err(err) => return Err(WorkspaceError::Io(err)),
        }
        match current.parent() {
            Some(parent) if parent != current => current = parent.to_path_buf(),
            _ => return Err(WorkspaceError::NoSpeccyDir { start: initial }),
        }
    }
}

/// Scan `.speccy/specs/` under `project_root` and parse every spec
/// directory whose name matches `^\d{4}-[a-z0-9-]+$`.
///
/// Infallible by design: per-spec parse failures are recorded inside
/// the returned [`ParsedSpec`]s, and a missing or unreadable
/// `.speccy/specs/` directory yields an empty `specs` vec.
///
/// This is the shared active-spec discovery helper for every hot-path
/// command (`status` default mode, `next`, `check`, `verify`, `lock`).
/// It deliberately excludes `.speccy/archive/`: archived specs are
/// invisible to hot-path commands by SPEC-0042 REQ-006. Opt-ins:
///
/// - [`scan_archive_specs`] returns archive specs as a separate slice (used by
///   `status --include-archive`).
/// - [`scan_with_archive`] returns one merged [`Workspace`] containing both
///   active and archive specs (used by `check`, `next`, and `verify` when
///   `--include-archive` is passed, so existing workspace-shaped logic flows
///   through unchanged).
#[must_use = "the returned workspace owns parsed artifacts the caller needs"]
pub fn scan(project_root: &Utf8Path) -> Workspace {
    scan_with_archive(project_root, false)
}

/// Like [`scan`], but optionally merges `.speccy/archive/` into the
/// returned [`Workspace`] as the sole opt-in path for read commands
/// (`check`, `next`, `verify`) that want to see archived specs
/// alongside active ones.
///
/// `include_archive = false` is identical to [`scan`]. When `true`,
/// archive specs are appended after the active list and the
/// supersession index is rebuilt across the merged set, so a
/// `superseded_by` row that points into the archive still resolves.
///
/// The `.speccy/archive/` layout is flat (SPEC-0042 contract), so
/// mission folders are not walked there.
#[must_use = "the returned workspace owns parsed artifacts the caller needs"]
pub fn scan_with_archive(project_root: &Utf8Path, include_archive: bool) -> Workspace {
    let specs_dir = project_root.join(".speccy").join("specs");
    let mut spec_dirs = enumerate_spec_dirs(&specs_dir);
    spec_dirs.sort();

    let mut specs: Vec<ParsedSpec> = spec_dirs.iter().map(|d| parse_spec_dir(d)).collect();

    if include_archive {
        specs.extend(scan_archive_specs(project_root));
    }

    let spec_md_refs: Vec<&SpecMd> = specs
        .iter()
        .filter_map(|s| s.spec_md.as_ref().ok())
        .collect();
    let supersession = supersession_index(&spec_md_refs);

    Workspace {
        project_root: project_root.to_path_buf(),
        specs,
        supersession,
    }
}

/// Compute staleness for one spec's TASKS.md.
///
/// `tasks` is the parsed TASKS.md or `None` if absent.
///
/// The [`StaleReason::BootstrapPending`] sentinel short-circuits the
/// rest of the check: when the frontmatter carries `bootstrap-pending`,
/// it is the sole reason.
#[must_use = "the returned Staleness drives both text and JSON output"]
pub fn stale_for(spec: &SpecMd, tasks: Option<&TasksDoc>) -> Staleness {
    let Some(tasks) = tasks else {
        return Staleness::fresh();
    };

    let stored_hash = extract_frontmatter_field(&tasks.frontmatter_raw, "spec_hash_at_generation")
        .unwrap_or_default();

    if stored_hash == BOOTSTRAP_PENDING {
        return Staleness {
            stale: true,
            reasons: vec![StaleReason::BootstrapPending],
        };
    }

    let mut reasons = Vec::new();

    let current_hash = const_hex::encode(spec.sha256);
    if stored_hash != current_hash {
        reasons.push(StaleReason::HashDrift);
    }

    Staleness {
        stale: !reasons.is_empty(),
        reasons,
    }
}

/// Derive a `SPEC-NNNN` identifier from a spec directory whose name
/// matches `^(\d{4})-[a-z0-9-]+$`.
///
/// Used as a fallback when a spec's SPEC.md fails to parse and the
/// frontmatter ID is unavailable.
#[must_use = "the returned ID identifies the spec for diagnostics and output"]
pub fn derive_spec_id_from_dir(dir: &Utf8Path) -> Option<String> {
    let name = dir.file_name()?;
    let caps = dir_name_regex().captures(name)?;
    let digits = caps.get(1)?.as_str();
    Some(format!("SPEC-{digits}"))
}

/// Count unchecked `- [ ]` items inside any `## Open questions` section
/// of a parsed SPEC.md. The heading match is case-insensitive.
#[must_use = "the count drives the open_questions field in status output"]
pub fn count_open_questions(spec: &SpecMd) -> usize {
    use comrak::Arena;
    use comrak::nodes::NodeValue;

    let arena = Arena::new();
    let root = crate::parse::markdown::parse_markdown(&arena, &spec.raw);

    let mut in_section = false;
    let mut count: usize = 0;
    for node in root.children() {
        let ast = node.data.borrow();
        match &ast.value {
            NodeValue::Heading(h) if h.level == 2 => {
                let text = crate::parse::markdown::inline_text(node);
                in_section = text.trim().eq_ignore_ascii_case("Open questions");
            }
            NodeValue::List(_) if in_section => {
                drop(ast);
                for item in node.children() {
                    let item_ast = item.data.borrow();
                    if !matches!(item_ast.value, NodeValue::Item(_)) {
                        continue;
                    }
                    drop(item_ast);
                    let Some(paragraph) = crate::parse::markdown::first_paragraph_child(item)
                    else {
                        continue;
                    };
                    let text = crate::parse::markdown::inline_text(paragraph);
                    let trimmed = text.trim_start();
                    if let Some(rest) = trimmed.strip_prefix("[ ]")
                        && !rest.trim().is_empty()
                    {
                        count = count.saturating_add(1);
                    }
                }
            }
            _ => {}
        }
    }
    count
}

/// Aggregated task-state counts for one spec's TASKS.md.
///
/// Zeroed when TASKS.md is absent or failed to parse.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TaskCounts {
    /// `pending`: needs work.
    pub open: usize,
    /// `in-progress`: claimed by an implementer.
    pub in_progress: usize,
    /// `in-review`: awaiting review.
    pub awaiting_review: usize,
    /// `completed`: all persona reviews passed.
    pub done: usize,
}

impl TaskCounts {
    /// Build counts from a parsed TASKS.md.
    #[must_use = "the returned counts drive status output"]
    pub fn from_tasks(tasks: &TasksDoc) -> Self {
        use crate::parse::TaskState;
        let mut counts = Self::default();
        for task in &tasks.tasks {
            match task.state {
                TaskState::Pending => counts.open = counts.open.saturating_add(1),
                TaskState::InProgress => {
                    counts.in_progress = counts.in_progress.saturating_add(1);
                }
                TaskState::InReview => {
                    counts.awaiting_review = counts.awaiting_review.saturating_add(1);
                }
                TaskState::Completed => counts.done = counts.done.saturating_add(1),
            }
        }
        counts
    }
}

fn enumerate_spec_dirs(specs_dir: &Utf8Path) -> Vec<Utf8PathBuf> {
    let pattern = dir_name_regex();
    let Ok(entries) = fs_err::read_dir(specs_dir.as_std_path()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if !metadata.is_dir() {
            continue;
        }
        let path = entry.path();
        let Ok(utf8) = Utf8PathBuf::from_path_buf(path) else {
            continue;
        };
        let Some(name) = utf8.file_name() else {
            continue;
        };
        if pattern.is_match(name) {
            // Ungrouped spec directly under `.speccy/specs/`.
            out.push(utf8);
        } else {
            // Treat as a mission (focus) folder. Architecture allows
            // exactly one level of grouping, so scan one level deeper
            // for `NNNN-slug` children. Spec IDs remain globally unique
            // across the workspace; the focus folder is purely a
            // grouping device.
            enumerate_focus_folder(&utf8, pattern, &mut out);
        }
    }
    out
}

fn enumerate_focus_folder(focus_dir: &Utf8Path, pattern: &Regex, out: &mut Vec<Utf8PathBuf>) {
    let Ok(entries) = fs_err::read_dir(focus_dir.as_std_path()) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if !metadata.is_dir() {
            continue;
        }
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !pattern.is_match(name) {
            continue;
        }
        let Ok(utf8) = Utf8PathBuf::from_path_buf(path) else {
            continue;
        };
        out.push(utf8);
    }
}

/// Resolve the `MISSION.md` path for a spec directory, if the spec lives
/// inside a mission (focus) folder.
///
/// A spec is considered mission-grouped when its parent directory is not
/// the `specs/` directory itself but one level above it (the focus folder).
/// The test is structural: if the parent folder's name does not match the
/// `^\d{4}-[a-z0-9-]+$` spec-directory pattern, it is treated as a focus
/// folder and we look for `MISSION.md` inside it.
///
/// Returns `Some(path)` when `<parent>/MISSION.md` exists as a regular
/// file, or `None` when the spec is flat or the file is absent.
fn resolve_mission_md_path(dir: &Utf8Path) -> Option<Utf8PathBuf> {
    let parent = dir.parent()?;
    let parent_name = parent.file_name()?;
    // If the parent name matches the spec-dir pattern it is the `specs/`
    // directory itself (or another spec dir — shouldn't happen), not a
    // focus folder.
    if dir_name_regex().is_match(parent_name) {
        return None;
    }
    let mission_path = parent.join("MISSION.md");
    let is_file = fs_err::metadata(mission_path.as_std_path()).is_ok_and(|m| m.is_file());
    is_file.then_some(mission_path)
}

/// Scan `.speccy/archive/` under `project_root` and parse every
/// archived spec directory whose name matches `^\d{4}-[a-z0-9-]+$`.
///
/// Returns an empty `Vec` when `.speccy/archive/` is absent or
/// unreadable. Per-spec parse failures are recorded inside the
/// returned [`ParsedSpec`]s (same shape as [`scan`] for active specs).
/// Unlike [`scan`], this helper does not walk mission folders: the
/// archive directory layout is flat by SPEC-0042 contract.
///
/// See `.speccy/specs/0042-archive-completed-specs/SPEC.md` REQ-007.
#[must_use = "the returned archive specs are the entire purpose of this call"]
pub fn scan_archive_specs(project_root: &Utf8Path) -> Vec<ParsedSpec> {
    let archive_dir = project_root.join(".speccy").join("archive");
    let pattern = dir_name_regex();
    let Ok(entries) = fs_err::read_dir(archive_dir.as_std_path()) else {
        return Vec::new();
    };
    let mut spec_dirs: Vec<Utf8PathBuf> = Vec::new();
    for entry in entries.flatten() {
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if !metadata.is_dir() {
            continue;
        }
        let Ok(utf8) = Utf8PathBuf::from_path_buf(entry.path()) else {
            continue;
        };
        let Some(name) = utf8.file_name() else {
            continue;
        };
        if pattern.is_match(name) {
            spec_dirs.push(utf8);
        }
    }
    spec_dirs.sort();
    spec_dirs.iter().map(|d| parse_spec_dir(d)).collect()
}

/// Parse one spec directory into a [`ParsedSpec`].
///
/// Used by [`scan`] (active specs) and [`scan_archive_specs`]
/// (archived specs). Per-artifact parse failures are recorded inside
/// the returned struct; this function does not return `Result`.
#[must_use = "the parsed spec carries diagnostics callers must surface"]
pub fn parse_spec_dir(dir: &Utf8Path) -> ParsedSpec {
    let spec_md_path = dir.join("SPEC.md");
    let tasks_md_path = dir.join("TASKS.md");
    let has_tasks = fs_err::metadata(tasks_md_path.as_std_path()).is_ok_and(|m| m.is_file());

    let spec_md_result = spec_md(&spec_md_path);
    let spec_doc_result = parse_spec_doc(&spec_md_path);
    let tasks_md_result = if has_tasks {
        Some(parse_one_tasks_xml(&tasks_md_path))
    } else {
        None
    };

    let report_md_path = dir.join("REPORT.md");
    let has_report = fs_err::metadata(report_md_path.as_std_path()).is_ok_and(|m| m.is_file());
    let report_md_result = if has_report {
        Some(parse_one_report_xml(&report_md_path))
    } else {
        None
    };

    let spec_id = spec_md_result
        .as_ref()
        .ok()
        .map(|s| s.frontmatter.id.clone())
        .or_else(|| derive_spec_id_from_dir(dir));

    let mission_md_path = resolve_mission_md_path(dir);

    ParsedSpec {
        spec_id,
        dir: dir.to_path_buf(),
        spec_md_path,
        tasks_md_path: has_tasks.then_some(tasks_md_path),
        mission_md_path,
        spec_md: spec_md_result,
        spec_doc: spec_doc_result,
        tasks_md: tasks_md_result,
        report_md: report_md_result,
    }
}

/// Extract a top-level YAML scalar field from raw frontmatter, without
/// running a full YAML parse. Returns the trimmed (quote-stripped)
/// value when present, else `None`.
#[must_use = "frontmatter field extraction is the entire purpose"]
pub fn extract_frontmatter_field(yaml: &str, field: &str) -> Option<String> {
    for line in yaml.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            continue;
        }
        let Some(rest) = line.strip_prefix(field) else {
            continue;
        };
        let Some(rest) = rest.strip_prefix(':') else {
            continue;
        };
        let trimmed = rest.trim();
        let stripped = trimmed
            .strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
            .or_else(|| {
                trimmed
                    .strip_prefix('\'')
                    .and_then(|s| s.strip_suffix('\''))
            })
            .unwrap_or(trimmed);
        return Some(stripped.to_owned());
    }
    None
}

/// Parse the typed `SpecDoc` from a SPEC.md path, propagating I/O and
/// parser errors through the existing [`crate::error::ParseError`]
/// channel.
fn parse_spec_doc(spec_md_path: &Utf8Path) -> ParseResult<SpecDoc> {
    let source = crate::parse::fs::read_to_string(spec_md_path)?;
    spec_xml::parse(&source, spec_md_path)
}

/// Inputs to [`validate_workspace_xml`] — the typed XML models for one
/// spec folder and the paths they came from. Paths are used for
/// diagnostics only; this function does no filesystem IO.
#[derive(Debug)]
pub struct XmlValidationInput<'a> {
    /// Parsed SPEC.md element tree for the spec under validation.
    pub spec: &'a SpecDoc,
    /// Parsed TASKS.md element tree, when present.
    pub tasks: Option<&'a TasksDoc>,
    /// Path to TASKS.md, used in diagnostics. Required when
    /// `tasks` is `Some`.
    pub tasks_path: Option<&'a Utf8Path>,
    /// Parsed REPORT.md element tree, when present. When `None`, the
    /// missing-coverage check is skipped (REQ-002 skip rule).
    pub report: Option<&'a ReportDoc>,
    /// Path to REPORT.md, used in diagnostics. Required when
    /// `report` is `Some`.
    pub report_path: Option<&'a Utf8Path>,
}

/// Cross-reference validation entry point reachable from the workspace
/// layer.
///
/// Thin wrapper over [`crate::parse::cross_ref::validate_workspace_xml`]
/// that lets `workspace.rs` own the call site. The wrapper itself does
/// no work beyond forwarding the inputs — the validation logic lives in
/// `cross_ref.rs` so the `ParseError` variants and the SPEC ↔ TASKS /
/// REPORT graph stay adjacent to the existing SPEC-internal cross-ref
/// helper.
#[must_use = "the returned diagnostics are the entire purpose of this call"]
pub fn validate_workspace_xml(input: &XmlValidationInput<'_>) -> Vec<ParseError> {
    cross_ref_validate_workspace_xml(
        input.spec,
        input.tasks,
        input.tasks_path,
        input.report,
        input.report_path,
    )
}

/// Per-spec typed-XML artifact parse results, populated by
/// [`parse_one_spec_xml_artifacts`].
///
/// Each field is `None` if the corresponding file is absent on disk,
/// otherwise `Some(Result<_, Box<ParseError>>)` so call sites can decide
/// whether to surface the parse failure or skip the spec.
#[derive(Debug)]
pub struct SpecXmlArtifacts {
    /// Typed TASKS.md model parse result, or `None` if TASKS.md is absent.
    pub tasks: Option<ParseResult<TasksDoc>>,
    /// Typed REPORT.md model parse result, or `None` if REPORT.md is absent.
    pub report: Option<ParseResult<ReportDoc>>,
}

/// Parse the TASKS.md and REPORT.md typed XML models for one spec
/// folder, when those files exist on disk.
#[must_use = "the returned typed XML results carry parse diagnostics the caller must surface"]
pub fn parse_one_spec_xml_artifacts(spec_dir: &Utf8Path) -> SpecXmlArtifacts {
    let tasks_path = spec_dir.join("TASKS.md");
    let report_path = spec_dir.join("REPORT.md");

    let tasks = if fs_err::metadata(tasks_path.as_std_path()).is_ok_and(|m| m.is_file()) {
        Some(parse_one_tasks_xml(&tasks_path))
    } else {
        None
    };
    let report = if fs_err::metadata(report_path.as_std_path()).is_ok_and(|m| m.is_file()) {
        Some(parse_one_report_xml(&report_path))
    } else {
        None
    };
    SpecXmlArtifacts { tasks, report }
}

fn parse_one_tasks_xml(path: &Utf8Path) -> ParseResult<TasksDoc> {
    let source = crate::parse::fs::read_to_string(path)?;
    task_xml::parse(&source, path)
}

fn parse_one_report_xml(path: &Utf8Path) -> ParseResult<ReportDoc> {
    let source = crate::parse::fs::read_to_string(path)?;
    report_xml::parse(&source, path)
}

#[expect(
    clippy::unwrap_used,
    reason = "compile-time literal regex; covered by unit tests"
)]
fn dir_name_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"^(\d{4})-[a-z0-9-]+$").unwrap())
}

#[cfg(test)]
mod tests {
    use super::derive_spec_id_from_dir;
    use camino::Utf8PathBuf;

    #[test]
    fn derive_spec_id_handles_well_formed_dirs() {
        let dir = Utf8PathBuf::from("/tmp/.speccy/specs/0042-user-signup");
        assert_eq!(derive_spec_id_from_dir(&dir), Some("SPEC-0042".to_owned()));
    }

    #[test]
    fn derive_spec_id_rejects_non_matching_dirs() {
        let dir = Utf8PathBuf::from("/tmp/.speccy/specs/_scratch");
        assert!(derive_spec_id_from_dir(&dir).is_none());
    }

    #[test]
    fn derive_spec_id_rejects_uppercase_slug() {
        let dir = Utf8PathBuf::from("/tmp/.speccy/specs/0001-FOO");
        assert!(derive_spec_id_from_dir(&dir).is_none());
    }

    #[test]
    fn extract_frontmatter_field_returns_unquoted_value() {
        let yaml = "spec: SPEC-0006\nspec_hash_at_generation: abc\n";
        assert_eq!(
            super::extract_frontmatter_field(yaml, "spec").as_deref(),
            Some("SPEC-0006"),
        );
    }

    #[test]
    fn extract_frontmatter_field_strips_double_and_single_quotes() {
        assert_eq!(
            super::extract_frontmatter_field("spec: \"SPEC-0006\"\n", "spec").as_deref(),
            Some("SPEC-0006"),
        );
        assert_eq!(
            super::extract_frontmatter_field("spec: 'SPEC-0006'\n", "spec").as_deref(),
            Some("SPEC-0006"),
        );
    }

    #[test]
    fn extract_frontmatter_field_skips_indented_lines() {
        let yaml = "other:\n  spec: nested-not-matched\nspec: SPEC-0006\n";
        assert_eq!(
            super::extract_frontmatter_field(yaml, "spec").as_deref(),
            Some("SPEC-0006"),
        );
    }

    #[test]
    fn extract_frontmatter_field_returns_none_when_absent() {
        let yaml = "spec_hash_at_generation: x\ngenerated_at: y\n";
        assert!(super::extract_frontmatter_field(yaml, "spec").is_none());
    }
}
