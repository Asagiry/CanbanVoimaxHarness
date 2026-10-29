---
description: Frontend reactivity, memory leaks, theme contrast, event gateway compliance, and UI anti-slop review checklist.
---
# Rule: Frontend UI Invariants & Performance Audit (frontend-invariants.md)

Authoritative frontend audit checklist for the **Reviewer** persona in CanbanVoimaxHarness.

---

## 1. Strict Ban on Direct `listen<any>` (Only `tauriEventGateway`)
- **Rule**: Components, hooks, and stores are **strictly forbidden** from invoking Tauri's `listen(...)` or `once(...)` directly, and strictly forbidden from using untyped `<any>` event payloads.
- **The Sole Lawful Gateway**: ALL Tauri backend event listening, event unlistening, and micro-batching MUST route through `src/services/tauriEventGateway.ts` via strongly typed event interfaces (`AgentThoughtPayload`, `AgentOutputPayload`, `AgentToolCallPayload`, `AgentTelemetryPayload`, `TaskAdvancedPayload`, etc.).
- **REJECTION TRIGGER**: Any direct `import { listen } from '@tauri-apps/api/event'` outside `src/services/tauriEventGateway.ts` or any `listen<any>` anywhere in `src/` triggers **IMMEDIATE REJECTION** (`CHANGES_REQUESTED`).

---

## 2. Component File Size Ceiling (<= 350 lines)
- **Rule**: React components must NOT exceed 350 lines of code. Keep components atomic, focused, and maintainable (gold standard: `KanbanCard.tsx` at ~239 lines).
- **Audit**: Check line counts of all new or modified `.tsx` files in `src/components/`:
  - Monolithic components exceeding 350 lines must be decomposed into dedicated subcomponents, extracted custom hooks (`use...`), or pure helper modules.
  - *Immunity for Pre-Existing Files*: Untouched legacy components that historically exceed 350 lines enjoy historical immunity (§ 4.1). However, if a task modifies or expands such a file, or creates a new component exceeding 350 lines, it must be decomposed.
- **REJECTION TRIGGER**: Introducing a new component or expanding an existing component beyond 350 lines without decomposition.

---

## 3. Vitest Unit Testing Requirement for Pure Utilities & Slices
- **Rule**: Pure utility functions (formatting, parsing, diffing, data transformations) and Zustand store slices MUST have matching Vitest unit tests located in adjacent `__tests__/*.test.ts` suites.
- **Audit**:
  - Run `pnpm test` (Vitest v5.0.5) to ensure 100% test pass rate across all suites.
  - Verify that new utilities in `src/lib/`, `src/types/`, or store slices in `src/stores/` have explicit test coverage.
- **Anti-Mock Policy**: Tests must verify real deterministic transformations, parsers, and reducers. Never substitute live runtime sensor verification with synthetic mocks for external subprocesses or IPC.

---

## 4. Zustand `useShallow` Invariant
- **Rule**: All store selector calls returning object or array slices must be wrapped with `useShallow`:
  ```tsx
  import { useShallow } from 'zustand/react/shallow';
  const columns = useBoardStore(useShallow(s => s.columns));
  ```
- **Audit**: Inspect all new or modified Zustand store subscriptions in `src/`.
- **REJECTION TRIGGER**: Returning an object literal or array filter from a selector without `useShallow` causes infinite re-render loops and UI freezes. If violated, **REJECT IMMEDIATELY**.

---

## 5. Flexbox Boundaries (`min-h-0 min-w-0`)
- **Rule**: In multi-column and nested flex layouts (such as `AgentConsoleDrawer`), every flex container must include `min-h-0 min-w-0 overflow-hidden` (or `overflow-y-auto`).
- **Audit**: Check CSS classes on flex containers. Missing `min-h-0` causes flex children to ignore parent bounds and expand off-screen.

---

## 6. Dual Design System Fidelity (Light Paper / Obsidian Dark)
- **Rule**: UI components must strictly adhere to either Vercel Light Paper (`#fafafa`, `#171717`, `#ebebeb`) or Obsidian Dark (`#07090e`, `#0d1117`, `#161b22`, `#21262d`, `#f0f6fc`).
- **Audit**: Reject arbitrary neon gradients, random purple accent chips, or unstandardized shadows. Relational task families use standardized bidirectional hover glow (`ring-2 ring-[#8b5cf6]/50 bg-purple-50/20 dark:bg-[#1c1b29]`).

---

## 7. Zoom Isolation & Overlay Mounting
- **Rule**: Fullscreen overlays, dialogs, and drawers (`AgentConsoleDrawer`, modals) must be mounted at document root outside `useUiZoom`.
- **Audit**: Verify that modal portals are not nested inside zoomed canvas containers where matrix transforms distort mouse coordinates.

---

## 8. Tabular Numerics for Metrics
- **Rule**: Any dynamic numeric counter, timer, token pill, or TPS telemetry must use the `.tabular-nums` CSS class to eliminate visual layout jitter.

---

## 9. Redundant Copy Elimination & Structural Layout-First Invariant
- **Rule**: Redundant slogans, explanatory filler text, marketing subtitles, and minor helper labels must be eliminated (§ 2.39).
- **Audit**: If an interface becomes confusing or incomprehensible when explanatory copy is removed, the layout itself is flawed. Reject solutions that add explanatory text or tooltips to patch poor visual hierarchy; mandate redesigning the layout from scratch.

---

## 10. Tri-Partite Experience Synergy Invariant (UX, DX, AX)
- **Rule**: Every change must simultaneously preserve or improve User Experience (UX), Developer Experience (DX), and AI Agent Experience (AX) (§ 2.40).
- **Audit**: Verify that UX improvements do not compromise DX or AX, and verify zero regression in existing functionality.