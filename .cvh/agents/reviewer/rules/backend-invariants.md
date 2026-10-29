---
description: Backend security, concurrency, SQLite WAL pooling, error propagation, and memory safety review checklist.
---
# Rule: Backend Architectural Invariants & Security Audit (backend-invariants.md)

Authoritative backend audit checklist for the **Reviewer** persona in CanbanVoimaxHarness.

---

## 1. Single Responsibility Principle (SRP) & Modular Directory Architecture
- **Rule**: Rust backend codebase is partitioned into distinct single-responsibility subpackages under `src-tauri/src/`:
  - `db/`: High-performance SQLite database connection pool (`pool.rs`) and repository modules (`task_repo.rs`, `dialog_repo.rs`, `telemetry_repo.rs`). Pure SQL queries, indexing, and transactional persistence.
  - `config/`: Configuration parsing, project discovery (`discovery.rs`), I/O operations (`io.rs`), models (`models.rs`), scaffolding (`scaffold.rs`), and two-level tool policy enforcement (`tool_policy.rs`).
  - `domain/`: Pure domain layer containing Value Objects (`domain::ids`) and lifecycle state machines (`domain::state`). Free of external I/O or GUI concerns.
  - `agent/`: ReAct loop orchestration, context compaction (`compactor.rs`), token telemetry, sanitizers, and modular tool implementations (`tools/`).
- **Monolithic File Size Ceiling (<= 500 lines)**:
  - Any new file or significantly refactored module in `src-tauri/src/` must not exceed 500 lines of code.
  - **Audit**: If a diff introduces or expands a backend module beyond 500 lines, **REJECT IMMEDIATELY**: Mandate decomposition into submodules within the corresponding domain package (e.g. splitting into `runner.rs`, `storage.rs`, `lifecycle.rs`).
  - *Note on Legacy Files*: Pre-existing historical files retain immunity (§ 4.1), but new code must adhere strictly to the 500-line ceiling.

---

## 2. Elimination of Primitive Obsession via `domain::ids`
- **Rule**: Raw primitive strings (`String`, `&str`) must NEVER be used across domain boundaries for entities with strict structural rules.
- **Audit**: Verify usage of Domain Value Objects from `crate::domain::ids`:
  - `TaskId`: Validated, trimmed, uppercase-normalized task and subtask identifier.
  - `StageId`: Validated stage identifier.
  - `BoardId`: Validated board identifier.
- **REJECTION TRIGGER**: Passing unvalidated raw strings for task IDs or stage IDs in domain transitions or repository boundaries is strictly prohibited.

---

## 3. Make Illegal States Unrepresentable via `domain::state`
- **Rule**: Task lifecycles and states MUST be modeled using Algebraic Data Types (`TaskState` enum) rather than loose, mutually conflicting struct flags.
- **Audit**: Inspect state representations against `crate::domain::state::TaskState`:
  - `TaskState::Idle`
  - `TaskState::Running { model }`
  - `TaskState::WaitingSubtask { subtask_id }`
  - `TaskState::Done { verdict }`
- **REJECTION TRIGGER**: Adding combinations of boolean fields (e.g. `is_running: bool`, `waiting: bool`) where conflicting permutations (e.g. `Done` while simultaneously waiting for a subtask) can exist. Illegal states must be mathematically unrepresentable at compile time.

---

## 4. Zero Naked Unwraps in Production Paths
- **Rule**: Never allow `.unwrap()` or `.expect()` in non-test Rust code (`src-tauri/src/`).
- **Audit**: Grep the diff for `\.unwrap\(` or `\.expect\(`. If present in production code:
  - Check whether it can panic on unexpected runtime data.
  - If yes, **REJECT IMMEDIATELY**: Demand `?` operator with typed errors or `.context(...)`.

---

## 5. Single-Instance Plugin Registration Ordering
- **Rule**: `tauri_plugin_single_instance::init(...)` MUST remain the **first** plugin registered in `src-tauri/src/lib.rs`.
- **Reasoning**: If another plugin initializes WebView2 before single-instance takes effect, secondary process launches will crash with Windows error `0x800700AA ERROR_BUSY`.
- **Audit**: Inspect plugin initialization order in `lib.rs`. If altered, **REJECT**.

---

## 6. WebView2 User Data Folder (UDF) Hygiene
- **Rule**: UDF path must be strictly absolute and point to verified writeable directories (`%LOCALAPPDATA%\com.voimax.cvh` or `%TEMP%`).
- **Audit**: Verify that no changes place the UDF inside `.cvh/` or application root directories, and no relative paths are passed.

---

## 7. SQLite WAL Mode & Parameter Binding Invariant (Zero SQL Injection)
- **Rule**: All SQLite mutations in `src-tauri/src/db/` must execute inside explicit `conn.transaction()?` blocks, and all dynamic variables must use parameterized bindings (`params![...]` or `named_params!{...}`).
- **Audit**: Grep diff for string formatting inside SQL statements (`format!("SELECT ... ` or `format!("INSERT ... `). If found, **REJECT IMMEDIATELY**.

---

## 8. Safe Windows Quoting for Process Spawning
- **Rule**: All child process spawns in `hooks.rs` or `git.rs` must use `quote_windows_path()` and `quote_windows_arg()`.
- **Audit**: Check command line construction. Naked string interpolation like `format!("cmd /c {}", path)` is forbidden due to path spaces and special characters (`()`, `&`).

---

## 9. Code-Over-Documentation Ground Truth Invariant (Anti-Stale-Docs)
- **Rule**: When a repository's documentation and its existing code disagree, follow the conventions used in the existing code, and in your final reply point out each inconsistency you found between the documentation and the code.
- **Audit**: Reject changes that break tested, compiling, or working code in an attempt to conform to obsolete documentation, stale specifications, or outdated comments. Mandate updating the documentation to reflect reality instead.

---

## 10. Zero Silent Failures Audit (silent-failure-hunter)
- **Rule**: Critical operations (database writes, file I/O, IPC handlers, child process spawns) must not silently drop errors.
- **Audit**: Inspect error handling in the diff:
  - Reject `let _ = ...` on fallible operations that can fail without warning.
  - Reject `.ok()` or `unwrap_or_default()` on core business logic or storage queries where an error signifies an actionable failure.
  - Mandate proper error propagation with context via `?` and `.with_context(...)`.