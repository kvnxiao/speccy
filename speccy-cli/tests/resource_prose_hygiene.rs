#![expect(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
//! Resource-prose hygiene lint: bounds internal artifact-ID provenance in
//! the agent-facing module bodies under `resources/modules/`.
//!
//! Non-reference bodies — skills, phases, personas, partials, and shared
//! rule files — must use only the generic placeholder ids (`SPEC-NNNN`,
//! `REQ-NNN`, `DEC-NNN`, `T-NNN`, `CHK-NNN`). A real Speccy
//! requirement / decision / task id cited as provenance is pure noise in
//! another repo, and an invitation to hallucinate.
//!
//! Worked-instance references under `resources/modules/references/` are
//! the carve-out. They carry one concrete, load-bearing example — the
//! `SPEC-0042` widget-render-timeout walkthrough — so concrete
//! `REQ` / `DEC` / `CHK` / `T` ids and the whitelisted `SPEC-0042` are
//! allowed there: `<task id="T-001" covers="REQ-001 REQ-002">` reads as a
//! worked example, where `T-NNN covers="REQ-NNN REQ-NNN"` reads as a
//! blank. Two bans still apply in references: any SPEC id other than the
//! exact `SPEC-0042`, and CLI lint codes (`TSK-` / `JNL-`) cited by number
//! rather than described by behavior.
//!
//! Source-only scan: the dogfood byte-identity test
//! (`tests/init.rs::dogfood_outputs_match_committed_tree`) already proves
//! eject == source, so scanning the ejected `.claude/` / `.codex/` / `.agents/`
//! trees would only double-count. `.speccy/specs/` is outside `resources/` and
//! is never walked — Speccy's own dogfood artifacts stay Speccy-specific.

use regex::Regex;
use std::path::Path;
use std::path::PathBuf;
use std::sync::OnceLock;

/// Workspace root, derived from `CARGO_MANIFEST_DIR` (the `speccy-cli` crate
/// dir) by walking one level up. Mirrors the helper in the sibling test
/// crates so this scan reads the on-disk canonical sources under
/// `resources/modules/`.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf(),
        Path::to_path_buf,
    )
}

/// `SPEC-NNNN`-shaped ids carrying a real digit run; every match except the
/// whitelisted `SPEC-0042` is a violation, references included. The generic
/// letter-form `SPEC-NNNN` has no digit run after the dash, so it never
/// matches.
fn spec_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"\bSPEC-\d{4,}\b").expect("valid SPEC id regex"))
}

/// Artifact-family ids (`REQ` / `DEC` / `CHK`) carrying a real digit run.
/// Banned outside references; **allowed** inside the worked-instance
/// references directory, where the concrete example is load-bearing. The
/// `\b` boundary keeps `CHK-001` out of [`task_regex`].
fn artifact_family_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"\b(?:REQ|DEC|CHK)-\d{3,}\b").expect("valid artifact id regex"))
}

/// CLI lint-code ids (`TSK` / `JNL`) carrying a real digit run; any match is
/// a violation **everywhere**, references included — lint codes are named by
/// the behavior they enforce, never cited by number. Split out from the
/// artifact families because the carve-out exempts the latter in references
/// but never these.
fn lint_code_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"\b(?:TSK|JNL)-\d{3,}\b").expect("valid lint-code id regex"))
}

/// `T-NNN` task ids carrying a real digit run. Banned outside references;
/// **allowed** inside the worked-instance references directory. The leading
/// `\b` keeps `TSK-003` (matched by [`lint_code_regex`]) and ISO timestamps
/// like `...T19:45:00Z` out of this regex.
fn task_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"\bT-\d{3,}\b").expect("valid task id regex"))
}

/// Every `resources/modules/**/*.md` source path under `root`, sorted.
fn module_md_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect_md(&root.join("resources").join("modules"), &mut out);
    out.sort();
    out
}

fn collect_md(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs_err::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect_md(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            out.push(path);
        }
    }
}

/// A single provenance violation: file (workspace-relative), 1-indexed line,
/// offending token, and the fix hint surfaced to the author.
struct Violation {
    rel_path: String,
    line_no: usize,
    token: String,
    fix: &'static str,
}

/// Workspace-relative path prefix marking the worked-instance references
/// directory — the one carve-out where concrete artifact ids are allowed.
const REFERENCES_PREFIX: &str = "resources/modules/references/";

/// Scan one module body for banned ids. `rel_path` is the
/// workspace-relative, forward-slashed path (used both to report violations
/// and to decide whether the references carve-out applies).
///
/// Bans applied everywhere: non-`SPEC-0042` SPEC ids, and `TSK-` / `JNL-`
/// lint codes. Bans applied only outside references: concrete `REQ` / `DEC`
/// / `CHK` artifact ids and `T-NNN` task ids.
fn violations_in(rel_path: &str, body: &str) -> Vec<Violation> {
    let is_references = rel_path.starts_with(REFERENCES_PREFIX);
    let mut out = Vec::new();
    for (idx, line) in body.lines().enumerate() {
        let line_no = idx + 1;
        for m in spec_regex().find_iter(line) {
            if m.as_str() == "SPEC-0042" {
                continue;
            }
            out.push(Violation {
                rel_path: rel_path.to_owned(),
                line_no,
                token: m.as_str().to_owned(),
                fix: "use the generic `SPEC-NNNN`, or the whitelisted example `SPEC-0042`",
            });
        }
        for m in lint_code_regex().find_iter(line) {
            out.push(Violation {
                rel_path: rel_path.to_owned(),
                line_no,
                token: m.as_str().to_owned(),
                fix: "cite CLI lint codes (TSK-/JNL-) by behavior, not by number (references included)",
            });
        }
        if is_references {
            continue;
        }
        for m in artifact_family_regex().find_iter(line) {
            out.push(Violation {
                rel_path: rel_path.to_owned(),
                line_no,
                token: m.as_str().to_owned(),
                fix: "use the generic `<PREFIX>-NNN` form; concrete REQ/DEC/CHK ids belong only in resources/modules/references/",
            });
        }
        for m in task_regex().find_iter(line) {
            out.push(Violation {
                rel_path: rel_path.to_owned(),
                line_no,
                token: m.as_str().to_owned(),
                fix: "use the generic `T-NNN` form; concrete task ids belong only in resources/modules/references/",
            });
        }
    }
    out
}

#[test]
fn module_prose_has_no_internal_artifact_id_provenance() {
    let root = workspace_root();
    let files = module_md_files(&root);

    // Floor guard: a path or layout change that returns near-zero files would
    // make the scan pass vacuously.
    assert!(
        files.len() >= 30,
        "resource-prose scan found only {} .md files under resources/modules/ — \
         the scan scope looks broken",
        files.len(),
    );

    let mut violations: Vec<Violation> = Vec::new();
    for path in &files {
        let rel_path = path
            .strip_prefix(&root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        let body = fs_err::read_to_string(path).expect("module body must be UTF-8 readable");
        violations.extend(violations_in(&rel_path, &body));
    }

    assert!(
        violations.is_empty(),
        "internal artifact-ID provenance found in resources/modules/ prose \
         (see AGENTS.md -> \"Authoring resource prose\"):\n{}",
        violations
            .iter()
            .map(|v| format!(
                "  {}:{} -- `{}` -> {}",
                v.rel_path, v.line_no, v.token, v.fix
            ))
            .collect::<Vec<_>>()
            .join("\n"),
    );
}

/// Guards the detection regexes themselves: a pattern that silently stopped
/// matching real ids would let the scan above pass vacuously. Asserts each
/// regex fires on a concrete id and stays quiet on the generic placeholder and
/// on lookalikes that belong to a different family.
#[test]
fn id_regexes_match_concrete_ids_and_skip_generic_placeholders() {
    assert!(
        spec_regex().is_match("SPEC-0045"),
        "concrete SPEC id matches"
    );
    assert!(
        spec_regex().is_match("SPEC-0042"),
        "the regex matches SPEC-0042; the scan exempts it by exact-string check, not by regex",
    );
    assert!(
        !spec_regex().is_match("SPEC-NNNN"),
        "generic SPEC placeholder has no digit run",
    );

    assert!(
        artifact_family_regex().is_match("REQ-001"),
        "concrete REQ id matches",
    );
    assert!(
        artifact_family_regex().is_match("DEC-001"),
        "concrete DEC id matches",
    );
    assert!(
        artifact_family_regex().is_match("CHK-001"),
        "concrete CHK id matches",
    );
    assert!(
        !artifact_family_regex().is_match("REQ-NNN"),
        "generic REQ placeholder has no digit run",
    );
    assert!(
        !artifact_family_regex().is_match("TSK-003"),
        "TSK is a CLI lint code, not an artifact family",
    );
    assert!(
        !artifact_family_regex().is_match("the SPC-* lint family"),
        "SPC is not in the artifact family set",
    );

    assert!(
        lint_code_regex().is_match("TSK-003"),
        "numbered TSK lint code matches the lint-code regex",
    );
    assert!(
        lint_code_regex().is_match("JNL-001"),
        "numbered JNL lint code matches the lint-code regex",
    );
    assert!(
        !lint_code_regex().is_match("TSK-NNN"),
        "generic lint-code placeholder has no digit run",
    );
    assert!(
        !lint_code_regex().is_match("REQ-001"),
        "REQ is an artifact family, not a lint code",
    );

    assert!(task_regex().is_match("T-001"), "concrete task id matches");
    assert!(
        !task_regex().is_match("T-NNN"),
        "generic task placeholder has no digit run",
    );
    assert!(
        !task_regex().is_match("TSK-003"),
        "the word boundary keeps TSK-003 out of the task regex (it is a lint code)",
    );
    assert!(
        !task_regex().is_match("2026-05-21T19:45:00Z"),
        "an ISO timestamp carries no `T-<digits>` token",
    );
}

/// Guards the references carve-out branch in [`violations_in`]: worked-instance
/// artifact ids are allowed only under `resources/modules/references/`, while
/// the two unconditional bans (non-`SPEC-0042` SPEC ids and numbered lint
/// codes) still apply there. Synthetic `(rel_path, body)` pairs exercise the
/// directory branch directly — deleting the branch (banning everywhere) or
/// over-relaxing it (skipping lint codes in references) fails this test.
#[test]
fn references_carve_out_allows_worked_instance_ids() {
    let refs = "resources/modules/references/spec.md";

    // The worked-instance id set is allowed in references.
    assert!(
        violations_in(
            refs,
            "<task id=\"T-001\" covers=\"REQ-001 REQ-002\"> CHK-001 DEC-001 SPEC-0042"
        )
        .is_empty(),
        "worked-instance ids (REQ/DEC/CHK/T + SPEC-0042) must be allowed in references/",
    );

    // A non-whitelisted SPEC id is still banned, references included.
    assert!(
        !violations_in(refs, "tracking SPEC-0061 here").is_empty(),
        "a SPEC id other than the whitelisted SPEC-0042 is still banned in references/",
    );

    // CLI lint codes cited by number are banned, references included.
    assert!(
        !violations_in(refs, "the TSK-003 lint fires").is_empty(),
        "lint code TSK-003 is banned even in references/",
    );
    assert!(
        !violations_in(refs, "the JNL-001 lint fires").is_empty(),
        "lint code JNL-001 is banned even in references/",
    );

    // Outside references, concrete artifact and task ids are banned as before.
    let non_refs = "resources/modules/skills/speccy-work.md";
    assert!(
        !violations_in(non_refs, "this work covers REQ-001").is_empty(),
        "concrete REQ id is banned outside references/",
    );
    assert!(
        !violations_in(non_refs, "implement T-001 next").is_empty(),
        "concrete task id is banned outside references/",
    );
    // ...but the whitelisted SPEC-0042 example is allowed anywhere.
    assert!(
        violations_in(non_refs, "see the SPEC-0042 example").is_empty(),
        "the whitelisted SPEC-0042 example is allowed outside references/ too",
    );
}

// ---- Wrapper-frontmatter description hygiene ------------------------------
//
// The discovery-layer `description` (AGENTS.md -> "Authoring resource prose"
// item 1) routes skills and subagents and lands verbatim in the host system
// prompt. Two checks apply to every wrapper, one only to user-routed skills:
//   * no angle brackets — a stray `<…>` reads as injected markup and is
//     rejected outright by claude.ai skill upload;
//   * <= 1024 chars — the upload field cap;
//   * a "Use when …" + "Do NOT trigger …" clause pair on `skills/**` only —
//     subagent wrappers are spawned programmatically, never user-routed, so a
//     negative trigger would be noise.
// This gates the stable frontmatter `description` field (a structural surface),
// not body prose, so it survives legitimate editorial rewrites.

/// Case-insensitive "Use when …" trigger-clause marker (tolerates "Use this
/// when").
fn trigger_clause_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| {
        Regex::new(r"(?i)\buse\s+(?:this\s+)?when\b").expect("valid trigger-clause regex")
    })
}

/// Case-insensitive "Do NOT …" negative-trigger marker.
fn negative_clause_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"(?i)\bdo\s+not\b").expect("valid negative-clause regex"))
}

/// Every wrapper-template source under `resources/agents/**`, sorted. Carries
/// the routing frontmatter; content-only include templates are filtered out
/// later by the absence of a `description` field.
fn wrapper_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect_tmpl(&root.join("resources").join("agents"), &mut out);
    out.sort();
    out
}

fn collect_tmpl(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs_err::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect_tmpl(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("tmpl") {
            out.push(path);
        }
    }
}

/// Strip one matched pair of surrounding ASCII quotes, if present.
fn unquote(s: &str) -> &str {
    for q in ['\'', '"'] {
        if let Some(inner) = s.strip_prefix(q).and_then(|t| t.strip_suffix(q)) {
            return inner;
        }
    }
    s
}

/// The `description` routing value of a wrapper, or `None` for a content-only
/// include template (no frontmatter). Handles YAML `description:` (`.md.tmpl`)
/// and TOML `description =` (`.codex/*.toml.tmpl`); each is a single physical
/// line in the current sources.
fn wrapper_description(body: &str) -> Option<&str> {
    body.lines().find_map(|line| {
        let rest = line.trim_start().strip_prefix("description")?.trim_start();
        let value = rest.strip_prefix(':').or_else(|| rest.strip_prefix('='))?;
        Some(unquote(value.trim()))
    })
}

/// A single description-hygiene problem: file (workspace-relative) plus a
/// one-line description of what is wrong.
struct DescIssue {
    rel_path: String,
    problem: String,
}

/// Apply the description checks to one wrapper. `rel_path` is forward-slashed;
/// the `skills/**` clause pair keys off the `/skills/` path segment.
fn description_issues(rel_path: &str, desc: &str) -> Vec<DescIssue> {
    let mut out = Vec::new();
    let push = |out: &mut Vec<DescIssue>, problem: String| {
        out.push(DescIssue {
            rel_path: rel_path.to_owned(),
            problem,
        });
    };

    if desc.contains('<') || desc.contains('>') {
        push(
            &mut out,
            "contains an angle bracket (`<`/`>`); it lands in the system prompt and \
             breaks claude.ai skill upload — name returned blocks by bare identifier, \
             e.g. `a drift-review block`"
                .to_owned(),
        );
    }
    let len = desc.chars().count();
    if len > 1024 {
        push(&mut out, format!("is {len} chars; must be <= 1024"));
    }
    if rel_path.contains("/skills/") {
        if !trigger_clause_regex().is_match(desc) {
            push(
                &mut out,
                "is a skill wrapper but has no \"Use when …\" trigger clause".to_owned(),
            );
        }
        if !negative_clause_regex().is_match(desc) {
            push(
                &mut out,
                "is a skill wrapper but has no \"Do NOT trigger …\" clause".to_owned(),
            );
        }
    }
    out
}

#[test]
fn wrapper_descriptions_are_upload_safe_and_well_routed() {
    let root = workspace_root();
    let files = wrapper_files(&root);

    // Floor guard: a path or layout change that returns near-zero files would
    // make the scan pass vacuously.
    assert!(
        files.len() >= 30,
        "wrapper scan found only {} .tmpl files under resources/agents/ — \
         the scan scope looks broken",
        files.len(),
    );

    let mut described = 0usize;
    let mut issues: Vec<DescIssue> = Vec::new();
    for path in &files {
        let rel_path = path
            .strip_prefix(&root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        let body = fs_err::read_to_string(path).expect("wrapper template must be UTF-8 readable");
        if let Some(desc) = wrapper_description(&body) {
            described += 1;
            issues.extend(description_issues(&rel_path, desc));
        }
    }

    // Floor guard: the routing wrappers (10 skills x 2 hosts, plus subagents)
    // must be found, else the description checks pass vacuously.
    assert!(
        described >= 25,
        "found only {described} wrapper descriptions — the discovery-layer scan looks broken",
    );

    assert!(
        issues.is_empty(),
        "wrapper-description hygiene violations \
         (see AGENTS.md -> \"Authoring resource prose\" item 1):\n{}",
        issues
            .iter()
            .map(|i| format!("  {} -- {}", i.rel_path, i.problem))
            .collect::<Vec<_>>()
            .join("\n"),
    );
}

/// Guards the extraction and the checks themselves, so the scan above can't
/// pass vacuously: extraction handles both wrapper formats and skips
/// content-only includes, and each check fires on a known-bad input and stays
/// quiet on a clean one.
#[test]
fn wrapper_description_extraction_and_checks() {
    assert_eq!(
        wrapper_description("---\nname: x\ndescription: 'hello world'\n---\nbody"),
        Some("hello world"),
        "YAML single-quoted description extracts and unquotes",
    );
    assert_eq!(
        wrapper_description("---\nname: x\ndescription: hello world\n---\n"),
        Some("hello world"),
        "YAML bare description extracts",
    );
    assert_eq!(
        wrapper_description("name = \"x\"\ndescription = 'hello world'\n"),
        Some("hello world"),
        "TOML description extracts and unquotes",
    );
    assert_eq!(
        wrapper_description("{% include \"modules/references/spec.md\" %}\n"),
        None,
        "a content-only include template carries no description",
    );

    let agent = "resources/agents/.claude/agents/x.md.tmpl";
    let skill = "resources/agents/.claude/skills/x/SKILL.md.tmpl";

    assert!(
        description_issues(agent, "Does a thing. Returns a `<drift-review>` block.")
            .iter()
            .any(|i| i.problem.contains("angle bracket")),
        "angle brackets are flagged anywhere",
    );
    assert!(
        description_issues(agent, "Does a thing. Use when the caller dispatches it.").is_empty(),
        "a clean subagent description needs no negative-trigger clause",
    );
    assert!(
        description_issues(skill, "Does a thing. Use when the user says foo.")
            .iter()
            .any(|i| i.problem.contains("Do NOT")),
        "a skill wrapper missing the negative clause is flagged",
    );
    assert!(
        description_issues(skill, "Does a thing. Do NOT trigger on bar.")
            .iter()
            .any(|i| i.problem.contains("Use when")),
        "a skill wrapper missing the trigger clause is flagged",
    );
    let long = "x".repeat(1025);
    assert!(
        description_issues(skill, &format!("Use when a. Do not b. {long}"))
            .iter()
            .any(|i| i.problem.contains("1024")),
        "an over-length description is flagged",
    );
}
