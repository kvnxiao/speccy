//! Embedded prompt-template loader.
//!
//! Templates ship inside the binary via [`include_dir!`] from the
//! `skills/shared/prompts/` directory at the workspace root. Per
//! SPEC-0005 DEC-002 and SPEC-0002 DEC-001 (`include_dir!` for
//! embedded resources).

use include_dir::Dir;
use include_dir::include_dir;
use thiserror::Error;

/// Embedded copy of every shipped prompt template. Sourced from
/// `skills/shared/prompts/` at compile time.
static PROMPTS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../skills/shared/prompts");

/// Failure mode of [`load_template`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PromptError {
    /// No embedded template matched the requested name.
    #[error("prompt template `{name}` not found in embedded bundle")]
    TemplateNotFound {
        /// File name that was looked up (e.g. `plan-greenfield.md`).
        name: String,
    },
    /// Embedded file content was not valid UTF-8. Reachable only if
    /// the build embeds a non-UTF-8 prompt by accident.
    #[error("prompt template `{name}` is not valid UTF-8")]
    NonUtf8Template {
        /// File name that contained the invalid bytes.
        name: String,
    },
}

/// Look up an embedded prompt template by file name.
///
/// `name` is the file name within `skills/shared/prompts/`, including
/// the `.md` extension (e.g. `"plan-greenfield.md"`).
///
/// # Errors
///
/// Returns [`PromptError::TemplateNotFound`] if no embedded entry
/// matches `name`, or [`PromptError::NonUtf8Template`] if the embedded
/// bytes are not valid UTF-8.
pub fn load_template(name: &str) -> Result<&'static str, PromptError> {
    let entry = PROMPTS
        .get_file(name)
        .ok_or_else(|| PromptError::TemplateNotFound {
            name: name.to_owned(),
        })?;
    entry
        .contents_utf8()
        .ok_or_else(|| PromptError::NonUtf8Template {
            name: name.to_owned(),
        })
}

#[cfg(test)]
mod tests {
    use super::PromptError;
    use super::load_template;

    #[test]
    fn loads_plan_greenfield_template() {
        let body = load_template("plan-greenfield.md")
            .expect("plan-greenfield.md must ship in the embedded bundle");
        assert!(
            body.contains("{{agents}}"),
            "plan-greenfield stub must contain `{{{{agents}}}}` placeholder",
        );
        assert!(
            body.contains("{{next_spec_id}}"),
            "plan-greenfield stub must contain `{{{{next_spec_id}}}}` placeholder",
        );
        assert!(
            !body.contains("{{vision}}"),
            "plan-greenfield template must not contain the retired `{{{{vision}}}}` placeholder",
        );
    }

    #[test]
    fn loads_plan_amend_template() {
        let body =
            load_template("plan-amend.md").expect("plan-amend.md must ship in the embedded bundle");
        assert!(
            body.contains("{{spec_md}}"),
            "plan-amend stub must contain `{{{{spec_md}}}}` placeholder",
        );
        assert!(
            body.contains("{{spec_id}}"),
            "plan-amend stub must contain `{{{{spec_id}}}}` placeholder",
        );
        assert!(
            body.contains("{{mission}}"),
            "plan-amend template must contain `{{{{mission}}}}` placeholder for nearest-parent MISSION.md content",
        );
    }

    #[test]
    fn unknown_template_returns_not_found() {
        let result = load_template("nope.md");
        let err = result.expect_err("unknown template name must return TemplateNotFound");
        assert!(
            matches!(err, PromptError::TemplateNotFound { ref name } if name == "nope.md"),
            "expected TemplateNotFound{{ name: \"nope.md\" }}, got {err:?}",
        );
    }
}
