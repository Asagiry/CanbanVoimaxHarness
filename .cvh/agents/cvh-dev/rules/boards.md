---
description: Multi-board layout configuration, pipeline column schemas, board presets, and routing rules in boards.json.
---
# Guide: Boards and Pipelines Configuration (boards.md)

Authoritative rules for `.cvh/boards.json` and `.cvh/pipeline.json`.

---

## 1. The Kanban Board as Application Control Plane
In CanbanVoimaxHarness, the Kanban board is the runtime execution environment.
Columns represent autonomous execution stages. Moving a task card triggers automated hooks and initiates agent execution loops.

---

## 2. Open, Project-Driven Control Plane Architecture
CanbanVoimaxHarness is an open, domain-agnostic desktop agentic IDE and orchestration environment for ANY project type (Software Dev, 3D/Voxel Art, Content, Lean Kanban).
The Kanban board is a dynamic control plane adaptable to arbitrary project workflows and multi-agent topologies.

The ONLY structural invariants across all boards:
1. **Intake Anchor**: First column (index 0, e.g. `backlog`), `persona: null`, `terminal: null`. Tasks wait here until selected for work.
2. **Delivery Anchor**: Final column (e.g. `ready`), `persona: null`, `terminal: true`. Tasks entering this column are finalized deliverables.
3. **Intermediate Execution Stages**: Completely arbitrary and defined by project requirements (any personas, models, tools, gatekeeper sensors, and routing hooks such as `routes.on_approved` / `routes.on_changes_requested`).

Boards are configured in `.cvh/boards.json` and mirrored in `.cvh/pipeline.json`.

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

---

## 5. Ironclad Board Preservation Invariant
- **Strict Board Isolation**: Tasks MUST strictly preserve their `board_id` across `advance_task`, agent loop completion, and UI moves. Cross-board hopping is forbidden.
- **Autonomous Progression Scope**: `advance_task` and agent finalization MUST advance tasks through columns belonging to the task's own board (`task.board_id`). Tasks must never be abducted to another board simply because the user switched tabs or changed `active_board_id`.
- **UI Drag & Drop**: Moving tasks in the UI preserves `task.boardId` unless explicitly re-assigned.


---

## 5. Multi-Stage Pipeline Topologies & Interactive Setup
When creating or restructuring pipelines, refer to `load_rule("pipeline-setup")`.
`cvh-dev` must interactively ask the user via `question` to select among standard topologies:
1. **Standard Peer Review**: `backlog` ➔ `dev` (Developer) ➔ `review` (Reviewer) ➔ `ready`.
2. **Full Cognitive Triad (CarSmash Model)**: `backlog` ➔ `plan` (Planner) ➔ `dev` (Developer) ➔ `review` (Reviewer) ➔ `retro` (Retro) ➔ `ready`.
3. **Lean Solo**: `backlog` ➔ `dev` (Developer) ➔ `ready`.
4. **Custom**: Defined interactively with the user.
