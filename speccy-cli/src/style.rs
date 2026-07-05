//! Semantic terminal styling for the human-facing commands.
//!
//! Colors carry meaning, not decoration: green for healthy/passed, yellow for
//! attention, red for failure, cyan for spec references, bold for headers, dim
//! for secondary text. Callers wrap already-formatted tokens with [`paint`];
//! the output sinks in `main.rs` print through `anstream`, which strips these
//! codes when stdout is not a terminal and honors `NO_COLOR`/`CLICOLOR_FORCE`.

use anstyle::AnsiColor;
use anstyle::Color;
use anstyle::Style;
use speccy_core::model::RequirementStatus;
use speccy_core::model::RiskTier;
use speccy_core::model::SpecStatus;

/// Bold, for section headers and card titles.
pub const HEADER: Style = Style::new().bold();
/// Cyan, for spec references (`SPEC-…`).
pub const SPEC_REF: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Cyan)));
/// Green, for healthy/passed/landed states.
pub const OK: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green)));
/// Yellow, for warnings, drift, and accepted-risk states.
pub const WARN: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Yellow)));
/// Red, for failures and the `speccy:` error prefix.
pub const ERR: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Red)));
/// Dimmed, for secondary text such as ages and hints.
pub const DIM: Style = Style::new().dimmed();

/// Wrap `s` in `style`'s ANSI start and reset sequences.
#[must_use = "the styled string is the whole point of calling this"]
pub fn paint(style: Style, s: &str) -> String {
    format!("{}{s}{}", style.render(), style.render_reset())
}

/// The style for a risk tier: green (routine) through red (critical).
#[must_use = "the returned style is used to paint the tier"]
pub fn risk_style(tier: RiskTier) -> Style {
    match tier {
        RiskTier::Minimal | RiskTier::Standard => OK,
        RiskTier::High => WARN,
        RiskTier::Critical => ERR,
    }
}

/// The style for a spec status: green when active/accepted, dim when inert.
#[must_use = "the returned style is used to paint the status"]
pub fn spec_status_style(status: SpecStatus) -> Style {
    match status {
        SpecStatus::Approved | SpecStatus::Accepted => OK,
        SpecStatus::Cancelled => WARN,
        SpecStatus::Draft | SpecStatus::Superseded | SpecStatus::Archived => DIM,
    }
}

/// The style for a requirement status: green passed, red failed, dim pending.
#[must_use = "the returned style is used to paint the status"]
pub fn req_status_style(status: RequirementStatus) -> Style {
    match status {
        RequirementStatus::Passed => OK,
        RequirementStatus::Failed | RequirementStatus::Blocked => ERR,
        RequirementStatus::ReviewPassed | RequirementStatus::Waived => WARN,
        RequirementStatus::Pending => DIM,
    }
}
