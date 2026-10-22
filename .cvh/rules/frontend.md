# Frontend Architecture & UI Invariants (React 19, Tailwind v4, Zustand)

This guide documents the authoritative frontend architecture of **CanbanVoimaxHarness (CVH)** located in `src/`.

---

## 1. Core Technologies & Dependencies

- **React 19.0.0**: Functional components, React 19 Concurrent Mode & StrictMode. Global error capture in `main.tsx` forwards renderer exceptions to Tauri via `log_ui_error`.
- **Vite 6.0.0**: Bundler with path alias `@` -> `./src`. Custom chunking strategy (`vendor-react`, `vendor-dnd`, `vendor-icons`, `vendor-tauri`). Server watch ignores `.cvh/`, `worktrees/`, and `src-tauri/` to prevent reload loops during autonomous agent writes.
- **Tailwind CSS v4 (`@tailwindcss/vite` 4.0.0)**: Built-in `@theme` configuration in `src/index.css`. Zero separate config files. Strict CSS variable tokens for the dual design system.
- **Zustand 5.0.3 (`zustand`, `zustand/react/shallow`)**: Multi-store decoupled architecture. Synchronous reads via `Store.getState()`.
- **Drag & Drop (`@hello-pangea/dnd` 18.0.1)**: Multi-droppable board columns and column reordering.
- **Desktop IPC (`@tauri-apps/api` 2.2.0)**: Asynchronous command invocation (`invoke`) and bidirectional event streaming (`listen`).

---

## 2. Design System & UI Invariants

### 2.1 Dual Design System: Light Paper & Obsidian Dark
- **Light Mode (Vercel & ChatGPT Paper Aesthetic)**:
  - Page Canvas: `#fafafa`, Elevated Cards: `#ffffff`, Sidebar: `#f9f9f9`.
  - Near-black typography: `#171717` (headings, primary labels), `#4d4d4d` (body).
  - Hairline dividers: `#ebebeb` and `#d4d4d8` (1px subtle separation; zero blurry shadows).
  - Copilot Sidecar (`CopilotDock`): Strictly paper aesthetic with ink typography.
- **Dark Mode (Obsidian Console Aesthetic)**:
  - Background: `#090a0f`, Surface: `#0d1117`, Card: `#161b22`, Border: `#21262d`, Text: `#f0f6fc`, Muted: `#8b949e`.
  - Circular reveal transition (`@keyframes circular-clip-path`) originating from theme toggle coordinates via View Transitions API.
- **Tabular Numerics**: Apply `.tabular-nums` across all timers, token telemetry, TPS counters, and duration pills to eliminate layout jitter.

### 2.2 Viewport, Zoom & Layout Invariants
- **Zoom Isolation**: Canvas zoom (`useUiZoom`, Ctrl+Wheel or Ctrl+Plus/Minus) applies transforms exclusively to the workspace container (`rootStyle`).
- **Outside Root Mounting**: `AgentConsoleDrawer` and `SplashScreen` are mounted at the React document root **outside** the zoom container, preventing matrix coordinate corruption and clipping of fixed overlays.
- **Three-Column Cockpit & Flexbox Boundaries**:
  - `AgentConsoleDrawer`: Left (SubagentsSidebar), Center (ActivityStream + SteerBar), Right (GitDiffSidebar).
  - **Strict Flexbox Rule**: Every container in this chain MUST use `flex flex-col min-h-0 min-w-0 overflow-hidden` to guarantee inner scrollbars function independently without blowing out viewport height.
- **Progressive Card Loading (Virtualization)**: `KanbanColumn` uses `PAGE_SIZE = 10`. Only 10 cards are mounted initially; "Show more" loads subsequent slices.
- **Subtask Collapsing**: In `ready` and active columns, completed subtasks are hidden from the column list and collapsed into parent card accordions.
- **Parent-Subtask Connectors**: Dynamic cubic Bezier curves connect subtasks to parent cards across columns with hover highlighting and `ResizeObserver`.
- **Context Menu Suppression**: Custom React portals (`TaskContextMenu`, `ColumnContextMenu`) replace default WebView2 context menus, while native menus are preserved in editable elements (`isEditableElement`).

---

## 3. State Management & Event Streaming Invariants

### 3.1 Store Responsibilities
- `boardStore`: Boards, pipeline columns, and tasks. Optimistic updates with authoritative fallback.
- `drawerStore`: Activity feed, live streaming, token telemetry, context compaction, and subagent sessions.
- `copilotStore`: Docked AI peer architect stream and markdown rendering.
- `cortexStore`: Agent persona and rule filesystem tree.
- `toolPolicyStore`: Two-level tool permission matrix.
- `projectStore` & `settingsStore`: Active repo paths and LLM endpoints.

### 3.2 Optimistic Updates & Concurrency Guard
- In `@hello-pangea/dnd`, `handleDragEnd` must **synchronously** update `tasks` via `setTasks(nextTasks)` before awaiting the asynchronous `move_task_stage()` IPC call.
- Retain a pre-drag snapshot to revert state if the backend rejects the transition (e.g. gatekeeper sensor failure).

### 3.3 Event Streaming & Micro-Batching
- `agent:thought` and `agent:output` chunks are buffered in memory and flushed to Zustand every **150ms** (`TEXT_FLUSH_MS`) to prevent UI lag.
- **Chronology Preservation Guard**: When a tool execution (`agent:tool_call`), user question (`agent:question`), or turn starts, `flushTask(taskId)` is executed **synchronously before registering the tool**, ensuring thought text always precedes tool cards.

### 3.4 React 19 & Zustand Performance Discipline
- When selecting arrays or objects from stores, **always wrap selectors in `useShallow`**:
  ```tsx
  const subtasks = useBoardStore(
    useShallow((s) => s.tasks.filter((t) => t.parentId === task.id))
  );
  ```
- Returning new arrays without `useShallow` triggers infinite re-render loops.
- Use `React.memo` on high-density cards (`KanbanCard`, `ThinkingCard`, `ToolExecutionCard`).

### 3.5 Dual Runtime Compatibility (Tauri IPC vs. Web Mode)
- All calls to `@tauri-apps/api` must be guarded with `try / catch` or dynamic imports so that Vite previews and unit tests execute without crashing.
