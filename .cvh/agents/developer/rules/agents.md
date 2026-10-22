# Rule: Agent Personas and Relay Handoff
Refer to `.cvh/rules/agents.md`. Load via `load_rule("agents")`.

- Persona directory structure: `persona.md`, `setting`, `rules/`, `hooks/`.
- YAML frontmatter with explicit tools, model, activation.
- Progressive disclosure: compact personas with JIT modular rules.
- Relay handoff via `advance_task(summary)`.
