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

### Worktree Creation Script Example (`.cvh/hooks/create_worktree.bat`):
```bat
@echo off
set "WT_DIR=worktrees\%CVH_TASK_ID%"
set "BRANCH=feat/%CVH_TASK_ID%"
if not exist "%WT_DIR%" (
    git worktree add -b "%BRANCH%" "%WT_DIR%" HEAD || exit /b 1
)
echo CVH_WORKTREE=%WT_DIR%
echo CVH_BRANCH=%BRANCH%
exit /b 0
```

### Worktree Creation Script Example (`.cvh/hooks/create_worktree.sh`):
```bash
#!/usr/bin/env bash
set -e
WT_DIR="worktrees/${CVH_TASK_ID}"
BRANCH="feat/${CVH_TASK_ID}"
if [ ! -d "$WT_DIR" ]; then
    git worktree add -b "$BRANCH" "$WT_DIR" HEAD
fi
echo "CVH_WORKTREE=$WT_DIR"
echo "CVH_BRANCH=$BRANCH"
exit 0
```

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

## 4. Hook Rejection Mechanism
When a hook script returns a non-zero exit code (`exit code != 0`):
1. The transition is aborted immediately; the task remains pinned in the source stage.
2. The error log (`stderr` or `stdout`) is recorded into SQLite task dialog history as a steer message (`### [HOOK REJECTED: ...]`).
3. The task status resets to `idle`, and its verdict is set to `CHANGES_REQUESTED`.
4. If the column configures an agent (`persona`) or has `autoRun: true`, the runtime triggers an `agent:autorun` event directing the agent to fix the defect.

---

## 5. Gatekeeper Sensor Checks
A column may declare a `gatekeeperSensor` command (e.g. `.cvh/hooks/gatekeeper_sensor.bat` or `cargo check`).
- The sensor executes strictly inside `CVH_WORKTREE_PATH` before moving a card into the target stage.
- If the sensor exits non-zero, the transition is blocked.
- Sensor execution is automatically skipped on initial moves from `backlog` into the first working stage, or when the worktree is empty.

---

## 6. Terminal Stage Invariants
- The `ready` stage (or any column with `terminal: true`) represents the delivery endpoint.
- Only an `on_enter` hook is permitted (e.g. verifying remote push, cleaning artifacts).
- An `on_leave` hook on a terminal stage is strictly forbidden and rejected by configuration normalizers.
