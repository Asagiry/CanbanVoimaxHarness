---
description: Authoritative navigation map and component router for the React 19 / Tailwind CSS v4 frontend (src/).
---
# Frontend Navigation Map & Component Router (frontend_map.md)

Authoritative directory topology, component mapping table, and modular store architecture for the React 19 / Tailwind CSS v4 frontend of **CanbanVoimaxHarness (`src/`)**.

---

## 1. Quick Component & Action Router ("UI feature / Store action" -> "Exact file path")

| UI Feature / Store Action | Exact File Path | Key Invariants & Ceilings |
| :--- | :--- | :--- |
| **Application Root Entrypoint** | `src/App.tsx` | Root providers, obsidian theme, zoom wrappers. |
| **Splash Bootstrap Screen** | `src/components/splash/SplashScreen.tsx` | Boot screen displayed while backend state synchronizes. |
| **Window Titlebar & Controls** | `src/components/layout/WindowControls.tsx` | Tauri window controls (minimize, maximize, close). |
| **Top Navigation Bar** | `src/components/layout/Topbar.tsx` | Board selector, active project, search filter. |
| **Kanban Card Container & Layout** | `src/components/card/KanbanCard.tsx` | Layout coordinator (~256 lines), memoized with `areCardPropsEqual`. |
| **Tactile Stage Stepper Navigation** | `src/components/card/CardHookNode.tsx` | 1-click stage switching, current stage pulse, checkmarks. |
| **Card Timer & Cadence Badge** | `src/components/card/CardTimerBadge.tsx` | Execution duration, active step timer, tabular-nums. |
| **Card Context Menu & Model Override** | `src/components/card/CardContextMenu.tsx` | Context actions, model selection, subtask deletion. |
| **Task Context Menu Alias** | `src/components/card/TaskContextMenu.tsx` | Backward-compatible alias for `CardContextMenu.tsx`. |
| **Card Subtask Indicator Badge** | `src/components/card/CardSubtaskBadge.tsx` | Subtask counter, status indicator, auto-unlock indicator. |
| **Card Tactical Checklist Progress** | `src/components/card/CardTodoProgress.tsx` | `todo_write` visual progress bar and count. |
| **Card Shallow Memo Comparators** | `src/components/card/cardComparators.ts` | `areCardPropsEqual` preventing cascading re-renders. |
| **Kanban Board Container & Drag-Drop** | `src/components/board/KanbanBoard.tsx` | Multi-column DND layout, column rendering. |
| **Kanban Column Container** | `src/components/board/KanbanColumn.tsx` | Stage column, auto-run emitter, column settings. |
| **Multi-Board Tab Header** | `src/components/board/BoardTabsHeader.tsx` | Multi-board tab switching, board creation modal trigger. |
| **Stage Context Menu & Settings** | `src/components/board/StageContextMenu.tsx` | Stage settings, persona assignment, auto-run toggle. |
| **Stage Hook & Sensor Badges** | `src/components/board/StageHookBadge.tsx` | Transition script and gatekeeper sensor status. |
| **Column Context Menu Alias** | `src/components/board/ColumnContextMenu.tsx` | Backward-compatible alias for `StageContextMenu.tsx`. |
| **Column Hook Badge Alias** | `src/components/board/ColumnHookBadge.tsx` | Backward-compatible alias for `StageHookBadge.tsx`. |
| **Create Task Modal** | `src/components/board/modals/CreateTaskModal.tsx` | Task creation modal with persona & model selection. |
| **Create Column Modal** | `src/components/board/modals/CreateColumnModal.tsx` | Column creation modal with hook configuration. |
| **Create Board Modal** | `src/components/board/CreateBoardModal.tsx` | Board creation modal with preset templates. |
| **Board Image Lightbox Modal** | `src/components/board/modals/ImageLightboxModal.tsx` | Full-screen image preview for board attachments. |
| **Agent Console Drawer (Root Container)**| `src/components/drawer/AgentConsoleDrawer.tsx` | Hoisted fixed overlay, flex bounded (`min-h-0 min-w-0`). |
| **Drawer Header & Action Stepper** | `src/components/drawer/DrawerHeader.tsx` | Task title, tactile stage stepper, abort/run controls. |
| **Activity Stream & Comment Cards** | `src/components/drawer/DrawerActivityStream.tsx` | Step commentary vs prominent terminal response card. |
| **Human-in-the-Loop Question Prompt Card** | `src/components/drawer/QuestionPromptCard.tsx` | Interactive clarifying question card, single/multiple choice, custom input, markdown rendering. |
| **Steer Instruction Prompt Bar** | `src/components/drawer/DrawerSteerBar.tsx` | Interactive steer input bar, optimistic submission. |
| **Git Diff Realtime Sidebar** | `src/components/drawer/GitDiffSidebar.tsx` | Resizable git diff panel with live change updates. |
| **Subagents Execution Sidebar** | `src/components/drawer/SubagentsSidebar.tsx` | Subagent hierarchy, active status, subtask logs. |
| **Task Tactical Todos Widget** | `src/components/drawer/TaskTodoWidget.tsx` | Drawer-hoisted checklist progress bar and task item list. |
| **Compacted Context Notification Card**| `src/components/drawer/CompactedContextCard.tsx`| Notification banner card displaying token compaction savings. |
| **Human-in-the-Loop Question Card** | `src/components/drawer/QuestionPromptCard.tsx` | Interactive prompt card for user response selection. |
| **Fullscreen Image Lightbox** | `src/components/drawer/ImageLightboxModal.tsx` | Drawer attachment image lightbox modal. |
| **Live Thinking Stream Card** | `src/components/drawer/ThinkingCard.tsx` | Real-time reasoning chunk renderer with animated pulse. |
| **Generic Tool Execution Card** | `src/components/drawer/ToolExecutionCard.tsx` | Collapsible card for standard tool invocations and results. |
| **Context Gauge Status Ring** | `src/components/drawer/ContextGauge.tsx` | Real-time context token gauge, status bar & header ring. |
| **Session Statistics Popover** | `src/components/drawer/SessionStatsPopover.tsx` | DSH-style speedometer popover (LLM time, tool time, TTFT, TPS). |
| **Token Usage Popover** | `src/components/drawer/TokenUsagePopover.tsx` | DSH-style token usage popover (Cache hit, uncached/cached input, output). |
| **Context Gauge Hover Popover** | `src/components/drawer/ContextPopover.tsx` | Proportional segment breakdown popover, compaction trigger. |
| **Telemetry Formatters** | `src/components/drawer/telemetryFormatters.ts` | Compact time, TTFT, TPS, and localized token formatters. |
| **Context Gauge Metrics Hook** | `src/components/drawer/useContextGaugeMetrics.ts` | Extracted context resolution, token counting, and compaction listeners. |
| **Activity Stream Scroll Hook** | `src/components/drawer/useActivityScroll.ts` | Activity feed scroll management, auto-scroll, and resize observer. |
| **Git Diff Viewer (Unified & Image)** | `src/components/drawer/diff/ImageDiffViewer.tsx` | Visual image diffs, side-by-side and slider modes. |
| **Git Diff File List Sidebar** | `src/components/drawer/diff/DiffFileList.tsx` | Changed files sidebar, line additions/deletions. |
| **Thought Stream Block Card** | `src/components/drawer/stream/ThoughtStreamCard.tsx` | Collapsible reasoning stream card with duration badge. |
| **Steer Directive Card** | `src/components/drawer/stream/SteerDirectiveCard.tsx` | In-stream steer instruction indicator. |
| **Subagent Activity Card** | `src/components/drawer/stream/SubagentActivityCard.tsx` | Child agent delegation summary and report view. |
| **Markdown Content Renderer** | `src/components/drawer/stream/MarkdownContent.tsx` | Streaming markdown tokens, copyable codeblocks, lightbox. |
| **Agent Explanation Card** | `src/components/drawer/stream/AgentExplanationCard.tsx` | Intermediate step commentary, action plan, and todo list. |
| **Terminal Response Card** | `src/components/drawer/stream/TerminalResponseCard.tsx` | Prominent final response card with copy and duration. |
| **Task Objective Card** | `src/components/drawer/stream/TaskObjectiveCard.tsx` | Task goal, clean prompt, and attachment gallery. |
| **Execution Error Banner** | `src/components/drawer/stream/ExecutionErrorBanner.tsx` | In-stream error banner with retry and dismiss controls. |
| **Stage Summary Card** | `src/components/drawer/stream/StageSummaryCard.tsx` | Dedicated stage handover summary card and reviewer verdict card. |
| **Stream Filtering Helpers** | `src/components/drawer/stream/streamHelpers.ts` | Stream activity dedup, prompt cleanup, duration helpers. |
| **Todo Tool Card** | `src/components/drawer/tools/TodoToolCard.tsx` | `todo_write` checklist progress bar, status icons, and task rendering. |
| **Todo Shared Checklist** | `src/components/drawer/tools/TodoChecklist.tsx` | Reusable dual-color progress bar and task item checklist. |
| **File Operations Tool Card** | `src/components/drawer/tools/FileOpsToolCard.tsx` | `read_file`, `write_file`, `edit_file` diff display. |
| **Shell Command Tool Card** | `src/components/drawer/tools/CommandToolCard.tsx` | Command invocation, exit code, spilled logs link. |
| **Subagent Tool Card** | `src/components/drawer/tools/SubagentToolCard.tsx` | Child agent execution card with progress spinner. |
| **Context Compaction Tool Card** | `src/components/drawer/tools/CompactContextToolCard.tsx` | Optimistic compaction card with before/after stats. |
| **Generic Tool Fallback Card** | `src/components/drawer/tools/GenericToolCard.tsx` | Fallback card for custom or generic tool calls. |
| **Search Tool Card** | `src/components/drawer/tools/SearchToolCard.tsx` | Search tool results card with hit highlighting. |
| **Web Research Tool Card** | `src/components/drawer/tools/WebToolCard.tsx` | Web search and web fetch status card. |
| **Shared Tool Styles & Components** | `src/components/drawer/tools/shared.tsx` | Tool card frame, headers, and status icons. |
| **Cortex Explorer Tree** | `src/components/cortex/CortexExplorer.tsx` | File navigation tree for personas, rules, and hooks. |
| **Agent Persona Visual Editor** | `src/components/cortex/AgentSettingVisualEditor.tsx` | Persona prompt editor, settings, icon/color selectors. |
| **Agent Mind Inspector** | `src/components/cortex/AgentMindInspector.tsx` | Real-time agent state, current prompt and tools viewer. |
| **Column Stage Config Card** | `src/components/cortex/ColumnStageCard.tsx` | Pipeline stage configuration card in Cortex view. |
| **Create Agent Persona Modal** | `src/components/cortex/CreateAgentModal.tsx` | Modal dialog for creating new agent personas. |
| **Edit Agent Persona Modal** | `src/components/cortex/EditAgentPersonaModal.tsx` | Modal dialog for updating agent persona metadata. |
| **Global Rules Section** | `src/components/cortex/GlobalRulesSection.tsx` | Global rules list, editor triggers, and toggle controls. |
| **New Cortex File Form** | `src/components/cortex/NewCortexFileForm.tsx` | Inline form for creating new rules or personas. |
| **Modular Rule Editor Modal** | `src/components/cortex/RuleEditorModal.tsx` | Markdown editor for `.cvh/rules/*.md` and role rules. |
| **Cortex UI Types & Interfaces** | `src/components/cortex/types.ts` | Cortex explorer interfaces, forms, and tree nodes. |
| **Dynamic Role Icon** | `src/components/common/AgentIcon.tsx` | SVG/Lucide persona icon with nominal color mapping. |
| **Brand & File Extension Icons** | `src/components/common/BrandFileIcons.tsx` | File extension brand iconography (TS, RS, JSON, MD). |
| **Application Error Boundary** | `src/components/common/ErrorBoundary.tsx` | React 19 error boundary preventing entire app unmount. |
| **Brand Logo Icon** | `src/components/common/Logo.tsx` | CanbanVoimaxHarness application logo. |
| **Persona Dropdown Selector** | `src/components/common/PersonaSelect.tsx` | Persona select dropdown with colored avatar badges. |
| **Baked-in Runtime Rule Icon** | `src/components/common/RuntimeRuleIcon.tsx` | Dedicated SVG prompt-chip icon and `isRuntimeRule` helper. |
| **Two-Level Tool Policy Modal** | `src/components/settings/ToolPolicyModal.tsx` | Interactive policy matrix: global switches & overrides. |
| **Global Settings Modal** | `src/components/settings/SettingsModal.tsx` | LLM endpoints, active model, and app settings. |
| **UI Zoom Controller Hook** | `src/hooks/useUiZoom.ts` | Ctrl+Plus/Minus/Wheel viewport scaling. |
| **UI Zoom Controller Unit Tests** | `src/hooks/__tests__/useUiZoom.test.ts` | Vitest suite for UI zoom scaling, modal overlays, and wheel listeners. |
| **Desktop Notifications Dispatcher** | `src/services/notificationService.ts` | Cross-platform notification dispatch & toast formatting. |
| **Typed Tauri Event Gateway** | `src/services/tauriEventGateway.ts` | 150ms micro-batching, typed events (zero `listen<any>`).|
| **Project Management Store** | `src/stores/projectStore.ts` | Multi-project workspace switching, active project persistence. |
| **Agent Session Status Store** | `src/stores/agentSessionStore.ts` | Authoritative background agent execution tracker. |
| **Settings Store** | `src/stores/settingsStore.ts` | LLM configuration, endpoints, temperature settings. |
| **Theme Management Store** | `src/stores/themeStore.ts` | Obsidian tactile theme configuration. |
| **Tool Policy Matrix Store** | `src/stores/toolPolicyStore.ts` | Frontend cache for tool policy permissions. |
| **Language & Translation Store** | `src/stores/languageStore.ts` | Russian/English localization switching. |
| **Task DTO Normalizer & Persona Resolver** | `src/stores/board/taskNormalizer.ts` | Dynamic persona mapping for terminal/working stages. |
| **Tasks CRUD Store Slice** | `src/stores/slices/tasksCrudSlice.ts` | Backend-authoritative task CRUD, monotonic IDs. |
| **Board Meta Store Slice** | `src/stores/slices/boardMetaSlice.ts` | Active board, board switching, column layout. |
| **Task Timers Store Slice** | `src/stores/slices/taskTimersSlice.ts` | Active task step timers and execution stopwatch. |
| **Drawer Meta Store Slice** | `src/stores/drawer/slices/drawerMetaSlice.ts` | `activeTaskId`, drawer open/close, view mode. |
| **Drawer Activity Store Slice** | `src/stores/drawer/slices/activitySlice.ts` | `activityFeed`, optimistic tool injection, deduplication. |
| **Drawer Stream Store Slice** | `src/stores/drawer/slices/streamSlice.ts` | Live streaming chunks, thoughts, token telemetry. |
| **Drawer Subagents Store Slice** | `src/stores/drawer/slices/subagentsSlice.ts` | Subagent lifecycle, active subtasks, subagent logs. |
| **Drawer Input Drafts Slice** | `src/stores/drawer/slices/draftsSlice.ts` | Per-task draft text persistence for steer input bar. |
| **Unified Cortex Store Slice** | `src/stores/cortex/cortexStoreSlice.ts` | File tree, personas CRUD, and rules state slice. |
| **Desktop Notifications Dispatcher** | `src/services/notificationService.ts` | Desktop notification dispatching, click navigation, and user preferences filter. |
| **Notification Markdown Formatter** | `src/services/notificationFormatter.ts` | Markdown plain-text normalization, word-boundary truncation, and emoji-free titles. |
| **i18n Translation Dispatcher** | `src/i18n/translations.ts` | Localized translation resolver. |
| **i18n Russian Dictionary** | `src/i18n/ru.ts` | Russian translation entries. |
| **i18n English Dictionary** | `src/i18n/en.ts` | English translation entries. |
| **i18n Contract Types** | `src/i18n/types.ts` | Nominal types and key interfaces for i18n dictionaries. |
| **i18n Facade** | `src/i18n/index.ts` | Re-exports for translation helpers. |
| **Agent Persona Palette** | `src/lib/agentPalette.ts` | Theme-safe color constants for personas. |
| **Column Stage Icons Mapping** | `src/lib/columnIcons.ts` | Lucide icon resolvers for stages. |
| **Client Image Compression** | `src/lib/imageCompression.ts` | Attachment image resizer and optimizer. |
| **Frontend Logging Utility** | `src/lib/logger.ts` | Structured browser logs with backend routing. |
| **Task Slug Generator** | `src/lib/slug.ts` | Deterministic worktree and git branch naming. |
| **Decoupled Markdown Token Parser** | `src/lib/markdown/parseMarkdownTokens.ts` | AST token parser for codeblocks, tables, inline styling. |
| **Markdown Parser Types** | `src/lib/markdown/types.ts` | AST interfaces for markdown tokens. |
| **Markdown Facade** | `src/lib/markdown/index.ts` | Re-exports for markdown parser. |
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
- **`DrawerActivityStream.tsx`**: Slim orchestration layer for chronological activity feed, scroll management, and activity card dispatching.
- **`QuestionPromptCard.tsx`**: Interactive clarifying question card (single/multiple choice, custom input) with streaming markdown rendering and inline token formatting.
- **`stream/` Subsystem**: Decomposed streaming components:
  - **`MarkdownContent.tsx`**: Lightweight markdown token renderer, tabular monospace tables, and copyable code blocks.
  - **`AgentExplanationCard.tsx`**: Intermediate assistant thoughts, action plans, and todo checklist rendering.
  - **`TerminalResponseCard.tsx`**: Collapsible terminal response card with model badge, duration, copy button, and full markdown rendering.
  - **`TaskObjectiveCard.tsx`**: Initial task goal and attachment preview gallery card.
  - **`ExecutionErrorBanner.tsx`**: Execution failure alert banner with retry trigger.
  - **`streamHelpers.ts`**: Pure helper functions for activity filtering, question deduplication (§ 2.20), prompt cleaning, and duration calculations.
- **`ContextGauge.tsx`**: Real-time context window capacity meter and bottom telemetry bar coordinator. Renders 3 interactive pills (Speedometer, Token Usage, Context Ring) without native tooltips.
- **`SessionStatsPopover.tsx`**: DSH-style session statistics popover card displaying LLM time, tool time, TTFT, and TPS with hover-bridge protection.
- **`TokenUsagePopover.tsx`**: DSH-style token usage popover card displaying total tokens, cache hit rate %, uncached input, cached input, and output tokens.
- **`ContextPopover.tsx`**: Extracted hover card popover displaying segmented token breakdown (System, Tools, Messages) and trigger action for context compaction.
- **`telemetryFormatters.ts`**: Pure formatting utilities for compact durations, TTFT, TPS, compact tokens, and localized Russian/English pluralization.
- **`useContextGaugeMetrics.ts`**: Custom hook encapsulating token caching, capacity resolution, telemetry calculation, and compaction listeners.
- **`diff/ImageDiffViewer.tsx`**: Interactive image diffing component with side-by-side, swipe-slider, and opacity-blend modes.

### 2.3 Store Slices & State Hygiene (`src/stores/`)
- **Backend-Authoritative Lifecycle**: Zustand state mutations occur strictly after Tauri IPC backend confirmation. No optimistic card position updates or client-generated task IDs.
- **`tasksCrudSlice.ts`**: Pure CRUD operations. Auto-unlocks parent tasks when subtasks are deleted or completed.
- **`taskTimersSlice.ts`**: Active task execution durations and stopwatch timers.
- **`activitySlice.ts`**: Maintains `activityFeed`. Implements in-place tool deduplication by deterministic `tool_id` to eliminate card flickering upon dialogue reloads.
- **`streamSlice.ts`**: Accumulates streamed thoughts and assistant tokens. Freezes TPS immediately upon generation termination without metric decay.
- **`draftsSlice.ts`**: Preserves per-task draft text input for the steer drawer bar across switches.
- **`cortexStoreSlice.ts`**: Unified state slice for Cortex rule files, agent personas, and pipeline stage synchronization.

### 2.4 Typed IPC Event Gateway (`src/services/tauriEventGateway.ts`)
- **Zero Raw `listen<any>`**: Direct invocations of Tauri `listen` in components or stores are strictly prohibited.
- **150ms Micro-Batching**: High-frequency streaming events (`agent:thought`, `agent:stream`) are buffered in a 150ms window before flushing to reactive stores, eliminating UI thread locking during high-speed LLM generation (>100 tok/s).
- **Strongly Typed Payloads**: Emits typed domain events (`AgentThoughtPayload`, `AgentToolCallPayload`, `TaskUpdatedPayload`).

### 2.5 Desktop Notifications Subsystem (`src/services/`)
- **`notificationService.ts`**: Coordinates native desktop notifications and Windows fallback toasts on background transitions, question prompts, and rejections. Handles focus and drawer routing on click.
- **`notificationFormatter.ts`**: Normalizes raw markdown summaries into clean plain-text typography with Unicode bullets and vertical code/quote bars, smart word-boundary truncation (up to 320 chars), and clean titles without emojis.

---

## 3. Component Ceilings & UI Invariants

1. **350-Line Component Ceiling**: Every React component is strictly capped at **350 lines**. Multi-concern views must be decomposed into atomic child components or custom hooks.
2. **Selective Zustand Selectors**: Never call `useBoardStore()` or `useDrawerStore()` without selectors. Always use atomic selectors with `useShallow` to isolate re-renders.
3. **Tactile Obsidian Dark Theme**: Deep obsidian palette (`#07090e`, `#0d1117`, `#161b22`, `#30363d`). All form inputs, selects, and dialogs must declare paired dark tokens.
4. **Monospace Numerals**: Timers, token counts, TPS, and cadence indicators must enforce `tabular-nums` and monospace styling.
5. **No Native Browser Tooltips**: Never use native `title="..."` attributes on buttons or interactive controls; use custom dark popovers or self-explanatory iconography.
6. **Architectural Map Synchronization Mandate**: Whenever adding, removing, renaming, or relocating files in `src/` or `src-tauri/src/`, this navigation map MUST be updated in the exact same commit. The Level 2 Reviewer rejects diffs with `CHANGES_REQUESTED` if files were altered without map synchronization.
7. **Direct Coordinate Navigation**: Jump directly to mapped files with `read_file` when coordinates or paths are known. Targeted `grep` for specific symbols (components, hooks, store actions) remains a natural, valid tool.
