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

### 1.2 Ironclad 1:1 Invariants
1. **column.persona strictly matches directory & frontmatter**:
   `column.persona` in `.cvh/boards.json` and `.cvh/pipeline.json` MUST STRICTLY MATCH:
   - The directory name `.cvh/agents/<role>/`
   - The frontmatter `name: <role>` inside `.cvh/agents/<role>/persona.md`
   Any mismatch breaks role resolution, file mapping, and runtime agent dispatch.

2. **stageOverrides.<stage_id> strictly matches column.id**:
   `stageOverrides.<stage_id>` in `.cvh/settings/tool-policy.json` MUST STRICTLY MATCH `column.id` in `boards.json` and `pipeline.json`.

---

## 2. Persona YAML Frontmatter
Every `persona.md` must start with clean YAML frontmatter:
```yaml
---
name: <role>
description: Use PROACTIVELY when <trigger conditions>
model: inherit
tools: [read_file, glob, grep, execute_command, edit_file, write_file, move_task, advance_task, todo_write, question, subagent, subagent_bg, collect_subagent, web_search, web_fetch]
scope: gated-writes
activation: on-enter <column-id> runs run_agent(<task-id>)
---
```

---

## 3. Strict Role Scope Invariant

### 3.1 Role Boundary: Infrastructure vs. Workflow Orchestration
- **cvh-dev (Harness Router / Infrastructure Architect)**:
  - Sole focus: configuring, repairing, and initializing `.cvh/` infrastructure (boards, pipelines, personas, rules, hooks, tool-policies).
  - **Task Card Operations**: `cvh-dev` primarily uses `advance_task` / `move_task`. When explicitly enabled in tool policy or requested by the user, `cvh-dev` is permitted to invoke `complete_task` or `create_task`.
  - **Handoff Mechanism**: When `cvh-dev` finishes its work, it calls `advance_task` / `move_task` with a comprehensive `summary` to advance the card to the next pipeline stage (e.g. `review`), or `reject_task` with `reason` if verification fails.

---

## 4. Sidecar Copilot Dock Architecture

- **Architecture & Purpose**: A persistent, right-docked architectural companion (`CopilotDock.tsx`, `copilotStore.ts`) providing real-time peer architectural consultation, multi-board supervision, and decomposition support alongside the active Kanban board.
- **Meta-Chat Persistence**: Dialogue history is persisted in SQLite (`.cvh/harness.db`, WAL mode) under `task_id = "copilot:meta-chat"` via `save_dialog_item` and restored with `load_copilot_history`.
- **Dedicated Copilot Tool Suite**: Pre-compiled with `get_copilot_tool_suite()` in `src-tauri/src/commands/copilot.rs`:
  - Operational tools: `create_task`, `create_tasks`, `complete_task`.
  - Inspection & edit tools: `read_file`, `write_file`, `edit_file`, `glob`, `grep`, `execute_command`, `web_search`.
- **ReAct Loop & Streaming**: Emits `copilot:stream` events for tokens, thoughts, tool calls, and tool results, with runtime abort support (`abort_copilot`).
- **Design Standard**: White paper / Vercel design system (clean `#171717` ink typography, `#ebebeb` borders, pure white container) distinguishing Copilot from the obsidian dark Kanban console.

---

## 5. Board Self-Healing & Task Stability Invariants

- **Self-Healing Integration**: Agents rely on the runtime board self-healing subsystem (`heal_task_board_and_column`, `heal_tasks`). When boards or columns are reconfigured, tasks orphaned by deleted boards or removed columns automatically self-heal back to `active_board_id` and the board's first column (`backlog`), preventing runtime crashes.
- **Relay Handoff**: When transitioning stages via `advance_task(summary)`, the summary is injected as `### [ЭСТАФЕТА ПАЙПЛАЙНА / RELAY HANDOFF]` into the next agent's prompt, guaranteeing 100% memory priming with zero turn degradation.

---

## 6. Two-Level Tool Policy Matrix (AND-Semantics)
- Global Switches: `.cvh/settings/tool-policy.json` (`globalEnabled`).
- Stage Overrides: `stageOverrides.<stage_id>` or column `allowedTools`.
- `effective(tool, stage) = globalEnabled[tool] && stageOverride[stage][tool]`.
- If a tool is globally disabled, a stage cannot re-enable it.

---

## 7. Progressive Disclosure
- Keep `persona.md` concise (≤30-50 lines).
- Offload domain-specific guides to modular files in `.cvh/agents/<role>/rules/*.md` (`tools.md`, `boards.md`, `agents.md`, `hooks.md`).
