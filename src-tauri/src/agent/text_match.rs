// Text matching, similarity, CRLF normalization, and whitespace-relaxed replacement routines.

pub mod diff_mismatch;
pub mod replace;

pub use diff_mismatch::*;
pub use replace::*;
