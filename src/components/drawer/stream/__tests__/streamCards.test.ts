import { describe, it, expect } from 'vitest';
import {
  formatDurationCompact,
  ThoughtStreamCard,
  SteerDirectiveCard,
  SubagentActivityCard,
  MarkdownContent,
  LightboxContext,
  renderInlineNodes,
  formatInline,
  AgentExplanationCard,
  TerminalResponseCard,
  TaskObjectiveCard,
  ExecutionErrorBanner,
  getCleanPrompt,
  isFinalResponseItem,
  hasNearbyTodoWrite,
  shouldSuppressExplanation,
  filterDisplayActivities,
  computeActivityDurations,
} from '../index';
import type { DrawerActivityItem } from '../../../../stores/drawerStore';

describe('ThoughtStreamCard & formatDurationCompact', () => {
  it('handles null, undefined, 0, or negative milliseconds gracefully', () => {
    expect(formatDurationCompact(null)).toBeNull();
    expect(formatDurationCompact(undefined)).toBeNull();
    expect(formatDurationCompact(0)).toBeNull();
    expect(formatDurationCompact(-50)).toBeNull();
    expect(formatDurationCompact(NaN)).toBeNull();
    expect(formatDurationCompact(Infinity)).toBeNull();
  });

  it('formats sub-second durations as milliseconds', () => {
    expect(formatDurationCompact(450)).toBe('450ms');
    expect(formatDurationCompact(12.4)).toBe('12ms');
    expect(formatDurationCompact(999)).toBe('999ms');
  });

  it('formats 1-10s durations with 1 decimal place', () => {
    expect(formatDurationCompact(1000)).toBe('1.0s');
    expect(formatDurationCompact(2450)).toBe('2.5s');
    expect(formatDurationCompact(9949)).toBe('9.9s');
  });

  it('formats 10s-59s durations as rounded seconds', () => {
    expect(formatDurationCompact(10000)).toBe('10s');
    expect(formatDurationCompact(45600)).toBe('46s');
    expect(formatDurationCompact(59400)).toBe('59s');
  });

  it('formats multi-minute durations as minutes and seconds', () => {
    expect(formatDurationCompact(60000)).toBe('1m 0s');
    expect(formatDurationCompact(75000)).toBe('1m 15s');
    expect(formatDurationCompact(135000)).toBe('2m 15s');
    expect(formatDurationCompact(3665000)).toBe('61m 5s');
  });
});

describe('Stream Cards and Markdown Exports', () => {
  it('exports all decomposed stream components and contexts correctly', () => {
    expect(ThoughtStreamCard).toBeDefined();
    expect(SteerDirectiveCard).toBeDefined();
    expect(SubagentActivityCard).toBeDefined();
    expect(MarkdownContent).toBeDefined();
    expect(LightboxContext).toBeDefined();
    expect(renderInlineNodes).toBeDefined();
    expect(formatInline).toBeDefined();
    expect(AgentExplanationCard).toBeDefined();
    expect(TerminalResponseCard).toBeDefined();
    expect(TaskObjectiveCard).toBeDefined();
    expect(ExecutionErrorBanner).toBeDefined();
  });
});

describe('formatInline & renderInlineNodes', () => {
  it('formats inline markdown nodes correctly', () => {
    const nodes = formatInline('Hello **bold** and *italic* and `code` and [link](https://example.com)');
    expect(nodes).toBeDefined();
    expect(nodes.length).toBeGreaterThan(0);
  });

  it('handles empty text', () => {
    expect(formatInline('')).toEqual([]);
  });
});

describe('streamHelpers', () => {
  describe('getCleanPrompt', () => {
    it('returns description if present and trims', () => {
      expect(getCleanPrompt('  Do something  ', 'Title')).toBe('Do something');
    });

    it('falls back to title if description is empty', () => {
      expect(getCleanPrompt('', 'Task Title')).toBe('Task Title');
    });

    it('strips [USER STEER] or [USER] suffix blocks', () => {
      const prompt = 'Initial goal\n\n[USER STEER]: Please also check tests';
      expect(getCleanPrompt(prompt, '')).toBe('Initial goal');

      const prompt2 = 'Initial goal\n[USER]: change approach';
      expect(getCleanPrompt(prompt2, '')).toBe('Initial goal');
    });
  });

  describe('isFinalResponseItem', () => {
    it('returns false if activity type is not output', () => {
      const act = { id: '1', type: 'thought', content: 'thinking' } as DrawerActivityItem;
      expect(isFinalResponseItem(act, 0, [act], false)).toBe(false);
    });

    it('returns true if output has ended and not running', () => {
      const act = { id: '1', type: 'output', content: 'Final text' } as DrawerActivityItem;
      expect(isFinalResponseItem(act, 0, [act], false)).toBe(true);
    });

    it('returns false if running and no following steer/user item', () => {
      const act = { id: '1', type: 'output', content: 'Streaming text' } as DrawerActivityItem;
      expect(isFinalResponseItem(act, 0, [act], true)).toBe(false);
    });

    it('returns false if intervening tool exists after output', () => {
      const act1 = { id: '1', type: 'output', content: 'Intermediate step' } as DrawerActivityItem;
      const act2 = { id: '2', type: 'tool', tool: { toolName: 'read_file' } } as any;
      expect(isFinalResponseItem(act1, 0, [act1, act2], false)).toBe(false);
    });

    it('returns true for summary type when not running and no intervening tools', () => {
      const actTool = { id: '1', type: 'tool', tool: { toolName: 'advance_task' } } as any;
      const actSummary = { id: '2', type: 'summary', content: 'Task completed' } as any;
      expect(isFinalResponseItem(actSummary, 1, [actTool, actSummary], false)).toBe(true);
    });

    it('returns true for output when activity.isFinal is true even if isRunning is true', () => {
      const act = { id: '1', type: 'output', content: 'Final response', isFinal: true } as DrawerActivityItem;
      expect(isFinalResponseItem(act, 0, [act], true)).toBe(true);
    });
  });

  describe('hasNearbyTodoWrite', () => {
    it('returns true if todo_write tool is within 3 items', () => {
      const activities: DrawerActivityItem[] = [
        { id: '1', type: 'tool', tool: { toolName: 'todo_write' } } as any,
        { id: '2', type: 'thought', content: 'thinking' } as any,
        { id: '3', type: 'output', content: 'todos planned' } as any,
      ];
      expect(hasNearbyTodoWrite(2, activities)).toBe(true);
    });

    it('returns false if no todo_write nearby', () => {
      const activities: DrawerActivityItem[] = [
        { id: '1', type: 'tool', tool: { toolName: 'read_file' } } as any,
        { id: '2', type: 'output', content: 'file read' } as any,
      ];
      expect(hasNearbyTodoWrite(1, activities)).toBe(false);
    });
  });

  describe('shouldSuppressExplanation', () => {
    it('suppresses empty explanations', () => {
      const act = { id: '1', type: 'output', content: '   ' } as DrawerActivityItem;
      expect(shouldSuppressExplanation(act, 0, [act])).toBe(true);
    });

    it('does not suppress explanation matching advance_task summary', () => {
      const actTool = {
        id: '1',
        type: 'tool',
        tool: { toolName: 'advance_task', params: { summary: 'Task completed successfully' } },
      } as any;
      const actExp = {
        id: '2',
        type: 'output',
        content: 'Task completed successfully',
      } as any;
      expect(shouldSuppressExplanation(actExp, 1, [actTool, actExp])).toBe(false);
    });

    it('suppresses duplicate explanation identical to previous output', () => {
      const act1 = { id: '1', type: 'output', content: 'Step completed' } as DrawerActivityItem;
      const act2 = { id: '2', type: 'output', content: 'Step completed' } as DrawerActivityItem;
      expect(shouldSuppressExplanation(act2, 1, [act1, act2])).toBe(true);
    });
  });

  describe('filterDisplayActivities', () => {
    it('deduplicates question items and tool calls for the same question', () => {
      const qAct = {
        id: 'q-123',
        type: 'question',
        tool: { questionId: 'q-123', toolName: 'question', params: { questionId: 'q-123' } },
      } as any;
      const toolAct = {
        id: 't-123',
        type: 'tool',
        tool: { id: 't-123', toolName: 'question', params: { questionId: 'q-123' } },
      } as any;
      const result = filterDisplayActivities([qAct, toolAct]);
      expect(result).toHaveLength(1);
      expect(result[0].type).toBe('question');
    });
  });

  describe('computeActivityDurations', () => {
    it('computes durations from explicit durationMs or timestamps', () => {
      const activities: DrawerActivityItem[] = [
        { id: '1', type: 'thought', durationMs: 250 } as any,
        {
          id: '2',
          type: 'tool',
          timestamp: '2026-10-30T10:00:00.000Z',
        } as any,
        {
          id: '3',
          type: 'output',
          timestamp: '2026-10-30T10:00:02.000Z',
        } as any,
      ];
      const durations = computeActivityDurations(activities, false);
      expect(durations[0]).toBe(250);
      expect(durations[1]).toBe(2000);
    });
  });
});
