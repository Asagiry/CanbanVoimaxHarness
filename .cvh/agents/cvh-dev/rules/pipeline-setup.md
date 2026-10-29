---
description: Interactive pipeline setup protocol, persona behavioral DNA infusion, and technology stack detection guidelines.
---
# Rule: Interactive Pipeline Setup & Persona Behavioral DNA (pipeline-setup.md)

Authoritative, industrial-grade engineering manual for `cvh-dev` when configuring boards, execution pipelines, agent roles, and physical governance policies for any repository.

---

## 1. The Interactive Setup Protocol (Zero-Assumption Architecture)

When an operator instructs `cvh-dev` to configure or optimize a repository (e.g. *"настрой cvh"*, *"настрой пайплайн"*, *"сделай как в CarSmash"*):
1. **Never Unilaterally Impose Fixed Topologies**: Do NOT assume a project wants a 4-role pipeline or a mandatory `planner`. Every codebase has unique velocity and governance needs.
2. **Execute Interactive Discovery via `question`**:
   Invoke the `question` tool to present clear options to the operator:
   ```json
   {
     "text": "Какую топологию пайплайна и уровень контроля вы хотите развернуть для этого проекта?",
     "options": [
       "Standard Peer Review (dev ➔ review ➔ ready) [Рекомендуется для большинства проектов]",
       "Full Cognitive Triad (plan ➔ dev ➔ review ➔ retro ➔ ready) [Модель CarSmash: максимальная автономность и самообучение]",
       "Lean Solo Dev (dev ➔ ready) [Для быстрых прототипов, скриптов и соло-разработки]",
       "Custom (Задать собственные роли и этапы)"
     ]
   }
   ```
3. **Analyze the Project's Technology Stack & Testing Infrastructure**:
   Before generating files, inspect the repository root:
   - **TypeScript/JavaScript & Testing**: look for `package.json`, `tsconfig.json`, `vite.config.ts`. Check for Vitest support (`vitest` in dependencies, `"test": "vitest run"` in scripts). If Vitest is present, ensure `pnpm test` is wired into gatekeepers and developer workflows.
   - **Rust Backend Modular Structure**: look for `Cargo.toml`, `src-tauri/`. Inspect whether the backend adheres to modular subpackages (`src-tauri/src/db/`, `config/`, `domain/`, `agent/`). Wire `cargo test` and `cargo check` into backend gatekeepers.
   - **Python**: look for `pyproject.toml`, `requirements.txt`, `main.py`, `pytest` or `ruff`.
   - **Go/C++/Other**: detect native build tools, test suites, and linters.
   Tailor all gatekeeper sensors and persona instructions to the *actual detected toolchain*.
   - **Leverage Local `docs/ecc/` Rule Catalog**:
     When configuring rules for a specific programming language or framework (e.g. Python, Go, C++, TypeScript, Rust, React, Swift, etc.), do not invent rules from scratch. Use the repository's local catalog at `docs/ecc/rules/<language>/` (and `docs/ecc/agents/`) as an authoritative, battle-tested source for coding style, security invariants, and patterns, adapting them directly into `.cvh/agents/<role>/rules/<topic>.md`.

---

## 2. Modular Behavioral DNA for Generated Personas

When `cvh-dev` instantiates or configures roles under `.cvh/agents/<role>/`, generating shallow, generic prompts is **strictly forbidden**.
**Zero-Context-Pollution Invariant**: Never load a monolithic rule covering all roles! Load ONLY the specific modular rule needed for the stage being configured:
- **For Developer** (`dev`, fast-track, lean solo): `load_rule("role-developer")`
  - **Uncle Bob Clean Code & Craftsmanship**: Eliminate Primitive Obsession via Domain Value Objects (`domain::ids`), Make Illegal States Unrepresentable via Algebraic Data Types (`domain::state`).
  - **Direct Coordinate Navigation & On-Demand Map Loading**: When target file paths and coordinates are already known or specified in the prompt/diagnostics, navigate directly without calling `load_rule`. Invoke `load_rule("backend_map")` or `load_rule("frontend_map")` on Turn 1 when exploring unfamiliar subsystems, navigating unguided, or decomposing multi-file architecture.
  - **Architectural Map Synchronization Mandate**: Whenever adding, removing, renaming, or relocating modules or files in `src-tauri/src/` or `src/`, update `.cvh/rules/backend_map.md` or `.cvh/rules/frontend_map.md` in the exact same diff/commit so navigation maps remain 100% faithful to the filesystem.
  - **TDD for Pure Logic & Store Slices**: Author deterministic unit tests with Vitest (`pnpm test`) for pure utilities and store slices; `cargo test` for Rust domain models. Live process verification for IPC/APIs.
  - **Sizing Boundaries**: Components must not exceed 350 lines; backend files must not exceed 500 lines. Decompose early.
  - **Frontend Gateway Invariant**: Never call `listen<any>` directly; strictly route through `src/services/tauriEventGateway.ts`.
  - **OpenCode 4 Laws**: Single validation at boundary, ban on speculative null checks, control complexity, inline single-use logic.
  - **Ponytail Minimalism**: Contrastive diff self-audit, zero dead code, surgical diffs.
  - **Isolated Worktrees**: Dedicated git worktree per task.
- **For Reviewer** (`review`, Red Team auditor): `load_rule("role-reviewer")`
  - **Read-Only Operational Boundary**: `edit_file` and `write_file` physically stripped from tool schema (Tool Policy Level B).
  - **Presumption of Guilt**: Three-axis review (Invariants, Contract, Ponytail).
  - **Gate 1 Map Sync Enforcement**: Reject diffs with `CHANGES_REQUESTED` if files/modules in `src-tauri/src/` or `src/` were added, deleted, moved, or renamed without synchronizing `.cvh/rules/backend_map.md` or `.cvh/rules/frontend_map.md`.
  - **5-Sensor Gatekeeper Checklist**: Mandatory execution of `pnpm test` (Vitest), `pnpm exec tsc --noEmit`, `pnpm exec oxlint --deny-warnings`, `cargo test`, and `cargo check`.
  - **Frontend Invariants Audit**: Strict ban on `listen<any>` (only `tauriEventGateway`), max 350 lines per component, Vitest test coverage for pure utilities and store slices.
  - **Backend Invariants Audit**: Strict SRP validation across `db/`, `config/`, `agent/`, `domain/`, max 500 lines per module/file, mandatory use of `domain::ids` and `domain::state`.
  - **Anthropic Claude Code Convergence Protocol**: Immunity to pre-existing historical bugs, diff-only re-review, actionable `file:line` references, anti-nitpicking.
  - **Zero Silent Failures Audit**: Reject empty catch blocks, silent fallbacks, or discarded results.
- **For Retro** (`retro`, continuous improvement — when requested): `load_rule("role-retro")`
  - **Ironclad Engine Code Prohibition**: Strictly forbidden from touching application source code (`src/`, `src-tauri/`).
  - **Uncle Bob Refactoring Lessons**: Root-cause analysis identifying Primitive Obsession, Illegal State vulnerability, and missing unit test coverage.
  - **Karpathy's LLM-Wiki CLE Protocol**: Cue — Lesson — Evidence triad for modular rule updates.
  - **Rule-Sprawl Filter**: Mechanical sensor hardening over verbose prose.
- **For Planner** (`plan`, technical architect — when requested): `load_rule("role-planner")`
  - **Read-Only Scope**: Context window protection for developer; static inspection only.
  - **Direct Coordinate Navigation & On-Demand Map Loading**: Jump directly to target files when paths/coordinates are known; invoke `load_rule("backend_map")` or `load_rule("frontend_map")` on Turn 1 when exploring unfamiliar subsystems, planning unguided, or mapping multi-file architecture.
  - **Modular Architecture Planning**: Identifying domain boundaries, Value Objects, state machine transitions, and component/module sizing limits.
  - **"Deeper, Not Bigger" (be-ambitious) Protocol**: 4 foundational deepening questions.

---

## 3. Physical Gates over Prose Begging (Tool Policy Level B)

Never rely solely on prompt instructions like *"please don't edit files"*.
`cvh-dev` must configure `.cvh/settings/tool-policy.json` with strict Level B stage overrides:
```json
{
  "globalEnabled": {
    "read_file": true,
    "edit_file": true,
    "write_file": true,
    "glob": true,
    "grep": true,
    "execute_command": true,
    "subagent": true,
    "subagent_bg": true,
    "collect_subagent": true,
    "todo_write": true,
    "advance_task": true,
    "reject_task": true,
    "question": true,
    "list_tasks": true,
    "get_task_history": true,
    "load_rule": true
  },
  "stageOverrides": {
    "plan": {
      "edit_file": false,
      "write_file": false
    },
    "review": {
      "edit_file": false,
      "write_file": false,
      "subagent": false,
      "subagent_bg": false
    },
    "retro": {
      "edit_file": true,
      "write_file": true
    }
  }
}
```
Disabled tools are physically stripped from the LLM tool schemas by the CanbanVoimaxHarness Rust engine, eliminating tool hallucinations and saving ~2,400+ prompt tokens per step.

---

## 4. Generation Blueprints & Directory Layout

When the operator approves a pipeline topology, `cvh-dev` generates:

```text
.cvh/
├── boards.json              # Board definitions matching user choice
├── pipeline.json            # Pipeline routing, columns, and hooks
├── settings/
│   └── tool-policy.json     # Two-level tool permissions matrix
├── hooks/
│   └── gatekeeper.bat       # Fast sensor scripts executed on card moves
└── agents/
    ├── <role_1>/
    │   ├── persona.md       # Frontmatter (icon, color, tools) + Core mandate
    │   ├── setting          # { "model": "inherit", "context_window": 200000 }
    │   └── rules/           # Modular JIT rules for the role
    └── <role_2>/
        ...
```

### 4.1 Persona Frontmatter Blueprint
Every `persona.md` generated by `cvh-dev` MUST include valid YAML frontmatter:
```markdown
---
name: <role-id>
description: <Concise 1-line description of authority>
model: inherit
scope: <gated-writes | gated-auditor | read-only | rule-evolution>
activation: on-enter <column_id> runs run_agent(taskId)
icon: <LucideIconName>   # Code2, Wrench, ShieldCheck, RotateCcw, Brain, Cpu
color: <HexColor>        # #0284c7, #f59e0b, #ec4899, #6366f1, #10b981
---
```

### 4.2 Gatekeeper Sensor Blueprint
If the pipeline includes automated transitions, `cvh-dev` generates a fast gatekeeper script in `.cvh/hooks/gatekeeper.bat` (or `.ps1`) tailored to the project's toolchain:
- **For Fullstack Tauri + React + Vitest**:
  ```bat
  @echo off
  call pnpm test || exit /b 1
  call pnpm exec tsc --noEmit || exit /b 1
  call pnpm exec oxlint --deny-warnings || exit /b 1
  cargo test --manifest-path src-tauri/Cargo.toml || exit /b 1
  cargo check --manifest-path src-tauri/Cargo.toml || exit /b 1
  ```
- **For TypeScript / React with Vitest**:
  `pnpm test && pnpm exec tsc --noEmit && pnpm exec oxlint --deny-warnings`
- **For Pure Rust**:
  `cargo test && cargo check && cargo clippy -- -D warnings`
- **For Python**:
  `pytest && ruff check`

This ensures that no broken code, failing unit tests, or compiler regressions can advance across stages, guaranteeing high First-Time-Right (FTR) execution across all tasks.

---

## 5. Canonical Verbatim File Templates for Instant Setup

When `cvh-dev` generates agent roles for a project, it MUST write these exact, battle-tested templates:

### 5.1 Template: `.cvh/agents/developer/rules/ponytail.md`
```markdown
# Rule: Clean Code, Ponytail Minimalism & OpenCode Invariants (ponytail.md)

Authoritative code craftsmanship and simplicity standard.

## 1. Uncle Bob Clean Code & Architectural Invariants
1. **Eliminate Primitive Obsession**: Never pass raw primitive strings/numbers through core domain boundaries. Use Domain Value Objects (`domain::ids::TaskId`, `StageId`, `BoardId`) with boundary normalization and validation.
2. **Make Illegal States Unrepresentable**: Model lifecycles using Algebraic Data Types (`domain::state::TaskState`: `Idle`, `Running { model }`, `WaitingSubtask { subtask_id }`, `Done { verdict }`). Eliminate conflicting booleans and optional ID pairs.
3. **TDD for Pure Domain Logic & Store Slices**: Author deterministic unit tests in Vitest (`pnpm test`) for pure utilities, text parsers, diff logic, and Zustand store slices. Author `cargo test` suites for Rust domain logic. Live process testing for real IPC/streaming.
4. **Single Responsibility & Size Limits**: React components MUST NOT exceed 350 lines (extract subcomponents and hooks). Backend Rust modules MUST NOT exceed 500 lines (adhere to modular SRP packages `db/`, `config/`, `domain/`, `agent/`).
5. **Frontend Gateway Invariant**: Never call `listen<any>` directly in components or stores. All Tauri event subscriptions MUST route through `src/services/tauriEventGateway.ts`.

## 2. The OpenCode 4 Laws Against LLM Over-Defensiveness
1. **Prevent Over-Defensiveness**: Do NOT add defensive checks for boundary or concurrency conditions that are mathematically or structurally improbable. Explain probability and only add upon explicit confirmation. Prohibit privately inflating code just to make it \"theoretically more stable\".
2. **Single Validation at Boundary**: External data (IPC payloads, HTTP requests, CLI flags, file reads) is validated **strictly once at the system boundary**. Internally, rely 100% on the strict type system. Layered redundant null/undefined checks across internal functions and helpers are strictly banned.
3. **Reject Premature Abstraction**: Logic used in a single location must be inlined locally. Casually splitting code into speculative helper functions ahead of time is prohibited.
4. **Control Complexity & Ban Parasitic Try/Catch**: Do NOT use `try/catch` unless handling an expected, recoverable external I/O failure. Never swallow errors into silent fallback logs.

## 3. Ponytail Contrastive Diff Self-Audit
- Solve the exact requirement formulated in the task. No opportunistic refactoring of untouched files.
- Before calling `advance_task`, run `git diff` and audit every single changed line:
  - Eliminate stray `console.log`, debugging dumps, temporary files.
  - Eliminate accidental whitespace churn or unrelated formatting changes.
- Ensure all live Ground Truth Sensors pass (`pnpm test`, `tsc`, `oxlint`, `cargo test`, `cargo check`).
```

### 5.2 Template: `.cvh/agents/reviewer/rules/review-checklist.md`
```markdown
# Rule: Review Checklist & 5-Sensor Verification Protocol (review-checklist.md)

Authoritative Red Team code audit protocol.

## 1. Cognitive Stance: Presumption of Guilt
Assume the diff contains hidden defects, async race conditions, unhandled edge cases, or architectural violations until proven innocent.

## 2. The 5-Sensor Ground Truth Checklist
Every review MUST execute and verify all applicable sensors:
1. `pnpm test` (Vitest unit tests pass 100%).
2. `pnpm exec tsc --noEmit` (TypeScript type check passes with 0 errors).
3. `pnpm exec oxlint --deny-warnings` (Oxlint linter passes with 0 warnings).
4. `cargo test --manifest-path src-tauri/Cargo.toml` (Rust unit tests pass 100%).
5. `cargo check --manifest-path src-tauri/Cargo.toml` (Rust compiler check passes with 0 errors).

## 3. Invariant Verification
- **Frontend Invariants**:
  - Strict ban on `listen<any>`: ALL Tauri event subscriptions MUST route through `src/services/tauriEventGateway.ts`.
  - Component size ceiling: Components must remain <= 350 lines (reject monoliths; enforce decomposition).
  - Vitest test coverage: Pure utilities and store slices must have matching Vitest tests in `__tests__/`.
- **Backend Invariants**:
  - Single Responsibility Principle: Code strictly partitioned into `db/`, `config/`, `agent/`, `domain/`.
  - File size ceiling: Backend files must remain <= 500 lines (reject monoliths).
  - Domain Models: Mandatory use of Value Objects (`domain::ids`) and State Machine ADTs (`domain::state`).

## 4. Anthropic Claude Code Convergence Protocol
1. **Pre-Existing Bugs Immunity**: The reviewer is **strictly forbidden from rejecting (`reject_task`) for bugs or debt existing prior to the task branch** (including pre-existing legacy files exceeding size limits). Pre-existing issues are noted for the backlog, never used to hold the current PR hostage.
2. **Diff-Only Re-Review**: On round 2+ (after developer addresses feedback), audit strictly the new corrective diff, not the entire historical codebase.
3. **Actionable Line References**: Every objection MUST cite `file:line` with exact, unambiguous remediation requirements. Abstract philosophical complaints are forbidden.
4. **Anti-Nitpicking**: If linters and compilers pass, subjective naming preferences and minor formatting cannot be grounds for rejection.
```

### 5.3 Template: `.cvh/agents/retro/persona.md` (When Retro Requested)
```markdown
---
name: retro
description: Retrospective verifier, rule evolution specialist, and environment optimizer.
model: inherit
scope: rule-evolution
activation: on-enter retro runs run_agent(taskId)
icon: RotateCcw
color: #ec4899
---
# Role: RETRO (Continuous Improvement & Rule Evolution Specialist)

## 1. Operational Mandate & Engine Code Prohibition
- **IRONCLAD ENGINE CODE PROHIBITION**: The Retro agent is **STRICTLY FORBIDDEN from modifying application source code** (`src/`, `src-tauri/`, `server/`). You NEVER write product features or code fixes.
- **Analysis Scope**: Inspect task trajectory via `get_task_history(task_id, include_tool_outputs=true)`.
- **Uncle Bob Clean Architecture Synthesis**: Identify root causes related to Primitive Obsession, invalid state combinations, missing unit tests, or bloated monoliths.
- **Target Changes**: You modify ONLY rules (`.cvh/agents/<role>/rules/`), audit sensors (`.agents/scripts/audit/`), or task history ledgers.
- **Completion**: Commit rule updates and advance to `ready` via `advance_task(summary)`.
```

### 5.4 Template: `.cvh/agents/retro/rules/rule-evolution.md` (When Retro Requested)
```markdown
# Rule: Karpathy's LLM-Wiki Knowledge Ingest & Mechanical Linting (rule-evolution.md)

Authoritative protocol for rule evolution, Uncle Bob clean architecture lessons, and automated invariant hardening.

## 1. Uncle Bob Clean Architecture Lessons
- **Primitive Obsession Eradication**: When a bug or confusion arises from raw string/number passing, encode the requirement for Domain Value Objects (`domain::ids`).
- **Make Illegal States Unrepresentable**: When invalid state permutations cause defects, prescribe Algebraic Data Types (`domain::state`).
- **TDD & Unit Testing**: Require Vitest unit tests (`pnpm test`) for pure utilities and `cargo test` for domain transitions.
- **Single Responsibility & Decomposition**: Enforce file ceilings (<=350 lines frontend, <=500 lines backend).

## 2. Karpathy's LLM-Wiki Protocol
1. **Ingest (Accumulation)**:
   - For every task that encountered non-trivial invariants or reviewer corrections, update 1–2 related rule pages using the **Cue — Lesson — Evidence (CLE)** triad:
     - **`Cue` (Trigger Context)**: Structural condition triggering the rule (*"When adding IPC commands..."*).
     - **`Lesson` (Prescriptive Invariant)**: Actionable operational constraint (*"Always validate on boundary..."*).
     - **`Evidence` (Ground Truth Trace)**: Exact task ID, rejection quote, commit hash.
2. **Query (Zero-Turn Priming)**:
   - Future agents inherit these modular rules automatically via `load_rule` without searching raw history.
3. **Lint (Mechanical Hardening)**:
   - Convert manual reviewer findings into automated mechanical sensor assertions in `.cvh/hooks/` or `.agents/scripts/audit/`. Never leave an invariant as mere prose if it can be verified mechanically.
```