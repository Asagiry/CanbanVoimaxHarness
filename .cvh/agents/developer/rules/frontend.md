---
description: React 19, Tailwind CSS v4, Zustand store slices, typed event gateway, and 350-line component ceilings.
---
# Frontend Architecture & UI Invariants (React 19, Tailwind v4, Zustand 5, TypeScript)

Authoritative frontend engineering standards and UI invariants of **CanbanVoimaxHarness (CVH)** located in `src/`.

---

## 1. Core Stack & Runtime Architecture

- **React 19.0.0**: Functional components, React 19 Concurrent Mode & `StrictMode`. Global error boundary in `main.tsx` forwards renderer exceptions to Tauri via `log_ui_error`.
- **Vite 6.0.0**: High-speed bundler with path alias `@` -> `./src`. Server watch ignores `.cvh/`, `worktrees/`, and `src-tauri/` to eliminate reload loops during agent writes.
- **Tailwind CSS v4 (`@tailwindcss/vite` 4.0.0)**: Built-in `@theme` configuration directly in `src/index.css`. Zero separate config files.
- **Zustand 5.0.3 (`zustand`, `zustand/react/shallow`)**: Multi-store decoupled slice architecture. Synchronous reads via `Store.getState()`.
- **Drag & Drop (`@hello-pangea/dnd` 18.0.1)**: Multi-droppable Kanban board columns and card reordering.
- **Desktop IPC & Typed Gateway**: Strongly typed communication via Tauri IPC commands (`invoke`) and the centralized `tauriEventGateway`.

---

## 2. Event Gateway & Domain Layer Invariants

### 2.1 Centralized Typed Event Gateway (`src/services/tauriEventGateway.ts`)
- **Hard Ban on Raw `listen<any>`**: Components and stores are **STRICTLY FORBIDDEN** from directly invoking `listen<any>` or untyped event listeners.
- **Single Gateway Authority**: All Tauri backend events flow exclusively through `setupTauriEventGateway()` backed by the authoritative `AgentEventMap` interface.
- **Streaming Micro-Batching (`TEXT_FLUSH_MS = 150ms`)**: Streaming chunks (`agent:thought`, `agent:output`) are buffered in memory via `TextBufferManager` and flushed to stores at 150ms intervals to eliminate UI frame drops.
- **Synchronous Pre-Tool Flush**: When a tool execution (`agent:tool_call`), question (`agent:question`), or turn end arrives, `flushTask(taskId)` executes synchronously *before* tool registration, preserving strict chronological narrative order.

### 2.2 Domain Value Objects & Nominal Types (`src/types/domain.ts`)
Adheres to Clean Architecture and Uncle Bob's Domain-Driven Design (DDD) principles:
- **Nominal Branded Types**:
  - `TaskId`: Branded string guaranteed to match `^[A-Z0-9-_.:]+$`, trimmed and uppercased (e.g. `"CVH-042"`).
  - `StageId`: Branded lowercase identifier (e.g. `"dev"`, `"review"`).
  - `BoardId`: Branded lowercase identifier (e.g. `"development"`, `"kanban"`).
- **Domain Factories & Type Guards**:
  - Boundary input MUST be validated and normalized via factories: `toTaskId(raw)`, `toStageId(raw)`, `toBoardId(raw)`.
  - Type guards `isTaskId(v)`, `isStageId(v)`, `isBoardId(v)` narrow untrusted data.
- **State Machine ADT (`TaskState`)**:
  - Discriminated union: `{ status: 'idle' } | { status: 'running'; model?: string } | { status: 'waiting_subtask'; waitingForSubtaskId: string } | { status: 'done'; verdict?: string }`.
  - Makes illegal states unrepresentable (e.g. waiting for a subtask without a subtask ID).
  - Use constructors `TaskStates.idle()`, `TaskStates.running(model)`, `TaskStates.waitingSubtask(id)`, `TaskStates.done(verdict)`.
- **Stage Predicates**: Use canonical predicates `isTerminalStage(col)` and `isIntakeStage(col)` rather than hardcoding string checks.

---

## 3. Modular Component Architecture & Single Responsibility Principle (SRP)

### 3.1 Component Size Ceiling (Max 250–350 Lines)
- **Strict SRP Mandate**: No React component may exceed **250–350 lines of code**.
- Any component expanding beyond 350 lines MUST be refactored into focused, atomic child components.
- Piling miscellaneous UI badges, context menus, tool cards, and modal logic into single parent components (such as `KanbanCard.tsx` or `AgentConsoleDrawer.tsx`) is strictly prohibited.

### 3.2 Modular Card Subsystem (`src/components/card/`)
Kanban card functionality is partitioned into specialized widgets coordinated by `KanbanCard.tsx`:
- `CardTimerBadge.tsx`: Tabular execution timer, active turn pulse, and elapsed duration display.
- `CardSubtaskBadge.tsx`: Child subtask counter chip, status indicators, and bidirectional hover illumination.
- `CardContextMenu.tsx`: Quick action dropdown, tactile stage switcher, and deletion confirmation.
- `CardHookNode.tsx`: Pipeline transition hook status pills, execution indicators, and manual hook triggers.
- `CardTodoProgress.tsx`: Mini progress bar tracking `todo_write` tactical checklists.

### 3.3 Modular Drawer Stream Subsystem (`src/components/drawer/stream/`)
Activity feed items are rendered by dedicated stream card components:
- `ThoughtStreamCard.tsx`: Reasoning and thought blocks with collapsible folding and token gauges.
- `SteerDirectiveCard.tsx`: User steer directives and manual prompt injections.
- `SubagentActivityCard.tsx`: Child subagent lifecycle, parallel task progress, and synthesis reports.

### 3.4 Decoupled Markdown Token Parsing (`src/lib/markdown/`)
- Markdown parsing logic resides in `src/lib/markdown/parseMarkdownTokens.ts`.
- AST token generation and regex processing are decoupled from React DOM rendering, ensuring fast, deterministic parsing with 100% test coverage.

### 3.5 Multi-Slice Zustand Architecture
Stores are decomposed into modular slices composed into a root store:
- **Cortex Store** (`src/stores/cortex/slices/`):
  - `personasSlice.ts`: Persona YAML loading, icons, colors, tool policies.
  - `rulesSlice.ts`: Markdown rule files reading, editing, and dynamic injection.
  - `cortexFilesSlice.ts`: Raw project configuration file operations.
- **Board Store** (`src/stores/slices/`):
  - `boardMetaSlice.ts`: Active board ID, board list, column definitions.
  - `tasksCrudSlice.ts`: Task creation, stage transitions, backend sync, subtask relations.
  - `taskTimersSlice.ts`: Live execution timing, stage elapsed times, returns counting.
- **Drawer Store** (`src/stores/drawer/slices/`):
  - `drawerMetaSlice.ts`: Drawer open/close state, active task ID, active tab.
  - `streamSlice.ts`: Thought chunks, assistant output, live token streaming.
  - `activitySlice.ts`: Tool execution cards, user steer cards, dialog history.
  - `subagentsSlice.ts`: Background subagent sessions, listeners, and progress.

---

## 4. Design System & UI Invariants

### 4.1 Dual Design System: Light Paper & Obsidian Dark
- **Light Mode (Vercel & ChatGPT Paper Aesthetic)**:
  - Page Canvas: `#fafafa`, Elevated Cards: `#ffffff`, Sidebar: `#f9f9f9`.
  - High-contrast typography: `#171717` (headings, primary labels), `#4d4d4d` (body).
  - Hairline dividers: `#ebebeb` and `#d4d4d8` (1px subtle separation; zero blurry shadows).
- **Dark Mode (Obsidian Console Aesthetic)**:
  - Background: `#090a0f`, Surface: `#0d1117`, Card: `#161b22`, Border: `#21262d`, Text: `#f0f6fc`, Muted: `#8b949e`.
  - Circular reveal transition originating from theme toggle coordinates via View Transitions API.
- **Tabular Numerics**: Apply `.tabular-nums` across all timers, token telemetry, TPS counters, and duration pills to eliminate layout jitter.
- **Bidirectional Subtask Illumination**: Hovering a parent or subtask card highlights the entire task family (parent, child subtasks, sibling subtasks) with `ring-2 ring-[#8b5cf6]/50 bg-purple-50/20 dark:bg-[#1c1b29]` for clear relational context without visual clutter.

### 4.2 Viewport, Zoom & Layout Invariants
- **Zoom Isolation**: Canvas zoom (`useUiZoom`, Ctrl+Wheel or Ctrl+Plus/Minus) applies transforms exclusively to the workspace container (`rootStyle`).
- **Outside Root Mounting**: `AgentConsoleDrawer` and `SplashScreen` are mounted at the React document root **outside** the zoom container, preventing coordinate distortion.
- **Three-Column Cockpit & Flexbox Boundaries**:
  - `AgentConsoleDrawer`: Left (SubagentsSidebar), Center (ActivityStream + SteerBar), Right (GitDiffSidebar).
  - **Strict Flexbox Rule**: Every container in this chain MUST use `flex flex-col min-h-0 min-w-0 overflow-hidden` to guarantee inner scrollbars function independently without blowing out viewport height.
- **Progressive Card Loading (Virtualization)**: `KanbanColumn` uses `PAGE_SIZE = 10`. Only 10 cards are mounted initially; "Show more" loads subsequent slices.

### 4.3 Redundant Copy Elimination & Tri-Partite Experience Invariants
- **Redundant Copy & Label Elimination**: Ruthlessly remove slogans, explanatory text, marketing subtitles, and minor helper labels. If an interface becomes incomprehensible after removal, the layout itself is flawed—redesign the spatial hierarchy and visual affordances from scratch.
- **Tri-Partite Experience Invariant (UX, DX, AX Synergy)**: Every modification must simultaneously consider and improve User Experience (intuitive, tactile, zero clutter), Developer Experience (modular, strictly typed, fast sensors), and AI Agent Experience (predictable schemas, clean context, reliable diffs), without breaking existing functionality.

---

## 5. State Management & React 19 Performance Discipline

### 5.1 Zustand `useShallow` Invariant
- When selecting arrays or objects from stores, **always wrap selectors in `useShallow`**:
  ```tsx
  import { useShallow } from 'zustand/react/shallow';

  const subtasks = useBoardStore(
    useShallow((s) => s.tasks.filter((t) => t.parentId === task.id))
  );
  ```
- **CRITICAL**: Returning new arrays or objects without `useShallow` triggers infinite re-render loops and UI freezes.

### 5.2 React.memo & Shallow Comparators
- Components with high-frequency updates (`KanbanCard`, `KanbanColumn`, badges) MUST be wrapped in `React.memo` using custom shallow property comparators (`areCardPropsEqual`, `areColumnPropsEqual`).

---

## 6. TypeScript Strict Standards & Verification Sensors

- **Strict Mode**: `strict: true`, `noUnusedLocals: true`, `noUnusedParameters: true`.
- **Avoid `any`**: Use `unknown` for untrusted input and narrow it with type guards or domain Value Object factories.
- **Zero Silent Failures (silent-failure-hunter)**:
  - **Empty Catch Ban**: Never write empty catch blocks (`catch {}` or `catch (err) { /* ignore */ }`).
  - **Dangerous Fallback Ban**: Never mask failed network calls, IPC invocations, or data operations with silent fallbacks (e.g. `.catch(() => [])` or `.catch(() => null)`).
  - **Diagnostic Context**: If an error is caught and handled gracefully, log it with context or forward to `log_ui_error`.
- **Mandatory Vitest & Turbo Sensors**:
  - All frontend code changes MUST satisfy:
    ```bash
    pnpm test && pnpm exec tsc --noEmit && pnpm exec oxlint --deny-warnings
    ```
  - `pnpm test` executes the full Vitest suite (all test suites must pass 100% with 0 failures).
