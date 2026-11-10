//! Context Compactor Subsystem (§ 2.22)
//! Facade re-exporting modular compactor submodules.

pub mod evaporation;
pub mod extractor;
pub mod multi_tier;
pub mod orchestrator;
pub mod tokens;
pub mod report;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod token_tests;

pub use evaporation::*;
pub use extractor::*;
pub use multi_tier::*;
pub use orchestrator::*;
pub use tokens::*;
