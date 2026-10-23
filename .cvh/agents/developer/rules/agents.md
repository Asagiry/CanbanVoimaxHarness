# Guide: Agent Personas and Modular Rules (agents.md)

Authoritative rules for `.cvh/agents/<role>/`.

## 1. Directory-Only Structure & Persona Invariant
All agent definitions reside in dedicated directories under `.cvh/agents/<role>/`:
```text
.cvh/agents/<role>/
├── persona.md       # Primary persona prompt with YAML frontmatter
└── setting          # JSON settings ({ "model": "gemini-3.8-flash-high", "temperature": 0.2, ... })
```
Flat persona files (`.cvh/agents/<role>.md`) are deprecated and ignored.

### 1.1 Invariant: 1 Persona = Exactly 1 Directory
- **1 Persona = 1 Directory**: Exactly one directory `.cvh/agents/<persona>/` per persona role.
- **No Column ID Duplication**: Never create duplicate persona directories based on column IDs. For example, if a column has `id: "plan"` and `persona: "planner"`, create strictly `.cvh/agents/planner/`, never `.cvh/agents/plan/`.
- **Reusable Personas**: Multiple pipeline columns across different boards can reference the same `persona` without directory duplication.

### 1.2 Ironclad 1:1 Invariants
1. **column.persona strictly matches directory & frontmatter**:
   `column.persona` in `.cvh/boards.json` and `.cvh/pipeline.json` MUST STRICTLY MATCH:
   - The directory name `.cvh/agents/<role>/`
   - The frontmatter `name: <role>` inside `.cvh/agents/<role>/persona.md`
2. **stageOverrides.<stage_id> strictly matches column.id**:
   `stageOverrides.<stage_id>` in `.cvh/settings/tool-policy.json` MUST STRICTLY MATCH each `column.id` in `boards.json` and `pipeline.json`.

### 1.3 Agent Settings and Customization (`setting`)
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
- **Priority & Precedence**: Values defined in `setting` (`icon`, `color`, `model`, `temperature`, `context_window`) take precedence over YAML frontmatter defaults.
- **Bi-directional Synchronization**: Changing the icon or color in `setting` automatically synchronizes with `persona.md` YAML frontmatter, and vice versa.
- **Color Invariants**: Gray (`#64748b`, `#6b7280`) and green (`#10b981`, `#22c55e`) colors are strictly reserved for system stages `backlog` and `ready` and forbidden for agents. Agents select colors from `AGENT_PALETTE`.
- **SVG Icon Pack**: Agents select icons from canonical 18-icon pack `AGENT_ICONS` (`Bot`, `Brain`, `Cpu`, `Code2`, `ShieldCheck`, `Sparkles`, `History`, `Terminal`, `Zap`, `Flame`, `Radar`, `Workflow`, `Boxes`, `Wrench`, `Rocket`, `Compass`, `Crosshair`, `Layers`).

---

## 2. Invariant: Passive Stages Intake & Completion (backlog and ready)
- `backlog`: Intake stage (index 0). Strictly `persona: null`, `autoRun: null`.
- `ready`: Terminal delivery stage (final index, `terminal: true`). Strictly `persona: null`, `autoRun: null`.
- Agents operate EXCLUSIVELY on intermediate pipeline stages (`plan`, `dev`, `review`, `retro`).
- The `ready` stage must NEVER have an `on_leave` hook.

---

## 3. Persona YAML Frontmatter
Every `persona.md` must start with clean YAML frontmatter:
```yaml
---
name: <role>
description: Use PROACTIVELY when <trigger conditions>
model: inherit
tools: [read_file, glob, grep, ...]
scope: <scope-name>
activation: on-enter <column-id> runs run_agent(<task-id>)
icon: <LucideIconName>
color: <HexColor>
---
```

---

## 4. Role Scope & Operational Boundaries

### 4.1 Planner (`planner` / `architect`)
- Software Architect & Technical Solution Planner.
- Operates on the `plan` stage.
- Allowed tools: ONLY reading and planning (`read_file`, `glob`, `grep`, `web_search`, `web_fetch`, `todo_write`, `advance_task`, `load_rule`).
- Code editing tools (`edit_file`, `write_file`, `execute_command`) are STRICTLY FORBIDDEN.
- Prepares actionable implementation checklists and hands off to the developer via `advance_task(summary)`.

### 4.2 Developer (`developer`)
- Autonomous Lead Software Engineer.
- Operates on the `dev` stage inside an isolated git worktree (`worktrees/<task-slug>`).
- Implements code, runs Turbo Sensors (`cargo check`, `npm run build`), updates modular rules simultaneously, and commits atomically.
- Hands off to review via `advance_task(summary)`.

### 4.3 Reviewer (`reviewer`)
- Ultra-Strict Code Auditor & Release Authority.
- Operates on the `review` stage.
- Audits sensor proofs, rule freshness, code invariants, and anti-slop.
- Code editing tools (`edit_file`, `write_file`) are STRICTLY FORBIDDEN.
- Merges to `main` upon approval.
- Routes approved tasks: immediately approved -> `ready`; approved after rejections -> `retro`.
- Rejects defect-laden tasks via `reject_task(reason)` back to `dev`.

### 4.4 Retro (`retro`)
- Continuous Improvement & Rule Evolution Specialist.
- Operates on the `retro` stage when a task required revisions after review rejection.
- Inspects task chronology via `get_task_history`, performs contrastive trajectory analysis, and updates `.cvh/rules/<topic>.md`.
- Advances task to `ready` via `advance_task(summary)`.

---

## 5. Relay Handoff Protocol
When transitioning stages via `advance_task(summary)`, the summary is injected as:
`### [RELAY HANDOFF]`
into the next agent's prompt, guaranteeing 100% memory priming with zero turn degradation.

---

## 6. Progressive Disclosure & Dynamic Rules
- Keep `persona.md` compact, focused on role identity, constraints, and execution workflow.
- Domain guidelines and technical invariants are decoupled into modular rules in `.cvh/rules/*.md` and loaded dynamically via `load_rule("<topic>")`.
