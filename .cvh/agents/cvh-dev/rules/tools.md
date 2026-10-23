# Guide: Authoritative Tools Catalog and Execution Rules (tools.md)

Authoritative guide for agent tools in CanbanVoimaxHarness.

## 1. Role Scope & Operational Boundary

### 1.1 Strict Role Scope: cvh-dev vs. Developer vs. Reviewer vs. Workflow Orchestrator
- **Task Card Creation & Completion Invariant**:
  - `create_task`, `create_tasks`, and `complete_task` are strictly reserved for **Workflow Orchestrators** (Root Orchestrator / Level 0), human users via the UI, and the **Sidecar Copilot** (`copilot:meta-chat`).
  - **cvh-dev, Developer, and Reviewer DO NOT CALL `create_task`, `create_tasks`, OR `complete_task` directly**.
  - Developer advances work to Review via `advance_task(summary)`.
  - Reviewer approves and advances to Ready via `advance_task(summary)` or rejects back to Dev via `reject_task(reason)`.

---

## 2. Authoritative Standard Tool Catalog

All standard tools are defined in Rust backend (`src-tauri/src/agent.rs`) and exposed to ReAct agent loops subject to the two-level tool policy matrix.

### 2.1 Pipeline Flow & Stage Navigation
- **`advance_task` / `move_task`**: Transitions current task to the next stage or a specific target stage.
- **`reject_task`**: Returns task to previous stage with specific rejection findings.

### 2.2 Subagent Delegation Hierarchy
- **`subagent` (Sync Oneshot)**: Blocks until completion and returns final report directly in-turn.
- **`subagent_bg` (Async Background)**: Launches subagent in background asynchronously.
- **`collect_subagent`**: Awaits background subagent completion without polling.
- **`subagent_list`**: Lists status of subagents for the task.

### 2.3 Knowledge & Dynamic Rules
- **`load_rule`**: Dynamically loads a role-specific project standard by topic.
- **`list_tasks`**: Retrieves compact registry of recent tasks from SQLite WAL.
- **`get_task_history`**: Retrieves execution chronology of a task for audits and retrospectives.

### 2.4 Human-in-the-Loop & Planning
- **`question` / `ask_user`**: Prompts user with interactive question and suspends execution.
- **`todo_write`**: Records and updates task checklist plan.

### 2.5 File System & Terminal Operations
- `read_file`, `write_file`, `edit_file`, `glob`, `grep`, `execute_command`.
