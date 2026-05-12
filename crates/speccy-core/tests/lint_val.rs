#![expect(
    clippy::panic_in_result_fn,
    reason = "tests use assert!/assert_eq! macros and return Result for ? propagation in setup"
)]
//! VAL-* lint diagnostics.

mod lint_common;

use indoc::indoc;
use lint_common::TestResult;
use lint_common::lint_fixture;
use lint_common::valid_spec_md;
use lint_common::write_spec_fixture;
use speccy_core::lint::types::Diagnostic;
use speccy_core::lint::types::Level;

fn assert_has_code(diags: &[Diagnostic], code: &str) {
    assert!(
        diags.iter().any(|d| d.code == code),
        "expected {code}, got: {:?}",
        diags.iter().map(|d| d.code).collect::<Vec<_>>(),
    );
}

fn assert_no_code(diags: &[Diagnostic], code: &str) {
    assert!(
        !diags.iter().any(|d| d.code == code),
        "unexpected {code}: {:?}",
        diags.iter().map(|d| d.code).collect::<Vec<_>>(),
    );
}

#[test]
fn val_001_fires_when_proves_is_empty() -> TestResult {
    let spec_toml = indoc! {r#"
        schema_version = 1

        [[requirements]]
        id = "REQ-001"
        checks = ["CHK-001"]

        [[checks]]
        id = "CHK-001"
        kind = "test"
        command = "cargo test"
        proves = ""
    "#};
    let fx = write_spec_fixture(&valid_spec_md("SPEC-0001"), spec_toml, None)?;
    let diags = lint_fixture(&fx);
    assert_has_code(&diags, "VAL-001");
    Ok(())
}

#[test]
fn val_002_fires_when_test_kind_carries_prompt() -> TestResult {
    let spec_toml = indoc! {r#"
        schema_version = 1

        [[requirements]]
        id = "REQ-001"
        checks = ["CHK-001"]

        [[checks]]
        id = "CHK-001"
        kind = "test"
        prompt = "this is wrong; tests need a command"
        proves = "covers REQ-001"
    "#};
    let fx = write_spec_fixture(&valid_spec_md("SPEC-0001"), spec_toml, None)?;
    let diags = lint_fixture(&fx);
    assert_has_code(&diags, "VAL-002");
    Ok(())
}

#[test]
fn val_003_fires_when_manual_kind_carries_command() -> TestResult {
    let spec_toml = indoc! {r#"
        schema_version = 1

        [[requirements]]
        id = "REQ-001"
        checks = ["CHK-001"]

        [[checks]]
        id = "CHK-001"
        kind = "manual"
        command = "true"
        proves = "covers REQ-001"
    "#};
    let fx = write_spec_fixture(&valid_spec_md("SPEC-0001"), spec_toml, None)?;
    let diags = lint_fixture(&fx);
    assert_has_code(&diags, "VAL-003");
    Ok(())
}

#[test]
fn val_004_fires_on_each_no_op_pattern() -> TestResult {
    let no_ops = [
        "true",
        ":",
        "exit 0",
        "/bin/true",
        "cmd /c exit 0",
        "exit /b 0",
    ];
    for cmd in no_ops {
        let spec_toml = format!(
            r#"schema_version = 1

[[requirements]]
id = "REQ-001"
checks = ["CHK-001"]

[[checks]]
id = "CHK-001"
kind = "test"
command = "{cmd}"
proves = "covers REQ-001"
"#
        );
        let fx = write_spec_fixture(&valid_spec_md("SPEC-0001"), &spec_toml, None)?;
        let diags = lint_fixture(&fx);
        let val4 = diags.iter().find(|d| d.code == "VAL-004");
        assert!(
            val4.is_some(),
            "VAL-004 did not fire for `{cmd}`; got: {diags:?}"
        );
        if let Some(d) = val4 {
            assert_eq!(d.level, Level::Warn);
        }
    }
    Ok(())
}

#[test]
fn val_004_tolerates_surrounding_whitespace() -> TestResult {
    let spec_toml = indoc! {r#"
        schema_version = 1

        [[requirements]]
        id = "REQ-001"
        checks = ["CHK-001"]

        [[checks]]
        id = "CHK-001"
        kind = "test"
        command = "  true  "
        proves = "covers REQ-001"
    "#};
    let fx = write_spec_fixture(&valid_spec_md("SPEC-0001"), spec_toml, None)?;
    let diags = lint_fixture(&fx);
    assert_has_code(&diags, "VAL-004");
    Ok(())
}

#[test]
fn val_004_does_not_fire_on_compound_commands() -> TestResult {
    let compounds = ["true && cargo test", ": ; do-real-work", "exit 0 || retry"];
    for cmd in compounds {
        let spec_toml = format!(
            r#"schema_version = 1

[[requirements]]
id = "REQ-001"
checks = ["CHK-001"]

[[checks]]
id = "CHK-001"
kind = "test"
command = "{cmd}"
proves = "covers REQ-001"
"#
        );
        let fx = write_spec_fixture(&valid_spec_md("SPEC-0001"), &spec_toml, None)?;
        let diags = lint_fixture(&fx);
        assert_no_code(&diags, "VAL-004");
    }
    Ok(())
}
