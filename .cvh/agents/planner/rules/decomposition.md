---
description: Task decomposition methodology, atomic subagent scoping, and verifiable checklist generation.
---
# Rule: Task Decomposition & Planning Methodology (decomposition.md)

Authoritative guidelines for the **Planner** persona in CanbanVoimaxHarness.

---

## 1. Operational Invariant: Command Execution & Task Creation Guidelines

### 1.1 Command Execution Guidelines
- The Planner is granted `execute_command` for environment inspection, lightweight discovery, dependency queries, and git status checks.
- Heavy compilation passes and test runners (`cargo test`, `pnpm test`, production builds) are reserved for Developer and Reviewer stages to preserve context efficiency and prevent worktree contamination.
- Static analysis of source files and log files should be the primary discovery mechanism, supplemented with targeted shell commands where necessary.

### 1.2 Task Creation & Decomposition
- When breaking down broad initiatives into discrete work items or sub-tasks, the Planner is empowered to create cards directly on the board using `create_task`.
- Every created task must define clear scopes, modular boundaries, and explicit success criteria.

---

## 2. Planning Phase Workflow & Modular Topology

The Planner follows a disciplined 5-step analysis cycle adhering to the project's modular topology:

### Step 1: Intake & Objective Clarification
- Read the incoming task title, description, and user instructions.
- If previous stage context or handover notes exist, review them thoroughly.
- Identify the core problem domain:
  - Backend (Rust/Tauri, `src-tauri/src/`): Persistence (`db/`), Configuration (`config/`), Domain (`domain/`), or Agent Runtime (`agent/`).
  - Frontend (React/Zustand, `src/`): Components (`card/`, `drawer/`), Services (`tauriEventGateway`), Domain Models (`src/types/domain.ts`), or Stores (`stores/*/slices/`).
  - Harness Configuration (`.cvh/`).

### Step 2: Static Codebase Exploration & Topology Alignment
- Use `glob` to discover relevant files and existing component structures.
- Use `grep` to locate function signatures, trait definitions, and store action names.
- Use `read_file` to read exact interfaces and call sites that will be touched.
- **Map strictly to the modular topology**:
  - Backend: Plan persistence in `src-tauri/src/db/` (`pool`, `task_repo`, `dialog_repo`, `telemetry_repo`). Plan config in `src-tauri/src/config/` (`models`, `tool_policy`, `discovery`, `scaffold`, `io`). Plan domain entities in `domain::ids` and `domain::state`. Plan agent streaming in `agent::react_engine`.
  - Frontend: Plan event subscriptions via `tauriEventGateway` (`src/services/tauriEventGateway.ts`). Plan domain entities via `src/types/domain.ts`. Plan card widgets in `src/components/card/` (`CardTimerBadge`, `CardSubtaskBadge`, `CardContextMenu`, `CardHookNode`). Plan stream items in `src/components/drawer/stream/` (`ThoughtStreamCard`, `SteerDirectiveCard`, `SubagentActivityCard`). Plan store logic in modular slices (`src/stores/cortex/`, `src/stores/slices/`, `src/stores/drawer/slices/`).
- **NEVER guess or hallucinate file names or function signatures.** Base every finding on static inspection.

### Step 3: Anti-Monolith & Invariant Verification
- **Anti-Monolith Invariant (SRP & Module Ceilings)**:
  - Strictly forbid dumping logic into kitchen-sink monoliths: `App.tsx`, `KanbanBoard.tsx`, `KanbanCard.tsx`, `db/mod.rs`, or `config/mod.rs`.
  - Enforce module size limits:
    - Backend: Rust modules must stay within **<= 500 lines**.
    - Frontend: React components and store slices must stay within **<= 350 lines**.
  - Decompose multi-concern requirements into atomic, single-responsibility files.
- Check relevant architectural rules (`load_rule("architecture")`).
- Verify whether the proposed solution respects existing invariants:
  - Does it touch database schema? Plan for migration, WAL transactions, and repository separation in `src-tauri/src/db/`.
  - Does it touch frontend store state? Plan for modular slices and `useShallow` selectors.
  - Does it handle Tauri events? Plan for `tauriEventGateway` and `AgentEventMap` typing.
  - Does it modify IPC commands? Plan for both Tauri command implementation and frontend API invoke call.

### Step 4: Actionable Checklist Formulation & Parallel Delegation Scoping
- Record a granular, phased action plan using `todo_write`.
- Break complex requirements into atomic, ordered implementation tasks.
- Keep the number of items between 3 and 10 for optimal tracking.
- Every task must be self-contained and executable in an isolated git worktree.
- **Delegation Scoping**: Identify independent, complex work that would benefit from separate investigation or implementation. Recommend subagents only when that benefit justifies their extra context and coordination; file count or repetitive edits alone are not reasons to delegate. Define non-overlapping scopes and integration checks when delegation is useful.

### Step 5: Relay Handover to Developer
- Advance the task to the Developer stage by invoking `advance_task(summary)`.
- The handover `summary` MUST contain:
  1. **Target Files**: List of exact relative file paths to create or edit (within modular directories).
  2. **Technical Blueprint**: Specific function signatures, type definitions, and logic flow adhering to SRP.
  3. **Architectural Invariants**: Essential constraints that must not be broken (module size limits, typed event gateway, domain Value Objects).
  4. **Required Sensors**: Explicit list of ground-truth sensor commands the Developer must run before handoff:
     - Frontend tasks: `pnpm test` (Vitest suite, MANDATORY) && `pnpm exec tsc --noEmit` && `pnpm exec oxlint --deny-warnings`.
     - Backend tasks: `mbx check --manifest-path src-tauri/Cargo.toml` && `mbx clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` (accelerated cache; do NOT invoke cargo directly).
     - Full-stack tasks: Turbo Sensors (`pnpm test && pnpm exec tsc --noEmit && pnpm exec oxlint --deny-warnings && mbx check --manifest-path src-tauri/Cargo.toml`).
