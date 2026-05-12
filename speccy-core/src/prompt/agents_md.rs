//! `AGENTS.md` loader.
//!
//! Missing AGENTS.md is a warning, not an error: a fresh repo may not
//! have one yet, and erroring out would block the very first
//! `speccy plan` invocation (per SPEC-0005 DEC-003). The function
//! returns a marker string the agent reading the rendered prompt can
//! recognise as "project conventions not loaded."

use camino::Utf8Path;
use std::io::Write;

const MARKER_MISSING: &str = "<!-- AGENTS.md missing; project conventions not loaded -->";

/// Load `AGENTS.md` from `project_root` with stderr as the warning
/// sink. See [`load_agents_md_with_warn`] for the form that accepts an
/// injected sink (used in tests).
#[must_use = "the loaded content (or marker) is inlined into the rendered prompt"]
pub fn load_agents_md(project_root: &Utf8Path) -> String {
    let stderr = std::io::stderr();
    let mut lock = stderr.lock();
    load_agents_md_with_warn(project_root, &mut lock)
}

/// Load `AGENTS.md` from `project_root`, writing one-line warnings to
/// `warn_out` when the file is missing or unreadable.
///
/// Returns the file content on success, or a marker string when
/// missing / unreadable so the rendered prompt still surfaces the
/// gap to the agent.
#[must_use = "the loaded content (or marker) is inlined into the rendered prompt"]
pub fn load_agents_md_with_warn<W: Write>(project_root: &Utf8Path, warn_out: &mut W) -> String {
    let path = project_root.join("AGENTS.md");
    match fs_err::read_to_string(path.as_std_path()) {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            if writeln!(
                warn_out,
                "speccy prompt: AGENTS.md not found at {path}; project conventions will not be loaded",
            )
            .is_err()
            {
                // Warning sink is closed; nothing actionable.
            }
            MARKER_MISSING.to_owned()
        }
        Err(err) => {
            if writeln!(
                warn_out,
                "speccy prompt: AGENTS.md at {path} could not be read: {err}",
            )
            .is_err()
            {
                // Warning sink is closed; nothing actionable.
            }
            format!("<!-- AGENTS.md unreadable: {err} -->")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::load_agents_md_with_warn;
    use camino::Utf8PathBuf;
    use tempfile::TempDir;

    fn make_tmp_root() -> (TempDir, Utf8PathBuf) {
        let dir = tempfile::tempdir().expect("tmpdir creation must succeed");
        let path = Utf8PathBuf::from_path_buf(dir.path().to_path_buf())
            .expect("tempdir path must be UTF-8");
        (dir, path)
    }

    #[test]
    fn returns_file_content_verbatim_when_present() {
        let (_tmp, root) = make_tmp_root();
        fs_err::write(root.join("AGENTS.md").as_std_path(), "# Agents\n<rest>")
            .expect("write must succeed");
        let mut warns = Vec::new();
        let out = load_agents_md_with_warn(&root, &mut warns);
        assert_eq!(out, "# Agents\n<rest>");
        assert!(
            warns.is_empty(),
            "no warning expected when file is present, got: {warns:?}",
        );
    }

    #[test]
    fn missing_file_returns_marker_and_warns() {
        let (_tmp, root) = make_tmp_root();
        let mut warns = Vec::new();
        let out = load_agents_md_with_warn(&root, &mut warns);
        assert!(
            out.contains("AGENTS.md missing"),
            "missing marker should mention `AGENTS.md missing`, got: {out}",
        );
        let warn_text = String::from_utf8(warns).expect("warning bytes UTF-8");
        assert!(
            warn_text.contains("AGENTS.md not found"),
            "expected stderr warning naming AGENTS.md, got: {warn_text}",
        );
        assert!(
            warn_text.contains(root.as_str()),
            "expected stderr warning naming the project root path, got: {warn_text}",
        );
    }
}
