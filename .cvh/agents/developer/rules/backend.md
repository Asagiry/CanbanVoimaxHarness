# Backend Architecture & Invariants (Rust 2021, Tauri v2, SQLite WAL)

Authoritative backend engineering standards and architectural invariants of **CanbanVoimaxHarness (CVH)** located in `src-tauri/`.

---

## 1. Core Stack & Runtime Architecture

- **Rust Edition 2021**: Idiomatic, safe, high-performance systems programming.
- **Tauri v2 (`tauri = "2"`, `tauri-build = "2"`)**: Desktop framework utilizing native webviews (WebView2 on Windows).
- **Embedded Database (`rusqlite = "0.32"`, bundled)**: High-concurrency ACID transactions operating strictly under WAL mode (`.cvh/harness.db`).
- **Async Runtime (`tokio = "1"`, full)**: Asynchronous ReAct agent loops, background subagents, child process management, and non-blocking IPC.
- **HTTP & SSE Streaming (`reqwest = "0.12"`, `eventsource-stream = "0.2"`)**: LLM communication supporting OpenAI-compatible and Gemini endpoints with real-time token/thought streaming.
- **Error Handling**: Strongly typed domain errors (`thiserror = "1.0"`) and application context propagation (`anyhow = "1.0"`).

---

## 2. Modular Subsystem Topology & Directory Layout

To eliminate monolithic sprawl and guarantee maintainability, backend logic is strictly partitioned into single-responsibility domain modules.

### 2.1 Persistence Layer (`src-tauri/src/db/`)
The legacy monolithic `db.rs` is strictly eradicated. All database operations reside in specialized repositories:
- `db/pool.rs`: Connection pooling, WAL configuration (`PRAGMA journal_mode = WAL`, `PRAGMA synchronous = NORMAL`, `PRAGMA foreign_keys = ON`), and migration execution.
- `db/task_repo.rs`: Core task entity CRUD, subtask hierarchies, parent-child cascades, and persistent monotonic task numbering (`allocate_next_task_num`).
- `db/dialog_repo.rs`: Activity feed items, real-time streaming tokens, thoughts, tool call records, and state-preserving compacted dialogue persistence.
- `db/telemetry_repo.rs`: Turn-level telemetry records (TPS, TTFT, token consumption) and permanent `task_ledger` audit records.
- `db/mod.rs`: Facade re-exporting repository functions for 100% backward compatibility.

### 2.2 Configuration & Project Harness Layer (`src-tauri/src/config/`)
The legacy monolithic `config.rs` is partitioned into modular submodules:
- `config/models.rs`: LLM model registry, active model resolution, board definitions, and pipeline routing structures.
- `config/tool_policy.rs`: Two-level tool policy matrix enforcement (Global switches AND Stage overrides), schema pruning, and persona frontmatter synchronization.
- `config/discovery.rs`: Multi-tier workspace root discovery (local directory, parent traversal, appdata cache).
- `config/scaffold.rs`: Fresh project scaffolding, default 3-column agile boards, and auto-gitignore enforcement (`.cvh/llm.json`, `.cvh/worktrees/`).
- `config/io.rs`: Safe, atomic disk read/write operations for harness JSON settings.

### 2.3 Domain Model & Value Objects (`src-tauri/src/domain/`)
Implements DDD principles and Uncle Bob's Clean Architecture:
- `domain::ids`: Immutable nominal Value Objects (`TaskId`, `StageId`, `BoardId`) with compile-time branding, strict regex enforcement (`^[A-Z0-9-_.:]+$`), case normalization, and deref coercion.
- `domain::state`: Algebraic Data Type `TaskState` (`Idle`, `Running { model }`, `WaitingSubtask { waiting_for_subtask_id }`, `Done { verdict }`), making illegal lifecycle states mathematically unrepresentable at compile time.

### 2.4 Agent Runtime & ReAct Engine (`src-tauri/src/agent/`)
- `agent::react_engine`: `ReActEngine<Driver>` generic execution engine eliminating duplication between root agent loops and in-process subagents. Provides unified SSE streaming chunk consumption, telemetry calculation (TTFT, TPS, Cache Hit Rate), abort checking, and SQLite persistence.
- `agent/compactor.rs`: State-preserving context compactor executing zero-LLM fast evaporation when context reaches 100% capacity.
- `agent/tool_schemas.rs`: Strict JSON schema definitions filtered dynamically against effective tool policies.
- `agent/tools/`: Modular tool handlers partitioned into domain submodules (`file_ops.rs`, `command.rs`, `web.rs`, `pipeline.rs`, `tasks.rs`, `planning.rs`, `subagent.rs`).

### 2.5 Anti-Monolith Invariant (Module Size Ceiling & SRP)
- **Ceiling Mandate**: The maximum size of any new or refactored Rust backend module is strictly capped at **350–500 lines**.
- **Prohibition of Kitchen-Sink Files**: Never append unrelated helper functions, extra database queries, or command handlers into single files.
- If a module approaches or exceeds 500 lines, it MUST be decomposed into focused submodules (repositories, domain models, or distinct service drivers) following Single Responsibility Principle (SRP).

---

## 3. Fundamental Architectural Invariants

### 3.1 Single-Instance Plugin Invariant (WebView2 Lock Collision Prevention)
- `tauri-plugin-single-instance` **MUST ALWAYS be the very first registered plugin** in `src-tauri/src/lib.rs`.
- When a secondary process is spawned, the plugin transmits launch arguments to the primary instance and terminates immediately *before* WebView2 initializes.
- This prevents fatal Windows WebView2 User Data Folder lock contention crashes (error code `0x800700AA ERROR_BUSY`).

### 3.2 WebView2 User Data Folder (UDF) Sanitizer (`src-tauri/src/udf.rs`, `main.rs`)
- Implements `resolve_and_prepare_webview2_udf()`, generating verified writeable UDF directories in `%LOCALAPPDATA%\com.voimax.cvh` or `%TEMP%`.
- **Strict Invariant**: NEVER use relative paths for UDF and NEVER place WebView2 UDF inside `.cvh/` project folders or application build directories.
- Injects `--disable-features=RendererCodeIntegrity` into `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` on Windows to eliminate DLL-injection false-positive crashes caused by third-party antiviruses or proxies.

### 3.3 SQLite WAL Mode Ledger (`.cvh/harness.db`, `src-tauri/src/db/`)
- High-concurrency ACID transactions configured with:
  ```sql
  PRAGMA journal_mode = WAL;
  PRAGMA synchronous = NORMAL;
  PRAGMA foreign_keys = ON;
  ```
- **Tables**:
  - `tasks`: Core task entities with parent/subtask relations (`FOREIGN KEY (parent_id) REFERENCES tasks(id) ON DELETE CASCADE`).
  - `task_todos`: Checklist items maintained via `todo_write`.
  - `dialog_items`: Activity feed containing streaming tokens, thoughts, tool calls, and tool outputs. Indexed by `(task_id, created_at)`.
  - `sequences`: Monotonic counters for non-reusable task numbering (`CVH-001`, `CVH-002`). Deleted IDs are never recycled.
  - `task_ledger`: Permanent audit records of engineering decisions (summary, commit hash, author, changed files, verdict).
  - `telemetry`: Token consumption metrics (input, output, cache read tokens, TPS, duration).
- **Atomic Transactions**: All mutations (`db_upsert_task`, `append_dialog_items`, `persist_compacted_dialog`) MUST run within explicit `conn.transaction()` blocks to eliminate partial state corruption.
- **Strict Parameter Binding**: Always use parameterized queries (`params![...]`, `named_params!{...}`). NEVER interpolate runtime variables into SQL strings via `format!("... WHERE id = '{id}'")` — prevent SQL injection and database corruption.

### 3.4 Self-Healing Multi-Board State (`src-tauri/src/commands/tasks/healing.rs`)
- **Orphaned Board Healing**: If `task.board_id` is missing or references a non-existent board, it dynamically heals to `active_board_id`, or `boards[0].id`, or `"development"`.
- **Orphaned Column Healing**: If `task.stage_id` does not exist on the target board, it dynamically heals to the board's first column (`backlog`).
- **Boundary Columns Protection**: `backlog` is normalized to index 0; `ready` (`terminal: true`) is normalized to the final index (`normalize_pipeline_columns`).

### 3.5 Safe Windows Quoting & Worktree Execution (`src-tauri/src/hooks.rs`, `git.rs`)
- Tasks operate inside isolated git worktrees at `worktrees/<task-slug>` on branch `feat/<task-slug>`.
- **Safe Command Formatting**: Windows shell commands MUST use `quote_windows_path()` and `quote_windows_arg()` to prevent shell syntax breakage from spaces, parentheses `()`, and ampersands `&`.
- Windows commands executed via `cmd.exe` use `/D /S /C "chcp 65001 >nul && <command>"` for consistent UTF-8 encoding.

### 3.6 Task Attachments & Vision LLM Pipeline
- **Attachment Persistence**: Task image attachments are persisted in task definitions and inherited by child subtasks.
- **Vision Model Context**: When attachments are present, the agent runtime packages image payloads into the initial user prompt using multimodal vision message constructs, preserving image references across both in-memory contexts and SQLite dialogue history.

---

## 4. Rust Systems Coding Standards

### 4.1 Ownership, Immutability & Zero-Cost Idioms
- **Immutable by Default**: Use `let` by default; only use `let mut` when in-place mutation is explicitly required.
- **Borrowing Over Cloning**: Borrow (`&T`, `&str`, `&[T]`) by default. Never clone merely to satisfy the borrow checker without understanding lifetime requirements.
- **Cow Allocation**: Use `Cow<'_, T>` when a function may or may not need to allocate an owned representation.
- **Constructors**: Use `impl Into<String>` for constructor parameters that need to own a string.

### 4.2 Error Handling & Zero Silent Failures (silent-failure-hunter)
- **Zero Naked Unwraps**: Never call `.unwrap()` or `.expect()` in production application paths.
- **Typed Errors vs Context**:
  - Library modules use strongly typed enums with `thiserror::Error`.
  - Application command boundaries use `anyhow::Context` (`.with_context(|| format!("failed to ..."))?`).
- **Zero Silent Failures (Silent Failure Ban)**:
  - Never swallow errors into silent defaults (`let _ = fallible_op()` or `Err(_) => Default::default()`) without explicit diagnostic logging or documented graceful degradation.
  - Distinguish between expected absence (`Option::None`) and unexpected failure (`Result::Err`). Never convert critical DB, I/O, or IPC failures into `None` silently via `.ok()` without context.
  - Retain causality: wrap errors with actionable messages that preserve the underlying error chain.
- **Parse, Don't Validate at Boundaries**:
  - Validate and convert unstructured data (raw IPC JSON, environment variables, config strings) into strongly typed Rust structs or enums immediately at the system boundary (`#[derive(Deserialize)]`).
  - Rely 100% on the Rust type system inside internal domain modules.
- **Async Runtime Safety**: Never run heavy synchronous CPU-bound loops or blocking disk I/O directly inside Tokio async tasks; wrap in `tokio::task::spawn_blocking`. Channels must be bounded (`tokio::sync::mpsc::channel(buffer)`).

### 4.3 Code-Over-Documentation Ground Truth Invariant (Anti-Stale-Docs)
- **Code As Ground Truth**: When a repository's documentation, architectural markdown files, specifications, or guidelines disagree with the running, verified codebase, follow the conventions used in the existing code.
- **Inconsistency Reporting Mandate**: Never refactor working, tested code to conform to obsolete, unmaintained, or conflicting documentation. In your final response or report, explicitly list each inconsistency found between the documentation and the code so documentation can be synchronized.
- **Instruction-Hierarchy Bias Defense**: Prevents models from prioritizing stale textual documentation over actual compiler-tested, sensor-verified production code.
