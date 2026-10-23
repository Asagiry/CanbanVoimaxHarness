# Frontend Architecture & UI Invariants (React 19, Tailwind v4, Zustand 5, TypeScript)

This guide documents the authoritative frontend engineering standards and UI invariants of **CanbanVoimaxHarness (CVH)** located in `src/`.

---

## 1. Core Stack & Runtime Architecture

- **React 19.0.0**: Functional components, React 19 Concurrent Mode & `StrictMode`. Global error boundary in `main.tsx` forwards renderer exceptions to Tauri via `log_ui_error`.
- **Vite 6.0.0**: High-speed bundler with path alias `@` -> `./src`. Vendor chunking strategy (`vendor-react`, `vendor-dnd`, `vendor-icons`, `vendor-tauri`). Server watch ignores `.cvh/`, `worktrees/`, and `src-tauri/` to eliminate reload loops during agent writes.
- **Tailwind CSS v4 (`@tailwindcss/vite` 4.0.0)**: Built-in `@theme` configuration directly in `src/index.css`. Zero separate config files.
- **Zustand 5.0.3 (`zustand`, `zustand/react/shallow`)**: Multi-store decoupled architecture. Synchronous reads via `Store.getState()`.
- **Drag & Drop (`@hello-pangea/dnd` 18.0.1)**: Multi-droppable Kanban board columns and card reordering.
- **Desktop IPC (`@tauri-apps/api` 2.2.0)**: Asynchronous command invocation (`invoke`) and bidirectional event streaming (`listen`).

---

## 2. Design System & UI Invariants

### 2.1 Dual Design System: Light Paper & Obsidian Dark
- **Light Mode (Vercel & ChatGPT Paper Aesthetic)**:
  - Page Canvas: `#fafafa`, Elevated Cards: `#ffffff`, Sidebar: `#f9f9f9`.
  - High-contrast typography: `#171717` (headings, primary labels), `#4d4d4d` (body).
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

## 3. State Management & React 19 Performance Discipline

### 3.1 Zustand `useShallow` Invariant
- When selecting arrays or objects from stores, **always wrap selectors in `useShallow`**:
  ```tsx
  import { useShallow } from "zustand/react/shallow";

  const subtasks = useBoardStore(
    useShallow((s) => s.tasks.filter((t) => t.parentId === task.id))
  );
  ```
- **CRITICAL**: Returning new arrays or objects without `useShallow` triggers infinite re-render loops and UI freezes.

### 3.2 Optimistic Updates & Authoritative Rollback
- In `@hello-pangea/dnd`, `handleDragEnd` must **synchronously** update `tasks` via `setTasks(nextTasks)` to preserve DOM node continuity.
- Retain a pre-drag snapshot to cleanly revert state if the backend rejects the transition (e.g. gatekeeper sensor failure or WIP limit).

### 3.3 Event Streaming & Micro-Batching
- `agent:thought` and `agent:output` chunks are buffered in memory and flushed to Zustand every **150ms** (`TEXT_FLUSH_MS`) to prevent UI lag.
- **Chronology Preservation Guard**: When a tool execution (`agent:tool_call`), user question (`agent:question`), or turn ends, `flushTask(taskId)` is executed **synchronously before registering the tool**, ensuring thought text always precedes tool cards.

### 3.4 Memoization on High-Density Cards
- Wrap high-frequency components (`KanbanCard`, `ThinkingCard`, `ToolExecutionCard`) in `React.memo` with shallow comparators.

---

## 4. TypeScript Strict Standards

- **Strict Mode**: `strict: true`, `noUnusedLocals: true`, `noUnusedParameters: true`.
- **Avoid `any`**: Use `unknown` for untrusted input and narrow it with type guards.
- **Named Interfaces**: Export explicit interfaces for component props, data models, and IPC payloads.
- **Sensor Check**: Verify frontend integrity via `npx tsc --noEmit` (must exit 0 with 0 errors).
