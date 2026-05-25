//! Bundle-shape tests for `speccy_cli::embedded::RESOURCES`.
//!
//! The embedded `resources/` snapshot exposes both `modules/`
//! (host-neutral content) and `agents/` (per-host wrapper templates) as
//! walkable directories.

use speccy_cli::embedded::RESOURCES;

#[test]
fn resources_bundle_contains_modules_and_agents() {
    for sub in ["modules/personas", "modules/skills"] {
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
}
