//! Prompt-rendering primitives shared by every phase command.
//!
//! Five helpers, each isolated in its own submodule:
//!
//! - [`template`] -- load embedded markdown templates by name.
//! - [`render`] -- single-pass `{{NAME}}` placeholder substitution.
//! - [`agents_md`] -- locate and read `AGENTS.md` from the project root.
//! - [`budget`] -- drop low-priority sections when the rendered prompt exceeds
//!   the character budget.
//! - [`id_alloc`] -- allocate the next `SPEC-NNNN` ID (`max + 1`).
//!
//! See `.speccy/specs/0005-plan-command/SPEC.md` REQ-003..REQ-006.

pub mod agents_md;
pub mod budget;
pub mod id_alloc;
pub mod render;
pub mod template;

pub use agents_md::load_agents_md;
pub use budget::DEFAULT_BUDGET;
pub use budget::TrimResult;
pub use budget::trim_to_budget;
pub use id_alloc::allocate_next_spec_id;
pub use render::render;
pub use template::PromptError;
pub use template::load_template;
