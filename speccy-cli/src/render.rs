//! `MiniJinja`-backed renderer for host skill packs.
//!
//! Per SPEC-0016, `speccy init --host <host>` walks the embedded
//! [`crate::embedded::RESOURCES`] bundle under `agents/.<install_root>/`
//! for each install root the chosen [`crate::host::HostChoice`] writes
//! to, renders every `.tmpl` file through `MiniJinja`, strips the
//! `.tmpl` suffix, and emits a [`RenderedFile`] whose `rel_path` lands
//! on disk at `<project_root>/<rel_path>`. Template `{% include %}`
//! directives resolve against `modules/...` paths inside the same
//! bundle, supplied by a custom loader.
//!
//! The renderer is host-aware: the template context comes from
//! [`crate::host::HostChoice::template_context`], so the same module
//! body file produces slash-prefixed command references for Claude
//! Code and bare command names for Codex.
//!
//! See `.speccy/specs/0016-templated-host-resources/SPEC.md`.

use crate::embedded::RESOURCES;
use crate::host::HostChoice;
use camino::Utf8PathBuf;
use include_dir::Dir;
use minijinja::Environment;
use minijinja::UndefinedBehavior;
use thiserror::Error;

/// Errors surfaced by [`render_host_pack`] and the supporting loader
/// pipeline. Each variant carries enough context to surface the
/// offending bundle path or include name in CLI error messages without
/// re-walking the bundle.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RenderError {
    /// A `.tmpl` file under `agents/.<install_root>/` failed to render.
    /// Wraps the underlying `MiniJinja` error, which already includes the
    /// template name and source position.
    #[error("`MiniJinja` render failed for `{template_name}`")]
    MiniJinjaRender {
        /// Bundle-relative path of the template that failed to render,
        /// e.g. `agents/.claude/skills/speccy-plan/SKILL.md.tmpl`.
        template_name: String,
        /// Underlying `MiniJinja` error.
        #[source]
        source: minijinja::Error,
    },
    /// A `.tmpl` file's content was not valid UTF-8. Reachable only if
    /// a non-text file ends up under `agents/.<install_root>/`, which
    /// the build is not supposed to allow.
    #[error("template `{template_name}` is not valid UTF-8")]
    NonUtf8Template {
        /// Bundle-relative path of the offending file.
        template_name: String,
    },
    /// The bundle path that should host wrapper templates was missing
    /// entirely. Reachable only when the workspace `resources/` tree
    /// is edited in a way that strips a required sub-directory before
    /// the next release.
    #[error("embedded resource bundle is missing sub-path `{subpath}`; this is a build bug")]
    BundleSubpathMissing {
        /// Sub-path inside [`RESOURCES`] that came back `None`.
        subpath: String,
    },
}

/// One rendered output file ready for write-out.
///
/// `rel_path` is project-root-relative (e.g.
/// `.claude/skills/speccy-plan/SKILL.md`) with the leading `agents/`
/// prefix from the bundle path already stripped and the trailing
/// `.tmpl` suffix removed. `contents` is the UTF-8 render output ready
/// to be written verbatim.
#[derive(Debug, Clone)]
#[must_use = "the rendered file must be written to disk to take effect"]
pub struct RenderedFile {
    /// Project-root-relative destination path for the rendered output.
    pub rel_path: Utf8PathBuf,
    /// UTF-8 rendered contents.
    pub contents: String,
}

/// Render every `.tmpl` file under the host's install roots in the
/// embedded [`RESOURCES`] bundle.
///
/// For each install root in [`HostChoice::install_roots`], the
/// renderer walks `agents/<install_root>/` recursively, renders every
/// file whose name ends in `.tmpl` through a strict-undefined
/// `MiniJinja` environment, and produces a [`RenderedFile`] whose
/// `rel_path` strips the `agents/` prefix and the `.tmpl` suffix.
///
/// Returns an empty vector if none of the host's install-root subtrees
/// exist (e.g. when the bundle is fresh and a host doesn't ship any
/// resources yet). The plan-print and Create/Overwrite classification
/// in [`crate::init`] handle the empty case cleanly.
///
/// # Errors
///
/// Returns [`RenderError::MiniJinjaRender`] when a template fails to
/// render (typically a missing context variable in strict mode or a
/// missing `{% include %}` target), [`RenderError::NonUtf8Template`]
/// when a bundle entry is not valid UTF-8, or
/// [`RenderError::BundleSubpathMissing`] when the per-host wrapper
/// subtree is absent entirely.
pub fn render_host_pack(host: HostChoice) -> Result<Vec<RenderedFile>, RenderError> {
    let mut env = build_environment();
    let ctx = host.template_context();
    let mut out: Vec<RenderedFile> = Vec::new();

    for install_root in host.install_roots() {
        let subpath = format!("agents/{install_root}");
        let Some(dir) = RESOURCES.get_dir(subpath.as_str()) else {
            // Missing per-host wrapper subtree is allowed: T-009/T-010
            // add `.codex/agents/` later, and `.codex/` may have no
            // skills today.
            continue;
        };
        let mut entries: Vec<&'static include_dir::File<'static>> = Vec::new();
        collect_tmpl_files(dir, &mut entries);
        entries.sort_by_key(|f| f.path());

        for file in entries {
            let bundle_path = file
                .path()
                .to_str()
                .ok_or_else(|| RenderError::NonUtf8Template {
                    template_name: file.path().display().to_string(),
                })?;
            let template_body =
                file.contents_utf8()
                    .ok_or_else(|| RenderError::NonUtf8Template {
                        template_name: bundle_path.to_owned(),
                    })?;
            let rendered = render_template(&mut env, bundle_path, template_body, &ctx)?;
            let rel_path = destination_rel_path(bundle_path)?;
            out.push(RenderedFile {
                rel_path,
                contents: rendered,
            });
        }
    }

    Ok(out)
}

/// Build a `MiniJinja` `Environment` configured to match Speccy's
/// wrapper expectations:
///
/// - Strict undefined behaviour so a missing context variable becomes a
///   render-time error instead of silently inserting an empty string.
/// - Trailing-newline preservation so the rendered output ends with the same
///   final byte as the source body (T-003 discovery).
/// - A loader rooted at `modules/...` inside the embedded [`RESOURCES`] bundle,
///   so wrappers can `{% include "modules/skills/speccy-<verb>.md" %}` and
///   resolve to the matching module body without needing on-disk template
///   files.
fn build_environment() -> Environment<'static> {
    let mut env = Environment::new();
    env.set_undefined_behavior(UndefinedBehavior::Strict);
    // T-003 discovery: the module body files start with a leading `\n`
    // and end with a trailing `\n`, matching the body slice that
    // `split_frontmatter` returns for the pre-SPEC-0016 per-host
    // SKILL.md files. The wrappers (`agents/.<host>/skills/.../SKILL.md.tmpl`)
    // are intentionally authored WITHOUT a trailing newline so the
    // module body's leading and trailing newlines are the only blank
    // lines straddling the include site, leaving the rendered output
    // byte-identical to the legacy SKILL.md files. `keep_trailing_newline
    // = true` preserves the absence of a trailing newline on the wrapper
    // and the presence of one on the module body.
    env.set_keep_trailing_newline(true);
    env.set_loader(load_from_resources);
    env
}

/// `set_loader` callback: resolves include names of the form
/// `modules/<sub>/<name>.md` (or any other path under the bundle root)
/// to the matching file inside [`RESOURCES`]. Returns `Ok(None)` when
/// the path does not resolve so `MiniJinja` can fall back to a
/// `TemplateNotFound` error with the included name.
fn load_from_resources(name: &str) -> Result<Option<String>, minijinja::Error> {
    // Defence in depth: reject directory-traversal segments before
    // looking the path up in the embedded bundle.
    for piece in name.split('/') {
        if piece == "." || piece == ".." || piece.contains('\\') {
            return Ok(None);
        }
    }
    let Some(file) = RESOURCES.get_file(name) else {
        return Ok(None);
    };
    let body = file.contents_utf8().ok_or_else(|| {
        minijinja::Error::new(
            minijinja::ErrorKind::InvalidOperation,
            format!("embedded include `{name}` is not valid UTF-8"),
        )
    })?;
    Ok(Some(body.to_owned()))
}

fn collect_tmpl_files(
    dir: &'static Dir<'static>,
    out: &mut Vec<&'static include_dir::File<'static>>,
) {
    for file in dir.files() {
        let is_template = file
            .path()
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("tmpl"));
        if is_template {
            out.push(file);
        }
    }
    for sub in dir.dirs() {
        collect_tmpl_files(sub, out);
    }
}

fn render_template(
    env: &mut Environment<'static>,
    template_name: &str,
    body: &str,
    ctx: &minijinja::Value,
) -> Result<String, RenderError> {
    env.add_template_owned(template_name.to_owned(), body.to_owned())
        .map_err(|err| RenderError::MiniJinjaRender {
            template_name: template_name.to_owned(),
            source: err,
        })?;
    let tmpl = env
        .get_template(template_name)
        .map_err(|err| RenderError::MiniJinjaRender {
            template_name: template_name.to_owned(),
            source: err,
        })?;
    tmpl.render(ctx)
        .map_err(|err| RenderError::MiniJinjaRender {
            template_name: template_name.to_owned(),
            source: err,
        })
}

/// Translate a bundle-relative template path
/// (`agents/.claude/skills/speccy-plan/SKILL.md.tmpl`) into a
/// project-root-relative destination path
/// (`.claude/skills/speccy-plan/SKILL.md`).
fn destination_rel_path(bundle_path: &str) -> Result<Utf8PathBuf, RenderError> {
    let stripped =
        bundle_path
            .strip_prefix("agents/")
            .ok_or_else(|| RenderError::BundleSubpathMissing {
                subpath: bundle_path.to_owned(),
            })?;
    let without_tmpl =
        stripped
            .strip_suffix(".tmpl")
            .ok_or_else(|| RenderError::BundleSubpathMissing {
                subpath: bundle_path.to_owned(),
            })?;
    Ok(Utf8PathBuf::from(without_tmpl))
}

#[cfg(test)]
mod tests {
    use super::destination_rel_path;
    use super::render_host_pack;
    use crate::host::HostChoice;

    #[test]
    fn destination_strips_agents_prefix_and_tmpl_suffix() {
        let out = destination_rel_path("agents/.claude/skills/speccy-plan/SKILL.md.tmpl")
            .expect("well-formed bundle path should translate");
        assert_eq!(out.as_str(), ".claude/skills/speccy-plan/SKILL.md");
    }

    #[test]
    fn destination_rejects_missing_agents_prefix() {
        let err = destination_rel_path("modules/skills/speccy-plan.md")
            .expect_err("non-agents prefix must be rejected");
        let msg = err.to_string();
        assert!(
            msg.contains("modules/skills/speccy-plan.md"),
            "error must name the bad path; got: {msg}",
        );
    }

    #[test]
    fn destination_rejects_missing_tmpl_suffix() {
        let err = destination_rel_path("agents/.claude/skills/speccy-plan/SKILL.md")
            .expect_err("non-.tmpl suffix must be rejected");
        let msg = err.to_string();
        assert!(
            msg.contains("SKILL.md"),
            "error must name the bad path; got: {msg}",
        );
    }

    #[test]
    fn render_host_pack_claude_code_emits_seven_skills() {
        let out = render_host_pack(HostChoice::ClaudeCode)
            .expect("render_host_pack(claude-code) should succeed");
        let skill_md_count = out
            .iter()
            .filter(|f| {
                f.rel_path.as_str().ends_with("/SKILL.md")
                    && f.rel_path.as_str().starts_with(".claude/skills/")
            })
            .count();
        assert_eq!(
            skill_md_count, 7,
            "claude-code host pack should render seven SKILL.md files; got {skill_md_count}",
        );
    }

    #[test]
    fn render_host_pack_codex_emits_seven_skills_under_dot_agents() {
        let out =
            render_host_pack(HostChoice::Codex).expect("render_host_pack(codex) should succeed");
        let skill_md_count = out
            .iter()
            .filter(|f| {
                f.rel_path.as_str().ends_with("/SKILL.md")
                    && f.rel_path.as_str().starts_with(".agents/skills/")
            })
            .count();
        assert_eq!(
            skill_md_count, 7,
            "codex host pack should render seven .agents/skills/.../SKILL.md files; got {skill_md_count}",
        );
    }

    #[test]
    fn render_host_pack_does_not_leak_cross_host_paths() {
        let out = render_host_pack(HostChoice::ClaudeCode)
            .expect("render_host_pack(claude-code) should succeed");
        for f in &out {
            assert!(
                !f.rel_path.as_str().starts_with(".agents/"),
                "claude-code host pack must not write to .agents/; got {}",
                f.rel_path,
            );
            assert!(
                !f.rel_path.as_str().starts_with(".codex/"),
                "claude-code host pack must not write to .codex/; got {}",
                f.rel_path,
            );
        }
    }

    #[test]
    fn render_host_pack_speccy_plan_contains_slash_prefixed_command() {
        let out = render_host_pack(HostChoice::ClaudeCode)
            .expect("render_host_pack(claude-code) should succeed");
        let plan = out
            .iter()
            .find(|f| f.rel_path.as_str() == ".claude/skills/speccy-plan/SKILL.md")
            .expect("claude-code render output must include speccy-plan");
        assert!(
            plan.contents.contains("/speccy-tasks"),
            "rendered speccy-plan SKILL.md must contain `/speccy-tasks`; got contents:\n{}",
            plan.contents,
        );
    }

    #[test]
    fn render_host_pack_codex_speccy_plan_uses_bare_command() {
        let out =
            render_host_pack(HostChoice::Codex).expect("render_host_pack(codex) should succeed");
        let plan = out
            .iter()
            .find(|f| f.rel_path.as_str() == ".agents/skills/speccy-plan/SKILL.md")
            .expect("codex render output must include speccy-plan");
        assert!(
            plan.contents.contains("speccy-tasks"),
            "rendered Codex speccy-plan SKILL.md must contain `speccy-tasks`; got:\n{}",
            plan.contents,
        );
        assert!(
            !plan.contents.contains("/speccy-tasks"),
            "rendered Codex speccy-plan SKILL.md must not contain `/speccy-tasks`; got:\n{}",
            plan.contents,
        );
    }
}
