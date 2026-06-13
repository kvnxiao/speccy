#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
//! Tests for reviewer persona shared blocks factored into
//! co-located snippet files under `resources/modules/personas/`.
//!
//! Checks:
//!
//! - [`snippet_files_exist`]: snippet files are present alongside persona
//!   bodies.
//! - [`no_partials_subdirectory`]: no `_partials/` directory under personas.
//! - [`persona_bodies_include_verdict_contract_snippet`]: each persona uses `{%
//!   include %}` for `verdict-return-contract.md`.
//! - [`persona_bodies_include_no_tasks_md_writes_snippet`]: each persona uses
//!   `{% include %}` for `no-tasks-md-writes.md`.
//! - [`persona_bodies_include_diff_fetch_snippet`]: each persona uses `{%
//!   include %}` for `diff-fetch-command.md`.
//! - [`rendered_personas_contain_no_minijinja_markup`]: the ejected
//!   `.claude/agents/reviewer-<persona>.md` files have no `{{`, `{%`, or `{#`.
//! - [`no_master_template_file_exists`]: no `reviewer.md.j2` or similar exists.

use speccy_cli::embedded::RESOURCES;
use speccy_cli::host::HostChoice;
use speccy_cli::render::render_host_pack;
use speccy_core::personas;
use std::path::Path;

fn workspace_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf(),
        std::path::Path::to_path_buf,
    )
}

/// Read a persona or snippet file from the embedded RESOURCES bundle.
fn read_module_file(name: &str) -> Option<&'static str> {
    let path = format!("modules/personas/{name}");
    RESOURCES.get_file(&path).and_then(|f| f.contents_utf8())
}

/// Read a persona body or snippet file, panicking with a clear message if
/// missing.
fn require_module_file(name: &str) -> &'static str {
    read_module_file(name).unwrap_or_else(|| {
        panic_with_message(&format!(
            "RESOURCES bundle must contain `modules/personas/{name}`; \
             T-006 requires this snippet to be created",
        ))
    })
}

/// Look up a rendered reviewer file in the `render_host_pack` output, panicking
/// with a clear message if absent.
fn require_rendered_file<'a>(
    rendered: &'a [speccy_cli::render::RenderedFile],
    rel: &str,
) -> &'a speccy_cli::render::RenderedFile {
    rendered
        .iter()
        .find(|f| f.rel_path.as_str() == rel)
        .unwrap_or_else(|| {
            panic_with_message(&format!(
                "rendered claude-code pack must include `{rel}`; \
                 render_host_pack should produce one entry per reviewer persona",
            ))
        })
}

/// Test-only failure path. Centralised so the `clippy::panic` expectation
/// is scoped to one function instead of every call site.
#[expect(
    clippy::panic,
    reason = "test-only fixture lookup; failure is a developer-facing assertion"
)]
fn panic_with_message(msg: &str) -> ! {
    panic!("{msg}");
}

const EXPECTED_SNIPPET_FILES: &[&str] = &[
    "verdict-return-contract.md",
    "no-tasks-md-writes.md",
    "inline-note-format.md",
    "diff-fetch-command.md",
];

/// All snippet files exist alongside the six persona body files.
#[test]
fn snippet_files_exist() {
    for snippet in EXPECTED_SNIPPET_FILES {
        let body = read_module_file(snippet);
        assert!(
            body.is_some(),
            "snippet `resources/modules/personas/{snippet}` must exist (T-006 REQ-007); \
             create it alongside the six reviewer persona body files",
        );
        let content = body.expect("checked above");
        assert!(
            !content.trim().is_empty(),
            "snippet `{snippet}` must not be empty",
        );
    }
}

/// No `_partials/` subdirectory exists anywhere under
/// `resources/modules/personas/`.
#[test]
fn no_partials_subdirectory() {
    let root = workspace_root();
    let partials = root
        .join("resources")
        .join("modules")
        .join("personas")
        .join("_partials");
    assert!(
        !partials.exists(),
        "`resources/modules/personas/_partials/` must not exist (REQ-007); \
         snippets live co-located in the personas/ directory itself",
    );
}

/// Each of the six reviewer persona body files uses `{% include %}` for the
/// `verdict-return-contract.md` snippet exactly once.
#[test]
fn persona_bodies_include_verdict_contract_snippet() {
    let expected_include = r#"{% include "modules/personas/verdict-return-contract.md" %}"#;
    for persona in personas::ALL {
        let file = format!("reviewer-{persona}.md");
        let body = require_module_file(&file);
        assert!(
            body.contains(expected_include),
            "persona `{file}` must contain `{expected_include}` exactly once (T-006 REQ-007); \
             the verdict-return contract text is shared across all six personas",
        );
    }
}

/// Each of the six reviewer persona body files uses `{% include %}` for the
/// `diff-fetch-command.md` snippet.
#[test]
fn persona_bodies_include_diff_fetch_snippet() {
    let expected_include = r#"{% include "modules/personas/diff-fetch-command.md" %}"#;
    for persona in personas::ALL {
        let file = format!("reviewer-{persona}.md");
        let body = require_module_file(&file);
        assert!(
            body.contains(expected_include),
            "persona `{file}` must contain `{expected_include}` (T-006 REQ-007); \
             the diff-fetch command boilerplate is shared across all six personas",
        );
    }
}

/// Each persona body uses `{% include %}` for the `inline-note-format.md`
/// snippet.
#[test]
fn persona_bodies_include_inline_note_format_snippet() {
    let expected_include = r#"{% include "modules/personas/inline-note-format.md" %}"#;
    for persona in personas::ALL {
        let file = format!("reviewer-{persona}.md");
        let body = require_module_file(&file);
        assert!(
            body.contains(expected_include),
            "persona `{file}` must contain `{expected_include}` (T-006 REQ-007); \
             the inline note format template is shared across all six personas",
        );
    }
}

/// The rendered `.claude/agents/reviewer-<persona>.md` files produced by
/// `render_host_pack` for `HostChoice::ClaudeCode` must contain no `MiniJinja`
/// markup: no `{{`, `{%`, or `{#` substrings.
#[test]
fn rendered_personas_contain_no_minijinja_markup() {
    let rendered = render_host_pack(HostChoice::ClaudeCode)
        .expect("render_host_pack(claude-code) must succeed");

    for persona in personas::ALL {
        let rel = format!(".claude/agents/reviewer-{persona}.md");
        let file = require_rendered_file(&rendered, &rel);

        for marker in ["{{", "{%", "{#"] {
            assert!(
                !file.contents.contains(marker),
                "rendered `{rel}` must not contain MiniJinja markup `{marker}`; \
                 all include directives must be fully expanded at render time",
            );
        }
    }
}

/// No master template file exists (no `reviewer.md.j2` or similar file name
/// that implies a single master template for all personas).
#[test]
fn no_master_template_file_exists() {
    let root = workspace_root();
    let personas_dir = root.join("resources").join("modules").join("personas");
    if let Ok(entries) = std::fs::read_dir(&personas_dir) {
        for entry in entries.filter_map(Result::ok) {
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();
            assert!(
                !name.ends_with(".j2"),
                "master template file `{name}` must not exist under `resources/modules/personas/`; \
                 the six persona body files are the source of truth (REQ-007)",
            );
            // No file like "reviewer.md.tmpl" which would serve as a master template
            let is_reviewer_master_tmpl =
                name.starts_with("reviewer") && name.ends_with(".tmpl") && !name.contains('-');
            assert!(
                !is_reviewer_master_tmpl,
                "master template file `{name}` must not exist under `resources/modules/personas/`",
            );
        }
    }
}
