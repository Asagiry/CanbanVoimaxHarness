---
description: Turn 1 runtime contract, batch tool execution & fail-fast mechanics, Global Rules vs Role Rules, Cortex Explorer, and CVH UI/UX architecture.
---
# Runtime Execution Contract, Cortex Explorer & UI/UX Architecture

## 1. Runtime Environment Contract (`.cvh/rules/runtime.md`)
- **Turn 1 Priming**: The harness reads `.cvh/rules/runtime.md` from the project repository root and automatically inlines it (with YAML frontmatter stripped) as Section 1a into the Turn 1 System Prompt of all agents across all columns/roles. If absent or unreadable, the backend falls back to `DEFAULT_RUNTIME_MANDATE`.
- **Exclusion from `load_rule` Suggestions**: Because `runtime.md` is already unconditionally primed into Turn 1, `get_available_rules_with_descriptions` explicitly filters out `runtime`. Agents do not need to call `load_rule("runtime")`.
- **Cortex Visibility**: Because it is stored in `.cvh/rules/`, it is fully visible and editable in the Cortex Explorer tab under "Общие правила" (Global Rules), enabling operators to tune batch mandates and global instructions on the fly.

## 2. Batch Tool Execution & Fail-Fast Semantics
- **Batch Tool Mandate**: When actions are predictable (e.g. multi-file edits, or edits followed by build/lint/test verification via `execute_command`), agents should emit multiple tool calls in a single turn. The backend executes them sequentially in order.
- **Fail-Fast Protection**:
  - In both `run_agent_loop` and `execute_subagent`, the harness monitors tool execution in each turn.
  - If a mutation tool (`edit_file`, `write_file`, `create_task`, `advance_task`, etc.) fails (`Err` / `success: false`), OR if `execute_command` fails (`Err` or non-zero `[Exit Code: N]`), the loop sets `batch_aborted_by_error`.
  - For all remaining tool calls in that turn, execution is immediately bypassed (`Skipped: previous tool '<name>' in this batch failed`) and recorded as `success: false`.
  - This fail-fast mechanism prevents cascading corruptions (such as compiling or advancing a task when an edit failed, or making edits predicated on a failed prior edit).

## 3. Global Rules vs Per-Role Modular Rules
- **Global Rules (`.cvh/rules/*.md`)**:
  - Stored directly in `.cvh/rules/`.
  - Visible in Cortex Explorer under "Общие правила" (Global Rules).
  - Common project maps and invariants: `backend_map.md`, `frontend_map.md`, `anti-sycophancy.md`, etc.
  - Dynamically loadable on demand via `load_rule("<stem>")` by any agent (except `runtime.md` which is primed at Turn 1).
- **Per-Role Modular Rules (`.cvh/agents/<role>/rules/*.md`)**:
  - Stored inside each agent's persona directory.
  - Specific to that agent's role responsibilities (e.g., `role-developer.md`, `boards.md`, `hooks.md`).
  - Loaded dynamically on demand via `load_rule("<stem>")` when the agent is acting in that role.

## 4. CVH UI/UX Architecture
- **Cortex Explorer**:
  - Four central tabs: Columns (Stage configuration & personas), Personas (Agent profiles & settings), Global Rules (`.cvh/rules/` markdown editor), and Harness Settings (`tool-policy.json`, LLM credentials, and board configs).
  - Allows live inspection and editing of agent prompts, global rules, and column gatekeepers without restarting the app.
- **Kanban Board**:
  - Visual stage columns matching `activeBoardId` in `.cvh/boards.json` (mirrored in `.cvh/pipeline.json`).
  - Drag-and-drop or programmatic task movement via `advance_task`, `move_task`, or `reject_task`.
  - Column enter hooks trigger agent execution (`run_agent(taskId)`).
- **Task Drawer & Live Activity Stream**:
  - Clicking any task card opens the Task Drawer.
  - Displays live streaming ReAct reasoning thoughts, tool execution cards with parameters, outputs, durations, error banners, and final response summaries.
  - Emits real-time Tauri IPC events (`agent:activity`, `subagent:activity`, `task:status`).
- **Tool Policy Matrix (`.cvh/settings/tool-policy.json`)**:
  - Configures global tool enablement and per-column/per-role overrides (`stageOverrides`).
  - Backend authoritative: agents cannot execute tools disabled by the matrix.
- **Worktree Isolation**:
  - Every engineering task executes in an isolated git worktree (`worktrees/<task-slug>`).
  - Protects the root repository tree from uncommitted or intermediate breaking changes.
  - Gatekeeper sensors (`cargo check`, `tsc`, `oxlint`, `vitest`) must verify the worktree before advancing.
