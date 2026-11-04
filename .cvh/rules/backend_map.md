---
description: Authoritative navigation map and file-by-file router for the Rust Tauri backend (src-tauri/src/).
---
# Backend Navigation Map & File-by-File Router (backend_map.md)

Authoritative directory topology, file mapping table, and subsystem architecture for the Rust 2021 / Tauri v2 backend of **CanbanVoimaxHarness (`src-tauri/src/`)**.

---

## 1. Quick File-by-File Router ("What to modify" -> "Exact file path")

| What you need to build / modify | Exact File Path | Key Invariants & Ceilings |
| :--- | :--- | :--- |
| **Domain Facade & Module Exports** | `src-tauri/src/domain/mod.rs` | Submodule re-exports (`ids`, `state`). |
| **Domain Nominal Types & IDs** | `src-tauri/src/domain/ids.rs` | `TaskId`, `StageId`, `BoardId`. Zero raw strings. |
| **Task State Machine & Transitions** | `src-tauri/src/domain/state.rs` | `TaskState` ADT (`Idle`, `Running`, `WaitingSubtask`, `WaitingSubagent`, `Done`). |
| **Database Facade & Module Exports** | `src-tauri/src/db/mod.rs` | Submodule re-exports (`pool`, `task_repo`, `dialog_repo`, `telemetry_repo`). |
| **Database Pool & Migrations** | `src-tauri/src/db/pool.rs` | R2D2 pool, WAL mode, foreign keys, 5s timeout. |
| **Task SQLite CRUD & Monotonic IDs** | `src-tauri/src/db/task_repo.rs` | Monotonic ID allocation, subtask cascade. |
| **Dialogue Items & Streaming Chunks** | `src-tauri/src/db/dialog_repo.rs` | Atomic upsert, tool JSON persistence. |
| **Token Usage & TPS Telemetry Repo** | `src-tauri/src/db/telemetry_repo.rs` | Aggregated tokens, non-decaying TPS metrics. |
| **Config Facade & Module Exports** | `src-tauri/src/config/mod.rs` | Submodule re-exports (`models`, `tool_policy`, `discovery`, `scaffold`, `io`). |
| **Pipeline & Board Data Models** | `src-tauri/src/config/models.rs` | `BoardDef`, `PipelineConfig`, embedded rules. |
| **Tool Policy Matrix & Authorization** | `src-tauri/src/config/tool_policy.rs` | Single Authoritative Layer: `stage_override ?? global_enabled ?? true`. |
| **Workspace Root Discovery** | `src-tauri/src/config/discovery.rs` | Multi-tier root discovery, `%LOCALAPPDATA%`. |
| **Scaffolding & Auto-Gitignore** | `src-tauri/src/config/scaffold.rs` | Automatic scaffold, auto-gitignore policies. |
| **Atomic File I/O** | `src-tauri/src/config/io.rs` | `atomic_write_file`, JSON load/save. |
| **Commands Facade & Module Exports** | `src-tauri/src/commands/mod.rs` | Top-level IPC command module aggregation. |
| **Central Application State Container** | `src-tauri/src/commands/state.rs` | `AppState`, connection pool, workspace root, active session registry. |
| **Task IPC Command Dispatcher** | `src-tauri/src/commands/tasks.rs` | IPC invoke gateway routing for tasks and agent lifecycle. |
| **Task Stage Transition Pipeline** | `src-tauri/src/commands/tasks/transitions.rs` | `TaskTransitionPipeline`, hooks, gatekeeper sensors. |
| **Task Lifecycle (Create, Delete, Subtask)**| `src-tauri/src/commands/tasks/lifecycle.rs` | Backend-authoritative card lifecycle, auto-unlock. |
| **Agent Runner Trigger & Steer** | `src-tauri/src/commands/tasks/runner.rs` | `run_agent`, `steer_agent`, active task registry. |
| **Task Serialization & Storage** | `src-tauri/src/commands/tasks/storage.rs` | `list_tasks`, `sync_tasks_to_disk`, foreign task filtration. |
| **Board & Column Self-Healing** | `src-tauri/src/commands/tasks/healing.rs` | Orphan board/column resolution on load. |
| **Cortex Rules & Personas File IPC** | `src-tauri/src/commands/cortex.rs` | Persona/rules file operations, YAML frontmatter. |
| **Boards & Pipeline IPC** | `src-tauri/src/commands/boards.rs` | Multi-board CRUD, column settings, pipeline sync. |
| **Dialogs History & Stream IPC** | `src-tauri/src/commands/dialogs.rs` | Dialogue history retrieval, steer persistence. |
| **Git Worktree & Branch IPC** | `src-tauri/src/commands/git.rs` | Worktree lifecycle, branch diffs, commit queries. |
| **LLM Settings & Test IPC** | `src-tauri/src/commands/llm.rs` | Endpoint config, model verification, test calls. |
| **Project Workspace Switching IPC** | `src-tauri/src/commands/projects.rs` | Workspace root switching, recent projects list. |
| **Tool Policy IPC** | `src-tauri/src/commands/tools.rs` | Tool policy inspection, stage overrides & matrix updates. |
| **Desktop Notifications & Windows Fallback** | `src-tauri/src/commands/notifications.rs` | Native WinRT toasts with PowerShell fallback, AUMID registration. |
| **ReAct Runtime & Prompt Compiler** | `src-tauri/src/agent.rs` | Core ReAct agent loop, tool dispatch, prompt priming & turn orchestration. |
| **Subagent & Session Registry** | `src-tauri/src/agent/session_registry.rs` | Active & background subagents, completion channels, lifecycle. |
| **Generic ReAct Engine Loop** | `src-tauri/src/agent/react_engine.rs` | `ReActEngine<Driver>`, unified streaming. |
| **State-Preserving Context Compactor**| `src-tauri/src/agent/compactor.rs` | `ToolCallSummaryExtractor` (CC=1), 100% ceiling. |
| **Unclosed Tool Call Sanitizer** | `src-tauri/src/agent/sanitizer.rs` | Tag stripping, unclosed JSON repairs. |
| **Non-Decaying TPS Telemetry** | `src-tauri/src/agent/telemetry.rs` | Frozen TPS on generation stop, TTFT split. |
| **5-Tier Fuzzy Text Matcher** | `src-tauri/src/agent/text_match.rs` | CRLF/LF adaptation, whitespace relaxation. |
| **LLM Tool Schemas & Definitions** | `src-tauri/src/agent/tool_schemas.rs` | JSON schemas, dynamic rule enumeration. |
| **Agent Tools Facade & Registry** | `src-tauri/src/agent/tools/mod.rs` | Tool executors re-exports and dispatch contracts. |
| **File Operations Tools** | `src-tauri/src/agent/tools/file_ops.rs` | `read_file`, `write_file`, `edit_file`, `glob`, `grep`. |
| **Command Execution Tool** | `src-tauri/src/agent/tools/command.rs` | `execute_command` (alias `bash`), timeout, stdio. |
| **Pipeline Navigation Tools** | `src-tauri/src/agent/tools/pipeline.rs` | `advance_task`, `reject_task`, relay handoff. |
| **Planning & Checklist Tools** | `src-tauri/src/agent/tools/planning.rs` | `todo_write`, `question` (ask_user). |
| **Modular Rule Loader Tool** | `src-tauri/src/agent/tools/rules.rs` | `load_rule` (JIT dynamic rule loader). |
| **Subagent Hierarchy Tools** | `src-tauri/src/agent/tools/subagent.rs` | `subagent`, `subagent_bg`, `collect_subagent`, `subagent_list`. |
| **Task Card & Query Tools** | `src-tauri/src/agent/tools/tasks.rs` | `create_task`, `create_tasks`, `complete_task`, `list_tasks`, `get_task_history`. |
| **Web Research Tools** | `src-tauri/src/agent/tools/web.rs` | `web_search`, `web_fetch`. |
| **Git Worktree Manager** | `src-tauri/src/git.rs` | Worktree isolation, prune, branch teardown. |
| **Lifecycle Hooks Runner** | `src-tauri/src/hooks.rs` | Transition scripts, process tree kill on Windows. |
| **LLM Facade & Module Exports** | `src-tauri/src/llm/mod.rs` | LLM client and models module re-exports. |
| **Universal LLM Client & SSE Stream** | `src-tauri/src/llm/client.rs` | OpenAI / Gemini streaming, thought extraction. |
| **LLM Configuration & Models** | `src-tauri/src/llm/models.rs` | Model overrides, endpoints, temperature. |
| **Custom SQLite UDFs** | `src-tauri/src/udf.rs` | Custom regex and search functions for SQLite connection. |
| **Structured Logger** | `src-tauri/src/logger.rs` | Rotating file & terminal logging configuration. |
| **Error Hierarchy & IPC Conversions** | `src-tauri/src/error.rs` | Domain errors and Tauri IPC error formatting. |
| **Tauri Plugin & IPC Registration** | `src-tauri/src/lib.rs` | Tauri invoke handler wiring and application setup. |
| **Desktop Executable Entrypoint** | `src-tauri/src/main.rs` | Windows subsystem bootstrap, panic hook, crash telemetry. |

---

## 2. Deep Subsystem Architectures

### 2.1 Persistence Subsystem (`src-tauri/src/db/`)
- **`pool.rs`**: Implements thread-safe `r2d2::Pool<SqliteConnectionManager>` for `<repo_root>/.cvh/harness.db`. Enforces `PRAGMA journal_mode = WAL`, `PRAGMA synchronous = NORMAL`, `PRAGMA foreign_keys = ON`, and `busy_timeout = 5000ms`. Executes initial migrations for `tasks`, `dialog_items`, and `telemetry`.
- **`task_repo.rs`**: Handles task entity persistence. Manages parent-subtask cascades, monotonic ID allocation (`allocate_next_task_num`), and relational state consistency.
- **`dialog_repo.rs`**: High-concurrency dialogue storage. Atomically upserts thoughts, streaming chunks, tool execution records, and steer events into `dialog_items` table indexed by `(task_id, created_at)`.
- **`telemetry_repo.rs`**: Persists token usage, duration, and generation performance metrics. Provides aggregated stats for reporting.

### 2.2 Configuration & Scaffolding Subsystem (`src-tauri/src/config/`)
- **`models.rs`**: Canonical data contracts (`BoardDef`, `BoardsConfig`, `PipelineConfig`, `ColumnDef`, `AgentSettings`). Defines system directory resolvers (`agent_rules_dir`, `global_rules_dir`, `tool_policy_path`).
- **`tool_policy.rs`**: Two-level tool authorization engine with Single Authoritative Layer: `stage_overrides` dictation takes precedence over `global_enabled` defaults (`effective(tool, stage) = stage_override[stage][tool] ?? global_enabled[tool] ?? true`). Zero-trust stripping purges disabled tools from LLM schemas.
- **`discovery.rs`**: Four-tier workspace root discovery traversing ancestors, inspecting `.cvh` / `.git`, and persisting active projects to `%LOCALAPPDATA%/com.voimax.cvh/active_project.txt`.
- **`scaffold.rs`**: Initial project initialization (`ensure_cvh_scaffold`, `ensure_cvh_dev_scaffold`). Enforces auto-gitignore entries (`.cvh/llm.json`, `.cvh/worktrees/`) and seeds initial 3-column boards.
- **`io.rs`**: Atomic disk operations. `atomic_write_file` writes via transient temp files and renames to prevent partial corruption.

### 2.3 Task Pipeline & Command Subsystem (`src-tauri/src/commands/`)
- **`mod.rs`**: IPC command handler facades and module re-exports.
- **`state.rs`**: Central application state container (`AppState`) managing database pool, workspace discovery root, and active session registries.
- **`tasks.rs`**: Top-level IPC command gateway routing commands (`list_tasks`, `create_task`, `advance_task`, `move_task_stage`, etc.) to modular pipelines.
- **`tasks/transitions.rs`**: Modular `TaskTransitionPipeline`. Executes stage movement, triggers `on_leave` and `on_enter` scripts, runs `gatekeeperSensor` commands in isolated worktrees, and transitions on verdicts (`APPROVED`, `APPROVED_WITH_CAVEATS`, `CHANGES_REQUESTED`).
- **`tasks/lifecycle.rs`**: Task creation and destruction commands. Enforces monotonic task numbers, parent-subtask waiting locks, and symmetric worktree teardown.
- **`tasks/runner.rs`**: Agent execution initiation (`run_agent`, `steer_agent`). Resolves active model hierarchy, prepares primed system messages, and tracks active agent runs.
- **`tasks/storage.rs`**: Task serialization, foreign task sanitization, and synchronization with disk ledgers and frontend subscribers via `tasks:updated`.
- **`tasks/healing.rs`**: Autonomous self-healing (`heal_task_board_and_column`, `heal_tasks`). Rescues orphan tasks assigned to non-existent boards or columns without failing.
- **`cortex.rs`**: File CRUD for personas, modular rules, and hooks.
- **`boards.rs`**: Multi-board CRUD (`.cvh/boards.json`), active board switching, and pipeline column management.
- **`dialogs.rs`**: Dialogue item retrieval, paginated history loading, and steer event storage.
- **`git.rs`**: Git worktree manager, branch listings, unified diff calculations, and working tree checks.
- **`llm.rs`**: LLM configuration management (`.cvh/llm.json`), live connectivity ping, and model catalogs.
- **`projects.rs`**: Project workspace root switching, recent projects history, and environment inspection.
- **`tools.rs`**: Two-level tool policy matrix queries and persistent updates (`.cvh/settings/tool-policy.json`).
- **`notifications.rs`**: Multi-tier desktop notifications (`show_system_notification`), native WinRT via `NotificationExt`, headless PowerShell toast fallback, and Start Menu AUMID shortcut registration (`ensure_windows_start_menu_shortcut`).

### 2.4 Domain Layer (`src-tauri/src/domain/`)
- **`mod.rs`**: Domain module root re-exporting `ids` and `state`.
- **`ids.rs`**: Strongly typed nominal branded types: `TaskId`, `StageId`, `BoardId`. Prevents primitive obsession and accidental string interchange.
- **`state.rs`**: `TaskState` algebraic data type (ADT) state machine (`Idle`, `Running`, `WaitingSubtask`, `WaitingSubagent`, `Done`). Rejects illegal state jumps at compile and runtime.

### 2.5 Agent Execution Engine (`src-tauri/src/agent/` & `agent.rs`)
- **`agent.rs`**: Core agent ReAct loop (`run_agent_loop`), in-process subagents (`execute_subagent`), Turn 1 runtime contract priming (`.cvh/rules/runtime.md`), batch tool execution fail-fast logic (`should_abort_tool_batch`), and system prompt assembly (`compile_primed_system_prompt_with_tools`).
- **`react_engine.rs`**: Generic `ReActEngine<Driver>` implementing multi-turn reasoning loops, event streaming (`AgentEvent`), and tool invocation dispatch.
- **`session_registry.rs`**: Concurrency-safe registry of active and background child subagent sessions (`subagent_bg`), managing asynchronous channels and cancellation tokens.
- **`compactor.rs`**: `ToolCallSummaryExtractor` (CC=1) extracting head/tail summaries of large tool outputs without nested branching. Enforces 100% context capacity threshold.
- **`sanitizer.rs`**: Cleans malformed LLM responses, stripping unclosed thinking tags and repairing incomplete tool invocation markers.
- **`telemetry.rs`**: Non-decaying TPS calculation, TTFT split, and generation token metrics.
- **`text_match.rs`**: Resilient 5-tier fuzzy text matcher for robust search-and-replace tool operations.
- **`tool_schemas.rs`**: Tool metadata, parameter schemas, and dynamic rule enumeration for LLM tool calling.
- **`tools/`**: Domain-partitioned tool executors conforming to command pattern: `mod.rs`, `file_ops`, `command`, `pipeline`, `planning`, `rules`, `subagent` (`subagent`, `subagent_bg`, `collect_subagent`, `subagent_list`), `tasks` (`create_task`, `create_tasks`, `complete_task`, `list_tasks`, `get_task_history`), `web`.

### 2.6 Entrypoint, UDF, Logging & Error Subsystem
- **`main.rs`**: Application entrypoint initializing the Windows GUI subsystem, setting panic hooks, configuring crash telemetry, and launching the Tauri runtime.
- **`lib.rs`**: Core library entrypoint registering Tauri plugins, state management, and wiring IPC handlers via `tauri::generate_handler!`.
- **`llm/`**: `mod.rs`, `client.rs` (OpenAI / Gemini streaming), `models.rs` (LLM configuration).
- **`udf.rs`**: Custom SQLite User-Defined Functions (e.g., regex matching and fuzzy text search) registered into SQLite connections upon pool initialization.
- **`logger.rs`**: Desktop logging configuration using structured rotation into `.cvh/logs/` and stdout streaming with configurable log levels.
- **`error.rs`**: Centralized domain error hierarchy (`CvhError`) mapping internal errors into serialized Tauri IPC error responses.

### 2.7 Backend Test Shield (277 Automated Tests)
Comprehensive test suites covering domain types, SQLite operations, config parsers, agent loops, and command dispatch across 277 tests (150 sync + 127 async tokio), including 66 inline unit tests:
- `domain/tests.rs`, `domain/state_tests.rs` (nominal types, state machine transitions).
- `db/tests.rs` (pool, task repository, dialog repository, telemetry aggregation).
- `config/tests.rs` (models, discovery, auto-scaffold, tool policy matrix).
- `agent/tests/*` (`compactor_tests.rs`, `engine_tests.rs`, `session_tests.rs`, `telemetry_tests.rs`, `tools_cmd_tests.rs`, `tools_fs_tests.rs`, `tools_pipeline_tests.rs`, `tools_policy_tests.rs`).
- `commands/tests/*` (`boards_tests.rs`, `cortex_tests.rs`, `dialogs_tests.rs`, `git_tests.rs`, `tasks_id_tests.rs`, `tasks_lifecycle_tests.rs`, `tasks_runner_tests.rs`, `tasks_subtasks_tests.rs`).

---

## 3. Subsystem Ceilings & Architectural Invariants

1. **500-Line Subsystem Ceiling**: Every Rust source file must adhere to Single Responsibility Principle (SRP) and remain under **500 lines** (excluding comprehensive test modules). Any file expanding beyond 500 lines must be decomposed into submodules.
2. **Zero Naked Unwrap**: Never call `.unwrap()` or `.expect()` in production paths. Propagate typed errors with `Result<T, anyhow::Error>` or `thiserror`.
3. **Nominal Branded Types**: Always accept and pass `TaskId`, `StageId`, and `BoardId` across domain boundaries. Never use naked strings for entity keys.
4. **Isolated Worktrees**: All file edits and compiler checks execute inside task worktrees (`worktrees/<slug>`). Never modify the root working directory during task execution.
5. **Windows Process Hygiene**: Shell commands and hook scripts must execute with process tree isolation and explicit timeouts; on Windows, use `taskkill /F /T /PID` on teardown.
6. **Architectural Map Synchronization Mandate**: Whenever adding, removing, renaming, or relocating files in `src-tauri/src/` or `src/`, this navigation map MUST be updated in the exact same commit. The Level 2 Reviewer rejects diffs with `CHANGES_REQUESTED` if files were altered without map synchronization.
7. **Direct Coordinate Navigation**: Jump directly to mapped files with `read_file` when coordinates or paths are known. Targeted `grep` for specific symbols (function names, types, commands) remains a natural, valid tool.

---

## 4. Accelerated Sensor Commands (`mbx`)

For rapid Rust compile checks and test execution in CanbanVoimaxHarness, use the accelerated build tool `mbx`:
- **Accelerated Check**: `mbx check --manifest-path src-tauri/Cargo.toml`
- **Accelerated Test**: `mbx test --manifest-path src-tauri/Cargo.toml`

### Invariants:
- Configured via `.mbx.toml` in repository root and uses `.toolchain/mbx-cache`.
- **Fallback Guarantee**: If `mbx` is absent or not installed in PATH, fall back seamlessly to standard `cargo check --manifest-path src-tauri/Cargo.toml` and `cargo test --manifest-path src-tauri/Cargo.toml`.

