import { BoardDef, PipelineColumn } from '../../types';

export const CANONICAL_BOARD_COLUMNS: PipelineColumn[] = [
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
    modelOverride: 'gemini-3.8-flash-high',
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

export const DEFAULT_BOARD: BoardDef = {
  id: 'development',
  name: 'cvh-board',
  columns: CANONICAL_BOARD_COLUMNS,
};
