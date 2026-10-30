import { describe, it, expect, beforeEach, beforeAll, vi } from 'vitest';
import { useBoardStore } from '../boardStore';
import { useDrawerStore } from '../../stores/drawerStore';
import { KanbanTask, BackendTaskDTO, PipelineColumn } from '../../types';
import { CANONICAL_BOARD_COLUMNS, DEFAULT_BOARD } from '../board/boardConstants';

const mockInvoke = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

describe('tasksCrudSlice (Clean Deterministic Store Tests)', () => {
  beforeAll(() => {
    (globalThis as unknown as { alert: (msg: string) => void }).alert = vi.fn();
  });

  beforeEach(() => {
    vi.clearAllMocks();
    useBoardStore.setState({
      boards: [DEFAULT_BOARD],
      activeBoardId: 'development',
      columns: CANONICAL_BOARD_COLUMNS,
      tasks: [],
      selectedTaskId: null,
      hoveredTaskId: null,
      searchQuery: '',
      hookStates: {},
      activeQuestions: {},
    });
    useDrawerStore.setState({
      activeTaskId: null,
      isOpen: false,
    });
  });

  const createSampleTask = (overrides?: Partial<KanbanTask>): KanbanTask => ({
    id: 'CVH-001',
    title: 'Implement unit tests',
    description: 'Unit testing tasks slice',
    priority: 'medium',
    columnId: 'backlog',
    boardId: 'development',
    persona: 'cvh-dev',
    status: 'idle',
    activeStageDuration: 0,
    stageDurations: {},
    totalElapsed: 0,
    isTimerRunning: false,
    returnsCount: 0,
    createdAt: 1000,
    updatedAt: 1000,
    ...overrides,
  });

  describe('Task Ingestion & Hydration', () => {
    it('setTasks replaces task list deterministically', () => {
      const taskA = createSampleTask({ id: 'CVH-001', title: 'Task A' });
      const taskB = createSampleTask({ id: 'CVH-002', title: 'Task B' });

      useBoardStore.getState().setTasks([taskA, taskB]);

      const state = useBoardStore.getState();
      expect(state.tasks).toHaveLength(2);
      expect(state.tasks[0].id).toBe('CVH-001');
      expect(state.tasks[1].id).toBe('CVH-002');
    });

    it('upsertBackendTask inserts a new task when not existing in store', () => {
      const dto: BackendTaskDTO = {
        id: 'CVH-005',
        title: 'New backend task',
        stageId: 'dev',
        boardId: 'development',
        status: 'running',
        priority: 'high',
        createdAt: 2000,
      };

      useBoardStore.getState().upsertBackendTask(dto);

      const tasks = useBoardStore.getState().tasks;
      expect(tasks).toHaveLength(1);
      expect(tasks[0].id).toBe('CVH-005');
      expect(tasks[0].title).toBe('New backend task');
      expect(tasks[0].columnId).toBe('dev');
      expect(tasks[0].status).toBe('running');
      expect(tasks[0].priority).toBe('high');
    });

    it('upsertBackendTask updates existing task while preserving client metadata (todos, durations)', () => {
      const initialTask = createSampleTask({
        id: 'CVH-001',
        title: 'Initial Title',
        todos: [{ id: 'td-1', content: 'Sub-action 1', status: 'completed' }],
        stageEnteredAt: 1500,
        stageDurations: { backlog: 120 },
      });
      useBoardStore.getState().setTasks([initialTask]);

      const dtoUpdate: BackendTaskDTO = {
        id: 'CVH-001',
        title: 'Updated Title from Server',
        stageId: 'backlog',
        stageEnteredAt: 1500,
      };

      useBoardStore.getState().upsertBackendTask(dtoUpdate);

      const updated = useBoardStore.getState().tasks.find((t) => t.id === 'CVH-001');
      expect(updated?.title).toBe('Updated Title from Server');
      expect(updated?.todos).toEqual([{ id: 'td-1', content: 'Sub-action 1', status: 'completed' }]);
      expect(updated?.stageEnteredAt).toBe(1500);
      expect(updated?.stageDurations).toEqual({ backlog: 120 });
    });

    it('addTask calls backend create_task and prepends task to store', async () => {
      mockInvoke.mockImplementation(async (cmd, args) => {
        if (cmd === 'create_task') {
          const payload = args as { title: string; stageId: string; boardId: string };
          return {
            id: 'CVH-099',
            title: payload.title,
            stageId: payload.stageId,
            boardId: payload.boardId,
            status: 'idle',
            createdAt: Date.now(),
          } satisfies BackendTaskDTO;
        }
        return null;
      });

      const result = await useBoardStore.getState().addTask({
        title: 'New User Feature',
        description: 'Created through UI',
      });

      expect(result).not.toBeNull();
      expect(result?.id).toBe('CVH-099');
      expect(result?.title).toBe('New User Feature');
      expect(useBoardStore.getState().tasks[0].id).toBe('CVH-099');
    });

    it('addTask returns null and does nothing if title is empty or whitespace', async () => {
      const result = await useBoardStore.getState().addTask({
        title: '   ',
      });

      expect(result).toBeNull();
      expect(mockInvoke).not.toHaveBeenCalled();
    });

    it('setBackendTasks hydrates tasks and cleans up orphaned subtask wait states', () => {
      const parentTask = createSampleTask({
        id: 'CVH-010',
        columnId: 'dev',
        status: 'waiting_subtask',
        waitingForSubtaskId: 'CVH-010.1',
      });
      useBoardStore.getState().setTasks([parentTask]);

      const rawBackendTasks: BackendTaskDTO[] = [
        {
          id: 'CVH-010',
          title: 'Parent Task',
          stageId: 'dev',
          status: 'waiting_subtask',
          waitingForSubtaskId: 'CVH-010.1',
        },
        {
          id: 'CVH-010.1',
          title: 'Subtask 1',
          parentId: 'CVH-010',
          stageId: 'ready',
          status: 'done',
        },
      ];

      useBoardStore.getState().setBackendTasks(rawBackendTasks);

      const state = useBoardStore.getState();
      const parent = state.tasks.find((t) => t.id === 'CVH-010');
      expect(parent?.waitingForSubtaskId).toBeUndefined();
      expect(parent?.status).toBe('idle');
    });
  });

  describe('Task Updates & Status Safety Guards', () => {
    it('updateTask applies partial updates cleanly to target task', () => {
      const task1 = createSampleTask({ id: 'CVH-001', title: 'Task 1' });
      const task2 = createSampleTask({ id: 'CVH-002', title: 'Task 2' });
      useBoardStore.getState().setTasks([task1, task2]);

      useBoardStore.getState().updateTask('CVH-001', {
        title: 'Renamed Task 1',
        description: 'New Description',
        priority: 'critical',
      });

      const tasks = useBoardStore.getState().tasks;
      const target = tasks.find((t) => t.id === 'CVH-001');
      const other = tasks.find((t) => t.id === 'CVH-002');

      expect(target?.title).toBe('Renamed Task 1');
      expect(target?.description).toBe('New Description');
      expect(target?.priority).toBe('critical');
      expect(other?.title).toBe('Task 2');
    });

    it('updateTaskStatus toggles timer on running and off on idle', () => {
      const task = createSampleTask({ id: 'CVH-001', status: 'idle', isTimerRunning: false });
      useBoardStore.getState().setTasks([task]);

      useBoardStore.getState().updateTaskStatus('CVH-001', 'running');
      let current = useBoardStore.getState().tasks.find((t) => t.id === 'CVH-001');
      expect(current?.status).toBe('running');

      useBoardStore.getState().updateTaskStatus('CVH-001', 'paused');
      current = useBoardStore.getState().tasks.find((t) => t.id === 'CVH-001');
      expect(current?.status).toBe('paused');
      expect(current?.isTimerRunning).toBe(false);
    });

    it('updateTaskStatus preserves waiting_subtask status when idle is attempted (Guard Invariant)', () => {
      const task = createSampleTask({
        id: 'CVH-001',
        status: 'waiting_subtask',
        waitingForSubtaskId: 'CVH-001.1',
      });
      useBoardStore.getState().setTasks([task]);

      useBoardStore.getState().updateTaskStatus('CVH-001', 'idle');

      const current = useBoardStore.getState().tasks.find((t) => t.id === 'CVH-001');
      expect(current?.status).toBe('waiting_subtask');
    });

    it('updateTaskStatus preserves done status when idle is attempted (Guard Invariant)', () => {
      const task = createSampleTask({
        id: 'CVH-001',
        status: 'done',
        columnId: 'ready',
      });
      useBoardStore.getState().setTasks([task]);

      useBoardStore.getState().updateTaskStatus('CVH-001', 'idle');

      const current = useBoardStore.getState().tasks.find((t) => t.id === 'CVH-001');
      expect(current?.status).toBe('done');
    });

    it('setTaskVerdict updates verdict and verdict note', () => {
      const task = createSampleTask({ id: 'CVH-001', verdict: 'NONE' });
      useBoardStore.getState().setTasks([task]);

      useBoardStore.getState().setTaskVerdict('CVH-001', 'APPROVED_WITH_CAVEATS', 'Needs minor polish');

      const current = useBoardStore.getState().tasks.find((t) => t.id === 'CVH-001');
      expect(current?.verdict).toBe('APPROVED_WITH_CAVEATS');
      expect(current?.verdictNote).toBe('Needs minor polish');
    });
  });

  describe('Column Transitions & Terminal States', () => {
    it('moveTask successfully updates stage, records stage duration, and triggers agent for active stages', async () => {
      const task = createSampleTask({
        id: 'CVH-001',
        columnId: 'backlog',
        activeStageDuration: 45,
        stageDurations: {},
      });
      useBoardStore.getState().setTasks([task]);

      mockInvoke.mockImplementation(async (cmd, args) => {
        if (cmd === 'move_task_stage') {
          const payload = args as { taskId: string; targetStageId: string; boardId: string };
          return {
            id: payload.taskId,
            title: task.title,
            stageId: payload.targetStageId,
            boardId: payload.boardId,
            status: 'idle',
          } satisfies BackendTaskDTO;
        }
        return null;
      });

      await useBoardStore.getState().moveTask('CVH-001', 'dev');

      const current = useBoardStore.getState().tasks.find((t) => t.id === 'CVH-001');
      expect(current?.columnId).toBe('dev');
      expect(current?.previousStageId).toBe('backlog');
      expect(current?.stageDurations.backlog).toBe(45);
      expect(mockInvoke).toHaveBeenCalledWith(
        'move_task_stage',
        expect.objectContaining({ taskId: 'CVH-001', targetStageId: 'dev' })
      );
    });

    it('moveTask to terminal column (ready) marks task done and computes totalElapsed', async () => {
      const createdAt = Date.now() - 60000;
      const task = createSampleTask({
        id: 'CVH-001',
        columnId: 'dev',
        status: 'running',
        createdAt,
        waitingForSubtaskId: 'CVH-001.1',
      });
      useBoardStore.getState().setTasks([task]);

      mockInvoke.mockImplementation(async (cmd, args) => {
        if (cmd === 'move_task_stage') {
          const payload = args as { taskId: string; targetStageId: string };
          return {
            id: payload.taskId,
            title: task.title,
            stageId: payload.targetStageId,
            status: 'idle',
            createdAt,
          } satisfies BackendTaskDTO;
        }
        return null;
      });

      await useBoardStore.getState().moveTask('CVH-001', 'ready');

      const current = useBoardStore.getState().tasks.find((t) => t.id === 'CVH-001');
      expect(current?.columnId).toBe('ready');
      expect(current?.status).toBe('done');
      expect(current?.waitingForSubtaskId).toBeUndefined();
      expect(current?.completedAt).toBeDefined();
      expect(current?.totalElapsed).toBeGreaterThanOrEqual(59);
    });

    it('moveTask to ready retains first working stage persona (planner) on multi-stage pipeline', async () => {
      const multiColumns: PipelineColumn[] = [
        { id: 'backlog', title: 'Backlog', color: '#888', icon: 'Inbox', persona: null, isIntake: true, modelOverride: null },
        { id: 'plan', title: 'Planning', color: '#0284c7', icon: 'Compass', persona: 'planner', modelOverride: null },
        { id: 'dev', title: 'Dev', color: '#3b82f6', icon: 'Code', persona: 'developer', modelOverride: null },
        { id: 'ready', title: 'Ready', color: '#10b981', icon: 'Check', persona: null, terminal: true, modelOverride: null },
      ];
      useBoardStore.setState({
        boards: [{ id: 'pipeline', name: 'Pipeline', columns: multiColumns }],
        activeBoardId: 'pipeline',
        columns: multiColumns,
      });

      const task = createSampleTask({
        id: 'CVH-040',
        columnId: 'plan',
        boardId: 'pipeline',
        persona: 'planner',
      });
      useBoardStore.getState().setTasks([task]);

      mockInvoke.mockImplementation(async (cmd, args) => {
        if (cmd === 'move_task_stage') {
          const payload = args as { taskId: string; targetStageId: string };
          return {
            id: payload.taskId,
            title: task.title,
            stageId: payload.targetStageId,
            boardId: 'pipeline',
            status: 'done',
          } satisfies BackendTaskDTO;
        }
        return null;
      });

      await useBoardStore.getState().moveTask('CVH-040', 'ready');

      const current = useBoardStore.getState().tasks.find((t) => t.id === 'CVH-040');
      expect(current?.columnId).toBe('ready');
      expect(current?.persona).toBe('planner');
    });

    it('moveTask backward increments returnsCount (Rework Tracking)', async () => {
      const task = createSampleTask({
        id: 'CVH-001',
        columnId: 'dev',
        returnsCount: 0,
      });
      useBoardStore.getState().setTasks([task]);

      mockInvoke.mockImplementation(async (cmd, args) => {
        if (cmd === 'move_task_stage') {
          const payload = args as { taskId: string; targetStageId: string };
          return {
            id: payload.taskId,
            title: task.title,
            stageId: payload.targetStageId,
          } satisfies BackendTaskDTO;
        }
        return null;
      });

      // dev (index 1) -> backlog (index 0) is a backward move
      await useBoardStore.getState().moveTask('CVH-001', 'backlog');

      const current = useBoardStore.getState().tasks.find((t) => t.id === 'CVH-001');
      expect(current?.returnsCount).toBe(1);
    });

    it('promoteTaskToNextStage advances to next sequential column', () => {
      const task = createSampleTask({ id: 'CVH-001', columnId: 'backlog' });
      useBoardStore.getState().setTasks([task]);

      const moveTaskSpy = vi.spyOn(useBoardStore.getState(), 'moveTask').mockResolvedValue();

      useBoardStore.getState().promoteTaskToNextStage('CVH-001');

      expect(moveTaskSpy).toHaveBeenCalledWith('CVH-001', 'dev');
    });

    it('promoteTaskToNextStage adheres to customFlow when present', () => {
      const task = createSampleTask({
        id: 'CVH-001',
        columnId: 'backlog',
        customFlow: ['backlog', 'ready'],
      });
      useBoardStore.getState().setTasks([task]);

      const moveTaskSpy = vi.spyOn(useBoardStore.getState(), 'moveTask').mockResolvedValue();

      useBoardStore.getState().promoteTaskToNextStage('CVH-001');

      expect(moveTaskSpy).toHaveBeenCalledWith('CVH-001', 'ready');
    });
  });

  describe('Symmetrical Parent-Subtask Lifecycle (§ 2.22)', () => {
    it('upserting a subtask locks parent task into waiting_subtask with waitingForSubtaskId', () => {
      const parent = createSampleTask({ id: 'CVH-010', columnId: 'dev', status: 'idle' });
      useBoardStore.getState().setTasks([parent]);

      const subtaskDto: BackendTaskDTO = {
        id: 'CVH-010.1',
        parentId: 'CVH-010',
        title: 'Audit review subtask',
        stageId: 'dev',
        status: 'running',
      };

      useBoardStore.getState().upsertBackendTask(subtaskDto);

      const state = useBoardStore.getState();
      const parentTask = state.tasks.find((t) => t.id === 'CVH-010');
      const childTask = state.tasks.find((t) => t.id === 'CVH-010.1');

      expect(childTask).toBeDefined();
      expect(parentTask?.status).toBe('waiting_subtask');
      expect(parentTask?.waitingForSubtaskId).toBe('CVH-010.1');
    });

    it('completing subtask unlocks parent back to idle and clears waitingForSubtaskId', () => {
      const parent = createSampleTask({
        id: 'CVH-010',
        columnId: 'dev',
        status: 'waiting_subtask',
        waitingForSubtaskId: 'CVH-010.1',
      });
      const subtask = createSampleTask({
        id: 'CVH-010.1',
        parentId: 'CVH-010',
        columnId: 'dev',
        status: 'running',
      });
      useBoardStore.getState().setTasks([parent, subtask]);

      // Subtask finishes and transitions to ready / done
      const subtaskFinishedDto: BackendTaskDTO = {
        id: 'CVH-010.1',
        parentId: 'CVH-010',
        title: 'Audit review subtask',
        stageId: 'ready',
        status: 'done',
      };

      useBoardStore.getState().upsertBackendTask(subtaskFinishedDto);

      const state = useBoardStore.getState();
      const parentTask = state.tasks.find((t) => t.id === 'CVH-010');
      expect(parentTask?.waitingForSubtaskId).toBeUndefined();
      expect(parentTask?.status).toBe('idle');
    });

    it('createSubtask calls backend IPC and updates local store relations', async () => {
      const parent = createSampleTask({ id: 'CVH-020', columnId: 'backlog', status: 'idle' });
      useBoardStore.getState().setTasks([parent]);

      mockInvoke.mockImplementation(async (cmd, args) => {
        if (cmd === 'create_subtask') {
          const payload = args as { parentId: string; targetStageId: string };
          return {
            id: `${payload.parentId}.1`,
            parentId: payload.parentId,
            title: 'Child subtask',
            stageId: payload.targetStageId,
            status: 'idle',
            createdAt: Date.now(),
          } satisfies BackendTaskDTO;
        }
        return null;
      });

      const created = await useBoardStore.getState().createSubtask('CVH-020', 'dev');

      expect(created?.id).toBe('CVH-020.1');
      const state = useBoardStore.getState();
      const parentTask = state.tasks.find((t) => t.id === 'CVH-020');
      expect(parentTask?.status).toBe('waiting_subtask');
      expect(parentTask?.waitingForSubtaskId).toBe('CVH-020.1');
    });
  });

  describe('Task Deletion & Cascading Teardown (§ 2.22 & § 2.26)', () => {
    it('deleteTask cascades and deletes both parent and all associated subtasks', async () => {
      const parent = createSampleTask({ id: 'CVH-050' });
      const child1 = createSampleTask({ id: 'CVH-050.1', parentId: 'CVH-050' });
      const child2 = createSampleTask({ id: 'CVH-050.2', parentId: 'CVH-050' });
      const independent = createSampleTask({ id: 'CVH-099' });

      useBoardStore.getState().setTasks([parent, child1, child2, independent]);
      useBoardStore.getState().setSelectedTaskId('CVH-050');

      mockInvoke.mockResolvedValue(true);

      await useBoardStore.getState().deleteTask('CVH-050');

      const remaining = useBoardStore.getState().tasks;
      expect(remaining).toHaveLength(1);
      expect(remaining[0].id).toBe('CVH-099');
      expect(useBoardStore.getState().selectedTaskId).toBeNull();
      expect(mockInvoke).toHaveBeenCalledWith('delete_task', { taskId: 'CVH-050' });
    });

    it('deleting a subtask unlocks parent task and clears waitingForSubtaskId', async () => {
      const parent = createSampleTask({
        id: 'CVH-040',
        status: 'waiting_subtask',
        waitingForSubtaskId: 'CVH-040.1',
      });
      const child = createSampleTask({
        id: 'CVH-040.1',
        parentId: 'CVH-040',
      });
      useBoardStore.getState().setTasks([parent, child]);

      mockInvoke.mockResolvedValue(true);

      await useBoardStore.getState().deleteTask('CVH-040.1');

      const state = useBoardStore.getState();
      expect(state.tasks).toHaveLength(1);
      const remainingParent = state.tasks[0];
      expect(remainingParent.id).toBe('CVH-040');
      expect(remainingParent.status).toBe('idle');
      expect(remainingParent.waitingForSubtaskId).toBeUndefined();
    });

    it('deleteTask automatically closes drawer if deleted task was open in drawer', async () => {
      const task = createSampleTask({ id: 'CVH-077' });
      useBoardStore.getState().setTasks([task]);
      useDrawerStore.setState({ activeTaskId: 'CVH-077', isOpen: true });

      mockInvoke.mockResolvedValue(true);

      await useBoardStore.getState().deleteTask('CVH-077');

      expect(useDrawerStore.getState().isOpen).toBe(false);
      expect(useDrawerStore.getState().activeTaskId).toBeNull();
    });
  });

  describe('Interactive Questions, Hook States & Selection', () => {
    it('sets and clears active questions', () => {
      useBoardStore.getState().setActiveQuestion({
        taskId: 'CVH-001',
        questionId: 'q-1',
        text: 'Proceed with migration?',
        timestamp: 12345,
      });

      expect(useBoardStore.getState().activeQuestions['CVH-001']?.text).toBe('Proceed with migration?');

      useBoardStore.getState().clearActiveQuestion('CVH-001');
      expect(useBoardStore.getState().activeQuestions['CVH-001']).toBeUndefined();
    });

    it('tracks hook lifecycle from start to finish', () => {
      const task = createSampleTask({ id: 'CVH-001' });
      useBoardStore.getState().setTasks([task]);

      useBoardStore.getState().setHookStart({
        taskId: 'CVH-001',
        stageId: 'dev',
        hookType: 'on_enter',
        hookName: 'gatekeeper.bat',
        description: 'Lint check',
      });

      let hook = useBoardStore.getState().hookStates['CVH-001']?.on_enter;
      expect(hook?.status).toBe('running');
      expect(hook?.hookName).toBe('gatekeeper.bat');

      useBoardStore.getState().setHookFinish({
        taskId: 'CVH-001',
        stageId: 'dev',
        hookType: 'on_enter',
        success: true,
        exitCode: 0,
        stdout: 'All checks passed',
      });

      hook = useBoardStore.getState().hookStates['CVH-001']?.on_enter;
      expect(hook?.status).toBe('success');
      expect(hook?.exitCode).toBe(0);
      expect(hook?.stdout).toBe('All checks passed');

      useBoardStore.getState().clearHookState('CVH-001', 'on_enter');
      expect(useBoardStore.getState().hookStates['CVH-001']?.on_enter).toBeUndefined();
    });

    it('sets search query and hovered/selected task IDs', () => {
      useBoardStore.getState().setSearchQuery('auth');
      expect(useBoardStore.getState().searchQuery).toBe('auth');

      useBoardStore.getState().setSelectedTaskId('CVH-001');
      expect(useBoardStore.getState().selectedTaskId).toBe('CVH-001');

      useBoardStore.getState().setHoveredTaskId('CVH-002');
      expect(useBoardStore.getState().hoveredTaskId).toBe('CVH-002');
    });
  });
});
