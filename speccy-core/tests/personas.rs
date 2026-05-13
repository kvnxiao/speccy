#![allow(
    clippy::expect_used,
    reason = "test code may .expect() with descriptive messages"
)]
//! SPEC-0009 CHK-001 / CHK-002 / CHK-003 — persona registry and resolver.

use camino::Utf8PathBuf;
use speccy_core::personas::ALL;
use speccy_core::personas::PersonaError;
use speccy_core::personas::resolve_file;
use speccy_core::personas::resolve_file_with_warn;
use std::fs;

const EXPECTED: &[&str] = &[
    "business",
    "tests",
    "security",
    "style",
    "architecture",
    "docs",
];

fn make_tmp_root() -> (tempfile::TempDir, Utf8PathBuf) {
    let tmp = tempfile::tempdir().expect("tempdir creation should succeed");
    let root =
        Utf8PathBuf::from_path_buf(tmp.path().to_path_buf()).expect("tempdir path should be UTF-8");
    (tmp, root)
}

fn write_override(root: &Utf8PathBuf, name: &str, content: &str) {
    let dir = root.join(".speccy").join("skills").join("personas");
    fs::create_dir_all(dir.as_std_path()).expect("override dir should create");
    let file = dir.join(format!("reviewer-{name}.md"));
    fs::write(file.as_std_path(), content).expect("override write should succeed");
}

#[test]
fn registry_contains_six_personas_in_declared_order() {
    assert_eq!(
        ALL, EXPECTED,
        "ALL must list the six personas in the order business, tests, security, style, architecture, docs",
    );
    assert_eq!(ALL.len(), 6, "registry must contain exactly six entries");
}

#[test]
fn registry_default_personas_is_first_four_prefix() {
    let default = ALL.get(..4).expect("ALL must have at least 4 elements");
    assert_eq!(
        default,
        &["business", "tests", "security", "style"],
        "DEFAULT_PERSONAS (SPEC-0007) is mechanically derived as &ALL[..4]",
    );
}

#[test]
fn registry_personas_are_unique() {
    let mut seen: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for name in ALL {
        assert!(
            seen.insert(name),
            "persona {name} appears more than once in ALL",
        );
    }
}

#[test]
fn resolve_local_first_returns_override_content() {
    let (_tmp, root) = make_tmp_root();
    write_override(
        &root,
        "security",
        "# Custom security persona\n\nProject override.\n",
    );
    let body = resolve_file("security", &root).expect("override should resolve");
    assert!(
        body.contains("Custom security persona"),
        "override content must be returned verbatim, got {body:?}",
    );
}

#[test]
fn resolve_local_first_returns_embedded_when_override_missing() {
    let (_tmp, root) = make_tmp_root();
    let body =
        resolve_file("security", &root).expect("embedded fallback should resolve when no override");
    assert!(
        !body.is_empty(),
        "embedded reviewer-security.md must ship in the bundle",
    );
}

#[test]
fn resolve_empty_override_falls_through_with_warning() {
    let (_tmp, root) = make_tmp_root();
    write_override(&root, "security", "   \n\n");
    let mut warns: Vec<u8> = Vec::new();
    let body = resolve_file_with_warn("security", &root, &mut warns)
        .expect("empty override must fall through, not error");
    assert!(
        !body.is_empty(),
        "fallback must return the embedded content, got empty body",
    );
    let warn_text = String::from_utf8(warns).expect("warning output is UTF-8");
    assert!(
        warn_text.contains("empty"),
        "warning should mention the override was empty, got: {warn_text}",
    );
    assert!(
        warn_text.contains("reviewer-security.md"),
        "warning should name the override file, got: {warn_text}",
    );
}

#[test]
fn resolve_empty_override_falls_through_silently_when_no_override_present() {
    let (_tmp, root) = make_tmp_root();
    let mut warns: Vec<u8> = Vec::new();
    let _body = resolve_file_with_warn("security", &root, &mut warns)
        .expect("missing override should fall through silently");
    assert!(
        warns.is_empty(),
        "absent override emits no warning; only empty/unreadable does, got: {warns:?}",
    );
}

#[test]
fn resolve_unknown_name_returns_unknown_name_error() {
    let (_tmp, root) = make_tmp_root();
    let err = resolve_file("nope", &root).expect_err("unknown name must error");
    assert!(
        matches!(
            &err,
            PersonaError::UnknownName { name, valid } if name == "nope" && *valid == ALL,
        ),
        "expected UnknownName{{nope, ALL}}, got {err:?}",
    );
}

#[test]
fn resolve_does_not_check_host_native_locations() {
    let (_tmp, root) = make_tmp_root();
    let claude_dir = root.join(".claude").join("commands");
    fs::create_dir_all(claude_dir.as_std_path()).expect(".claude/commands should create");
    let host_file = claude_dir.join("reviewer-security.md");
    fs::write(
        host_file.as_std_path(),
        "# Host-native persona\nShould NOT be returned.\n",
    )
    .expect("host-native write should succeed");

    let body = resolve_file("security", &root).expect("embedded fallback should be used");
    assert!(
        !body.contains("Host-native persona"),
        "host-native location must not be in the resolution chain, got: {body:?}",
    );
}
