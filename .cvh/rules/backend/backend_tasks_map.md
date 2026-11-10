---
description: Task lifecycle, runner, stage transitions, Git operations, and hook engines.
---
# Task lifecycle and workspace operations

Commands coordinate task state, stage hooks and agent sessions. `runner.rs` starts, aborts and steers runs; `transitions.rs` owns stage movement. Domain IDs and transition types are in `backend_storage_map`. Child processes require timeout and cancellation handling, including Windows process-tree cleanup. All modules strictly observe the <=500 lines ceiling.

| File | Responsibility |
| --- | --- |
| `src-tauri/src/commands/state.rs` | `AppState`, connection pool, workspace root, active session registry. |
| `src-tauri/src/commands/tasks.rs` | IPC invoke gateway routing for tasks and agent lifecycle. |
| `src-tauri/src/commands/tasks/timing.rs` | Task duration calculation, execution timing, and stopwatch formatters. |
| `src-tauri/src/commands/tasks/transitions.rs` | Public facade re-exporting task transition pipeline and stage movement. |
| `src-tauri/src/commands/tasks/transitions/move_stage.rs` | Atomic stage movement, SQLite transaction boundary, and hook triggers. |
| `src-tauri/src/commands/tasks/transitions/advance.rs` | Automated task progression to the next stage upon completion. |
| `src-tauri/src/commands/tasks/transitions/advance_child.rs` | Child subtask progression and parent task wake-up logic. |
| `src-tauri/src/commands/tasks/transitions/resolve.rs` | Stage and column resolution across active board definitions. |
| `src-tauri/src/commands/tasks/transitions/subtasks.rs` | Subtask dependency verification and terminal completion cascades. |
| `src-tauri/src/commands/tasks/transitions/reorder.rs` | Drag-and-drop task reordering within columns. |
| `src-tauri/src/commands/tasks/transition_hooks.rs` | Public facade re-exporting `TaskTransitionPipeline` and hook runners. |
| `src-tauri/src/commands/tasks/transition_hooks/pipeline.rs` | `TaskTransitionPipeline` struct and execution stage orchestration. |
| `src-tauri/src/commands/tasks/transition_hooks/pipeline_enter.rs` | Pre-entry hooks and gatekeeper sensor execution logic. |
| `src-tauri/src/commands/tasks/transition_hooks/pipeline_leave.rs` | `on_leave`/`on_finish`/`on_exit` best-effort script execution logic. |
| `src-tauri/src/commands/tasks/transition_hooks/scripts.rs` | Shell script locating and execution environment setup. |
| `src-tauri/src/commands/tasks/transition_hooks/helpers.rs` | Hook state mapping, duration calculation, and error logging. |
| `src-tauri/src/commands/tasks/lifecycle.rs` | Public facade re-exporting task CRUD and query commands. |
| `src-tauri/src/commands/tasks/lifecycle/create.rs` | Task creation, initial stage assignment, and monotonic ID allocation. |
| `src-tauri/src/commands/tasks/lifecycle/update.rs` | Task title, description, and metadata updates. |
| `src-tauri/src/commands/tasks/lifecycle/delete.rs` | Task deletion and atomic SQLite cleanup. |
| `src-tauri/src/commands/tasks/lifecycle/query.rs` | `list_tasks`, task search, and filtering. |
| `src-tauri/src/commands/tasks/lifecycle/subtasks.rs` | Subtask attachment, child creation, and parent relationship management. |
| `src-tauri/src/commands/tasks/lifecycle/hook_records.rs` | Task hook execution record serialization. |
| `src-tauri/src/commands/tasks/runner.rs` | Public facade re-exporting agent execution runner commands. |
| `src-tauri/src/commands/tasks/runner/execution.rs` | Agent session initialization, environment setup, and spawn logic. |
| `src-tauri/src/commands/tasks/runner/execution_events.rs` | Runner event dispatch, telemetry forwarding, and UI updates. |
| `src-tauri/src/commands/tasks/runner/abort.rs` | Task run cancellation and graceful process tree termination. |
| `src-tauri/src/commands/tasks/runner/steer.rs` | Live steering directive queuing and delivery. |
| `src-tauri/src/commands/tasks/runner/model_resolve.rs` | Dynamic model override and persona resolution. |
| `src-tauri/src/commands/tasks/storage.rs` | Foreign task filtration and project boundary enforcement. |
| `src-tauri/src/commands/tasks/healing.rs` | Orphan board/column resolution on startup and migration. |
| `src-tauri/src/commands/boards.rs` | Multi-board CRUD, column settings, and pipeline sync. |
| `src-tauri/src/commands/personas/mod.rs` | Personas command module re-exports. |
| `src-tauri/src/commands/personas/crud.rs` | Persona creation, renaming, and deletion commands. |
| `src-tauri/src/commands/personas/meta.rs` | Persona metadata, icons, and color assignments. |
| `src-tauri/src/commands/dialogs.rs` | Public facade re-exporting dialogue history commands. |
| `src-tauri/src/commands/dialogs/activity.rs` | Task dialogue history querying and stopped item cleanup. |
| `src-tauri/src/commands/dialogs/subagents.rs` | Subagent session dialogue mapping and cache pruning. |
| `src-tauri/src/commands/git.rs` | Public facade re-exporting native Git commands. |
| `src-tauri/src/commands/git/commands.rs` | Git commit queries, branch status, and merge checks. |
| `src-tauri/src/commands/git/diff_parser.rs` | Unified Git diff calculation and structured hunk parsing. |
| `src-tauri/src/commands/git/worktree.rs` | Native worktree lifecycle commands and path resolution. |
| `src-tauri/src/commands/cvh_git.rs` | CVH repository sync and Git state verification commands. |
| `src-tauri/src/commands/projects.rs` | Workspace root switching and recent projects list. |
| `src-tauri/src/git/mod.rs` | Worktree isolation, prune, and branch teardown. |
| `src-tauri/src/git/cvh_save.rs` | Automated Git save and commit helpers for CVH configuration. |
| `src-tauri/src/git/cvh_sync.rs` | Live `.cvh/` sync to worktrees, secret exclusion, assume-unchanged. |
| `src-tauri/src/hooks.rs` | Public facade re-exporting script runner and process utilities. |
| `src-tauri/src/hooks/runner.rs` | Script runner executing `.bat`, `.sh`, and `.ps1` transition hooks. |
| `src-tauri/src/hooks/sensor.rs` | Gatekeeper sensor command execution with exit code inspection. |
| `src-tauri/src/hooks/process.rs` | Cross-platform process tree kill (`taskkill /F /T` on Windows). |

## Related checks

Choose checks for the changed behavior; use the active role's `sensors` rule for execution policy.

- `src-tauri/src/commands/tests/mod.rs` — Commands test suite router.
- `src-tauri/src/commands/tests/boards_tests.rs` — Boards tests.
- `src-tauri/src/commands/tests/dialogs_tests.rs` — Dialogs tests.
- `src-tauri/src/commands/tests/git_tests.rs` — Git tests.
- `src-tauri/src/commands/tests/tasks_id_tests.rs` — Task ID tests.
- `src-tauri/src/commands/tests/tasks_lifecycle_tests.rs` — Task lifecycle tests.
- `src-tauri/src/commands/tests/tasks_runner_tests.rs` — Task runner tests.
- `src-tauri/src/commands/tests/tasks_subtasks_tests.rs` — Task subtasks tests.
- `src-tauri/src/commands/tests/tasks_hooks_tests.rs` — Task transition hooks tests.
