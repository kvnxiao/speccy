//! Domain types and the canonical enum vocabularies.
//!
//! Enum *values* are owned by `DESIGN.md` with the state machine that owns
//! them; `TERMINOLOGY.md` names the vocabularies. This module is the single
//! Rust home for those values. Payload shapes are owned by `SCHEMAS.md`.

use serde::Deserialize;
use serde::Serialize;

// --------------------------------------------------------------------------
// Enum vocabularies
// --------------------------------------------------------------------------

/// Run state — a single flat enum (DESIGN § Spec Draft and Run State).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunState {
    /// Tasks are being claimed and built.
    Implementing,
    /// Requirements are under verification.
    Verifying,
    /// All requirements resolved; parked at the ship gate.
    Verified,
    /// A change has been submitted for landing.
    Submitted,
    /// The change has landed.
    Landed,
    /// Escalated to a human after exhausting a cap.
    Escalated,
    /// Cancelled before completion.
    Cancelled,
}

impl RunState {
    /// Active states can still transition to `cancelled` and hold a lease.
    #[must_use = "returns the active check without side effects"]
    pub fn is_active(self) -> bool {
        matches!(
            self,
            RunState::Implementing | RunState::Verifying | RunState::Verified
        )
    }

    /// The `snake_case` wire string (matches the serde representation).
    #[must_use = "returns the wire string without side effects"]
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
    /// Not yet claimed.
    Queued,
    /// Claimed and under implementation.
    Building,
    /// Implementation complete; under reviewer fan-out.
    InReview,
    /// Review passed and folded into the run.
    Integrated,
}

impl TaskStatus {
    /// The `snake_case` wire string (matches the serde representation).
    #[must_use = "returns the wire string without side effects"]
    pub fn as_str(self) -> &'static str {
        match self {
            TaskStatus::Queued => "queued",
            TaskStatus::Building => "building",
            TaskStatus::InReview => "in_review",
            TaskStatus::Integrated => "integrated",
        }
    }
}

/// Requirement status — six canonical values (DESIGN § Requirement Resolution
/// Rules).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequirementStatus {
    /// Evidence not collected yet (initial status).
    #[default]
    Pending,
    /// Automated evidence passed.
    Passed,
    /// Cleared by reviewer judgement rather than automated evidence.
    ReviewPassed,
    /// Evidence collected and did not pass.
    Failed,
    /// Cannot be evaluated; a dependency or precondition is unmet.
    Blocked,
    /// Deliberately excused by a human.
    Waived,
}

impl RequirementStatus {
    /// A requirement is *resolved* when it is `passed`, `review_passed`, or
    /// `waived` (DESIGN § Requirement Resolution Rules).
    #[must_use = "returns the resolved check without side effects"]
    pub fn is_resolved(self) -> bool {
        matches!(
            self,
            RequirementStatus::Passed | RequirementStatus::ReviewPassed | RequirementStatus::Waived
        )
    }

    /// The `snake_case` wire string (matches the serde representation).
    #[must_use = "returns the wire string without side effects"]
    pub fn as_str(self) -> &'static str {
        match self {
            RequirementStatus::Pending => "pending",
            RequirementStatus::Passed => "passed",
            RequirementStatus::ReviewPassed => "review_passed",
            RequirementStatus::Failed => "failed",
            RequirementStatus::Blocked => "blocked",
            RequirementStatus::Waived => "waived",
        }
    }
}

/// Risk tier (DESIGN § Acceptance Ledger). Controls evidence strictness and
/// the number of gates, not the workflow shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskTier {
    /// Lowest strictness; fewest gates.
    Minimal,
    /// Default tier.
    Standard,
    /// Elevated strictness.
    High,
    /// Highest strictness; most gates.
    Critical,
}

impl RiskTier {
    /// Parse the tier from its wire string, if valid.
    #[must_use = "the parsed tier is useless if discarded"]
    pub fn parse(s: &str) -> Option<RiskTier> {
        match s {
            "minimal" => Some(RiskTier::Minimal),
            "standard" => Some(RiskTier::Standard),
            "high" => Some(RiskTier::High),
            "critical" => Some(RiskTier::Critical),
            _ => None,
        }
    }

    /// The `snake_case` wire string (matches the serde representation).
    #[must_use = "returns the wire string without side effects"]
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
    /// Editable candidate, not yet approved.
    Draft,
    /// Approved for a run.
    Approved,
    /// Abandoned before acceptance.
    Cancelled,
    /// Requirements accepted after a successful run.
    Accepted,
    /// Replaced by a newer spec.
    Superseded,
    /// Retired from the planning-candidate set.
    Archived,
}

impl SpecStatus {
    /// The `snake_case` wire string (matches the serde representation).
    #[must_use = "returns the wire string without side effects"]
    pub fn as_str(self) -> &'static str {
        match self {
            SpecStatus::Draft => "draft",
            SpecStatus::Approved => "approved",
            SpecStatus::Cancelled => "cancelled",
            SpecStatus::Accepted => "accepted",
            SpecStatus::Superseded => "superseded",
            SpecStatus::Archived => "archived",
        }
    }
}

/// Directive action — closed vocabulary (DESIGN § Deterministic Loop Driving).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectiveAction {
    /// Claim the next queued task.
    ClaimTask,
    /// Dispatch a worker to implement a claimed task.
    DispatchWorker,
    /// Dispatch a verifier to collect requirement evidence.
    DispatchVerifier,
    /// Park at a human gate.
    AwaitHumanGate,
    /// Stop; there is nothing more to do.
    Halt,
}

/// The gate a `await_human_gate` directive is parked at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Gate {
    /// Human decides whether to ship the verified change.
    ShipDecision,
    /// Human intervention after a cap is exhausted.
    Escalation,
    /// Human confirmation of an accepted-risk waiver.
    AcceptedRiskConfirmation,
}

/// Round scope (`task` per-task repair, `run` run-gate review).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoundScope {
    /// Per-task repair round.
    Task,
    /// Run-gate review round.
    Run,
}

/// Evidence kind (DESIGN § Acceptance Ledger).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    /// A shell command whose exit status is the evidence.
    Command,
    /// Reviewer judgement.
    Review,
    /// A browser-driven check.
    Browser,
    /// An API-level check.
    Api,
    /// Human-attested evidence.
    Manual,
}

impl EvidenceKind {
    /// Parse the kind from its wire string, if valid.
    #[must_use = "the parsed kind is useless if discarded"]
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
    /// Must be resolved before shipping.
    Blocking,
    /// Non-blocking suggestion.
    Advisory,
    /// Positive observation, not a defect.
    Positive,
    /// Reviewer is unsure; needs a human look.
    Uncertain,
}

impl FindingSeverity {
    /// Parse the severity from its wire string, if valid.
    #[must_use = "the parsed severity is useless if discarded"]
    pub fn parse(s: &str) -> Option<FindingSeverity> {
        match s {
            "blocking" => Some(FindingSeverity::Blocking),
            "advisory" => Some(FindingSeverity::Advisory),
            "positive" => Some(FindingSeverity::Positive),
            "uncertain" => Some(FindingSeverity::Uncertain),
            _ => None,
        }
    }

    /// The `snake_case` wire string (matches the serde representation).
    #[must_use = "returns the wire string without side effects"]
    pub fn as_str(self) -> &'static str {
        match self {
            FindingSeverity::Blocking => "blocking",
            FindingSeverity::Advisory => "advisory",
            FindingSeverity::Positive => "positive",
            FindingSeverity::Uncertain => "uncertain",
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
    /// The declared requirements, or an empty slice if none.
    #[must_use = "returns the requirements without side effects"]
    pub fn requirements(&self) -> &[Requirement] {
        self.requirements.as_deref().unwrap_or(&[])
    }
    /// The declared tasks, or an empty slice if none.
    #[must_use = "returns the tasks without side effects"]
    pub fn tasks(&self) -> &[TaskDef] {
        self.tasks.as_deref().unwrap_or(&[])
    }
    /// The requirement with the given id, if present.
    #[must_use = "returns the requirement without side effects"]
    pub fn requirement(&self, id: &str) -> Option<&Requirement> {
        self.requirements().iter().find(|r| r.id == id)
    }
    /// The parsed risk tier, or `None` if unset or invalid.
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

/// In-scope and out-of-scope bullet lists.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Scope {
    #[serde(default, rename = "in")]
    pub in_: Vec<String>,
    #[serde(default)]
    pub out: Vec<String>,
}

/// One acceptance requirement with its declared evidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Requirement {
    pub id: String,
    pub statement: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scenario: Option<Scenario>,
    #[serde(default)]
    pub evidence: Vec<EvidenceRequest>,
}

/// A Given/When/Then scenario attached to a requirement.
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
    /// The parsed evidence kind, or `None` if unset or invalid.
    pub fn kind_enum(&self) -> Option<EvidenceKind> {
        self.kind.as_deref().and_then(EvidenceKind::parse)
    }
}

/// One planned task and the requirements it targets.
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

/// A reference to the change produced by a run.
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
