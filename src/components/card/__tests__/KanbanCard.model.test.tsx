import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderToString } from 'react-dom/server';
import KanbanCard from '../KanbanCard';
import { resolveCardModel, formatCardModel } from '../cardModel';
import { KanbanTask, PipelineStage } from '../../../types';
import { useBoardStore } from '../../../stores/boardStore';
import { useDrawerStore } from '../../../stores/drawerStore';
import { useCortexStore } from '../../../stores/cortexStore';
import { useSettingsStore } from '../../../stores/settingsStore';
import { CANONICAL_BOARD_COLUMNS, DEFAULT_BOARD } from '../../../stores/board/boardConstants';
import type { Provider } from '../../../stores/settings/providers';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue([]),
}));

vi.mock('@hello-pangea/dnd', () => ({
  Draggable: ({ children }: { children: any }) =>
    children(
      { innerRef: () => {}, draggableProps: { style: {} }, dragHandleProps: {} },
      { isDragging: false }
    ),
}));

const createTask = (overrides?: Partial<KanbanTask>): KanbanTask => ({
  id: 'CVH-088',
  title: 'Model Task',
  description: 'desc',
  priority: 'medium',
  stageId: 'dev',
  boardId: 'development',
  persona: 'cvh-dev',
  status: 'running',
  activeStageDuration: 0,
  stageDurations: {},
  totalElapsed: 0,
  isTimerRunning: false,
  returnsCount: 0,
  createdAt: 1000,
  updatedAt: 1000,
  ...overrides,
});

function setColumns(columns: PipelineStage[]) {
  useBoardStore.setState({
    boards: [DEFAULT_BOARD],
    activeBoardId: 'development',
    stages: columns,
    columns,
    tasks: [],
    selectedTaskId: null,
    hoveredTaskId: null,
    searchQuery: '',
    hookStates: {},
    activeQuestions: {},
  });
}

function setPersonaModel(model: string | null, providerId?: string | null) {
  useCortexStore.setState({
    agentPersonas: [{ role: 'cvh-dev', hasPrompt: true, model, providerId }],
  } as never);
}

function setGlobalModel(model: string, provider = '') {
  useSettingsStore.setState({
    settings: { ...useSettingsStore.getState().settings, model, provider },
    providers: [],
  } as never);
}

function setProviders(providers: Provider[]) {
  useSettingsStore.setState({ providers } as never);
}

const provider = (id: string, name: string, models: Provider['models']): Provider => ({
  id: id as never,
  name,
  isCustom: false,
  apiKey: '',
  baseUrl: '',
  protocol: 'openai' as const,
  models,
});

// Same selector expressions as KanbanCard.tsx (SSR snapshot limitation:
// renderToString only sees initial store snapshot, so store-driven cases
// are verified via getState + resolveCardModel, the exact wiring KanbanCard uses).
function cardModelFor(task: KanbanTask) {
  const columns = useBoardStore.getState().columns;
  const taskColumn = columns.find((c) => c.id === task.stageId);
  const assignedPersona = taskColumn?.persona
    ? useCortexStore.getState().agentPersonas.find((p) => p.role === taskColumn.persona)
    : undefined;
  const globalModel = useSettingsStore.getState().settings.model;
  const globalProvider = useSettingsStore.getState().settings.provider;
  const providers = useSettingsStore.getState().providers;
  return resolveCardModel(task, taskColumn, assignedPersona, globalModel, providers, globalProvider);
}

function cardModelTextFor(task: KanbanTask) {
  const result = cardModelFor(task);
  return result ? formatCardModel(result) : undefined;
}

describe('KanbanCard model display (CVH-096)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useDrawerStore.setState({ activeTaskId: null, isOpen: false });
    setGlobalModel('');
  });

  it('shows persona model', () => {
    setColumns(CANONICAL_BOARD_COLUMNS);
    setPersonaModel('persona-model-1');
    expect(cardModelFor(createTask())).toEqual({ label: 'persona-model-1' });
  });

  it('shows column override when persona model empty', () => {
    const cols = CANONICAL_BOARD_COLUMNS.map((c) =>
      c.id === 'dev' ? { ...c, modelOverride: 'col-model-1' } : c
    );
    setColumns(cols);
    setPersonaModel(null);
    expect(cardModelFor(createTask())).toEqual({ label: 'col-model-1' });
  });

  it('steer pair overrides persona and column and shows provider', () => {
    const cols = CANONICAL_BOARD_COLUMNS.map((c) =>
      c.id === 'dev' ? { ...c, modelOverride: 'col-model-1' } : c
    );
    setColumns(cols);
    setPersonaModel('persona-model-1');
    setProviders([provider('p1', 'P1', [{ id: 'steer-model-1' }])]);
    expect(cardModelFor(createTask({ providerId: 'p1', model: 'steer-model-1' }))).toEqual({
      label: 'steer-model-1',
      providerName: 'P1',
    });
    expect(cardModelTextFor(createTask({ providerId: 'p1', model: 'steer-model-1' }))).toBe('P1 / steer-model-1');
    const html = renderToString(
      <KanbanCard task={createTask({ providerId: 'p1', model: 'steer-model-1' })} index={0} />
    );
    expect(html).toContain('steer-model-1');
  });

  it('persona model resolves provider via catalog', () => {
    setColumns(CANONICAL_BOARD_COLUMNS);
    setPersonaModel('raw-id');
    setProviders([provider('p1', 'P1', [{ id: 'raw-id', displayName: 'Pretty Label' }])]);
    expect(cardModelTextFor(createTask())).toBe('P1 / Pretty Label');
  });

  it('unknown model with no providers shows bare label', () => {
    setColumns(CANONICAL_BOARD_COLUMNS);
    setPersonaModel('unknown-model');
    setProviders([]);
    expect(cardModelTextFor(createTask())).toBe('unknown-model');
  });

  it('settings fallback uses settings.provider when catalog misses', () => {
    setColumns(CANONICAL_BOARD_COLUMNS);
    setPersonaModel(null);
    setGlobalModel('global-model-1', 'Fallback Provider');
    setProviders([provider('p1', 'P1', [{ id: 'other' }])]);
    expect(cardModelTextFor(createTask())).toBe('Fallback Provider / global-model-1');
  });

  it('shows global settings model as fallback', () => {
    setColumns(CANONICAL_BOARD_COLUMNS);
    setPersonaModel(null);
    setGlobalModel('global-model-1');
    expect(cardModelFor(createTask())).toEqual({ label: 'global-model-1' });
  });

  it('returns undefined when nothing set', () => {
    setColumns(CANONICAL_BOARD_COLUMNS);
    setPersonaModel(null);
    setGlobalModel('');
    expect(cardModelFor(createTask())).toBeUndefined();
  });

  it('ignores stale task.model without providerId', () => {
    setColumns(CANONICAL_BOARD_COLUMNS);
    setPersonaModel('persona-model-1');
    expect(cardModelFor(createTask({ model: 'stale-model' }))).toEqual({ label: 'persona-model-1' });
  });

  it('changing agentPersonas[].model in store updates resolved card model', () => {
    setColumns(CANONICAL_BOARD_COLUMNS);
    setPersonaModel('model-a');
    expect(cardModelFor(createTask())).toEqual({ label: 'model-a' });
    setPersonaModel('model-b');
    expect(cardModelFor(createTask())).toEqual({ label: 'model-b' });
  });
});
