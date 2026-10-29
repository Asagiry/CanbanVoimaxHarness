//! Domain Module (§ Phase 3.1 DDD Foundation)
//!
//! Provides core Domain Value Objects for tasks, pipeline stages, and boards.
//! Designed under Uncle Bob's Clean Architecture & ub-mode principles:
//! - Strongly typed Domain Value Objects (TaskId, StageId, BoardId)
//! - Parse-don't-validate boundary discipline
//! - Non-breaking additive foundation

pub mod ids;
pub mod state;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod state_tests;

pub use ids::{BoardId, DomainError, StageId, TaskId};
pub use state::*;
