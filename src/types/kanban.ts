export interface TodoItem {
  id: string;
  content: string;
  status: 'pending' | 'in_progress' | 'completed';
}

export type {
  KanbanTask,
  TaskPriority,
  TaskStatus,
  TaskVerdict,
  FastTrackConfig,
  BoardDef,
  PipelineColumn,
  PipelineConfig,
} from './index';
