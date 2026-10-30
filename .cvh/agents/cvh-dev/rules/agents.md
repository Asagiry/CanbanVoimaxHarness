---
description: Authoritative directory structure, persona definitions, setting schemas, and modular rule conventions for CVH agents.
---
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

---

## 2. Global Shared Rules & Architectural Navigation Maps (`.cvh/rules/`)
Shared architectural contracts, system-wide invariants, and repository navigation maps reside in `.cvh/rules/`:
```text
.cvh/rules/
├── <topic>.md          # System-wide architectural invariants or standards
├── architecture.md     # Overall project topology and component maps
└── ...
```
- **Universal Availability**: Global rules in `.cvh/rules/` are accessible to all agents via `load_rule("<rule_stem>")` without requiring a role prefix.
- **Zero Duplication**: Shared architectural contracts, router maps, and cross-cutting specifications belong in `.cvh/rules/` rather than duplicated across role directories.
- **Subagent Codebase Discovery Mandate**: When initializing or configuring a project, `cvh-dev` MUST NOT copy CVH-specific internal paths (e.g. `src-tauri/`, Tauri IPC, React stores) into external repositories. Instead, `cvh-dev` must dispatch specialized subagents (`subagent`) to explore the directory structure, identify key frameworks, and author tailored, project-specific navigation maps directly into `.cvh/rules/`.
- **Architectural Map Synchronization Mandate**: Whenever agents add, remove, rename, or relocate modules/files in the project, they MUST update the corresponding project navigation maps in `.cvh/rules/` in the exact same diff/commit so navigation maps remain 100% faithful to the filesystem.

---

## 3. Universal Rule Specification & Metadata Invariant
Every modular rule file in `.cvh/rules/*.md` and `.cvh/agents/<role>/rules/*.md` MUST strictly adhere to the Universal Rule Specification:
1. **Mandatory YAML Frontmatter**: Every rule MUST begin with YAML frontmatter declaring a concise, one-sentence `description`:
   ```markdown
   ---
   description: Authoritative 1-sentence summary of rule invariants and domain scope.
   ---
   # Rule Title...
   ```
2. **Dynamic JIT Prompt Integration**: The backend automatically extracts frontmatter descriptions to populate the `load_rule` tool instructions in system prompts, giving agents full awareness of what each rule governs without burning context on full bodies.
3. **English-Only Content Invariant**: All rule files MUST be authored 100% in clear, concise, authoritative English.
4. **Timeless Contract Invariant**: Rules represent eternal architectural standards. NEVER record transient bug histories, commit messages, or task changelogs in rule files.

---

## 4. Persona Creation & Behavioral DNA Infusion
When instantiating personas, `cvh-dev` must NEVER generate shallow placeholder prompts.
Refer to `load_rule("pipeline-setup")` for the complete behavioral specifications:
- **Developer**: OpenCode 4 Laws (anti-overengineering), Ponytail minimal diffs, Zero-TDD, contrastive diff self-audit.
- **Reviewer**: Read-only (Tool Policy Level B), Presumption of Guilt, Three-Axis Review, Anthropic Claude Code convergence (immunity to historical pre-existing bugs, diff-only re-review, line-by-line actionable references).
- **Planner**: Read-only, context window protection for developer, subagent explorer, question-based clarification.
- **Retro**: Engine code prohibition (`src/` strictly forbidden), Karpathy LLM-Wiki knowledge ingest, Cue-Lesson-Evidence protocol, automated audit hardening.
