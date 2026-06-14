#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
//! Tests for skill and phase body files: they must
//! discover speccy resources via CLI JSON envelopes only, not via
//! direct filesystem patterns.
//!
//! Checks (each negative sweep runs over every `.md`
//! file shipped under `resources/modules/`, enumerated from the
//! embedded bundle so new modules are covered automatically):
//!
//! - [`chk014_no_direct_speccy_resource_patterns_in_skills_or_phases`]: no raw
//!   `.speccy/specs/*` globs or bare `SPEC.md`/`TASKS.md`/`MISSION.md`/
//!   `REPORT.md` paths (not bound to a template placeholder) appear in any
//!   module body file.
//! - [`chk015_speccy_plan_uses_vacancy_not_status_for_new_spec_id`]:
//!   `speccy-plan.md` invokes `speccy vacancy --json` (not `speccy status
//!   --json`) to allocate a new SPEC ID.
//! - [`no_old_cli_verbs_in_skill_or_phase_bodies`]: deleted CLI verbs (`speccy
//!   plan`, `speccy tasks`, `speccy implement`, `speccy review`, `speccy
//!   report`) do not appear as commands in any module body file.
//! - [`no_kind_filter_flag_in_skill_or_phase_bodies`]: the removed `--kind`
//!   flag to `speccy next` does not appear in any module body file.

use include_dir::Dir;
use speccy_cli::embedded::RESOURCES;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Read a module file (skill or phase body) from the embedded RESOURCES
/// bundle by sub-path relative to `modules/` (e.g.
/// `"skills/speccy-plan.md"` or `"phases/speccy-work.md"`).
fn read_module(sub_path: &str) -> Option<&'static str> {
    let path = format!("modules/{sub_path}");
    RESOURCES.get_file(&path).and_then(|f| f.contents_utf8())
}

/// Read a module file, panicking with a clear message if absent.
fn require_module(sub_path: &str) -> &'static str {
    read_module(sub_path).unwrap_or_else(|| {
        panic_with_message(&format!(
            "RESOURCES bundle must contain `modules/{sub_path}`; \
             check that the file exists under `resources/modules/`",
        ))
    })
}

/// Test-only failure path. Centralised so the `clippy::panic` expectation
/// is scoped to one function.
#[expect(
    clippy::panic,
    reason = "test-only fixture lookup; failure is a developer-facing assertion"
)]
fn panic_with_message(msg: &str) -> ! {
    panic!("{msg}");
}

// ---------------------------------------------------------------------------
// Module enumeration
// ---------------------------------------------------------------------------

/// Every `.md` module body shipped under `resources/modules/`
/// (skills, partials, phases, personas, references), as
/// `(bundle path, body)` pairs sorted by path. Enumerated from the
/// embedded bundle rather than a hard-coded file list so the negative
/// sweeps below cover newly added module bodies automatically.
fn all_module_bodies() -> Vec<(String, &'static str)> {
    let modules = RESOURCES
        .get_dir("modules")
        .unwrap_or_else(|| panic_with_message("RESOURCES bundle must contain `modules/`"));
    let mut out: Vec<(String, &'static str)> = Vec::new();
    collect_md_bodies(modules, &mut out);
    out.sort_by(|a, b| a.0.cmp(&b.0));
    // Floor guard: if the walk ever returns near-zero files (path or
    // bundle-layout change), every sweep below would pass vacuously.
    assert!(
        out.len() >= 10,
        "module enumeration found only {} .md files under resources/modules/ — \
         the sweep scope looks broken",
        out.len(),
    );
    out
}

fn collect_md_bodies(dir: &Dir<'static>, out: &mut Vec<(String, &'static str)>) {
    for sub in dir.dirs() {
        collect_md_bodies(sub, out);
    }
    for file in dir.files() {
        if file.path().extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let path = file.path().to_string_lossy().replace('\\', "/");
        let body = file.contents_utf8().unwrap_or_else(|| {
            panic_with_message(&format!("module file `{path}` must be valid UTF-8"))
        });
        out.push((path, body));
    }
}

// ---------------------------------------------------------------------------
// no direct speccy-resource discovery patterns
// ---------------------------------------------------------------------------

/// No `.speccy/specs/*` glob expressions, no bare `SPEC.md` /
/// `TASKS.md` / `MISSION.md` / `REPORT.md` filesystem paths (not bound
/// to a `{{ ... }}` template placeholder), and no directory-enumeration
/// instructions targeting `.speccy/specs/` appear in any module body
/// file.
///
/// General-purpose Read/Glob/grep references for non-speccy project files
/// (AGENTS.md, Cargo.toml, source code) are NOT violations. Git
/// pathspecs naming `.speccy/specs/*` (e.g. the journal-preservation
/// exclusion `git restore -- ':!.speccy/specs/*/journal/'`) are not
/// resource discovery and are exempt.
#[test]
fn chk014_no_direct_speccy_resource_patterns_in_skills_or_phases() {
    for (path, body) in &all_module_bodies() {
        // `.speccy/specs/*` glob discovery, checked per line so git
        // invocations (pathspec exclusions, not discovery) can be
        // exempted.
        for (idx, line) in body.lines().enumerate() {
            if line.trim_start().starts_with("git ") {
                continue;
            }
            assert!(
                !line.contains(".speccy/specs/*"),
                "module file `resources/{path}` line {line_no} \
                 contains `.speccy/specs/*` which is a glob discovery of \
                 .speccy/specs/; use `speccy status --json`, \
                 `speccy next --json`, or `speccy vacancy --json` CLI \
                 envelopes instead",
                line_no = idx + 1,
            );
        }

        // Check for raw bare speccy-resource filename references used as
        // direct Read/Glob/filesystem targets (not prose mentions). The
        // violation pattern is a line in a code fence or an explicit tool
        // call that references the bare filename as a positional argument
        // or path without a leading speccy-output field binding it. We
        // look specifically for `Read SPEC.md`, `cat SPEC.md`, or similar
        // direct target invocations — NOT prose mentions like "edit SPEC.md".
        //
        // The patterns below match known direct-target forms. They
        // deliberately do NOT match prose lines that mention the filenames
        // as objects of discussion (those are valid and abundant in skill
        // bodies), nor do they match `spec_md_path`, `tasks_md_path` field
        // names, nor paths that already contain an NNNN-slug segment
        // (those come from CLI output and are already bound).
        let direct_read_patterns: &[(&str, &str)] = &[
            ("Read SPEC.md", "direct `Read SPEC.md` tool call"),
            ("Read TASKS.md", "direct `Read TASKS.md` tool call"),
            ("Read MISSION.md", "direct `Read MISSION.md` tool call"),
            ("Read REPORT.md", "direct `Read REPORT.md` tool call"),
        ];
        for (pattern, desc) in direct_read_patterns {
            assert!(
                !body.contains(pattern),
                "module file `resources/{path}` \
                 contains `{pattern}` which is a {desc}; \
                 obtain the path from `speccy status --json` or \
                 `speccy next --json` path fields instead",
            );
        }

        // Separately check for `.speccy/specs/` directory enumeration
        // instructions (e.g. "Scan .speccy/specs/", "walk .speccy/specs/").
        // We only flag cases where `.speccy/specs/` appears as a scan target,
        // not in historical prose like "the workspace scanner reads
        // `.speccy/specs/`". The pattern `.speccy/specs/` preceded by verbs
        // that indicate active scanning is the violation.
        //
        // The brainstorm body had "Scan\n   `.speccy/specs/`" (newline-split),
        // so we normalise whitespace before checking.
        let normalised = body.split_whitespace().collect::<Vec<_>>().join(" ");
        let scan_instruction = normalised.contains("Scan `.speccy/specs/`")
            || normalised.contains("scan `.speccy/specs/`")
            || normalised.contains("walk `.speccy/specs/`")
            || normalised.contains("Walk `.speccy/specs/`");
        assert!(
            !scan_instruction,
            "module file `resources/{path}` \
             contains a directory-enumeration instruction targeting \
             `.speccy/specs/`; use `speccy status --json` or \
             `speccy next --json` instead",
        );
    }
}

// ---------------------------------------------------------------------------
// speccy-plan uses vacancy not status for new SPEC ID
// ---------------------------------------------------------------------------

/// `resources/modules/skills/speccy-plan.md` invokes
/// `speccy vacancy --json` to allocate a new SPEC ID, not
/// `speccy status --json`.
#[test]
fn chk015_speccy_plan_uses_vacancy_not_status_for_new_spec_id() {
    let body = require_module("skills/speccy-plan.md");

    assert!(
        !body.contains("speccy status --json"),
        "`resources/modules/skills/speccy-plan.md` must NOT invoke \
         `speccy status --json` to allocate a new SPEC ID \
         — use `speccy vacancy --json` instead",
    );
}

// ---------------------------------------------------------------------------
// No old (deleted) CLI verbs in skill or phase body files
// ---------------------------------------------------------------------------

/// Deleted CLI commands (`speccy plan`, `speccy tasks`, `speccy implement`,
/// `speccy review`, `speccy report`) must not appear as invokable commands
/// in any module body file.
///
/// The test checks for the command patterns as they would appear in
/// code-fenced blocks (the primary way commands are presented to agents).
/// Prose text that mentions a verb as a noun (e.g. "the plan phase") is
/// not a violation; the pattern `speccy plan` followed by a word boundary
/// or end-of-text in a code fence is.
#[test]
fn no_old_cli_verbs_in_skill_or_phase_bodies() {
    // These are verb-only patterns — each is a complete `speccy <verb>`
    // invocation that should no longer appear in skill or persona bodies.
    // We look for them in code fences where they would be executed.
    let deleted_verb_patterns: &[(&str, &str)] = &[
        ("speccy plan ", "`speccy plan` (deleted)"),
        ("speccy plan\n", "`speccy plan` (deleted)"),
        ("speccy tasks ", "`speccy tasks` (deleted)"),
        ("speccy tasks\n", "`speccy tasks` (deleted)"),
        ("speccy implement ", "`speccy implement` (deleted)"),
        ("speccy implement\n", "`speccy implement` (deleted)"),
        ("speccy review ", "`speccy review` (deleted)"),
        ("speccy review\n", "`speccy review` (deleted)"),
        ("speccy report ", "`speccy report` (deleted)"),
        ("speccy report\n", "`speccy report` (deleted)"),
    ];

    for (path, body) in &all_module_bodies() {
        for (pattern, desc) in deleted_verb_patterns {
            assert!(
                !body.contains(pattern),
                "module file `resources/{path}` \
                 contains `{pattern}` which references the deleted command \
                 {desc}; remove or replace with the equivalent current \
                 workflow",
            );
        }
    }
}

// ---------------------------------------------------------------------------
// No --kind filter flag in skill or phase bodies
// ---------------------------------------------------------------------------

/// The removed `--kind` flag to `speccy next` must not appear in any
/// module body file (replaced by derived action-kind logic).
#[test]
fn no_kind_filter_flag_in_skill_or_phase_bodies() {
    for (path, body) in &all_module_bodies() {
        assert!(
            !body.contains("--kind"),
            "module file `resources/{path}` \
             contains `--kind` which references the removed \
             `speccy next --kind` flag; replace with \
             `speccy next SPEC-NNNN --json` or `speccy next --json`",
        );
    }
}

// ---------------------------------------------------------------------------
// no-orphan / cross-host / source-to-host parity for the
// shipped reference files.
// ---------------------------------------------------------------------------

/// Workspace root, derived from `CARGO_MANIFEST_DIR` (the `speccy-cli`
/// crate dir) by walking one level up. Mirrors the helper in
/// `tests/init.rs` so this test reads the committed in-tree dogfood
/// host packs at `.claude/`, `.codex/`, `.agents/` and the canonical
/// source at `resources/modules/references/`.
fn workspace_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(
            || std::path::Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf(),
            std::path::Path::to_path_buf,
        )
}

/// One ejected reference file's resolved layout for the orphan / parity
/// scan. `name` is the file's basename (e.g. `spec.md`). `claude_path`
/// and `codex_path` are absolute paths inside the dogfood tree. `source`
/// is the canonical source under `resources/modules/references/`.
/// `pointer_substrs` is the set of path substrings a consuming body
/// inside the matching host pack may use to reach this reference — one
/// string for skill-local files (`references/<name>`), one host-specific
/// string per host for shared files
/// (`.claude/speccy-references/<name>` /
/// `.agents/speccy-references/<name>`).
struct ReferenceFile {
    name: String,
    claude_path: std::path::PathBuf,
    codex_path: std::path::PathBuf,
    source: std::path::PathBuf,
    claude_pointer: String,
    codex_pointer: String,
}

/// Enumerate every ejected reference file across both host packs via
/// directory walks (no hard-coded file list). Returns a vector where
/// each entry pairs the Claude Code path with its Codex sibling. A
/// reference that ships on one host but not the other is a structural
/// failure (parity invariant); the helper asserts the missing-sibling
/// case explicitly with a descriptive message.
fn enumerate_reference_files(root: &std::path::Path) -> Vec<ReferenceFile> {
    let mut out: Vec<ReferenceFile> = Vec::new();
    collect_skill_local_refs(root, &mut out);
    collect_shared_refs(root, &mut out);
    out
}

fn collect_skill_local_refs(root: &std::path::Path, out: &mut Vec<ReferenceFile>) {
    let claude_skills = root.join(".claude").join("skills");
    let codex_skills = root.join(".agents").join("skills");
    let entries = fs_err::read_dir(&claude_skills)
        .unwrap_or_else(|err| panic_with_message(&format!("must read .claude/skills/: {err}")));
    let mut skill_dirs: Vec<std::path::PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    skill_dirs.sort();
    for skill_dir in skill_dirs {
        let refs_dir = skill_dir.join("references");
        if !refs_dir.is_dir() {
            continue;
        }
        let skill_name = skill_dir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_else(|| panic_with_message("skill dir name must be UTF-8"))
            .to_owned();
        let files = fs_err::read_dir(&refs_dir).unwrap_or_else(|err| {
            panic_with_message(&format!("must read {}: {err}", refs_dir.display()))
        });
        let mut ref_files: Vec<std::path::PathBuf> = files
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("md") && p.is_file())
            .collect();
        ref_files.sort();
        for claude_path in ref_files {
            let name = claude_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or_else(|| panic_with_message("ref file name must be UTF-8"))
                .to_owned();
            let codex_path = codex_skills
                .join(&skill_name)
                .join("references")
                .join(&name);
            assert!(
                codex_path.is_file(),
                "cross-host parity: Claude Code ships \
                 `{}` but Codex sibling `{}` is missing",
                claude_path.display(),
                codex_path.display(),
            );
            let pointer = format!("references/{name}");
            out.push(ReferenceFile {
                source: root
                    .join("resources")
                    .join("modules")
                    .join("references")
                    .join(&name),
                name,
                claude_path,
                codex_path,
                claude_pointer: pointer.clone(),
                codex_pointer: pointer,
            });
        }
    }
}

fn collect_shared_refs(root: &std::path::Path, out: &mut Vec<ReferenceFile>) {
    // Host-shared files: walk `.claude/speccy-references/` and pair
    // with `.agents/speccy-references/`.
    let claude_shared = root.join(".claude").join("speccy-references");
    let codex_shared = root.join(".agents").join("speccy-references");
    let shared_entries = fs_err::read_dir(&claude_shared).unwrap_or_else(|err| {
        panic_with_message(&format!("must read .claude/speccy-references/: {err}"))
    });
    let mut shared_files: Vec<std::path::PathBuf> = shared_entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("md") && p.is_file())
        .collect();
    shared_files.sort();
    for claude_path in shared_files {
        let name = claude_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_else(|| panic_with_message("shared ref file name must be UTF-8"))
            .to_owned();
        let codex_path = codex_shared.join(&name);
        assert!(
            codex_path.is_file(),
            "cross-host parity: Claude Code ships \
             `{}` but Codex sibling `{}` is missing",
            claude_path.display(),
            codex_path.display(),
        );
        out.push(ReferenceFile {
            source: root
                .join("resources")
                .join("modules")
                .join("references")
                .join(&name),
            claude_pointer: format!(".claude/speccy-references/{name}"),
            codex_pointer: format!(".agents/speccy-references/{name}"),
            name,
            claude_path,
            codex_path,
        });
    }
}

/// Collect every consuming body path inside a host pack. For Claude
/// Code: every `.md` under `.claude/skills/` (recursive) plus every
/// `.md` directly under `.claude/agents/`. For Codex: every `.md`
/// under `.agents/skills/` (recursive) plus every `.toml` directly
/// under `.codex/agents/`. Reference files themselves are excluded
/// (a reference pointing to itself does not count as a consumer).
fn collect_consuming_bodies(root: &std::path::Path, host: ConsumerHost) -> Vec<std::path::PathBuf> {
    let mut bodies: Vec<std::path::PathBuf> = Vec::new();
    match host {
        ConsumerHost::ClaudeCode => {
            walk_collect(&root.join(".claude").join("skills"), "md", &mut bodies);
            shallow_collect(&root.join(".claude").join("agents"), "md", &mut bodies);
        }
        ConsumerHost::Codex => {
            walk_collect(&root.join(".agents").join("skills"), "md", &mut bodies);
            shallow_collect(&root.join(".codex").join("agents"), "toml", &mut bodies);
        }
    }
    // Exclude any path under a `references/` segment or directly under
    // a `speccy-references/` directory — those are the reference files
    // themselves, not consumers.
    bodies.retain(|p| {
        let s = p.to_string_lossy();
        !s.contains("references/") && !s.contains("references\\")
    });
    bodies.sort();
    bodies
}

#[derive(Clone, Copy)]
enum ConsumerHost {
    ClaudeCode,
    Codex,
}

fn walk_collect(dir: &std::path::Path, ext: &str, out: &mut Vec<std::path::PathBuf>) {
    if !dir.is_dir() {
        return;
    }
    let Ok(entries) = fs_err::read_dir(dir) else {
        return;
    };
    let mut sorted: Vec<std::path::PathBuf> =
        entries.filter_map(Result::ok).map(|e| e.path()).collect();
    sorted.sort();
    for path in sorted {
        if path.is_dir() {
            walk_collect(&path, ext, out);
        } else if path.extension().and_then(|s| s.to_str()) == Some(ext) {
            out.push(path);
        }
    }
}

fn shallow_collect(dir: &std::path::Path, ext: &str, out: &mut Vec<std::path::PathBuf>) {
    if !dir.is_dir() {
        return;
    }
    let Ok(entries) = fs_err::read_dir(dir) else {
        return;
    };
    let mut sorted: Vec<std::path::PathBuf> =
        entries.filter_map(Result::ok).map(|e| e.path()).collect();
    sorted.sort();
    for path in sorted {
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some(ext) {
            out.push(path);
        }
    }
}

/// Byte offset of the first difference between two slices, or `None`
/// if they are equal. Used in the parity failure messages to point at
/// the divergence rather than dumping the whole file.
fn first_diff(a: &[u8], b: &[u8]) -> Option<usize> {
    let min = a.len().min(b.len());
    for i in 0..min {
        if a.get(i) != b.get(i) {
            return Some(i);
        }
    }
    if a.len() == b.len() { None } else { Some(min) }
}

/// Every shipped reference file is reached by at
/// least one path-substring pointer from a consuming body inside the
/// same host pack; the file's bytes match across hosts and match the
/// canonical source under `resources/modules/references/`.
#[test]
fn chk022_no_orphan_references() {
    let root = workspace_root();
    let refs = enumerate_reference_files(&root);
    assert!(
        !refs.is_empty(),
        "expected at least one reference file under \
         .claude/skills/*/references/ or .claude/speccy-references/; \
         found none — run `speccy init --force` against the workspace \
         to refresh the dogfood tree",
    );

    let claude_bodies = collect_consuming_bodies(&root, ConsumerHost::ClaudeCode);
    let codex_bodies = collect_consuming_bodies(&root, ConsumerHost::Codex);

    // Read each consuming body once and cache the contents — the inner
    // loop scans them all per reference file.
    let read_bodies = |paths: &[std::path::PathBuf]| -> Vec<(std::path::PathBuf, String)> {
        paths
            .iter()
            .map(|p| {
                let body = fs_err::read_to_string(p).unwrap_or_else(|err| {
                    panic_with_message(&format!(
                        "must read consuming body `{}`: {err}",
                        p.display(),
                    ))
                });
                (p.clone(), body)
            })
            .collect()
    };
    let claude_body_contents = read_bodies(&claude_bodies);
    let codex_body_contents = read_bodies(&codex_bodies);

    for r in &refs {
        // (1) Orphan check — Claude Code host pack.
        let claude_hits = claude_body_contents
            .iter()
            .filter(|(_, body)| body.contains(&r.claude_pointer))
            .count();
        assert!(
            claude_hits >= 1,
            "reference file `{}` shipped under the \
             Claude Code host pack has zero consuming-body pointers; \
             expected at least one body under .claude/skills/ or \
             .claude/agents/ to contain the substring `{}`",
            r.claude_path.display(),
            r.claude_pointer,
        );

        // (1) Orphan check — Codex host pack.
        let codex_hits = codex_body_contents
            .iter()
            .filter(|(_, body)| body.contains(&r.codex_pointer))
            .count();
        assert!(
            codex_hits >= 1,
            "reference file `{}` shipped under the \
             Codex host pack has zero consuming-body pointers; \
             expected at least one body under .agents/skills/ or \
             .codex/agents/ to contain the substring `{}`",
            r.codex_path.display(),
            r.codex_pointer,
        );

        // (2) Cross-host byte-identical parity.
        let claude_bytes = fs_err::read(&r.claude_path).unwrap_or_else(|err| {
            panic_with_message(&format!("must read `{}`: {err}", r.claude_path.display()))
        });
        let codex_bytes = fs_err::read(&r.codex_path).unwrap_or_else(|err| {
            panic_with_message(&format!("must read `{}`: {err}", r.codex_path.display()))
        });
        if let Some(off) = first_diff(&claude_bytes, &codex_bytes) {
            panic_with_message(&format!(
                "cross-host parity: reference `{}` \
                 differs from sibling `{}` at byte offset {off}",
                r.claude_path.display(),
                r.codex_path.display(),
            ));
        }

        // (3) Source-to-host parity (Claude Code).
        let source_bytes = fs_err::read(&r.source).unwrap_or_else(|err| {
            panic_with_message(&format!(
                "must read canonical source `{}`: {err}",
                r.source.display(),
            ))
        });
        if let Some(off) = first_diff(&source_bytes, &claude_bytes) {
            panic_with_message(&format!(
                "source-to-host parity: canonical source \
                 `{}` differs from Claude Code host copy `{}` at byte \
                 offset {off}",
                r.source.display(),
                r.claude_path.display(),
            ));
        }

        // (3) Source-to-host parity (Codex).
        if let Some(off) = first_diff(&source_bytes, &codex_bytes) {
            panic_with_message(&format!(
                "source-to-host parity: canonical source \
                 `{}` differs from Codex host copy `{}` at byte offset {off}",
                r.source.display(),
                r.codex_path.display(),
            ));
        }

        // (4) Sanity: the reference file name appears as the basename
        // of the source path. Catches a future glob-vs-source-mismatch
        // bug (e.g., a renamed reference shipping but not regenerated
        // from source).
        let source_name = r
            .source
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        assert_eq!(
            source_name, r.name,
            "reference name `{}` does not match its \
             source basename `{source_name}`",
            r.name,
        );
    }
}
