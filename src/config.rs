//! `.speccy/project.yaml` — machine-readable policy the controller enforces
//! (DESIGN § Harness-Native Install Packs, full schema). Missing file → defaults.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{Result, SpeccyError};
use crate::model::RiskTier;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectConfig {
    pub risk_default: String,
    pub caps: Caps,
    pub evidence: EvidenceConfig,
    pub review: ReviewConfig,
    pub provenance: ProvenanceConfig,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            risk_default: "standard".into(),
            caps: Caps::default(),
            evidence: EvidenceConfig::default(),
            review: ReviewConfig::default(),
            provenance: ProvenanceConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Caps {
    pub task_repair_rounds: u32,
    pub run_review_rounds: u32,
    pub structured_output_retries: u32,
    pub max_tasks: Option<u32>,
    pub max_run_wall_clock_minutes: Option<u32>,
}

impl Default for Caps {
    fn default() -> Self {
        Self {
            task_repair_rounds: 3,
            run_review_rounds: 3,
            structured_output_retries: 3,
            max_tasks: None,
            max_run_wall_clock_minutes: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EvidenceConfig {
    pub command_timeout_seconds: u64,
    pub command_output_max_bytes: u64,
    pub command_policy: CommandPolicy,
}

impl Default for EvidenceConfig {
    fn default() -> Self {
        Self {
            command_timeout_seconds: 600,
            command_output_max_bytes: 1_048_576,
            command_policy: CommandPolicy::default(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CommandPolicy {
    /// Whole-command glob patterns; empty = any approved command may run.
    pub allow: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ReviewConfig {
    pub personas: Vec<Persona>,
}

impl Default for ReviewConfig {
    fn default() -> Self {
        Self {
            personas: default_roster(),
        }
    }
}

/// The default reviewer roster (DESIGN § Reviewer Personas).
pub fn default_roster() -> Vec<Persona> {
    ["spec-fidelity", "defects", "security", "style"]
        .into_iter()
        .map(|name| Persona {
            name: name.into(),
            model: None,
            min_risk: None,
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Persona {
    pub name: String,
    /// Optional model: a plain string, or a map keyed by target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<serde_json::Value>,
    /// Persona joins only at this tier or above.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_risk: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProvenanceConfig {
    pub extra_terms: Vec<String>,
}

impl ProjectConfig {
    /// Load `<workspace_root>/.speccy/project.yaml`, or defaults if absent.
    pub fn load(workspace_root: &Path) -> Result<ProjectConfig> {
        let path = workspace_root.join(".speccy").join("project.yaml");
        match std::fs::read_to_string(&path) {
            Ok(text) => serde_saphyr::from_str(&text).map_err(|e| {
                SpeccyError::validation(format!("failed to parse {}: {e}", path.display()))
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(ProjectConfig::default()),
            Err(e) => Err(SpeccyError::io(format!(
                "failed to read {}: {e}",
                path.display()
            ))),
        }
    }

    pub fn risk_default_tier(&self) -> RiskTier {
        RiskTier::parse(&self.risk_default).unwrap_or(RiskTier::Standard)
    }

    /// Persona names to fan out at the given tier (DESIGN § Reviewer Personas).
    /// `minimal` collapses to one combined reviewer; `standard`+ runs the full
    /// configured roster minus personas gated above the tier.
    pub fn roster_for(&self, tier: RiskTier) -> Vec<String> {
        if tier == RiskTier::Minimal {
            return vec!["combined".to_string()];
        }
        self.review
            .personas
            .iter()
            .filter(|p| match p.min_risk.as_deref().and_then(RiskTier::parse) {
                Some(min) => tier >= min,
                None => true,
            })
            .map(|p| p.name.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_when_absent() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = ProjectConfig::load(dir.path()).unwrap();
        assert_eq!(cfg.caps.task_repair_rounds, 3);
        assert_eq!(cfg.risk_default_tier(), RiskTier::Standard);
        assert_eq!(cfg.roster_for(RiskTier::High).len(), 4);
        assert_eq!(
            cfg.roster_for(RiskTier::Minimal),
            vec!["combined".to_string()]
        );
    }

    #[test]
    fn min_risk_gates_persona() {
        let mut cfg = ProjectConfig::default();
        cfg.review.personas.push(Persona {
            name: "perf".into(),
            model: None,
            min_risk: Some("high".into()),
        });
        assert!(!cfg
            .roster_for(RiskTier::Standard)
            .contains(&"perf".to_string()));
        assert!(cfg.roster_for(RiskTier::High).contains(&"perf".to_string()));
    }
}
