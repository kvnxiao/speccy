#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert! macros and return Result for ? propagation in setup"
)]
//! JSON output contract tests for `speccy next --json`.
//! Covers SPEC-0007 CHK-007 and CHK-008.

mod common;

use common::TestResult;
use common::Workspace;
use common::spec_md_template;
use common::valid_spec_toml;
use common::write_spec;
use speccy_cli::next::NextArgs;
use speccy_cli::next::run;
use speccy_core::next::KindFilter;

fn tasks_md(spec_id: &str, body: &str) -> String {
    let body = convert_legacy_to_xml(spec_id, body);
    format!(
        "---\nspec: {spec_id}\nspec_hash_at_generation: bootstrap-pending\ngenerated_at: 2026-05-11T00:00:00Z\n---\n\n# Tasks: {spec_id}\n\n{body}",
    )
}

#[expect(
    clippy::format_push_string,
    reason = "narrow test-only legacy-to-XML transform; flattening hurts readability"
)]
fn convert_legacy_to_xml(spec_id: &str, body: &str) -> String {
    let mut out = format!("<tasks spec=\"{spec_id}\">\n\n");
    let mut current: Option<(String, String, String, Vec<String>)> = None;
    let push = |out: &mut String, cur: (String, String, String, Vec<String>)| {
        let (id, state, title, notes) = cur;
        let covers = notes
            .iter()
            .find_map(|n| n.strip_prefix("Covers:").map(|c| c.trim().to_owned()))
            .unwrap_or_else(|| "REQ-001".to_owned());
        out.push_str(&format!(
            "<task id=\"{id}\" state=\"{state}\" covers=\"{covers}\">\n{title}\n"
        ));
        for note in &notes {
            out.push_str("- ");
            out.push_str(note);
            out.push('\n');
        }
        out.push_str("\n<task-scenarios>\n- placeholder.\n</task-scenarios>\n</task>\n\n");
    };
    for line in body.lines() {
        let trimmed_start = line.trim_start();
        if let Some(rest) = trimmed_start.strip_prefix("- [")
            && let Some((glyph, after)) = rest.split_once("] ")
            && let Some(after) = after.strip_prefix("**")
            && let Some((id, title)) = after.split_once("**")
        {
            let title = title.trim_start_matches(':').trim().to_owned();
            let state = match glyph {
                "~" => "in-progress",
                "?" => "in-review",
                "x" => "completed",
                _ => "pending",
            }
            .to_owned();
            if let Some(cur) = current.take() {
                push(&mut out, cur);
            }
            current = Some((id.to_owned(), state, title, Vec::new()));
            continue;
        }
        if let Some(rest) = trimmed_start.strip_prefix("- ")
            && let Some(ref mut cur) = current
        {
            cur.3.push(rest.to_owned());
            continue;
        }
        if current.is_none() && !line.is_empty() {
            out.push_str(line);
            out.push('\n');
        }
    }
    if let Some(cur) = current.take() {
        push(&mut out, cur);
    }
    out.push_str("</tasks>\n");
    out
}

fn render(ws: &Workspace, kind: Option<KindFilter>) -> Result<String, Box<dyn std::error::Error>> {
    let mut buf: Vec<u8> = Vec::new();
    run(NextArgs { kind, json: true }, &ws.root, &mut buf)?;
    Ok(String::from_utf8(buf)?)
}

// -- CHK-007 ----------------------------------------------------------------

#[test]
fn envelope_and_variants() -> TestResult {
    // implement
    let ws = Workspace::new()?;
    let body = "- [ ] **T-001**: implement signup\n  - Covers: REQ-001\n  - Suggested files: `src/auth.rs`\n";
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks_md("SPEC-0001", body)),
    )?;
    let text = render(&ws, None)?;
    let parsed: serde_json::Value = serde_json::from_str(&text)?;
    assert_eq!(parsed.get("kind"), Some(&serde_json::json!("implement")));
    assert_eq!(parsed.get("schema_version"), Some(&serde_json::json!(1)));
    assert_eq!(parsed.get("spec"), Some(&serde_json::json!("SPEC-0001")));
    assert_eq!(parsed.get("task"), Some(&serde_json::json!("T-001")));
    assert_eq!(
        parsed.get("task_line"),
        Some(&serde_json::json!("implement signup")),
    );
    assert_eq!(parsed.get("covers"), Some(&serde_json::json!(["REQ-001"])),);
    assert_eq!(
        parsed.get("suggested_files"),
        Some(&serde_json::json!(["src/auth.rs"])),
    );
    assert_eq!(
        parsed.get("prompt_command"),
        Some(&serde_json::json!("speccy implement SPEC-0001/T-001")),
    );

    // review
    let ws2 = Workspace::new()?;
    let review_body = "- [?] **T-002**: review me\n  - Covers: REQ-001\n";
    write_spec(
        &ws2.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks_md("SPEC-0001", review_body)),
    )?;
    let text2 = render(&ws2, None)?;
    let parsed2: serde_json::Value = serde_json::from_str(&text2)?;
    assert_eq!(parsed2.get("kind"), Some(&serde_json::json!("review")));
    assert_eq!(parsed2.get("schema_version"), Some(&serde_json::json!(1)));
    assert_eq!(parsed2.get("spec"), Some(&serde_json::json!("SPEC-0001")));
    assert_eq!(parsed2.get("task"), Some(&serde_json::json!("T-002")));
    assert_eq!(
        parsed2.get("personas"),
        Some(&serde_json::json!([
            "business", "tests", "security", "style"
        ])),
    );
    assert_eq!(
        parsed2.get("prompt_command_template"),
        Some(&serde_json::json!(
            "speccy review SPEC-0001/T-002 --persona {persona}"
        )),
    );

    // report
    let ws3 = Workspace::new()?;
    let done_body = "- [x] **T-001**: done\n  - Covers: REQ-001\n";
    write_spec(
        &ws3.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks_md("SPEC-0001", done_body)),
    )?;
    let text3 = render(&ws3, None)?;
    let parsed3: serde_json::Value = serde_json::from_str(&text3)?;
    assert_eq!(parsed3.get("kind"), Some(&serde_json::json!("report")));
    assert_eq!(parsed3.get("spec"), Some(&serde_json::json!("SPEC-0001")));
    assert_eq!(
        parsed3.get("prompt_command"),
        Some(&serde_json::json!("speccy report SPEC-0001")),
    );

    // blocked: empty workspace.
    let ws4 = Workspace::new()?;
    let text4 = render(&ws4, None)?;
    let parsed4: serde_json::Value = serde_json::from_str(&text4)?;
    assert_eq!(parsed4.get("kind"), Some(&serde_json::json!("blocked")));
    assert_eq!(
        parsed4.get("reason"),
        Some(&serde_json::json!("no specs in workspace")),
    );

    Ok(())
}

// -- CHK-008 ----------------------------------------------------------------

#[test]
fn determinism() -> TestResult {
    let ws = Workspace::new()?;
    let body = "- [ ] **T-001**: a\n  - Covers: REQ-001\n- [?] **T-002**: b\n  - Covers: REQ-001\n";
    write_spec(
        &ws.root,
        "0001-foo",
        &spec_md_template("SPEC-0001", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks_md("SPEC-0001", body)),
    )?;
    write_spec(
        &ws.root,
        "0002-bar",
        &spec_md_template("SPEC-0002", "in-progress"),
        &valid_spec_toml(),
        Some(&tasks_md("SPEC-0002", body)),
    )?;

    let a = render(&ws, None)?;
    let b = render(&ws, None)?;
    assert_eq!(a, b, "two consecutive JSON renders must be byte-identical");

    // Also test the kind-filtered variants for determinism.
    let a_imp = render(&ws, Some(KindFilter::Implement))?;
    let b_imp = render(&ws, Some(KindFilter::Implement))?;
    assert_eq!(a_imp, b_imp);

    let a_rev = render(&ws, Some(KindFilter::Review))?;
    let b_rev = render(&ws, Some(KindFilter::Review))?;
    assert_eq!(a_rev, b_rev);
    Ok(())
}
