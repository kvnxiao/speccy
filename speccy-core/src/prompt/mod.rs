//! Prompt-rendering primitives shared by every phase command.
//!
//! Five helpers, each isolated in its own submodule:
//!
//! - [`template`] -- load embedded markdown templates by name.
//! - [`render`] -- single-pass `{{NAME}}` placeholder substitution.
//! - [`budget`] -- drop low-priority sections when the rendered prompt exceeds
//!   the character budget.
//! - [`id_alloc`] -- allocate the next `SPEC-NNNN` ID (`max + 1`); walks
//!   `specs/**` recursively so flat and mission-grouped specs share one ID
//!   space.
//! - [`spec_slice`] -- emit a task-scoped Markdown slice of a `SpecDoc` driven
//!   by the task's `Covers:` list (frontmatter + heading + overview + covered
//!   requirements with nested scenarios + decisions).
//!
//! See `.speccy/specs/0005-plan-command/SPEC.md` REQ-003..REQ-007.
//! See `.speccy/specs/0023-single-phase-skill-primitives/SPEC.md` REQ-005
//! for the retirement of the `AGENTS.md` loader and REQ-006 for the
//! retirement of the SPEC.md / TASKS.md / MISSION.md loaders: rendered
//! prompts now name the file's repo-relative path and the agent reads it
//! via the host's Read primitive on demand.

pub mod budget;
pub mod id_alloc;
pub mod render;
pub mod spec_slice;
pub mod template;

pub use budget::DEFAULT_BUDGET;
pub use budget::TrimResult;
pub use budget::trim_to_budget;
pub use id_alloc::allocate_next_spec_id;
pub use render::render;
pub use spec_slice::slice_for_task;
pub use template::PROMPTS;
pub use template::PromptError;
pub use template::load_template;
