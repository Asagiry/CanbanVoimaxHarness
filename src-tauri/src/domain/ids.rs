//! Domain Value Objects: TaskId, StageId, BoardId
//!
//! Enforces DDD boundary discipline and type system invariants per Uncle Bob (ub-mode):
//! - Strict validation at boundary
//! - Normalization of case and whitespace (trim)
//! - Immutable, self-encapsulated Value Objects
//! - Value equality, ordering, hashing, serialization, and deref coercion.

use std::borrow::Borrow;
use std::fmt;
use std::ops::Deref;
use std::str::FromStr;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

/// Domain error variants for Value Object creation and validation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DomainError {
    #[error("Task ID cannot be empty or blank")]
    EmptyTaskId,

    #[error("Task ID '{0}' contains invalid characters or exceeds maximum length (128)")]
    InvalidTaskIdFormat(String),

    #[error("Stage ID cannot be empty or blank")]
    EmptyStageId,

    #[error("Stage ID '{0}' contains invalid characters or exceeds maximum length (64)")]
    InvalidStageIdFormat(String),

    #[error("Board ID cannot be empty or blank")]
    EmptyBoardId,

    #[error("Board ID '{0}' contains invalid characters or exceeds maximum length (64)")]
    InvalidBoardIdFormat(String),

    #[error(transparent)]
    State(#[from] crate::domain::StateError),
}

impl From<DomainError> for String {
    fn from(err: DomainError) -> Self {
        err.to_string()
    }
}

// ============================================================================
// TaskId Value Object
// ============================================================================

/// Unique identifier for a task or subtask (e.g. `CVH-001`, `CVH-042.1`).
///
/// Invariants:
/// - Leading and trailing whitespace is trimmed.
/// - ASCII letters are normalized to uppercase.
/// - Cannot be empty or blank.
/// - Maximum length of 128 characters.
/// - Valid characters are ASCII alphanumeric (`A-Z`, `0-9`), `-`, `_`, `.`, `:`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId(String);

impl TaskId {
    /// Parses and normalizes a task identifier string.
    pub fn parse(s: &str) -> Result<Self, DomainError> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(DomainError::EmptyTaskId);
        }
        if trimmed.len() > 128 {
            return Err(DomainError::InvalidTaskIdFormat(trimmed.to_string()));
        }

        for c in trimmed.chars() {
            if !(c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == ':') {
                return Err(DomainError::InvalidTaskIdFormat(trimmed.to_string()));
            }
        }

        let normalized = trimmed.to_ascii_uppercase();
        Ok(Self(normalized))
    }

    /// Creates a new `TaskId` from any string-like reference.
    pub fn new(id: impl AsRef<str>) -> Result<Self, DomainError> {
        Self::parse(id.as_ref())
    }

    /// Returns the borrowed string slice representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the `TaskId` and returns the inner `String`.
    pub fn into_inner(self) -> String {
        self.0
    }

    /// Returns `true` if this task ID represents a subtask (e.g. `CVH-001.1`).
    pub fn is_subtask(&self) -> bool {
        self.subtask_index().is_some()
    }

    /// Returns the root task identifier prefix (everything before the first `.`).
    ///
    /// For `CVH-001.2`, this returns `"CVH-001"`.
    /// For `CVH-001`, this returns `"CVH-001"`.
    pub fn root_id(&self) -> &str {
        self.0.split('.').next().unwrap_or(self.as_str())
    }

    /// Returns the numeric subtask index if this is a subtask (e.g. `1` for `CVH-001.1`).
    pub fn subtask_index(&self) -> Option<u32> {
        let mut parts = self.0.split('.');
        let _root = parts.next()?;
        let suffix = parts.next()?;
        suffix.parse::<u32>().ok()
    }
}

// ============================================================================
// StageId Value Object
// ============================================================================

/// Identifier for a pipeline stage or column (e.g. `backlog`, `dev`, `review`, `ready`).
///
/// Invariants:
/// - Leading and trailing whitespace is trimmed.
/// - Normalized to lowercase.
/// - Cannot be empty or blank.
/// - Maximum length of 64 characters.
/// - Valid characters are alphanumeric (including Unicode letters), `-`, `_`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StageId(String);

impl StageId {
    /// Parses and normalizes a stage identifier string.
    pub fn parse(s: &str) -> Result<Self, DomainError> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(DomainError::EmptyStageId);
        }
        if trimmed.len() > 64 {
            return Err(DomainError::InvalidStageIdFormat(trimmed.to_string()));
        }

        for c in trimmed.chars() {
            if !(c.is_alphanumeric() || c == '-' || c == '_') {
                return Err(DomainError::InvalidStageIdFormat(trimmed.to_string()));
            }
        }

        let normalized = trimmed.to_lowercase();
        Ok(Self(normalized))
    }

    /// Creates a new `StageId` from any string-like reference.
    pub fn new(id: impl AsRef<str>) -> Result<Self, DomainError> {
        Self::parse(id.as_ref())
    }

    /// Returns the borrowed string slice representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the `StageId` and returns the inner `String`.
    pub fn into_inner(self) -> String {
        self.0
    }

    /// Returns `true` if this stage represents an intake column (`backlog`, `waiting`, `ожидание`).
    pub fn is_intake(&self) -> bool {
        let s = self.as_str();
        s == "backlog" || s == "waiting" || s == "ожидание"
    }

    /// Returns `true` if this stage represents a terminal column (`ready`, `done`, `готово`).
    pub fn is_terminal(&self) -> bool {
        let s = self.as_str();
        s == "ready" || s == "done" || s == "готово"
    }
}

// ============================================================================
// BoardId Value Object
// ============================================================================

/// Identifier for a Kanban board (e.g. `development`, `modeler`, `kanban`, `empty`).
///
/// Invariants:
/// - Leading and trailing whitespace is trimmed.
/// - Normalized to lowercase.
/// - Cannot be empty or blank.
/// - Maximum length of 64 characters.
/// - Valid characters are alphanumeric, `-`, `_`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BoardId(String);

impl BoardId {
    /// Parses and normalizes a board identifier string.
    pub fn parse(s: &str) -> Result<Self, DomainError> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(DomainError::EmptyBoardId);
        }
        if trimmed.len() > 64 {
            return Err(DomainError::InvalidBoardIdFormat(trimmed.to_string()));
        }

        for c in trimmed.chars() {
            if !(c.is_alphanumeric() || c == '-' || c == '_') {
                return Err(DomainError::InvalidBoardIdFormat(trimmed.to_string()));
            }
        }

        let normalized = trimmed.to_lowercase();
        Ok(Self(normalized))
    }

    /// Creates a new `BoardId` from any string-like reference.
    pub fn new(id: impl AsRef<str>) -> Result<Self, DomainError> {
        Self::parse(id.as_ref())
    }

    /// Returns the borrowed string slice representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the `BoardId` and returns the inner `String`.
    pub fn into_inner(self) -> String {
        self.0
    }

    /// Returns `true` if this is the canonical default board (`development`).
    pub fn is_default(&self) -> bool {
        self.as_str() == "development"
    }
}

impl Default for BoardId {
    fn default() -> Self {
        Self("development".to_string())
    }
}

// ============================================================================
// Macro Implementation of Common Traits (Deref, Display, Borrow, Serde, PartialEq)
// ============================================================================

macro_rules! impl_id_traits {
    ($type:ident) => {
        impl Deref for $type {
            type Target = str;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl fmt::Display for $type {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl AsRef<str> for $type {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl Borrow<str> for $type {
            fn borrow(&self) -> &str {
                self.as_str()
            }
        }

        impl FromStr for $type {
            type Err = DomainError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::parse(s)
            }
        }

        impl TryFrom<&str> for $type {
            type Error = DomainError;
            fn try_from(s: &str) -> Result<Self, Self::Error> {
                Self::parse(s)
            }
        }

        impl TryFrom<String> for $type {
            type Error = DomainError;
            fn try_from(s: String) -> Result<Self, Self::Error> {
                Self::parse(&s)
            }
        }

        impl Serialize for $type {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $type {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let raw = String::deserialize(deserializer)?;
                Self::parse(&raw).map_err(serde::de::Error::custom)
            }
        }

        impl PartialEq<str> for $type {
            fn eq(&self, other: &str) -> bool {
                self.as_str() == other
            }
        }

        impl PartialEq<&str> for $type {
            fn eq(&self, other: &&str) -> bool {
                self.as_str() == *other
            }
        }

        impl PartialEq<String> for $type {
            fn eq(&self, other: &String) -> bool {
                self.as_str() == other.as_str()
            }
        }

        impl PartialEq<$type> for str {
            fn eq(&self, other: &$type) -> bool {
                self == other.as_str()
            }
        }

        impl PartialEq<$type> for &str {
            fn eq(&self, other: &$type) -> bool {
                *self == other.as_str()
            }
        }

        impl PartialEq<$type> for String {
            fn eq(&self, other: &$type) -> bool {
                self.as_str() == other.as_str()
            }
        }
    };
}

impl_id_traits!(TaskId);
impl_id_traits!(StageId);
impl_id_traits!(BoardId);
