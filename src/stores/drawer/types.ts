import {
  LiveStreamEntry,
  ToolExecution,
  ActivityItem,
  ActivityType,
  SubagentInfo,
  SubagentStatus,
  ReasoningEffort,
} from '../../types';

export type DrawerTab = 'thoughts' | 'output' | 'tools';
export type DrawerActivityItem = ActivityItem;
export type { ActivityType };

export interface TaskTelemetry {
  inputTokens: number;
  outputTokens: number;
  cacheReadTokens: number;
  cacheCreationTokens: number;
  cacheHitRate: number;
  durationMs?: number;
  tps?: number;
  ttftMs?: number;
  systemTokens?: number;
  toolTokens?: number;
}

export interface TaskTelemetrySummary {
  totalInputTokens: number;
  totalCachedTokens: number;
  totalOutputTokens: number;
  totalTokens: number;
  cacheHitRate: number;
  turnCount: number;
}

export interface DrawerMetaState {
  isOpen: boolean;
  activeTaskId: string | null;
  activeTab: DrawerTab;
}

export interface DrawerMetaActions {
  openDrawer: (taskId: string, initialTab?: DrawerTab) => void;
  closeDrawer: () => void;
  goBackDrawer: () => void;
  setActiveTab: (tab: DrawerTab) => void;
  resetProjectContext: () => void;
}

export type DrawerMetaSlice = DrawerMetaState & DrawerMetaActions;

export interface StreamState {
  activeStreamIds: Record<string, string | null>;
  streamRollbackVersions: Record<string, number>;
  connectionWait: Record<string, { streamId: string; delaySecs: number } | null>;
  streamEntries: Record<string, { thoughts: LiveStreamEntry[]; outputs: LiveStreamEntry[] }>;
  taskTelemetry: Record<string, TaskTelemetry>;
  taskTelemetrySummary: Record<string, TaskTelemetrySummary>;
  thinkingStartTime: Record<string, number | null>;
  generationStartTime: Record<string, number | null>;
  generationEndTime: Record<string, number | null>;
  taskStartTime: Record<string, number | null>;
  taskEndTime: Record<string, number | null>;
  streamedTurnTokens: Record<string, number>;
  finalTps: Record<string, number | null>;
  tokenOverrides: Record<string, number | undefined>;
  cumulativeOutputTokens: Record<string, number>;
  cumulativeGenerationSec: Record<string, number>;
}

export interface StreamActions {
  startStream: (taskId: string, streamId: string) => void;
  retryStream: (taskId: string, streamId: string, delaySecs: number) => void;
  appendThought: (taskId: string, content: string, streamId?: string) => void;
  appendOutput: (
    taskId: string,
    content: string,
    type?: 'output' | 'error' | 'system' | 'steer' | 'user',
    images?: string[],
    streamId?: string
  ) => void;
  updateTelemetry: (taskId: string, telemetry: Partial<TaskTelemetry>) => void;
  setTaskTelemetrySummary: (taskId: string, summary: TaskTelemetrySummary) => void;
  setThinkingStartTime: (taskId: string, time: number | null) => void;
  setGenerationStartTime: (taskId: string, time: number | null) => void;
  setGenerationEndTime: (taskId: string, time: number | null) => void;
  setTaskEndTime: (taskId: string, time: number | null) => void;
  recordStreamedChars: (taskId: string, chars: number) => void;
  resetTurnStreamMetrics: (taskId: string) => void;
  setFinalTps: (taskId: string, v: number | null) => void;
  recordTurnMetrics: (taskId: string, tokens: number, seconds: number) => void;
  setTaskTokens: (taskId: string, tokens: number) => void;
}

export type StreamSlice = StreamState & StreamActions;

export interface ActivityState {
  activityFeed: Record<string, DrawerActivityItem[]>;
  dialogItems?: Record<string, DrawerActivityItem[]>;
  toolExecutions: Record<string, ToolExecution[]>;
  activeAgentTaskIds: Record<string, boolean>;
}

export interface ActivityActions {
  setActiveAgentTask: (taskId: string, isActive: boolean) => void;
  markFinalResponse: (taskId: string, response?: string) => void;
  addToolExecution: (taskId: string, tool: ToolExecution) => void;
  updateToolExecution: (taskId: string, toolId: string, partial: Partial<ToolExecution>) => void;
  injectOptimisticCompactTool: (taskId: string, customId?: string) => string;
  compactTaskContext: (taskId: string) => Promise<{ savedTokens: number; evaporatedCount: number }>;
  clearTaskLogs: (taskId: string) => Promise<void> | void;
  clearTaskActivity: (taskId: string) => Promise<void> | void;
  clearTaskDialog?: (taskId: string) => Promise<void> | void;
  deleteDialogItem?: (taskId: string, itemId: string) => Promise<void>;
  evaporateClientToolLogs: (taskId: string) => { evaporatedCount: number; savedTokens: number };
  loadTaskActivity: (taskId: string) => Promise<void>;
  loadTaskDialog: (taskId: string) => Promise<void>;
  saveTaskActivity: (taskId: string) => Promise<void>;
  saveTaskDialog?: (taskId: string) => Promise<void>;
  flushSaveDialog: (taskId: string) => Promise<void>;
  appendActivityItem: (taskId: string, item: DrawerActivityItem) => void;
  addQuestion: (
    taskId: string,
    question: {
      questionId: string;
      text: string;
      options?: Array<{ label: string; description?: string }>;
      multiple?: boolean;
    }
  ) => void;
  recordQuestionAnswer: (taskId: string, questionId: string, answer: string) => Promise<boolean>;
}

export type ActivitySlice = ActivityState & ActivityActions;

export interface SubagentsState {
  activeSubagentId: string | null;
  activeSubagentTitle: string | null;
  activeSubagentRole: string | null;
  activeSubagentPrompt: string | null;
  toolIdToSubagentId: Record<string, string>;
  subagentIdToToolId: Record<string, string>;
  subagents: Record<string, SubagentInfo[]>;
  subagentConfigs: Record<string, { model?: string; providerId?: string; reasoningEffort?: ReasoningEffort }>;
}

export interface SubagentsActions {
  registerSubagentMapping: (toolId: string, subagentId: string) => void;
  openSubagent: (subagentId: string, title: string, role: string, prompt: string) => void;
  closeSubagent: () => void;
  loadSubagentDialog: (parentTaskId: string, subagentId: string) => Promise<void>;
  loadSubagents: (parentTaskId: string) => Promise<void>;
  registerSubagent: (parentTaskId: string, subagent: SubagentInfo) => void;
  updateSubagentStatus: (subagentId: string, status: SubagentStatus, error?: string) => void;
  recordSubagentStep: (subagentId: string, activity?: DrawerActivityItem) => void;
  updateSubagentConfig: (
    subagentId: string,
    config: { model?: string; providerId?: string; reasoningEffort?: ReasoningEffort }
  ) => void;
}

export type SubagentsSlice = SubagentsState & SubagentsActions;

export interface TaskDraft {
  text: string;
  images: Array<{ id: string; name: string; dataUrl: string }>;
}

export interface DraftsState {
  taskDrafts: Record<string, TaskDraft>;
}

export interface DraftsActions {
  setTaskDraft: (draftKey: string, draft: TaskDraft) => void;
  clearTaskDraft: (draftKey: string) => void;
  getTaskDraft: (draftKey: string) => TaskDraft | undefined;
}

export type DraftsSlice = DraftsState & DraftsActions;

export type DrawerState = DrawerMetaSlice &
  StreamSlice &
  ActivitySlice &
  SubagentsSlice &
  DraftsSlice;

export type DrawerSliceSet = (
  partial:
    | DrawerState
    | Partial<DrawerState>
    | ((state: DrawerState) => DrawerState | Partial<DrawerState>),
  replace?: boolean | undefined
) => void;

export type DrawerSliceGet = () => DrawerState;
