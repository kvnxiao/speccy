//! Controller operation dispatch.
//!
//! Routes a parsed `speccy ctl` command to its handler and returns a JSON
//! value for the `data` field of the response envelope. Behavior is owned by
//! `DESIGN.md`; payload shapes by `SCHEMAS.md`.
//!
//! M0: every operation is wired to the CLI surface but returns a structured
//! `not_implemented` error. Later milestones replace these arms with real
//! controller logic.

use crate::cli::{
    CtlCommand, EvidenceOp, FindingOp, PacketOp, RequirementOp, RunOp, SpecOp, TaskOp,
};
use crate::error::{Result, SpeccyError};

/// Dispatch a controller operation, returning its `data` payload.
pub fn dispatch(command: CtlCommand) -> Result<serde_json::Value> {
    match command {
        CtlCommand::Spec(op) => spec(op),
        CtlCommand::Run(op) => run(op),
        CtlCommand::Task(op) => task(op),
        CtlCommand::Packet(op) => packet(op),
        CtlCommand::Evidence(op) => evidence(op),
        CtlCommand::Finding(op) => finding(op),
        CtlCommand::Requirement(op) => requirement(op),
    }
}

fn spec(op: SpecOp) -> Result<serde_json::Value> {
    let name = match op {
        SpecOp::Start(_) => "spec start",
        SpecOp::Status(_) => "spec status",
        SpecOp::RecordDraft(_) => "spec record-draft",
        SpecOp::PatchDraft(_) => "spec patch-draft",
        SpecOp::RecordDecision(_) => "spec record-decision",
    };
    not_implemented(name)
}

fn run(op: RunOp) -> Result<serde_json::Value> {
    let name = match op {
        RunOp::Start(_) => "run start",
        RunOp::Status(_) => "run status",
        RunOp::Next(_) => "run next",
        RunOp::RecordDecision(_) => "run record-decision",
        RunOp::RecordShip(_) => "run record-ship",
    };
    not_implemented(name)
}

fn task(op: TaskOp) -> Result<serde_json::Value> {
    let name = match op {
        TaskOp::Claim(_) => "task claim",
        TaskOp::RecordHandoff(_) => "task record-handoff",
    };
    not_implemented(name)
}

fn packet(op: PacketOp) -> Result<serde_json::Value> {
    let name = match op {
        PacketOp::Planning(_) => "packet planning",
        PacketOp::Task(_) => "packet task",
        PacketOp::Verification(_) => "packet verification",
        PacketOp::Review(_) => "packet review",
        PacketOp::Escalation(_) => "packet escalation",
    };
    not_implemented(name)
}

fn evidence(op: EvidenceOp) -> Result<serde_json::Value> {
    let name = match op {
        EvidenceOp::Collect(_) => "evidence collect",
        EvidenceOp::Record(_) => "evidence record",
    };
    not_implemented(name)
}

fn finding(op: FindingOp) -> Result<serde_json::Value> {
    let FindingOp::Record(_) = op;
    not_implemented("finding record")
}

fn requirement(op: RequirementOp) -> Result<serde_json::Value> {
    let RequirementOp::SetStatus(_) = op;
    not_implemented("requirement set-status")
}

fn not_implemented(op: &str) -> Result<serde_json::Value> {
    Err(SpeccyError::not_implemented(format!(
        "controller operation `{op}` is not implemented yet"
    )))
}
