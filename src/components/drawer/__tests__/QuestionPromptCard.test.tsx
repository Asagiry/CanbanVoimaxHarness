import { describe, it, expect, vi, beforeEach } from 'vitest';
import React from 'react';
import { renderToString } from 'react-dom/server';
import QuestionPromptCard from '../QuestionPromptCard';
import { QuestionPromptCardInner } from '../QuestionPromptCard';
import type { DrawerActivityItem } from '../../../stores/drawerStore';
import { useDrawerStore } from '../../../stores/drawerStore';

// Mock Tauri IPC
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue([]),
}));

// Dispatcher simulator for simulating React hook execution in node environment
class ReactDispatcherSimulator {
  render<T>(componentFn: () => T, stateOverrides?: Map<number, any>): T {
    const internals = (React as any).__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE;
    const prevDispatcher = internals.H;
    let stateCallIndex = 0;

    const mockDispatcher = {
      useSyncExternalStore: (_subscribe: any, getSnapshot: any) => getSnapshot(),
      useState: (initial: any) => {
        const idx = stateCallIndex++;
        const val = stateOverrides && stateOverrides.has(idx) ? stateOverrides.get(idx) : (typeof initial === 'function' ? initial() : initial);
        return [val, () => {}];
      },
      useMemo: (factory: any) => factory(),
      useEffect: () => {},
      useCallback: (fn: any) => fn,
      useRef: (initial: any) => ({ current: initial }),
      useContext: (ctx: any) => ctx?._currentValue,
      useDebugValue: () => {},
      useId: () => 'test-id',
      useLayoutEffect: () => {},
      useInsertionEffect: () => {},
      useTransition: () => [false, (fn: any) => fn()],
      useActionState: (action: any, initialState: any) => [initialState, action, false],
      useOptimistic: (passthrough: any) => [passthrough, () => {}],
    };

    internals.H = mockDispatcher;
    try {
      return componentFn();
    } finally {
      internals.H = prevDispatcher;
    }
  }
}

function findElements(node: any, predicate: (n: any) => boolean): any[] {
  const matches: any[] = [];
  function walk(curr: any) {
    if (!curr) return;
    if (Array.isArray(curr)) {
      curr.forEach(walk);
      return;
    }
    if (typeof curr !== 'object') return;
    if (predicate(curr)) matches.push(curr);
    if (curr.props && curr.props.children) {
      if (Array.isArray(curr.props.children)) {
        curr.props.children.forEach(walk);
      } else {
        walk(curr.props.children);
      }
    }
  }
  walk(node);
  return matches;
}

describe('QuestionPromptCard Markdown View & Interactivity', () => {
  const mockRecordQuestionAnswer = vi.fn().mockResolvedValue(undefined);

  beforeEach(() => {
    vi.clearAllMocks();
    useDrawerStore.setState({
      recordQuestionAnswer: mockRecordQuestionAnswer,
    });
  });

  const createActivity = (overrides?: Partial<DrawerActivityItem>): DrawerActivityItem => ({
    id: 'q-act-1',
    type: 'tool',
    timestamp: '12:00:00',
    content: '',
    tool: {
      id: 'q-tool-1',
      toolName: 'question',
      timestamp: '12:00:00',
      durationMs: 350,
      status: 'running',
      params: {
        questionId: 'q-100',
        text: '# Архитектурный разбор\n\n---\n\n### 1. Источник правды\n\nЯвляется ли `boards.json` **главным** конфигом?\n\n* Пункт первый\n* Пункт второй',
        options: [
          {
            label: 'Использовать `boards.json` (Рекомендуется)',
            description: 'Основной источник в `scaffold.rs`',
          },
          {
            label: 'Сохранить старый конфиг',
            description: 'Устаревший конфиг',
          },
        ],
        multiple: false,
      },
    },
    ...overrides,
  });

  describe('Markdown Question Text Rendering', () => {
    it('renders structured markdown elements (h2, hr, h4, strong, code, bullets) instead of raw text in h4', () => {
      const activity = createActivity();
      const html = renderToString(<QuestionPromptCard activity={activity} taskId="CVH-032" />);

      // Must render h2 heading from `# Архитектурный разбор`
      expect(html).toContain('<h2');
      expect(html).toContain('Архитектурный разбор');

      // Must render horizontal break `<hr`
      expect(html).toContain('<hr');

      // Must render h4 subheading from `### 1. Источник правды`
      expect(html).toContain('<h4');
      expect(html).toContain('1. Источник правды');

      // Must render bold text from `**главным**`
      expect(html).toContain('<strong');
      expect(html).toContain('главным');

      // Must render inline code from `` `boards.json` ``
      expect(html).toContain('<code');
      expect(html).toContain('boards.json');

      // Must render bullet items
      expect(html).toContain('•</span>');
      expect(html).toContain('Пункт первый');
      expect(html).toContain('Пункт второй');

      // Must NOT render raw markdown syntax like `# Архитектурный` or `---` inside a single h4
      expect(html).not.toMatch(/<h4[^>]*>\s*# Архитектурный/);
    });

    it('handles escaped literal \\n in question text gracefully', () => {
      const activity = createActivity({
        tool: {
          id: 'q-tool-escaped',
          toolName: 'question',
          timestamp: '12:00:00',
          durationMs: 200,
          status: 'running',
          params: {
            questionId: 'q-escaped',
            text: '# Вопрос с экранированием\\n\\nТекст второй строки',
            options: ['Да', 'Нет'],
          },
        },
      });

      const html = renderToString(<QuestionPromptCard activity={activity} taskId="CVH-032" />);
      expect(html).toContain('<h2');
      expect(html).toContain('Вопрос с экранированием');
      expect(html).toContain('Текст второй строки');
    });
  });

  describe('Inline Formatting in Options and Descriptions', () => {
    it('renders inline code in option labels and descriptions', () => {
      const activity = createActivity();
      const html = renderToString(<QuestionPromptCard activity={activity} taskId="CVH-032" />);

      // Option label: `boards.json` formatted in <code>
      expect(html).toContain('<code');
      expect(html).toContain('boards.json</code>');

      // Recommendation tag cleanly separated
      expect(html).toContain('Рекомендуется');

      // Option description: `scaffold.rs` formatted in <code>
      expect(html).toContain('scaffold.rs</code>');
    });
  });

  describe('Answered State', () => {
    it('renders answered badge, disabled buttons, and selected state when question has answer', () => {
      const activity = createActivity({
        tool: {
          id: 'q-tool-ans',
          toolName: 'question',
          timestamp: '12:00:00',
          durationMs: 800,
          status: 'success',
          stdout: 'Использовать `boards.json` (Рекомендуется)',
          params: {
            questionId: 'q-100',
            text: 'Вопрос решен?',
            options: ['Использовать `boards.json` (Рекомендуется)', 'Другое'],
            selectedAnswer: 'Использовать `boards.json` (Рекомендуется)',
          },
        },
      });

      const html = renderToString(<QuestionPromptCard activity={activity} taskId="CVH-032" />);

      // Status pill shows "Отвечено"
      expect(html).toContain('Отвечено');
      expect(html).not.toContain('Ожидает ответа');

      // Buttons are disabled
      expect(html).toContain('disabled=""');

      // Selected label has "Выбрано"
      expect(html).toContain('Выбрано');

      // Footer note contains locked answer notice
      expect(html).toContain('Зафиксированный ответ:');
      expect(html).toContain('Агент возобновил выполнение');
    });

    it('renders custom answered response with inline formatting if answer was not predefined', () => {
      const activity = createActivity({
        tool: {
          id: 'q-tool-cust',
          toolName: 'question',
          timestamp: '12:00:00',
          durationMs: 500,
          status: 'success',
          stdout: 'Специальный выбор `config.json`',
          params: {
            questionId: 'q-100',
            text: 'Какой конфиг выбрать?',
            options: ['Вариант А', 'Вариант Б'],
            selectedAnswer: 'Специальный выбор `config.json`',
          },
        },
      });

      const html = renderToString(<QuestionPromptCard activity={activity} taskId="CVH-032" />);
      expect(html).toContain('Пользовательский ответ:');
      expect(html).toContain('<code');
      expect(html).toContain('config.json</code>');
    });
  });

  describe('Multiple Choice Mode', () => {
    it('renders multiple choice badge and submit button', () => {
      const activity = createActivity({
        tool: {
          id: 'q-tool-mult',
          toolName: 'question',
          timestamp: '12:00:00',
          durationMs: 150,
          status: 'running',
          params: {
            questionId: 'q-mult',
            text: 'Выберите несколько опций',
            multiple: true,
            options: ['Опция 1', 'Опция 2', 'Опция 3'],
          },
        },
      });

      const html = renderToString(<QuestionPromptCard activity={activity} taskId="CVH-032" />);
      expect(html).toContain('множественный выбор');
      expect(html).toContain('Отправить ответ');
      expect(html).toContain('Выбрано вариантов:');
    });
  });

  describe('Option Selection and Answer Dispatch', () => {
    it('dispatches recordQuestionAnswer when single choice option is clicked', async () => {
      const activity = createActivity();
      const simulator = new ReactDispatcherSimulator();
      const tree = simulator.render(() => QuestionPromptCardInner({ activity, taskId: 'CVH-032' }));

      // Find option buttons in rendered tree
      const buttons = findElements(
        tree,
        (node) => {
          if (!node || typeof node !== 'object') return false;
          return node.type === 'button' && typeof node.props?.onClick === 'function';
        }
      );

      // Trigger click on first option if found
      expect(buttons.length).toBeGreaterThanOrEqual(1);
      await buttons[0].props.onClick();

      expect(mockRecordQuestionAnswer).toHaveBeenCalledWith(
        'CVH-032',
        'q-100',
        'Использовать `boards.json` (Рекомендуется)'
      );
    });

    it('submits custom input when form is submitted with non-empty text', async () => {
      const activity = createActivity();
      const simulator = new ReactDispatcherSimulator();
      // Override state call index 2 (customInput)
      const overrides = new Map<number, any>([[2, 'Мой кастомный конфиг']]);
      const tree = simulator.render(
        () => QuestionPromptCardInner({ activity, taskId: 'CVH-032' }),
        overrides
      );

      // Find form element
      const forms = findElements(
        tree,
        (node) => node && node.type === 'form' && typeof node.props?.onSubmit === 'function'
      );

      expect(forms.length).toBe(1);
      const fakeEvent = { preventDefault: vi.fn() };
      await forms[0].props.onSubmit(fakeEvent);

      expect(fakeEvent.preventDefault).toHaveBeenCalled();
      expect(mockRecordQuestionAnswer).toHaveBeenCalledWith(
        'CVH-032',
        'q-100',
        'Мой кастомный конфиг'
      );
    });
  });
});
