//! Parsers for Speccy's five artifact files plus cross-reference utilities.
//!
//! The public API is intentionally narrow: one function per artifact, plus
//! two pure analysis helpers (`cross_ref`, `supersession_index`). Every
//! parser returns [`crate::error::ParseError`] on failure; no panics are
//! reachable from these entry points.
//!
//! SPEC.md is carried by the raw XML element parser ([`spec_xml`]): a line-
//! aware scanner over a Markdown body that recognises a closed whitelist of
//! Speccy element tags (`<requirement>`, `<scenario>`, `<decision>`,
//! `<open-question>`, `<changelog>`, plus optional `<spec>` root and
//! `<overview>` section). The SPEC-0019 HTML-comment marker form has been
//! removed; surviving marker comments are surfaced as
//! [`crate::error::ParseError::LegacyMarker`] with the equivalent raw XML
//! element form in the diagnostic.
//!
//! The SPEC-0019 marker parser is gone (REQ-002 "deleted, not feature-
//! flagged"). The following doctest pins that contract: it must fail to
//! compile because no `parse_spec_markers`, `render_spec_markers`, or
//! `MarkerSpan` symbol is reachable from this module after SPEC-0020.
//!
//! ```compile_fail
//! use speccy_core::parse::parse_spec_markers;
//! use speccy_core::parse::render_spec_markers;
//! use speccy_core::parse::MarkerSpan;
//! ```

pub mod cross_ref;
pub mod frontmatter;
pub mod markdown;
pub mod report_md;
pub mod spec_md;
pub mod spec_xml;
pub mod supersession;
pub mod tasks_md;
pub mod toml_files;

pub use cross_ref::CrossRef;
pub use cross_ref::cross_ref;
pub use report_md::ReportFrontmatter;
pub use report_md::ReportMd;
pub use report_md::ReportOutcome;
pub use report_md::report_md;
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
pub use spec_xml::render as render_spec_xml;
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
