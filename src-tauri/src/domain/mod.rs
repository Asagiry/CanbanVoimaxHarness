//! Domain Module (§ Phase 3.1 DDD Foundation)
//!
//! Provides core Domain Value Objects for tasks, pipeline stages, and boards.
//! Authoritative domain terms and ubiquitous language are codified in `GLOSSARY.md` (repository root).
//!
//! Designed under Eric Evans' Domain-Driven Design (DDD) & Uncle Bob's Clean Architecture:
//! - Canonical domain definitions: see `GLOSSARY.md`
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
