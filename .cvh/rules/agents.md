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

## 2. Инвариант пассивных стадий intake & completion (backlog и ready)
- `backlog` — стадия постановки и накопления задач (Intake). Здесь задачи ждут, пока их возьмут в работу.
- `ready` — терминальная стадия завершения (Done / Delivery). Здесь задачи полностью готовы.
- В `backlog` и в `ready` СТРОГО ЗАПРЕЩЕНО назначать персоны агентов (СТРОГО `persona: null`, `autoRun: null`).
- Никаких ботов в беклоге и готово! Агенты (разработчики, ревьюеры) работают ИСКЛЮЧИТЕЛЬНО на промежуточных стадиях пайплайна (`dev`, `review`).

---

## 3. Persona YAML Frontmatter
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

## 4. Strict Role Scope Invariant

### 4.1 Role Boundaries
- **Developer (`developer`)**:
  - Autonomous software engineer working inside isolated git worktree (`worktrees/<task-slug>`).
  - Implements requirements, runs ground-truth sensors (`cargo check`, `npm run build`), updates architectural rules (`rules-sync`), commits atomically, and hands off via `advance_task(summary)`.
- **Reviewer (`reviewer`)**:
  - Ultra-strict code auditor and gatekeeper.
  - Audits sensor results, rule freshness, code quality, and security.
  - Upon approval, merges feature branch to `main` and pushes to remote, then advances to `ready`.
  - Upon rejection, issues `reject_task(reason)` with actionable change requests back to `dev`.
- **CVH-Dev (`cvh-dev`)**:
  - Harness router and infrastructure architect. Configures `.cvh/` (boards, pipelines, personas, rules, hooks, tool-policies).
  - Absolute ban on `create_task`, `create_tasks`, `complete_task`.

---

## 5. Relay Handoff Protocol
When transitioning stages via `advance_task(summary)`, the summary is injected as `### [ЭСТАФЕТА ПАЙПЛАЙНА / RELAY HANDOFF]` into the next agent's prompt, guaranteeing 100% memory priming with zero turn degradation.

---

## 6. Two-Level Tool Policy Matrix (AND-Semantics)
- Global Switches: `.cvh/settings/tool-policy.json` (`globalEnabled`).
- Stage Overrides: `stageOverrides.<stage_id>`.
- `effective(tool, stage) = globalEnabled[tool] && stageOverride[stage][tool]`.
- If a tool is globally disabled, a stage cannot re-enable it.

---

## 7. Progressive Disclosure
- Keep `persona.md` concise and focused.
- Offload domain-specific architectural guides to modular files in `.cvh/rules/*.md` and `.cvh/agents/<role>/rules/*.md`.
