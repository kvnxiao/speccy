//! VAL-* rules: check-definition completeness and no-op detection.

use crate::error::ParseError;
use crate::lint::types::Diagnostic;
use crate::lint::types::Level;
use crate::lint::types::ParsedSpec;
use crate::parse::CheckPayload;
use regex::RegexSet;
use std::sync::OnceLock;

const VAL_001: &str = "VAL-001";
const VAL_002: &str = "VAL-002";
const VAL_003: &str = "VAL-003";
const VAL_004: &str = "VAL-004";

/// Append every VAL-* diagnostic for one spec.
pub fn lint(spec: &ParsedSpec, out: &mut Vec<Diagnostic>) {
    // The parser surfaces "missing command/prompt" as InvalidCheckEntry.
    // The lint engine maps those into VAL-002 / VAL-003 based on the
    // reason text. Other parse errors are already covered by SPC-001.
    if let Err(ParseError::InvalidCheckEntry {
        check_id, reason, ..
    }) = &spec.spec_toml
    {
        if reason.contains("neither") {
            out.push(Diagnostic::with_file(
                VAL_002,
                Level::Error,
                spec.spec_id.clone(),
                spec.spec_toml_path.clone(),
                format!(
                    "check `{check_id}` is missing both `command` and `prompt`; one is required"
                ),
            ));
        } else if reason.contains("both") {
            out.push(Diagnostic::with_file(
                VAL_002,
                Level::Error,
                spec.spec_id.clone(),
                spec.spec_toml_path.clone(),
                format!(
                    "check `{check_id}` declares both `command` and `prompt`; exactly one is required"
                ),
            ));
        }
    }

    let Some(spec_toml) = spec.spec_toml_ok() else {
        return;
    };

    for check in &spec_toml.checks {
        if check.proves.trim().is_empty() {
            out.push(Diagnostic::with_file(
                VAL_001,
                Level::Error,
                spec.spec_id.clone(),
                spec.spec_toml_path.clone(),
                format!("check `{id}` is missing the `proves` field", id = check.id),
            ));
        }

        match (&check.payload, check.kind.as_str()) {
            (CheckPayload::Prompt(_), "test" | "command") => {
                out.push(Diagnostic::with_file(
                    VAL_002,
                    Level::Error,
                    spec.spec_id.clone(),
                    spec.spec_toml_path.clone(),
                    format!(
                        "check `{id}` has kind `{kind}` but declares `prompt` instead of `command`",
                        id = check.id,
                        kind = check.kind,
                    ),
                ));
            }
            (CheckPayload::Command(_), "manual") => {
                out.push(Diagnostic::with_file(
                    VAL_003,
                    Level::Error,
                    spec.spec_id.clone(),
                    spec.spec_toml_path.clone(),
                    format!(
                        "check `{id}` has kind `manual` but declares `command` instead of `prompt`",
                        id = check.id,
                    ),
                ));
            }
            _ => {}
        }

        if let CheckPayload::Command(cmd) = &check.payload {
            let trimmed = cmd.trim();
            if is_no_op(trimmed) {
                out.push(Diagnostic::with_file(
                    VAL_004,
                    Level::Warn,
                    spec.spec_id.clone(),
                    spec.spec_toml_path.clone(),
                    format!(
                        "check `{id}` command is a known no-op (`{trimmed}`); replace with a meaningful proof",
                        id = check.id,
                    ),
                ));
            }
        }
    }
}

fn is_no_op(trimmed: &str) -> bool {
    no_op_set().is_match(trimmed)
}

#[expect(
    clippy::unwrap_used,
    reason = "compile-time literal regex set; covered by unit tests"
)]
fn no_op_set() -> &'static RegexSet {
    static CELL: OnceLock<RegexSet> = OnceLock::new();
    CELL.get_or_init(|| {
        RegexSet::new([
            r"^true$",
            r"^:$",
            r"^exit\s+0$",
            r"^/bin/true$",
            r"^cmd\s+/c\s+exit\s+0$",
            r"^exit\s+/b\s+0$",
        ])
        .unwrap()
    })
}
