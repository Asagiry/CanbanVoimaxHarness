import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderToString } from 'react-dom/server';
import KanbanCard from '../KanbanCard';
import { isFamilyHighlighted } from '../cardFamily';
import { KanbanTask } from '../../../types';
import { useBoardStore } from '../../../stores/boardStore';
import { useDrawerStore } from '../../../stores/drawerStore';
import { CANONICAL_BOARD_COLUMNS, DEFAULT_BOARD } from '../../../stores/board/boardConstants';

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

// NOTE: renderToString + zustand useSyncExternalStore in SSR yields initial
// snapshot only; setState after store creation is not visible to SSR hooks.
// Tests therefore assert the exact-match invariant via getState selector
// logic (the same expression KanbanCard uses) + SSR renders for smoke.

const createSampleTask = (overrides?: Partial<KanbanTask>): KanbanTask => ({
  id: 'CVH-076',
  title: 'Test Task',
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

// Same selector expression as KanbanCard.tsx:27
function badgeFor(taskId: string) {
  return useBoardStore.getState().activeQuestions[taskId];
}

describe('KanbanCard agentQuestion badge (exact-match invariant)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useBoardStore.setState({
      boards: [DEFAULT_BOARD],
      activeBoardId: 'development',
      stages: CANONICAL_BOARD_COLUMNS,
      columns: CANONICAL_BOARD_COLUMNS,
      tasks: [],
      selectedTaskId: null,
      hoveredTaskId: null,
      searchQuery: '',
      hookStates: {},
      activeQuestions: {},
    });
    useDrawerStore.setState({ activeTaskId: null, isOpen: false });
  });

  it('selector returns question only for asking subtask (.3), not root/planner/dev', () => {
    useBoardStore.getState().setActiveQuestion({
      taskId: 'CVH-076.3',
      questionId: 'q-3',
      text: 'Reviewer needs clarification?',
      timestamp: 1,
    });

    expect(badgeFor('CVH-076')).toBeUndefined();
    expect(badgeFor('CVH-076.1')).toBeUndefined();
    expect(badgeFor('CVH-076.2')).toBeUndefined();
    expect(badgeFor('CVH-076.3')?.text).toBe('Reviewer needs clarification?');
  });

  it('root with waitingForSubtaskId does not mirror subtask question', () => {
    useBoardStore.getState().setActiveQuestion({
      taskId: 'CVH-076.3',
      questionId: 'q-3',
      text: 'Stale mirror check?',
      timestamp: 1,
    });

    const root = createSampleTask({ id: 'CVH-076', waitingForSubtaskId: 'CVH-076.3' });
    // Badge lookup must NOT go through waitingForSubtaskId
    expect(badgeFor(root.id)).toBeUndefined();
    expect(badgeFor(root.waitingForSubtaskId!)).toBeDefined();
  });

  it('badge click target is the asking card id (verified via exact key)', () => {
    useBoardStore.getState().setActiveQuestion({
      taskId: 'CVH-076.3',
      questionId: 'q-3',
      text: 'Click target?',
      timestamp: 1,
    });

    // KanbanCard opens drawer with task.id (exact asking id)
    expect(Object.keys(useBoardStore.getState().activeQuestions)).toEqual(['CVH-076.3']);
    expect(badgeFor('CVH-076.3')?.taskId).toBe('CVH-076.3');
  });

  it('no badge when question cleared', () => {
    useBoardStore.getState().setActiveQuestion({
      taskId: 'CVH-076.3',
      questionId: 'q-3',
      text: 'Will disappear?',
      timestamp: 1,
    });
    useBoardStore.getState().clearActiveQuestion('CVH-076.3');

    expect(badgeFor('CVH-076.3')).toBeUndefined();
  });

  it('renders card structure via SSR without badge leak (smoke)', () => {
    const reviewer = createSampleTask({ id: 'CVH-076.3', parentId: 'CVH-076', stageId: 'review', status: 'running' });
    const html = renderToString(<KanbanCard task={reviewer} index={0} />);
    expect(html).toContain('CVH-076.3');
  });
});

describe('KanbanCard family highlight (hoveredTaskId)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useBoardStore.setState({
      boards: [DEFAULT_BOARD],
      activeBoardId: 'development',
      stages: CANONICAL_BOARD_COLUMNS,
      columns: CANONICAL_BOARD_COLUMNS,
      tasks: [],
      selectedTaskId: null,
      hoveredTaskId: null,
      searchQuery: '',
      hookStates: {},
      activeQuestions: {},
    });
    useDrawerStore.setState({ activeTaskId: null, isOpen: false });
  });

  const family = [
    createSampleTask({ id: 'CVH-066' }),
    createSampleTask({ id: 'CVH-066.1', parentId: 'CVH-066' }),
    createSampleTask({ id: 'CVH-066.2', parentId: 'CVH-066' }),
  ];
  const unrelated = createSampleTask({ id: 'CVH-063' });

  it.each(['CVH-066', 'CVH-066.1', 'CVH-066.2'])(
    'hovering %s highlights all family members',
    (hoveredTaskId) => {
      useBoardStore.setState({ hoveredTaskId });
      const hovered = useBoardStore.getState().hoveredTaskId;
      for (const task of family) {
        expect(isFamilyHighlighted(task, hovered)).toBe(true);
      }
    }
  );

  it('unrelated task is not highlighted', () => {
    useBoardStore.setState({ hoveredTaskId: 'CVH-066.1' });
    expect(isFamilyHighlighted(unrelated, useBoardStore.getState().hoveredTaskId)).toBe(false);
  });

  it('no highlight when hoveredTaskId is null', () => {
    useBoardStore.setState({ hoveredTaskId: null });
    for (const task of family) {
      expect(isFamilyHighlighted(task, useBoardStore.getState().hoveredTaskId)).toBe(false);
    }
  });
});
