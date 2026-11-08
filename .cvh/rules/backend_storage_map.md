---
description: Domain IDs, task state, SQLite and conversation snapshots.
---
# Domain and persistence

Use nominal task, stage and board IDs across domain boundaries. SQLite repositories own persisted data; the pool configures WAL, foreign keys and migrations. Exact hidden context snapshots preserve message boundaries; dialogue deletion invalidates snapshots and uses `type='deleted'` rows to reject stale saves.

| File | Responsibility |
| --- | --- |
| `src-tauri/src/domain/mod.rs` | Submodule re-exports (`ids`, `state`). |
| `src-tauri/src/domain/ids.rs` | Nominal `TaskId`, `StageId` and `BoardId` types. |
| `src-tauri/src/domain/state.rs` | Task lifecycle states and allowed transitions. |
| `src-tauri/src/db/mod.rs` | Repository and pool module exports. |
| `src-tauri/src/db/pool.rs` | R2D2 pool, WAL mode, foreign keys, 5s timeout. |
| `src-tauri/src/db/task_repo.rs` | Monotonic ID allocation, subtask cascade. |
| `src-tauri/src/db/dialog_repo.rs` | Atomic upsert, tool JSON persistence; `type='deleted'` rows reject stale saves and invalidate context snapshots. |
| `src-tauri/src/db/context_repo.rs` | Hidden SQLite snapshots preserve message boundaries and pending user input across restarts; file mirror at `.cvh/context/*.json` with compacted-guard (no save after a newer `compacted` item). |
| `src-tauri/src/db/telemetry_repo.rs` | Aggregated tokens, non-decaying TPS metrics. |

## Related checks

Choose checks for the changed behavior; use the active role's `sensors` rule for execution policy.

- `src-tauri/src/domain/tests.rs` — tests.
- `src-tauri/src/domain/state_tests.rs` — state tests.
- `src-tauri/src/db/tests.rs` — tests.
