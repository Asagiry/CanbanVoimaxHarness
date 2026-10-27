# Frontend Architecture & UI Invariants (React 19, Tailwind v4, Zustand 5, TypeScript)

Authoritative frontend engineering standards and UI invariants of **CanbanVoimaxHarness (CVH)** located in `src/`.

---

## 1. Core Stack & Runtime Architecture

- **React 19.0.0**: Functional components, React 19 Concurrent Mode & `StrictMode`. Global error boundary in `main.tsx` forwards renderer exceptions to Tauri via `log_ui_error`.
- **Vite 6.0.0**: High-speed bundler with path alias `@` -> `./src`. Server watch ignores `.cvh/`, `worktrees/`, and `src-tauri/` to eliminate reload loops during agent writes.
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
- **Dark Mode (Obsidian Console Aesthetic)**:
  - Background: `#090a0f`, Surface: `#0d1117`, Card: `#161b22`, Border: `#21262d`, Text: `#f0f6fc`, Muted: `#8b949e`.
  - Circular reveal transition originating from theme toggle coordinates via View Transitions API.
- **Tabular Numerics**: Apply `.tabular-nums` across all timers, token telemetry, TPS counters, and duration pills to eliminate layout jitter.
- **Bidirectional Subtask Illumination**: Hovering a parent or subtask card highlights the entire task family (parent, child subtasks, sibling subtasks) with `ring-2 ring-[#8b5cf6]/50 bg-purple-50/20 dark:bg-[#1c1b29]` for clear relational context without visual clutter.

### 2.2 Viewport, Zoom & Layout Invariants
- **Zoom Isolation**: Canvas zoom (`useUiZoom`, Ctrl+Wheel or Ctrl+Plus/Minus) applies transforms exclusively to the workspace container (`rootStyle`).
- **Outside Root Mounting**: `AgentConsoleDrawer` and `SplashScreen` are mounted at the React document root **outside** the zoom container, preventing coordinate distortion.
- **Three-Column Cockpit & Flexbox Boundaries**:
  - `AgentConsoleDrawer`: Left (SubagentsSidebar), Center (ActivityStream + SteerBar), Right (GitDiffSidebar).
  - **Strict Flexbox Rule**: Every container in this chain MUST use `flex flex-col min-h-0 min-w-0 overflow-hidden` to guarantee inner scrollbars function independently without blowing out viewport height.
- **Progressive Card Loading (Virtualization)**: `KanbanColumn` uses `PAGE_SIZE = 10`. Only 10 cards are mounted initially; "Show more" loads subsequent slices.

### 2.3 Task Creation & Image Attachments
- **Modal & Layout Bounding**: New Task dialog enforces responsive bounds (`max-w-[95vw] max-h-[90vh]`) with scrollable flex containment (`flex flex-col min-h-0 overflow-y-auto`).
- **Clipboard & File Attachments**: Supports direct image pasting from clipboard with automatic client-side compression before storage or IPC transfer.
- **Attachment Previews & Lightbox**: Interactive attachment thumbnails provide removal, click-to-preview lightbox modal, and attachment gallery presentation in drawer feeds.

### 2.4 Redundant Copy Elimination & Tri-Partite Experience Invariants
- **Redundant Copy & Label Elimination**: Ruthlessly remove slogans, explanatory text, marketing subtitles, and minor helper labels. If an interface becomes incomprehensible after removal, the layout itself is flawed—redesign the spatial hierarchy and visual affordances from scratch rather than masking poor design with explanatory copy.
- **Tri-Partite Experience Invariant (UX, DX, AX Synergy)**: Every modification must simultaneously consider and improve User Experience (intuitive, tactile, zero clutter), Developer Experience (modular, strictly typed, fast sensors), and AI Agent Experience (predictable schemas, clean context, reliable diffs), without breaking existing functionality.

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

### 3.2 Event Streaming & Micro-Batching
- `agent:thought` and `agent:output` chunks are buffered in memory and flushed to Zustand every **150ms** (`TEXT_FLUSH_MS`) to prevent UI lag.
- When a tool execution (`agent:tool_call`), user question (`agent:question`), or turn ends, `flushTask(taskId)` is executed **synchronously before registering the tool**, ensuring thought text always precedes tool cards.

---

## 4. TypeScript Strict Standards & Zero Silent Failures

- **Strict Mode**: `strict: true`, `noUnusedLocals: true`, `noUnusedParameters: true`.
- **Avoid `any`**: Use `unknown` for untrusted input and narrow it with type guards.
- **Zero Silent Failures (silent-failure-hunter)**:
  - **Empty Catch Ban**: Never write empty catch blocks (`catch {}` or `catch (err) { /* ignore */ }`).
  - **Dangerous Fallback Ban**: Never mask failed network calls, IPC invocations, or data operations with silent fallbacks (e.g. `.catch(() => [])` or `.catch(() => null)`) that hide the root cause from the user and error boundaries.
  - **Diagnostic Context**: If an error is caught and handled gracefully, log it with context or forward to `log_ui_error`. Never leave downstream UI in a zombie state with uncommunicated failures.
- **Sensor Check**: Verify frontend integrity via `pnpm exec tsc --noEmit && pnpm exec oxlint --deny-warnings` (must exit 0 with 0 errors/warnings).
