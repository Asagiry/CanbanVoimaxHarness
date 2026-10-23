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
- **Audit**: Reject arbitrary neon gradients, random purple accent chips, or unstandardized shadows.

---

## 4. Zoom Isolation & Overlay Mounting
- **Rule**: Fullscreen overlays, dialogs, and drawers (`AgentConsoleDrawer`, modals) must be mounted at document root outside `useUiZoom`.
- **Audit**: Verify that modal portals are not nested inside zoomed canvas containers where matrix transforms distort mouse coordinates.

---

## 5. Tabular Numerics for Metrics
- **Rule**: Any dynamic numeric counter, timer, token pill, or TPS telemetry must use the `.tabular-nums` CSS class to eliminate visual layout jitter.
