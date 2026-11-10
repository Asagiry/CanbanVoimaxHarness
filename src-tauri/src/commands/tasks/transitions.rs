//! Task Pipeline Transitions & Stage Progression Subsystem (§ 2.2, § 2.6, § 2.7, § 2.27)
//! Facade re-exporting modular transition submodules.

#[path = "transition_hooks.rs"]
pub mod transition_hooks;
pub use transition_hooks::*;

pub mod advance;
pub mod advance_child;
pub mod move_stage;
pub mod reorder;
pub mod resolve;
pub mod subtasks;

#[cfg(test)]
mod tests;

pub use advance::*;
pub use advance_child::*;
pub use move_stage::*;
pub use reorder::*;
pub use resolve::*;
pub use subtasks::*;
