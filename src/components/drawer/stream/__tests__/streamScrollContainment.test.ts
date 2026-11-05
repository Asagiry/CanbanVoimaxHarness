import React from 'react';
import { describe, it, expect } from 'vitest';
import * as fs from 'fs';
import * as path from 'path';
import { renderToString } from 'react-dom/server';
import { MarkdownContent } from '../MarkdownContent';
import { TaskObjectiveCard } from '../TaskObjectiveCard';
import StageSummaryCard from '../StageSummaryCard';
import type { KanbanTask } from '../../../../types';
import type { DrawerActivityItem } from '../../../../stores/drawerStore';

describe('Drawer Activity Stream Scroll & Overflow Containment', () => {
  const drawerActivityStreamPath = path.resolve(__dirname, '../../DrawerActivityStream.tsx');
  const drawerActivityStreamSource = fs.readFileSync(drawerActivityStreamPath, 'utf-8');

  describe('DrawerActivityStream viewport containment', () => {
    it('enforces overflow-x-hidden on the main feed scrollable container', () => {
      expect(drawerActivityStreamSource).toContain('overflow-x-hidden');
      expect(drawerActivityStreamSource).toMatch(
        /className="[^"]*overflow-y-auto\s+overflow-x-hidden[^"]*"/
      );
    });

    it('enforces w-full and min-w-0 on the central column wrapper', () => {
      expect(drawerActivityStreamSource).toMatch(
        /className="[^"]*w-full\s+max-w-4xl\s+mx-auto\s+space-y-3\.5\s+min-w-0[^"]*"/
      );
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

    it('renders TaskObjectiveCard with min-w-0 and overflow-hidden', () => {
      const html = renderToString(React.createElement(TaskObjectiveCard, { task: mockTask }));
      expect(html).toContain('min-w-0');
      expect(html).toContain('overflow-hidden');
      expect(html).toContain('break-words');
    });

    it('renders StageSummaryCard (reject variant) with min-w-0 and break-words', () => {
      const rejectActivity: DrawerActivityItem = {
        id: 'act-1',
        type: 'reject' as any,
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
        type: 'summary' as any,
        content: 'Task completed successfully',
        timestamp: '12:00:00',
      };
      const html = renderToString(React.createElement(StageSummaryCard, { activity: summaryActivity }));
      expect(html).toContain('min-w-0');
      expect(html).toContain('break-words');
    });
  });
});
