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
- **Planning (`plan`)**: Architecture, requirements decomposition, and implementation checklist planning (agent `planner`).
- **Developer (`dev`)**: Implementation inside an isolated git worktree, ground-truth sensors, rule synchronization (agent `developer`).
- **Review (`review`)**: Strict Harness2.0 Blind Selection audit across 7 criteria and 6 mandatory gates (agent `reviewer`).
- **Retro (`retro`)**: Execution trajectory analysis when tasks required fixes after review rejection; rule evolution in `.cvh/rules/` (agent `retro`).
- **Ready**: ID `"ready"` (with `terminal: true`) must reside at the final position.
- **Immutable Anchors**: Removing `backlog` or `ready` is forbidden across all boards. If omitted, the backend normalization routine (`normalize_pipeline_columns`) restores them automatically.

---

## 3. Passive Stages Invariant (backlog and ready)
- `backlog`: Intake and accumulation stage. Tasks wait here until selected for work.
- `ready`: Terminal delivery stage (`terminal: true`). Tasks here are fully verified, merged, and completed.
- **Strict Bot Ban**: Both `backlog` and `ready` MUST have `persona: null` and `autoRun: null`. No autonomous agents ever run in backlog or ready.
- **Terminal Exit Ban**: The `ready` stage must NEVER define an `on_leave` hook. Only `on_enter` is permitted (e.g., verifying remote push).

---

## 4. Post-Review Routing Invariant
- **Approved Immediately (First Attempt)**: When a task passes review on its initial submission without prior rejections (`CHANGES_REQUESTED`), it advances directly to `ready`.
- **Approved After Revisions**: When a task previously received `CHANGES_REQUESTED` and required developer revisions, approval routes the card to `retro` to record root-cause learnings and evolve rules.
- **From Retro Stage**: Once the lesson is codified in `.cvh/rules/`, the Retro agent advances the card to `ready`.

---

## 5. Self-Healing Multi-Board State
- **Autonomous Task Healing**: The backend implements automatic recovery (`heal_task_board_and_column` and `heal_tasks` in `src-tauri/src/commands/tasks.rs`).
- **Orphaned Board Resolution**: If `task.board_id` is missing or references a non-existent board, the runtime reassigns the task to `active_board_id` (or the first available board).
- **Orphaned Column Resolution**: If `task.stage_id` does not exist on the target board, the task is safely moved to the board's intake column (`backlog`).

---

## 6. Strict 1:1 Mapping Invariants
- **`column.persona` strictly matches directory and frontmatter**: `column.persona` in `boards.json` and `pipeline.json` MUST strictly match the directory name `.cvh/agents/<role>/` and the `name: <role>` field in `persona.md` frontmatter.
- **`stageOverrides.<stage_id>` strictly matches `column.id`**: The override section in `.cvh/settings/tool-policy.json` MUST contain keys matching every `column.id` across boards.

---

## 7. Board Reconfiguration & Role Scope
- **Preserving Execution Context**: When modifying boards, never delete an active board or column that contains currently running tasks.
- **Preserving Existing Tasks**: When restructuring boards, existing task records in SQLite retain their identifiers and are reassigned deterministically.

---

## 8. Ironclad Board Preservation Invariant
- **Strict Board Isolation**: Tasks MUST strictly preserve their `board_id` across `advance_task`, agent loop completion, and UI moves. Cross-board hopping is forbidden.
- **Autonomous Progression Scope**: `advance_task` and agent finalization MUST advance tasks through columns belonging to the task's own board (`task.board_id`). Tasks must never be abducted to another board simply because the user switched tabs or changed `active_board_id`.
- **UI Drag & Drop**: Moving tasks in the UI preserves `task.boardId` unless explicitly re-assigned.
