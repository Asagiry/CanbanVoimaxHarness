import { BoardDef, PipelineStage, PipelineColumn } from '../../types';

export const CANONICAL_BOARD_STAGES: PipelineStage[] = [
  {
    id: 'backlog',
    title: 'Backlog',
    color: '#64748b',
    icon: 'Inbox',
    persona: null,
    modelOverride: null,
    wipLimit: 10,
    hooks: {},
    gatekeeperSensor: null,
  },
  {
    id: 'dev',
    title: 'CVH Development',
    color: '#6366f1',
    icon: 'Bot',
    persona: 'cvh-dev',
    modelOverride: null,
    temperature: 0.2,
    wipLimit: 3,
    hooks: {},
    gatekeeperSensor: null,
  },
  {
    id: 'ready',
    title: 'Ready',
    color: '#10b981',
    icon: 'CheckCircle2',
    persona: null,
    terminal: true,
    modelOverride: null,
    hooks: {},
    gatekeeperSensor: null,
  },
];

export const CANONICAL_BOARD_COLUMNS: PipelineColumn[] = CANONICAL_BOARD_STAGES;

export const DEFAULT_BOARD: BoardDef = {
  id: 'development',
  name: 'cvh-board',
  stages: CANONICAL_BOARD_STAGES,
  columns: CANONICAL_BOARD_STAGES,
};
