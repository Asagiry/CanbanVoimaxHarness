import { describe, it, expect } from 'vitest';
import { normalizeKanbanTask } from '../taskNormalizer';
import { PipelineColumn, BoardDef, BackendTaskDTO } from '../../../types';

describe('taskNormalizer dynamic persona resolution', () => {
  const multiStageColumns: PipelineColumn[] = [
    {
      id: 'backlog',
      title: 'Backlog',
      color: '#64748b',
      icon: 'Inbox',
      persona: null,
      isIntake: true,
      modelOverride: null,
    },
    {
      id: 'plan',
      title: 'Planning',
      color: '#0284c7',
      icon: 'Compass',
      persona: 'planner',
      modelOverride: null,
    },
    {
      id: 'dev',
      title: 'Development',
      color: '#6366f1',
      icon: 'Code',
      persona: 'developer',
      modelOverride: null,
    },
    {
      id: 'review',
      title: 'Review',
      color: '#a855f7',
      icon: 'Eye',
      persona: 'reviewer',
      modelOverride: null,
    },
    {
      id: 'retro',
      title: 'Retro',
      color: '#ec4899',
      icon: 'RotateCcw',
      persona: 'retro',
      modelOverride: null,
    },
    {
      id: 'ready',
      title: 'Ready',
      color: '#10b981',
      icon: 'CheckCircle2',
      persona: null,
      terminal: true,
      modelOverride: null,
    },
  ];

  const multiStageBoard: BoardDef = {
    id: 'canban-pipeline',
    name: 'CanbanVoimaxHarness',
    columns: multiStageColumns,
  };

  const cvhBoardColumns: PipelineColumn[] = [
    {
      id: 'backlog',
      title: 'Backlog',
      color: '#64748b',
      icon: 'Inbox',
      persona: null,
      isIntake: true,
      modelOverride: null,
    },
    {
      id: 'dev',
      title: 'CVH Development',
      color: '#6366f1',
      icon: 'Bot',
      persona: 'cvh-dev',
      modelOverride: null,
    },
    {
      id: 'ready',
      title: 'Ready',
      color: '#10b981',
      icon: 'CheckCircle2',
      persona: null,
      terminal: true,
      modelOverride: null,
    },
  ];

  const cvhBoard: BoardDef = {
    id: 'development',
    name: 'cvh-board',
    columns: cvhBoardColumns,
  };

  it('resolves root parent task in ready to the first working stage persona (planner) on multi-stage board', () => {
    const rawDto: BackendTaskDTO = {
      id: 'CVH-040',
      title: 'Refactor UI Components',
      stageId: 'ready',
      boardId: 'canban-pipeline',
      status: 'done',
    };

    const task = normalizeKanbanTask(rawDto, {
      boards: [multiStageBoard],
      activeBoardId: 'canban-pipeline',
      columns: multiStageColumns,
    });

    expect(task).not.toBeNull();
    expect(task?.columnId).toBe('ready');
    expect(task?.status).toBe('done');
    // Root task executed stage 0 (plan), persona must be planner NOT developer!
    expect(task?.persona).toBe('planner');
  });

  it('resolves root parent task in ready to cvh-dev on cvh-board', () => {
    const rawDto: BackendTaskDTO = {
      id: 'CVH-001',
      title: 'Core Backend Fix',
      stageId: 'ready',
      boardId: 'development',
      status: 'done',
    };

    const task = normalizeKanbanTask(rawDto, {
      boards: [cvhBoard],
      activeBoardId: 'development',
      columns: cvhBoardColumns,
    });

    expect(task).not.toBeNull();
    expect(task?.columnId).toBe('ready');
    expect(task?.persona).toBe('cvh-dev');
  });

  it('resolves subtasks strictly to their assigned column persona', () => {
    const devSubtaskDto: BackendTaskDTO = {
      id: 'CVH-040.1',
      parentId: 'CVH-040',
      title: 'CVH-040 (Development)',
      stageId: 'dev',
      boardId: 'canban-pipeline',
      status: 'done',
    };

    const reviewSubtaskDto: BackendTaskDTO = {
      id: 'CVH-040.2',
      parentId: 'CVH-040',
      title: 'CVH-040 (Review)',
      stageId: 'review',
      boardId: 'canban-pipeline',
      status: 'done',
    };

    const retroSubtaskDto: BackendTaskDTO = {
      id: 'CVH-040.3',
      parentId: 'CVH-040',
      title: 'CVH-040 (Retro)',
      stageId: 'retro',
      boardId: 'canban-pipeline',
      status: 'done',
    };

    const devTask = normalizeKanbanTask(devSubtaskDto, {
      boards: [multiStageBoard],
      activeBoardId: 'canban-pipeline',
      columns: multiStageColumns,
    });
    const reviewTask = normalizeKanbanTask(reviewSubtaskDto, {
      boards: [multiStageBoard],
      activeBoardId: 'canban-pipeline',
      columns: multiStageColumns,
    });
    const retroTask = normalizeKanbanTask(retroSubtaskDto, {
      boards: [multiStageBoard],
      activeBoardId: 'canban-pipeline',
      columns: multiStageColumns,
    });

    expect(devTask?.persona).toBe('developer');
    expect(reviewTask?.persona).toBe('reviewer');
    expect(retroTask?.persona).toBe('retro');
  });

  it('resolves custom board with arbitrary persona without any hardcoded developer fallback', () => {
    const customColumns: PipelineColumn[] = [
      { id: 'inbox', title: 'Inbox', color: '#888', icon: 'Inbox', persona: null, isIntake: true, modelOverride: null },
      { id: 'architect', title: 'Architecture', color: '#999', icon: 'Cpu', persona: 'system-architect', modelOverride: null },
      { id: 'finished', title: 'Finished', color: '#0f0', icon: 'Check', persona: null, terminal: true, modelOverride: null },
    ];
    const customBoard: BoardDef = {
      id: 'custom-board',
      name: 'Custom',
      columns: customColumns,
    };

    const rawDto: BackendTaskDTO = {
      id: 'CUST-100',
      title: 'Design System',
      stageId: 'finished',
      boardId: 'custom-board',
      status: 'done',
    };

    const task = normalizeKanbanTask(rawDto, {
      boards: [customBoard],
      activeBoardId: 'custom-board',
      columns: customColumns,
    });

    expect(task?.persona).toBe('system-architect');
  });
});
