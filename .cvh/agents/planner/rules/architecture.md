---
description: System architectural topology, subsystem boundaries, data flow invariants, and module ceiling planning.
---
# Rule: System Architecture & Subsystem Invariants (architecture.md)

Authoritative architectural reference for the **Planner** persona in CanbanVoimaxHarness.

---

## 1. System Topology Overview

CanbanVoimaxHarness (CVH) is an autonomous software development harness structured across three core layers:

1. **Backend Control Plane (Rust 2021 + Tauri v2)**:
   - Modular domain architecture partitioned across specialized repositories, services, and domain models.
   - Persistence layer: SQLite with Write-Ahead Logging (WAL) in `.cvh/harness.db` via `src-tauri/src/db/`.
   - Configuration layer: Modular harness settings, model registries, and tool policy matrix via `src-tauri/src/config/`.
   - Domain layer: Algebraic Data Types and nominal branded types via `src-tauri/src/domain/`.
   - Agent runtime: Unified ReAct streaming and telemetry via `agent::react_engine` (`ReActEngine<Driver>`).
   - IPC bridge: Strongly-typed Tauri commands and centralized event dispatch.

2. **Frontend UI Cockpit (React 19 + Tailwind v4 + Zustand 5)**:
   - High-density Kanban board, Agent Console Drawer, Git diff inspector, Cortex Mind Explorer.
   - Centralized typed event gateway: `tauriEventGateway` (`src/services/tauriEventGateway.ts`) replacing raw `listen<any>`.
   - Domain layer: Branded nominal Value Objects and state machines in `src/types/domain.ts`.
   - Modular UI architecture: Specialized card widgets (`card/`), stream cards (`drawer/stream/`), and token parsers (`markdown/`).
   - State management: Multi-slice Zustand stores (`cortex/slices/`, `board/slices/`, `drawer/slices/`) subscribing via `useShallow`.
   - Dual-theme design system: Vercel Light Paper (`#fafafa`) and Obsidian Dark (`#090a0f`).

3. **Autonomous Execution Subsystem (Git Worktrees & Hooks)**:
   - Each task executes in an isolated Git worktree: `worktrees/<task-slug>` on feature branch `feat/<task-slug>`.
   - Stage transitions execute automated lifecycle hooks (.bat, .sh, .py, .ps1) with context variables (`CVH_TASK_ID`, `CVH_WORKTREE_PATH`, `CVH_STAGE_ID`).

---

## 2. Core Subsystems & Key File Targets

When planning technical solutions, identify affected subsystems and map them strictly to the modular topology:

| Subsystem | Primary Files / Modules | Architectural Responsibilities |
|---|---|---|
| **Persistence (SQLite)** | `src-tauri/src/db/pool.rs`, `task_repo.rs`, `dialog_repo.rs`, `telemetry_repo.rs` | WAL connection pool, task CRUD, subtask cascades, dialog activity feed, telemetry & task ledger. (Monolithic `db.rs` is eradicated). |
| **Config & Scaffolding** | `src-tauri/src/config/models.rs`, `tool_policy.rs`, `discovery.rs`, `scaffold.rs`, `io.rs` | LLM models, two-level tool policy matrix, workspace root discovery, project scaffolding, atomic JSON I/O. (Monolithic `config.rs` is eradicated). |
| **Domain Layer (Rust)** | `src-tauri/src/domain/ids.rs`, `state.rs` | Nominal Value Objects (`TaskId`, `StageId`, `BoardId`) and `TaskState` ADT state machine. |
| **Agent Engine** | `src-tauri/src/agent/react_engine.rs`, `compactor.rs`, `tool_schemas.rs`, `tools/*` | `ReActEngine<Driver>`, unified SSE streaming, telemetry (TTFT, TPS, Cache Hit Rate), context compactor, modular tool execution. |
| **Task Commands** | `src-tauri/src/commands/tasks/healing.rs`, `lifecycle.rs`, `runner.rs`, `storage.rs`, `transitions.rs` | Self-healing boards, task lifecycle, agent runner dispatch, stage transitions. |
| **Worktree & Git** | `src-tauri/src/git.rs`, `commands/git.rs` | Native worktree creation, diff generation, branch merging to `main`, two-step worktree pruning. |
| **Hooks Engine** | `src-tauri/src/hooks.rs` | Transition scripts runner, Windows argument escaping, timeout enforcement, gatekeeper sensor checks. |
| **Frontend Event Gateway** | `src/services/tauriEventGateway.ts` | Centralized typed event listeners (`AgentEventMap`), 150ms streaming micro-batching. (Strict ban on `listen<any>`). |
| **Frontend Domain Layer** | `src/types/domain.ts` | Value Objects (`TaskId`, `StageId`, `BoardId`), factories (`toTaskId`), state machine (`TaskState`, `TaskStates`). |
| **Frontend Card Subsystem** | `src/components/card/CardTimerBadge.tsx`, `CardSubtaskBadge.tsx`, `CardContextMenu.tsx`, `CardHookNode.tsx`, `KanbanCard.tsx` | Modular card widgets and layout coordinator. (Card widgets <= 250–350 lines). |
| **Frontend Drawer Streams** | `src/components/drawer/stream/ThoughtStreamCard.tsx`, `SteerDirectiveCard.tsx`, `SubagentActivityCard.tsx` | Modular thought, steer directive, and subagent stream cards. |
| **Frontend Markdown** | `src/lib/markdown/parseMarkdownTokens.ts` | Decoupled Markdown token parsing and syntax highlighting tokens. |
| **Frontend State Slices** | `src/stores/cortex/slices/*`, `src/stores/slices/*` (board), `src/stores/drawer/slices/*` | Modular Zustand state slices composed into unified stores. |

---

## 3. High-Level Invariants to Plan For

1. **Anti-Monolith & Module Size Limits**:
   - **Backend Module Ceiling**: Maximum **350–500 lines** per Rust file.
   - **Frontend Component Ceiling**: Maximum **250–350 lines** per React component.
   - **No Kitchen-Sink Dumps**: Strictly prohibit planning additions into `App.tsx`, `KanbanBoard.tsx`, `KanbanCard.tsx`, `db/mod.rs`, or `config/mod.rs`.
2. **Centralized Event Gateway Invariant**: Plan all Tauri event handling through `tauriEventGateway`. Raw `listen<any>` in React components is strictly forbidden.
3. **Domain Layer Value Objects**: Always plan use of strongly typed Value Objects from `src/types/domain.ts` and `src-tauri/src/domain/`. Validate once at the boundary, trust internally.
4. **Single-Instance Plugin Order**: `tauri-plugin-single-instance` MUST always remain the first initialized plugin in `src-tauri/src/lib.rs`.
5. **WebView2 UDF Path**: WebView2 User Data Folder must reside in verified writeable `%LOCALAPPDATA%` paths, never inside `.cvh/`.
6. **SQLite WAL Transactions**: All multi-step database mutations must be planned inside explicit transaction blocks with immediate locks in `src-tauri/src/db/`.
7. **Zustand `useShallow`**: Any frontend store subscription extracting object or array slices must use `useShallow`.
8. **Ponytail Minimalism**: Solutions must minimize diff footprint within modular boundaries—no speculative rewrites, zero unwarranted dependencies.
9. **Mandatory Sensors in Handover**: Always specify the required Ground Truth sensors for the Developer:
   - Frontend: Vitest suite (`pnpm test`), TypeScript check (`pnpm exec tsc --noEmit`), linter (`pnpm exec oxlint --deny-warnings`).
   - Backend: `cargo check --manifest-path src-tauri/Cargo.toml` and `cargo clippy -- -D warnings`.
   - Full-stack: Turbo Sensors (`pnpm test && pnpm exec tsc --noEmit && pnpm exec oxlint --deny-warnings && cargo check`).
