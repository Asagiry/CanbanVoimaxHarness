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
