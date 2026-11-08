---
description: Task lifecycle, runner, stage transitions and Git.
---
# Task lifecycle and workspace operations

Commands coordinate task state, stage hooks and agent sessions. `runner.rs` starts, aborts and steers runs; `transitions.rs` owns stage movement. Domain IDs and transition types are in `backend_storage_map`. Child processes require timeout and cancellation handling, including Windows process-tree cleanup.

| File | Responsibility |
| --- | --- |
| `src-tauri/src/commands/state.rs` | `AppState`, connection pool, workspace root, active session registry. |
| `src-tauri/src/commands/tasks.rs` | IPC invoke gateway routing for tasks and agent lifecycle. |
| `src-tauri/src/commands/tasks/transitions.rs` | Stage movement and lifecycle execution using `TaskTransitionPipeline`. |
| `src-tauri/src/commands/tasks/transition_hooks.rs` | `TaskTransitionPipeline` definition (line 274), hooks and transition execution. |
| `src-tauri/src/commands/tasks/lifecycle.rs` | Backend-authoritative card lifecycle, auto-unlock; `list_tasks` (line 25). |
| `src-tauri/src/commands/tasks/runner.rs` | Model/persona resolution, run, abort, steer and manual compaction. |
| `src-tauri/src/commands/tasks/storage.rs` | Foreign task filtration. |
| `src-tauri/src/commands/tasks/healing.rs` | Orphan board/column resolution on load. |
| `src-tauri/src/commands/boards.rs` | Multi-board CRUD, column settings, pipeline sync. |
| `src-tauri/src/commands/dialogs.rs` | Dialogue history, steer persistence and stopped-dialog item deletion. |
| `src-tauri/src/commands/git.rs` | Worktree lifecycle, branch diffs, commit queries. |
| `src-tauri/src/commands/projects.rs` | Workspace root switching, recent projects list. |
| `src-tauri/src/git/mod.rs` | Worktree isolation, prune, branch teardown. |
| `src-tauri/src/git/cvh_sync.rs` | Live `.cvh/` sync to worktrees, secret exclusion, assume-unchanged. |
| `src-tauri/src/hooks.rs` | Transition scripts, process tree kill on Windows. |

## Related checks

Choose checks for the changed behavior; use the active role's `sensors` rule for execution policy.

- `src-tauri/src/commands/tests/boards_tests.rs` — boards tests.
- `src-tauri/src/commands/tests/dialogs_tests.rs` — dialogs tests.
- `src-tauri/src/commands/tests/git_tests.rs` — git tests.
- `src-tauri/src/commands/tests/tasks_id_tests.rs` — tasks id tests.
- `src-tauri/src/commands/tests/tasks_lifecycle_tests.rs` — tasks lifecycle tests.
- `src-tauri/src/commands/tests/tasks_runner_tests.rs` — tasks runner tests.
- `src-tauri/src/commands/tests/tasks_subtasks_tests.rs` — tasks subtasks tests.
