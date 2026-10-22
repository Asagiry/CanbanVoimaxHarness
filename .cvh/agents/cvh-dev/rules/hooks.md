# Guide: Transition Scripts and Lifecycle Hooks

Authoritative rules for `.cvh/hooks/` and column lifecycle scripts.

## 1. Lifecycle Triggers & Worktree Governance
Columns declare transition hooks in their column configuration:
- `hooks.on_enter`: Triggered when a card enters the stage.
- `hooks.on_finish`: Triggered when an agent finishes execution in the stage.
- **Hook-Driven Worktree Provisioning**:
  - Built-in hook: `"builtin:create_worktree"` automatically provisions an isolated git worktree at `worktrees/<task-slug>` upon entering `backlog` (or another designated intake column) and checks out a feature branch.
  - Worktree creation is managed exclusively by stage lifecycle hooks, rather than hardcoded side-effects.
- **"Отключить хук бэклога" (Skip Backlog Hook Checkbox)**:
  - In the Task Creation dialog, the checkbox **"Отключить хук бэклога"** (`skipWorktree: true` / `skip_worktree: true`) instructs the backend to bypass execution of the backlog's transition hook (`builtin:create_worktree`).
  - When checked, the task card is placed in backlog without creating an isolated worktree directory or git feature branch, allowing lightweight administrative tasks to execute directly in the repository root or defer worktree setup.

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
