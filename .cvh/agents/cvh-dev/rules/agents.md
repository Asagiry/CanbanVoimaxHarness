# Guide: Agent Personas and Modular Rules

Authoritative rules for `.cvh/agents/<role>/`.

## 1. Directory-Only Structure & Persona Invariant
All agent definitions reside in dedicated directories under `.cvh/agents/<role>/`:
```text
.cvh/agents/<role>/
├── persona.md       # Primary persona prompt with YAML frontmatter (or agent.md)
├── setting          # JSON settings ({ "model": "gemini-3.8-flash-high", "context_window": 128000, "temperature": 0.2, "max_tokens": 8192 })
└── rules/           # Modular rule files loaded during zero-turn priming
    ├── rule1.md
    └── ...
```
Flat persona files (`.cvh/agents/<role>.md`) are deprecated and ignored.

### 1.1 Invariant: 1 Persona = Exactly 1 Directory
- **1 Persona = 1 Directory**: Exactly one directory `.cvh/agents/<persona>/` per persona role.
- **No Column ID Duplication**: Never create duplicate persona directories based on column IDs. For example, if a column has `id: "review"` and `persona: "reviewer"`, create strictly `.cvh/agents/reviewer/`, never `.cvh/agents/review/`.
- **Reusable Personas**: Multiple pipeline columns across different boards can reference the same `persona` without directory duplication.

## 2. Persona YAML Frontmatter
Every `persona.md` must start with clean YAML frontmatter:
```yaml
---
name: <role>
description: Use PROACTIVELY when <trigger conditions>
model: inherit
tools: [read_file, glob, grep, execute_command, edit_file, write_file]
scope: gated-writes
activation: on-enter <column-id> runs run_agent(<task-id>)
---
```

## 3. Two-Level Tool Policy Matrix (AND-Semantics)
- Global Switches: `.cvh/settings/tool-policy.json` (`globalEnabled`).
- Stage Overrides: `stageOverrides.<stage_id>` or column `allowedTools`.
- `effective(tool, stage) = globalEnabled[tool] && stageOverride[stage][tool]`.
- If a tool is globally disabled, a stage cannot re-enable it.

## 4. Progressive Disclosure
- Keep `persona.md` concise (≤30-50 lines).
- Offload domain-specific guides to modular files in `.cvh/agents/<role>/rules/*.md`.
