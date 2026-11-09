import React, { useState, useMemo, useCallback } from 'react';
import type { KanbanTask } from '../../types';
import type { DrawerActivityItem } from '../../stores/drawerStore';
import ImageLightboxModal from './ImageLightboxModal';
import { useShallow } from 'zustand/react/shallow';
import { useDrawerStore } from '../../stores/drawerStore';
import { useBoardStore } from '../../stores/boardStore';
import { useTranslation } from '../../stores/languageStore';
import { ArrowDown, WifiOff } from 'lucide-react';
import { useActivityScroll } from './useActivityScroll';
import {
  SubagentActivityCard,
  LightboxContext,
  TaskObjectiveCard,
  ExecutionErrorBanner,
  ActivityItem,
  filterDisplayActivities,
  computeActivityDurations,
  computeActivityFlags,
} from './stream';

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
  } = useDrawerStore(
    useShallow((s) => ({
      activeSubagentId: s.activeSubagentId,
      activeSubagentTitle: s.activeSubagentTitle,
      activeSubagentRole: s.activeSubagentRole,
      activeSubagentPrompt: s.activeSubagentPrompt,
    }))
  );
  const { t } = useTranslation();
  const connectionWait = useDrawerStore((state) => {
    const taskId = activeSubagentId || task.id;
    return state.connectionWait[state.toolIdToSubagentId[taskId] || taskId];
  });

  const effectiveAttachments =
    task.attachments && task.attachments.length > 0
      ? task.attachments
      : task.parentId
        ? subtasks?.find((t) => t.id === task.parentId)?.attachments ||
          useBoardStore.getState().tasks.find((t) => t.id === task.parentId)?.attachments
        : undefined;

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

  const displayActivities = useMemo(
    () => filterDisplayActivities(unifiedActivities, task?.id),
    [unifiedActivities, task?.id]
  );

  const activityDurations = useMemo(
    () => computeActivityDurations(displayActivities, isRunning),
    [displayActivities, isRunning]
  );

  const activityFlags = useMemo(
    () => computeActivityFlags(displayActivities ?? [], isRunning),
    [displayActivities, isRunning]
  );

  const {
    scrollRef,
    showNewButton,
    isCollapsed,
    toggleCollapsed,
    scrollToBottom,
    handleFeedScroll,
  } = useActivityScroll(displayActivities, task?.id);

  const handleToggleOutput = useCallback(
    (key: string, idx: number, total: number, activity?: DrawerActivityItem, isFinal?: boolean) =>
      toggleCollapsed(key, idx, total, activity, isFinal),
    [toggleCollapsed]
  );

  return (
    <LightboxContext.Provider value={handleOpenLightbox}>
      <div className="relative flex-1 flex flex-col min-h-0 min-w-0 bg-[#fafafa] dark:bg-[#0b0b0e] overflow-hidden transition-colors">
        {connectionWait && (
          <div role="status" className="flex items-center gap-2 px-4 py-2 text-xs text-[#8f8f8f] dark:text-[#71717a]">
            <WifiOff size={14} aria-hidden="true" className="shrink-0" />
            <span>{t('drawer.waitingForConnection')}</span>
          </div>
        )}
        {runError && (
          <ExecutionErrorBanner
            error={runError}
            onRetry={onRunAgent}
            onDismiss={onClearRunError}
          />
        )}

        <div
          ref={scrollRef}
          onScroll={handleFeedScroll}
          onClick={handleContainerClick}
          className="flex-1 min-h-0 overflow-y-auto overflow-x-hidden p-4 md:p-6"
        >
          <div className="w-full max-w-4xl mx-auto space-y-3.5 min-w-0">
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

            {(displayActivities ?? []).map((activity, idx) => {
              const key = activity?.id ?? `activity-${idx}`;
              const nextActivity = displayActivities[idx + 1];
              const currentBatch = activity?.tool?.batch;
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
                <ActivityItem
                  key={key}
                  activity={activity}
                  idx={idx}
                  isRunning={isRunning}
                  isLast={idx === ((displayActivities?.length ?? 1) - 1)}
                  durationMs={activityDurations[idx]}
                  isExpanded={!isCollapsed(key, idx, displayActivities?.length ?? 0, activity, activityFlags[idx]?.isFinal)}
                  onToggleExpanded={() =>
                    handleToggleOutput(key, idx, displayActivities?.length ?? 0, activity, activityFlags[idx]?.isFinal)
                  }
                  onImageClick={handleOpenLightbox}
                  taskId={activeSubagentId || task.id}
                  hasNextInBatch={hasNextInBatch}
                  nextActivity={nextActivity}
                  flag={activityFlags[idx]}
                />
              );
            })}
          </div>
        </div>

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
