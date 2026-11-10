import React from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderToString } from 'react-dom/server';
import DrawerActivityStream from '../../DrawerActivityStream';
import { MarkdownContent } from '../MarkdownContent';
import { TaskObjectiveCard } from '../TaskObjectiveCard';
import { SubagentActivityCard } from '../SubagentActivityCard';
import StageSummaryCard from '../StageSummaryCard';
import { ExecutionErrorBanner } from '../ExecutionErrorBanner';
import type { KanbanTask } from '../../../../types';
import type { DrawerActivityItem } from '../../../../stores/drawerStore';
import { useDrawerStore } from '../../../../stores/drawerStore';
import { useBoardStore } from '../../../../stores/boardStore';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue([]),
}));

describe('Drawer Activity Stream Scroll & Overflow Containment', () => {
  const mockTask: KanbanTask = {
    id: 'CVH-065',
    title: 'Test Objective Title',
    description: 'Test Objective Description',
    stageId: 'dev',
    boardId: 'development',
    persona: 'developer',
    priority: 'medium',
    status: 'idle',
    activeStageDuration: 0,
    stageDurations: {},
    totalElapsed: 0,
    isTimerRunning: false,
    returnsCount: 0,
    createdAt: 1000,
    updatedAt: 1000,
  };

  beforeEach(() => {
    vi.clearAllMocks();
    useDrawerStore.setState({
      activeTaskId: 'CVH-065',
      activeSubagentId: null,
      activeSubagentTitle: null,
      activeSubagentRole: null,
      activeSubagentPrompt: null,
      subagents: {},
      toolExecutions: {},
      toolIdToSubagentId: {},
      connectionWait: {},
    });
    useBoardStore.setState({
      tasks: [mockTask],
    });
  });

  describe('DrawerActivityStream viewport containment', () => {
    it('renders main feed scrollable container with overflow-x-hidden and overflow-y-auto', () => {
      const html = renderToString(
        React.createElement(DrawerActivityStream, {
          task: mockTask,
          unifiedActivities: [],
          isRunning: false,
          runError: null,
          onRunAgent: vi.fn(),
          onClearRunError: vi.fn(),
        })
      );

      // Verify outer layout boundary
      expect(html).toContain('overflow-hidden');
      expect(html).toContain('min-w-0');

      // Verify the main feed scrollable element
      expect(html).toContain('overflow-y-auto');
      expect(html).toContain('overflow-x-hidden');
    });

    it('renders central column wrapper with w-full, max-w-4xl, and min-w-0', () => {
      const html = renderToString(
        React.createElement(DrawerActivityStream, {
          task: mockTask,
          unifiedActivities: [],
          isRunning: false,
          runError: null,
          onRunAgent: vi.fn(),
          onClearRunError: vi.fn(),
        })
      );

      expect(html).toContain('w-full');
      expect(html).toContain('max-w-4xl');
      expect(html).toContain('min-w-0');
      expect(html).toContain('space-y-3.5');
    });

    it('renders TaskObjectiveCard inside the stream with objective content and containment', () => {
      const html = renderToString(
        React.createElement(DrawerActivityStream, {
          task: mockTask,
          unifiedActivities: [],
          isRunning: false,
          runError: null,
          onRunAgent: vi.fn(),
          onClearRunError: vi.fn(),
        })
      );

      expect(html).toContain('Task Objective');
      expect(html).toContain('Test Objective Description');
      expect(html).toContain('min-w-0');
      expect(html).toContain('overflow-hidden');
    });

    it('renders ExecutionErrorBanner when runError prop is provided', () => {
      const html = renderToString(
        React.createElement(DrawerActivityStream, {
          task: mockTask,
          unifiedActivities: [],
          isRunning: false,
          runError: 'Agent execution failed: connection reset',
          onRunAgent: vi.fn(),
          onClearRunError: vi.fn(),
        })
      );

      expect(html).toContain('Execution error:');
      expect(html).toContain('Agent execution failed: connection reset');
      expect(html).toContain('min-w-0');
      expect(html).toContain('truncate');
    });

    it('renders activity items inside the stream container', () => {
      const activities: DrawerActivityItem[] = [
        {
          id: 'act-thought-1',
          type: 'thought',
          content: 'Analyzing repository structure for layout containment...',
          timestamp: '10:00:00',
        },
        {
          id: 'act-summary-1',
          type: 'summary',
          content: 'Implemented stream scroll containment successfully',
          timestamp: '10:01:00',
        },
      ];

      const html = renderToString(
        React.createElement(DrawerActivityStream, {
          task: mockTask,
          unifiedActivities: activities,
          isRunning: false,
          runError: null,
          onRunAgent: vi.fn(),
          onClearRunError: vi.fn(),
        })
      );

      expect(html).toContain('Analyzing repository structure for layout containment...');
      expect(html).toContain('Implemented stream scroll containment successfully');
    });
  });

  describe('MarkdownContent token containment', () => {
    it('renders root wrapper with break-words, overflow-wrap:anywhere, and min-w-0', () => {
      const html = renderToString(React.createElement(MarkdownContent, { content: 'Simple text' }));
      expect(html).toContain('break-words');
      expect(html).toContain('[overflow-wrap:anywhere]');
      expect(html).toContain('min-w-0');
    });

    it('renders inline code with break-all to prevent horizontal string blowout', () => {
      const html = renderToString(
        React.createElement(MarkdownContent, {
          content: 'Here is `family=Geist:wght@300;400;500;600;700&long_unbroken_query_param=1234567890`.',
        })
      );
      expect(html).toContain('break-all');
      expect(html).toContain('font-mono');
    });

    it('renders markdown tables inside a w-full max-w-full overflow-x-auto container', () => {
      const tableMarkdown = `
| Header 1 | Header 2 |
| --- | --- |
| Cell 1 | Cell 2 |
`;
      const html = renderToString(React.createElement(MarkdownContent, { content: tableMarkdown }));
      expect(html).toContain('w-full');
      expect(html).toContain('max-w-full');
      expect(html).toContain('overflow-x-auto');
    });

    it('renders code blocks inside a w-full max-w-full container', () => {
      const codeMarkdown = '```typescript\nconst x = 1;\n```';
      const html = renderToString(React.createElement(MarkdownContent, { content: codeMarkdown }));
      expect(html).toContain('w-full');
      expect(html).toContain('max-w-full');
    });

    it('renders paragraph tokens with break-words', () => {
      const html = renderToString(React.createElement(MarkdownContent, { content: 'Standard paragraph text' }));
      expect(html).toMatch(/<p[^>]*class="[^"]*break-words[^"]*"/);
    });
  });

  describe('Stream Cards boundary hardening', () => {
    it('renders TaskObjectiveCard with min-w-0, overflow-hidden, and break-words', () => {
      const html = renderToString(React.createElement(TaskObjectiveCard, { task: mockTask }));
      expect(html).toContain('min-w-0');
      expect(html).toContain('overflow-hidden');
      expect(html).toContain('break-words');
    });

    it('renders SubagentActivityCard (objective view) with min-w-0, overflow-hidden, and break-words', () => {
      const html = renderToString(
        React.createElement(SubagentActivityCard, {
          activeSubagentId: 'sub-001',
          activeSubagentTitle: 'Worker Subagent Task',
          activeSubagentRole: 'developer',
          activeSubagentPrompt: 'Implement test containment',
        })
      );
      expect(html).toContain('Worker Subagent Task');
      expect(html).toContain('DEVELOPER');
      expect(html).toContain('Implement test containment');
      expect(html).toContain('min-w-0');
      expect(html).toContain('overflow-hidden');
      expect(html).toContain('break-words');
    });

    it('renders SubagentActivityCard (activity report item) with min-w-0, overflow-hidden, and break-words', () => {
      const subagentActivity: DrawerActivityItem = {
        id: 'act-sub-1',
        type: 'tool',
        content: 'Completed unit test suite refactoring',
        timestamp: '11:00:00',
        tool: {
          id: 'tool-1',
          toolName: 'subagent_report',
          status: 'success',
          durationMs: 4200,
          timestamp: '11:00:00',
          params: {
            role: 'qa',
            title: 'Audit Verification',
          },
        },
      };

      const html = renderToString(
        React.createElement(SubagentActivityCard, {
          activity: subagentActivity,
          durationMs: 4200,
        })
      );
      expect(html).toContain('Audit Verification');
      expect(html).toContain('QA');
      expect(html).toContain('Completed unit test suite refactoring');
      expect(html).toContain('min-w-0');
      expect(html).toContain('overflow-hidden');
      expect(html).toContain('break-words');
    });

    it('renders StageSummaryCard (reject variant) with min-w-0 and break-words', () => {
      const rejectActivity: DrawerActivityItem = {
        id: 'act-1',
        type: 'reject',
        content: 'Verification failed',
        timestamp: '12:00:00',
      };
      const html = renderToString(React.createElement(StageSummaryCard, { activity: rejectActivity }));
      expect(html).toContain('min-w-0');
      expect(html).toContain('break-words');
    });

    it('renders StageSummaryCard (summary variant) with min-w-0 and break-words', () => {
      const summaryActivity: DrawerActivityItem = {
        id: 'act-2',
        type: 'summary',
        content: 'Task completed successfully',
        timestamp: '12:00:00',
      };
      const html = renderToString(React.createElement(StageSummaryCard, { activity: summaryActivity }));
      expect(html).toContain('min-w-0');
      expect(html).toContain('break-words');
    });

    it('renders ExecutionErrorBanner with min-w-0, truncate, and error styling', () => {
      const html = renderToString(
        React.createElement(ExecutionErrorBanner, {
          error: 'Critical failure during execution step',
          onRetry: vi.fn(),
          onDismiss: vi.fn(),
        })
      );
      expect(html).toContain('Critical failure during execution step');
      expect(html).toContain('min-w-0');
      expect(html).toContain('truncate');
      expect(html).toContain('Execution error:');
    });
  });
});
