#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
//! Integration tests for `speccy_core::tasks::commit_frontmatter`.
//! Covers SPEC-0006 REQ-003 (hash/timestamp recording) and REQ-004
//! (body-byte preservation).

use camino::Utf8PathBuf;
use indoc::indoc;
use jiff::Timestamp;
use speccy_core::tasks::CommitError;
use speccy_core::tasks::commit_frontmatter;
use tempfile::TempDir;

struct Fixture {
    _dir: TempDir,
    path: Utf8PathBuf,
}

fn write_tmp(content: &str) -> Fixture {
    let dir = tempfile::tempdir().expect("tempdir creation should succeed");
    let std_path = dir.path().join("TASKS.md");
    fs_err::write(&std_path, content).expect("writing fixture should succeed");
    let path = Utf8PathBuf::from_path_buf(std_path).expect("tempdir path should be UTF-8");
    Fixture { _dir: dir, path }
}

fn fixed_ts() -> Timestamp {
    "2026-05-13T15:30:42Z"
        .parse::<Timestamp>()
        .expect("hardcoded ISO timestamp parses")
}

fn sha_full(byte: u8) -> [u8; 32] {
    [byte; 32]
}

fn read(path: &Utf8PathBuf) -> String {
    fs_err::read_to_string(path.as_std_path()).expect("read TASKS.md should succeed")
}

#[test]
fn hash_and_timestamp_replace_managed_fields_in_place() {
    let src = indoc! {r"
        ---
        spec: SPEC-0001
        spec_hash_at_generation: bootstrap-pending
        generated_at: 2026-05-11T00:00:00Z
        ---

        # Tasks: SPEC-0001

        - [ ] **T-001**: Add user table
          - Covers: REQ-001
    "};
    let fx = write_tmp(src);
    let hash = sha_full(0xab);
    let now = fixed_ts();

    commit_frontmatter(&fx.path, "SPEC-0001", &hash, now)
        .expect("commit should succeed on well-formed file");

    let after = read(&fx.path);
    assert!(
        after.contains(&format!(
            "spec_hash_at_generation: {hex}",
            hex = "ab".repeat(32)
        )),
        "hash hex must be the full 64-char lowercase form, got: {after}",
    );
    assert!(
        after.contains("generated_at: 2026-05-13T15:30:42Z"),
        "generated_at must be the supplied ISO-Z timestamp, got: {after}",
    );
    assert!(
        after.contains("spec: SPEC-0001"),
        "spec field must be preserved, got: {after}",
    );
}

#[test]
fn bootstrap_pending_sentinel_is_replaced_on_first_commit() {
    let src = indoc! {r"
        ---
        spec: SPEC-0006
        spec_hash_at_generation: bootstrap-pending
        generated_at: 2026-05-11T00:00:00Z
        ---

        body
    "};
    let fx = write_tmp(src);
    let hash = sha_full(0x42);
    let now = fixed_ts();

    commit_frontmatter(&fx.path, "SPEC-0006", &hash, now).expect("commit should succeed");

    let after = read(&fx.path);
    assert!(
        !after.contains("bootstrap-pending"),
        "sentinel must be replaced after first --commit: {after}",
    );
    let hex = "42".repeat(32);
    assert!(
        after.contains(&format!("spec_hash_at_generation: {hex}")),
        "real hash must replace the sentinel, got: {after}",
    );
}

#[test]
fn spec_id_mismatch_returns_error_without_modifying_file() {
    let src = indoc! {r"
        ---
        spec: SPEC-0001
        spec_hash_at_generation: bootstrap-pending
        generated_at: 2026-05-11T00:00:00Z
        ---

        body content
    "};
    let fx = write_tmp(src);
    let before = read(&fx.path);
    let hash = sha_full(0xff);

    let result = commit_frontmatter(&fx.path, "SPEC-0006", &hash, fixed_ts());
    let err = result.expect_err("mismatched SPEC-IDs must return SpecIdMismatch");
    assert!(
        matches!(
            &err,
            CommitError::SpecIdMismatch { in_file, in_arg }
                if in_file == "SPEC-0001" && in_arg == "SPEC-0006"
        ),
        "expected SpecIdMismatch{{ in_file: SPEC-0001, in_arg: SPEC-0006 }}, got {err:?}",
    );

    let after = read(&fx.path);
    assert_eq!(
        before, after,
        "file must NOT be modified when spec mismatches",
    );
}

#[test]
fn missing_frontmatter_prepends_canonical_block() {
    let src = "# No frontmatter here\n\n- [ ] **T-001**: thing\n";
    let fx = write_tmp(src);
    let hash = sha_full(0x10);
    let now = fixed_ts();

    commit_frontmatter(&fx.path, "SPEC-0006", &hash, now).expect("commit should succeed");

    let after = read(&fx.path);
    assert!(
        after.starts_with("---\nspec: SPEC-0006\nspec_hash_at_generation: "),
        "fresh frontmatter must be prepended in canonical order, got: {after}",
    );
    assert!(
        after.contains("\ngenerated_at: 2026-05-13T15:30:42Z\n---\n"),
        "generated_at must follow hash in canonical order, got: {after}",
    );
    assert!(
        after.ends_with(src),
        "original body bytes must be preserved verbatim after the fence, got: {after}",
    );
}

#[test]
fn other_frontmatter_fields_are_preserved_byte_identically() {
    let src = indoc! {r"
        ---
        spec: SPEC-0001
        spec_hash_at_generation: bootstrap-pending
        generated_at: 2026-05-11T00:00:00Z
        notes_for_future: keep this verbatim
        owner: agent/claude-1
        ---

        body
    "};
    let fx = write_tmp(src);
    let hash = sha_full(0xcc);

    commit_frontmatter(&fx.path, "SPEC-0001", &hash, fixed_ts()).expect("commit should succeed");

    let after = read(&fx.path);
    assert!(
        after.contains("notes_for_future: keep this verbatim"),
        "non-managed frontmatter fields must be preserved verbatim, got: {after}",
    );
    assert!(
        after.contains("owner: agent/claude-1"),
        "non-managed frontmatter fields must be preserved verbatim, got: {after}",
    );
}

#[test]
fn non_canonical_field_order_is_preserved() {
    let src = indoc! {r"
        ---
        spec: SPEC-0001
        generated_at: 2026-05-11T00:00:00Z
        spec_hash_at_generation: bootstrap-pending
        ---

        body
    "};
    let fx = write_tmp(src);
    let hash = sha_full(0x77);
    let now = fixed_ts();

    commit_frontmatter(&fx.path, "SPEC-0001", &hash, now).expect("commit should succeed");

    let after = read(&fx.path);
    let gen_at_pos = after
        .find("generated_at:")
        .expect("generated_at must be present");
    let hash_pos = after
        .find("spec_hash_at_generation:")
        .expect("hash field must be present");
    assert!(
        gen_at_pos < hash_pos,
        "non-canonical order (generated_at before hash) must be preserved, got: {after}",
    );
}

#[test]
fn body_byte_preservation_with_lf_line_endings() {
    let body = "\n# Tasks: SPEC-0001\n\n- [ ] **T-001**: thing\n  - Covers: REQ-001\n";
    let src = format!(
        "---\nspec: SPEC-0001\nspec_hash_at_generation: bootstrap-pending\ngenerated_at: 2026-05-11T00:00:00Z\n---{body}",
    );
    let fx = write_tmp(&src);
    let hash = sha_full(0x05);

    commit_frontmatter(&fx.path, "SPEC-0001", &hash, fixed_ts()).expect("commit should succeed");

    let after = read(&fx.path);
    assert!(
        after.ends_with(body),
        "LF body must be preserved byte-identically, got tail: {tail:?}",
        tail = after.bytes().rev().take(80).collect::<Vec<_>>(),
    );
}

#[test]
fn body_byte_preservation_with_crlf_line_endings() {
    let body = "\r\n# Tasks: SPEC-0001\r\n\r\n- [ ] **T-001**: thing\r\n";
    let src = format!(
        "---\r\nspec: SPEC-0001\r\nspec_hash_at_generation: bootstrap-pending\r\ngenerated_at: 2026-05-11T00:00:00Z\r\n---{body}",
    );
    let fx = write_tmp(&src);
    let hash = sha_full(0x06);

    commit_frontmatter(&fx.path, "SPEC-0001", &hash, fixed_ts()).expect("commit should succeed");

    let after = read(&fx.path);
    assert!(
        after.ends_with(body),
        "CRLF body must remain byte-identical, got tail: {tail:?}",
        tail = after.bytes().rev().take(80).collect::<Vec<_>>(),
    );
}

#[test]
fn body_byte_preservation_with_trailing_whitespace() {
    let body = "\n- [ ] **T-001**: title   \n  - Covers: REQ-001\t\n\n\n";
    let src = format!(
        "---\nspec: SPEC-0001\nspec_hash_at_generation: bootstrap-pending\ngenerated_at: 2026-05-11T00:00:00Z\n---{body}",
    );
    let fx = write_tmp(&src);
    let hash = sha_full(0x08);

    commit_frontmatter(&fx.path, "SPEC-0001", &hash, fixed_ts()).expect("commit should succeed");

    let after = read(&fx.path);
    assert!(
        after.ends_with(body),
        "trailing whitespace and blank lines must be preserved, got: {after:?}",
    );
}

#[test]
fn tasks_md_not_found_returns_typed_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = Utf8PathBuf::from_path_buf(dir.path().join("TASKS.md")).expect("tempdir path UTF-8");
    let hash = sha_full(0x00);

    let err = commit_frontmatter(&path, "SPEC-0001", &hash, fixed_ts())
        .expect_err("missing file must error");
    assert!(
        matches!(&err, CommitError::TasksMdNotFound { path: p } if p == &path),
        "expected TasksMdNotFound naming the path, got {err:?}",
    );
}

#[test]
fn two_commits_in_same_second_produce_identical_bytes() {
    let src = indoc! {r"
        ---
        spec: SPEC-0001
        spec_hash_at_generation: bootstrap-pending
        generated_at: 2026-05-11T00:00:00Z
        ---

        body
    "};
    let fx = write_tmp(src);
    let hash = sha_full(0xaa);
    let now = fixed_ts();

    commit_frontmatter(&fx.path, "SPEC-0001", &hash, now).expect("first commit");
    let after_first = read(&fx.path);

    commit_frontmatter(&fx.path, "SPEC-0001", &hash, now).expect("second commit");
    let after_second = read(&fx.path);

    assert_eq!(
        after_first, after_second,
        "two --commits at the same UTC second must produce byte-identical output",
    );
}

#[test]
fn missing_managed_field_appended_when_others_present() {
    let src = indoc! {r"
        ---
        spec: SPEC-0001
        ---

        body
    "};
    let fx = write_tmp(src);
    let hash = sha_full(0xbe);

    commit_frontmatter(&fx.path, "SPEC-0001", &hash, fixed_ts()).expect("commit should succeed");

    let after = read(&fx.path);
    assert!(
        after.contains("spec_hash_at_generation: "),
        "missing hash field must be appended, got: {after}",
    );
    assert!(
        after.contains("generated_at: 2026-05-13T15:30:42Z"),
        "missing timestamp field must be appended, got: {after}",
    );
}
