import React, { useState, useMemo, useCallback } from 'react';
import type { KanbanTask } from '../../types';
import type { DrawerActivityItem } from '../../stores/drawerStore';
import ToolExecutionCard from './ToolExecutionCard';
import QuestionPromptCard from './QuestionPromptCard';
import CompactedContextCard from './CompactedContextCard';
import ImageLightboxModal from './ImageLightboxModal';
import { useDrawerStore } from '../../stores/drawerStore';
import { useBoardStore } from '../../stores/boardStore';
import { useTranslation } from '../../stores/languageStore';
import { ArrowDown, AlertTriangle, Info } from 'lucide-react';
import { useActivityScroll } from './useActivityScroll';
import {
  ThoughtStreamCard,
  SteerDirectiveCard,
  SubagentActivityCard,
  LightboxContext,
  AgentExplanationCard,
  TerminalResponseCard,
  TaskObjectiveCard,
  ExecutionErrorBanner,
  StageSummaryCard,
  isFinalResponseItem,
  hasNearbyTodoWrite,
  shouldSuppressExplanation,
  filterDisplayActivities,
  computeActivityDurations,
} from './stream';
import { tryExtractTodos } from '../../lib/markdown';

export { MarkdownContent, LightboxContext } from './stream';

export interface DrawerActivityStreamProps {
  task: KanbanTask;
  unifiedActivities: DrawerActivityItem[];
  isRunning: boolean;
  runError: string | null;
  onRunAgent: () => void;
  onClearRunError: () => void;
  subtasks?: KanbanTask[];
  onOpenDrawer?: (taskId: string) => void;
}

export default function DrawerActivityStream({
  task,
  unifiedActivities,
  isRunning,
  runError,
  onRunAgent,
  onClearRunError,
  subtasks,
}: DrawerActivityStreamProps) {
  const {
    activeSubagentId,
    activeSubagentTitle,
    activeSubagentRole,
    activeSubagentPrompt,
  } = useDrawerStore();
  const { t } = useTranslation();

  const effectiveAttachments =
    task.attachments && task.attachments.length > 0
      ? task.attachments
      : task.parentId
        ? subtasks?.find((t) => t.id === task.parentId)?.attachments ||
          useBoardStore.getState().tasks.find((t) => t.id === task.parentId)?.attachments
        : undefined;

  // Image Lightbox state & handlers (§ 2.13 Viewport Overlay & Image Lightbox Invariant)
  const [lightboxImage, setLightboxImage] = useState<{ src: string; alt?: string } | null>(null);

  const handleOpenLightbox = useCallback((src: string, alt?: string) => {
    setLightboxImage({ src, alt });
  }, []);

  const handleContainerClick = useCallback((e: React.MouseEvent) => {
    const target = e.target as HTMLElement;
    if (target.tagName === 'IMG') {
      const img = target as HTMLImageElement;
      if (img.src && !img.closest('button') && !img.classList.contains('no-lightbox')) {
        e.preventDefault();
        e.stopPropagation();
        handleOpenLightbox(img.src, img.alt || '');
      }
    }
  }, [handleOpenLightbox]);

  // Obeying § 2.20 & Question Tool Dedup:
  const displayActivities = useMemo(
    () => filterDisplayActivities(unifiedActivities, task?.id),
    [unifiedActivities, task?.id]
  );

  // Pre-compute elapsed duration for each activity item
  const activityDurations = useMemo(
    () => computeActivityDurations(displayActivities, isRunning),
    [displayActivities, isRunning]
  );

  const {
    scrollRef,
    collapsedOutputs,
    showNewButton,
    toggleOutput,
    scrollToBottom,
    handleFeedScroll,
  } = useActivityScroll(displayActivities, task?.id);

  return (
    <LightboxContext.Provider value={handleOpenLightbox}>
      <div className="relative flex-1 flex flex-col min-h-0 min-w-0 bg-[#fafafa] dark:bg-[#0b0b0e] overflow-hidden transition-colors">
        {/* Warning Banner on agent run error */}
        {runError && (
          <ExecutionErrorBanner
            error={runError}
            onRetry={onRunAgent}
            onDismiss={onClearRunError}
          />
        )}

        {/* Main Unified Activity Feed Scrollable Area */}
        <div
          ref={scrollRef}
          onScroll={handleFeedScroll}
          onClick={handleContainerClick}
          className="flex-1 min-h-0 overflow-y-auto p-4 md:p-6"
        >
          <div className="max-w-4xl mx-auto space-y-3.5">
            {activeSubagentId ? (
              <SubagentActivityCard
                activeSubagentId={activeSubagentId}
                activeSubagentTitle={activeSubagentTitle}
                activeSubagentRole={activeSubagentRole}
                activeSubagentPrompt={activeSubagentPrompt}
                effectiveAttachments={effectiveAttachments}
                onImageClick={handleOpenLightbox}
              />
            ) : (
              <TaskObjectiveCard
                task={task}
                effectiveAttachments={effectiveAttachments}
                onImageClick={handleOpenLightbox}
              />
            )}

            {/* Unified Chronological Activity Stream */}
            {(displayActivities ?? []).map((activity, idx) => {
              const key = activity?.id ?? `activity-${idx}`;

              // e) Interactive Question Prompt Card (Human-in-the-Loop)
              if (
                activity?.type === 'question' ||
                (activity?.type === 'tool' &&
                  (activity.tool?.toolName === 'question' || activity.tool?.toolName === 'ask_user'))
              ) {
                return (
                  <div key={key} className="animate-silk-entrance motion-reduce:animate-none">
                    <QuestionPromptCard activity={activity} taskId={task.id} />
                  </div>
                );
              }

              // Compacted context accordion card
              if (activity?.type === 'compacted') {
                return (
                  <div key={key} className="animate-silk-entrance motion-reduce:animate-none">
                    <CompactedContextCard activity={activity} />
                  </div>
                );
              }

              // b) Live Streaming Thinking Block
              if (activity?.type === 'thought') {
                const isLastActivity = idx === ((displayActivities?.length ?? 1) - 1);
                return (
                  <ThoughtStreamCard
                    key={key}
                    activity={activity}
                    isTaskRunning={isRunning}
                    isLive={isRunning && isLastActivity}
                    taskId={task.id}
                    durationMs={activityDurations[idx] || activity.durationMs}
                  />
                );
              }

              // d) Inline Tool Execution Card (including subagent tool executions)
              if (activity?.type === 'tool' && activity?.tool) {
                const toolWithDuration = {
                  ...activity.tool,
                  durationMs: activity.tool.durationMs || activityDurations[idx] || 0,
                };
                const currentBatch = activity.tool.batch;
                const nextActivity = displayActivities[idx + 1];
                const hasNextInBatch = Boolean(
                  currentBatch &&
                  currentBatch.total > 1 &&
                  currentBatch.index < currentBatch.total &&
                  nextActivity?.type === 'tool' &&
                  nextActivity.tool?.batch &&
                  nextActivity.tool.batch.total === currentBatch.total &&
                  nextActivity.tool.batch.index === currentBatch.index + 1
                );

                return (
                  <div key={key} className="relative animate-silk-entrance motion-reduce:animate-none">
                    <ToolExecutionCard tool={toolWithDuration} />
                    {hasNextInBatch && (
                      <div
                        data-testid="batch-chain-connector"
                        className="absolute left-4 -bottom-3.5 h-3.5 border-l-2 border-dashed border-blue-500/40 dark:border-blue-400/30 z-10 pointer-events-none"
                      />
                    )}
                  </div>
                );
              }

              // User steer message, agent message, or Gatekeeper Sensor rejection
              if (activity?.type === 'user' || activity?.type === 'steer' || (activity?.type as string) === 'agent') {
                return (
                  <SteerDirectiveCard
                    key={key}
                    activity={activity}
                    onImageClick={handleOpenLightbox}
                  />
                );
              }

              // Subagent Activity / Report Item
              if ((activity?.type as string) === 'subagent') {
                return (
                  <SubagentActivityCard
                    key={key}
                    activity={activity}
                    durationMs={activityDurations[idx] || activity.durationMs}
                    onImageClick={handleOpenLightbox}
                  />
                );
              }

              // System notices
              if (activity?.type === 'system') {
                return (
                  <div
                    key={key}
                    className="animate-silk-entrance motion-reduce:animate-none flex items-center space-x-2 rounded-md border border-[#ebebeb] dark:border-[#212126] bg-[#f4f4f5] dark:bg-[#17171b] px-3 py-2 text-xs text-[#4d4d4d] dark:text-[#a1a1aa] font-mono shadow-xs"
                  >
                    <Info className="h-3.5 w-3.5 text-[#666666] dark:text-[#71717a] shrink-0" />
                    <span className="flex-1">{activity?.content ?? ''}</span>
                  </div>
                );
              }

              // Error notices
              if (activity?.type === 'error') {
                return (
                  <div
                    key={key}
                    className="animate-silk-entrance motion-reduce:animate-none flex items-start space-x-2 rounded-md border border-[#fecaca] dark:border-[#da3633] bg-[#fef2f2] dark:bg-[#3b1219] p-3 text-xs text-[#991b1b] dark:text-[#ff7b72] font-mono shadow-xs"
                  >
                    <AlertTriangle className="h-4 w-4 text-[#dc2626] dark:text-[#ff7b72] shrink-0 mt-0.5" />
                    <div className="flex-1 whitespace-pre-wrap select-text">
                      {activity?.content ?? ''}
                    </div>
                  </div>
                );
              }

              // Dedicated Stage Summary Card (advance_task handover) or Reviewer Verdict Card
              if ((activity?.type as string) === 'summary' || (activity?.type as string) === 'reject') {
                return (
                  <StageSummaryCard
                    key={key}
                    activity={activity}
                    durationMs={activityDurations[idx] || activity.durationMs}
                    onImageClick={handleOpenLightbox}
                  />
                );
              }
              // c) Assistant Response: True Final Response vs Intermediate Step Commentary
              const isFinal = isFinalResponseItem(activity, idx, displayActivities ?? [], isRunning);

              if (!isFinal) {
                const nearbyTodo = hasNearbyTodoWrite(idx, displayActivities ?? []);
                if (shouldSuppressExplanation(activity, idx, displayActivities ?? [])) {
                  return null;
                }
                const explanationText = activity?.content;
                const todoData = explanationText ? tryExtractTodos(explanationText) : null;
                if (todoData && nearbyTodo && !todoData.leadingText) {
                  return null;
                }
                return (
                  <div key={key} className="animate-silk-entrance motion-reduce:animate-none">
                    <AgentExplanationCard
                      activity={activity}
                      content={activity?.content}
                      durationMs={activityDurations[idx] || activity?.durationMs}
                      hasNearbyTodoWrite={nearbyTodo}
                      onImageClick={handleOpenLightbox}
                    />
                  </div>
                );
              }

              // True Final Response / Output Markdown Block (collapsible card)
              return (
                <TerminalResponseCard
                  key={key}
                  activity={activity}
                  durationMs={activityDurations[idx] || activity?.durationMs}
                  isExpanded={!collapsedOutputs.has(key)}
                  onToggleExpanded={() => toggleOutput(key)}
                  onImageClick={handleOpenLightbox}
                />
              );
            })}
          </div>
        </div>

        {/* Floating Scroll to Bottom Button */}
        {showNewButton && (
          <button
            type="button"
            onClick={scrollToBottom}
            className="absolute bottom-4 left-1/2 -translate-x-1/2 flex items-center space-x-1.5 px-3 py-1.5 rounded-md bg-[#171717] hover:bg-[#262626] text-white text-xs font-medium font-sans shadow-md transition cursor-pointer active:scale-[0.98]"
            aria-label={t('drawer.scrollToNewTitle')}
          >
            <ArrowDown className="h-3.5 w-3.5" />
            <span>{t('drawer.scrollToNew')}</span>
          </button>
        )}

        {/* Obeying § 2.13 Viewport Overlay & Image Lightbox Invariants */}
        {lightboxImage && (
          <ImageLightboxModal
            src={lightboxImage.src}
            alt={lightboxImage.alt}
            onClose={() => setLightboxImage(null)}
          />
        )}
      </div>
    </LightboxContext.Provider>
  );
}
