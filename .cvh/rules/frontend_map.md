---
description: Authoritative navigation map and component router for the React 19 / Tailwind CSS v4 frontend (src/).
---
# Frontend Navigation Map & Component Router (frontend_map.md)

Authoritative directory topology, component mapping table, and modular store architecture for the React 19 / Tailwind CSS v4 frontend of **CanbanVoimaxHarness (`src/`)**.

---

## 1. Quick Component & Action Router ("UI feature / Store action" -> "Exact file path")

| UI Feature / Store Action | Exact File Path | Key Invariants & Ceilings |
| :--- | :--- | :--- |
| **Kanban Card Container & Layout** | `src/components/card/KanbanCard.tsx` | Layout coordinator, memoized with `areCardPropsEqual`. |
| **Tactile Stage Stepper Navigation** | `src/components/card/CardHookNode.tsx` | 1-click stage switching, current stage pulse, checkmarks. |
| **Card Timer & Cadence Badge** | `src/components/card/CardTimerBadge.tsx` | Execution duration, active step timer, tabular-nums. |
| **Card Context Menu & Model Override** | `src/components/card/CardContextMenu.tsx` | Context actions, model selection, subtask deletion. |
| **Card Subtask Indicator Badge** | `src/components/card/CardSubtaskBadge.tsx` | Subtask counter, status indicator, auto-unlock indicator. |
| **Card Tactical Checklist Progress** | `src/components/card/CardTodoProgress.tsx` | `todo_write` visual progress bar and count. |
| **Card Shallow Memo Comparators** | `src/components/card/cardComparators.ts` | `areCardPropsEqual` preventing cascading re-renders. |
| **Kanban Board Container & Drag-Drop** | `src/components/board/KanbanBoard.tsx` | Multi-column DND layout, column rendering. |
| **Kanban Column Container** | `src/components/board/KanbanColumn.tsx` | Stage column, auto-run emitter, column settings. |
| **Multi-Board Tab Header** | `src/components/board/BoardTabsHeader.tsx` | Multi-board tab switching, board creation modal trigger. |
| **Column Context Menu** | `src/components/board/ColumnContextMenu.tsx` | Column settings, persona assignment, auto-run toggle. |
| **Column Hook & Sensor Badges** | `src/components/board/ColumnHookBadge.tsx` | Transition script and gatekeeper sensor status. |
| **Create Task Modal** | `src/components/board/modals/CreateTaskModal.tsx` | Task creation modal with persona & model selection. |
| **Create Column Modal** | `src/components/board/modals/CreateColumnModal.tsx` | Column creation modal with hook configuration. |
| **Create Board Modal** | `src/components/board/CreateBoardModal.tsx` | Board creation modal with preset templates. |
| **Agent Console Drawer (Root Container)**| `src/components/drawer/AgentConsoleDrawer.tsx` | Hoisted fixed overlay, flex bounded (`min-h-0 min-w-0`). |
| **Drawer Header & Action Stepper** | `src/components/drawer/DrawerHeader.tsx` | Task title, tactile stage stepper, abort/run controls. |
| **Activity Stream & Comment Cards** | `src/components/drawer/DrawerActivityStream.tsx` | Step commentary vs prominent terminal response card. |
| **Steer Instruction Prompt Bar** | `src/components/drawer/DrawerSteerBar.tsx` | Interactive steer input bar, optimistic submission. |
| **Proportional Context Gauge & Popover**| `src/components/drawer/ContextGauge.tsx` | Context token gauge, popover breakdown, compact trigger. |
| **Subagents Execution Sidebar** | `src/components/drawer/SubagentsSidebar.tsx` | Subagent hierarchy, active status, subtask logs. |
| **Git Diff Viewer (Unified & Image)** | `src/components/drawer/diff/ImageDiffViewer.tsx` | Visual image diffs, side-by-side and slider modes. |
| **Git Diff File List Sidebar** | `src/components/drawer/diff/DiffFileList.tsx` | Changed files sidebar, line additions/deletions. |
| **Thought Stream Block Card** | `src/components/drawer/stream/ThoughtStreamCard.tsx` | Collapsible reasoning stream card with duration badge. |
| **Steer Directive Card** | `src/components/drawer/stream/SteerDirectiveCard.tsx` | In-stream steer instruction indicator. |
| **Subagent Activity Card** | `src/components/drawer/stream/SubagentActivityCard.tsx` | Child agent delegation summary and report view. |
| **File Operations Tool Card** | `src/components/drawer/tools/FileOpsToolCard.tsx` | `read_file`, `write_file`, `edit_file` diff display. |
| **Shell Command Tool Card** | `src/components/drawer/tools/CommandToolCard.tsx` | Command invocation, exit code, spilled logs link. |
| **Subagent Tool Card** | `src/components/drawer/tools/SubagentToolCard.tsx` | Child agent execution card with progress spinner. |
| **Context Compaction Tool Card** | `src/components/drawer/tools/CompactContextToolCard.tsx` | Optimistic compaction card with before/after stats. |
| **Cortex Explorer Tree** | `src/components/cortex/CortexExplorer.tsx` | File navigation tree for personas, rules, and hooks. |
| **Agent Persona Visual Editor** | `src/components/cortex/AgentSettingVisualEditor.tsx` | Persona prompt editor, settings, icon/color selectors. |
| **Modular Rule Editor Modal** | `src/components/cortex/RuleEditorModal.tsx` | Markdown editor for `.cvh/rules/*.md` and role rules. |
| **Two-Level Tool Policy Modal** | `src/components/settings/ToolPolicyModal.tsx` | Interactive policy matrix: global switches & overrides. |
| **Global Settings Modal** | `src/components/settings/SettingsModal.tsx` | LLM endpoints, active model, and app settings. |
| **Window Titlebar & Controls** | `src/components/layout/WindowControls.tsx` | Tauri window controls (minimize, maximize, close). |
| **Top Navigation Bar** | `src/components/layout/Topbar.tsx` | Board selector, active project, search filter. |
| **Tasks CRUD Store Slice** | `src/stores/slices/tasksCrudSlice.ts` | Backend-authoritative task CRUD, monotonic IDs. |
| **Board Meta Store Slice** | `src/stores/slices/boardMetaSlice.ts` | Active board, board switching, column layout. |
| **Drawer Meta Store Slice** | `src/stores/drawer/slices/drawerMetaSlice.ts` | `activeTaskId`, drawer open/close, view mode. |
| **Drawer Activity Store Slice** | `src/stores/drawer/slices/activitySlice.ts` | `activityFeed`, optimistic tool injection, deduplication. |
| **Drawer Stream Store Slice** | `src/stores/drawer/slices/streamSlice.ts` | Live streaming chunks, thoughts, token telemetry. |
| **Drawer Subagents Store Slice** | `src/stores/drawer/slices/subagentsSlice.ts` | Subagent lifecycle, active subtasks, subagent logs. |
| **Cortex Files Store Slice** | `src/stores/cortex/slices/cortexFilesSlice.ts` | File tree hierarchy, disk file read/write. |
| **Cortex Personas Store Slice** | `src/stores/cortex/slices/personasSlice.ts` | Persona CRUD, settings synchronization. |
| **Cortex Rules Store Slice** | `src/stores/cortex/slices/rulesSlice.ts` | Rule creation, editing, and frontmatter management. |
| **Typed Tauri Event Gateway** | `src/services/tauriEventGateway.ts` | 150ms micro-batching, typed events (zero `listen<any>`).|
| **Nominal Domain Types & Models** | `src/types/domain.ts` | `TaskId`, `StageId`, `BoardId`, `TaskState`. |

---

## 2. Modular Architecture & Data Flow

### 2.1 Card Subsystem (`src/components/card/`)
- **`KanbanCard.tsx`**: Clean, lightweight layout coordinator. Wraps components in `React.memo` with `areCardPropsEqual`. Strictly avoids wholesale subscriptions to `useBoardStore`.
- **`CardHookNode.tsx`**: Renders the tactile stage stepper on every card. Enables 1-click stage advancement, displays pulse animations for active stages, and verifies gatekeeper sensors.
- **`CardTimerBadge.tsx`**: Displays active step time and cumulative execution duration using monospace numerals (`tabular-nums`) to prevent layout shifts.
- **`CardContextMenu.tsx`**: Right-click actions for cards: model overrides, manual compaction, stage moving, subtask inspection, and deletion.

### 2.2 Console Drawer Subsystem (`src/components/drawer/`)
- **`AgentConsoleDrawer.tsx`**: Slide-out execution console. Rendered at root fragment level with `position: fixed` outside any transformed or flex-row containers to prevent clipping. Enforces `min-h-0` and `min-w-0` on scrollable flex children.
- **`DrawerActivityStream.tsx`**: Renders chronological conversation and execution events. Distinguishes intermediate step commentary (`isFinal: false`) from prominent terminal response cards (`isFinal: true`).
- **`ContextGauge.tsx`**: Real-time context window capacity meter. Resolves window capacity from persona settings (`context_window`), proportional segment allocations (System, Tools, Messages), and provides an interactive compaction popover.
- **`diff/ImageDiffViewer.tsx`**: Interactive image diffing component with side-by-side, swipe-slider, and opacity-blend modes.

### 2.3 Store Slices & State Hygiene (`src/stores/`)
- **Backend-Authoritative Lifecycle**: Zustand state mutations occur strictly after Tauri IPC backend confirmation. No optimistic card position updates or client-generated task IDs.
- **`tasksCrudSlice.ts`**: Pure CRUD operations. Auto-unlocks parent tasks when subtasks are deleted or completed.
- **`activitySlice.ts`**: Maintains `activityFeed`. Implements in-place tool deduplication by deterministic `tool_id` to eliminate card flickering upon dialogue reloads.
- **`streamSlice.ts`**: Accumulates streamed thoughts and assistant tokens. Freezes TPS immediately upon generation termination without metric decay.

### 2.4 Typed IPC Event Gateway (`src/services/tauriEventGateway.ts`)
- **Zero Raw `listen<any>`**: Direct invocations of Tauri `listen` in components or stores are strictly prohibited.
- **150ms Micro-Batching**: High-frequency streaming events (`agent:thought`, `agent:stream`) are buffered in a 150ms window before flushing to reactive stores, eliminating UI thread locking during high-speed LLM generation (>100 tok/s).
- **Strongly Typed Payloads**: Emits typed domain events (`AgentThoughtPayload`, `AgentToolCallPayload`, `TaskUpdatedPayload`).

---

## 3. Component Ceilings & UI Invariants

1. **350-Line Component Ceiling**: Every React component is strictly capped at **350 lines**. Multi-concern views must be decomposed into atomic child components or custom hooks.
2. **Selective Zustand Selectors**: Never call `useBoardStore()` or `useDrawerStore()` without selectors. Always use atomic selectors with `useShallow` to isolate re-renders.
3. **Tactile Obsidian Dark Theme**: Deep obsidian palette (`#07090e`, `#0d1117`, `#161b22`, `#30363d`). All form inputs, selects, and dialogs must declare paired dark tokens.
4. **Monospace Numerals**: Timers, token counts, TPS, and cadence indicators must enforce `tabular-nums` and monospace styling.
5. **No Native Browser Tooltips**: Never use native `title="..."` attributes on buttons or interactive controls; use custom dark popovers or self-explanatory iconography.
6. **Architectural Map Synchronization Mandate**: Whenever adding, removing, renaming, or relocating files in `src/` or `src-tauri/src/`, this navigation map MUST be updated in the exact same commit. The Level 2 Reviewer rejects diffs with `CHANGES_REQUESTED` if files were altered without map synchronization.
7. **Direct Coordinate Navigation**: Jump directly to mapped files with `read_file` when coordinates or paths are known. Targeted `grep` for specific symbols (components, hooks, store actions) remains a natural, valid tool.
