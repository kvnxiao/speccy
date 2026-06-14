#![expect(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
//! Pin-shape invariants.
//!
//! Scans every shipped agent/skill file under the templated host-pack
//! source at `resources/agents/` and the in-tree dogfood pack at
//! `.claude/`, `.codex/`, `.agents/`, then asserts:
//!
//! 1. No long-form versioned snapshot IDs (`claude-opus-`, `claude-sonnet-`,
//!    `claude-haiku-`) appear in any `model` value.
//! 2. No `model` value contains the substring `haiku` — Haiku is not used
//!    anywhere in the pin assignment.
//! 3. Every Claude Code pinned `model:` value matches `^(opus|sonnet)\[1m\]$` —
//!    the `[1m]` 1M-context-window suffix is load-bearing for the headroom
//!    phase workers and reviewers need.
//! 4. Every Codex pinned `model` value equals the literal `gpt-5.5`.
//! 5. Every Opus-pinned `effort:` value is one of `low`, `medium`, `high`,
//!    `xhigh`, `max`.
//! 6. Every Sonnet-pinned `effort:` value is one of `low`, `medium`, `high`,
//!    `max` — Sonnet does not support the `xhigh` tier.
//! 7. Every Codex pinned `model_reasoning_effort` value is one of `low`,
//!    `medium`, `high`, `xhigh`.
//! 8. The four mechanical-phase Claude Code SKILL.md files plus
//!    `speccy-review`'s SKILL.md (rendered and templated) carry no `model`,
//!    `effort`, `context`, or `agent` keys — slash-command invocation runs in
//!    the parent session.

use camino::Utf8Path;
use camino::Utf8PathBuf;
use regex::Regex;
use serde::Deserialize;

fn workspace_root() -> Utf8PathBuf {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set by cargo");
    let manifest = Utf8PathBuf::from(manifest_dir);
    manifest
        .parent()
        .expect("speccy-core has a parent")
        .to_path_buf()
}

/// Centralised panic so the `clippy::panic` expectation is scoped to
/// one helper instead of every assertion site.
#[expect(
    clippy::panic,
    reason = "test-only fixture lookup; failure is a developer-facing assertion"
)]
fn fail(msg: &str) -> ! {
    panic!("{msg}");
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HostKind {
    ClaudeCode,
    Codex,
}

#[derive(Debug, Clone, Copy)]
enum FrontMatter {
    Yaml,
    Toml,
}

/// Roots scanned for pin frontmatter. Each entry names a directory
/// relative to the workspace root, the host vendor whose conventions
/// govern its frontmatter, and the serialisation shape (`.md` files
/// carry YAML frontmatter, `.toml` files have TOML root keys).
///
/// The list deliberately covers both the templated source at
/// `resources/agents/` and the rendered in-tree dogfood pack so a
/// drift between template and rendered output that introduces a
/// disallowed model/effort value would surface in both places.
const SCAN_ROOTS: &[(&str, HostKind, FrontMatter)] = &[
    (".claude/agents", HostKind::ClaudeCode, FrontMatter::Yaml),
    (".claude/skills", HostKind::ClaudeCode, FrontMatter::Yaml),
    (".codex/agents", HostKind::Codex, FrontMatter::Toml),
    (".agents/skills", HostKind::Codex, FrontMatter::Yaml),
    (
        "resources/agents/.claude/agents",
        HostKind::ClaudeCode,
        FrontMatter::Yaml,
    ),
    (
        "resources/agents/.claude/skills",
        HostKind::ClaudeCode,
        FrontMatter::Yaml,
    ),
    (
        "resources/agents/.codex/agents",
        HostKind::Codex,
        FrontMatter::Toml,
    ),
    (
        "resources/agents/.agents/skills",
        HostKind::Codex,
        FrontMatter::Yaml,
    ),
];

#[derive(Debug, Default, Deserialize)]
struct YamlPins {
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    effort: Option<String>,
    #[serde(default)]
    context: Option<String>,
    #[serde(default)]
    agent: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct TomlPins {
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    model_reasoning_effort: Option<String>,
}

/// A normalised view of one shipped agent/skill file's pin
/// frontmatter. `model` / `effort` apply on the Claude Code side;
/// `model` / `reasoning_effort` apply on the Codex side (Codex has no
/// `effort` axis on the model identifier). The Claude-Code-only
/// `context` / `agent` auto-fork wiring keys are not captured here:
/// their absence is only required on the explicit unpinned skill
/// list, which `unpinned_claude_skills_have_no_pin_keys` checks
/// directly via `read_yaml_pins`.
#[derive(Debug)]
struct PinRecord {
    /// Path relative to the workspace root, used for failure messages.
    path: Utf8PathBuf,
    host: HostKind,
    model: Option<String>,
    effort: Option<String>,
    reasoning_effort: Option<String>,
}

fn split_frontmatter(source: &str) -> Option<(&str, &str)> {
    let after_open = source
        .strip_prefix("---\n")
        .or_else(|| source.strip_prefix("---\r\n"))?;
    let close_idx = after_open.find("\n---")?;
    let yaml = after_open.get(..close_idx)?;
    let after_close = after_open.get(close_idx.saturating_add(4)..)?;
    let body = after_close.strip_prefix('\n').unwrap_or(after_close);
    Some((yaml, body))
}

fn read_yaml_pins(path: &Utf8Path) -> YamlPins {
    let contents = fs_err::read_to_string(path)
        .unwrap_or_else(|err| fail(&format!("file `{path}` must be readable: {err}")));
    let Some((yaml, _body)) = split_frontmatter(&contents) else {
        fail(&format!(
            "file `{path}` must have a `---` frontmatter fence"
        ));
    };
    serde_saphyr::from_str::<YamlPins>(yaml).unwrap_or_else(|err| {
        fail(&format!(
            "file `{path}` frontmatter must be valid YAML: {err}"
        ))
    })
}

fn read_toml_pins(path: &Utf8Path) -> TomlPins {
    let contents = fs_err::read_to_string(path)
        .unwrap_or_else(|err| fail(&format!("file `{path}` must be readable: {err}")));
    toml::from_str::<TomlPins>(&contents)
        .unwrap_or_else(|err| fail(&format!("file `{path}` must be valid TOML: {err}")))
}

/// Compare the file's effective extension (peeling one `.tmpl` layer)
/// against the expected shape, case-insensitively.
fn matches_shape(file: &Utf8Path, shape: FrontMatter) -> bool {
    let effective_ext = match file.extension() {
        Some(e) if e.eq_ignore_ascii_case("tmpl") => file
            .file_stem()
            .and_then(|s| Utf8Path::new(s).extension())
            .unwrap_or_default(),
        Some(e) => e,
        None => "",
    };
    match shape {
        FrontMatter::Yaml => effective_ext.eq_ignore_ascii_case("md"),
        FrontMatter::Toml => effective_ext.eq_ignore_ascii_case("toml"),
    }
}

fn visit(dir: &Utf8Path, out: &mut Vec<Utf8PathBuf>) {
    let entries =
        fs_err::read_dir(dir).unwrap_or_else(|err| fail(&format!("read_dir `{dir}`: {err}")));
    for entry in entries {
        let entry = entry.unwrap_or_else(|err| fail(&format!("dir entry in `{dir}`: {err}")));
        let std_path = entry.path();
        let path = Utf8PathBuf::from_path_buf(std_path)
            .unwrap_or_else(|p| fail(&format!("non-UTF-8 path under `{dir}`: {}", p.display())));
        if path.is_dir() {
            visit(&path, out);
        } else if path.is_file() {
            out.push(path);
        }
    }
}

fn collect_pin_records(root: &Utf8Path) -> Vec<PinRecord> {
    let mut out: Vec<PinRecord> = Vec::new();
    for (subpath, host, shape) in SCAN_ROOTS {
        let dir = root.join(subpath);
        if !dir.exists() {
            continue;
        }
        let mut files: Vec<Utf8PathBuf> = Vec::new();
        visit(&dir, &mut files);
        for file in files {
            if !matches_shape(&file, *shape) {
                continue;
            }
            // Reference files under `<skill>/references/` and
            // `speccy-references/` are plain Markdown with no YAML/TOML
            // frontmatter. They carry no `model:` / `effort:`
            // pins and are out of scope for the pin-shape checks.
            let file_str = file.as_str().replace('\\', "/");
            if file_str.contains("/references/") || file_str.contains("/speccy-references/") {
                continue;
            }
            let rel = file
                .strip_prefix(root)
                .map_or_else(|_| file.clone(), Utf8Path::to_path_buf);
            match shape {
                FrontMatter::Yaml => {
                    let pins = read_yaml_pins(&file);
                    out.push(PinRecord {
                        path: rel,
                        host: *host,
                        model: pins.model,
                        effort: pins.effort,
                        reasoning_effort: None,
                    });
                }
                FrontMatter::Toml => {
                    let pins = read_toml_pins(&file);
                    out.push(PinRecord {
                        path: rel,
                        host: *host,
                        model: pins.model,
                        effort: None,
                        reasoning_effort: pins.model_reasoning_effort,
                    });
                }
            }
        }
    }
    out
}

/// Guard against a silent-pass regression: every other test in this
/// file iterates `collect_pin_records` and asserts `violations.is_empty()`.
/// If the scan ever stops finding files (paths broken, harness change),
/// those tests would pass vacuously. The floor checks every test does
/// useful work.
#[test]
fn scan_finds_expected_minimum_files() {
    let records = collect_pin_records(&workspace_root());
    // Lower bound: 9 Claude-Code agents + 9 templates + 8 Claude-Code
    // SKILL.md + 8 templates + 9 Codex TOMLs + 9 templates + 8 Codex
    // SKILL.md + 8 templates = 68. Floor set well below to absorb
    // future skill additions without re-tuning the test, while still
    // catching a path-broken scan that returns near-zero files.
    assert!(
        records.len() >= 40,
        "scan must find at least 40 agent/skill files; found {} — pin invariants are checked across the full host-pack tree",
        records.len(),
    );
}

#[test]
fn no_long_form_versioned_model_ids() {
    let records = collect_pin_records(&workspace_root());
    let mut violations: Vec<String> = Vec::new();
    for r in &records {
        let Some(model) = &r.model else { continue };
        for needle in ["claude-opus-", "claude-sonnet-", "claude-haiku-"] {
            if model.contains(needle) {
                violations.push(format!(
                    "`{path}` has `model = {model:?}` containing `{needle}` — long-form versioned snapshot IDs are forbidden in shipped pins",
                    path = r.path,
                ));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "long-form-snapshot-ID invariant violated:\n{}",
        violations.join("\n"),
    );
}

#[test]
fn no_haiku_in_model_values() {
    let records = collect_pin_records(&workspace_root());
    let mut violations: Vec<String> = Vec::new();
    for r in &records {
        let Some(model) = &r.model else { continue };
        if model.contains("haiku") {
            violations.push(format!(
                "`{path}` has `model = {model:?}` containing `haiku` — Haiku-tier pins are disallowed anywhere in the pin assignment",
                path = r.path,
            ));
        }
    }
    assert!(
        violations.is_empty(),
        "no-Haiku invariant violated:\n{}",
        violations.join("\n"),
    );
}

#[test]
fn claude_pinned_model_matches_alias_with_1m_suffix() {
    let regex = Regex::new(r"^(opus|sonnet)\[1m\]$")
        .expect("hardcoded Claude Code model-alias regex is valid");
    let records = collect_pin_records(&workspace_root());
    let mut violations: Vec<String> = Vec::new();
    for r in &records {
        if r.host != HostKind::ClaudeCode {
            continue;
        }
        let Some(model) = &r.model else { continue };
        if !regex.is_match(model) {
            violations.push(format!(
                "`{path}` has `model: {model:?}` which does not match `^(opus|sonnet)\\[1m\\]$` — the `[1m]` 1M-context-window suffix is required on every Claude Code pin",
                path = r.path,
            ));
        }
    }
    assert!(
        violations.is_empty(),
        "Claude Code model-alias invariant violated:\n{}",
        violations.join("\n"),
    );
}

#[test]
fn codex_pinned_model_equals_gpt55() {
    let records = collect_pin_records(&workspace_root());
    let mut violations: Vec<String> = Vec::new();
    for r in &records {
        if r.host != HostKind::Codex {
            continue;
        }
        let Some(model) = &r.model else { continue };
        if model != "gpt-5.5" {
            violations.push(format!(
                "`{path}` has `model = {model:?}` — every Codex pin must be the literal `gpt-5.5`",
                path = r.path,
            ));
        }
    }
    assert!(
        violations.is_empty(),
        "Codex model invariant violated:\n{}",
        violations.join("\n"),
    );
}

#[test]
fn opus_pinned_effort_is_valid() {
    let allowed = ["low", "medium", "high", "xhigh", "max"];
    let records = collect_pin_records(&workspace_root());
    let mut violations: Vec<String> = Vec::new();
    for r in &records {
        if r.host != HostKind::ClaudeCode {
            continue;
        }
        let Some(model) = r.model.as_deref() else {
            continue;
        };
        if !model.starts_with("opus") {
            continue;
        }
        match r.effort.as_deref() {
            None => violations.push(format!(
                "`{path}` is Opus-pinned ({model:?}) but is missing the `effort:` key — every Opus pin must declare an effort tier",
                path = r.path,
            )),
            Some(value) if !allowed.contains(&value) => violations.push(format!(
                "`{path}` Opus pin has `effort: {value:?}` outside the allowed Opus tiers {allowed:?}",
                path = r.path,
            )),
            Some(_) => {}
        }
    }
    assert!(
        violations.is_empty(),
        "Opus effort invariant violated:\n{}",
        violations.join("\n"),
    );
}

#[test]
fn sonnet_pinned_effort_is_valid_and_never_xhigh() {
    // Sonnet does not support the xhigh tier.
    let allowed = ["low", "medium", "high", "max"];
    let records = collect_pin_records(&workspace_root());
    let mut violations: Vec<String> = Vec::new();
    for r in &records {
        if r.host != HostKind::ClaudeCode {
            continue;
        }
        let Some(model) = r.model.as_deref() else {
            continue;
        };
        if !model.starts_with("sonnet") {
            continue;
        }
        match r.effort.as_deref() {
            None => violations.push(format!(
                "`{path}` is Sonnet-pinned ({model:?}) but is missing the `effort:` key — every Sonnet pin must declare an effort tier",
                path = r.path,
            )),
            Some(value) if !allowed.contains(&value) => violations.push(format!(
                "`{path}` Sonnet pin has `effort: {value:?}` — Sonnet does not support `xhigh`; allowed values are {allowed:?}",
                path = r.path,
            )),
            Some(_) => {}
        }
    }
    assert!(
        violations.is_empty(),
        "Sonnet effort invariant violated:\n{}",
        violations.join("\n"),
    );
}

#[test]
fn codex_pinned_reasoning_effort_is_valid() {
    // Codex `gpt-5.5` accepts `low`, `medium`, `high`, `xhigh` as
    // `model_reasoning_effort` tiers. The shipped Codex pins
    // use `low/medium/high` only; `xhigh` is permitted so future
    // heavier reviewer work can opt into the higher tier without test
    // churn.
    let allowed = ["low", "medium", "high", "xhigh"];
    let records = collect_pin_records(&workspace_root());
    let mut violations: Vec<String> = Vec::new();
    for r in &records {
        if r.host != HostKind::Codex {
            continue;
        }
        // Only check files that carry a `model` pin; unpinned files
        // legitimately have no `model_reasoning_effort` either.
        if r.model.is_none() {
            continue;
        }
        match r.reasoning_effort.as_deref() {
            None => violations.push(format!(
                "`{path}` carries a Codex `model` pin but is missing `model_reasoning_effort` — every Codex pin must declare a reasoning_effort tier",
                path = r.path,
            )),
            Some(value) if !allowed.contains(&value) => violations.push(format!(
                "`{path}` Codex pin has `model_reasoning_effort = {value:?}` outside the allowed Codex tiers {allowed:?}",
                path = r.path,
            )),
            Some(_) => {}
        }
    }
    assert!(
        violations.is_empty(),
        "Codex reasoning_effort invariant violated:\n{}",
        violations.join("\n"),
    );
}

/// The five Claude Code phase-skill SKILL.md files (and their templated
/// sources) that are declared unpinned.
/// Slash-command invocation runs in the parent session; the
/// cost-and-time pin lives in the matching subagent files instead.
const UNPINNED_CLAUDE_SKILLS: &[&str] = &[
    ".claude/skills/speccy-decompose/SKILL.md",
    ".claude/skills/speccy-work/SKILL.md",
    ".claude/skills/speccy-ship/SKILL.md",
    ".claude/skills/speccy-bootstrap/SKILL.md",
    ".claude/skills/speccy-review/SKILL.md",
    "resources/agents/.claude/skills/speccy-decompose/SKILL.md.tmpl",
    "resources/agents/.claude/skills/speccy-work/SKILL.md.tmpl",
    "resources/agents/.claude/skills/speccy-ship/SKILL.md.tmpl",
    "resources/agents/.claude/skills/speccy-bootstrap/SKILL.md.tmpl",
    "resources/agents/.claude/skills/speccy-review/SKILL.md.tmpl",
];

#[test]
fn unpinned_claude_skills_have_no_pin_keys() {
    let root = workspace_root();
    let mut violations: Vec<String> = Vec::new();
    for rel in UNPINNED_CLAUDE_SKILLS {
        let path = root.join(rel);
        if !path.exists() {
            violations.push(format!(
                "`{rel}` must exist (declared as an unpinned skill surface)"
            ));
            continue;
        }
        let pins = read_yaml_pins(&path);
        if let Some(value) = pins.model {
            violations.push(format!(
                "`{rel}` has `model: {value:?}` — this skill must be unpinned (slash-command invocation runs in the parent session)"
            ));
        }
        if let Some(value) = pins.effort {
            violations.push(format!(
                "`{rel}` has `effort: {value:?}` — this skill must have no `effort:` key"
            ));
        }
        if let Some(value) = pins.context {
            violations.push(format!(
                "`{rel}` has `context: {value:?}` — `context: fork` was dropped from the phase-skill surface"
            ));
        }
        if let Some(value) = pins.agent {
            violations.push(format!(
                "`{rel}` has `agent: {value:?}` — the auto-fork `agent:` pointer was dropped from the phase-skill surface"
            ));
        }
    }
    assert!(
        violations.is_empty(),
        "Unpinned-skills invariant violated:\n{}",
        violations.join("\n"),
    );
}
