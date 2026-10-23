# Rule: System Architecture & Subsystem Invariants (architecture.md)

Authoritative architectural reference for the **Planner** persona in CanbanVoimaxHarness.

---

## 1. System Topology Overview

CanbanVoimaxHarness (CVH) is an autonomous software development harness structured across three core layers:

1. **Backend Control Plane (Rust 2021 + Tauri v2)**:
   - Orchestration runtime, ReAct agent loop, tool execution engine, Git worktree lifecycle, hook dispatchers.
   - Persistence layer: SQLite with Write-Ahead Logging (WAL) in `.cvh/harness.db`.
   - IPC bridge: Strongly-typed Tauri commands and real-time WebSocket/Tauri events.

2. **Frontend UI Cockpit (React 19 + Tailwind v4 + Zustand 5)**:
   - High-density Kanban board, Agent Console Drawer, Git diff inspector, Cortex Mind Explorer.
   - State management: Atomic Zustand stores subscribing via `useShallow` to prevent unnecessary re-renders.
   - Dual-theme design system: Vercel Light Paper (`#fafafa`) and Obsidian Dark (`#090a0f`).

3. **Autonomous Execution Subsystem (Git Worktrees & Hooks)**:
   - Each task executes in an isolated Git worktree: `worktrees/<task-slug>` on feature branch `feat/<task-slug>`.
   - Stage transitions execute automated lifecycle hooks (.bat, .sh, .py, .ps1) with environment variables (`CVH_TASK_ID`, `CVH_WORKTREE_PATH`, `CVH_STAGE_ID`).

---

## 2. Core Subsystems & Key File Targets

When planning technical solutions, identify affected subsystems and target files:

| Subsystem | Primary Files | Architectural Responsibilities |
|---|---|---|
| **Agent Engine** | `src-tauri/src/agent.rs`, `agent/compactor.rs` | ReAct loop, tool registry, token management, context compaction, subagent dispatch. |
| **Kanban & Pipeline** | `src-tauri/src/commands/boards.rs`, `tasks.rs`, `config.rs` | Column normalization, board CRUD, stage routing, task state transitions. |
| **Persistence (SQLite)** | `src-tauri/src/db.rs` | SQLite connection pooling, WAL mode, transaction safety, task records, dialog history. |
| **Worktree & Git** | `src-tauri/src/git.rs`, `commands/git.rs` | Worktree creation, diff generation, branch merging to `main`, cleanup. |
| **Hooks Engine** | `src-tauri/src/hooks.rs` | Process spawning, Windows argument escaping, timeout enforcement, sensor checks. |
| **Frontend Board** | `src/components/board/KanbanBoard.tsx`, `TaskCard.tsx` | Task card drag-and-drop, stage columns, subtask hierarchy. |
| **Frontend Drawer** | `src/components/drawer/AgentConsoleDrawer.tsx` | Dialogue history, streaming agent thoughts, tool execution cards, user steer. |
| **Frontend State** | `src/stores/taskStore.ts`, `boardStore.ts`, `cortexStore.ts`, `toolPolicyStore.ts` | Reactive state, optimistic updates, IPC synchronization. |

---

## 3. High-Level Invariants to Plan For

1. **Single-Instance Plugin Order**: `tauri-plugin-single-instance` MUST always remain the first initialized plugin in `src-tauri/src/lib.rs`.
2. **WebView2 UDF Path**: WebView2 User Data Folder must reside in verified writeable `%LOCALAPPDATA%` paths, never inside `.cvh/`.
3. **SQLite WAL Transactions**: All multi-step database mutations must be planned inside explicit transaction blocks with immediate locks.
4. **Zustand `useShallow`**: Any frontend store subscription extracting object or array slices must use `useShallow`.
5. **Ponytail Minimalism**: Solutions must minimize diff footprint—no speculative rewrites, zero unwarranted dependencies.
