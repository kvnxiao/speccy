//! Parsers for Speccy's artifact files plus cross-reference utilities.
//!
//! The public API is intentionally narrow: one function per artifact, plus
//! two pure analysis helpers (`cross_ref`, `supersession_index`). Every
//! parser returns [`crate::error::ParseError`] on failure; no panics are
//! reachable from these entry points.
//!
//! SPEC.md is carried by the raw XML element parser ([`spec_xml`]): a line-
//! aware scanner over a Markdown body that recognises a closed whitelist of
//! Speccy element tags (`<requirement>`, `<scenario>`, `<decision>`,
//! `<open-question>`, `<changelog>`, plus the top-level section wrappers
//! `<goals>`, `<non-goals>`, `<user-stories>`, `<assumptions>` and the
//! per-requirement `<done-when>` / `<behavior>` sub-sections).
//! TASKS.md and REPORT.md are parsed by [`task_xml`] / [`report_xml`].

pub mod cross_ref;
pub mod frontmatter;
pub(crate) mod fs;
pub(crate) mod journal_common;
pub mod journal_xml;
pub mod markdown;
pub mod report_xml;
pub mod spec_md;
pub mod spec_xml;
pub mod supersession;
pub mod task_xml;
pub mod vet_xml;
pub mod xml_scanner;

pub use cross_ref::CrossRef;
pub use cross_ref::cross_ref;
pub use journal_xml::JournalDoc;
pub use journal_xml::JournalEntry;
pub use journal_xml::parse as parse_journal_xml;
pub use journal_xml::serialize::BlockInputs;
pub use journal_xml::serialize::NoRoundError;
pub use journal_xml::serialize::SerializeError;
pub use journal_xml::serialize::TaskBlockKind;
pub use journal_xml::serialize::derive_round;
pub use journal_xml::serialize::render_fresh_frontmatter;
pub use journal_xml::serialize::validate_and_render_block;
pub use report_xml::CoverageResult;
pub use report_xml::ReportDoc;
pub use report_xml::RequirementCoverage;
pub use report_xml::parse as parse_report_xml;
pub use spec_md::ChangelogRow;
pub use spec_md::ReqHeading;
pub use spec_md::SpecFrontmatter;
pub use spec_md::SpecMd;
pub use spec_md::SpecStatus;
pub use spec_md::spec_md;
pub use spec_xml::Decision;
pub use spec_xml::DecisionStatus;
pub use spec_xml::ElementSpan;
pub use spec_xml::OpenQuestion;
pub use spec_xml::Requirement;
pub use spec_xml::Scenario;
pub use spec_xml::SpecDoc;
pub use spec_xml::parse as parse_spec_xml;
pub use supersession::SupersessionIndex;
pub use supersession::supersession_index;
pub use task_xml::LEGAL_TRANSITION_EDGES;
pub use task_xml::MisplacedJournalElement;
pub use task_xml::SpliceError;
pub use task_xml::Task;
pub use task_xml::TaskState;
pub use task_xml::TasksDoc;
pub use task_xml::TransitionKind;
pub use task_xml::classify_transition;
pub use task_xml::parse as parse_task_xml;
pub use task_xml::splice_task_state;
pub use vet_xml::Invocation;
pub use vet_xml::VetBlock;
pub use vet_xml::VetDoc;
pub use vet_xml::parse as parse_vet_xml;
pub use vet_xml::parse_in_flight as parse_vet_in_flight;
pub use vet_xml::serialize::AppendPlan;
pub use vet_xml::serialize::AppendPlanError;
pub use vet_xml::serialize::VetBlockInputs;
pub use vet_xml::serialize::VetBlockKind;
pub use vet_xml::serialize::VetSerializeError;
pub use vet_xml::serialize::plan_append as plan_vet_append;
pub use vet_xml::serialize::render_fresh_vet_frontmatter;
pub use vet_xml::serialize::render_section_heading as render_vet_section_heading;
pub use vet_xml::serialize::validate_and_render_vet_block;
