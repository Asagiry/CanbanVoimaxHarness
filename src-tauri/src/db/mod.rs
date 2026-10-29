//! Database layer for CanbanVoimaxHarness (.cvh/harness.db).
//!
//! Provides connection pooling (WAL mode), task repository,
//! dialog activity repository, and telemetry/ledger repository.

pub mod dialog_repo;
pub mod pool;
pub mod task_repo;
pub mod telemetry_repo;

#[cfg(test)]
mod tests;

// Facade re-exports for complete 100% backward compatibility
pub use dialog_repo::*;
pub use pool::*;
pub use task_repo::*;
pub use telemetry_repo::*;
