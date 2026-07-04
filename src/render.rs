//! Harness-aware template rendering (DESIGN § Harness-Aware Template
//! Rendering). The template bundle is embedded in the binary and rendered to
//! target-specific harness packs with `minijinja` (strict undefined). One
//! reviewer subagent renders per configured persona per target.

use minijinja::{Environment, UndefinedBehavior};
use rust_embed::RustEmbed;
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::config::{Persona, ProjectConfig};
use crate::error::{Result, SpeccyError};

/// Current pack version (bumped when templates change).
pub const PACK_VERSION: &str = "0.1.0";

#[derive(RustEmbed)]
#[folder = "templates/"]
struct Templates;

/// A supported harness target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Harness {
    Claude,
    Codex,
}

impl Harness {
    pub fn key(self) -> &'static str {
        match self {
            Harness::Claude => "claude",
            Harness::Codex => "codex",
        }
    }
    pub fn parse(s: &str) -> Option<Harness> {
        match s {
            "claude" => Some(Harness::Claude),
            "codex" => Some(Harness::Codex),
            _ => None,
        }
    }
}

/// A rendered managed file: its repo-relative path, contents, the source
/// template id, and the source template's content hash (both recorded in the
/// pack lock for freshness/drift checks — a template edit changes the source
/// hash even when `PACK_VERSION` is unchanged).
#[derive(Debug, Clone)]
pub struct ManagedFile {
    pub path: String,
    pub contents: String,
    pub template_id: String,
    pub source_hash: String,
}

const SKILLS: &[&str] = &["brainstorm", "plan", "implement", "ship"];
const ROLES: &[&str] = &["planner", "worker", "verifier", "repair"];

/// The synthetic reviewer a `minimal`-risk run collapses to (DESIGN § Reviewer
/// Personas). Rendered unconditionally so the collapsed roster always names a
/// subagent that exists in the pack.
const COMBINED_PERSONA: &str = "combined";

/// Render the full pack for one harness target from the project config.
pub fn render_pack(target: Harness, config: &ProjectConfig) -> Result<Vec<ManagedFile>> {
    let env = build_env()?;
    let mut files = Vec::new();

    for name in SKILLS {
        let template_id = format!("skill-{name}.j2");
        let ctx = base_context(target);
        files.push(managed(&env, skill_path(target, name), template_id, ctx)?);
    }
    for role in ROLES {
        let template_id = format!("agent-{role}.j2");
        let ctx = base_context(target);
        files.push(managed(&env, agent_path(target, role), template_id, ctx)?);
    }

    // One reviewer file per configured persona, plus the synthetic `combined`
    // reviewer the minimal tier collapses to (unless the roster already names
    // one).
    let mut personas: Vec<Persona> = config.review.personas.clone();
    if !personas.iter().any(|p| p.name == COMBINED_PERSONA) {
        personas.push(Persona {
            name: COMBINED_PERSONA.to_string(),
            model: None,
            min_risk: None,
        });
    }
    for persona in &personas {
        let mut ctx = base_context(target);
        ctx["persona"] = persona_context(target, persona);
        files.push(managed(
            &env,
            reviewer_path(target, &persona.name),
            "agent-reviewer.j2".to_string(),
            ctx,
        )?);
    }
    Ok(files)
}

/// Render one managed file, capturing its source-template content hash.
fn managed(
    env: &Environment,
    path: String,
    template_id: String,
    ctx: serde_json::Value,
) -> Result<ManagedFile> {
    let contents = render(env, &template_id, ctx)?;
    let source_hash = template_source_hash(&template_id)?;
    Ok(ManagedFile {
        path,
        contents,
        template_id,
        source_hash,
    })
}

/// Content hash of an embedded source template.
fn template_source_hash(template_id: &str) -> Result<String> {
    let file = Templates::get(template_id)
        .ok_or_else(|| SpeccyError::io(format!("embedded template {template_id} missing")))?;
    let mut hasher = Sha256::new();
    hasher.update(&file.data);
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn build_env() -> Result<Environment<'static>> {
    let mut env = Environment::new();
    env.set_undefined_behavior(UndefinedBehavior::Strict);
    env.add_filter("toml_escape", |s: String| {
        s.replace('\\', "\\\\").replace('"', "\\\"")
    });
    env.add_filter("yaml_escape", |s: String| {
        // Quote as a YAML double-quoted scalar.
        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
    });
    for path in Templates::iter() {
        let file = Templates::get(&path)
            .ok_or_else(|| SpeccyError::io(format!("embedded template {path} missing")))?;
        let source = String::from_utf8_lossy(&file.data).into_owned();
        env.add_template_owned(path.to_string(), source)
            .map_err(|e| SpeccyError::io(format!("template {path} failed to load: {e}")))?;
    }
    Ok(env)
}

fn render(env: &Environment, template_id: &str, ctx: serde_json::Value) -> Result<String> {
    let tpl = env
        .get_template(template_id)
        .map_err(|e| SpeccyError::io(format!("no template {template_id}: {e}")))?;
    let mut out = tpl
        .render(ctx)
        .map_err(|e| SpeccyError::io(format!("rendering {template_id} failed: {e}")))?;
    if !out.ends_with('\n') {
        out.push('\n');
    }
    Ok(out)
}

fn base_context(target: Harness) -> serde_json::Value {
    let (plan_command, question_tool) = match target {
        Harness::Claude => ("/plan", "AskUserQuestion"),
        Harness::Codex => ("/plan", "request_user_input"),
    };
    json!({
        "target": { "harness": target.key(), "scope": "repo" },
        "names": { "plan_command": plan_command, "question_tool": question_tool },
        "controller": { "cmd": "speccy ctl", "protocol": "0.1" },
        "pack": { "version": PACK_VERSION },
    })
}

/// Resolve a persona's model for this target: a plain string, or a map keyed by
/// target ("claude"/"codex").
fn persona_context(target: Harness, persona: &Persona) -> serde_json::Value {
    let model = persona.model.as_ref().and_then(|m| match m {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Object(map) => map
            .get(target.key())
            .and_then(|v| v.as_str().map(str::to_string)),
        _ => None,
    });
    json!({ "name": persona.name, "model": model, "charter": charter_for(&persona.name) })
}

/// The default charter line for a known persona; a custom persona gets a
/// generic lens description.
fn charter_for(name: &str) -> &'static str {
    match name {
        "spec-fidelity" => {
            "Do the changes satisfy the linked requirements? Any scope drift? Is the evidence non-vacuous and adequate for the risk tier?"
        }
        "defects" => {
            "Implementation correctness independent of the spec text: logic errors, edge cases, error handling, concurrency, silent failures."
        }
        "security" => {
            "Injection, authn/authz, secret handling, unsafe defaults, dependency risk."
        }
        "style" => {
            "Documented conventions, language/framework idioms, comment quality, and process-provenance leakage a regex cannot catch."
        }
        "combined" => {
            "The single combined reviewer for minimal-risk specs: spec fidelity, defects, security, and style in one lens. Keep it proportional to the small change."
        }
        _ => "Review the change through this lens and record structured findings.",
    }
}

fn skill_path(target: Harness, name: &str) -> String {
    match target {
        Harness::Claude => format!(".claude/skills/speccy-{name}/SKILL.md"),
        Harness::Codex => format!(".agents/skills/speccy-{name}/SKILL.md"),
    }
}

fn agent_path(target: Harness, role: &str) -> String {
    match target {
        Harness::Claude => format!(".claude/agents/speccy-{role}.md"),
        Harness::Codex => format!(".codex/agents/speccy-{role}.toml"),
    }
}

fn reviewer_path(target: Harness, persona: &str) -> String {
    match target {
        Harness::Claude => format!(".claude/agents/speccy-reviewer-{persona}.md"),
        Harness::Codex => format!(".codex/agents/speccy-reviewer-{persona}.toml"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_both_targets_without_undefined_errors() {
        let config = ProjectConfig::default();
        for target in [Harness::Claude, Harness::Codex] {
            let files = render_pack(target, &config).unwrap();
            // 4 skills + 4 roles + 4 personas + the synthetic `combined` reviewer.
            assert_eq!(files.len(), 13, "{target:?}");
            assert!(
                files.iter().any(|f| f.path.contains("speccy-reviewer-combined")),
                "combined reviewer must be rendered"
            );
            for f in &files {
                assert!(!f.contents.trim().is_empty(), "{} empty", f.path);
            }
        }
    }

    #[test]
    fn roster_change_changes_persona_file_count() {
        let mut config = ProjectConfig::default();
        config.review.personas.push(Persona {
            name: "perf".into(),
            model: None,
            min_risk: None,
        });
        let files = render_pack(Harness::Claude, &config).unwrap();
        assert!(files
            .iter()
            .any(|f| f.path.contains("speccy-reviewer-perf")));
        // 4 skills + 4 roles + 5 personas + the synthetic `combined` reviewer.
        assert_eq!(files.len(), 14);
    }
}
