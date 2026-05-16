//! Parsers for Speccy's five artifact files plus cross-reference utilities.
//!
//! The public API is intentionally narrow: one function per artifact, plus
//! two pure analysis helpers (`cross_ref`, `supersession_index`). Every
//! parser returns [`crate::error::ParseError`] on failure; no panics are
//! reachable from these entry points.

pub mod cross_ref;
pub mod frontmatter;
pub mod markdown;
pub mod report_md;
pub mod spec_markers;
pub mod spec_md;
pub mod supersession;
pub mod tasks_md;
pub mod toml_files;

pub use cross_ref::CrossRef;
pub use cross_ref::cross_ref;
pub use report_md::ReportFrontmatter;
pub use report_md::ReportMd;
pub use report_md::ReportOutcome;
pub use report_md::report_md;
pub use spec_markers::Decision;
pub use spec_markers::DecisionStatus;
pub use spec_markers::MarkerSpan;
pub use spec_markers::OpenQuestion;
pub use spec_markers::Requirement;
pub use spec_markers::Scenario;
pub use spec_markers::SpecDoc;
pub use spec_markers::parse as parse_spec_markers;
pub use spec_markers::render as render_spec_markers;
pub use spec_md::ChangelogRow;
pub use spec_md::ReqHeading;
pub use spec_md::SpecFrontmatter;
pub use spec_md::SpecMd;
pub use spec_md::SpecStatus;
pub use spec_md::spec_md;
pub use supersession::SupersessionIndex;
pub use supersession::supersession_index;
pub use tasks_md::Task;
pub use tasks_md::TaskState;
pub use tasks_md::TaskWarning;
pub use tasks_md::TasksFrontmatter;
pub use tasks_md::TasksMd;
pub use tasks_md::tasks_md;
pub use toml_files::ProjectConfig;
pub use toml_files::SpeccyConfig;
pub use toml_files::speccy_toml;
