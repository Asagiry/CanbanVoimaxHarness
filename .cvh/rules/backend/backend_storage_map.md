---
description: Domain IDs, task state, SQLite repositories and conversation snapshots.
---
# Domain and persistence

Use nominal task, stage and board IDs across domain boundaries. SQLite repositories own persisted data; the pool configures WAL, foreign keys and migrations. Exact hidden context snapshots preserve message boundaries; dialogue deletion invalidates snapshots and uses `type='deleted'` rows to reject stale saves. All modules strictly observe the <=500 lines ceiling.

| File | Responsibility |
| --- | --- |
| `src-tauri/src/domain/mod.rs` | Submodule re-exports (`ids`, `state`). |
| `src-tauri/src/domain/ids.rs` | Nominal `TaskId`, `StageId` and `BoardId` branded Value Objects. |
| `src-tauri/src/domain/state.rs` | Task lifecycle states (`TaskState` ADT) and valid transitions. |
| `src-tauri/src/domain/state_serde.rs` | Specialized Serde serialization and deserialization for `TaskState`. |
| `src-tauri/src/db/mod.rs` | Repository and pool module exports. |
| `src-tauri/src/db/pool.rs` | R2D2 pool factory, WAL mode, foreign keys, 5s busy timeout. |
| `src-tauri/src/db/migrations.rs` | Automated database schema migrations and indexing. |
| `src-tauri/src/db/task_repo.rs` | Public facade re-exporting task repository operations. |
| `src-tauri/src/db/task_repo/task_query.rs` | Task queries, retrieval by ID, stage, and board. |
| `src-tauri/src/db/task_repo/upsert.rs` | Atomic task insertion, updates, and state persistence. |
| `src-tauri/src/db/task_repo/delete.rs` | Atomic task deletion and subtask cascade cleanup. |
| `src-tauri/src/db/task_repo/sequence.rs` | Monotonic `CVH-NNN` task ID number allocation. |
| `src-tauri/src/db/task_repo/todos.rs` | Task checklist item CRUD and state synchronization. |
| `src-tauri/src/db/dialog_repo.rs` | Public facade re-exporting dialogue repository operations. |
| `src-tauri/src/db/dialog_repo/dialog_query.rs` | Dialogue items and activity feed queries by task. |
| `src-tauri/src/db/dialog_repo/mutation.rs` | Atomic dialogue item appending and `type='deleted'` tombstones. |
| `src-tauri/src/db/dialog_repo/tools.rs` | Tool execution logs and JSON payload persistence. |
| `src-tauri/src/db/context_repo.rs` | Hidden SQLite snapshots preserve message boundaries and pending user input across restarts; file mirror at `.cvh/context/*.json` with compacted-guard. |
| `src-tauri/src/db/telemetry_repo.rs` | Aggregated tokens, non-decaying TPS metrics. |

## Related checks

Choose checks for the changed behavior; use the active role's `sensors` rule for execution policy.

- `src-tauri/src/domain/tests.rs` — Domain tests.
- `src-tauri/src/domain/state_tests.rs` — State machine tests.
- `src-tauri/src/db/tests.rs` — Database repository and pool tests.
