# Guide: Lifecycle Hooks and Transition Scripts (hooks.md)

Authoritative rules for `.cvh/hooks/` and column lifecycle scripts.

---

## 1. Hook Invocation Triggers
Columns declare transition hooks in configuration (`boards.json` and `pipeline.json`) within the `hooks` object:
- `hooks.on_enter`: Executes when a task enters the stage.
- `hooks.on_leave`: Executes when a task exits the stage.

All hooks are executable scripts (`.bat`, `.cmd`, `.ps1`, `.sh`, `.py`) located in `.cvh/hooks/` or inside an agent's directory.

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
