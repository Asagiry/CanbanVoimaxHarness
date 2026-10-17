# Guide: Boards and Pipelines Configuration

Authoritative rules for `.cvh/boards.json` and `.cvh/pipeline.json`.

## 1. Boards as Application Control Plane
In CanbanVoimaxHarness, the Kanban board IS the runtime control plane.
Columns represent autonomous stages. Moving a card triggers stage automations and agents.

## 2. Canonical Ordering Invariant
- **Backlog**: id `"backlog"` MUST ALWAYS exist at index 0 (the first column).
- **Ready**: id `"ready"` (with `terminal: true`) MUST ALWAYS exist at the final index.
- **Middle stages**: Preserve relative pipeline order (e.g. `dev` -> `review` -> `qa`).
- Deleting `backlog` or `ready` is strictly prohibited on all boards. If omitted in a payload, the harness automatically re-injects them.

## 3. Configuration Files
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
  Stage properties: `id`, `title`, `persona`, `contextWindow`, `gatekeeperSensor`, `allowedTools`, `terminal`, `hooks`, `routes`.

## 4. Persona Mapping Invariant: 1 Persona = Exactly 1 Directory
- **Single Source of Truth**: Exactly 1 persona corresponds to 1 directory under `.cvh/agents/<persona>/`.
- **Decoupled Column IDs and Personas**: Columns define stages (`id`), while `persona` defines the autonomous agent role executing in that stage.
- **No Duplication by Column ID**: Never create duplicate persona directories based on column IDs. For example, if a column has `id: "review"` and `persona: "reviewer"`, create strictly `.cvh/agents/reviewer/`, never `.cvh/agents/review/`. Multiple columns can reference the same persona without duplicating directories.

## 5. Ironclad 1:1 Invariants
- **column.persona strictly matches directory name & frontmatter name**:
  `column.persona` in `boards.json` and `pipeline.json` MUST STRICTLY MATCH the directory name `.cvh/agents/<role>/` and frontmatter `name: <role>`.
- **stageOverrides.<stage_id> strictly matches column.id**:
  `stageOverrides.<stage_id>` in `.cvh/settings/tool-policy.json` MUST STRICTLY MATCH `column.id` in `boards.json` and `pipeline.json`.

## 6. Board Reconfiguration Invariant
When creating, renaming, or modifying boards in .cvh/boards.json or .cvh/pipeline.json, NEVER delete the board or column where your current task is running. If you create new boards, ensure all existing tasks in .cvh/tasks.json have their boardId cleanly preserved or mapped to the intended boards.
