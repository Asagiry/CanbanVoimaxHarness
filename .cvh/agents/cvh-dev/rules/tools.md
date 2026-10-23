# Guide: Authoritative Tools Catalog and Execution Rules

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
- **`advance_task` / `move_task`**:
  - Transitions the current task to the next stage in the pipeline or directly to a specific target stage. Emits `agent:autorun` on entry into the target stage.
  - Parameters: `stage_id` (optional), `summary` (optional).
- **`reject_task`**:
  - Returns the task to the previous pipeline stage with specific rejection findings.
  - Parameters: `reason` (required).

### 2.2 Subagent Delegation Hierarchy
- **`subagent` (Sync Oneshot)**:
  - Delegates a focused, independent sub-task to a specialized subagent synchronously. Blocks execution until completion and returns the final report directly in-turn.
- **`subagent_bg` (Async Background)**:
  - Launches a specialized subagent in the background asynchronously. Returns `{ "subagent_id": "...", "status": "running" }`.
- **`collect_subagent`**:
  - Awaits completion of one or multiple background subagents and retrieves their final reports without polling.
- **`subagent_list`**:
  - Lists status of all subagents for the task.

### 2.3 Knowledge, Modular Rules & Task Registry Tools
- **`load_rule`**:
  - Dynamically loads a modular project standard, architectural invariant, or tech guideline by topic (`backend`, `frontend`, `rules-sync`, `review-checklist`, `boards`, `agents`, `hooks`, `tools`).
- **`list_tasks`**:
  - Retrieves a compact registry of recent Kanban board tasks from SQLite WAL (`.cvh/harness.db`).
- **`get_task_history`**:
  - Retrieves execution chronology of a task for audits and retrospectives.

### 2.4 Human-in-the-Loop & Planning
- **`question` / `ask_user`**:
  - Prompts the user with an interactive clarifying question and suspends execution until answered.
- **`todo_write`**:
  - Records and updates the step-by-step task execution plan.

### 2.5 File System & Terminal Operations (Worktree-Scoped)
- **`read_file`**: Read UTF-8 text file with line numbers.
- **`write_file`**: Write complete UTF-8 content to file.
- **`edit_file`**: Targeted substring replacement.
- **`glob`**: Find files matching pattern within worktree.
- **`grep`**: Regular expression search across files in worktree.
- **`execute_command`**: Run shell command with piped stdout/stderr.

### 2.6 External Web & Research Tools
- **`web_search`**: DuckDuckGo web search.
- **`web_fetch`**: HTTP/HTTPS URL fetch returning cleaned Markdown text.

---

## 3. Tool Execution Policies & Runtime Invariants

### 3.1 Two-Level Tool Policy Matrix (Strict AND-Semantics)
- `effective(tool, stage) = globalEnabled[tool] && stageOverrides[stage][tool]`.
- Disabled tools are stripped from the tool schema sent to the LLM.

### 3.2 English-Only Schemas Invariant (BPE Token Savings)
- All tool names, descriptions, parameter names, and property descriptions in tool schemas sent to the LLM MUST be strictly authored in English (~70% token savings).

### 3.3 Large Tool Output Spilling Invariant
- Outputs exceeding 500 lines or 64 KB for shell commands, or 2,000 lines / 128 KB for `read_file`, are automatically written to `.cvh/spill/<timestamp>-<tool>.log`.
