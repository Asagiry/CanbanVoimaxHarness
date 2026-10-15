# Guide: Transition Scripts and Lifecycle Hooks

Authoritative rules for `.cvh/hooks/` and column lifecycle scripts.

## 1. Lifecycle Triggers
Columns declare transition hooks in their column configuration:
- `hooks.on_enter`: Triggered when a card enters the stage.
- `hooks.on_finish`: Triggered when an agent finishes execution in the stage.
- Built-in hook: `"builtin:create_worktree"` automatically provisions an isolated git worktree upon entering backlog.

## 2. Supported Formats
Hook runner supports `.bat`, `.cmd`, `.ps1`, `.py`, `.sh`, and native binaries.

## 3. Injected Environment Variables
All hook processes receive the full task execution context:
- `CVH_TASK_ID`: Unique task identifier (e.g. `CVH-042`).
- `CVH_TASK_TITLE`: Human-readable task title.
- `CVH_WORKTREE_PATH`: Absolute path to the isolated worktree directory.
- `CVH_BRANCH`: Feature branch name (`feat/<task-slug>`).
- `CVH_STAGE_ID`: Target stage identifier.
- `CVH_VERDICT`: Reviewer verdict when leaving review (`APPROVED`, `APPROVED_WITH_CAVEATS`, `CHANGES_REQUESTED`).

## 4. Gatekeeper Sensors
Columns may define `gatekeeperSensor` (e.g. `cargo check` or `pnpm run build`).
Gatekeeper sensors execute inside `CVH_WORKTREE_PATH` before any card advance. If exit code != 0, transition is blocked.
