//! Domain types and the canonical enum vocabularies.
//!
//! Enum *values* are owned by `DESIGN.md` with the state machine that owns
//! them; `TERMINOLOGY.md` names the vocabularies. This module is the single
//! Rust home for those values. Payload shapes are owned by `SCHEMAS.md`.

use serde::{Deserialize, Serialize};

// --------------------------------------------------------------------------
// Enum vocabularies
// --------------------------------------------------------------------------

/// Run state — a single flat enum (DESIGN § Spec Draft and Run State).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunState {
    Implementing,
    Verifying,
    Verified,
    Submitted,
    Landed,
    Escalated,
    Cancelled,
}

impl RunState {
    /// Active states can still transition to `cancelled` and hold a lease.
    pub fn is_active(self) -> bool {
        matches!(
            self,
            RunState::Implementing | RunState::Verifying | RunState::Verified
        )
    }

    /// The snake_case wire string (matches the serde representation).
    pub fn as_str(self) -> &'static str {
        match self {
            RunState::Implementing => "implementing",
            RunState::Verifying => "verifying",
            RunState::Verified => "verified",
            RunState::Submitted => "submitted",
            RunState::Landed => "landed",
            RunState::Escalated => "escalated",
            RunState::Cancelled => "cancelled",
        }
    }
}

/// Task status (DESIGN § Task).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Queued,
    Building,
    InReview,
    Integrated,
    Deferred,
}

/// Requirement status — six canonical values (DESIGN § Requirement Resolution Rules).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequirementStatus {
    /// Evidence not collected yet (initial status).
    #[default]
    Pending,
    Passed,
    ReviewPassed,
    Failed,
    Blocked,
    Waived,
}

impl RequirementStatus {
    /// A requirement is *resolved* when it is `passed`, `review_passed`, or
    /// `waived` (DESIGN § Requirement Resolution Rules).
    pub fn is_resolved(self) -> bool {
        matches!(
            self,
            RequirementStatus::Passed | RequirementStatus::ReviewPassed | RequirementStatus::Waived
        )
    }
}

/// Risk tier (DESIGN § Acceptance Ledger). Controls evidence strictness and
/// the number of gates, not the workflow shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskTier {
    Minimal,
    Standard,
    High,
    Critical,
}

impl RiskTier {
    /// Parse the tier from its wire string, if valid.
    pub fn parse(s: &str) -> Option<RiskTier> {
        match s {
            "minimal" => Some(RiskTier::Minimal),
            "standard" => Some(RiskTier::Standard),
            "high" => Some(RiskTier::High),
            "critical" => Some(RiskTier::Critical),
            _ => None,
        }
    }

    /// The snake_case wire string (matches the serde representation).
    pub fn as_str(self) -> &'static str {
        match self {
            RiskTier::Minimal => "minimal",
            RiskTier::Standard => "standard",
            RiskTier::High => "high",
            RiskTier::Critical => "critical",
        }
    }
}

/// Spec status — controls whether an old spec is a planning candidate
/// (TERMINOLOGY § Spec status).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpecStatus {
    Draft,
    Approved,
    Cancelled,
    Accepted,
    Superseded,
    Archived,
}

/// Directive action — closed vocabulary (DESIGN § Deterministic Loop Driving).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectiveAction {
    ClaimTask,
    DispatchWorker,
    DispatchVerifier,
    AwaitHumanGate,
    Halt,
}

/// The gate a `await_human_gate` directive is parked at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Gate {
    ShipDecision,
    Escalation,
    AcceptedRiskConfirmation,
}

/// Round scope (`task` per-task repair, `run` run-gate review).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoundScope {
    Task,
    Run,
}

/// Evidence kind (DESIGN § Acceptance Ledger).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    Command,
    Review,
    Browser,
    Api,
    Manual,
}

impl EvidenceKind {
    pub fn parse(s: &str) -> Option<EvidenceKind> {
        match s {
            "command" => Some(EvidenceKind::Command),
            "review" => Some(EvidenceKind::Review),
            "browser" => Some(EvidenceKind::Browser),
            "api" => Some(EvidenceKind::Api),
            "manual" => Some(EvidenceKind::Manual),
            _ => None,
        }
    }
}

/// Finding severity (`SCHEMAS.md` § finding record).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingSeverity {
    Blocking,
    Advisory,
    Positive,
    Uncertain,
}

impl FindingSeverity {
    pub fn parse(s: &str) -> Option<FindingSeverity> {
        match s {
            "blocking" => Some(FindingSeverity::Blocking),
            "advisory" => Some(FindingSeverity::Advisory),
            "positive" => Some(FindingSeverity::Positive),
            "uncertain" => Some(FindingSeverity::Uncertain),
            _ => None,
        }
    }
}

// --------------------------------------------------------------------------
// Spec draft payloads (SCHEMAS § spec record-draft / patch-draft)
// --------------------------------------------------------------------------

/// One candidate spec revision. Fields are lenient (Option) so structural lint
/// can report missing sections and invalid values rather than failing to parse.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpecDraft {
    pub goal: Option<String>,
    pub scope: Option<Scope>,
    pub risk: Option<String>,
    pub assumptions: Option<Vec<String>>,
    pub non_goals: Option<Vec<String>>,
    pub observations: Option<Vec<String>>,
    pub open_questions: Option<Vec<String>>,
    pub requirements: Option<Vec<Requirement>>,
    pub tasks: Option<Vec<TaskDef>>,
}

impl SpecDraft {
    pub fn requirements(&self) -> &[Requirement] {
        self.requirements.as_deref().unwrap_or(&[])
    }
    pub fn tasks(&self) -> &[TaskDef] {
        self.tasks.as_deref().unwrap_or(&[])
    }
    pub fn requirement(&self, id: &str) -> Option<&Requirement> {
        self.requirements().iter().find(|r| r.id == id)
    }
    pub fn risk_tier(&self) -> Option<RiskTier> {
        self.risk.as_deref().and_then(RiskTier::parse)
    }

    /// Merge the `set` sections of a patch over this draft; only present
    /// (`Some`) sections are replaced (SCHEMAS § spec patch-draft).
    pub fn apply_patch(&mut self, patch: SpecDraft) {
        if patch.goal.is_some() {
            self.goal = patch.goal;
        }
        if patch.scope.is_some() {
            self.scope = patch.scope;
        }
        if patch.risk.is_some() {
            self.risk = patch.risk;
        }
        if patch.assumptions.is_some() {
            self.assumptions = patch.assumptions;
        }
        if patch.non_goals.is_some() {
            self.non_goals = patch.non_goals;
        }
        if patch.observations.is_some() {
            self.observations = patch.observations;
        }
        if patch.open_questions.is_some() {
            self.open_questions = patch.open_questions;
        }
        if patch.requirements.is_some() {
            self.requirements = patch.requirements;
        }
        if patch.tasks.is_some() {
            self.tasks = patch.tasks;
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Scope {
    #[serde(default, rename = "in")]
    pub in_: Vec<String>,
    #[serde(default)]
    pub out: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Requirement {
    pub id: String,
    pub statement: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scenario: Option<Scenario>,
    #[serde(default)]
    pub evidence: Vec<EvidenceRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub given: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub then: Option<String>,
}

/// A declared evidence request. Extended browser/api fields (setup, steps,
/// vacuity) are accepted and ignored for now; MVP does not execute them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceRequest {
    pub id: String,
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl EvidenceRequest {
    pub fn kind_enum(&self) -> Option<EvidenceKind> {
        self.kind.as_deref().and_then(EvidenceKind::parse)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDef {
    pub id: String,
    pub title: Option<String>,
    #[serde(default)]
    pub requirements: Vec<String>,
    #[serde(default)]
    pub constraints: Vec<String>,
}

// --------------------------------------------------------------------------
// Change reference (SCHEMAS § run record-ship)
// --------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeRef {
    pub kind: String, // pull_request | branch | patch | none
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head_sha: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,
}
