# Rule: Frontend UI Invariants & Performance Audit (frontend-invariants.md)

Authoritative frontend audit checklist for the **Reviewer** persona in CanbanVoimaxHarness.

---

## 1. Zustand `useShallow` Invariant
- **Rule**: All store selector calls returning object or array slices must be wrapped with `useShallow`:
  ```tsx
  import { useShallow } from 'zustand/react/shallow';
  const columns = useBoardStore(useShallow(s => s.columns));
  ```
- **Audit**: Inspect all new or modified Zustand store subscriptions in `src/`.
- **REJECTION TRIGGER**: Returning an object literal or array filter from a selector without `useShallow` causes infinite re-render loops and UI freezes. If violated, **REJECT IMMEDIATELY**.

---

## 2. Flexbox Boundaries (`min-h-0 min-w-0`)
- **Rule**: In multi-column and nested flex layouts (such as `AgentConsoleDrawer`), every flex container must include `min-h-0 min-w-0 overflow-hidden` (or `overflow-y-auto`).
- **Audit**: Check CSS classes on flex containers. Missing `min-h-0` causes flex children to ignore parent bounds and expand off-screen.

---

## 3. Dual Design System Fidelity (Light Paper / Obsidian Dark)
- **Rule**: UI components must strictly adhere to either Vercel Light Paper (`#fafafa`, `#171717`, `#ebebeb`) or Obsidian Dark (`#090a0f`, `#0d1117`, `#21262d`, `#f0f6fc`).
- **Audit**: Reject arbitrary neon gradients, random purple accent chips, or unstandardized shadows. Relational task families use standardized bidirectional hover glow (`ring-2 ring-[#8b5cf6]/50 bg-purple-50/20 dark:bg-[#1c1b29]`).

---

## 4. Zoom Isolation & Overlay Mounting
- **Rule**: Fullscreen overlays, dialogs, and drawers (`AgentConsoleDrawer`, modals) must be mounted at document root outside `useUiZoom`.
- **Audit**: Verify that modal portals are not nested inside zoomed canvas containers where matrix transforms distort mouse coordinates.

---

## 5. Tabular Numerics for Metrics
- **Rule**: Any dynamic numeric counter, timer, token pill, or TPS telemetry must use the `.tabular-nums` CSS class to eliminate visual layout jitter.

---

## 6. Redundant Copy Elimination & Structural Layout-First Invariant
- **Rule**: Redundant slogans, explanatory filler text, marketing subtitles, and minor helper labels must be eliminated.
- **Audit**: If an interface becomes confusing or incomprehensible when explanatory copy is removed, the layout itself is flawed. Reject solutions that add explanatory text or tooltips to patch poor visual hierarchy; mandate redesigning the layout from scratch.

---

## 7. Tri-Partite Experience Synergy Invariant (UX, DX, AX)
- **Rule**: Every change must simultaneously preserve or improve User Experience (UX), Developer Experience (DX), and AI Agent Experience (AX).
- **Audit**: Verify that UX improvements do not compromise DX or AX, and verify zero regression in existing functionality.
