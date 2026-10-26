# Guide: Agent Personas and Modular Rules (agents.md)

Authoritative rules for `.cvh/agents/<role>/`.

## 1. Directory-Only Structure & Persona Invariant
All agent definitions reside in dedicated directories under `.cvh/agents/<role>/`:
```text
.cvh/agents/<role>/
├── persona.md       # Primary persona prompt with YAML frontmatter
├── setting          # JSON settings ({ "model": "gemini-3.8-flash-high", "temperature": 0.2, ... })
└── rules/           # Role-specific modular rules
    ├── rule1.md
    └── ...
```

### 1.1 Invariant: 1 Persona = Exactly 1 Directory
- **1 Persona = 1 Directory**: Exactly one directory `.cvh/agents/<persona>/` per persona role.
- **No Column ID Duplication**: Never create duplicate persona directories based on column IDs.
- **Role-Specific Isolation**: Each agent persona MUST maintain only the modular rules relevant to its operational domain inside its own `rules/` catalog.

### 1.2 Agent Settings and Customization (`setting`)
Each agent directory `.cvh/agents/<role>/setting` contains JSON configuration without extension:
```json
{
  "model": "gemini-3.8-flash-high",
  "temperature": 0.2,
  "context_window": 200000,
  "max_tokens": 8192,
  "icon": "Cpu",
  "color": "#6366f1"
}
```
- **Priority & Precedence**: Values defined in `setting` take precedence over YAML frontmatter defaults.
- **Color Invariants**: Gray (`#64748b`) and green (`#10b981`) colors are strictly reserved for system stages `backlog` and `ready` and forbidden for agents.
