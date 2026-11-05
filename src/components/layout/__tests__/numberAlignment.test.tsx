import { describe, it, expect, beforeEach, vi } from 'vitest';
import { renderToString } from 'react-dom/server';
import Topbar from '../Topbar';
import BoardTabsHeader from '../../board/BoardTabsHeader';
import TaskTodoWidget from '../../drawer/TaskTodoWidget';
import SubagentsSidebar from '../../drawer/SubagentsSidebar';
import * as projectStoreModule from '../../../stores/projectStore';
import * as agentSessionStoreModule from '../../../stores/agentSessionStore';
import { useBoardStore } from '../../../stores/boardStore';
import { useDrawerStore } from '../../../stores/drawerStore';
import { setLanguage } from '../../../stores/languageStore';
import { KanbanTask } from '../../../types';

describe('Number and Counter Baseline Alignment', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    setLanguage('en');
  });

  describe('Topbar active project agent counter alignment', () => {
    it('renders project running agent counter with text-xs, font-mono, leading-none, and tabular-nums', () => {
      const projectPath = '/projects/CanbanVoimaxHarness';
      
      vi.spyOn(projectStoreModule, 'useProjectStore').mockReturnValue({
        activeProject: {
          id: 'proj-1',
          name: 'CanbanVoimaxHarness',
          rootPath: projectPath,
          createdAt: Date.now(),
          updatedAt: Date.now(),
        },
        availableProjects: [
          {
            id: 'proj-1',
            name: 'CanbanVoimaxHarness',
            rootPath: projectPath,
            createdAt: Date.now(),
            updatedAt: Date.now(),
          },
        ],
        hasActiveProject: true,
        isTauriEnvironment: true,
        switchProject: vi.fn(),
        updateActiveProject: vi.fn(),
        loadProjects: vi.fn(),
        selectProjectFolder: vi.fn(),
        openFolder: vi.fn(),
        setActiveProjectByPath: vi.fn(),
        removeProjectFromHistory: vi.fn(),
      });

      vi.spyOn(agentSessionStoreModule, 'useAgentSessionStore').mockImplementation(((selector?: (s: any) => any) => {
        const state = {
          sessions: {
            'sess-1': {
              sessionId: 'sess-1',
              taskId: 'CVH-059',
              status: 'running',
              startedAt: Date.now(),
              projectPath,
            },
            'sess-2': {
              sessionId: 'sess-2',
              taskId: 'CVH-059.1',
              status: 'running',
              startedAt: Date.now(),
              projectPath,
            },
          },
        };
        return selector ? selector(state) : state;
      }) as any);

      const html = renderToString(<Topbar />);

      // The count of running sessions is 2
      expect(html).toContain('2 active agent(s)');
      expect(html).toContain('font-mono text-xs text-emerald-600 dark:text-emerald-400 font-semibold tabular-nums leading-none translate-y-[0.5px]');
      // Ensure the old superscript-like text-[10px] is absent on this badge
      expect(html).not.toContain('text-[10px] text-emerald-600');
    });
  });

  describe('BoardTabsHeader badge and counter alignment', () => {
    it('renders board task pills with inline-flex, leading-none, tabular-nums, and py-0.5 without py-0.2', () => {
      useBoardStore.setState({
        boards: [
          {
            id: 'development',
            name: 'Dev Board',
            stages: [],
          },
        ],
        activeBoardId: 'development',
        tasks: [
          { id: '1', title: 'Task 1', stageId: 'dev', boardId: 'development' } as unknown as KanbanTask,
          { id: '2', title: 'Task 2', stageId: 'dev', boardId: 'development' } as unknown as KanbanTask,
        ],
      });

      const html = renderToString(
        <BoardTabsHeader
          tasksCount={2}
          onOpenAddColumn={() => {}}
          onOpenAddTask={() => {}}
        />
      );

      // Board tab task pill badge
      expect(html).toContain('inline-flex items-center justify-center px-1.5 py-0.5 rounded-full text-[10px] font-mono font-medium leading-none tabular-nums shrink-0');
      // Board tasks count text on the right
      expect(html).toContain('tabular-nums font-mono leading-none translate-y-px');
      // Invalid py-0.2 must not be present
      expect(html).not.toContain('py-0.2');
    });
  });

  describe('TaskTodoWidget numbers alignment', () => {
    it('renders todo counter container with leading-none translate-y-px', () => {
      const sampleTask = {
        id: 'CVH-059',
        title: 'Task Todo Alignment',
        todos: [
          { id: '1', content: 'Step 1', status: 'completed' },
          { id: '2', content: 'Step 2', status: 'pending' },
        ],
      } as unknown as KanbanTask;

      const html = renderToString(<TaskTodoWidget task={sampleTask} />);

      expect(html).toContain('leading-none translate-y-px');
      expect(html).toContain('tabular-nums text-[11px] font-semibold text-[#15803d]');
    });
  });

  describe('SubagentsSidebar badge alignment', () => {
    it('renders subagent pills with inline-flex, leading-none, tabular-nums and py-0.5', () => {
      useDrawerStore.setState({
        activeSubagentId: undefined,
        activeSubagentTitle: undefined,
        activityFeed: {},
      });
      useBoardStore.setState({
        tasks: [],
      });

      const html = renderToString(
        <SubagentsSidebar
          taskId="CVH-059"
          taskTitle="Subagent Test"
          isCollapsed={false}
          onToggleCollapse={() => {}}
        />
      );

      expect(html).toContain('inline-flex items-center justify-center font-mono text-[11px] tabular-nums px-1.5 py-0.5 rounded-full leading-none');
      expect(html).not.toContain('py-0.2');
    });
  });
});
