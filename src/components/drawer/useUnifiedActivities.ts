import { useMemo } from 'react';
import type { KanbanTask } from '../../types';
import { useDrawerStore, DrawerActivityItem } from '../../stores/drawerStore';
import { useShallow } from 'zustand/react/shallow';

export function useUnifiedActivities(
  activeTaskId: string | null,
  activeSubagentId: string | null,
  task?: KanbanTask
) {
  const rawActiveFeed = useDrawerStore(useShallow((s) => {
    if (activeSubagentId) {
      const subFeed = s.activityFeed[activeSubagentId] || [];
      if (subFeed.length > 0) return subFeed;
      const mappedId = s.toolIdToSubagentId[activeSubagentId];
      if (mappedId && s.activityFeed[mappedId]?.length) return s.activityFeed[mappedId];
      for (const [toolId, subId] of Object.entries(s.toolIdToSubagentId)) {
        if (subId === activeSubagentId && s.activityFeed[toolId]?.length) return s.activityFeed[toolId];
      }
      return subFeed;
    }
    return activeTaskId ? (s.activityFeed[activeTaskId] || []) : [];
  }));

  const activeFeed = useMemo(
    () => (activeSubagentId ? rawActiveFeed.filter((item) => item.type !== 'prompt') : rawActiveFeed),
    [activeSubagentId, rawActiveFeed]
  );

  const activeStream = useDrawerStore(useShallow((s) =>
    activeTaskId ? s.streamEntries[activeTaskId] : undefined
  ));

  const activeTaskTools = useDrawerStore(useShallow((s) =>
    activeTaskId ? s.toolExecutions[activeTaskId] : undefined
  ));

  const activeSubagentTools = useDrawerStore(useShallow((s) =>
    activeSubagentId ? s.toolExecutions[activeSubagentId] : undefined
  ));

  const isAgentActive = useDrawerStore((s) => Boolean(
    (activeTaskId && s.activeAgentTaskIds[activeTaskId]) ||
    (activeSubagentId && s.activeAgentTaskIds[activeSubagentId])
  ));

  const unifiedActivities = useMemo(() => {
    if (activeSubagentId) {
      return activeFeed;
    }

    if (!task) return [];

    if (activeFeed && activeFeed.length > 0) {
      return activeFeed;
    }

    const rawStream = activeStream || { thoughts: [], outputs: [] };
    const rawTools = activeTaskTools || [];

    const merged: DrawerActivityItem[] = [
      ...(rawStream.thoughts ?? []).map((th) => ({
        id: th?.id ?? `th-${Date.now()}`,
        type: 'thought' as const,
        timestamp: th?.timestamp ?? '',
        content: th?.content ?? '',
        durationMs: th?.durationMs,
      })),
      ...(rawStream.outputs ?? []).map((out) => ({
        id: out?.id ?? `out-${Date.now()}`,
        type: (out?.type === 'steer' || out?.type === 'system' || out?.type === 'error' || out?.type === 'user'
          ? out.type
          : 'output') as DrawerActivityItem['type'],
        timestamp: out?.timestamp ?? '',
        content: out?.content ?? '',
      })),
      ...(rawTools ?? []).map((t) => ({
        id: t?.id ?? `tool-${Date.now()}`,
        type: 'tool' as const,
        timestamp: t?.timestamp ?? '',
        tool: t,
      })),
    ];

    return merged;
  }, [task, activeSubagentId, activeFeed, activeStream, activeTaskTools]);

  const turns = useMemo(() => {
    if (!task) return 0;
    const feed = activeFeed || [];
    const feedCount = (feed ?? []).filter((a) => a?.type === 'tool' || a?.type === 'thought' || a?.type === 'output').length;
    const toolsCount = activeTaskTools?.length || 0;
    const outputsCount = activeStream?.outputs?.filter((o) => o?.type === 'output').length || 0;
    return Math.max(feedCount, toolsCount + outputsCount, 0);
  }, [task, activeFeed, activeTaskTools, activeStream]);

  return {
    rawActiveFeed,
    activeFeed,
    activeStream,
    activeTaskTools,
    activeSubagentTools,
    isAgentActive,
    unifiedActivities,
    turns,
  };
}
