//! Shared fixtures for `speccy status` integration tests.
//!
//! Each integration test binary compiles this module independently and
//! uses only a subset of the helpers. The module-level expect below
//! silences dead-code warnings in test binaries that exercise only a
//! subset; a deliberately-unused [`touch_for_dead_code_expect`] function
//! guarantees the expectation is fulfilled in every binary.

#![expect(
    dead_code,
    reason = "shared test helpers; each test binary uses only a subset"
)]

use camino::Utf8Path;
use camino::Utf8PathBuf;
use indoc::indoc;
use std::fmt::Write as _;
use tempfile::TempDir;

pub type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub struct Workspace {
    pub _dir: TempDir,
    pub root: Utf8PathBuf,
}

impl Workspace {
    pub fn new() -> TestResult<Self> {
        let dir = tempfile::tempdir()?;
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf())
            .map_err(|p| format!("tempdir path must be UTF-8: {}", p.display()))?;
        fs_err::create_dir_all(root.join(".speccy").as_std_path())?;
        Ok(Workspace { _dir: dir, root })
    }
}

pub fn write_spec(
    root: &Utf8Path,
    dir_name: &str,
    spec_md: &str,
    tasks_md: Option<&str>,
) -> TestResult<Utf8PathBuf> {
    let dir = root.join(".speccy").join("specs").join(dir_name);
    fs_err::create_dir_all(dir.as_std_path())?;
    fs_err::write(dir.join("SPEC.md").as_std_path(), spec_md)?;
    if let Some(tm) = tasks_md {
        fs_err::write(dir.join("TASKS.md").as_std_path(), tm)?;
    }
    Ok(dir)
}

pub fn spec_md_template(id: &str, status: &str) -> String {
    let template = indoc! {r#"
        ---
        id: __ID__
        slug: x
        title: Example __ID__
        status: __STATUS__
        created: 2026-05-11
        ---

        # __ID__

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
        Body.

        <done-when>
        - placeholder.
        </done-when>

        <behavior>
        - placeholder.
        </behavior>

        <scenario id="CHK-001">
        Given REQ-001, when the suite runs, then it covers REQ-001.
        </scenario>
        </requirement>

        ## Changelog

        <changelog>
        | Date | Author | Summary |
        |------|--------|---------|
        | 2026-05-11 | t | init |
        </changelog>
    "#};
    template.replace("__ID__", id).replace("__STATUS__", status)
}

pub fn spec_md_with_open_questions(id: &str, status: &str, questions: usize) -> String {
    let base = spec_md_template(id, status);
    // Inject the Open questions section before the changelog so the
    // marker parser still sees the required `speccy:changelog` block.
    let marker = "## Changelog";
    let split_idx = base.find(marker).unwrap_or(base.len());
    let (before, after) = base.split_at(split_idx);
    let mut s = String::from(before);
    s.push_str("## Open questions\n\n");
    for i in 0..questions {
        if writeln!(s, "- [ ] open question {i}").is_err() {
            break;
        }
    }
    s.push('\n');
    s.push_str(after);
    s
}

/// Deliberately-unused helper. Each integration test binary uses only
/// a subset of this module's helpers; this function is never called,
/// guaranteeing the module-level `expect(dead_code)` is fulfilled.
pub fn touch_for_dead_code_expect() {
    let _ = indoc! {""};
}

pub fn bootstrap_tasks_md(spec_id: &str) -> String {
    format!(
        "---\nspec: {spec_id}\nspec_hash_at_generation: bootstrap-pending\ngenerated_at: 2026-05-11T00:00:00Z\n---\n\n# Tasks: {spec_id}\n\n\n\n<task id=\"T-001\" state=\"pending\" covers=\"REQ-001\">\nfirst\n\n<task-scenarios>\n- placeholder.\n</task-scenarios>\n</task>\n",
    )
}

/// Lowercase hex SHA-256 of the given bytes. Mirrors the encoding the
/// production code uses for `tasks_hash` in VET.md gate blocks.
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest as _;
    const_hex::encode(sha2::Sha256::digest(bytes))
}

/// Wrap a `<task>` element body in the TASKS.md frontmatter shape with
/// a bootstrap-pending spec hash. Used by every `next_*` integration
/// test that needs a minimum-viable TASKS.md.
pub fn tasks_md_xml(spec_id: &str, tasks_xml: &str) -> String {
    format!(
        "---\nspec: {spec_id}\nspec_hash_at_generation: bootstrap-pending\ngenerated_at: 2026-05-11T00:00:00Z\n---\n\n# Tasks: {spec_id}\n\n\n\n{tasks_xml}\n\n",
    )
}

/// Render a single `<task>` element body covering REQ-001 with a
/// placeholder scenarios block.
pub fn task_xml(id: &str, state: &str) -> String {
    format!(
        "<task id=\"{id}\" state=\"{state}\" covers=\"REQ-001\">\ndo the thing\n\n<task-scenarios>\n- placeholder.\n</task-scenarios>\n</task>\n\n",
    )
}

/// Count non-blank lines in `s`, excluding any line that falls within a
/// `reconcile-policy` shared-partial marker block or a
/// `retry-shape` shared-rule marker block. SPEC-0045/REQ-008 inlines
/// the reconcile partial verbatim into phase-worker SKILL.md stubs;
/// SPEC-0047/REQ-002 inlines the retry-shape rule into the
/// `/speccy-work` and `/speccy-orchestrate` skill bodies. Both
/// marker-bounded regions are explicit, auditable exemptions from
/// the "thin stub" non-empty-line cap. For each marker type, uses
/// the first open marker and the last close marker as the boundary
/// so the inlined content's own illustrative inner markers (inside
/// fenced code blocks) stay inside the block.
pub fn non_blank_line_count_outside_shared_markers(s: &str) -> usize {
    let exemptions: [(&str, &str); 2] = [
        (
            "<!-- Shared partial: reconcile-policy.",
            "<!-- End shared partial: reconcile-policy. -->",
        ),
        (
            "<!-- Shared rule: retry-shape.",
            "<!-- End shared rule: retry-shape. -->",
        ),
    ];
    let lines: Vec<&str> = s.lines().collect();
    let ranges: Vec<(usize, usize)> = exemptions
        .iter()
        .filter_map(|(open_marker, close_marker)| {
            let open_idx = lines
                .iter()
                .position(|l| l.trim().starts_with(open_marker))?;
            let close_idx = lines
                .iter()
                .rposition(|l| l.trim().starts_with(close_marker))?;
            Some((open_idx, close_idx))
        })
        .collect();
    let mut count = 0usize;
    for (idx, line) in lines.iter().enumerate() {
        if ranges.iter().any(|(o, c)| idx >= *o && idx <= *c) {
            continue;
        }
        if !line.trim().is_empty() {
            count += 1;
        }
    }
    count
}

/// Render a single-invocation VET.md for `spec_id` whose terminal `<gate>`
/// carries `verdict` and `tasks_hash`, built entirely from the exported
/// production renderers so the fixture matches the real grammar by
/// construction (SPEC-0061 REQ-004 / DEC-004). `extra_body_line`, when
/// `Some`, is appended to the gate body — used by the spoof fixture that
/// embeds a line-isolated fake `<gate>` inside a block body. `leading`
/// blocks (already rendered via [`render_vet_block`]) are emitted in the
/// invocation section before the terminal gate — used by the fixture that
/// asserts a `drift-review` then `gate` read back.
///
/// `verdict` must be in the gate domain (`passed` / `failed`); an
/// out-of-domain value is a test bug and surfaces as a renderer error.
pub fn render_vet_md(
    spec_id: &str,
    verdict: &str,
    tasks_hash: &str,
    extra_body_line: Option<&str>,
    leading: &[String],
) -> TestResult<String> {
    use speccy_core::parse::render_fresh_vet_frontmatter;
    use speccy_core::parse::render_vet_section_heading;

    let date = "2026-05-22T00:00:00Z";
    let mut body = String::from("stub.");
    if let Some(line) = extra_body_line {
        body.push('\n');
        body.push_str(line);
    }
    let gate = render_vet_block(&VetTestBlock::Gate {
        verdict,
        tasks_hash,
        body: &body,
    })?;
    let frontmatter = render_fresh_vet_frontmatter(spec_id, date);
    let heading = render_vet_section_heading(1, date);
    let mut doc = format!("{frontmatter}{heading}");
    for block in leading {
        doc.push_str(block);
        doc.push('\n');
    }
    doc.push_str(&gate);
    Ok(doc)
}

/// A vet block to render through the production renderer for test fixtures.
#[derive(Clone, Copy)]
pub enum VetTestBlock<'a> {
    /// A `<drift-review>` block with the given verdict and round.
    DriftReview {
        /// `pass` or `blocking`.
        verdict: &'a str,
        /// Round counter.
        round: u32,
    },
    /// A terminal `<gate>` block.
    Gate {
        /// `passed` or `failed`.
        verdict: &'a str,
        /// Lowercase hex SHA-256 of the sibling TASKS.md.
        tasks_hash: &'a str,
        /// Block body.
        body: &'a str,
    },
}

/// Render one vet block via the production [`validate_and_render_vet_block`]
/// renderer, so test fixtures cannot diverge from the real grammar.
pub fn render_vet_block(block: &VetTestBlock<'_>) -> TestResult<String> {
    use speccy_core::parse::VetBlockInputs;
    use speccy_core::parse::VetBlockKind;
    use speccy_core::parse::validate_and_render_vet_block;

    let date = "2026-05-22T00:00:00Z";
    let inputs = match *block {
        VetTestBlock::DriftReview { verdict, round } => VetBlockInputs {
            kind: VetBlockKind::DriftReview,
            date,
            round,
            verdict: Some(verdict),
            model: Some("m"),
            tasks_hash: None,
            body: "drift body",
        },
        VetTestBlock::Gate {
            verdict,
            tasks_hash,
            body,
        } => VetBlockInputs {
            kind: VetBlockKind::Gate,
            date,
            round: 1,
            verdict: Some(verdict),
            model: None,
            tasks_hash: Some(tasks_hash),
            body,
        },
    };
    Ok(validate_and_render_vet_block(&inputs)?)
}

/// Write a single-invocation `journal/VET.md` for `spec_id` whose terminal
/// gate carries `verdict` and `tasks_hash`, via [`render_vet_md`].
pub fn write_vet_md(
    spec_dir: &Utf8Path,
    spec_id: &str,
    verdict: &str,
    tasks_hash: &str,
) -> TestResult {
    let journal = spec_dir.join("journal");
    fs_err::create_dir_all(journal.as_std_path())?;
    let body = render_vet_md(spec_id, verdict, tasks_hash, None, &[])?;
    fs_err::write(journal.join("VET.md").as_std_path(), body)?;
    Ok(())
}

/// Write a fresh, passing `journal/VET.md` whose `tasks_hash` matches
/// the supplied TASKS.md bytes. Drives the SPEC-0041 fresh-pass gate
/// branch in `speccy next`. Delegates to [`write_vet_md`].
pub fn write_fresh_pass_vet_md(spec_dir: &Utf8Path, spec_id: &str, tasks_md: &str) -> TestResult {
    let hash = sha256_hex(tasks_md.as_bytes());
    write_vet_md(spec_dir, spec_id, "passed", &hash)
}
