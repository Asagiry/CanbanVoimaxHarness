# Backend Architecture & Invariants (Rust, Tauri v2, SQLite WAL)

This guide documents the authoritative backend architecture of **CanbanVoimaxHarness (CVH)** located in `src-tauri/`.

---

## 1. Core Technologies & Dependencies

- **Rust Edition 2021**: Idiomatic, high-performance systems programming.
- **Tauri v2 (`tauri = "2"`, `tauri-build = "2"`)**: Desktop application framework using native webviews (WebView2 on Windows).
- **Embedded Database (`rusqlite = "0.32"`, bundled)**: High-throughput local database operating strictly under WAL mode (`.cvh/harness.db`).
- **Async Runtime (`tokio = "1"`, full)**: Powers asynchronous ReAct agent loops, background subagents, child process management, and file I/O.
- **HTTP & SSE Streaming (`reqwest = "0.12"`, `eventsource-stream = "0.2"`)**: LLM communication supporting OpenAI-compatible endpoints with real-time token/thought streaming.
- **Error Handling (`thiserror = "1.0"`, `anyhow = "1.0"`)**: Strongly typed domain errors (`AppError`) and context propagation.

---

## 2. Fundamental Architectural Invariants

### 2.1 Single-Instance Plugin Invariant (WebView2 Lock Collision Prevention)
- `tauri-plugin-single-instance` **MUST ALWAYS be the very first registered plugin** in `src-tauri/src/lib.rs`.
- When a secondary process is spawned (e.g. user clicks shortcut while app is running), the plugin transmits launch arguments to the primary instance and terminates immediately *before* WebView2 initializes.
- This prevents fatal Windows WebView2 User Data Folder lock contention crashes (error code `0x800700AA ERROR_BUSY`).

### 2.2 WebView2 User Data Folder (UDF) Sanitizer (`src-tauri/src/udf.rs`, `main.rs`)
- Implements `resolve_and_prepare_webview2_udf()`, generating verified writeable UDF directories in `%LOCALAPPDATA%\com.voimax.cvh` or `%TEMP%`.
- **Strict Invariant**: NEVER use relative paths for UDF and NEVER place WebView2 UDF inside `.cvh/` project folders or application build directories.
- Disables Chromium `RendererCodeIntegrity` via `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` on Windows to eliminate DLL-injection false-positive crashes caused by third-party antiviruses or proxies.

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
  - `task_ledger`: Engineering decision records (summary, commit hash, author, changed files, verdict).
  - `telemetry`: Token consumption metrics (input, output, cache read tokens, TPS, duration).
- **Atomic Transactions**: All mutations (`db_upsert_task`, `append_dialog_items`, `persist_compacted_dialog`) run within explicit `conn.transaction()` blocks to eliminate partial state corruption.

### 2.4 Self-Healing Multi-Board State (`heal_tasks`, `src-tauri/src/commands/tasks.rs`)
- **Orphaned Board Healing**: If `task.board_id` is missing or references a non-existent board, it dynamically heals to `active_board_id`, or `boards[0].id`, or `"development"`.
- **Orphaned Column Healing**: If `task.stage_id` does not exist on the target board, it dynamically heals to the board's first column (typically `"backlog"`).
- **Boundary Columns Protection**: `backlog` is normalized to index 0; `ready` (`terminal: true`) is normalized to the final index (`normalize_pipeline_columns`). Duplicate column IDs are rejected.

### 2.5 ReAct Agent Execution Loop (`src-tauri/src/agent.rs`)
- **Zero-Turn Memory Priming**: Compiles strictly static system prompts from `.cvh/agents/<role>/persona.md` and rules. No dynamic timestamps or UUIDs in the system prompt to maximize LLM prompt caching.
- **Two-Level Tool Policy Matrix (AND-Semantics)**:
  `effective(tool, stage) = globalEnabled[tool] && stageOverrides[stage][tool]`. Disabled tools are completely stripped from JSON schemas sent to the LLM.
- **Zero-Cost Log Evaporation (`compactor.rs`)**:
  Tool outputs exceeding 500 characters or spilled logs (`.cvh/spill/`) older than 2 turns are evaporated into lightweight summaries, preserving token budgets.
- **Large Output Spilling**: Command outputs >500 lines or >64 KB are automatically spilled to `.cvh/spill/`, returning head/tail slices.
- **English-Only Schemas**: All tool names and parameter descriptions sent to LLMs must be strictly in English for ~70% BPE token savings.

### 2.6 Git Worktree Lifecycle & Safe Windows Quoting (`src-tauri/src/git.rs`, `hooks.rs`)
- Tasks operate inside isolated git worktrees at `worktrees/<task-slug>` on branch `feat/<task-slug>`.
- **Safe Command Formatting**: Windows shell commands MUST use `quote_windows_path()` and `quote_windows_arg()` to prevent shell syntax breakage from spaces, parentheses `()`, and ampersands `&`.
- Windows commands executed via `cmd.exe` use `/D /S /C "chcp 65001 >nul && <command>"` for consistent UTF-8 encoding.

---

## 3. Development, Verification & Clippy Guidelines

- **Sensors**: Always verify changes using `cargo check --manifest-path src-tauri/Cargo.toml`.
- **Zero Naked Unwraps**: Never use `.unwrap()` or `.expect()` in production paths. Use strongly typed errors with `Result<T, AppError>` or `anyhow::Context`.
- **Clippy Debt**: Code must pass `cargo clippy` without warnings.
- **Clean Imports & Ponytail Minimalism**: No unused crates, no dead code, no placeholder mocks.
