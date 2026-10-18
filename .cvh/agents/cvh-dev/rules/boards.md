# Guide: Boards and Pipelines Configuration

Authoritative rules for `.cvh/boards.json` and `.cvh/pipeline.json`.

## 1. Boards as Application Control Plane
In CanbanVoimaxHarness, the Kanban board IS the runtime control plane.
Columns represent autonomous stages. Moving a card triggers stage automations and agents.

## 2. Canonical Ordering Invariant
- **Backlog**: id `"backlog"` MUST ALWAYS exist at index 0 (the first column, intake).
- **Ready**: id `"ready"` (with `terminal: true`) MUST ALWAYS exist at the final index (completion).
- **Middle stages**: Preserve relative pipeline order (e.g. `dev` -> `review` -> `qa`).
- **Immutable Anchors**: Deleting `backlog` or `ready` is strictly prohibited on all boards. If omitted in an input payload, the harness backend (`normalize_columns`) automatically re-injects them.

## 3. Board Self-Healing Subsystem Invariant
- **Autonomous Task Healing**: The Rust desktop backend implements dynamic self-healing (`heal_task_board_and_column` and `heal_tasks` in `src-tauri/src/commands/tasks.rs`).
- **Orphaned Board Resolution**: If a task's `board_id` is missing, empty, or references a deleted board, the harness dynamically heals it to `active_board_id` (or the first existing board, or `"development"`).
- **Orphaned Column Resolution**: If a task's `stage_id` does not exist within its target board's columns, the harness dynamically heals it to the board's first column (typically `"backlog"`).
- **Execution Points**: Healing executes autonomously on task loading (`load_tasks`), board deletion, board switching, and task imports, ensuring zero orphaned task crashes.

## 4. Configuration Files
- `.cvh/boards.json`:
  ```json
  {
    "activeBoardId": "development",
    "boards": [
      {
        "id": "development",
        "name": "Development",
        "columns": [ ... ]
      }
    ]
  }
  ```
- `.cvh/pipeline.json`:
  Mirrors the active board's columns and lifecycle settings.
  Stage properties: `id`, `title`, `persona`, `gatekeeperSensor`, `allowedTools`, `terminal`, `hooks`, `routes`.

## 5. Persona Mapping Invariant: 1 Persona = Exactly 1 Directory
- **Single Source of Truth**: Exactly 1 persona corresponds to 1 directory under `.cvh/agents/<persona>/`.
- **Decoupled Column IDs and Personas**: Columns define stages (`id`), while `persona` defines the autonomous agent role executing in that stage.
- **No Duplication by Column ID**: Never create duplicate persona directories based on column IDs. For example, if a column has `id: "review"` and `persona: "reviewer"`, create strictly `.cvh/agents/reviewer/`, never `.cvh/agents/review/`. Multiple columns can reference the same persona without duplicating directories.

## 6. Ironclad 1:1 Invariants
- **column.persona strictly matches directory name & frontmatter name**:
  `column.persona` in `boards.json` and `pipeline.json` MUST STRICTLY MATCH the directory name `.cvh/agents/<role>/` and frontmatter `name: <role>`.
- **stageOverrides.<stage_id> strictly matches column.id**:
  `stageOverrides.<stage_id>` in `.cvh/settings/tool-policy.json` MUST STRICTLY MATCH `column.id` in `boards.json` and `pipeline.json`.

## 7. Board Reconfiguration & Strict Role Scope Invariants
- **Preserve Running Context**: When creating, renaming, or modifying boards in `.cvh/boards.json` or `.cvh/pipeline.json`, NEVER delete the board or column where your current task is running.
- **Task Preservation**: If creating or restructuring boards, ensure all existing tasks in `.cvh/tasks.json` have their `boardId` cleanly preserved or mapped to intended boards.
- **Strict Role Scope**: `cvh-dev` configures and repairs infrastructure; `cvh-dev` does NOT create or complete operational task cards on the board (reserved for Root Orchestrator and Copilot).

## 8. Sidecar Copilot Multi-Board Integration
- The **Sidecar Copilot Dock** (`CopilotDock.tsx`, `copilot:meta-chat`) loads full multi-board and column topology dynamically on every user turn via `compile_copilot_system_prompt`.
- Copilot is empowered to decompose requirements into new task cards across any board or column using `create_task` / `create_tasks`, while `cvh-dev` remains strictly scoped to infrastructure maintenance.
