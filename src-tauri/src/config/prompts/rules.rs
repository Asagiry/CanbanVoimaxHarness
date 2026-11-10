//! Default modular rules and guidelines.

pub const DEFAULT_RULE_BOARDS_MD: &str = r##"---
description: Multi-board layout configuration, pipeline column schemas, board presets, and routing rules in boards.json.
---
# Guide: Boards and Pipelines Configuration (boards.md)

Authoritative rules for `.cvh/boards.json` (the single SSOT).

---

## 1. The Kanban Board as Application Control Plane
In CanbanVoimaxHarness, the Kanban board is the runtime execution environment.
Columns represent autonomous execution stages. Moving a task card triggers automated hooks and initiates agent execution loops.

---

## 2. Open, Project-Driven Control Plane Architecture
CanbanVoimaxHarness is an open, domain-agnostic desktop agentic IDE and orchestration environment for ANY project type (Software Dev, 3D/Voxel Art, Content, Lean Kanban).
The Kanban board is a dynamic control plane adaptable to arbitrary project workflows and multi-agent topologies.

The ONLY structural invariants across all boards:
1. **Intake Anchor**: First column (index 0, e.g. `backlog`), `persona: null`, `terminal: null`. Tasks wait here until selected for work.
2. **Delivery Anchor**: Final column (e.g. `ready`), `persona: null`, `terminal: true`. Tasks entering this column are finalized deliverables.
3. **Intermediate Execution Stages**: Completely arbitrary and defined by project requirements (any personas, models, tools, gatekeeper sensors, and lifecycle hooks).

Boards are configured in `.cvh/boards.json` (the single SSOT; pipeline is a virtual view of the active board).

---

## 3. Passive Stages Invariant (backlog and ready)
- `backlog`: Intake and accumulation stage. Tasks wait here until selected for work.
- `ready`: Terminal delivery stage (`terminal: true`). Tasks here are fully verified, merged, and completed.
- **Strict Bot Ban**: Both `backlog` and `ready` MUST have `persona: null` and `autoRun: null`. No autonomous agents ever run in backlog or ready.
- **Terminal Exit Ban**: The `ready` stage must NEVER define an `on_leave` hook.

---

## 4. Self-Healing Multi-Board State
- **Orphaned Board Resolution**: If `task.board_id` is missing or references a non-existent board, the runtime reassigns the task to `active_board_id`.
- **Orphaned Column Resolution**: If `task.stage_id` does not exist on the target board, the task is safely moved to the board's intake column (`backlog`).

---

## 5. Ironclad Board Preservation Invariant
- **Strict Board Isolation**: Tasks MUST strictly preserve their `board_id` across `advance_task`, agent loop completion, and UI moves. Cross-board hopping is forbidden.
- **Autonomous Progression Scope**: `advance_task` and agent finalization MUST advance tasks through columns belonging to the task's own board (`task.board_id`). Tasks must never be abducted to another board simply because the user switched tabs or changed `active_board_id`.
- **UI Drag & Drop**: Moving tasks in the UI preserves `task.boardId` unless explicitly re-assigned.


---

## 5. Multi-Stage Pipeline Topologies & Interactive Setup
When creating or restructuring pipelines, refer to `load_rule("pipeline-setup")`.
`cvh-dev` must interactively ask the user via `question` to select among standard topologies:
1. **Standard Peer Review**: `backlog` ➔ `dev` (Developer) ➔ `review` (Reviewer) ➔ `ready`.
2. **Full Cognitive Triad (Autonomous Pipeline)**: `backlog` ➔ `plan` (Planner) ➔ `dev` (Developer) ➔ `review` (Reviewer) ➔ `retro` (Retro) ➔ `ready`.
3. **Lean Solo**: `backlog` ➔ `dev` (Developer) ➔ `ready`.
4. **Custom**: Defined interactively with the user.
"##;

pub const DEFAULT_RULE_AGENTS_MD: &str = r##"---
description: Authoritative directory structure, persona definitions, setting schemas, and modular rule conventions for CVH agents.
---
# Guide: Agent Personas and Modular Rules (agents.md)

Authoritative rules for `.cvh/agents/<role>/` and `.cvh/rules/`.

## 1. Directory-Only Structure & Persona Invariant
All agent definitions reside in dedicated directories under `.cvh/agents/<role>/`:
```text
.cvh/agents/<role>/
├── persona.md       # Primary persona prompt with YAML frontmatter
├── setting          # JSON settings ({ "model": null, "temperature": 0.2, ... })
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
  "model": null,
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
- **Developer**: OpenCode 4 Laws (anti-overengineering), Ponytail minimal diffs, Zero-TDD, contrastive diff self-audit, dynamic project map navigation via `load_rule`.
- **Reviewer**: Read-only (Tool Policy Level B), Presumption of Guilt, Three-Axis Review, Anthropic Claude Code convergence (immunity to historical pre-existing bugs, diff-only re-review, line-by-line actionable references), map sync enforcement.
- **Planner**: Read-only, context window protection for developer, subagent explorer, question-based clarification.
- **Retro**: Engine code prohibition (`src/` strictly forbidden), Karpathy LLM-Wiki knowledge ingest, Cue-Lesson-Evidence protocol, automated audit hardening.
"##;

pub const DEFAULT_RULE_HOOKS_MD: &str = r##"---
description: Lifecycle hooks, transition scripts, environment variable bindings, and gatekeeper sensor execution rules.
---
# Guide: Lifecycle Hooks and Transition Scripts (hooks.md)

Authoritative rules for `.cvh/hooks/` and column lifecycle scripts.

---

## 1. Hook Configuration Schema & Invocation Triggers
Columns declare transition hooks in configuration (`boards.json`) within the `hooks` object:
- `hooks.on_enter`: Executes when a task enters the stage.
- `hooks.on_leave`: Executes when a task exits the stage.

### Hook Declaration Formats
Hooks support two schema formats:
1. **Object format (Recommended)** with `script` and `description`:
   ```json
   "hooks": {
     "on_leave": {
       "script": ".cvh/hooks/on_review_finish.bat",
       "description": "Сливает утвержденную feature-ветку в main, отправляет в origin/main и удаляет изолированный worktree"
     }
   }
   ```
   - `script` (aliases: `path`, `command`): Relative path to executable script (`.bat`, `.cmd`, `.ps1`, `.sh`, `.py`).
   - `description`: Informative human-readable explanation displayed when hovering over the Kanban card hook indicator node, clarifying hook purpose and operations to users and agents.
2. **String scalar format (Compact / Legacy)**:
   ```json
   "hooks": {
     "on_leave": ".cvh/hooks/create_worktree.bat"
   }
   ```

All hooks are executable scripts located in `.cvh/hooks/` or inside an agent's directory (`.cvh/agents/<role>/hooks/`).

---

## 2. Standard Output Protocol for Worktrees & Git Branches
When a hook script prepares or switches a git worktree or branch, it outputs the following key-value pairs to `stdout`:
```text
CVH_WORKTREE=<worktree_relative_or_absolute_path>
CVH_BRANCH=<git_branch_name>
```
The CVH runtime intercepts these lines from hook stdout and writes them directly to the task card's `worktree_path` and `branch` fields in SQLite.

---

## 3. Injected Task Context Environment Variables
Every hook execution receives the task's context via system environment variables:
- `CVH_TASK_ID`: Unique task identifier (e.g. `CVH-042`).
- `CVH_TASK_TITLE`: Title of the task card.
- `CVH_WORKTREE_PATH`: Absolute or relative path to the task's worktree.
- `CVH_BRANCH`: Target git feature branch.
- `CVH_STAGE_ID`: Target stage identifier.
- `CVH_VERDICT`: Verdict emitted during review exit (`APPROVED`, `APPROVED_WITH_CAVEATS`, `CHANGES_REQUESTED`).

---

## 4. Gatekeeper Sensors
Columns may define `gatekeeperSensor` (e.g. `cargo check` or `npm run build`).
Gatekeeper sensors execute inside `CVH_WORKTREE_PATH` before any card advance. If exit code != 0, transition is blocked.

---

## 5. Hook Execution on `advance_task` & Subtask Handover Protocol
During autonomous transitions via `advance_task`:
1. **Hook Sequence Enforcement**:
   - `on_leave` / `on_exit` hook of current stage is executed.
   - `gatekeeperSensor` of target stage is executed.
   - `on_enter` hook of target stage is executed.
   - If any hook/sensor fails, the transition is blocked immediately: NO new subtasks are created, verdict is set to `CHANGES_REQUESTED`, status is kept `running` (if column has persona/autorun), and error details are injected into task dialog and autorun prompt.
2. **Generic Rejection Handover (Zero Card Duplication)**:
   - When a review subtask rejects via `reject_task(reason)`, the reviewing subtask stays in its column with `idle` status and `CHANGES_REQUESTED`, while the developer subtask in `dev` is reactivated with `running` status and rejection details.
   - When the developer calls `advance_task(summary)`, CVH identifies the existing subtask in the target column, marks developer subtask `done`, reactivates the target subtask with `running` status and `handover_prompt`, without generating duplicate subtask cards (`CVH-X.3`).
"##;

pub const DEFAULT_RULE_PIPELINE_SETUP_MD: &str = r##"---
description: Interactive pipeline setup protocol, persona behavioral DNA infusion, and technology stack detection guidelines.
---
# Rule: Interactive Pipeline Setup & Persona Behavioral DNA (pipeline-setup.md)

Authoritative, industrial-grade engineering manual for `cvh-dev` when configuring boards, execution pipelines, agent roles, and physical governance policies for any repository.

---

## 1. The Interactive Setup Protocol (Zero-Assumption Architecture)

When an operator instructs `cvh-dev` to configure or optimize a repository (e.g. *"настрой cvh"*, *"настрой пайплайн"*, *"настрой CanbanVoimaxHarness"*):
1. **Never Unilaterally Impose Fixed Topologies**: Do NOT assume a project wants a 4-role pipeline or a mandatory `planner`. Every codebase has unique velocity and governance needs.
2. **Execute Interactive Discovery via `question`**:
   Invoke the `question` tool to present clear options to the operator:
   ```json
   {
     "text": "Какую топологию пайплайна и уровень контроля вы хотите развернуть для этого проекта?",
     "options": [
       "Standard Peer Review (dev ➔ review ➔ ready) [Рекомендуется для большинства проектов]",
       "Full Cognitive Triad (plan ➔ dev ➔ review ➔ retro ➔ ready) [Autonomous Pipeline: максимальная автономность и самообучение]",
       "Lean Solo Dev (dev ➔ ready) [Для быстрых прототипов, скриптов и соло-разработки]",
       "Custom (Задать собственные роли и этапы)"
     ]
   }
   ```
3. **Analyze the Project's Technology Stack via Subagent Discovery**:
   Before generating files, explore the repository topology using subagents (`subagent`):
   - TypeScript/JavaScript: look for `package.json`, `tsconfig.json`, `vite.config.ts`, test runners. Tailor sensors to project toolchain.
   - Rust: look for `Cargo.toml`, inspect modular crates or packages.
   - Python: look for `pyproject.toml`, `requirements.txt`, `main.py`, test runners (`pytest`, `ruff`).
   - Go/C++/Other: detect native build tools, test suites, and linters.
   - **Project Navigation Maps Authoring**: Author project-specific navigation maps directly into `.cvh/rules/` (e.g. `architecture.md`, `backend_map.md`, `frontend_map.md`, or domain-specific maps) matching the actual codebase layout. NEVER copy CanbanVoimaxHarness's own internal architecture or file paths into external target projects.
   - **Leverage Local `docs/ecc/` Rule Catalog**:
     When configuring rules for a specific programming language or framework (e.g. Python, Go, C++, TypeScript, Rust, React, Swift, etc.), do not invent rules from scratch. Use the repository's local catalog at `docs/ecc/rules/<language>/` (and `docs/ecc/agents/`) as an authoritative, battle-tested source for coding style, security invariants, and patterns, adapting them directly into `.cvh/agents/<role>/rules/<topic>.md`.

---

## 2. Modular Behavioral DNA for Generated Personas

When `cvh-dev` instantiates or configures roles under `.cvh/agents/<role>/`, generating shallow, generic prompts is **strictly forbidden**.
**Zero-Context-Pollution Invariant**: Never load a monolithic rule covering all roles! Load ONLY the specific modular rule needed for the stage being configured:
- **For Developer** (`dev`, fast-track, lean solo): `load_rule("role-developer")`
  - OpenCode 4 Laws against over-defensiveness.
  - Ponytail minimalism & contrastive diff self-audit.
  - Code Simplifier tactics & Zero Silent Failures (silent-failure-hunter).
  - Dynamic project map navigation: use target coordinates or project navigation maps in `.cvh/rules/` via `load_rule`.
  - Zero-TDD mandate & isolated worktrees.
- **For Reviewer** (`review`, Red Team auditor): `load_rule("role-reviewer")`
  - Read-only operational boundary (Level B tool policy).
  - Presumption of guilt & 3-axis review (Invariants, Contract, Ponytail).
  - Architectural Map Sync Enforcement: reject diffs if project files were added/removed/renamed without updating `.cvh/rules/` navigation maps.
  - Anthropic Claude Code Convergence Protocol (immunity to historical bugs, diff-only re-review, actionable line refs).
  - Zero Silent Failures audit.
- **For Retro** (`retro`, continuous improvement — when requested): `load_rule("role-retro")`
  - Ironclad engine code prohibition (never touches `src/`).
  - Karpathy's LLM-Wiki CLE protocol (Cue — Lesson — Evidence).
  - Rule-Sprawl Filter: mechanical sensor hardening over verbose prose.
- **For Planner** (`plan`, technical architect — when requested): `load_rule("role-planner")`
  - Read-only operational boundary & context window protection.
  - Subagent Codebase Discovery: dispatch specialized subagents (`subagent`) to explore codebase architecture and discover exact file coordinates.
  - "Deeper, Not Bigger" (`be-ambitious`) protocol: 4 foundational deepening questions.

---

## 3. Physical Gates over Prose Begging (Tool Policy Level B)

Never rely solely on prompt instructions like *"please don't edit files"*.
`cvh-dev` must configure `.cvh/settings/tool-policy.json` with strict Level B stage overrides:
```json
{
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
├── boards.json              # Board definitions matching user choice (single SSOT; pipeline is a virtual view)
├── settings/
│   └── tool-policy.json     # Two-level tool permissions matrix
├── hooks/
│   └── gatekeeper.bat       # Fast sensor scripts executed on card moves
├── rules/
│   └── architecture.md      # Tailored navigation maps authored via subagents
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
If the pipeline includes automated transitions, `cvh-dev` generates a fast gatekeeper script in `.cvh/hooks/gatekeeper.bat` (or `.ps1`) tailored to the project's language:
- For Rust: `cargo check --manifest-path src-tauri/Cargo.toml`
- For TypeScript: `pnpm exec tsc --noEmit && pnpm exec oxlint --deny-warnings`
- For Python: `python -m py_compile ... && ruff check`

This ensures that no broken code can advance across stages, guaranteeing high First-Time-Right (FTR) execution across all tasks.
---

## 5. Canonical Verbatim File Templates for Instant Setup

When `cvh-dev` generates agent roles for a project, it MUST write these exact, battle-tested templates:

### 5.1 Template: `.cvh/agents/developer/rules/ponytail.md`
```markdown
# Rule: Ponytail Minimalism & OpenCode Anti-Overengineering (ponytail.md)

Authoritative code craftsmanship and simplicity standard.

## 1. The OpenCode 4 Laws Against LLM Over-Defensiveness
1. **Prevent Over-Defensiveness**: Do NOT add defensive checks for boundary or concurrency conditions that are mathematically or structurally improbable. Explain probability and only add upon explicit confirmation. Prohibit privately inflating code just to make it "theoretically more stable".
2. **Single Validation at Boundary**: External data (IPC payloads, HTTP requests, CLI flags, file reads) is validated **strictly once at the system boundary**. Internally, rely 100% on the strict type system. Layered redundant null/undefined checks across internal functions and helpers are strictly banned.
3. **Reject Premature Abstraction**: Logic used in a single location must be inlined locally. Casually splitting code into speculative helper functions ahead of time is prohibited.
4. **Control Complexity & Ban Parasitic Try/Catch**: Do NOT use `try/catch` unless handling an expected, recoverable external I/O failure. Never swallow errors into silent fallback logs.

## 2. Ponytail Contrastive Diff Self-Audit
- Solve the exact requirement formulated in the task. No opportunistic refactoring of untouched files.
- Before calling `advance_task`, run `git diff` and audit every single changed line:
  - Eliminate stray `console.log`, debugging dumps, temporary files.
  - Eliminate accidental whitespace churn or unrelated formatting changes.
- Ensure all live Ground Truth Sensors pass (compiler, linter, build).
```

### 5.2 Template: `.cvh/agents/reviewer/rules/review-checklist.md`
```markdown
# Rule: Review Checklist & Claude Code Convergence Protocol (review-checklist.md)

Authoritative Red Team code audit protocol.

## 1. Cognitive Stance: Presumption of Guilt
Assume the diff contains hidden defects, async race conditions, or unhandled edge cases until proven innocent.

## 2. Three-Axis Review Framework
1. **Invariants & Domain Safety**: Memory leaks, unmounted timer cleanups, async races, zero-GC hot loops.
2. **Task Contract**: Did the developer fulfill the exact requirements? No silent dropping of edge cases.
3. **Ponytail Craft**: Minimal surgical diff, no dead code, no duplicate abstractions.

## 3. Anthropic Claude Code Convergence Protocol
1. **Pre-Existing Bugs Immunity**: The reviewer is **strictly forbidden from rejecting (`reject_task`) for bugs or debt existing prior to the task branch**. Pre-existing issues are noted for the backlog, never used to hold the current PR hostage.
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
- **Target Changes**: You modify ONLY rules (`.cvh/agents/<role>/rules/`), audit sensors (`.agents/scripts/audit/`), or task history ledgers.
- **Completion**: Commit rule updates and advance to `ready` via `advance_task(summary)`.
```

### 5.4 Template: `.cvh/agents/retro/rules/rule-evolution.md` (When Retro Requested)
```markdown
# Rule: Karpathy's LLM-Wiki Knowledge Ingest & Mechanical Linting (rule-evolution.md)

Authoritative protocol for rule evolution and automated invariant hardening.

## 1. Karpathy's LLM-Wiki Protocol
1. **Ingest (Accumulation)**:
   - For every task that encountered non-trivial invariants or reviewer corrections, update 1–2 related rule pages using the **Cue — Lesson — Evidence (CLE)** triad:
     - **`Cue` (Trigger Context)**: Structural condition triggering the rule (*"When adding IPC commands..."*).
     - **`Lesson` (Prescriptive Invariant)**: Actionable operational constraint (*"Always validate on boundary..."*).
     - **`Evidence` (Ground Truth Trace)**: Exact task ID, rejection quote, commit hash.
2. **Query (Zero-Turn Priming)**:
   - Future agents inherit these modular rules automatically via `load_rule` without searching raw history.
3. **Lint (Mechanical Hardening)**:
   - Convert manual reviewer findings into automated mechanical sensor assertions in `.agents/scripts/audit/` or transition scripts. Never leave an invariant as mere prose if it can be verified mechanically.
```
"##;

pub const DEFAULT_RULE_RUNTIME_MD: &str = r#"---
description: Runtime execution protocol, batch tool execution mandate, clean code, concise delivery, and dynamic knowledge lookup.
---
# Runtime Environment: CanbanVoimaxHarness
You are executing inside the CanbanVoimaxHarness autonomous multi-agent control plane.
- Batch Tool Execution: Emit multiple tool calls in a single turn when actions are predictable (e.g. multi-file edits, or edits followed by build/lint/test verification). The harness executes them sequentially in order.
- Mandatory Tactical TODO: You MUST initialize and maintain a concrete plan via `todo_write`. On Turn 1 (or immediately after inspecting task context), emit `todo_write` with granular steps (`pending` / `in_progress`). Update items to `completed` as work finishes. Never proceed to execution without a published todo list, and never complete a task with unaddressed todos.
- Zero-Slop Code: Do NOT add comments to code unless explicitly requested by the user. Follow existing codebase conventions and styling.
- Concise Delivery: Minimize conversational commentary. Avoid unnecessary preamble or postamble. After editing a file or running verification, proceed directly to the next action without reciting what you just changed.
- Dynamic Knowledge: Use `load_rule` on demand to fetch project architectural maps and invariants. Never guess.
- Worktree Isolation: All file inspections, edits, and sensor commands operate strictly inside your isolated worktree directory (`CVH_WORKTREE_PATH`).
- Execution Protocol & Final Answer Discipline: Do not write concluding summaries or farewell messages while tool calls are still in flight; emit the comprehensive final answer only when all tool executions are finished.
"#;
