import { describe, it, expect, beforeEach } from 'vitest';
import { renderToString } from 'react-dom/server';
import TaskTodoWidget from '../TaskTodoWidget';
import TodoChecklist from '../tools/TodoChecklist';
import TodoToolCard from '../tools/TodoToolCard';
import { KanbanTask, ToolExecution } from '../../../types';
import { setLanguage } from '../../../stores/languageStore';
import { useDrawerStore } from '../../../stores/drawerStore';
import { useBoardStore } from '../../../stores/boardStore';

describe('TaskTodoWidget and TodoChecklist Localization & Styling', () => {
  const sampleTask: KanbanTask = {
    id: 'CVH-018',
    title: 'Align todo styling and localization',
    stageId: 'dev',
    priority: 'medium',
    status: 'running',
    persona: 'developer',
    activeStageDuration: 0,
    stageDurations: {},
    totalElapsed: 0,
    isTimerRunning: true,
    returnsCount: 0,
    todos: [
      { id: '1', content: 'Step 1: Planning and diagnosis', status: 'completed' },
      { id: '2', content: 'Step 2: Component implementation', status: 'in_progress' },
      { id: '3', content: 'Step 3: Verification and testing', status: 'pending' },
    ],
  };

  beforeEach(() => {
    setLanguage('en');
    useDrawerStore.setState({
      activeSubagentId: undefined,
      activeSubagentTitle: undefined,
      activityFeed: {},
    });
    useBoardStore.setState({
      tasks: [sampleTask],
    });
  });

  it('renders trigger button with "Tasks" in EN and "Задачи" in RU', () => {
    setLanguage('en');
    const htmlEn = renderToString(<TaskTodoWidget task={sampleTask} />);
    expect(htmlEn).toContain('Tasks');
    expect(htmlEn).toContain('Agent task execution plan');

    setLanguage('ru');
    const htmlRu = renderToString(<TaskTodoWidget task={sampleTask} />);
    expect(htmlRu).toContain('Задачи');
    expect(htmlRu).toContain('План выполнения задач агента');
  });

  it('renders popover subheader with "done", "in progress", "queued" in EN', () => {
    setLanguage('en');
    const html = renderToString(<TaskTodoWidget task={sampleTask} defaultOpen={true} />);
    expect(html).toContain('Execution Plan');
    expect(html).toContain('done');
    expect(html).toContain('in progress');
    expect(html).toContain('queued');
    expect(html).toContain('1'); // completed count
    expect(html).toContain('1'); // in progress count
  });

  it('renders popover subheader with "готово", "в работе", "в очереди" in RU', () => {
    setLanguage('ru');
    const html = renderToString(<TaskTodoWidget task={sampleTask} defaultOpen={true} />);
    expect(html).toContain('План выполнения');
    expect(html).toContain('готово');
    expect(html).toContain('в работе');
    expect(html).toContain('в очереди');
  });

  it('renders active step with spinning loader icon and amber highlight', () => {
    setLanguage('en');
    const html = renderToString(<TaskTodoWidget task={sampleTask} defaultOpen={true} />);
    expect(html).toContain('animate-spin');
    expect(html).toContain('text-amber-600');
    expect(html).toContain('border-amber-500/20');
    expect(html).toContain('Step 2: Component implementation');
  });

  it('renders completed step with checkmark pop icon and emerald styling', () => {
    setLanguage('en');
    const html = renderToString(<TaskTodoWidget task={sampleTask} defaultOpen={true} />);
    expect(html).toContain('animate-checkmark-pop');
    expect(html).toContain('text-emerald-600');
    expect(html).toContain('line-through');
    expect(html).toContain('Step 1: Planning and diagnosis');
  });

  it('renders dual-color progress bar track with correct widths', () => {
    const html = renderToString(
      <TodoChecklist
        todos={[
          { id: '1', content: 'Step 1', status: 'completed' },
          { id: '2', content: 'Step 2', status: 'in_progress' },
          { id: '3', content: 'Step 3', status: 'pending' },
          { id: '4', content: 'Step 4', status: 'pending' },
        ]}
      />
    );
    // completed: 1/4 = 25%, in-progress: 1/4 = 25%
    expect(html).toContain('width:25%');
    expect(html).toContain('bg-emerald-500');
    expect(html).toContain('bg-amber-500');
  });

  it('renders localized empty state message when no todos exist', () => {
    const emptyTask: KanbanTask = {
      ...sampleTask,
      todos: [],
    };

    setLanguage('en');
    const htmlEn = renderToString(<TaskTodoWidget task={emptyTask} defaultOpen={true} />);
    expect(htmlEn).toContain('Plan is currently empty. Steps will appear here when the agent calls the todo_write tool.');

    setLanguage('ru');
    const htmlRu = renderToString(<TaskTodoWidget task={emptyTask} defaultOpen={true} />);
    expect(htmlRu).toContain('План пока пуст. Шаги появятся здесь, когда агент вызовет инструмент todo_write.');
  });

  it('renders localized strings in TodoToolCard for EN and RU', () => {
    const tool: ToolExecution = {
      id: 'tool-1',
      toolName: 'todo_write',
      timestamp: '12:00:00',
      durationMs: 120,
      status: 'success',
      params: {
        todos: [
          { content: 'First task', status: 'completed' },
          { content: 'Second task', status: 'in_progress' },
        ],
      },
    };

    setLanguage('en');
    const htmlEn = renderToString(<TodoToolCard tool={tool} />);
    expect(htmlEn).toContain('1/2 completed (50%)');
    expect(htmlEn).toContain('1 in progress');

    setLanguage('ru');
    const htmlRu = renderToString(<TodoToolCard tool={tool} />);
    expect(htmlRu).toContain('1/2 выполнено (50%)');
    expect(htmlRu).toContain('1 в работе');
  });
});
