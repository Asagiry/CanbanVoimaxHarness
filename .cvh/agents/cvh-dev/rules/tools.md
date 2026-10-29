# Guide: Authoritative Tools Catalog and Execution Rules (tools.md)

Authoritative guide for agent tools in CanbanVoimaxHarness.

## 1. Role Scope & Operational Boundary

### 1.1 Strict Role Scope: cvh-dev vs. Developer vs. Reviewer vs. Workflow Orchestrator
- **Task Card Creation & Completion Invariant (§ 2.8, § 2.16)**:
  - `create_task`, `create_tasks`, and `complete_task` are strictly reserved for **Workflow Orchestrators** (Root Orchestrator / Level 0) and human users via the UI.
  - **cvh-dev, Developer, and Reviewer DO NOT CALL `create_task`, `create_tasks`, OR `complete_task` directly**.
  - Developer advances work to Review via `advance_task(summary)`.
  - Reviewer approves and advances to Ready/Retro via `advance_task(summary)` or rejects back to Dev via `reject_task(reason)`.
  - cvh-dev completes configuration tasks and advances via `advance_task(summary)`.

---

## 2. Authoritative Standard Tool Catalog

All standard tools are defined in Rust backend (`src-tauri/src/agent.rs` and `src-tauri/src/agent/tools/`) and exposed to ReAct agent loops subject to the two-level tool policy matrix.

### 2.1 Pipeline Flow & Stage Navigation
- **`advance_task` / `move_task`**: Transitions current task to the next stage or a specific target stage. Enforces Relay Handoff context injection via `summary`.
- **`reject_task`**: Returns task to previous stage with specific rejection findings, line references, and failure reasons.

### 2.2 Subagent Delegation Hierarchy
- **`subagent` (Sync Oneshot)**: Blocks until completion and returns final report directly in-turn without card movement or worktree duplication.
- **`subagent_bg` (Async Background)**: Launches subagent in background asynchronously, returning `subagent_id` immediately.
- **`collect_subagent`**: Awaits background subagent completion without polling.
- **`subagent_list`**: Lists status of subagents for the task.

### 2.3 Knowledge & Dynamic Rules
- **`load_rule`**: Dynamically loads a role-specific project standard by topic (JIT memory priming).
- **`list_tasks`**: Retrieves compact registry of recent tasks from SQLite WAL.
- **`get_task_history`**: Retrieves execution chronology of a task for audits and retrospectives.

### 2.4 Human-in-the-Loop & Planning
- **`question` / `ask_user`**: Prompts user with interactive question and suspends execution until an answer is received.
- **`todo_write`**: Records and updates task checklist plan (`todos: [{content, status}]`).

### 2.5 File System & Code Inspection Operations
- `read_file`: Reads UTF-8 text file with line numbers (`file_path`, `offset`, `limit`). Subject to 2,000 lines / 128 KB spilling limit.
- `write_file`: Writes complete UTF-8 file content (`file_path`, `content`).
- `edit_file` (alias: `str_replace`): Precise text replacement with progressive line-ending normalization and whitespace tolerance.
- `glob`: Finds files by pattern within active worktree.
- `grep`: Searches file contents by regex within active worktree.

### 2.6 Shell Execution & Sensor Verification Tools
- **`execute_command` (alias: `bash`)**: Runs shell commands inside the worktree with timeout bounds and output spilling (>500 lines / 64 KB).
- **Standard Sensor Commands**:
  - **Vitest Unit Tests**: `pnpm test` (executes `vitest run` on frontend pure functions, domain types, and store slices).
  - **TypeScript Sensor**: `pnpm exec tsc --noEmit`
  - **Frontend Linter**: `pnpm exec oxlint --deny-warnings`
  - **Rust Domain Tests**: `cargo test --manifest-path src-tauri/Cargo.toml` (executes domain and backend unit tests).
  - **Rust Compiler Sensor**: `cargo check --manifest-path src-tauri/Cargo.toml`
  - **Rust Clippy Sensor**: `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`

### 2.7 Modular Architectural Boundaries in Tool Usage
- **Backend Subsystem Awareness**: Tool operations touching the backend must respect the modular domain boundaries:
  - `src-tauri/src/db/`: Database pool and repositories (`task_repo.rs`, `dialog_repo.rs`, `telemetry_repo.rs`).
  - `src-tauri/src/config/`: Configuration I/O, discovery, models, scaffolding, and tool policy.
  - `src-tauri/src/domain/`: Pure Value Objects (`domain::ids`) and ADT state machine (`domain::state`).
  - `src-tauri/src/agent/`: ReAct loop, context compaction, telemetry, sanitizers, and tools.
  - Monolithic limit: backend files must not exceed 500 lines.
- **Frontend Subsystem Awareness**: Tool operations touching the UI must respect event isolation and sizing invariants:
  - `src/services/tauriEventGateway.ts`: The sole authoritative gateway for Tauri backend events. Direct `listen<any>` is strictly forbidden.
  - React components must not exceed 350 lines; decompose into subcomponents or dedicated hooks.
  - Pure utilities and store slices require matching Vitest test files in `__tests__/`.