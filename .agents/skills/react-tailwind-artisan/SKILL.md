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
