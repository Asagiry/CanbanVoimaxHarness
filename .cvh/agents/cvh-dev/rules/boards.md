# Guide: Boards and Pipelines Configuration (boards.md)

Authoritative rules for `.cvh/boards.json` and `.cvh/pipeline.json`.

---

## 1. The Kanban Board as Application Control Plane
In CanbanVoimaxHarness, the Kanban board is the runtime execution environment.
Columns represent autonomous execution stages. Moving a task card triggers automated hooks and initiates agent execution loops.

---

## 2. Canonical Stage Sequence
The canonical autonomous engineering pipeline follows this sequential order:
```text
[Backlog] ──► [Planning] ──► [Developer] ──► [Review] ──┬──(Approved immediately)──────► [Ready]
 (index 0)       (plan)         (dev)       (review)    │                                (terminal)
                                                        └──(Approved after changes)──► [Retro] ────┘
                                                                                        (retro)
```
- **Backlog**: ID `"backlog"` must always reside at index 0 (incoming tasks).
- **Planning (`plan`)**: Architecture, requirements decomposition, static planning without checks (agent `planner`).
- **Developer (`dev`)**: Implementation inside isolated git worktree, compiler sensors, rule synchronization (agent `developer`).
- **Review (`review`)**: Strict Harness2.0 Blind Selection audit across 7 criteria and 6 mandatory gates (agent `reviewer`).
- **Retro (`retro`)**: Execution trajectory analysis when tasks required fixes after review rejection; rule evolution (agent `retro`).
- **Ready**: ID `"ready"` (with `terminal: true`) must reside at the final position.
- **Immutable Anchors**: Removing `backlog` or `ready` is forbidden across all boards.

---

## 3. Passive Stages Invariant (backlog and ready)
- `backlog`: Intake and accumulation stage. Tasks wait here until selected for work.
- `ready`: Terminal delivery stage (`terminal: true`). Tasks here are fully verified, merged, and completed.
- **Strict Bot Ban**: Both `backlog` and `ready` MUST have `persona: null` and `autoRun: null`. No autonomous agents ever run in backlog or ready.
- **Terminal Exit Ban**: The `ready` stage must NEVER define an `on_leave` hook.

---

## 4. Self-Healing Multi-Board State
- **Orphaned Board Resolution**: If `task.board_id` is missing or references a non-existent board, the runtime reassigns the task to `active_board_id`.
- **Orphaned Column Resolution**: If `task.stage_id` does not exist on the target board, the task is safely moved to the board's intake column (`backlog`).
