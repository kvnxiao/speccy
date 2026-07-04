//! `speccy install` — render and manage repo-local harness packs
//! (DESIGN § Install Flow). Idempotent: create missing, repair missing, report
//! updates; never rewrite edited prose without `--update`/`--force`.

use std::collections::BTreeMap;
use std::io::IsTerminal;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::config::ProjectConfig;
use crate::error::{Result, SpeccyError};
use crate::render::{self, Harness, ManagedFile, PACK_VERSION};
use crate::store::write_atomic;

/// Parsed `speccy install` options (subset the command exposes).
pub struct InstallOptions {
    pub target: String,
    pub update: bool,
    pub dry_run: bool,
    pub yes: bool,
    pub check: bool,
    pub force: bool,
}

/// What we intend to do with one managed file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Create,
    UpToDate,
    Outdated,   // unmodified since install but the render changed
    Conflicted, // locally edited and the render changed
    Remove,     // previously managed, no longer rendered (e.g. removed persona)
}

struct Planned {
    path: String,
    contents: String,
    template_id: String,
    target: Option<Harness>,
    action: Action,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct PackLock {
    pack_version: String,
    #[serde(default)]
    files: Vec<LockEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LockEntry {
    path: String,
    target: String,
    template_id: String,
    rendered_hash: String,
}

/// Run the install command against a repo root. Returns the human-facing report.
pub fn run(repo_root: &Path, opts: &InstallOptions) -> Result<String> {
    let config = ProjectConfig::load(repo_root)?;
    let targets = detect_targets(repo_root, &opts.target)?;
    let lock = read_lock(repo_root)?;

    // Render every managed pack file for the selected targets.
    let mut rendered: Vec<Planned> = Vec::new();
    for target in &targets {
        for f in render::render_pack(*target, &config)? {
            rendered.push(classify(repo_root, &lock, f, Some(*target)));
        }
    }

    // Orphans: previously-managed files no longer in the render set.
    let rendered_paths: std::collections::HashSet<String> =
        rendered.iter().map(|p| p.path.clone()).collect();
    for entry in &lock.files {
        if rendered_paths.contains(&entry.path) || !targets.iter().any(|t| t.key() == entry.target)
        {
            continue;
        }
        // Only auto-remove an orphan (e.g. a removed persona) if it is still
        // unmodified since we rendered it; a locally edited file is left alone.
        match std::fs::read_to_string(repo_root.join(&entry.path)) {
            Ok(current) if hash(&current) == entry.rendered_hash => rendered.push(Planned {
                path: entry.path.clone(),
                contents: String::new(),
                template_id: entry.template_id.clone(),
                target: Harness::parse(&entry.target),
                action: Action::Remove,
            }),
            _ => {}
        }
    }

    if opts.check {
        return check_report(&rendered);
    }

    // Compute what will actually be written under the chosen mode.
    let to_write: Vec<&Planned> = rendered
        .iter()
        .filter(|p| should_write(p.action, opts))
        .collect();
    let project_yaml_missing = !repo_root.join(".speccy/project.yaml").exists();
    let gitignore_needs_block = !gitignore_has_block(repo_root)?;

    let report = preview(
        &rendered,
        &targets,
        project_yaml_missing,
        gitignore_needs_block,
        opts,
    );

    if opts.dry_run {
        return Ok(report);
    }
    let nothing_to_do = to_write.is_empty() && !project_yaml_missing && !gitignore_needs_block;
    if nothing_to_do {
        return Ok(report);
    }
    if !opts.yes && !confirm(&report)? {
        return Ok("Install aborted; nothing written.".to_string());
    }

    // Write.
    if project_yaml_missing {
        write_atomic(
            &repo_root.join(".speccy/project.yaml"),
            default_project_yaml().as_bytes(),
        )?;
    }
    if gitignore_needs_block {
        append_gitignore_block(repo_root)?;
    }
    for p in &to_write {
        match p.action {
            Action::Remove => {
                let _ = std::fs::remove_file(repo_root.join(&p.path));
            }
            Action::Conflicted if opts.update && !opts.force => {
                write_conflict(repo_root, p)?;
            }
            _ => write_atomic(&repo_root.join(&p.path), p.contents.as_bytes())?,
        }
    }

    // Rewrite the pack lock to reflect the on-disk managed set.
    write_lock(repo_root, &rendered, &lock, opts)?;

    Ok(format!("{report}\n\nInstall OK. Commit these workflow artifacts to share the workflow; runtime state lives in ~/.speccy/ only."))
}

fn classify(repo_root: &Path, lock: &PackLock, f: ManagedFile, target: Option<Harness>) -> Planned {
    let new_hash = hash(&f.contents);
    let disk = std::fs::read_to_string(repo_root.join(&f.path)).ok();
    let recorded = lock
        .files
        .iter()
        .find(|e| e.path == f.path)
        .map(|e| e.rendered_hash.clone());
    let action = match disk {
        None => Action::Create,
        Some(current) => {
            let disk_hash = hash(&current);
            if disk_hash == new_hash {
                Action::UpToDate
            } else if recorded.as_deref() == Some(disk_hash.as_str()) {
                Action::Outdated
            } else {
                Action::Conflicted
            }
        }
    };
    Planned {
        path: f.path,
        contents: f.contents,
        template_id: f.template_id,
        target,
        action,
    }
}

fn should_write(action: Action, opts: &InstallOptions) -> bool {
    match action {
        Action::Create => true,
        Action::UpToDate => false,
        Action::Outdated => opts.update || opts.force,
        Action::Conflicted => opts.update || opts.force,
        // Orphans are added only when unmodified, so removing them is always safe.
        Action::Remove => true,
    }
}

fn detect_targets(repo_root: &Path, target: &str) -> Result<Vec<Harness>> {
    let has_codex = repo_root.join(".codex").exists() || repo_root.join(".agents").exists();
    let has_claude = repo_root.join(".claude").exists();
    let targets = match target {
        "codex" => vec![Harness::Codex],
        "claude" => vec![Harness::Claude],
        "all" => vec![Harness::Claude, Harness::Codex],
        "auto" => {
            let mut t = Vec::new();
            if has_claude {
                t.push(Harness::Claude);
            }
            if has_codex {
                t.push(Harness::Codex);
            }
            t
        }
        other => {
            return Err(SpeccyError::validation(format!(
                "unknown target `{other}`; use auto | codex | claude | all"
            )))
        }
    };
    if targets.is_empty() {
        return Err(SpeccyError::validation(
            "no supported harness detected; choose --target codex|claude|all",
        ));
    }
    Ok(targets)
}

fn preview(
    rendered: &[Planned],
    targets: &[Harness],
    project_yaml_missing: bool,
    gitignore_needs_block: bool,
    opts: &InstallOptions,
) -> String {
    let mut out = format!(
        "Detected harnesses: {}\nRendering pack @ {PACK_VERSION}\n\n",
        targets
            .iter()
            .map(|t| t.key())
            .collect::<Vec<_>>()
            .join(", ")
    );
    let mut lines = Vec::new();
    if project_yaml_missing {
        lines.push("  create  .speccy/project.yaml".to_string());
    }
    lines.push("  write   .speccy/pack-lock.yaml".to_string());
    for p in rendered {
        let verb = match p.action {
            Action::Create => "create",
            Action::UpToDate => "ok    ",
            Action::Outdated if should_write(p.action, opts) => "update",
            Action::Outdated => "stale ",
            Action::Conflicted if opts.force => "force ",
            Action::Conflicted if opts.update => "conflict",
            Action::Conflicted => "modified",
            Action::Remove if should_write(p.action, opts) => "remove",
            Action::Remove => "orphan",
        };
        lines.push(format!("  {verb}  {}", p.path));
    }
    if gitignore_needs_block {
        lines.push("  update  .gitignore  (defensive .speccy/ block)".to_string());
    }
    out.push_str(&lines.join("\n"));
    out
}

fn check_report(rendered: &[Planned]) -> Result<String> {
    let drift: Vec<&Planned> = rendered
        .iter()
        .filter(|p| !matches!(p.action, Action::UpToDate))
        .collect();
    if drift.is_empty() {
        Ok("packs OK; all managed files match the pack lock.".to_string())
    } else {
        let detail = drift
            .iter()
            .map(|p| format!("  {:?}  {}", p.action, p.path))
            .collect::<Vec<_>>()
            .join("\n");
        Err(SpeccyError::validation(format!(
            "pack drift detected ({} files):\n{detail}",
            drift.len()
        )))
    }
}

fn confirm(report: &str) -> Result<bool> {
    if !std::io::stdin().is_terminal() {
        return Err(SpeccyError::validation(
            "this install would write files; re-run with --yes (noninteractive) or --dry-run",
        ));
    }
    println!("{report}\n\nProceed? [y/N]");
    let mut line = String::new();
    std::io::stdin()
        .read_line(&mut line)
        .map_err(|e| SpeccyError::io(format!("failed to read confirmation: {e}")))?;
    Ok(matches!(line.trim(), "y" | "Y" | "yes"))
}

fn write_conflict(repo_root: &Path, p: &Planned) -> Result<()> {
    // Preserve the local file; write the proposed render for review.
    let dest = repo_root.join(".speccy/pack-updates/latest").join(&p.path);
    write_atomic(&dest, p.contents.as_bytes())?;
    Ok(())
}

// --- pack lock ---

fn lock_path(repo_root: &Path) -> std::path::PathBuf {
    repo_root.join(".speccy/pack-lock.yaml")
}

fn read_lock(repo_root: &Path) -> Result<PackLock> {
    match std::fs::read_to_string(lock_path(repo_root)) {
        Ok(text) => serde_saphyr::from_str(&text)
            .map_err(|e| SpeccyError::io(format!("corrupt pack-lock.yaml: {e}"))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(PackLock::default()),
        Err(e) => Err(e.into()),
    }
}

fn write_lock(
    repo_root: &Path,
    rendered: &[Planned],
    old: &PackLock,
    opts: &InstallOptions,
) -> Result<()> {
    // The lock records the managed files that are (or remain) on disk.
    let mut entries: BTreeMap<String, LockEntry> = BTreeMap::new();
    for e in &old.files {
        entries.insert(e.path.clone(), e.clone());
    }
    for p in rendered {
        match p.action {
            Action::Remove if should_write(p.action, opts) => {
                entries.remove(&p.path);
            }
            Action::Remove => {}
            Action::Conflicted if opts.update && !opts.force => { /* file unchanged on disk */ }
            _ if should_write(p.action, opts) || p.action == Action::UpToDate => {
                entries.insert(
                    p.path.clone(),
                    LockEntry {
                        path: p.path.clone(),
                        target: p.target.map(|t| t.key().to_string()).unwrap_or_default(),
                        template_id: p.template_id.clone(),
                        rendered_hash: hash(&p.contents),
                    },
                );
            }
            _ => {}
        }
    }
    let lock = PackLock {
        pack_version: PACK_VERSION.to_string(),
        files: entries.into_values().collect(),
    };
    let text = render_lock_yaml(&lock);
    write_atomic(&lock_path(repo_root), text.as_bytes())
}

/// Render the pack lock as readable YAML (deterministic key order).
fn render_lock_yaml(lock: &PackLock) -> String {
    let mut out = format!("pack_version: \"{}\"\nfiles:\n", lock.pack_version);
    for e in &lock.files {
        out.push_str(&format!(
            "  - path: {}\n    target: {}\n    template_id: {}\n    rendered_hash: {}\n",
            e.path, e.target, e.template_id, e.rendered_hash
        ));
    }
    out
}

// --- gitignore + project.yaml ---

const GITIGNORE_BLOCK: &str = "\
# Speccy runtime state must never live in a repo (DESIGN § Git Policy).
.speccy/*
!.speccy/project.yaml
!.speccy/pack-lock.yaml
";

fn gitignore_has_block(repo_root: &Path) -> Result<bool> {
    match std::fs::read_to_string(repo_root.join(".gitignore")) {
        Ok(text) => Ok(text.contains("!.speccy/project.yaml")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}

fn append_gitignore_block(repo_root: &Path) -> Result<()> {
    let path = repo_root.join(".gitignore");
    let mut text = std::fs::read_to_string(&path).unwrap_or_default();
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push('\n');
    text.push_str(GITIGNORE_BLOCK);
    write_atomic(&path, text.as_bytes())
}

fn default_project_yaml() -> String {
    let cfg = ProjectConfig::default();
    let personas = cfg
        .review
        .personas
        .iter()
        .map(|p| format!("    - name: {}", p.name))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "risk_default: {}\ncaps:\n  task_repair_rounds: {}\n  run_review_rounds: {}\n  structured_output_retries: {}\n  max_tasks: null\n  max_run_wall_clock_minutes: null\nevidence:\n  command_timeout_seconds: {}\n  command_output_max_bytes: {}\n  command_policy:\n    allow: []\nreview:\n  personas:\n{}\nprovenance:\n  extra_terms: []\n",
        cfg.risk_default,
        cfg.caps.task_repair_rounds,
        cfg.caps.run_review_rounds,
        cfg.caps.structured_output_retries,
        cfg.evidence.command_timeout_seconds,
        cfg.evidence.command_output_max_bytes,
        personas,
    )
}

fn hash(contents: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(contents.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}
