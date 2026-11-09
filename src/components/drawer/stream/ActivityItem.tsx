import React, { memo, useMemo } from 'react';
import { AlertTriangle, Info } from 'lucide-react';
import type { DrawerActivityItem } from '../../../stores/drawerStore';
import ToolExecutionCard from '../ToolExecutionCard';
import QuestionPromptCard from '../QuestionPromptCard';
import CompactedContextCard from '../CompactedContextCard';
import ActivityItemActions from './ActivityItemActions';
import {
  ThoughtStreamCard,
  SteerDirectiveCard,
  SubagentActivityCard,
  AgentExplanationCard,
  TerminalResponseCard,
  StageSummaryCard,
} from './index';
import type { ActivityFlag } from './streamHelpers';

export interface ActivityItemProps {
  activity: DrawerActivityItem;
  idx: number;
  isRunning: boolean;
  isLast: boolean;
  durationMs?: number;
  isExpanded: boolean;
  onToggleExpanded: () => void;
  onImageClick: (src: string, alt?: string) => void;
  taskId: string;
  hasNextInBatch: boolean;
  nextActivity?: DrawerActivityItem;
  flag?: ActivityFlag;
}

function ActivityItemInner({
  activity,
  isRunning,
  isLast,
  durationMs,
  isExpanded,
  onToggleExpanded,
  onImageClick,
  taskId,
  hasNextInBatch,
  flag,
}: ActivityItemProps) {
  let card: React.ReactNode = null;

  if (
    activity?.type === 'question' ||
    (activity?.type === 'tool' &&
      (activity.tool?.toolName === 'question' || activity.tool?.toolName === 'ask_user'))
  ) {
    card = (
      <div className="animate-silk-entrance motion-reduce:animate-none activity-card-cv">
        <QuestionPromptCard activity={activity} taskId={taskId} />
      </div>
    );
  } else if (activity?.type === 'compacted') {
    card = (
      <div className="animate-silk-entrance motion-reduce:animate-none activity-card-cv">
        <CompactedContextCard activity={activity} />
      </div>
    );
  } else if (activity?.type === 'thought') {
    card = (
      <ThoughtStreamCard
        activity={activity}
        isTaskRunning={isRunning}
        isLive={isRunning && isLast}
        taskId={taskId}
        durationMs={durationMs || activity.durationMs}
        isExpanded={isExpanded}
        onToggleExpanded={onToggleExpanded}
      />
    );
  } else if (activity?.type === 'tool' && activity?.tool) {
    if (!(activity.tool.toolName === 'compact_context' && activity.tool.status === 'success')) {
      const tool = activity.tool;
      const toolWithDuration = useMemo(
        () => ({
          ...tool,
          durationMs: tool.durationMs || durationMs || 0,
        }),
        [tool, durationMs]
      );
      card = (
        <div className="relative animate-silk-entrance motion-reduce:animate-none activity-card-cv">
          <ToolExecutionCard tool={toolWithDuration} isExpanded={isExpanded} onToggleExpanded={onToggleExpanded} />
          {hasNextInBatch && (
            <div
              data-testid="batch-chain-connector"
              className="absolute left-4 -bottom-3.5 h-3.5 border-l-2 border-dashed border-blue-500/40 dark:border-blue-400/30 z-10 pointer-events-none"
            />
          )}
        </div>
      );
    }
  } else if (
    activity?.type === 'user' ||
    activity?.type === 'steer' ||
    (activity?.type as string) === 'agent'
  ) {
    card = <SteerDirectiveCard activity={activity} onImageClick={onImageClick} />;
  } else if ((activity?.type as string) === 'subagent') {
    card = (
      <SubagentActivityCard
        activity={activity}
        durationMs={durationMs || activity.durationMs}
        onImageClick={onImageClick}
      />
    );
  } else if (activity?.type === 'system') {
    card = (
      <div className="animate-silk-entrance motion-reduce:animate-none activity-card-cv flex items-center space-x-2 rounded-md border border-[#ebebeb] dark:border-[#212126] bg-[#f4f4f5] dark:bg-[#17171b] px-3 py-2 text-xs text-[#4d4d4d] dark:text-[#a1a1aa] font-mono shadow-xs">
        <Info className="h-3.5 w-3.5 text-[#666666] dark:text-[#71717a] shrink-0" />
        <span className="flex-1">{activity?.content ?? ''}</span>
      </div>
    );
  } else if (activity?.type === 'error') {
    card = (
      <div className="animate-silk-entrance motion-reduce:animate-none activity-card-cv flex items-start space-x-2 rounded-md border border-[#fecaca] dark:border-[#da3633] bg-[#fef2f2] dark:bg-[#3b1219] p-3 text-xs text-[#991b1b] dark:text-[#ff7b72] font-mono shadow-xs">
        <AlertTriangle className="h-4 w-4 text-[#dc2626] dark:text-[#ff7b72] shrink-0 mt-0.5" />
        <div className="flex-1 whitespace-pre-wrap select-text">{activity?.content ?? ''}</div>
      </div>
    );
  } else if ((activity?.type as string) === 'summary' || (activity?.type as string) === 'reject') {
    card = (
      <StageSummaryCard
        activity={activity}
        durationMs={durationMs || activity.durationMs}
        onImageClick={onImageClick}
      />
    );
  } else {
    const isFinal = flag?.isFinal ?? false;
    if (!isFinal) {
      const nearbyTodo = flag?.nearbyTodo ?? false;
      if (flag?.suppressed) {
        card = null;
      } else if (flag?.todoData && nearbyTodo && !flag.todoData.leadingText) {
        card = null;
      } else {
        card = (
          <div className="animate-silk-entrance motion-reduce:animate-none activity-card-cv">
            <AgentExplanationCard
              activity={activity}
              content={activity?.content}
              durationMs={durationMs || activity?.durationMs}
              hasNearbyTodoWrite={nearbyTodo}
              onImageClick={onImageClick}
            />
          </div>
        );
      }
    } else {
      card = (
        <TerminalResponseCard
          activity={activity}
          durationMs={durationMs || activity?.durationMs}
          isExpanded={isExpanded}
          onToggleExpanded={onToggleExpanded}
          onImageClick={onImageClick}
        />
      );
    }
  }

  if (!card) return null;

  return (
    <ActivityItemActions taskId={taskId} itemId={activity.id} disabled={isRunning}>
      {card}
    </ActivityItemActions>
  );
}

function areEqual(prev: ActivityItemProps, next: ActivityItemProps): boolean {
  if (prev.activity?.id !== next.activity?.id) return false;
  if ((prev.activity?.content?.length ?? 0) !== (next.activity?.content?.length ?? 0)) return false;
  if (prev.activity?.tool?.status !== next.activity?.tool?.status) return false;
  if (prev.activity?.tool?.durationMs !== next.activity?.tool?.durationMs) return false;
  if (prev.isExpanded !== next.isExpanded) return false;
  if (prev.isRunning !== next.isRunning) return false;
  if (prev.isLast !== next.isLast) return false;
  if (prev.durationMs !== next.durationMs) return false;
  if (prev.hasNextInBatch !== next.hasNextInBatch) return false;
  if (prev.taskId !== next.taskId) return false;
  if (prev.flag !== next.flag) return false;
  if (prev.activity?.type !== next.activity?.type) return false;
  if (prev.activity?.tool?.toolName !== next.activity?.tool?.toolName) return false;
  if (prev.activity?.tool?.id !== next.activity?.tool?.id) return false;
  if ((prev.activity?.tool?.stdout?.length ?? 0) !== (next.activity?.tool?.stdout?.length ?? 0)) return false;
  if ((prev.activity?.tool?.stderr?.length ?? 0) !== (next.activity?.tool?.stderr?.length ?? 0)) return false;
  if (prev.activity?.tool?.params !== next.activity?.tool?.params) return false;
  if (prev.activity?.tool?.batch?.index !== next.activity?.tool?.batch?.index) return false;
  if (prev.activity?.tool?.batch?.total !== next.activity?.tool?.batch?.total) return false;
  if (prev.activity?.isFinal !== next.activity?.isFinal) return false;
  if (prev.activity?.durationMs !== next.activity?.durationMs) return false;
  return true;
}

export const ActivityItem = memo(ActivityItemInner, areEqual);
