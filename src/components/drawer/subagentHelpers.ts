import { Language } from '../../stores/languageStore';
import { useCortexStore } from '../../stores/cortexStore';
import { useBoardStore } from '../../stores/boardStore';
import type { SubagentInfo, ToolExecution, KanbanTask } from '../../types';

export function formatSubagentDisplayTitle(title?: string | null, lang?: Language): string {
  const fallback = lang === 'ru' ? 'Субагент' : 'Subagent';
  if (!title) return fallback;
  if (title.includes('sub-call_')) {
    const subWord = lang === 'ru' ? 'субагент' : 'subagent';
    const cleaned = title.replace(/(?:[A-Za-z0-9_-]+\.)?sub-call_\d+/g, subWord).trim();
    return cleaned || fallback;
  }
  return title;
}

export function getSubagentRoleLabel(role: string, isRu: boolean): string {
  const r = role.trim();
  if (!r || r.toLowerCase() === 'general' || r.toLowerCase() === 'subagent') {
    return isRu ? 'Субагент' : 'Subagent';
  }
  if (r.toLowerCase() === 'subtask') {
    return isRu ? 'Подзадача' : 'Subtask';
  }
  if (r.toLowerCase() === 'cvh-dev') {
    return 'CVH-DEV';
  }
  const matched = useCortexStore.getState().agentPersonas?.find(
    (p) => p.role.toLowerCase() === r.toLowerCase()
  );
  if (matched?.persona?.trim()) {
    return matched.persona.trim();
  }
  return r.charAt(0).toUpperCase() + r.slice(1);
}

export function formatSubagentRoleDisplay(
  role?: string | null,
  subIndex?: number,
  isBoardSubtask?: boolean,
  lang?: Language
): string {
  const isRu = lang === 'ru';
  if (isBoardSubtask) {
    const r = (role || '').trim();
    if (r && r.toLowerCase() !== 'general') {
      return getSubagentRoleLabel(r, isRu);
    }
    return isRu ? 'Подзадача' : 'Subtask';
  }

  const baseName = isRu ? 'Субагент' : 'Subagent';
  if (subIndex != null) {
    return `${baseName} #${subIndex}`;
  }
  return baseName;
}

export function formatDuration(ms?: number, isRunning?: boolean): string {
  if (ms === undefined || ms === null || !Number.isFinite(ms) || ms < 0) {
    return isRunning ? '0s' : '—';
  }
  if (!isRunning && ms < 1000) return `${Math.round(ms)}ms`;
  const totalSecs = Math.floor(ms / 1000);
  if (totalSecs < 60) return `${totalSecs}s`;
  const mins = Math.floor(totalSecs / 60);
  const secs = totalSecs % 60;
  return `${mins}m ${secs}s`;
}

export function formatSteps(count: number, lang: Language = 'en'): string {
  if (lang === 'en') {
    return count === 1 ? '1 step' : `${count} steps`;
  }
  const mod10 = count % 10;
  const mod100 = count % 100;
  if (mod100 >= 11 && mod100 <= 19) return `${count} шагов`;
  if (mod10 === 1) return `${count} шаг`;
  if (mod10 >= 2 && mod10 <= 4) return `${count} шага`;
  return `${count} шагов`;
}

export function getSubagentsForTask(
  taskId: string,
  storeSubagents: Record<string, SubagentInfo[]>,
  toolExecutions: Record<string, ToolExecution[]>,
  toolIdToSubagentId: Record<string, string>,
  boardTasks: KanbanTask[],
  columns?: { id: string; persona?: string | null; title?: string }[]
): SubagentInfo[] {
  const cols = columns || useBoardStore.getState().columns || [];
  const map = new Map<string, SubagentInfo>();

  // 1. Explicit store subagents (from backend list_subagents + live events)
  const list = storeSubagents[taskId] || [];
  for (const sub of list) {
    map.set(sub.id, {
      ...sub,
      createdAt: sub.createdAt || (sub.status === 'running' ? Date.now() : undefined),
    });
  }

  // 2. Derive from parent task's tool executions if not yet tracked
  const tools = toolExecutions[taskId] || [];
  for (const t of tools) {
    if (t.toolName === 'subagent' || t.toolName === 'subagent_bg') {
      const params = (t.params as Record<string, unknown>) || {};
      const subId =
        (params?.subagent_id as string) ||
        toolIdToSubagentId[t.id] ||
        t.id;
      const existing = map.get(subId);
      const isBg =
        t.toolName === 'subagent_bg' ||
        (params?.is_bg === true) ||
        (params?.isBg === true) ||
        existing?.isBg === true;
      const parsedTime = t.timestamp ? new Date(t.timestamp).getTime() : undefined;
      const validParsedTime = parsedTime && !isNaN(parsedTime) && parsedTime > 0 ? parsedTime : undefined;
      const candidateTimes: number[] = [];
      if (existing?.createdAt && existing.createdAt > 0) candidateTimes.push(existing.createdAt);
      if (validParsedTime) candidateTimes.push(validParsedTime);
      const toolCreatedAt =
        candidateTimes.length > 0
          ? Math.min(...candidateTimes)
          : (t.status === 'running' ? Date.now() : undefined);

      if (!existing) {
        map.set(subId, {
          id: subId,
          parentTaskId: taskId,
          toolCallId: t.id,
          title: (params?.title as string) || (params?.prompt as string)?.slice(0, 50) || 'Делегированная задача',
          role: (params?.role as string) || 'general',
          prompt: (params?.prompt as string) || '',
          status: t.status === 'running' ? 'running' : t.status === 'failed' ? 'error' : 'finished',
          createdAt: toolCreatedAt,
          durationMs: isBg ? undefined : t.durationMs,
          stepsCount: 0,
          isBg,
        });
      } else {
        if (isBg) {
          existing.isBg = true;
        }
        if (toolCreatedAt) {
          if (!existing.createdAt || toolCreatedAt < existing.createdAt) {
            existing.createdAt = toolCreatedAt;
          }
        }
        if (!isBg && !existing.durationMs && t.durationMs) {
          existing.durationMs = t.durationMs;
        }
      }
    }
  }

  // 3. Subtasks on the Kanban board
  const childTasks = (boardTasks || []).filter((t) => t.parentId === taskId || (t.id.startsWith(taskId + '.') && !t.parentId));
  for (const child of childTasks) {
    const childCreated = (child as any).createdAt ? Number((child as any).createdAt) : undefined;
    const childStageId = child.stageId;
    const col = (cols || []).find((c) => c.id === childStageId);
    const resolvedRole = child.persona || col?.persona || col?.title || 'subtask';
    map.set(child.id, {
      id: child.id,
      parentTaskId: taskId,
      title: child.title,
      role: resolvedRole,
      prompt: child.description || '',
      status: child.status === 'running' ? 'running' : child.status === 'done' ? 'finished' : 'finished',
      createdAt: childCreated,
      durationMs: child.totalElapsed ? child.totalElapsed * 1000 : undefined,
      stepsCount: (child.todos || []).length,
      isBoardSubtask: true,
    });
  }

  const rawList = Array.from(map.values());

  const result = rawList.filter((sub, idx, all) => {
    const isPhantomCandidate =
      sub.status !== 'running' &&
      (!sub.stepsCount || sub.stepsCount === 0) &&
      (!sub.durationMs || sub.durationMs === 0);

    if (!isPhantomCandidate) return true;

    const subPrompt = (sub.prompt || '').trim();
    const subTitle = (sub.title || '').trim();

    const hasBetterDuplicate = all.some((other, otherIdx) => {
      if (otherIdx === idx) return false;
      const otherPrompt = (other.prompt || '').trim();
      const otherTitle = (other.title || '').trim();
      const samePrompt = subPrompt && otherPrompt && subPrompt === otherPrompt;
      const sameTitle = subTitle && otherTitle && subTitle === otherTitle;
      if (!samePrompt && !sameTitle) return false;

      if (other.status === 'running') return true;

      const otherHasStepsOrDuration =
        (typeof other.stepsCount === 'number' && other.stepsCount > 0) ||
        (typeof other.durationMs === 'number' && other.durationMs > 0);

      if (otherHasStepsOrDuration) return true;

      const otherIsFinished = other.status === 'finished';
      if (otherIsFinished && (other.stepsCount || 0) > (sub.stepsCount || 0)) return true;
      if (otherIsFinished && (other.durationMs || 0) > (sub.durationMs || 0)) return true;

      if (otherIdx < idx && samePrompt && sameTitle) return true;

      return false;
    });

    return !hasBetterDuplicate;
  });

  result.sort((a, b) => {
    const aTime = a.createdAt || 0;
    const bTime = b.createdAt || 0;
    if (aTime && bTime && aTime !== bTime) {
      return aTime - bTime;
    }
    return 0;
  });

  return result;
}

export function getSubagentOrderMap(
  taskId: string,
  storeSubagents: Record<string, SubagentInfo[]>,
  toolExecutions: Record<string, ToolExecution[]>,
  toolIdToSubagentId: Record<string, string>,
  boardTasks: KanbanTask[],
  columns?: { id: string; persona?: string | null; title?: string }[]
): Map<string, number> {
  const map = new Map<string, number>();
  if (!taskId) return map;
  const list = getSubagentsForTask(taskId, storeSubagents, toolExecutions, toolIdToSubagentId, boardTasks, columns);
  list.forEach((sub, idx) => {
    const num = idx + 1;
    map.set(sub.id, num);
    if (sub.toolCallId) {
      map.set(sub.toolCallId, num);
    }
    const mapped = toolIdToSubagentId[sub.id];
    if (mapped) {
      map.set(mapped, num);
    }
  });
  return map;
}
