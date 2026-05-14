//! Prompt-rendering primitives shared by every phase command.
//!
//! Six helpers, each isolated in its own submodule:
//!
//! - [`template`] -- load embedded markdown templates by name.
//! - [`render`] -- single-pass `{{NAME}}` placeholder substitution.
//! - [`agents_md`] -- locate and read `AGENTS.md` from the project root.
//! - [`mission_md`] -- walk upward from a spec dir for the nearest parent
//!   `MISSION.md`.
//! - [`budget`] -- drop low-priority sections when the rendered prompt exceeds
//!   the character budget.
//! - [`id_alloc`] -- allocate the next `SPEC-NNNN` ID (`max + 1`); walks
//!   `specs/**` recursively so flat and mission-grouped specs share one ID
//!   space.
//!
//! See `.speccy/specs/0005-plan-command/SPEC.md` REQ-003..REQ-007.

pub mod agents_md;
pub mod budget;
pub mod id_alloc;
pub mod mission_md;
pub mod render;
pub mod template;

pub use agents_md::load_agents_md;
pub use budget::DEFAULT_BUDGET;
pub use budget::TrimResult;
pub use budget::trim_to_budget;
pub use id_alloc::allocate_next_spec_id;
pub use mission_md::find_nearest_mission_md;
pub use render::render;
pub use template::PromptError;
pub use template::load_template;
