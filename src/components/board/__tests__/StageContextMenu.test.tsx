import { describe, it, expect, vi, beforeEach } from 'vitest';
import React from 'react';
import { renderToString } from 'react-dom/server';
import StageContextMenu, { StageContextMenuProps } from '../StageContextMenu';
import type { PipelineStage } from '../../../types';
import { useBoardStore } from '../../../stores/boardStore';
import { setLanguage } from '../../../stores/languageStore';

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockImplementation((cmd: string) => {
    if (cmd === 'list_agent_personas') {
      return Promise.resolve([
        { role: 'planner', isBuiltin: true },
        { role: 'developer', isBuiltin: true },
        { role: 'reviewer', isBuiltin: true },
      ]);
    }
    if (cmd === 'get_pipeline') {
      return Promise.resolve({
        id: 'default-engineering',
        title: 'Инженерный пайплайн',
        stages: [],
        columns: [],
      });
    }
    return Promise.resolve(null);
  }),
}));

// Mock PersonaSelect to simplify testing
vi.mock('../../common/PersonaSelect', () => ({
  default: () => React.createElement('div', { 'data-testid': 'persona-select' }),
}));

// Custom Dispatcher Simulator to test React hooks in node vitest environment
class ReactDispatcherSimulator {
  render<T>(componentFn: () => T, stateOverrides?: Map<number, any>, stateSetters?: Map<number, (v: any) => void>): T {
    const internals = (React as any).__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE;
    const prevDispatcher = internals.H;
    let stateCallIndex = 0;

    const mockDispatcher = {
      useSyncExternalStore: (_subscribe: any, getSnapshot: any) => getSnapshot(),
      useState: (initial: any) => {
        const idx = stateCallIndex++;
        const val = stateOverrides && stateOverrides.has(idx)
          ? stateOverrides.get(idx)
          : (typeof initial === 'function' ? initial() : initial);
        const setter = (nextVal: any) => {
          if (stateSetters && stateSetters.has(idx)) {
            stateSetters.get(idx)!(nextVal);
          }
        };
        return [val, setter];
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

describe('StageContextMenu: Agent Assignment and On Enter / On Leave Hooks', () => {
  const simulator = new ReactDispatcherSimulator();

  const mockStages: PipelineStage[] = [
    {
      id: 'backlog',
      title: 'Бэклог',
      color: '#64748b',
      icon: 'Inbox',
      persona: null,
      isIntake: true,
      modelOverride: null,
      hooks: {
        on_leave: {
          script: '.cvh/hooks/create_worktree.bat',
          description: 'Create worktree on stage leave',
        },
      },
    },
    {
      id: 'dev',
      title: 'Разработка',
      color: '#6366f1',
      icon: 'Code',
      persona: 'developer',
      modelOverride: null,
      hooks: {
        on_enter: {
          script: 'scripts/enter_dev.bat',
        },
      },
    },
    {
      id: 'review',
      title: 'Ревью',
      color: '#a855f7',
      icon: 'Eye',
      persona: 'reviewer',
      modelOverride: null,
      hooks: {
        on_leave: {
          script: '.cvh/hooks/on_review_finish.bat',
        },
      },
    },
    {
      id: 'ready',
      title: 'Готово',
      color: '#10b981',
      icon: 'Check',
      persona: null,
      terminal: true,
      modelOverride: null,
    },
  ];

  function renderMenu(
    props: StageContextMenuProps,
    stateOverrides?: Map<number, any>,
    stateSetters?: Map<number, (v: any) => void>
  ) {
    const node = simulator.render(() => StageContextMenu(props), stateOverrides, stateSetters);
    return {
      node,
      html: renderToString(node),
    };
  }

  beforeEach(() => {
    setLanguage('ru');
    useBoardStore.setState({
      stages: mockStages,
      columns: mockStages,
      boards: [
        {
          id: 'default',
          name: 'Main Board',
          stages: mockStages,
          columns: mockStages,
        },
      ],
      activeBoardId: 'default',
    });
  });

  describe('1. canChangeAgent guard', () => {
    it('does NOT render "Сменить агента" for backlog (intake stage)', () => {
      const stage = mockStages.find((s) => s.id === 'backlog')!;
      const { html } = renderMenu({ stage, x: 100, y: 100, onClose: vi.fn() });

      expect(html).not.toContain('Сменить агента');
      expect(html).toContain('Переименовать');
      expect(html).toContain('Скрипт входа');
      expect(html).toContain('Скрипт выхода');
    });

    it('does NOT render "Сменить агента" for ready (terminal stage)', () => {
      const stage = mockStages.find((s) => s.id === 'ready')!;
      const { html } = renderMenu({ stage, x: 100, y: 100, onClose: vi.fn() });

      expect(html).not.toContain('Сменить агента');
      expect(html).toContain('Переименовать');
      expect(html).toContain('Скрипт входа');
      expect(html).toContain('Скрипт выхода');
    });

    it('renders "Сменить агента" for regular intermediate stages (dev, review)', () => {
      const devStage = mockStages.find((s) => s.id === 'dev')!;
      const { html: htmlDev } = renderMenu({ stage: devStage, x: 100, y: 100, onClose: vi.fn() });
      expect(htmlDev).toContain('Сменить агента');

      const reviewStage = mockStages.find((s) => s.id === 'review')!;
      const { html: htmlReview } = renderMenu({ stage: reviewStage, x: 100, y: 100, onClose: vi.fn() });
      expect(htmlReview).toContain('Сменить агента');
    });

    it('does not render "Change Agent" in English for backlog or ready', () => {
      setLanguage('en');
      const stage = mockStages.find((s) => s.id === 'backlog')!;
      const { html } = renderMenu({ stage, x: 100, y: 100, onClose: vi.fn() });
      expect(html).not.toContain('Change Agent');
      expect(html).toContain('Entry Script');
      expect(html).toContain('Exit Script');
    });
  });

  describe('2. Dual Hook Menu Items (On Enter and On Leave)', () => {
    it('renders both "Скрипт входа" and "Скрипт выхода" buttons in main menu', () => {
      const stage = mockStages.find((s) => s.id === 'backlog')!;
      const { html } = renderMenu({ stage, x: 100, y: 100, onClose: vi.fn() });

      expect(html).toContain('Скрипт входа');
      expect(html).toContain('Скрипт выхода');
    });

    it('displays script basename on the button for stages with on_leave hook', () => {
      // backlog has .cvh/hooks/create_worktree.bat -> basename is create_worktree.bat
      const backlogStage = mockStages.find((s) => s.id === 'backlog')!;
      const { html: htmlBacklog } = renderMenu({ stage: backlogStage, x: 100, y: 100, onClose: vi.fn() });
      expect(htmlBacklog).toContain('create_worktree.bat');

      // review has .cvh/hooks/on_review_finish.bat -> basename is on_review_finish.bat
      const reviewStage = mockStages.find((s) => s.id === 'review')!;
      const { html: htmlReview } = renderMenu({ stage: reviewStage, x: 100, y: 100, onClose: vi.fn() });
      expect(htmlReview).toContain('on_review_finish.bat');
    });

    it('displays script basename on the button for stages with on_enter hook', () => {
      // dev has scripts/enter_dev.bat -> basename is enter_dev.bat
      const devStage = mockStages.find((s) => s.id === 'dev')!;
      const { html: htmlDev } = renderMenu({ stage: devStage, x: 100, y: 100, onClose: vi.fn() });
      expect(htmlDev).toContain('enter_dev.bat');
    });
  });

  describe('3. Script Editor View (mode === "script")', () => {
    it('displays on_leave script details when editing exit script', () => {
      const backlogStage = mockStages.find((s) => s.id === 'backlog')!;
      // State indices:
      // idx 1: mode = 'script'
      // idx 5: scriptType = 'on_leave'
      // idx 6: draftScript = '.cvh/hooks/create_worktree.bat'
      const stateOverrides = new Map<number, any>([
        [1, 'script'],
        [5, 'on_leave'],
        [6, '.cvh/hooks/create_worktree.bat'],
      ]);

      const { html } = renderMenu(
        { stage: backlogStage, x: 100, y: 100, onClose: vi.fn() },
        stateOverrides
      );

      expect(html).toContain('Скрипт выхода');
      expect(html).toContain('Текущий: .cvh/hooks/create_worktree.bat');
      expect(html).toContain('.cvh/hooks/create_worktree.bat');
      expect(html).toContain('Запускается при выходе задачи из стадии');
    });

    it('displays on_enter script details when editing entry script', () => {
      const devStage = mockStages.find((s) => s.id === 'dev')!;
      const stateOverrides = new Map<number, any>([
        [1, 'script'],
        [5, 'on_enter'],
        [6, 'scripts/enter_dev.bat'],
      ]);

      const { html } = renderMenu(
        { stage: devStage, x: 100, y: 100, onClose: vi.fn() },
        stateOverrides
      );

      expect(html).toContain('Скрипт входа');
      expect(html).toContain('Текущий: scripts/enter_dev.bat');
      expect(html).toContain('scripts/enter_dev.bat');
      expect(html).toContain('Запускается при входе задачи в стадию');
    });

    it('shows "не задан" when hook script is not configured', () => {
      const backlogStage = mockStages.find((s) => s.id === 'backlog')!;
      // Entry script on backlog is not configured
      const stateOverrides = new Map<number, any>([
        [1, 'script'],
        [5, 'on_enter'],
        [6, ''],
      ]);

      const { html } = renderMenu(
        { stage: backlogStage, x: 100, y: 100, onClose: vi.fn() },
        stateOverrides
      );

      expect(html).toContain('Скрипт входа');
      expect(html).toContain('Текущий: не задан');
    });
  });

  describe('4. Interactivity & Transition Triggers', () => {
    it('sets scriptType to "on_leave" and populates draft when exit script button is clicked', () => {
      const backlogStage = mockStages.find((s) => s.id === 'backlog')!;
      const stateSetters = new Map<number, any>();
      const stateOverrides = new Map<number, any>();

      let capturedMode = 'main';
      let capturedScriptType = 'on_enter';
      let capturedDraftScript = '';

      stateSetters.set(1, (m: string) => { capturedMode = m; });
      stateSetters.set(5, (t: 'on_enter' | 'on_leave') => { capturedScriptType = t; });
      stateSetters.set(6, (s: string) => { capturedDraftScript = s; });

      const { node } = renderMenu(
        { stage: backlogStage, x: 100, y: 100, onClose: vi.fn() },
        stateOverrides,
        stateSetters
      );

      // Find the leave script button in the main menu children
      const menuDiv = node as React.ReactElement<any>;
      const childrenList = React.Children.toArray(menuDiv.props.children);
      const mainFrag = childrenList.find((c: any) => c && c.props && c.props.children) as React.ReactElement<any>;
      const buttons = React.Children.toArray(mainFrag.props.children);

      const leaveButton = buttons.find((btn: any) => {
        const text = btn?.props?.children;
        if (Array.isArray(text)) {
          return text.some((child: any) => child?.props?.children === 'Скрипт выхода');
        }
        return false;
      }) as React.ReactElement<any>;

      expect(leaveButton).toBeDefined();
      leaveButton.props.onClick();

      expect(capturedMode).toBe('script');
      expect(capturedScriptType).toBe('on_leave');
      expect(capturedDraftScript).toBe('.cvh/hooks/create_worktree.bat');
    });

    it('sets scriptType to "on_enter" and populates draft when enter script button is clicked', () => {
      const devStage = mockStages.find((s) => s.id === 'dev')!;
      const stateSetters = new Map<number, any>();
      const stateOverrides = new Map<number, any>();

      let capturedMode = 'main';
      let capturedScriptType = 'on_leave';
      let capturedDraftScript = '';

      stateSetters.set(1, (m: string) => { capturedMode = m; });
      stateSetters.set(5, (t: 'on_enter' | 'on_leave') => { capturedScriptType = t; });
      stateSetters.set(6, (s: string) => { capturedDraftScript = s; });

      const { node } = renderMenu(
        { stage: devStage, x: 100, y: 100, onClose: vi.fn() },
        stateOverrides,
        stateSetters
      );

      const menuDiv = node as React.ReactElement<any>;
      const childrenList = React.Children.toArray(menuDiv.props.children);
      const mainFrag = childrenList.find((c: any) => c && c.props && c.props.children) as React.ReactElement<any>;
      const buttons = React.Children.toArray(mainFrag.props.children);

      const enterButton = buttons.find((btn: any) => {
        const text = btn?.props?.children;
        if (Array.isArray(text)) {
          return text.some((child: any) => child?.props?.children === 'Скрипт входа');
        }
        return false;
      }) as React.ReactElement<any>;

      expect(enterButton).toBeDefined();
      enterButton.props.onClick();

      expect(capturedMode).toBe('script');
      expect(capturedScriptType).toBe('on_enter');
      expect(capturedDraftScript).toBe('scripts/enter_dev.bat');
    });
  });
});
