---
description: Lifecycle hooks, transition scripts, environment variable bindings, and gatekeeper sensor execution rules.
---
# Guide: Lifecycle Hooks and Transition Scripts (hooks.md)

Authoritative rules for `.cvh/hooks/` and column lifecycle scripts.

---

## 1. Hook Configuration Schema & Invocation Triggers
Columns declare transition hooks in configuration (`boards.json` and `pipeline.json`) within the `hooks` object:
- `hooks.on_enter`: Executes when a task enters the stage.
- `hooks.on_leave`: Executes when a task exits the stage.

### Hook Declaration Formats
Hooks support two schema formats:
1. **Object format (Recommended)** with `script` and `description`:
   ```json
   "hooks": {
     "on_leave": {
       "script": ".cvh/hooks/on_review_finish.bat",
       "description": "Сливает утвержденную feature-ветку в main, отправляет в origin/main и удаляет изолированный worktree"
     }
   }
   ```
   - `script` (aliases: `path`, `command`): Relative path to executable script (`.bat`, `.cmd`, `.ps1`, `.sh`, `.py`).
   - `description`: Informative human-readable explanation displayed when hovering over the Kanban card hook indicator node, clarifying hook purpose and operations to users and agents.
2. **String scalar format (Compact / Legacy)**:
   ```json
   "hooks": {
     "on_leave": ".cvh/hooks/create_worktree.bat"
   }
   ```

All hooks are executable scripts located in `.cvh/hooks/` or inside an agent's directory (`.cvh/agents/<role>/hooks/`).

---

## 2. Standard Output Protocol for Worktrees & Git Branches
When a hook script prepares or switches a git worktree or branch, it outputs the following key-value pairs to `stdout`:
```text
CVH_WORKTREE=<worktree_relative_or_absolute_path>
CVH_BRANCH=<git_branch_name>
```
The CVH runtime intercepts these lines from hook stdout and writes them directly to the task card's `worktree_path` and `branch` fields in SQLite.

---

## 3. Injected Task Context Environment Variables
Every hook execution receives the task's context via system environment variables:
- `CVH_TASK_ID`: Unique task identifier (e.g. `CVH-042`).
- `CVH_TASK_TITLE`: Title of the task card.
- `CVH_WORKTREE_PATH`: Absolute or relative path to the task's worktree.
- `CVH_BRANCH`: Target git feature branch.
- `CVH_STAGE_ID`: Target stage identifier.
- `CVH_VERDICT`: Verdict emitted during review exit (`APPROVED`, `APPROVED_WITH_CAVEATS`, `CHANGES_REQUESTED`).

---

## 4. Gatekeeper Sensors
Columns may define `gatekeeperSensor` (e.g. `cargo check` or `npm run build`).
Gatekeeper sensors execute inside `CVH_WORKTREE_PATH` before any card advance. If exit code != 0, transition is blocked.

---

## 5. Hook Execution on `advance_task` & Subtask Handover Protocol
During autonomous transitions via `advance_task`:
1. **Hook Sequence Enforcement**:
   - `on_leave` / `on_exit` hook of current stage is executed.
   - `gatekeeperSensor` of target stage is executed.
   - `on_enter` hook of target stage is executed.
   - If any hook/sensor fails, the transition is blocked immediately: NO new subtasks are created, verdict is set to `CHANGES_REQUESTED`, status is kept `running` (if column has persona/autorun), and error details are injected into task dialog and autorun prompt.
2. **Generic Rejection Handover (Zero Card Duplication)**:
   - When a review subtask rejects via `reject_task(reason)`, the reviewing subtask stays in its column with `idle` status and `CHANGES_REQUESTED`, while the developer subtask in `dev` is reactivated with `running` status and rejection details.
   - When the developer calls `advance_task(summary)`, CVH identifies the existing subtask in the target column, marks developer subtask `done`, reactivates the target subtask with `running` status and `handover_prompt`, without generating duplicate subtask cards (`CVH-X.3`).
