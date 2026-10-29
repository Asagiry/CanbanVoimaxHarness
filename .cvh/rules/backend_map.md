---
description: Authoritative navigation map and file-by-file router for the Rust Tauri backend (src-tauri/src/).
---
# Backend Navigation Map & File-by-File Router (backend_map.md)

Authoritative directory topology, file mapping table, and subsystem architecture for the Rust 2021 / Tauri v2 backend of **CanbanVoimaxHarness (`src-tauri/src/`)**.

---

## 1. Quick File-by-File Router ("What to modify" -> "Exact file path")

| What you need to build / modify | Exact File Path | Key Invariants & Ceilings |
| :--- | :--- | :--- |
| **Domain Nominal Types & IDs** | `src-tauri/src/domain/ids.rs` | `TaskId`, `StageId`, `BoardId`. Zero raw strings. |
| **Task State Machine & Transitions** | `src-tauri/src/domain/state.rs` | `TaskState` enum. Valid transitions only. |
| **Database Pool & Migrations** | `src-tauri/src/db/pool.rs` | R2D2 pool, WAL mode, foreign keys, 5s timeout. |
| **Task SQLite CRUD & Monotonic IDs** | `src-tauri/src/db/task_repo.rs` | Monotonic ID allocation, subtask cascade. |
| **Dialogue Items & Streaming Chunks** | `src-tauri/src/db/dialog_repo.rs` | Atomic upsert, tool JSON persistence. |
| **Token Usage & TPS Telemetry Repo** | `src-tauri/src/db/telemetry_repo.rs` | Aggregated tokens, non-decaying TPS metrics. |
| **Pipeline & Board Data Models** | `src-tauri/src/config/models.rs` | `BoardDef`, `PipelineConfig`, embedded rules. |
| **Two-Level Tool Policy Matrix** | `src-tauri/src/config/tool_policy.rs` | Strict AND-semantics: global && stage override. |
| **Workspace Root Discovery** | `src-tauri/src/config/discovery.rs` | Multi-tier root discovery, `%LOCALAPPDATA%`. |
| **Scaffolding & Auto-Gitignore** | `src-tauri/src/config/scaffold.rs` | Automatic scaffold, auto-gitignore policies. |
| **Atomic File I/O** | `src-tauri/src/config/io.rs` | `atomic_write_file`, JSON load/save. |
| **Task Stage Transition Pipeline** | `src-tauri/src/commands/tasks/transitions.rs` | `TaskTransitionPipeline`, hooks, gatekeeper sensors. |
| **Task Lifecycle (Create, Delete, Subtask)**| `src-tauri/src/commands/tasks/lifecycle.rs` | Backend-authoritative card lifecycle, auto-unlock. |
| **Agent Runner Trigger & Steer** | `src-tauri/src/commands/tasks/runner.rs` | `run_agent`, `steer_agent`, active task registry. |
| **Task Serialization & Storage** | `src-tauri/src/commands/tasks/storage.rs` | `load_tasks`, `sync_tasks_to_disk`. |
| **Board & Column Self-Healing** | `src-tauri/src/commands/tasks/healing.rs` | Orphan board/column resolution on load. |
| **Generic ReAct Engine Loop** | `src-tauri/src/agent/react_engine.rs` | `ReActEngine<Driver>`, unified streaming. |
| **State-Preserving Context Compactor**| `src-tauri/src/agent/compactor.rs` | `ToolCallSummaryExtractor` (CC=1), 100% ceiling. |
| **Unclosed Tool Call Sanitizer** | `src-tauri/src/agent/sanitizer.rs` | Tag stripping, unclosed JSON repairs. |
| **Non-Decaying TPS Telemetry** | `src-tauri/src/agent/telemetry.rs` | Frozen TPS on generation stop, TTFT split. |
| **5-Tier Fuzzy Text Matcher** | `src-tauri/src/agent/text_match.rs` | CRLF/LF adaptation, whitespace relaxation. |
| **LLM Tool Schemas & Definitions** | `src-tauri/src/agent/tool_schemas.rs` | JSON schemas, dynamic rule enumeration. |
| **File Operations Tools** | `src-tauri/src/agent/tools/file_ops.rs` | `read_file`, `write_file`, `edit_file`, `glob`, `grep`. |
| **Command Execution Tool** | `src-tauri/src/agent/tools/command.rs` | `execute_command` (alias `bash`), timeout, stdio. |
| **Pipeline Navigation Tools** | `src-tauri/src/agent/tools/pipeline.rs` | `advance_task`, `reject_task`, relay handoff. |
| **Planning & Checklist Tools** | `src-tauri/src/agent/tools/planning.rs` | `todo_write`, `question` (ask_user). |
| **Modular Rule Loader Tool** | `src-tauri/src/agent/tools/rules.rs` | `load_rule` (JIT dynamic rule loader). |
| **Subagent Hierarchy Tools** | `src-tauri/src/agent/tools/subagent.rs` | `subagent`, `subagent_bg`, `collect_subagent`. |
| **Task Card Tools (Level 0 Only)** | `src-tauri/src/agent/tools/tasks.rs` | `create_task`, `create_tasks`, `complete_task`. |
| **Web Research Tools** | `src-tauri/src/agent/tools/web.rs` | `web_search`, `web_fetch`. |
| **Git Worktree Manager** | `src-tauri/src/git.rs` | Worktree isolation, prune, branch teardown. |
| **Lifecycle Hooks Runner** | `src-tauri/src/hooks.rs` | Transition scripts, process tree kill on Windows. |
| **Universal LLM Client & SSE Stream** | `src-tauri/src/llm/client.rs` | OpenAI / Gemini streaming, thought extraction. |
| **LLM Configuration & Models** | `src-tauri/src/llm/models.rs` | Model overrides, endpoints, temperature. |
| **Tauri IPC Command Registration** | `src-tauri/src/lib.rs` | Plugin builder, invoke handlers, AppState. |

---

## 2. Deep Subsystem Architectures

### 2.1 Persistence Subsystem (`src-tauri/src/db/`)
- **`pool.rs`**: Implements thread-safe `r2d2::Pool<SqliteConnectionManager>` for `<repo_root>/.cvh/harness.db`. Enforces `PRAGMA journal_mode = WAL`, `PRAGMA synchronous = NORMAL`, `PRAGMA foreign_keys = ON`, and `busy_timeout = 5000ms`. Executes initial migrations for `tasks`, `dialog_items`, and `telemetry`.
- **`task_repo.rs`**: Handles task entity persistence. Manages parent-subtask cascades, monotonic ID allocation (`allocate_next_task_num`), and relational state consistency.
- **`dialog_repo.rs`**: High-concurrency dialogue storage. Atomically upserts thoughts, streaming chunks, tool execution records, and steer events into `dialog_items` table indexed by `(task_id, created_at)`.
- **`telemetry_repo.rs`**: Persists token usage, duration, and generation performance metrics. Provides aggregated stats for reporting.

### 2.2 Configuration & Scaffolding Subsystem (`src-tauri/src/config/`)
- **`models.rs`**: Canonical data contracts (`BoardDef`, `BoardsConfig`, `PipelineConfig`, `ColumnDef`, `AgentSettings`). Defines system directory resolvers (`agent_rules_dir`, `global_rules_dir`, `tool_policy_path`).
- **`tool_policy.rs`**: Two-level tool authorization engine. Enforces strict AND-semantics: `effective(tool, stage) = global_enabled[tool] && stage_override[stage][tool]`. Zero-trust stripping purges disabled tools from LLM schemas.
- **`discovery.rs`**: Four-tier workspace root discovery traversing ancestors, inspecting `.cvh` / `.git`, and persisting active projects to `%LOCALAPPDATA%/com.voimax.cvh/active_project.txt`.
- **`scaffold.rs`**: Initial project initialization (`ensure_cvh_scaffold`, `ensure_cvh_dev_scaffold`). Enforces auto-gitignore entries (`.cvh/llm.json`, `.cvh/worktrees/`) and seeds initial 3-column boards.
- **`io.rs`**: Atomic disk operations. `atomic_write_file` writes via transient temp files and renames to prevent partial corruption.

### 2.3 Task Pipeline & Command Subsystem (`src-tauri/src/commands/tasks/`)
- **`transitions.rs`**: Modular `TaskTransitionPipeline`. Executes stage movement, triggers `on_leave` and `on_enter` scripts, runs `gatekeeperSensor` commands in isolated worktrees, and routes on verdicts (`routes.on_approved`, `routes.on_changes_requested`).
- **`lifecycle.rs`**: Task creation and destruction commands. Enforces monotonic task numbers, parent-subtask waiting locks, and symmetric worktree teardown.
- **`runner.rs`**: Agent execution initiation (`run_agent`, `run_agent_loop`). Resolves active model hierarchy, prepares primed system messages, and tracks active agent runs.
- **`storage.rs`**: Task serialization and synchronization with disk ledgers and frontend subscribers via `tasks:updated`.
- **`healing.rs`**: Autonomous self-healing (`heal_task_board_and_column`, `heal_tasks`). Rescues orphan tasks assigned to non-existent boards or columns without failing.

### 2.4 Domain Layer (`src-tauri/src/domain/`)
- **`ids.rs`**: Strongly typed nominal branded types: `TaskId`, `StageId`, `BoardId`. Prevents primitive obsession and accidental string interchange.
- **`state.rs`**: `TaskState` state machine (`Backlog`, `InProgress`, `Review`, `Done`). Rejects illegal state jumps at compile and runtime.

### 2.5 Agent Execution Engine (`src-tauri/src/agent/`)
- **`react_engine.rs`**: Generic `ReActEngine<Driver>` implementing multi-turn reasoning loops, event streaming (`AgentEvent`), and tool invocation dispatch.
- **`compactor.rs`**: `ToolCallSummaryExtractor` (CC=1) extracting head/tail summaries of large tool outputs without nested branching. Enforces 100% context capacity threshold.
- **`sanitizer.rs`**: Cleans malformed LLM responses, stripping unclosed thinking tags and repairing incomplete tool invocation markers.
- **`tools/`**: Domain-partitioned tool executors conforming to command pattern: `file_ops`, `command`, `pipeline`, `planning`, `rules`, `subagent`, `tasks`, `web`.

---

## 3. Subsystem Ceilings & Architectural Invariants

1. **500-Line Subsystem Ceiling**: Every Rust source file must adhere to Single Responsibility Principle (SRP) and remain under **500 lines** (excluding comprehensive test modules). Any file expanding beyond 500 lines must be decomposed into submodules.
2. **Zero Naked Unwrap**: Never call `.unwrap()` or `.expect()` in production paths. Propagate typed errors with `Result<T, anyhow::Error>` or `thiserror`.
3. **Nominal Branded Types**: Always accept and pass `TaskId`, `StageId`, and `BoardId` across domain boundaries. Never use naked strings for entity keys.
4. **Isolated Worktrees**: All file edits and compiler checks execute inside task worktrees (`worktrees/<slug>`). Never modify the root working directory during task execution.
5. **Windows Process Hygiene**: Shell commands and hook scripts must execute with process tree isolation and explicit timeouts; on Windows, use `taskkill /F /T /PID` on teardown.
