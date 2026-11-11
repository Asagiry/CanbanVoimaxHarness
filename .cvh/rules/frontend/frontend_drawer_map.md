---
description: Agent drawer, stream, tool cards, diff inspectors, and telemetry UI.
---
# Agent drawer and conversation

The drawer is a root-level fixed overlay; scrollable flex children retain `min-h-0 min-w-0`. Activity identity and deduplication belong to the drawer slices. Provider usage and local context estimates stay distinct. All components and slices strictly observe the <=350 lines ceiling.

| File | Responsibility |
| --- | --- |
| `src/components/drawer/AgentConsoleDrawer.tsx` | Hoisted fixed overlay coordinator, layout bounds (`min-h-0 min-w-0`). |
| `src/components/drawer/useDrawerTaskState.ts` | Extracted drawer hook for active task, stage, and title state. |
| `src/components/drawer/useUnifiedActivities.ts` | Extracted drawer hook for activity feed merging, optimistic tools, and streaming thoughts. |
| `src/components/drawer/useAgentExecution.ts` | Extracted drawer hook for agent run, abort, and model trigger execution. |
| `src/components/drawer/DrawerHeader.tsx` | Task title, stage stepper and abort/run controls. |
| `src/components/drawer/stageStepperHelpers.ts` | Stage duration, running status, and task resolution logic. |
| `src/components/drawer/DrawerActivityStream.tsx` | Step commentary vs prominent terminal response card. |
| `src/components/drawer/QuestionPromptCard.tsx` | Interactive clarifying question card, single/multiple choice, custom input, markdown rendering. |
| `src/components/drawer/DrawerSteerBar.tsx` | Interactive steer input bar coordinator, optimistic submission. |
| `src/components/drawer/steer/SteerImageBar.tsx` | Attached image preview thumbnails with remove triggers. |
| `src/components/drawer/steer/SteerModelMenu.tsx` | Inline model selector dropdown for steer directives. |
| `src/components/drawer/steer/useSteerModelResolution.ts` | Hook resolving active model overrides and persona defaults for steer. |
| `src/components/drawer/steer/useSteerImages.ts` | Hook managing image clipboard paste, file drag-and-drop, and compression. |
| `src/components/drawer/steer/types.ts` | Shared types and prop interfaces for steer bar components. |
| `src/components/drawer/steer/index.ts` | Barrel re-exports for steer bar components. |
| `src/components/drawer/EffortSelector.tsx` | Reasoning effort selector (`off`/`low`/`medium`/`high` from `src/types/index.ts`; "Max" is a UI label for `high`) for tasks and personas. |
| `src/components/drawer/GitDiffSidebar.tsx` | Resizable git diff panel facade with live change updates. |
| `src/components/drawer/diff/sidebar/useDiffResize.ts` | Hook managing horizontal drag-to-resize width for git diff sidebar. |
| `src/components/drawer/diff/sidebar/useGitDiffData.ts` | Hook fetching task git diffs, changed files summary, and stats. |
| `src/components/drawer/diff/sidebar/DiffCollapsedRail.tsx` | Compact vertical rail when diff sidebar is collapsed. |
| `src/components/drawer/diff/sidebar/DiffHeader.tsx` | Header bar showing commit stats, file counts, and collapse toggle. |
| `src/components/drawer/diff/sidebar/index.ts` | Barrel re-exports for diff sidebar components. |
| `src/components/drawer/SubagentsSidebar.tsx` | Subagent hierarchy inspection sidebar coordinator. |
| `src/components/drawer/subagents/useSidebarResize.ts` | Hook managing horizontal drag-to-resize width for subagents sidebar. |
| `src/components/drawer/subagents/BoardSubtaskCard.tsx` | Card displaying linked board child subtasks and status pills. |
| `src/components/drawer/subagents/InProcessSubagentCard.tsx` | Card displaying running in-process subagents and activity steps. |
| `src/components/drawer/subagents/index.ts` | Barrel re-exports for subagent sidebar components. |
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
| `src/components/drawer/useContextGaugeMetrics.ts` | Context gauge coordinator hook orchestrating breakdown, telemetry, and compaction actions. |
| `src/components/drawer/metrics/types.ts` | Shared context gauge metric interfaces and hook options types. |
| `src/components/drawer/metrics/tokenMetrics.ts` | Pure token estimation, compaction log parsing, incremental char cache, and ring geometry. |
| `src/components/drawer/metrics/telemetryMetrics.ts` | Turn/step counters, tool duration aggregation, TPS/TTFT formatting, and token display summaries. |
| `src/components/drawer/metrics/useGaugeEffects.ts` | Effects for persona context limit sync, telemetry ledger sync, and manual compaction triggers. |
| `src/components/drawer/useActivityScroll.ts` | Activity feed scroll management, auto-scroll, and resize observer. |
| `src/components/drawer/diff/ImageDiffViewer.tsx` | Visual image diffs coordinator with multi-mode comparison. |
| `src/components/drawer/diff/image/ImageDiffLightbox.tsx` | Full-screen image diff lightbox with zoom controls. |
| `src/components/drawer/diff/image/ImageDiffTwoUp.tsx` | Side-by-side (2-up) comparison view for before/after images. |
| `src/components/drawer/diff/image/ImageDiffSplit.tsx` | Interactive split-slider comparison view with draggable divider. |
| `src/components/drawer/diff/image/ImageDiffOnion.tsx` | Onion-skin opacity crossfade comparison view. |
| `src/components/drawer/diff/image/ImageDiffSingle.tsx` | Single image fallback view for newly added/deleted assets. |
| `src/components/drawer/diff/image/types.ts` | Shared types and prop interfaces for image diff modes. |
| `src/components/drawer/diff/image/index.ts` | Barrel re-exports for image diff components. |
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
| `src/components/drawer/stream/ActivityItem.tsx` | Memoized per-item renderer extracted from DrawerActivityStream. |
| `src/components/drawer/stream/TerminalResponseCard.tsx` | Prominent final response card with copy and duration. |
| `src/components/drawer/stream/TaskObjectiveCard.tsx` | Task goal, clean prompt, and attachment gallery. |
| `src/components/drawer/stream/ExecutionErrorBanner.tsx` | In-stream error banner with retry and dismiss controls. |
| `src/components/drawer/stream/StageSummaryCard.tsx` | Dedicated stage handover summary card and reviewer verdict card. |
| `src/components/drawer/stream/index.ts` | Barrel re-exports for stream cards and helpers. |
| `src/components/drawer/stream/streamHelpers.ts` | Stream activity dedup, prompt cleanup, duration helpers. |
| `src/components/drawer/tools/TodoToolCard.tsx` | `todo_write` checklist progress bar, status icons, and task rendering. |
| `src/components/drawer/tools/TodoChecklist.tsx` | Reusable task checklist and progress rendering. |
| `src/components/drawer/tools/FileOpsToolCard.tsx` | File operations card coordinator (`read_file`, `write_file`, `edit_file`). |
| `src/components/drawer/tools/fileOps/FileOpsHeader.tsx` | Header bar showing file operation action, path, and stats. |
| `src/components/drawer/tools/fileOps/FileOpsDiffView.tsx` | Synthesized diff viewer for file replacements and edits. |
| `src/components/drawer/tools/fileOps/FileOpsReadView.tsx` | Formatted preview for read_file operations with line numbers. |
| `src/components/drawer/tools/fileOps/fileOpsUtils.ts` | Utilities for diff parsing, syntax language detection, and truncation. |
| `src/components/drawer/tools/fileOps/index.ts` | Barrel re-exports for file ops components. |
| `src/components/drawer/tools/CommandToolCard.tsx` | Command invocation, exit code, spilled logs link. |
| `src/components/drawer/tools/SubagentToolCard.tsx` | Subagent execution card coordinator. |
| `src/components/drawer/tools/subagent/SubagentStatusBadge.tsx` | Status pill (running, completed, failed, cancelled) with spinner. |
| `src/components/drawer/tools/subagent/SubagentListCard.tsx` | Card displaying list of registered subagent sessions. |
| `src/components/drawer/tools/subagent/AwaitedSubagentRow.tsx` | Status row for background subagents being actively awaited. |
| `src/components/drawer/tools/subagent/CollectSubagentCard.tsx` | Result card for `collect_subagent` displaying returned summaries. |
| `src/components/drawer/tools/subagent/SingleSubagentCard.tsx` | Detailed view for individual synchronous subagent runs. |
| `src/components/drawer/tools/subagent/index.ts` | Barrel re-exports for subagent tool card components. |
| `src/components/drawer/tools/CompactContextToolCard.tsx` | Optimistic compaction card with before/after stats. |
| `src/components/drawer/tools/GenericToolCard.tsx` | Fallback card for custom or generic tool calls. |
| `src/components/drawer/tools/SearchToolCard.tsx` | Search tool results card with hit highlighting. |
| `src/components/drawer/tools/WebToolCard.tsx` | Web search and web fetch status card. |
| `src/components/drawer/tools/shared.tsx` | Tool card frame, headers, and status icons. |
| `src/stores/agentSessionStore.ts` | Authoritative background agent execution tracker. |
| `src/stores/drawerStore.ts` | Composed Zustand drawer store (`useDrawerStore`) wiring drawer slices. |
| `src/stores/drawer/types.ts` | Drawer state, activity item, tab and telemetry contracts. |
| `src/stores/drawer/drawerHelpers.ts` | Drawer helper utilities. |
| `src/stores/drawer/activityLoadHelpers.ts` | `buildLoadedTaskActivityPatch`, `MAX_LOAD_ROLLBACK_RETRIES`, and SQLite activity hydration helpers for `activitySlice`. |
| `src/stores/drawer/activityDedup.ts` | Re-export facade for drawer activity deduplication and rolling-window helpers. |
| `src/stores/drawer/dedup/windowing.ts` | 500-item rolling-window caps (`capActivityFeed`, `capToolExecutions`) and log evaporation. |
| `src/stores/drawer/dedup/feedMerge.ts` | Final response merging and optimistic user/steer deduplication helpers. |
| `src/stores/drawer/dedup/toolMerge.ts` | Tool execution upsert/update and subagent ID extraction helpers. |
| `src/stores/drawer/dedup/hydration.ts` | SQLite dialog history deduplication and store state hydration. |
| `src/stores/drawer/activityQuestionHelpers.ts` | Interactive question activity helpers. |
| `src/stores/drawer/subagentListeners.ts` | Subagent event listener wiring for drawer slices. |
| `src/stores/drawer/index.ts` | Barrel re-exports for drawer store types, helpers and slices. |
| `src/stores/drawer/slices/drawerMetaSlice.ts` | `activeTaskId`, drawer open/close, view mode. |
| `src/stores/drawer/slices/activitySlice.ts` | `activityFeed`, optimistic tool injection, deduplication. |
| `src/stores/drawer/slices/streamSlice.ts` | Stream slice facade & lifecycle coordinator (`startStream`, `retryStream`). |
| `src/stores/drawer/slices/stream/chunkBuffer.ts` | RAF/timeout stream chunk batching buffer and rollback drop helpers. |
| `src/stores/drawer/slices/stream/appendActions.ts` | Store actions for appending streamed thought and assistant output chunks. |
| `src/stores/drawer/slices/stream/telemetryActions.ts` | Store actions for TTFT, TPS, token counters, and turn telemetry updates. |
| `src/stores/drawer/slices/subagentsSlice.ts` | Subagent lifecycle, active subtasks, subagent logs. |
| `src/stores/drawer/slices/draftsSlice.ts` | Per-task draft text persistence for steer input bar. |

## Related checks

Choose checks for the changed behavior; use the active role's `sensors` rule for execution policy.

- `src/stores/__tests__/streamRetry.test.ts` — Stream attempt rollback, buffered chunk ordering and subagent isolation.
- `src/stores/__tests__/toolTimeTracking.test.ts` — Vitest suite for tool execution duration, fallback, and metric tracking.
- `src/components/drawer/stream/__tests__/streamScrollContainment.test.ts` — Vitest suite for drawer feed overflow containment, token wrapping, and card bounds.
