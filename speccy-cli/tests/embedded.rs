//! Bundle-shape tests for `speccy_cli::embedded::RESOURCES`.
//!
//! SPEC-0016 CHK-002 names this file's
//! `resources_bundle_contains_modules_and_agents` test as its proof: the
//! embedded `resources/` snapshot exposes both `modules/` (host-neutral
//! content) and `agents/` (per-host wrapper templates) as walkable directories,
//! and the legacy pre-SPEC-0016 `skills/` tree is absent from the workspace and
//! from the bundle.

use speccy_cli::embedded::RESOURCES;

#[test]
fn resources_bundle_contains_modules_and_agents() {
    for sub in ["modules/personas", "modules/prompts", "modules/skills"] {
        assert!(
            RESOURCES.get_dir(sub).is_some(),
            "embedded RESOURCES bundle must expose `{sub}/` as a walkable directory",
        );
    }

    for host_root in ["agents/.claude", "agents/.codex", "agents/.agents"] {
        assert!(
            RESOURCES.get_dir(host_root).is_some(),
            "embedded RESOURCES bundle must expose `{host_root}/` as a walkable directory",
        );
    }

    assert!(
        RESOURCES.get_dir("skills").is_none(),
        "embedded RESOURCES bundle must not contain a top-level `skills/` directory; the legacy pre-SPEC-0016 tree was retired in SPEC-0016 T-008",
    );

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("speccy-cli/ should have a parent (workspace root)");
    assert!(
        !workspace_root.join("skills").exists(),
        "workspace must not contain a top-level `skills/` directory; per-host wrappers now live under `resources/agents/<install_root>/`",
    );
}
