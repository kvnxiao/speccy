//! Structured error type for every parser in `speccy-core`.
//!
//! All public parsers return [`Result<T, ParseError>`]. The variants
//! intentionally name the file path or label whenever it is available so
//! downstream consumers (`speccy status`, `speccy verify`, the lint engine)
//! can render actionable diagnostics without re-parsing the source file.

use camino::Utf8PathBuf;
use std::io;
use thiserror::Error;

/// Every failure mode reachable through [`crate::parse`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ParseError {
    /// The file could not be read off disk.
    #[error("failed to read {path}")]
    Io {
        /// Path that failed to read.
        path: Utf8PathBuf,
        /// Underlying I/O error.
        #[source]
        source: io::Error,
    },

    /// Path is not valid UTF-8. Speccy targets UTF-8 paths only outside OS
    /// boundaries; non-UTF-8 inputs are refused up front.
    #[error("path is not valid UTF-8: {0}")]
    NonUtf8Path(String),

    /// File content is not valid UTF-8. Speccy does not attempt encoding
    /// detection.
    #[error("{path} is not valid UTF-8")]
    NonUtf8File {
        /// Path that contained the invalid bytes.
        path: Utf8PathBuf,
    },

    /// The TOML payload at `path` could not be deserialised.
    #[error("failed to parse TOML in {path}: {message}")]
    Toml {
        /// Path that failed to parse.
        path: Utf8PathBuf,
        /// Underlying TOML error message.
        message: String,
    },

    /// The YAML frontmatter could not be deserialised.
    #[error("failed to parse YAML frontmatter{location}: {message}", location = location_suffix(.label.as_deref()))]
    Yaml {
        /// Optional file or section label.
        label: Option<String>,
        /// Underlying YAML error message.
        message: String,
    },

    /// `schema_version` declared in a TOML config is not a supported value.
    #[error(
        "{path} declares unsupported schema_version = {value}; speccy supports schema_version = 1"
    )]
    UnsupportedSchemaVersion {
        /// Path of the offending file.
        path: Utf8PathBuf,
        /// Value found in the file.
        value: i64,
    },

    /// A required field was missing from a parsed payload.
    #[error("missing required field `{field}` in {context}")]
    MissingField {
        /// Field that was expected.
        field: String,
        /// Description of the parent table or struct.
        context: String,
    },

    /// A per-spec `spec.toml` file was present after the SPEC-0019
    /// migration. SPEC-0019 REQ-002 deleted per-spec TOML; the marker
    /// tree in `SPEC.md` is the only carrier. The workspace loader
    /// surfaces a stray file as a per-spec parse failure so callers
    /// (lint, status, verify) see it through the existing per-spec
    /// error channel.
    #[error(
        "stray per-spec spec.toml present at {path}: SPEC-0019 removed spec.toml; the marker tree in SPEC.md is the only spec carrier"
    )]
    StraySpecToml {
        /// Absolute path to the stray `spec.toml` file.
        path: Utf8PathBuf,
    },

    /// A markdown file declared an opening `---` fence but no closing one.
    #[error("unterminated YAML frontmatter in {path}")]
    UnterminatedFrontmatter {
        /// Path of the offending file.
        path: Utf8PathBuf,
    },

    /// A markdown frontmatter field carried a value outside the allowed
    /// closed set (e.g. SPEC.md `status` or REPORT.md `outcome`).
    #[error("invalid value for `{field}` in {path}: `{value}` is not one of {allowed}")]
    InvalidEnumValue {
        /// Path of the offending file.
        path: Utf8PathBuf,
        /// Field that carried the invalid value.
        field: String,
        /// Offending value.
        value: String,
        /// Comma-separated list of allowed values.
        allowed: String,
    },

    /// A marker comment used a name outside the closed Speccy set.
    #[error("unknown speccy marker `{marker_name}` in {path} at byte offset {offset}")]
    UnknownMarkerName {
        /// Path of the offending file.
        path: Utf8PathBuf,
        /// Marker name found in the source (without the `speccy:` prefix).
        marker_name: String,
        /// Byte offset of the marker's start in the source.
        offset: usize,
    },

    /// A marker comment carried an attribute outside the set allowed for
    /// that marker name.
    #[error(
        "unknown attribute `{attribute}` on speccy marker `{marker_name}` in {path} at byte offset {offset}"
    )]
    UnknownMarkerAttribute {
        /// Path of the offending file.
        path: Utf8PathBuf,
        /// Marker name carrying the attribute.
        marker_name: String,
        /// Attribute name found in the source.
        attribute: String,
        /// Byte offset of the marker's start in the source.
        offset: usize,
    },

    /// A marker comment was syntactically malformed (non-line-isolated,
    /// unquoted attribute, missing end marker, bad nesting).
    #[error("malformed speccy marker in {path} at byte offset {offset}: {reason}")]
    MalformedMarker {
        /// Path of the offending file.
        path: Utf8PathBuf,
        /// Byte offset of the offending marker's start in the source.
        offset: usize,
        /// Human-readable reason.
        reason: String,
    },

    /// A marker id failed the id-pattern regex for its marker name.
    #[error(
        "speccy marker `{marker_name}` in {path} has invalid id `{id}` (expected pattern {expected_pattern})"
    )]
    InvalidMarkerId {
        /// Path of the offending file.
        path: Utf8PathBuf,
        /// Marker name carrying the id.
        marker_name: String,
        /// Id found in the source.
        id: String,
        /// Regex pattern that the id should have matched.
        expected_pattern: String,
    },

    /// Two markers of the same kind reused the same id within one spec.
    #[error("duplicate speccy marker id `{id}` for `{marker_name}` in {path}")]
    DuplicateMarkerId {
        /// Path of the offending file.
        path: Utf8PathBuf,
        /// Marker name carrying the duplicate id.
        marker_name: String,
        /// Id that appeared more than once.
        id: String,
    },

    /// A `speccy:scenario` marker appeared outside any
    /// `speccy:requirement` marker.
    #[error("scenario marker outside any requirement in {path} at byte offset {offset}")]
    ScenarioOutsideRequirement {
        /// Path of the offending file.
        path: Utf8PathBuf,
        /// Scenario id when present in the malformed marker.
        scenario_id: Option<String>,
        /// Byte offset of the offending marker's start in the source.
        offset: usize,
    },

    /// A marker block that is required to carry non-whitespace Markdown
    /// body contained only whitespace.
    #[error("speccy marker `{marker_name}` in {path} at byte offset {offset} has an empty body")]
    EmptyMarkerBody {
        /// Path of the offending file.
        path: Utf8PathBuf,
        /// Marker name with the empty body.
        marker_name: String,
        /// Id of the offending marker when one is set.
        id: Option<String>,
        /// Byte offset of the marker's start in the source.
        offset: usize,
    },

    /// A marker attribute's value was outside the allowed closed set for
    /// that attribute (e.g. `decision` `status`, `open-question` `resolved`).
    #[error(
        "invalid value for attribute `{attribute}` on speccy marker `{marker_name}` in {path}: `{value}` is not one of {allowed}"
    )]
    InvalidMarkerAttributeValue {
        /// Path of the offending file.
        path: Utf8PathBuf,
        /// Marker name carrying the attribute.
        marker_name: String,
        /// Attribute name.
        attribute: String,
        /// Offending value.
        value: String,
        /// Comma-separated list of allowed values.
        allowed: String,
    },

    /// A SPEC.md still carries a SPEC-0019 HTML-comment Speccy marker
    /// (`<!-- speccy:NAME ... -->` or `<!-- /speccy:NAME -->`) outside any
    /// fenced code block. After the SPEC-0020 migration the raw XML element
    /// form is the only accepted carrier; surfacing this as a dedicated
    /// variant lets the diagnostic suggest the equivalent element syntax.
    #[error(
        "legacy HTML-comment speccy marker in {path} at byte offset {offset}: {legacy_form} (rewrite as the raw XML element {suggested_element})"
    )]
    LegacyMarker {
        /// Path of the offending file.
        path: Utf8PathBuf,
        /// Byte offset of the offending marker's start in the source.
        offset: usize,
        /// Legacy marker line as it appears in the source.
        legacy_form: String,
        /// Suggested raw XML element form that replaces the legacy marker.
        suggested_element: String,
    },

    /// A SPEC.md used an element name retired by SPEC-0021 (`<spec>`,
    /// `<overview>`). SPEC-0021 DEC-008 retires these from the whitelist
    /// in the same pass that adds the new section tags; surfacing the
    /// retirement in the diagnostic lets the author find SPEC-0021 in
    /// the project log when they hit the message.
    #[error(
        "retired speccy element `<{marker_name}>` in {path} at byte offset {offset}: this element was retired in SPEC-0021"
    )]
    RetiredMarkerName {
        /// Path of the offending file.
        path: Utf8PathBuf,
        /// Marker name found in the source.
        marker_name: String,
        /// Byte offset of the offending tag's start in the source.
        offset: usize,
    },

    /// A SPEC.md is missing a top-level section element that SPEC-0021
    /// requires exactly once (`<goals>`, `<non-goals>`, `<user-stories>`).
    #[error("missing required top-level section `<{element_name}>` in {path}")]
    MissingRequiredSection {
        /// Path of the offending file.
        path: Utf8PathBuf,
        /// Element name that should have been present.
        element_name: String,
    },

    /// A SPEC.md has two or more occurrences of a top-level section
    /// element that SPEC-0021 caps at exactly one
    /// (`<goals>`, `<non-goals>`, `<user-stories>`, `<assumptions>`).
    #[error(
        "duplicate top-level section `<{element_name}>` in {path} at byte offset {offset}: the section may appear at most once"
    )]
    DuplicateSection {
        /// Path of the offending file.
        path: Utf8PathBuf,
        /// Element name that was repeated.
        element_name: String,
        /// Byte offset of the offending duplicate's start in the source.
        offset: usize,
    },

    /// A `<requirement>` is missing one of its required sub-sections
    /// (`<done-when>`, `<behavior>`).
    #[error(
        "requirement `{requirement_id}` in {path} is missing required `<{element_name}>` element"
    )]
    MissingRequirementSection {
        /// Path of the offending file.
        path: Utf8PathBuf,
        /// Id of the offending requirement.
        requirement_id: String,
        /// Element name that should have been present inside the requirement.
        element_name: String,
    },

    /// A `<requirement>` carries two or more occurrences of a sub-section
    /// element that SPEC-0021 caps at exactly one
    /// (`<done-when>`, `<behavior>`).
    #[error(
        "requirement `{requirement_id}` in {path} has duplicate `<{element_name}>` element at byte offset {offset}"
    )]
    DuplicateRequirementSection {
        /// Path of the offending file.
        path: Utf8PathBuf,
        /// Id of the offending requirement.
        requirement_id: String,
        /// Element name that was repeated.
        element_name: String,
        /// Byte offset of the offending duplicate's start in the source.
        offset: usize,
    },

    /// A `<requirement>` violates the SPEC-0021 DEC-002 ordering rule
    /// (`<done-when>` before `<behavior>` before any `<scenario>`).
    #[error(
        "requirement `{requirement_id}` in {path} violates section order at byte offset {offset}: {reason}"
    )]
    RequirementSectionOrder {
        /// Path of the offending file.
        path: Utf8PathBuf,
        /// Id of the offending requirement.
        requirement_id: String,
        /// Byte offset of the offending tag's start in the source.
        offset: usize,
        /// Human-readable explanation of the ordering violation.
        reason: String,
    },
}

fn location_suffix(label: Option<&str>) -> String {
    match label {
        Some(label) => format!(" in {label}"),
        None => String::new(),
    }
}
