---
name: react-tailwind-artisan
description: Specialized UI/UX engineering skill for CanbanVoimaxHarness. Covers React 19, Tailwind CSS v4, Lucide icons, Framer Motion animations, DND board interactions, tactile pop-art styling, and dark/light themes.
---

# React 19 & Tailwind CSS v4 UI Artisan Skill

### Directives:
1. **Design System & Typography**:
   - Modern cyber-slate / obsidian palette for dark theme; crisp slate-50 / white for light theme.
   - Accent colors: Emerald `#10b981` (Approved/Ready), Amber `#f59e0b` (In Dev / Caveats), Crimson `#ef4444` (Changes Requested), Cyan `#06b6d4` (Retro), Indigo `#6366f1` (Todo).
   - Monospace tabular numbers (`font-mono tabular-nums`) for all timers, token counters, and commit hashes.
2. **Interactive Kanban Board**:
   - Smooth drag and drop using `@hello-pangea/dnd` or `dnd-kit`.
   - Card fronts display: task ID, title, persona badge, stage indicator, returns count (`↩ Returns: 1x`), frozen stage timers (`Dev: 14m`, `Review: 2m`), total time, and fast-track tags.
3. **Slide-Out Drawer (Real-Time Agent Console)**:
   - Live stream of agent thoughts (`<thinking>` tokens) and assistant text.
   - Collapsible tool execution cards with syntax-highlighted code diffs and terminal outputs.
   - Direct control actions: Pause, Force Kill, Promote to Ready, Retry.
4. **Cortex Explorer («Проводник разума агентов»)**:
   - Tree view of agent mind files (`.cvh/agents/*.md`, project rules, skill catalog) organized by pipeline stage.
   - Zero-turn memory inspection and in-app modal rule editor with instant disk persistence.
5. **Frontend State & Sensor Invariants**:
   - Zero-GC / Re-Render Loop Guard: Do not instantiate unmemoized objects/callbacks in drag-and-drop contexts or 1-second timer ticker intervals.
   - Zustand Interface Completeness: Every action declared in a state interface MUST be fully implemented with synchronized state transitions (e.g., coupling `status` and `isTimerRunning`).
   - Ground Truth Build Verification: Always verify frontend builds via `npm run build` (`tsc && vite build`) with zero TypeScript errors or linter warnings.
   - JSX Tag Hierarchy & Cleanliness: Maintain strict balanced opening/closing tag pairs (prevent accidental duplicate closing tags like `</button></button>`) and prune unused Lucide icon imports before submission.
   - Accessibility & Unique IDs: Use React 19's `useId()` for all modal and form inputs to prevent duplicate DOM element ID warnings.
