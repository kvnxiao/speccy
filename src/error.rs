//! The controller result type, the closed error taxonomy, and the JSON
//! response envelope every `speccy ctl` operation returns.
//!
//! Shapes are owned by `SCHEMAS.md` (§ Envelope). Every operation returns
//! `{ok: true, data}` or `{ok: false, error: {code, message, details}}`.

use serde::Serialize;

/// The closed error-code vocabulary (`SCHEMAS.md` § Envelope).
///
/// Serializes as the snake_case string the protocol specifies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// Operation exists in the CLI surface but is not yet wired up.
    NotImplemented,
    /// A payload failed schema/structural validation, or a policy refused it.
    ValidationFailed,
    /// The requested state transition is illegal from the current state.
    InvalidTransition,
    /// The run lease is held by another agent.
    LeaseHeld,
    /// The workspace is not a git repository (or a subtree of one).
    NotAGitRepo,
    /// `run start` refused because the worktree has uncommitted changes.
    DirtyWorktree,
    /// A referenced entity (spec, run, task, …) does not exist.
    NotFound,
    /// A selector matched more than one candidate.
    AmbiguousSelector,
    /// A repair/resource cap was exhausted.
    CapExhausted,
    /// An underlying I/O or storage operation failed.
    IoError,
}

/// One structured finding, shared by `error.details` and `data.lint.findings`
/// (`SCHEMAS.md` § Envelope).
#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    /// Machine-readable finding code, e.g. `missing_evidence_request`.
    pub code: String,
    /// Optional JSON-ish path into the offending payload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Human-readable explanation.
    pub message: String,
}

impl Finding {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            path: None,
            message: message.into(),
        }
    }

    pub fn at(
        code: impl Into<String>,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            path: Some(path.into()),
            message: message.into(),
        }
    }
}

/// A controller error: a code, a message, and optional structured details.
///
/// This is the failure half of every controller operation. It is never
/// coerced into partial state — the operation rejects and returns this.
#[derive(Debug, Clone, thiserror::Error)]
#[error("{code:?}: {message}")]
pub struct SpeccyError {
    pub code: ErrorCode,
    pub message: String,
    pub details: Vec<Finding>,
}

impl SpeccyError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: Vec::new(),
        }
    }

    /// Attach structured findings (e.g. lint findings on a validation error).
    pub fn with_details(mut self, details: Vec<Finding>) -> Self {
        self.details = details;
        self
    }

    pub fn not_implemented(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotImplemented, message)
    }
    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::ValidationFailed, message)
    }
    pub fn invalid_transition(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidTransition, message)
    }
    pub fn lease_held(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::LeaseHeld, message)
    }
    pub fn not_a_git_repo(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotAGitRepo, message)
    }
    pub fn dirty_worktree(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::DirtyWorktree, message)
    }
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotFound, message)
    }
    pub fn ambiguous_selector(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::AmbiguousSelector, message)
    }
    pub fn cap_exhausted(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::CapExhausted, message)
    }
    pub fn io(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::IoError, message)
    }
}

impl From<std::io::Error> for SpeccyError {
    fn from(e: std::io::Error) -> Self {
        SpeccyError::io(e.to_string())
    }
}

/// The controller result type.
pub type Result<T> = std::result::Result<T, SpeccyError>;

/// Serialized failure body: `{code, message, details}`.
#[derive(Debug, Serialize)]
struct ErrorBody<'a> {
    code: ErrorCode,
    message: &'a str,
    details: &'a [Finding],
}

/// Render a controller result as the response envelope.
///
/// Success is `{"ok": true, "data": <value>}`; failure is
/// `{"ok": false, "error": {code, message, details}}`. Serialization of the
/// success payload is itself fallible, so a serialize failure degrades to an
/// `io_error` envelope rather than panicking.
pub fn envelope<T: Serialize>(result: &Result<T>) -> serde_json::Value {
    match result {
        Ok(data) => match serde_json::to_value(data) {
            Ok(value) => serde_json::json!({ "ok": true, "data": value }),
            Err(e) => error_envelope(&SpeccyError::io(format!(
                "failed to serialize response data: {e}"
            ))),
        },
        Err(e) => error_envelope(e),
    }
}

fn error_envelope(e: &SpeccyError) -> serde_json::Value {
    serde_json::json!({
        "ok": false,
        "error": ErrorBody { code: e.code, message: &e.message, details: &e.details },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_code_serializes_snake_case() {
        let v = serde_json::to_value(ErrorCode::NotImplemented).unwrap();
        assert_eq!(v, serde_json::json!("not_implemented"));
        let v = serde_json::to_value(ErrorCode::NotAGitRepo).unwrap();
        assert_eq!(v, serde_json::json!("not_a_git_repo"));
    }

    #[test]
    fn ok_envelope_wraps_data() {
        let r: Result<_> = Ok(serde_json::json!({ "spec_ref": "SPEC-1" }));
        let env = envelope(&r);
        assert_eq!(env["ok"], serde_json::json!(true));
        assert_eq!(env["data"]["spec_ref"], serde_json::json!("SPEC-1"));
    }

    #[test]
    fn err_envelope_carries_code_message_details() {
        let r: Result<()> =
            Err(
                SpeccyError::validation("bad draft").with_details(vec![Finding::at(
                    "missing_evidence_request",
                    "requirements[R1]",
                    "no evidence",
                )]),
            );
        let env = envelope(&r);
        assert_eq!(env["ok"], serde_json::json!(false));
        assert_eq!(env["error"]["code"], serde_json::json!("validation_failed"));
        assert_eq!(env["error"]["message"], serde_json::json!("bad draft"));
        assert_eq!(
            env["error"]["details"][0]["code"],
            serde_json::json!("missing_evidence_request")
        );
        assert_eq!(
            env["error"]["details"][0]["path"],
            serde_json::json!("requirements[R1]")
        );
    }

    #[test]
    fn empty_details_serialize_as_array() {
        let r: Result<()> = Err(SpeccyError::not_found("no spec"));
        let env = envelope(&r);
        assert_eq!(env["error"]["details"], serde_json::json!([]));
    }
}
