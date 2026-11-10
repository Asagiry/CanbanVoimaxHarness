---
description: System architectural topology, decomposed backend/frontend subsystems, and strict line ceiling invariants.
---
# System Architecture & Subsystem Invariants (architecture.md)

Authoritative architectural reference and module topology for **CanbanVoimaxHarness (CVH)**.

---

## 1. System Topology Overview

CanbanVoimaxHarness operates as a high-velocity, deterministic multi-agent development environment organized across three tiers:

1. **Backend Control Plane (Rust 2021 + Tauri v2)**:
   - Modular domain architecture partitioned into single-responsibility subpackages under `src-tauri/src/`.
   - Persistence layer: R2D2 SQLite pool with Write-Ahead Logging (WAL) in `.cvh/harness.db` via `db/`.
   - Configuration layer: Modular harness settings, provider catalog, and tool policy matrix via `config/`.
   - Domain layer: Algebraic Data Types (`TaskState`) and nominal branded types (`TaskId`, `StageId`, `BoardId`) via `domain/`.
   - Agent runtime: Decomposed ReAct streaming engine (`agent/react_engine/`), state-preserving compactor (`agent/compactor/`), and specialized tool handlers (`agent/tools/`).
   - Desktop platform & IPC: Strongly typed Tauri command handlers under `commands/`, WinRT notifications, and WebView2 UDF isolation.

2. **Frontend Tactical Cockpit (React 19 + Tailwind v4 + Zustand 5)**:
   - High-density Kanban board, Agent Console Drawer, Git diff inspector, and Cortex Mind Explorer.
   - Centralized typed event gateway: `src/services/gateway/` via `tauriEventGateway.ts` (strict ban on raw `listen<any>`).
   - Domain layer: Nominal branded types and state predicates in `src/types/domain.ts`.
   - Modular UI architecture: Cohesive card widgets (`components/card/`), stream cards (`components/drawer/stream/`), subagent cards (`components/drawer/tools/subagent/`), and AST token parser (`lib/markdown/`).
   - State management: Composed Zustand stores with atomic slices (`stores/slices/tasks/`, `stores/drawer/slices/`, `stores/cortex/`).

3. **Autonomous Execution Subsystem (Native Git Worktrees & Hooks)**:
   - Each task executes in an isolated native Git worktree: `worktrees/<task-slug>` on feature branch `feat/<task-slug>`.
   - Stage transitions execute automated lifecycle hooks (`.bat`, `.sh`, `.ps1`) with context environment variables (`CVH_TASK_ID`, `CVH_WORKTREE_PATH`, `CVH_STAGE_ID`, etc.).

---

## 2. Decomposed Backend Subsystems (`src-tauri/src/`)

All historical backend monoliths were refactored in CVH-069 into cohesive subpackages with lightweight facades:

| Subsystem | Modules & Submodules | Architectural Responsibilities |
|---|---|---|
| **Agent Core** | `agent.rs` (facade), `agent/types.rs`, `agent/prompts.rs`, `agent/spill.rs`, `agent/command_runner.rs`, `agent/steering.rs`, `agent/thinking_parser.rs`, `agent/engine_context.rs`, `agent/engine_tools.rs`, `agent/engine.rs`, `agent/rule_tools.rs`, `agent/session_registry.rs`, `agent/tool_schemas.rs` | Prompt assembly, ReAct turn execution, command spawning, steer injection, thinking block parsing, subagent sessions, and tool schema generation. |
| **Subagent Runtime** | `agent/subagent_runtime.rs`, `subagent_setup.rs`, `subagent_turn.rs`, `subagent_synth.rs`, `subagent_exec.rs`, `subagent_retry.rs` | Isolated in-process subagent execution, retry loops, result synthesis, and background session tracking. |
| **Context Compactor** | `agent/compactor.rs` (facade), `agent/compactor/tokens.rs`, `extractor.rs`, `evaporation.rs`, `multi_tier.rs`, `orchestrator.rs`, `report.rs` | Token estimation, typed tool call summary extraction, stale output evaporation (>500 chars older than 2 turns), multi-tier compaction passes. |
| **ReAct Engine** | `agent/react_engine.rs` (facade), `agent/react_engine/engine.rs`, `drivers.rs`, `stream_consumer.rs` | Generic `ReActEngine<Driver>` loop, streaming SSE chunk consumption, and active generation telemetry. |
| **Text Matcher** | `agent/text_match.rs` (facade), `agent/text_match/diff_mismatch.rs`, `replace.rs` | Resilient line similarity matching, fuzzy text replacement, and diff mismatch reporting. |
| **Agent Tools** | `agent/tools/mod.rs`, `command.rs`, `file_ops.rs`, `pipeline.rs`, `planning.rs`, `rules.rs`, `web.rs`<br>• `agent/tools/subagent/` (`background.rs`, `collect.rs`, `guards.rs`, `list.rs`, `run.rs`)<br>• `agent/tools/tasks/` (`complete.rs`, `create.rs`, `params.rs`, `query.rs`) | Individual tool execution handlers with permission checks, argument validation, and output spilling. |
| **Configuration** | `config/mod.rs`, `models.rs`, `providers.rs`, `tool_policy.rs`, `discovery.rs`, `scaffold.rs`, `io.rs`, `pipeline.rs`, `projects.rs`, `agent_settings.rs`<br>• `config/prompts/` (`mod.rs`, `roles.rs`, `rules.rs`) | Model configurations, LLM provider catalogs, single-authoritative tool policy matrix, project root discovery, prompt templates. |
| **Domain Layer** | `domain/mod.rs`, `domain/ids.rs`, `domain/state.rs`, `domain/state_serde.rs` | Nominal branded Value Objects (`TaskId`, `StageId`, `BoardId`), `TaskState` ADT, and custom Serde logic. |
| **Persistence (SQLite)** | `db/mod.rs`, `pool.rs`, `migrations.rs`, `context_repo.rs`, `telemetry_repo.rs`<br>• `db/task_repo/` (`delete.rs`, `sequence.rs`, `task_query.rs`, `todos.rs`, `upsert.rs`)<br>• `db/dialog_repo/` (`dialog_query.rs`, `mutation.rs`, `tools.rs`) | R2D2 connection pooling in WAL mode, monotonic task IDs, subtask cascades, dialogue persistence, context snapshots, and telemetry. |
| **Task Lifecycle & Transitions** | `commands/tasks.rs`, `commands/tasks/storage.rs`, `commands/tasks/healing.rs`, `commands/tasks/timing.rs`<br>• `commands/tasks/transitions/` (`advance.rs`, `advance_child.rs`, `move_stage.rs`, `reorder.rs`, `resolve.rs`, `subtasks.rs`)<br>• `commands/tasks/transition_hooks/` (`helpers.rs`, `pipeline.rs`, `pipeline_enter.rs`, `pipeline_leave.rs`, `scripts.rs`)<br>• `commands/tasks/lifecycle/` (`create.rs`, `delete.rs`, `hook_records.rs`, `query.rs`, `subtasks.rs`, `update.rs`)<br>• `commands/tasks/runner/` (`abort.rs`, `execution.rs`, `execution_events.rs`, `model_resolve.rs`, `steer.rs`) | Task CRUD, atomic stage moves, transition pipeline execution, sensor hooks, agent task runner, and run aborts. |
| **Commands & System** | `commands/mod.rs`, `state.rs`, `boards.rs`, `cortex.rs`, `llm.rs`, `notifications.rs`, `projects.rs`, `tools.rs`, `cvh_git.rs`<br>• `commands/personas/` (`crud.rs`, `meta.rs`, `mod.rs`)<br>• `commands/dialogs/` (`activity.rs`, `subagents.rs`)<br>• `commands/git/` (`commands.rs`, `diff_parser.rs`, `worktree.rs`) | IPC invoke handlers, multi-board management, persona editors, dialog queries, and native Git operations. |
| **Hook Execution** | `hooks.rs` (facade), `hooks/process.rs`, `hooks/runner.rs`, `hooks/sensor.rs` | Windows command-line quoting, process tree kill (`taskkill /F /T`), and sensor execution. |
| **LLM Streaming Client** | `llm/mod.rs`, `client.rs` (facade), `usage.rs`, `responses_event.rs`, `cache.rs`, `models.rs` | Universal SSE streaming client, retry backoffs, token usage tracking, and prefix caching diagnostics. |

---

## 3. Decomposed Frontend Subsystems (`src/`)

All historical frontend monoliths were refactored in CVH-069 into cohesive subcomponents, custom hooks, and store slices:

| Subsystem | Modules & Submodules | Architectural Responsibilities |
|---|---|---|
| **Event Gateway** | `src/services/tauriEventGateway.ts` (facade)<br>• `src/services/gateway/` (`types.ts`, `textBuffer.ts`, `helpers.ts`, `agentEventRouter.ts`, `taskEventRouter.ts`, `index.ts`) | Single entrypoint for all Tauri event subscriptions; streaming micro-batching, event routing, and text buffer ring. |
| **Task Stores & CRUD** | `src/stores/slices/tasksCrudSlice.ts` (facade)<br>• `src/stores/slices/tasks/` (`tasksCoreSlice.ts`, `tasksCreateSlice.ts`, `tasksMoveSlice.ts`, `tasksMutateSlice.ts`, `index.ts`) | Atomic Zustand task state slices: query, creation, stage movement, and mutation. |
| **Kanban Board** | `src/components/board/KanbanBoard.tsx`<br>• Hooks: `useBoardDragDrop.ts`, `useBoardTaskFilter.ts`<br>• Columns: `KanbanColumn.tsx`, `BoardTabsHeader.tsx`<br>• `StageContextMenu.tsx` & `stageMenu/` (`StageMainMenu.tsx`, `StageRenameView.tsx`, `StagePersonaView.tsx`, `StageScriptView.tsx`, `pipelineConfig.ts`) | Multi-column drag-and-drop board, column layout, stage context menu, and board tabs. |
| **Kanban Card** | `src/components/card/KanbanCard.tsx`, `cardFamily.ts`, `cardModel.ts`, `cardComparators.ts`<br>• Widgets: `CardTimerBadge.tsx`, `CardSubtaskBadge.tsx`, `CardContextMenu.tsx`, `CardHookNode.tsx`, `CardTodoProgress.tsx` | Layout coordinator and modular card widgets with custom memo equality comparators. |
| **Agent Console Drawer** | `src/components/drawer/AgentConsoleDrawer.tsx`<br>• Hooks: `useDrawerTaskState.ts`, `useUnifiedActivities.ts`, `useAgentExecution.ts`<br>• Header & Stepper: `DrawerHeader.tsx`, `stageStepperHelpers.ts`, `TaskTodoWidget.tsx`<br>• Steer Bar: `DrawerSteerBar.tsx` & `steer/` (`SteerImageBar.tsx`, `SteerModelMenu.tsx`, `useSteerModelResolution.ts`, `useSteerImages.ts`) | Fixed overlay console, activity stream integration, unified execution controls, and steer input. |
| **Drawer Subagents & Diff** | `src/components/drawer/SubagentsSidebar.tsx` & `subagents/` (`BoardSubtaskCard.tsx`, `InProcessSubagentCard.tsx`, `useSidebarResize.ts`)<br>• `GitDiffSidebar.tsx` & `diff/sidebar/` (`DiffHeader.tsx`, `DiffCollapsedRail.tsx`, `useGitDiffData.ts`, `useDiffResize.ts`)<br>• `diff/ImageDiffViewer.tsx` & `diff/image/` (`ImageDiffTwoUp.tsx`, `ImageDiffSplit.tsx`, `ImageDiffOnion.tsx`, `ImageDiffSingle.tsx`, `ImageDiffLightbox.tsx`) | Subagent hierarchy inspection, resizable Git diff sidebar, and visual multi-mode image diffing. |
| **Drawer Tool Cards** | • `tools/SubagentToolCard.tsx` & `tools/subagent/` (`SubagentStatusBadge.tsx`, `SubagentListCard.tsx`, `AwaitedSubagentRow.tsx`, `CollectSubagentCard.tsx`, `SingleSubagentCard.tsx`)<br>• `tools/FileOpsToolCard.tsx` & `tools/fileOps/` (`FileOpsHeader.tsx`, `FileOpsDiffView.tsx`, `FileOpsReadView.tsx`, `fileOpsUtils.ts`)<br>• `TodoToolCard.tsx`, `CommandToolCard.tsx`, `SearchToolCard.tsx`, `WebToolCard.tsx` | Specialized visual tool execution renderers. |
| **Cortex Mind Explorer** | `src/components/cortex/CortexExplorer.tsx`, `CvhSaveButton.tsx`, `DirtyDot.tsx`, `src/stores/cortex/cortexGitSave.ts`<br>• `AgentSettingVisualEditor.tsx` & `settingEditor/` (`CustomizationSection.tsx`, `ModelParametersSection.tsx`, `ContextLimitsSection.tsx`, `settingSchema.ts`)<br>• `RuleEditorModal.tsx`, `ColumnStageCard.tsx`, `GlobalRulesSection.tsx` | Mind tree navigation, persona editing, setting schema forms, rule editing, and Git save integration. |

---

## 4. Strict Architectural Invariants

1. **Universal Line Ceilings**:
   - **Backend Rust Modules**: Strictly **<= 500 lines** per file in `src-tauri/src/`.
   - **Frontend TS/React Files**: Strictly **<= 350 lines** per component or store slice in `src/`.
   - **Zero Legacy Immunity for Refactored Code**: All historical monoliths decomposed in CVH-069 are strictly subject to these ceilings. Any file touched or introduced in or after CVH-069 must comply unconditionally.
2. **Centralized Event Ingestion**:
   - ALL Tauri event listeners must register via `tauriEventGateway` (`src/services/tauriEventGateway.ts`). Direct `listen<any>` is strictly forbidden.
3. **Domain Value Objects & Algebraic Data Types**:
   - Use nominal branded types (`TaskId`, `StageId`, `BoardId`) and `TaskState` state machine representations in both Rust and TypeScript. Validate once at system boundaries, trust internally.
4. **Single-Instance Plugin Order**:
   - `tauri-plugin-single-instance` MUST always remain the first initialized plugin in `src-tauri/src/lib.rs`.
5. **Database Transaction Safety**:
   - Multi-step database mutations must use explicit SQLite transactions with immediate locking and parameter bindings in `src-tauri/src/db/`.
6. **Zero Silent Failures**:
   - Fallible operations must propagate errors via typed `Result` or log clear warnings. Empty `catch {}` blocks or swallowed errors (`let _ = ...`) are banned.

---

## 5. Topic Map Router

For detailed file routes, load the specialized topic maps on demand:
- Backend: `load_rule("backend_map")` (routes to `backend_agent_map`, `backend_config_map`, `backend_tasks_map`, `backend_storage_map`, `backend_platform_map`).
- Frontend: `load_rule("frontend_map")` (routes to `frontend_board_map`, `frontend_drawer_map`, `frontend_cortex_map`, `frontend_settings_map`, `frontend_shared_map`).
