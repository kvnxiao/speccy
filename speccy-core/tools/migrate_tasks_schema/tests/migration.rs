//! Integration tests for `migrate-tasks-schema` (SPEC-0029 T-002).
//!
//! These tests cover the four task-body scenarios plus the six
//! `<task-scenarios>` assertions: legacy → XML conversion, idempotency,
//! mixed-form normalisation, freshly-decomposed pass-through, hash
//! neutrality, and structural fidelity through the shipped
//! `parse_task_xml`.

use camino::Utf8Path;
use indoc::indoc;
use migrate_tasks_schema::Outcome;
use migrate_tasks_schema::migrate;
use migrate_tasks_schema::migrate_file;
use speccy_core::parse::BodyItem;
use speccy_core::parse::ReviewVerdict;
use speccy_core::parse::parse_task_xml;
use tempfile::TempDir;

const FRONTMATTER: &str = indoc! {r"
    ---
    spec: SPEC-0099
    spec_hash_at_generation: 0000000000000000000000000000000000000000000000000000000000000000
    generated_at: 2026-05-18T00:00:00Z
    ---

    # Tasks: SPEC-0099 Migration fixture

"};

fn synthetic(body: &str) -> String {
    format!("{FRONTMATTER}{body}")
}

fn fixture_path() -> &'static Utf8Path {
    Utf8Path::new("fixture/TASKS.md")
}

#[test]
fn legacy_bullets_convert_to_canonical_xml_form() {
    // SPEC-0029 T-002 task-scenarios ¶1.
    let src = synthetic(indoc! {r#"
        <tasks spec="SPEC-0099">

        <task id="T-001" state="in-review" covers="REQ-001">
        Some free prose describing the task.

        - Suggested files: `foo.rs`, `bar.rs`

        <task-scenarios>
        Given a thing, when an action, then an outcome.
        </task-scenarios>

        - Implementer note (session-s1):
          - Completed: shipped the feature
          - Undone: (none)
          - Commands run: `cargo test`
          - Exit codes: 0
          - Discovered issues: (none)
          - Procedural compliance: (none)
        - Review (business, blocking): the slice lacks the X path described in REQ-001.
          The diff at file.rs:42 misses the contracted Y behavior; surface a real
          test that fails when Y is removed.
        - Retry: add the missing X path and the failing Y guard test described in
          the business review.

        </task>

        </tasks>
    "#});

    let migrated = migrate(&src);

    assert!(
        migrated.contains(r#"<implementer-note session="s1">"#),
        "expected implementer-note open tag, got:\n{migrated}"
    );
    assert!(
        migrated.contains("</implementer-note>"),
        "expected implementer-note close tag, got:\n{migrated}"
    );
    assert!(
        migrated.contains(r#"<review persona="business" verdict="blocking">"#),
        "expected review open tag, got:\n{migrated}"
    );
    assert!(
        migrated.contains("</review>"),
        "expected review close tag, got:\n{migrated}"
    );
    assert!(
        migrated.contains("<retry>"),
        "expected retry open tag, got:\n{migrated}"
    );
    assert!(
        migrated.contains("</retry>"),
        "expected retry close tag, got:\n{migrated}"
    );
    // Legacy bullets are gone.
    assert!(
        !migrated.contains("- Implementer note (session-"),
        "legacy implementer-note bullet survived: \n{migrated}"
    );
    assert!(
        !migrated.contains("- Review ("),
        "legacy review bullet survived: \n{migrated}"
    );
    assert!(
        !migrated.contains("- Retry: "),
        "legacy retry bullet survived: \n{migrated}"
    );
    // The implementer-note body keeps its six sub-bullets as markdown
    // payload (per DEC-004); dedented to top-level.
    assert!(
        migrated.contains("- Completed: shipped the feature"),
        "implementer-note sub-bullet should survive dedented as top-level bullet, got:\n{migrated}"
    );
    assert!(
        migrated.contains("- Procedural compliance: (none)"),
        "implementer-note sub-bullet should survive dedented, got:\n{migrated}"
    );
    // `<task-scenarios>` body and `Suggested files:` bullet preserved
    // byte-identical.
    assert!(
        migrated.contains("Given a thing, when an action, then an outcome."),
        "task-scenarios body should be preserved, got:\n{migrated}"
    );
    assert!(
        migrated.contains("- Suggested files: `foo.rs`, `bar.rs`"),
        "Suggested files bullet should be preserved, got:\n{migrated}"
    );
}

#[test]
fn migrated_output_parses_via_shipped_task_xml() {
    // SPEC-0029 T-002 task-scenarios ¶2.
    let src = synthetic(indoc! {r#"
        <tasks spec="SPEC-0099">

        <task id="T-001" state="in-review" covers="REQ-001">
        prose.

        <task-scenarios>
        Given X, when Y, then Z.
        </task-scenarios>

        - Implementer note (session-s1):
          - Completed: foo
          - Undone: bar
          - Commands run: baz
          - Exit codes: 0
          - Discovered issues: (none)
          - Procedural compliance: (none)
        - Review (business, blocking): the slice lacks X.
          Add coverage for Y.
        - Retry: do X and Y as the business review said.

        </task>

        </tasks>
    "#});

    let migrated = migrate(&src);
    let doc = parse_task_xml(&migrated, fixture_path())
        .expect("migrated source must parse under the shipped task_xml parser");

    let task = doc.tasks.first().expect("one task expected");
    assert_eq!(task.id, "T-001");
    assert_eq!(task.body_items.len(), 3, "expected 3 body items");

    // Source-order: ImplementerNote, Review, Retry.
    match task.body_items.first().expect("body_items[0]") {
        BodyItem::ImplementerNote { session, body, .. } => {
            assert_eq!(session, "s1");
            assert!(
                body.contains("Completed: foo"),
                "body should include Completed: foo, got: {body}"
            );
            assert!(
                body.contains("Procedural compliance: (none)"),
                "body should include Procedural compliance, got: {body}"
            );
        }
        other => panic!("expected ImplementerNote at [0], got {other:?}"),
    }
    match task.body_items.get(1).expect("body_items[1]") {
        BodyItem::Review {
            persona,
            verdict,
            body,
            ..
        } => {
            assert_eq!(persona, "business");
            assert_eq!(*verdict, ReviewVerdict::Blocking);
            assert!(
                body.contains("the slice lacks X"),
                "review body lost content: {body}"
            );
        }
        other => panic!("expected Review at [1], got {other:?}"),
    }
    match task.body_items.get(2).expect("body_items[2]") {
        BodyItem::Retry { body, .. } => {
            assert!(
                body.contains("do X and Y"),
                "retry body lost content: {body}"
            );
        }
        other => panic!("expected Retry at [2], got {other:?}"),
    }
}

#[test]
fn already_migrated_input_is_byte_identical_output() {
    // SPEC-0029 T-002 task-scenarios ¶3 (idempotency on already-migrated input).
    let src = synthetic(indoc! {r#"
        <tasks spec="SPEC-0099">

        <task id="T-001" state="in-review" covers="REQ-001">
        prose.

        <task-scenarios>
        Given X, when Y, then Z.
        </task-scenarios>

        <implementer-note session="s1">
        - Completed: shipped
        - Undone: (none)
        - Commands run: `cargo test`
        - Exit codes: 0
        - Discovered issues: (none)
        - Procedural compliance: (none)
        </implementer-note>

        <review persona="business" verdict="pass">
        slice looks correct end-to-end.
        </review>

        </task>

        </tasks>
    "#});

    let migrated = migrate(&src);
    assert_eq!(
        migrated, src,
        "already-migrated input should be byte-identical output"
    );
}

#[test]
fn migration_tool_is_idempotent_on_two_runs() {
    // SPEC-0029 T-002 task-scenarios ¶4 (running twice produces same output and
    // second is no-op).
    let src = synthetic(indoc! {r#"
        <tasks spec="SPEC-0099">

        <task id="T-001" state="in-review" covers="REQ-001">
        prose.

        <task-scenarios>
        Given a thing, when an action, then an outcome.
        </task-scenarios>

        - Implementer note (session-s1):
          - Completed: foo
          - Undone: (none)
          - Commands run: bar
          - Exit codes: 0
          - Discovered issues: (none)
          - Procedural compliance: (none)
        - Review (business, pass): looks fine.

        </task>

        </tasks>
    "#});

    let first = migrate(&src);
    let second = migrate(&first);
    assert_eq!(
        second, first,
        "running migration twice must be a no-op on the second run"
    );
    // The first run must have actually converted something.
    assert_ne!(first, src, "first run should have converted bullets");
}

#[test]
fn mixed_legacy_and_xml_normalises_to_single_canonical_form() {
    // SPEC-0029 T-002 task-scenarios ¶3 partial-migration hypothetical: a
    // task whose body carries BOTH a legacy bullet and an already-migrated
    // XML element. Legacy bullets convert; XML elements pass through.
    let src = synthetic(indoc! {r#"
        <tasks spec="SPEC-0099">

        <task id="T-001" state="in-review" covers="REQ-001">
        prose.

        <task-scenarios>
        Given X, when Y, then Z.
        </task-scenarios>

        <implementer-note session="s1">
        - Completed: first session shipped
        - Undone: (none)
        - Commands run: cargo test
        - Exit codes: 0
        - Discovered issues: (none)
        - Procedural compliance: (none)
        </implementer-note>

        - Review (business, blocking): missing the Y behavior.
          The diff misses Y; surface a guard test.
        - Retry: add Y coverage as the business reviewer requested.

        </task>

        </tasks>
    "#});

    let migrated = migrate(&src);

    // The pre-existing XML implementer-note survives unchanged.
    assert!(
        migrated.contains(r#"<implementer-note session="s1">"#),
        "pre-existing XML implementer-note should survive: {migrated}"
    );
    assert!(
        migrated.contains("- Completed: first session shipped"),
        "pre-existing implementer-note body should be preserved verbatim: {migrated}"
    );
    // The legacy review and retry got converted.
    assert!(
        migrated.contains(r#"<review persona="business" verdict="blocking">"#),
        "legacy review should be converted: {migrated}"
    );
    assert!(
        migrated.contains("<retry>"),
        "legacy retry should be converted: {migrated}"
    );
    assert!(
        !migrated.contains("- Review ("),
        "legacy review bullet should be gone: {migrated}"
    );
    assert!(
        !migrated.contains("- Retry: "),
        "legacy retry bullet should be gone: {migrated}"
    );

    // The output parses cleanly under the shipped parser.
    let doc = parse_task_xml(&migrated, fixture_path())
        .expect("migrated source must parse under task_xml");
    let task = doc.tasks.first().expect("one task expected");
    assert_eq!(task.body_items.len(), 3);
}

#[test]
fn freshly_decomposed_task_with_no_body_items_passes_through() {
    // SPEC-0029 T-002 task-scenarios ¶4: a task whose body has no legacy
    // bullets and no new XML elements — only prose, Suggested files, and
    // task-scenarios — passes through unchanged.
    let src = synthetic(indoc! {r#"
        <tasks spec="SPEC-0099">

        <task id="T-001" state="pending" covers="REQ-001">
        ## T-001: Some task title

        Free prose describing what needs to be done.

        - Suggested files: `foo.rs`, `bar.rs`

        <task-scenarios>
        Given the world, when X happens, then Y is observed.
        </task-scenarios>
        </task>

        </tasks>
    "#});

    let migrated = migrate(&src);
    assert_eq!(
        migrated, src,
        "freshly-decomposed task body without legacy bullets or new XML elements must pass through byte-identical"
    );
}

#[test]
fn migration_is_hash_neutral() {
    // SPEC-0029 T-002 task-scenarios ¶5: `spec_hash_at_generation` value
    // is byte-identical before and after migration.
    let hash = "abc123def456abc123def456abc123def456abc123def456abc123def456abcd";
    let src = format!(
        indoc! {r#"
            ---
            spec: SPEC-0099
            spec_hash_at_generation: {hash}
            generated_at: 2026-05-18T00:00:00Z
            ---

            # Tasks: SPEC-0099

            <tasks spec="SPEC-0099">

            <task id="T-001" state="in-review" covers="REQ-001">
            prose.

            <task-scenarios>
            Given X, when Y, then Z.
            </task-scenarios>

            - Implementer note (session-s1):
              - Completed: foo
              - Undone: bar
              - Commands run: baz
              - Exit codes: 0
              - Discovered issues: (none)
              - Procedural compliance: (none)

            </task>

            </tasks>
        "#},
        hash = hash
    );

    let migrated = migrate(&src);

    // The hash value survives byte-identical.
    let needle = format!("spec_hash_at_generation: {hash}");
    assert!(
        migrated.contains(&needle),
        "spec_hash_at_generation value should be byte-identical: {migrated}"
    );
}

#[test]
fn migrate_file_writes_only_when_content_differs() {
    let dir = TempDir::new().expect("create tempdir");
    let path =
        camino::Utf8PathBuf::from_path_buf(dir.path().join("TASKS.md")).expect("utf8 tempdir path");

    let src = synthetic(indoc! {r#"
        <tasks spec="SPEC-0099">

        <task id="T-001" state="in-review" covers="REQ-001">
        prose.

        <task-scenarios>
        Given X, when Y, then Z.
        </task-scenarios>

        - Implementer note (session-s1):
          - Completed: shipped
          - Undone: (none)
          - Commands run: cargo test
          - Exit codes: 0
          - Discovered issues: (none)
          - Procedural compliance: (none)

        </task>

        </tasks>
    "#});

    fs_err::write(&path, src.as_bytes()).expect("write fixture");

    // First migration: writes.
    let outcome = migrate_file(&path).expect("first migrate_file should succeed");
    assert_eq!(outcome, Outcome::Migrated);

    let after_first = fs_err::read_to_string(&path).expect("read after first");
    assert_ne!(after_first, src, "first migration should change content");

    // Second migration: byte-equal, no write.
    let outcome2 = migrate_file(&path).expect("second migrate_file should succeed");
    assert_eq!(outcome2, Outcome::Unchanged);

    let after_second = fs_err::read_to_string(&path).expect("read after second");
    assert_eq!(
        after_second, after_first,
        "second run must produce byte-identical file"
    );
}

#[test]
fn legacy_review_with_retry_annotation_drops_third_token() {
    // The legacy `Review (persona, verdict, retry)` shape carries a third
    // token; the new XML schema attributes retries by position (DEC-008),
    // so the third token is dropped.
    let src = synthetic(indoc! {r#"
        <tasks spec="SPEC-0099">

        <task id="T-001" state="in-review" covers="REQ-001">
        prose.

        <task-scenarios>
        Given X, when Y, then Z.
        </task-scenarios>

        - Review (business, pass, retry): after retry the slice satisfies REQ-001.

        </task>

        </tasks>
    "#});

    let migrated = migrate(&src);
    assert!(
        migrated.contains(r#"<review persona="business" verdict="pass">"#),
        "review element should carry only persona+verdict, got:\n{migrated}"
    );
    // The third-token annotation does not leak into the XML attributes.
    assert!(
        !migrated.contains("retry\""),
        "legacy retry annotation must not leak into XML attribute, got:\n{migrated}"
    );
}

#[test]
fn binary_does_not_appear_in_speccy_cli_subcommands() {
    // SPEC-0029 T-002 task-scenarios ¶6: the migration tool is not a
    // speccy subcommand. The shipped `speccy --help` output enumerates
    // its subcommands; the migration tool is a separate binary in a
    // separate crate (`migrate-tasks-schema`), so it cannot surface there.
    //
    // We verify this structurally: speccy-cli's Cargo.toml declares a
    // single `[[bin]] name = "speccy"`, with no entry for migration. This
    // is a static-file assertion rather than a live `--help` parse so
    // the test doesn't depend on speccy-cli being built first.
    let cargo_toml = include_str!("../../../../speccy-cli/Cargo.toml");
    assert!(
        !cargo_toml.contains("migrate-tasks-schema"),
        "speccy-cli must not reference migrate-tasks-schema as a binary or dep"
    );
    assert!(
        !cargo_toml.contains("migrate_tasks_schema"),
        "speccy-cli must not reference the migration tool"
    );
}
