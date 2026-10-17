# Guide: Authoritative Tools Catalog and Execution Rules

Authoritative guide for agent tools in CanbanVoimaxHarness.

## 1. Role Scope & Operational Boundary

### 1.1 Strict Role Scope: cvh-dev vs. Workflow Orchestrator
- **Task Card Creation & Completion Invariant**:
  - `create_task`, `create_tasks`, and `complete_task` are strictly reserved for **Workflow Orchestrators** (Root Orchestrator / Level 0), human users via the UI, and the **Sidecar Copilot** (`copilot:meta-chat`).
  - **cvh-dev IS STRICTLY FORBIDDEN FROM CALLING `create_task`, `create_tasks`, OR `complete_task`**.
  - `cvh-dev` is the **Harness Router & Infrastructure Architect**. Its responsibility is configuring, repairing, and initializing `.cvh/` infrastructure (boards, pipelines, personas, rules, hooks, tool-policies).
  - When `cvh-dev` finishes its assigned infrastructure work, it uses `move_task` / `advance_task` with a comprehensive `summary` to hand over to the next pipeline stage (e.g. `review`), or `reject_task` with a `reason` if audit checks fail. It does **not** create or complete operational task cards.

---

## 2. Authoritative Standard Tool Catalog

All standard tools are defined in Rust backend (`src-tauri/src/agent.rs`) and exposed to ReAct agent loops subject to the two-level tool policy matrix.

### 2.1 Pipeline Flow & Stage Navigation
- **`advance_task` / `move_task`**:
  - **Description**: Transitions the current task to the next stage in the pipeline (sequential forward step) or moves it directly to a specific target stage. Emits `agent:autorun` on entry into the target stage.
  - **Parameters**:
    - `stage_id` / `target_stage` / `column_id` (string, optional): Target pipeline stage (e.g., `"review"`, `"retro"`, `"ready"`). If omitted, advances to the next sequential column.
    - `summary` (string, optional): Comprehensive handover summary of changes, sensor verification results, and operational context. Recorded into `task.handover_summary` and injected into the next stage's agent prompt via Relay Handoff.
  - **Aliases**: `move_task`, `move_task_stage`, `advance_stage`, `next_stage`.
- **`reject_task`**:
  - **Description**: Returns the task to the previous pipeline stage with specific rejection findings (step backward). Unlocks the task card and resets status to idle.
  - **Parameters**:
    - `reason` (string, required): Concrete reason for rejection (failed sensor checks, compiler errors, architectural violations).

### 2.2 Subagent Delegation Hierarchy
- **`subagent` (Sync Oneshot)**:
  - **Description**: Delegates a focused, independent sub-task to a specialized subagent synchronously. Blocks execution until completion and returns the final report directly in-turn. Subagents run in-process inside the current working directory without moving Kanban cards or creating separate worktrees.
  - **Parameters**:
    - `title` (string, required): Short task description.
    - `prompt` (string, required): Detailed task instructions.
    - `role` (string, optional): Subagent role persona (e.g., `"general"`, `"developer"`, `"reviewer"`, `"explorer"`). Default: `"general"`.
    - `tools` (string[], optional): Filter list of tools allowed for this subagent. If omitted, inherits all tools available to the caller.
- **`subagent_bg` (Async Background)**:
  - **Description**: Launches a specialized subagent in the background asynchronously. Returns immediately with `{ "subagent_id": "...", "status": "running" }`.
  - **Parameters**: Same as `subagent` (`title`, `prompt`, `role`, `tools`).
- **`collect_subagent`**:
  - **Description**: Awaits the completion of one or multiple background subagents (launched via `subagent_bg`) and retrieves their final reports without busy-polling loops.
  - **Parameters**:
    - `subagent_id` (string, optional): Single subagent ID to await.
    - `subagent_ids` (string[], optional): Array of subagent IDs to await.

### 2.3 Human-in-the-Loop & Planning
- **`question` / `ask_user`**:
  - **Description**: Prompts the user with an interactive clarifying question and suspends agent execution until the user selects options or types an answer.
  - **Parameters**:
    - `text` (string, required): The specific question.
    - `options` (array of `{ label: string, description?: string }`, optional): Predefined choices. Recommend best choice with `"(Рекомендуется)"` or `"(Recommended)"`.
    - `multiple` (boolean, optional): Allow multi-selection.
- **`todo_write`**:
  - **Description**: Records and updates the tactical step-by-step task execution plan to track progress visibly.
  - **Parameters**:
    - `todos` (array of `{ content: string, status: "pending" | "in_progress" | "completed" }`, required): Complete list replacing previous list.

### 2.4 File System & Terminal Operations (Worktree-Scoped)
All file operations execute strictly within the active isolated worktree directory:
- **`read_file`**: Read UTF-8 text file with 1-based line numbers.
  - Parameters: `file_path` (or `path`, required), `offset` (default: 1), `limit` (default: 2000).
- **`write_file`**: Write complete UTF-8 content to a file.
  - Parameters: `file_path` (or `path`, required), `content` (required).
- **`edit_file`**: Targeted substring replacement. Aliased as `str_replace`.
  - Parameters: `file_path` (required), `old_string` (or `old_str`, required), `new_string` (or `new_str`, required), `replace_all` (boolean, default: false).
- **`glob`**: Find files matching pattern within worktree.
  - Parameters: `pattern` (required), `path` (default: `"."`).
- **`grep`**: Regular expression search across files in worktree.
  - Parameters: `pattern` (required), `path` (default: `"."`).
- **`execute_command`**: Run shell command with piped stdout/stderr. Aliased as `bash`.
  - Parameters: `command` (or `cmd`, required), `timeout_seconds` (default: 60).

### 2.5 External Web & Research Tools
- **`web_search`**: Web search via DuckDuckGo for documentation, APIs, and error troubleshooting.
  - Parameters: `query` (required), `limit` (default: 5).
- **`web_fetch`**: HTTP/HTTPS URL fetch returning cleaned Markdown text.
  - Parameters: `url` (required).

---

## 3. Orchestrator-Only & Copilot-Only Tools (Forbidden for cvh-dev)

The following tools exist in the engine but are strictly forbidden for `cvh-dev`:
- **`create_task`**: Decomposes requirements and creates a new card on the Kanban board.
- **`create_tasks`**: Batch decomposes and creates multiple cards on the Kanban board.
- **`complete_task`**: Marks a task as complete and advances it directly to `ready`/`done`.

---

## 4. Tool Execution Policies & Runtime Invariants

### 4.1 Two-Level Tool Policy Matrix (Strict AND-Semantics)
- Tool availability is governed by:
  `effective(tool, stage) = globalEnabled[tool] && stageOverrides[stage][tool]`
- If a tool is globally disabled in `.cvh/settings/tool-policy.json`, no stage can re-enable it.
- **Zero-Trust Request Stripping**: Any disabled tool is completely stripped from the tool schema sent to the LLM, preventing tool hallucination and saving context tokens.

### 4.2 Large Tool Output Spilling Invariant
- Outputs exceeding 100 lines or 8 KB are automatically written to:
  `.cvh/spill/<timestamp>-<tool>.log`
- The model receives only the head (20 lines) and tail (20 lines) along with the spill file path.
