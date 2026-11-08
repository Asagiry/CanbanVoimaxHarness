---
description: Agent drawer, stream, tool cards and telemetry UI.
---
# Agent drawer and conversation

The drawer is a root-level fixed overlay; scrollable flex children retain `min-h-0 min-w-0`. Activity identity and deduplication belong to the drawer slices. Provider usage and local context estimates stay distinct.

| File | Responsibility |
| --- | --- |
| `src/components/drawer/AgentConsoleDrawer.tsx` | Hoisted fixed overlay, flex bounded (`min-h-0 min-w-0`). |
| `src/components/drawer/DrawerHeader.tsx` | Task title, stage stepper and abort/run controls. |
| `src/components/drawer/stageStepperHelpers.ts` | Stage duration, running status, and task resolution logic. |
| `src/components/drawer/DrawerActivityStream.tsx` | Step commentary vs prominent terminal response card. |
| `src/components/drawer/QuestionPromptCard.tsx` | Interactive clarifying question card, single/multiple choice, custom input, markdown rendering. |
| `src/components/drawer/DrawerSteerBar.tsx` | Interactive steer input bar, optimistic submission. |
| `src/components/drawer/EffortSelector.tsx` | Reasoning effort selector (`off`/`low`/`medium`/`high` from `src/types/index.ts`; "Max" is a UI label for `high`) for tasks and personas. |
| `src/components/drawer/GitDiffSidebar.tsx` | Resizable git diff panel with live change updates. |
| `src/components/drawer/SubagentsSidebar.tsx` | Subagent hierarchy, active status, subtask logs. |
| `src/components/drawer/TaskTodoWidget.tsx` | Drawer-hoisted checklist progress bar and task item list. |
| `src/components/drawer/CompactedContextCard.tsx` | Notification banner card displaying token compaction savings. |
| `src/components/drawer/ImageLightboxModal.tsx` | Canonical owner of the attachment image lightbox modal (board copy is a re-export). |
| `src/components/drawer/ThinkingCard.tsx` | Real-time reasoning chunk renderer with animated pulse. |
| `src/components/drawer/ToolExecutionCard.tsx` | Collapsible card for standard tool invocations and results. |
| `src/components/drawer/ContextGauge.tsx` | Real-time context token gauge, status bar & header ring. |
| `src/components/drawer/SessionStatsPopover.tsx` | LLM/tool time, TTFT and TPS statistics. |
| `src/components/drawer/TokenUsagePopover.tsx` | Cache ratio, total input, cached/uncached input and output. |
| `src/components/drawer/ContextPopover.tsx` | Proportional segment breakdown popover, compaction trigger. |
| `src/components/drawer/telemetryFormatters.ts` | Compact time, TTFT, TPS, and localized token formatters. |
| `src/components/drawer/useContextGaugeMetrics.ts` | Extracted context resolution, token counting, and compaction listeners. |
| `src/components/drawer/useActivityScroll.ts` | Activity feed scroll management, auto-scroll, and resize observer. |
| `src/components/drawer/diff/ImageDiffViewer.tsx` | Visual image diffs, side-by-side and slider modes. |
| `src/components/drawer/diff/DiffFileList.tsx` | Changed files sidebar, line additions/deletions. |
| `src/components/drawer/diff/index.ts` | Barrel re-exports for diff viewers and utils. |
| `src/components/drawer/diff/utils.ts` | Diff parsing/formatting helpers (`parseCleanDiff`, `isImageFile`, `formatBytes`). |
| `src/components/drawer/subagentHelpers.ts` | Subagent display title/role label formatting helpers. |
| `src/components/drawer/stream/ThoughtStreamCard.tsx` | Collapsible reasoning stream card with duration badge. |
| `src/components/drawer/stream/SteerDirectiveCard.tsx` | In-stream steer instruction indicator. |
| `src/components/drawer/stream/SubagentActivityCard.tsx` | Child agent delegation summary and report view. |
| `src/components/drawer/stream/MarkdownContent.tsx` | Streaming markdown tokens, copyable codeblocks, lightbox. |
| `src/components/drawer/stream/AgentExplanationCard.tsx` | Intermediate step commentary, action plan, and todo list. |
| `src/components/drawer/stream/ActivityItemActions.tsx` | Stopped-dialog deletion with visible persistence errors. |
| `src/components/drawer/stream/TerminalResponseCard.tsx` | Prominent final response card with copy and duration. |
| `src/components/drawer/stream/TaskObjectiveCard.tsx` | Task goal, clean prompt, and attachment gallery. |
| `src/components/drawer/stream/ExecutionErrorBanner.tsx` | In-stream error banner with retry and dismiss controls. |
| `src/components/drawer/stream/StageSummaryCard.tsx` | Dedicated stage handover summary card and reviewer verdict card. |
| `src/components/drawer/stream/index.ts` | Barrel re-exports for stream cards and helpers. |
| `src/components/drawer/stream/streamHelpers.ts` | Stream activity dedup, prompt cleanup, duration helpers. |
| `src/components/drawer/tools/TodoToolCard.tsx` | `todo_write` checklist progress bar, status icons, and task rendering. |
| `src/components/drawer/tools/TodoChecklist.tsx` | Reusable task checklist and progress rendering. |
| `src/components/drawer/tools/FileOpsToolCard.tsx` | `read_file`, `write_file`, `edit_file` diff display. |
| `src/components/drawer/tools/CommandToolCard.tsx` | Command invocation, exit code, spilled logs link. |
| `src/components/drawer/tools/SubagentToolCard.tsx` | Child agent execution card with progress spinner. |
| `src/components/drawer/tools/CompactContextToolCard.tsx` | Optimistic compaction card with before/after stats. |
| `src/components/drawer/tools/GenericToolCard.tsx` | Fallback card for custom or generic tool calls. |
| `src/components/drawer/tools/SearchToolCard.tsx` | Search tool results card with hit highlighting. |
| `src/components/drawer/tools/WebToolCard.tsx` | Web search and web fetch status card. |
| `src/components/drawer/tools/shared.tsx` | Tool card frame, headers, and status icons. |
| `src/stores/agentSessionStore.ts` | Authoritative background agent execution tracker. |
| `src/stores/drawerStore.ts` | Composed Zustand drawer store (`useDrawerStore`) wiring drawer slices. |
| `src/stores/drawer/types.ts` | Drawer state, activity item, tab and telemetry contracts. |
| `src/stores/drawer/drawerHelpers.ts` | Drawer helper utilities. |
| `src/stores/drawer/activityDedup.ts` | Activity feed deduplication helpers. |
| `src/stores/drawer/activityQuestionHelpers.ts` | Interactive question activity helpers. |
| `src/stores/drawer/subagentListeners.ts` | Subagent event listener wiring for drawer slices. |
| `src/stores/drawer/index.ts` | Barrel re-exports for drawer store types, helpers and slices. |
| `src/stores/drawer/slices/drawerMetaSlice.ts` | `activeTaskId`, drawer open/close, view mode. |
| `src/stores/drawer/slices/activitySlice.ts` | `activityFeed`, optimistic tool injection, deduplication. |
| `src/stores/drawer/slices/streamSlice.ts` | Live streaming chunks, thoughts, token telemetry. |
| `src/stores/drawer/slices/subagentsSlice.ts` | Subagent lifecycle, active subtasks, subagent logs. |
| `src/stores/drawer/slices/draftsSlice.ts` | Per-task draft text persistence for steer input bar. |

## Related checks

Choose checks for the changed behavior; use the active role's `sensors` rule for execution policy.

- `src/stores/__tests__/streamRetry.test.ts` — Stream attempt rollback, buffered chunk ordering and subagent isolation.

- `src/stores/__tests__/toolTimeTracking.test.ts` — Vitest suite for tool execution duration, fallback, and metric tracking.
- `src/components/drawer/stream/__tests__/streamScrollContainment.test.ts` — Vitest suite for drawer feed overflow containment, token wrapping, and card bounds.
