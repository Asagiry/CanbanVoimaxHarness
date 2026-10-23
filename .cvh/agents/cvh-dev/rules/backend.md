# Backend Architecture & Invariants (Rust 2021, Tauri v2, SQLite WAL)

This guide documents the authoritative backend engineering standards and architectural invariants of **CanbanVoimaxHarness (CVH)** located in `src-tauri/`.

---

## 1. Core Stack & Runtime Architecture

- **Rust Edition 2021**: Idiomatic, safe, high-performance systems programming.
- **Tauri v2 (`tauri = "2"`, `tauri-build = "2"`)**: Desktop framework utilizing native webviews (WebView2 on Windows).
- **Embedded Database (`rusqlite = "0.32"`, bundled)**: High-concurrency ACID transactions operating strictly under WAL mode (`.cvh/harness.db`).
- **Async Runtime (`tokio = "1"`, full)**: Asynchronous ReAct agent loops, background subagents, child process management, and non-blocking IPC.
- **HTTP & SSE Streaming (`reqwest = "0.12"`, `eventsource-stream = "0.2"`)**: LLM communication supporting OpenAI-compatible and Gemini endpoints with real-time token/thought streaming.
- **Error Handling**: Strongly typed domain errors (`thiserror = "1.0"`) and application context propagation (`anyhow = "1.0"`).

---

## 2. Fundamental Architectural Invariants

### 2.1 Single-Instance Plugin Invariant (WebView2 Lock Collision Prevention)
- `tauri-plugin-single-instance` **MUST ALWAYS be the very first registered plugin** in `src-tauri/src/lib.rs`.
- When a secondary process is spawned, the plugin transmits launch arguments to the primary instance and terminates immediately *before* WebView2 initializes.
- This prevents fatal Windows WebView2 User Data Folder lock contention crashes (error code `0x800700AA ERROR_BUSY`).

### 2.2 WebView2 User Data Folder (UDF) Sanitizer (`src-tauri/src/udf.rs`, `main.rs`)
- Implements `resolve_and_prepare_webview2_udf()`, generating verified writeable UDF directories in `%LOCALAPPDATA%\com.voimax.cvh` or `%TEMP%`.
- **Strict Invariant**: NEVER use relative paths for UDF and NEVER place WebView2 UDF inside `.cvh/` project folders or application build directories.
- Injects `--disable-features=RendererCodeIntegrity` into `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` on Windows to eliminate DLL-injection false-positive crashes caused by third-party antiviruses or proxies.

### 2.3 SQLite WAL Mode Ledger (`.cvh/harness.db`, `src-tauri/src/db.rs`)
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

### 2.4 Self-Healing Multi-Board State (`src-tauri/src/commands/tasks.rs`)
- **Orphaned Board Healing**: If `task.board_id` is missing or references a non-existent board, it dynamically heals to `active_board_id`, or `boards[0].id`, or `"development"`.
- **Orphaned Column Healing**: If `task.stage_id` does not exist on the target board, it dynamically heals to the board's first column (`backlog`).
- **Boundary Columns Protection**: `backlog` is normalized to index 0; `ready` (`terminal: true`) is normalized to the final index (`normalize_pipeline_columns`).

### 2.5 Safe Windows Quoting & Worktree Execution (`src-tauri/src/hooks.rs`, `git.rs`)
- Tasks operate inside isolated git worktrees at `worktrees/<task-slug>` on branch `feat/<task-slug>`.
- **Safe Command Formatting**: Windows shell commands MUST use `quote_windows_path()` and `quote_windows_arg()` to prevent shell syntax breakage from spaces, parentheses `()`, and ampersands `&`.
- Windows commands executed via `cmd.exe` use `/D /S /C "chcp 65001 >nul && <command>"` for consistent UTF-8 encoding.

### 2.6 Task Attachments & Vision LLM Pipeline (`src-tauri/src/commands/tasks.rs`, `agent.rs`)
- **Tauri IPC Command**: `create_task` accepts optional `attachments: Option<Vec<String>>`, passed through `internal_create_task` and persisted in SQLite `tasks.attachments` as serialized JSON array.
- **LLM Vision Context**: `AgentSessionConfig` carries `task_attachments: Option<Vec<String>>`. When starting turn 0, if attachments are present, the initial user prompt is packaged via `ChatMessage::user_with_images(initial_user_prompt, atts)`. Reconstructed SQLite dialogue contexts also maintain vision attachments on `messages[1]`.

---

## 3. Rust Systems Coding Standards

### 3.1 Ownership, Immutability & Zero-Cost Idioms
- **Immutable by Default**: Use `let` by default; only use `let mut` when in-place mutation is explicitly required.
- **Borrowing Over Cloning**: Borrow (`&T`, `&str`, `&[T]`) by default. Never clone merely to satisfy the borrow checker without understanding lifetime requirements.
- **Cow Allocation**: Use `Cow<'_, T>` when a function may or may not need to allocate an owned representation.
- **Constructors**: Use `impl Into<String>` for constructor parameters that need to own a string.

### 3.2 Error Handling Discipline
- **Zero Naked Unwraps**: Never call `.unwrap()` or `.expect()` in production application paths.
- **Typed Errors vs Context**:
  - Library modules use strongly typed enums with `thiserror::Error`.
  - Application command boundaries use `anyhow::Context` (`.with_context(|| format!("failed to ..."))?`).
- **Async Runtime Safety**: Never run heavy synchronous CPU-bound loops or blocking disk I/O directly inside Tokio async tasks; wrap in `tokio::task::spawn_blocking`. Channels must be bounded (`tokio::sync::mpsc::channel(buffer)`).

---

## 4. NASA Ground Truth Turbo Sensors

- **Verification Mandate**: Changes must be verified against real compiler and linter sensors before handoff:
  - `cargo check --manifest-path src-tauri/Cargo.toml` (0 errors).
  - `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` (0 warnings).
- **Anti-Mock Invariant**: Verify actual database writes, process spawns, and IPC contracts rather than purely synthetic in-memory mocks.
