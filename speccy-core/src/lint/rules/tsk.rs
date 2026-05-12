//! TSK-* rules: TASKS.md structural and staleness diagnostics.

use crate::error::ParseError;
use crate::lint::types::Diagnostic;
use crate::lint::types::Level;
use crate::lint::types::ParsedSpec;
use std::collections::HashSet;

const TSK_001: &str = "TSK-001";
const TSK_002: &str = "TSK-002";
const TSK_003: &str = "TSK-003";
const TSK_004: &str = "TSK-004";

const BOOTSTRAP_SENTINEL: &str = "bootstrap-pending";
const REQUIRED_FRONTMATTER_FIELDS: &[&str] = &["spec", "spec_hash_at_generation", "generated_at"];

/// Append every TSK-* diagnostic for one spec.
pub fn lint(spec: &ParsedSpec, out: &mut Vec<Diagnostic>) {
    let Some(tasks_path) = spec.tasks_md_path.clone() else {
        return;
    };
    let Some(tasks_result) = &spec.tasks_md else {
        return;
    };

    match tasks_result {
        Err(err) => emit_tasks_parse_error(spec, &tasks_path, err, out),
        Ok(tasks_md) => {
            for warning in &tasks_md.warnings {
                out.push(Diagnostic::with_location(
                    TSK_002,
                    Level::Error,
                    spec.spec_id.clone(),
                    tasks_path.clone(),
                    u32::try_from(warning.line).unwrap_or(0),
                    warning.message.clone(),
                ));
            }
            tsk_001_covers(spec, &tasks_path, tasks_md, out);
            tsk_003_staleness(spec, &tasks_path, tasks_md, out);
        }
    }
}

fn emit_tasks_parse_error(
    spec: &ParsedSpec,
    tasks_path: &camino::Utf8Path,
    err: &ParseError,
    out: &mut Vec<Diagnostic>,
) {
    match err {
        ParseError::MissingField { field, .. } if field == "frontmatter" => {
            out.push(Diagnostic::with_file(
                TSK_004,
                Level::Error,
                spec.spec_id.clone(),
                tasks_path.to_path_buf(),
                "TASKS.md has no YAML frontmatter; required fields are: spec, spec_hash_at_generation, generated_at"
                    .to_owned(),
            ));
        }
        ParseError::Yaml { message, .. } => {
            let missing: Vec<&&str> = REQUIRED_FRONTMATTER_FIELDS
                .iter()
                .filter(|f| message.contains(**f))
                .collect();
            if missing.is_empty() {
                out.push(Diagnostic::with_file(
                    TSK_004,
                    Level::Error,
                    spec.spec_id.clone(),
                    tasks_path.to_path_buf(),
                    format!("TASKS.md frontmatter failed to parse: {message}"),
                ));
            } else {
                for field in missing {
                    out.push(Diagnostic::with_file(
                        TSK_004,
                        Level::Error,
                        spec.spec_id.clone(),
                        tasks_path.to_path_buf(),
                        format!("TASKS.md frontmatter is missing required field `{field}`"),
                    ));
                }
            }
        }
        other => {
            out.push(Diagnostic::with_file(
                TSK_004,
                Level::Error,
                spec.spec_id.clone(),
                tasks_path.to_path_buf(),
                format!("TASKS.md could not be parsed: {other}"),
            ));
        }
    }
}

fn tsk_001_covers(
    spec: &ParsedSpec,
    tasks_path: &camino::Utf8Path,
    tasks_md: &crate::parse::TasksMd,
    out: &mut Vec<Diagnostic>,
) {
    let mut known_reqs: HashSet<&str> = HashSet::new();
    if let Some(spec_md) = spec.spec_md_ok() {
        for req in &spec_md.requirements {
            known_reqs.insert(req.id.as_str());
        }
    }
    if let Some(spec_toml) = spec.spec_toml_ok() {
        for req in &spec_toml.requirements {
            known_reqs.insert(req.id.as_str());
        }
    }

    if known_reqs.is_empty() {
        // If we don't know any REQ IDs (e.g. both SPEC.md and spec.toml
        // failed to parse), suppress TSK-001 to avoid noise stacking on
        // an upstream parse failure.
        return;
    }

    for task in &tasks_md.tasks {
        for covered in &task.covers {
            if !known_reqs.contains(covered.as_str()) {
                out.push(Diagnostic::with_location(
                    TSK_001,
                    Level::Error,
                    spec.spec_id.clone(),
                    tasks_path.to_path_buf(),
                    u32::try_from(task.line).unwrap_or(0),
                    format!(
                        "task `{tid}` covers `{covered}` but that REQ is not declared in SPEC.md or spec.toml",
                        tid = task.id,
                    ),
                ));
            }
        }
    }
}

fn tsk_003_staleness(
    spec: &ParsedSpec,
    tasks_path: &camino::Utf8Path,
    tasks_md: &crate::parse::TasksMd,
    out: &mut Vec<Diagnostic>,
) {
    let stored_hash = tasks_md.frontmatter.spec_hash_at_generation.as_str();

    if stored_hash == BOOTSTRAP_SENTINEL {
        out.push(Diagnostic::with_file(
            TSK_003,
            Level::Info,
            spec.spec_id.clone(),
            tasks_path.to_path_buf(),
            format!(
                "TASKS.md has `spec_hash_at_generation: bootstrap-pending`; run `speccy tasks {id} --commit` to record the real sha256",
                id = spec
                    .spec_id
                    .clone()
                    .unwrap_or_else(|| "SPEC-NNNN".to_owned()),
            ),
        ));
        return;
    }

    if let Some(spec_md) = spec.spec_md_ok() {
        let current = hex_encode(&spec_md.sha256);
        let expected_form = stored_hash.strip_prefix("sha256:").unwrap_or(stored_hash);
        if !expected_form.eq_ignore_ascii_case(&current) {
            out.push(Diagnostic::with_file(
                TSK_003,
                Level::Warn,
                spec.spec_id.clone(),
                tasks_path.to_path_buf(),
                format!(
                    "TASKS.md may be stale: stored `spec_hash_at_generation` = `{stored_hash}` but current SPEC.md sha256 = `{current}`. Run `/speccy-amend` to reconcile."
                ),
            ));
            return;
        }
    }

    if let (Some(spec_mtime), Some(tasks_mtime)) = (spec.spec_md_mtime, spec.tasks_md_mtime)
        && spec_mtime > tasks_mtime
    {
        out.push(Diagnostic::with_file(
            TSK_003,
            Level::Warn,
            spec.spec_id.clone(),
            tasks_path.to_path_buf(),
            "TASKS.md may be stale: SPEC.md mtime is newer than TASKS.md mtime. Run `/speccy-amend` to reconcile."
                .to_owned(),
        ));
    }
}

fn hex_encode(bytes: &[u8; 32]) -> String {
    let mut out = String::with_capacity(64);
    for byte in bytes {
        let hi = (byte >> 4) & 0x0f;
        let lo = byte & 0x0f;
        out.push(hex_digit(hi));
        out.push(hex_digit(lo));
    }
    out
}

fn hex_digit(nibble: u8) -> char {
    match nibble {
        0..=9 => char::from(b'0'.saturating_add(nibble)),
        10..=15 => char::from(b'a'.saturating_add(nibble.saturating_sub(10))),
        _ => '?',
    }
}
