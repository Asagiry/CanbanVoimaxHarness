import { ToolExecution } from '../../types';
import { DrawerActivityItem, DrawerState } from './types';
import { capActivityFeed, capToolExecutions } from './activityDedup';
import { scheduleActivitySave } from './drawerHelpers';
import { useBoardStore } from '../boardStore';

export type ActivityStateSetter = (
  updater: (state: DrawerState) => Partial<DrawerState> | DrawerState
) => void;

export interface QuestionPayload {
  questionId: string;
  text: string;
  options?: Array<{ label: string; description?: string }>;
  multiple?: boolean;
}

/**
 * Finds an existing question item in the activity feed that matches an incoming `question` or `ask_user` tool execution.
 */
export function findMatchingQuestionIndexForTool(
  existingFeed: DrawerActivityItem[],
  tool: ToolExecution
): number {
  const qParamId = (tool.params as Record<string, unknown> | undefined)?.questionId as
    | string
    | undefined;
  for (let i = existingFeed.length - 1; i >= 0; i--) {
    const f = existingFeed[i];
    if (
      (f.type === 'question' &&
        ((qParamId &&
          ((f.tool?.params as Record<string, unknown> | undefined)?.questionId === qParamId ||
            f.tool?.id === qParamId)) ||
          (!qParamId &&
            f.tool?.status === 'running' &&
            !(f.tool?.params as Record<string, unknown> | undefined)?.callId))) ||
      f.id === `q-${tool.id}` ||
      f.tool?.id === tool.id
    ) {
      return i;
    }
  }
  return -1;
}

/**
 * Finds an existing question or running question tool call in the feed to synchronize in-place.
 */
export function findMatchingQuestionTargetIndex(
  existingFeed: DrawerActivityItem[],
  questionId: string,
  itemId = `q-${questionId}`
): number {
  let existingExactIndex = -1;
  for (let i = existingFeed.length - 1; i >= 0; i--) {
    const f = existingFeed[i];
    if (
      f.id === itemId ||
      f.tool?.id === questionId ||
      (f.tool?.params as Record<string, unknown> | undefined)?.questionId === questionId
    ) {
      existingExactIndex = i;
      break;
    }
  }
  if (existingExactIndex !== -1) return existingExactIndex;

  for (let i = existingFeed.length - 1; i >= 0; i--) {
    const f = existingFeed[i];
    if (
      f.type === 'tool' &&
      (f.tool?.toolName === 'question' || f.tool?.toolName === 'ask_user') &&
      f.tool?.status === 'running' &&
      (!(f.tool?.params as Record<string, unknown> | undefined)?.questionId ||
        (f.tool?.params as Record<string, unknown> | undefined)?.questionId === questionId)
    ) {
      return i;
    }
  }
  return -1;
}

/**
 * Checks whether an activity item corresponds to the given questionId or tool callId.
 */
export function isMatchingQuestionItem(
  item: DrawerActivityItem,
  questionId: string
): boolean {
  return Boolean(
    item.id === `q-${questionId}` ||
      item.id === questionId ||
      (item.tool &&
        (item.tool.id === questionId ||
          (item.tool.params as Record<string, unknown> | undefined)?.questionId === questionId ||
          (item.tool.params as Record<string, unknown> | undefined)?.callId === questionId))
  );
}

/**
 * Pure helper that inserts or synchronizes a question card into `activityFeed` and `toolExecutions`.
 */
export function applyPendingQuestionToState(
  existingFeed: DrawerActivityItem[],
  existingTools: ToolExecution[],
  taskId: string,
  question: QuestionPayload,
  timeStr = new Date().toLocaleTimeString('en-US', { hour12: false })
): { updatedFeed: DrawerActivityItem[]; updatedTools?: ToolExecution[] } {
  const questionId = question.questionId;
  const itemId = `q-${questionId}`;
  const targetIndex = findMatchingQuestionTargetIndex(existingFeed, questionId, itemId);

  if (targetIndex !== -1) {
    const prev = existingFeed[targetIndex];
    const updatedItem: DrawerActivityItem = {
      ...prev,
      id: itemId,
      type: 'question',
      timestamp: prev.timestamp || timeStr,
      content: question.text || prev.content,
      tool: {
        ...prev.tool,
        id: questionId,
        toolName: 'question',
        timestamp: prev.tool?.timestamp || prev.timestamp || timeStr,
        durationMs: prev.tool?.durationMs || 0,
        status: prev.tool?.status || 'running',
        params: {
          ...(prev.tool?.params as Record<string, unknown>),
          taskId,
          questionId,
          callId: prev.tool?.id,
          text: question.text,
          options: question.options,
          multiple: question.multiple,
        },
      },
    };

    const updatedFeed = [...existingFeed];
    updatedFeed[targetIndex] = updatedItem;

    const updatedTools = existingTools.map((t) => {
      if (t.id === prev.tool?.id || t.id === questionId) {
        return {
          ...t,
          params: {
            ...(t.params as Record<string, unknown>),
            taskId,
            questionId,
            callId: t.id,
            text: question.text,
            options: question.options,
            multiple: question.multiple,
          },
        };
      }
      return t;
    });

    return {
      updatedFeed: capActivityFeed(updatedFeed),
      updatedTools: capToolExecutions(updatedTools),
    };
  }

  const item: DrawerActivityItem = {
    id: itemId,
    type: 'question',
    timestamp: timeStr,
    content: question.text,
    tool: {
      id: questionId,
      toolName: 'question',
      timestamp: timeStr,
      durationMs: 0,
      status: 'running',
      params: {
        taskId,
        questionId,
        text: question.text,
        options: question.options,
        multiple: question.multiple,
      },
    },
  };

  return {
    updatedFeed: capActivityFeed([...existingFeed, item]),
  };
}

/**
 * Pure helper that marks a matching question item as answered in `activityFeed`.
 */
export function resolveQuestionAnswerInFeed(
  existingFeed: DrawerActivityItem[],
  questionId: string,
  answer: string
): DrawerActivityItem[] {
  return existingFeed.map((item) => {
    if (isMatchingQuestionItem(item, questionId)) {
      const prevParams = (item.tool?.params as Record<string, unknown>) || {};
      return {
        ...item,
        tool: item.tool
          ? {
              ...item.tool,
              status: 'success' as const,
              params: {
                ...prevParams,
                selectedAnswer: answer,
              },
            }
          : undefined,
      };
    }
    return item;
  });
}

/**
 * Store action helper to add or synchronize a pending question card in the drawer state.
 */
export function setPendingQuestion(
  set: ActivityStateSetter,
  taskId: string,
  question: QuestionPayload
): void {
  const timeStr = new Date().toLocaleTimeString('en-US', { hour12: false });
  set((state) => {
    const existingFeed = state.activityFeed[taskId] || [];
    const existingTools = state.toolExecutions[taskId] || [];
    const { updatedFeed, updatedTools } = applyPendingQuestionToState(
      existingFeed,
      existingTools,
      taskId,
      question,
      timeStr
    );

    return {
      activityFeed: {
        ...state.activityFeed,
        [taskId]: updatedFeed,
      },
      ...(updatedTools
        ? {
            toolExecutions: {
              ...state.toolExecutions,
              [taskId]: updatedTools,
            },
          }
        : {}),
    };
  });
}

/**
 * Clears the active question badge on the board store for the given task.
 */
export function clearPendingQuestion(taskId: string): void {
  useBoardStore.getState().clearActiveQuestion(taskId);
}

/**
 * Submits the user's answer to the backend and resolves the question card in `activityFeed`.
 */
export async function answerQuestion(
  set: ActivityStateSetter,
  taskId: string,
  questionId: string,
  answer: string
): Promise<boolean> {
  clearPendingQuestion(taskId);
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('answer_agent_question', { taskId, questionId, answer });
  } catch (err) {
    console.error('Failed to invoke answer_agent_question:', err);
    return false;
  }

  set((state) => {
    const existingFeed = state.activityFeed[taskId] || [];
    const updatedFeed = resolveQuestionAnswerInFeed(existingFeed, questionId, answer);

    return {
      activityFeed: {
        ...state.activityFeed,
        [taskId]: updatedFeed,
      },
    };
  });
  scheduleActivitySave(taskId);
  return true;
}
