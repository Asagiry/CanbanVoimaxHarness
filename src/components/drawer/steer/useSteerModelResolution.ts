import React from 'react';
import type { KanbanTask, ReasoningEffort } from '../../../types';
import { useDrawerStore } from '../../../stores/drawerStore';
import { useBoardStore } from '../../../stores/boardStore';
import { useCortexStore } from '../../../stores/cortexStore';
import { useSettingsStore } from '../../../stores/settingsStore';
import { getSubagentOrderMap } from '../subagentHelpers';
import { GroupedModelOption } from './types';

export function useSteerModelResolution(task: KanbanTask, selectableModels: string[]) {
  const activeSubagentId = useDrawerStore((s) => s.activeSubagentId);
  const activeSubagentRole = useDrawerStore((s) => s.activeSubagentRole);
  const storeSubagents = useDrawerStore((s) => s.subagents);
  const subagentConfigs = useDrawerStore((s) => s.subagentConfigs);
  const toolExecutions = useDrawerStore((s) => s.toolExecutions);
  const toolIdToSubagentId = useDrawerStore((s) => s.toolIdToSubagentId);

  const boardTasks = useBoardStore((s) => s.tasks);
  const subagentNumber = React.useMemo(() => {
    if (!activeSubagentId || !task?.id) return null;
    const map = getSubagentOrderMap(task.id, storeSubagents, toolExecutions, toolIdToSubagentId, boardTasks);
    return map.get(activeSubagentId) ?? (toolIdToSubagentId[activeSubagentId] ? map.get(toolIdToSubagentId[activeSubagentId]) : null) ?? 1;
  }, [activeSubagentId, task?.id, storeSubagents, toolExecutions, toolIdToSubagentId, boardTasks]);

  const isBoardSubtask = React.useMemo(() => {
    if (!activeSubagentId) return false;
    return boardTasks.some((t) => t.id === activeSubagentId);
  }, [activeSubagentId, boardTasks]);

  const boards = useBoardStore((s) => s.boards) ?? [];
  const columns = useBoardStore((s) => s.columns) ?? [];
  const taskBoard = boards.find((b) => b.id === task.boardId);
  const boardCols = taskBoard?.columns && taskBoard.columns.length > 0 ? taskBoard.columns : columns;
  const taskColumn = boardCols.find((c) => c.id === task.stageId) || columns.find((c) => c.id === task.stageId);

  const workingCols = boardCols.filter(
    (c) => !c.isIntake && c.id !== 'backlog' && !c.terminal && c.id !== 'ready' && c.id !== 'done'
  );
  const firstStagePersona = workingCols[0]?.persona;
  const defaultWorkingPersona = firstStagePersona || 'general';

  const isTerminalTask = Boolean(taskColumn?.terminal || task.stageId === 'ready' || task.stageId === 'done');
  const effectivePersonaCandidate = (!task.parentId && (isTerminalTask || !taskColumn?.persona))
    ? (firstStagePersona || task.persona)
    : (task.persona || taskColumn?.persona);
  const taskPersona = effectivePersonaCandidate || defaultWorkingPersona;
  const activePersona = activeSubagentRole || taskPersona;
  const effectiveGaugeTaskId = activeSubagentId || task.id;
  const effectiveGaugePersona = activeSubagentId ? (activeSubagentRole || 'general') : activePersona;

  const agentPersonas = useCortexStore((s) => s.agentPersonas);
  const matchedPersona = agentPersonas.find(
    (p) => p.role.toLowerCase() === activePersona.toLowerCase()
  );

  const parentMatchedPersona = agentPersonas.find(
    (p) => p.role.toLowerCase() === taskPersona.toLowerCase()
  );

  const effectiveLimit = matchedPersona?.contextWindow || parentMatchedPersona?.contextWindow || 128000;

  const providers = useSettingsStore((s) => s.providers) ?? [];
  const defaultModel = useSettingsStore((s) => s.settings?.model) || 'default';
  const activeBoardSubtask = isBoardSubtask ? boardTasks.find((t) => t.id === activeSubagentId) : undefined;
  const resolvedSubId = activeSubagentId ? (toolIdToSubagentId[activeSubagentId] || activeSubagentId) : null;
  const subConfig = activeSubagentId
    ? activeBoardSubtask
      ? {
          model: activeBoardSubtask.model,
          providerId: activeBoardSubtask.providerId ?? undefined,
          reasoningEffort: activeBoardSubtask.reasoningEffort as ReasoningEffort | undefined,
        }
      : (subagentConfigs?.[activeSubagentId] || (resolvedSubId ? subagentConfigs?.[resolvedSubId] : undefined))
    : undefined;

  const modelId = activeSubagentId
    ? (subConfig?.model || matchedPersona?.model || task.model || defaultModel)
    : (task.model || matchedPersona?.model || defaultModel);

  const pairProvider = activeSubagentId
    ? ((subConfig?.providerId ? providers.find((p) => p.id === subConfig.providerId) : undefined) ||
        (matchedPersona?.providerId ? providers.find((p) => p.id === matchedPersona.providerId) : undefined) ||
        (task.providerId ? providers.find((p) => p.id === task.providerId) : undefined) ||
        providers.find((p) => p.models.some((m) => m.id === modelId)) ||
        null)
    : ((task.providerId ? providers.find((p) => p.id === task.providerId) : undefined) ||
        (matchedPersona?.providerId ? providers.find((p) => p.id === matchedPersona.providerId) : undefined) ||
        providers.find((p) => p.models.some((m) => m.id === modelId)) ||
        null);

  const modelDisplay =
    pairProvider?.models.find((m) => m.id === modelId)?.displayName ||
    modelId;
  const providerModelLabel = pairProvider
    ? `${pairProvider.name} / ${modelDisplay}`
    : modelDisplay;
  const personaEffort = matchedPersona?.reasoningEffort || null;

  const groupedModels: GroupedModelOption[] = React.useMemo(() => {
    const flat = providers.flatMap((p) => p.models.map((m) => ({
      providerName: p.name,
      providerId: p.id,
      id: m.id,
      displayName: m.displayName || m.id,
      label: `${p.name} / ${m.displayName || m.id}`,
    })));
    if (flat.length === 0 && (selectableModels?.length ?? 0) > 0) {
      return (selectableModels ?? []).map((id) => ({ providerName: '', providerId: '', id, displayName: id, label: id }));
    }
    return flat;
  }, [providers, selectableModels]);

  return {
    activeSubagentId,
    activeSubagentRole,
    subagentNumber,
    isBoardSubtask,
    activePersona,
    effectiveGaugeTaskId,
    effectiveGaugePersona,
    effectiveLimit,
    matchedPersona,
    modelId,
    providerModelLabel,
    personaEffort,
    subConfig,
    groupedModels,
  };
}
