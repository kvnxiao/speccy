//! Serde `Serialize` structs for the `speccy context` bundle envelope.
//!
//! `speccy context <task-selector> --json` emits one schema-versioned
//! JSON bundle scoped to a single task. This module owns the envelope's
//! `Serialize` shape; assembly from parsed workspace state lives in
//! [`crate::context`].
//!
//! The envelope follows the workspace-wide convention established by
//! `next_output.rs`: `schema_version` is the first serialized field,
//! pinned at `1` pre-v1. SPEC-0056 grows this envelope across tasks
//! T-002..T-006; this file carries the T-002 slice — spec identity
//! (REQ-001's `schema_version` + REQ-002's id/title/status) and the
//! intent block (REQ-002's goals / non-goals / decisions). Later tasks
//! add the task entry, covering requirements, journal, sibling index,
//! paths, and consistency sections.
//!
//! See `.speccy/specs/0056-task-context-bundle/SPEC.md`.

use serde::Serialize;

/// The `speccy context` JSON bundle envelope.
///
/// `schema_version` is declared first so it is the first serialized
/// field, matching the `next` / `status` / `journal show` envelopes.
#[derive(Debug, Clone, Serialize)]
pub struct ContextBundle {
    /// Schema version. Pinned at `1` pre-v1.
    pub schema_version: u32,
    /// Spec identity from SPEC.md frontmatter (REQ-002).
    pub spec: SpecIdentity,
    /// Authoring-intent slice: goals, non-goals, decisions (REQ-002).
    pub intent: Intent,
}

/// Spec identity drawn from SPEC.md frontmatter (REQ-002).
#[derive(Debug, Clone, Serialize)]
pub struct SpecIdentity {
    /// Frontmatter `id` (`SPEC-NNNN`).
    pub id: String,
    /// Frontmatter `title`.
    pub title: String,
    /// Frontmatter `status`, in its on-disk string form (e.g.
    /// `in-progress`).
    pub status: String,
}

/// The authoring-intent slice of the bundle (REQ-002).
///
/// Carries the `<goals>` and `<non-goals>` bodies verbatim plus every
/// `<decision>` with its DEC id and body. The Summary narrative,
/// `<user-stories>`, Notes, and non-covered requirement bodies are
/// deliberately excluded — they are not part of the task-scoped intent
/// slice.
#[derive(Debug, Clone, Serialize)]
pub struct Intent {
    /// Body of the `<goals>` element, verbatim.
    pub goals: String,
    /// Body of the `<non-goals>` element, verbatim.
    pub non_goals: String,
    /// Every `<decision>` in declared order.
    pub decisions: Vec<DecisionEntry>,
}

/// One `<decision>` projected into the bundle (REQ-002).
#[derive(Debug, Clone, Serialize)]
pub struct DecisionEntry {
    /// The `DEC-NNN` id.
    pub id: String,
    /// The decision body, verbatim.
    pub body: String,
}
