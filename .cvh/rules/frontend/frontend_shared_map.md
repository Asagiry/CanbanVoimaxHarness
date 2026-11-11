---
description: App shell, event gateway, shared UI, i18n and utilities.
---
# App shell and shared frontend

Typed Tauri events enter through `tauriEventGateway.ts` and the modular `src/services/gateway/` router, which batches frequent stream updates. Shared UI uses focused Zustand selectors. Keep translations in the typed i18n dictionaries. All files strictly observe the <=350 lines ceiling.

| File | Responsibility |
| --- | --- |
| `src/main.tsx` | React entrypoint mounting the application. |
| `src/index.css` | Global styles and Tailwind entry. |
| `src/App.tsx` | Application composition, providers, bootstrap and overlays. |
| `src/components/splash/SplashScreen.tsx` | Boot screen displayed while backend state synchronizes. |
| `src/components/layout/WindowControls.tsx` | Tauri window controls (minimize, maximize, close). |
| `src/components/layout/Topbar.tsx` | Board selector, active project, search filter. |
| `src/components/common/AgentIcon.tsx` | SVG/Lucide persona icon with nominal color mapping. |
| `src/components/common/BrandFileIcons.tsx` | File extension brand iconography (TS, RS, JSON, MD). |
| `src/components/common/ErrorBoundary.tsx` | React 19 error boundary preventing entire app unmount. |
| `src/components/common/Logo.tsx` | CanbanVoimaxHarness application logo. |
| `src/components/common/PersonaSelect.tsx` | Persona select dropdown with colored avatar badges. |
| `src/components/common/RuntimeRuleIcon.tsx` | Dedicated SVG prompt-chip icon and `isRuntimeRule` helper. |
| `src/components/common/HookIcon.tsx` | Visual icon resolver for on_enter (ArrowRightToLine) vs on_leave/on_finish (ArrowRightFromLine) hooks. |
| `src/hooks/useUiZoom.ts` | Ctrl+Plus/Minus/Wheel viewport scaling. |
| `src/services/notificationService.ts` | Desktop notifications, user preferences and click navigation. |
| `src/services/notificationFormatter.ts` | Markdown plain-text normalization, word-boundary truncation, and emoji-free titles. |
| `src/services/tauriEventGateway.ts` | Public facade re-exporting typed event gateway router. |
| `src/services/gateway/types.ts` | Event mapping contracts (`AgentEventMap`, `TaskEventMap`, etc.). |
| `src/services/gateway/textBuffer.ts` | Streaming text ring buffer with 150ms flush micro-batching. |
| `src/services/gateway/helpers.ts` | Pure event payload validators and extraction helpers. |
| `src/services/gateway/agentEventRouter.ts` | Agent thought, tool, and error event routing to drawer stores. |
| `src/services/gateway/taskEventRouter.ts` | Task lifecycle, timer, and stage transition event routing to board stores. |
| `src/services/gateway/index.ts` | Barrel re-exports for event gateway service. |
| `src/stores/projectStore.ts` | Multi-project workspace switching, active project persistence. |
| `src/stores/project/projectHelpers.ts` | `DEFAULT_PROJECT`, `isInvalidProjectPath`, opened-projects storage and store reset helpers for `projectStore`. |
| `src/stores/themeStore.ts` | Theme selection and persistence. |
| `src/stores/languageStore.ts` | Russian/English localization switching. |
| `src/i18n/translations.ts` | Localized translation resolver. |
| `src/i18n/ru.ts` | Russian translation entries. |
| `src/i18n/en.ts` | English translation entries. |
| `src/i18n/types.ts` | Nominal types and key interfaces for i18n dictionaries. |
| `src/i18n/index.ts` | Re-exports for translation helpers. |
| `src/lib/agentPalette.ts` | Theme-safe color constants for personas. |
| `src/lib/columnIcons.ts` | Lucide icon resolvers for stages. |
| `src/lib/imageCompression.ts` | Attachment image resizer and optimizer. |
| `src/lib/logger.ts` | Structured browser logs with backend routing. |
| `src/lib/slug.ts` | Deterministic worktree and git branch naming. |
| `src/lib/markdown/parseMarkdownTokens.ts` | AST token parser for codeblocks, tables, inline styling. |
| `src/lib/markdown/types.ts` | AST interfaces for markdown tokens. |
| `src/lib/markdown/index.ts` | Re-exports for markdown parser. |
| `src/types/domain.ts` | `TaskId`, `StageId`, `BoardId`, `ProviderId`, `TaskState`, `StageLike`, `CANONICAL_TERMINAL_STAGES`, `CANONICAL_INTAKE_STAGES`, `isTerminalStage`, `isIntakeStage`. |
| `src/types/index.ts` | Main types barrel (re-exports `domain.ts`); `ReasoningEffort`, `REASONING_EFFORTS`, `normalizeReasoningEffort`. |
| `src/types/kanban.ts` | Kanban task/board domain types. |
| `src/types/runtime.ts` | Extracted runtime types (`ReasoningEffort`, `CortexAgentFile`, `CortexAgentStage`, `ToolExecution`, etc.) re-exported by `src/types/index.ts`. |

## Related checks

Choose checks for the changed behavior; use the active role's `sensors` rule for execution policy.

- `src/hooks/__tests__/useUiZoom.test.ts` — Vitest suite for UI zoom scaling, modal overlays, and wheel listeners.
- `src/components/common/__tests__/HookIcon.test.tsx` — Vitest suite for hook moment resolution and visual icon rendering.
- `src/services/__tests__/notificationFormatter.test.ts` — Notification markdown formatter checks.
- `src/services/__tests__/tauriEventGateway.test.ts` — Tauri event gateway routing, buffer disposal, and project switch reset tests.
