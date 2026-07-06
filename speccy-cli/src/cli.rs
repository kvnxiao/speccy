//! The `speccy` command-line surface.
//!
//! Two families live here: the machine-facing `speccy ctl <noun> <verb>`
//! controller operations (DESIGN § Controller API Surface) that install-pack
//! skills call, and the human-facing commands (DESIGN § CLI/Admin Flow). The
//! ctl surface always emits the JSON envelope; `--json` is accepted globally
//! and is a no-op for ctl (which is JSON-only) but selects JSON output for the
//! human commands.

use clap::Args;
use clap::Parser;
use clap::Subcommand;

#[derive(Debug, Parser)]
#[command(
    name = "speccy",
    version,
    about = "A spec-driven run controller for coding agents",
    long_about = "Speccy is a deterministic run controller that coding-agent \
                  harnesses call. It never launches an LLM."
)]
pub struct Cli {
    /// Emit machine-readable JSON. Always on for `ctl`; selects JSON for human
    /// commands that also render text.
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Machine-facing controller operations used by install-pack skills.
    #[command(subcommand)]
    Ctl(CtlCommand),

    /// Check the local controller, git, and store health.
    Doctor,

    // --- Human-facing commands (DESIGN § CLI/Admin Flow) ---
    /// Show one card per active run in this workspace.
    Status,
    /// List specs in this workspace; `--query` previews selector matches.
    List(ListArgs),
    /// Show the state-aware human packet for a spec/run.
    Review(ReviewArgs),
    /// Record that a submitted run's change landed.
    Accept(AcceptArgs),
    /// Hide a stale accepted spec from active views.
    Archive(SelectorArgs),
    /// Cancel the current or selected spec/run.
    Cancel(SelectorArgs),
    /// Record plain engineering intent as a draft spec, outside a harness.
    New(NewArgs),
    /// Install or update repo-local harness packs.
    Install(InstallArgs),
    /// Export a spec, review packet, or redacted run bundle.
    #[command(subcommand)]
    Export(ExportCommand),
}

/// The `speccy ctl` noun tree.
#[derive(Debug, Subcommand)]
pub enum CtlCommand {
    /// Spec lifecycle operations (spec-scoped, not lease-gated).
    #[command(subcommand)]
    Spec(SpecOp),
    /// Run lifecycle and the loop-driving `next` directive.
    #[command(subcommand)]
    Run(RunOp),
    /// Task claim and handoff operations.
    #[command(subcommand)]
    Task(TaskOp),
    /// Deterministic work-order packet assembly.
    #[command(subcommand)]
    Packet(PacketOp),
    /// Evidence collection and recording.
    #[command(subcommand)]
    Evidence(EvidenceOp),
    /// Reviewer finding recording.
    #[command(subcommand)]
    Finding(FindingOp),
    /// Requirement status transitions.
    #[command(subcommand)]
    Requirement(RequirementOp),
}

#[derive(Debug, Subcommand)]
pub enum SpecOp {
    /// Create a spec from a small intent record (`request.json`).
    Start(InputArgs),
    /// Read current spec status.
    Status(SpecRef),
    /// Submit one complete candidate revision; returns lint findings.
    RecordDraft(SpecInput),
    /// Replace named sections of the current draft; returns lint findings.
    PatchDraft(SpecInput),
    /// Record a spec-scoped decision
    /// (approve/reject/split/`scope_change`/cancel).
    RecordDecision(SpecInput),
}

#[derive(Debug, Subcommand)]
pub enum RunOp {
    /// Open a run against an approved revision (gates: git, clean worktree).
    Start(RunStartArgs),
    /// Read current run status.
    Status(RunRef),
    /// Return the single next directive; the loop's only entry point.
    Next(RunNextArgs),
    /// Record a run-scoped gate decision (waive/rework/…).
    RecordDecision(RunInput),
    /// Record a ship: `verified -> submitted` plus the change reference.
    RecordShip(RunInput),
    /// Signal a harness interrupt (structured-output retry exhaustion); parks
    /// the run at the escalation gate.
    Interrupt(RunInput),
}

#[derive(Debug, Subcommand)]
pub enum TaskOp {
    /// Claim the next task; pins `baseline_commit`.
    Claim(TaskClaimArgs),
    /// Record a worker handoff; moves the task to `in_review`.
    RecordHandoff(RunInput),
}

#[derive(Debug, Subcommand)]
pub enum PacketOp {
    /// Deterministic planning work order.
    Planning(SpecRef),
    /// Task packet scoped to linked requirements.
    Task(TaskRef),
    /// Verification packet naming the persona roster.
    Verification(VerificationArgs),
    /// Human-facing review packet.
    Review(RunRef),
    /// Requirement-scoped escalation packet.
    Escalation(RunRef),
}

#[derive(Debug, Subcommand)]
pub enum EvidenceOp {
    /// Execute declared `kind: command` evidence and record results.
    Collect(EvidenceCollectArgs),
    /// Record non-command evidence (command output is refused here).
    Record(RunInput),
}

#[derive(Debug, Subcommand)]
pub enum FindingOp {
    /// Record a structured reviewer finding (lease-free).
    Record(RunInput),
}

#[derive(Debug, Subcommand)]
pub enum RequirementOp {
    /// Transition requirement statuses from evidence and findings.
    SetStatus(RunInput),
}

// --- Shared argument groups ---

/// `--input <path|->` for operations that read a payload.
#[derive(Debug, Args)]
pub struct InputArgs {
    /// Payload file path, or `-` to read from stdin.
    #[arg(long)]
    pub input: String,
}

/// `--spec <ref>` only.
#[derive(Debug, Args)]
pub struct SpecRef {
    #[arg(long)]
    pub spec: String,
}

/// `--spec <ref> --input <path|->`.
#[derive(Debug, Args)]
pub struct SpecInput {
    #[arg(long)]
    pub spec: String,
    #[arg(long)]
    pub input: String,
}

/// `--run <id>` only.
#[derive(Debug, Args)]
pub struct RunRef {
    #[arg(long)]
    pub run: String,
}

/// `--run <id> --lease <token> --input <path|->` for lease-gated writes.
#[derive(Debug, Args)]
pub struct RunInput {
    #[arg(long)]
    pub run: String,
    /// Live lease token (required for state-mutating ops; unused by lease-free
    /// ops).
    #[arg(long)]
    pub lease: Option<String>,
    #[arg(long)]
    pub input: String,
}

#[derive(Debug, Args)]
pub struct RunStartArgs {
    #[arg(long)]
    pub spec: String,
    #[arg(long)]
    pub revision: String,
}

#[derive(Debug, Args)]
pub struct RunNextArgs {
    #[arg(long)]
    pub run: String,
    /// Opaque caller-chosen agent ID; binds/renews the run lease.
    #[arg(long)]
    pub agent: String,
}

#[derive(Debug, Args)]
pub struct TaskRef {
    #[arg(long)]
    pub run: String,
    #[arg(long)]
    pub task: String,
}

#[derive(Debug, Args)]
pub struct TaskClaimArgs {
    #[arg(long)]
    pub run: String,
    #[arg(long)]
    pub task: String,
    #[arg(long)]
    pub agent: String,
    #[arg(long)]
    pub lease: String,
}

#[derive(Debug, Args)]
pub struct VerificationArgs {
    #[arg(long)]
    pub run: String,
    /// Comma-separated requirement IDs to scope the packet.
    #[arg(long, value_delimiter = ',')]
    pub requirements: Vec<String>,
}

#[derive(Debug, Args)]
pub struct EvidenceCollectArgs {
    #[arg(long)]
    pub run: String,
    /// Requirements whose `kind: command` evidence to collect.
    #[arg(long, value_delimiter = ',')]
    pub requirements: Vec<String>,
    /// Optional qualified request IDs (`R.E`) to narrow collection.
    #[arg(long, value_delimiter = ',')]
    pub requests: Vec<String>,
}

// --- Human command arguments ---

/// Optional free-text selector, resolved to a spec/run.
#[derive(Debug, Args)]
pub struct SelectorArgs {
    /// Spec reference or free-text selector; inferred when omitted.
    pub selector: Option<String>,
}

#[derive(Debug, Args)]
pub struct ListArgs {
    /// Preview which specs match a selector without taking action.
    #[arg(long)]
    pub query: Option<String>,
    /// Show all specs including hidden statuses.
    #[arg(long)]
    pub all: bool,
    /// Show accepted specs.
    #[arg(long)]
    pub accepted: bool,
    /// Show archived specs.
    #[arg(long)]
    pub archived: bool,
    /// Filter to a specific spec status.
    #[arg(long)]
    pub status: Option<String>,
}

#[derive(Debug, Args)]
pub struct ReviewArgs {
    pub selector: Option<String>,
    /// Drill into ledger, command logs, artifacts, findings, and diff.
    #[arg(long)]
    pub evidence: bool,
}

#[derive(Debug, Args)]
pub struct AcceptArgs {
    pub selector: Option<String>,
    /// Associate a PR URL for recovery/manual association.
    #[arg(long)]
    pub pr: Option<String>,
    /// Free-text note for recovery/manual association.
    #[arg(long)]
    pub note: Option<String>,
}

#[derive(Debug, Args)]
pub struct NewArgs {
    /// Plain engineering intent, verbatim.
    pub request: String,
    /// Mutable working title.
    #[arg(long)]
    pub title: Option<String>,
}

#[derive(Debug, Args)]
#[expect(clippy::struct_excessive_bools, reason = "independent clap CLI flags")]
pub struct InstallArgs {
    /// Harness target(s): auto | codex | claude | all.
    #[arg(long, default_value = "auto")]
    pub target: String,
    /// Apply reviewable pack updates via three-way merge.
    #[arg(long)]
    pub update: bool,
    /// Print the would-write listing and stop.
    #[arg(long)]
    pub dry_run: bool,
    /// Skip the interactive confirmation before writing.
    #[arg(long)]
    pub yes: bool,
    /// Exit nonzero when packs are missing, outdated, or conflicted.
    #[arg(long)]
    pub check: bool,
    /// Overwrite managed pack files with the current template.
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Subcommand)]
pub enum ExportCommand {
    /// Export the human review packet.
    Review(ExportArgs),
    /// Export the full spec.
    Spec(ExportArgs),
    /// Export a redacted run bundle for audit/debugging.
    RunBundle(ExportBundleArgs),
}

#[derive(Debug, Args)]
pub struct ExportArgs {
    pub selector: Option<String>,
    /// Destination directory.
    #[arg(long)]
    pub dest: Option<String>,
}

#[derive(Debug, Args)]
pub struct ExportBundleArgs {
    pub selector: Option<String>,
    #[arg(long)]
    pub dest: Option<String>,
    /// Redact known secrets from the bundle.
    #[arg(long)]
    pub redact: bool,
}
