# Rule: Boards and Pipelines Invariants
Refer to `.cvh/rules/boards.md`. Load via `load_rule("boards")`.

- Canonical ordering: `backlog` at 0, `ready` at final (`terminal: true`).
- 1:1 Invariant: `column.persona` strictly matches `.cvh/agents/<role>/` and frontmatter `name`.
- `stageOverrides.<stage_id>` strictly matches `column.id`.
- Dynamic board self-healing subsystem protects against orphaned boards/columns.
