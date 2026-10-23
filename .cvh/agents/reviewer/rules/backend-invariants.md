# Rule: Backend Architectural Invariants & Security Audit (backend-invariants.md)

Authoritative backend audit checklist for the **Reviewer** persona in CanbanVoimaxHarness.

---

## 1. Zero Naked Unwraps in Production Paths
- **Rule**: Never allow `.unwrap()` or `.expect()` in non-test Rust code (`src-tauri/src/`).
- **Audit**: Grep the diff for `\.unwrap\(` or `\.expect\(`. If present in production code:
  - Check whether it can panic on unexpected runtime data.
  - If yes, **REJECT IMMEDIATELY**: Demand `?` operator with typed errors or `.context(...)`.

---

## 2. Single-Instance Plugin Registration Ordering
- **Rule**: `tauri_plugin_single_instance::init(...)` MUST remain the **first** plugin registered in `src-tauri/src/lib.rs`.
- **Reasoning**: If another plugin initializes WebView2 before single-instance takes effect, secondary process launches will crash with Windows error `0x800700AA ERROR_BUSY`.
- **Audit**: Inspect plugin initialization order in `lib.rs`. If altered, **REJECT**.

---

## 3. WebView2 User Data Folder (UDF) Hygiene
- **Rule**: UDF path must be strictly absolute and point to verified writeable directories (`%LOCALAPPDATA%\com.voimax.cvh` or `%TEMP%`).
- **Audit**: Verify that no changes place the UDF inside `.cvh/` or application root directories, and no relative paths are passed.

---

## 4. SQLite WAL Mode & Transaction Integrity
- **Rule**: All multi-statement mutations in `src-tauri/src/db.rs` must execute inside explicit `conn.transaction()?` blocks.
- **Audit**: Inspect any database insert, update, or delete operations. If mutations are uncommitted or lack transaction wrappers, **REJECT**.

---

## 5. Safe Windows Quoting for Process Spawning
- **Rule**: All child process spawns in `hooks.rs` or `git.rs` must use `quote_windows_path()` and `quote_windows_arg()`.
- **Audit**: Check command line construction. Naked string interpolation like `format!("cmd /c {}", path)` is forbidden due to path spaces and special characters (`()`, `&`).

---

## 6. Terminal Stage Cleansing & Relational Subtask Invariant
- **Rule**: Any task entering a terminal stage (`ready`, `done`) or with status `done` must have `waiting_for_subtask_id` set to `None`. Subtasks marked `done` must never leave parent tasks in `waiting_subtask` or with stale `waiting_for_subtask_id`.
- **Audit**: Inspect transitions in `tasks.rs` and `boardStore.ts`. Ensure `heal_tasks`, `save_tasks`, and `internal_move_task_stage` clear `waiting_for_subtask_id` and never resurrect completed subtasks.
