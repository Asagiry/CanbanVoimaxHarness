import { describe, it, expect, beforeEach, beforeAll, vi } from 'vitest';
import { useDrawerStore } from '../drawerStore';

const mockInvoke = vi.fn().mockResolvedValue([]);

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

describe('drawerMetaSlice (Direct Navigation & Back Navigation)', () => {
  beforeAll(() => {
    (globalThis as unknown as { alert: (msg: string) => void }).alert = vi.fn();
    if (typeof (globalThis as any).window === 'undefined') {
      (globalThis as any).window = globalThis;
    }
    (globalThis as any).__TAURI_INTERNALS__ = {
      invoke: (...args: unknown[]) => mockInvoke(...args),
    };
  });

  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockResolvedValue([]);
    useDrawerStore.setState({
      isOpen: false,
      activeTaskId: null,
      activeTab: 'thoughts',
      activeSubagentId: null,
      activeSubagentTitle: null,
      activeSubagentRole: null,
      activeSubagentPrompt: null,
    });
  });

  it('1. Initial state: drawer is closed and activeTaskId is null', () => {
    const state = useDrawerStore.getState();
    expect(state.isOpen).toBe(false);
    expect(state.activeTaskId).toBeNull();
  });

  it('2. Direct navigation: openDrawer updates activeTaskId directly without history', () => {
    const store = useDrawerStore.getState();
    store.openDrawer('CVH-010');

    expect(useDrawerStore.getState().isOpen).toBe(true);
    expect(useDrawerStore.getState().activeTaskId).toBe('CVH-010');

    store.openDrawer('CVH-010.1');

    expect(useDrawerStore.getState().isOpen).toBe(true);
    expect(useDrawerStore.getState().activeTaskId).toBe('CVH-010.1');
  });

  it('3. goBackDrawer() on root task closes drawer directly', () => {
    const store = useDrawerStore.getState();
    store.openDrawer('CVH-010');

    expect(useDrawerStore.getState().activeTaskId).toBe('CVH-010');
    expect(useDrawerStore.getState().isOpen).toBe(true);

    store.goBackDrawer();

    const state = useDrawerStore.getState();
    expect(state.isOpen).toBe(false);
    expect(state.activeTaskId).toBeNull();
  });

  it('4. goBackDrawer() on subtask (CVH-010.1 / CVH-010.4) directly closes drawer to board', () => {
    const store = useDrawerStore.getState();
    store.openDrawer('CVH-010.4');

    expect(useDrawerStore.getState().activeTaskId).toBe('CVH-010.4');
    expect(useDrawerStore.getState().isOpen).toBe(true);

    store.goBackDrawer();

    const state = useDrawerStore.getState();
    expect(state.isOpen).toBe(false);
    expect(state.activeTaskId).toBeNull();
  });

  it('5. When activeSubagentId is set, goBackDrawer() closes the subagent first, then closes drawer on next call', () => {
    const store = useDrawerStore.getState();
    store.openDrawer('CVH-010.1');

    // Open subagent inside subtask
    useDrawerStore.setState({
      activeSubagentId: 'sub-agent-123',
      activeSubagentTitle: 'Audit subagent',
    });

    expect(useDrawerStore.getState().activeSubagentId).toBe('sub-agent-123');
    expect(useDrawerStore.getState().activeTaskId).toBe('CVH-010.1');

    // First goBackDrawer closes subagent, retaining task
    store.goBackDrawer();

    expect(useDrawerStore.getState().activeSubagentId).toBeNull();
    expect(useDrawerStore.getState().activeTaskId).toBe('CVH-010.1');
    expect(useDrawerStore.getState().isOpen).toBe(true);

    // Second goBackDrawer directly closes drawer
    store.goBackDrawer();

    expect(useDrawerStore.getState().isOpen).toBe(false);
    expect(useDrawerStore.getState().activeTaskId).toBeNull();
  });

  it('6. closeDrawer() resets isOpen and all transient state', () => {
    const store = useDrawerStore.getState();
    store.openDrawer('CVH-010.1');

    store.closeDrawer();

    const state = useDrawerStore.getState();
    expect(state.isOpen).toBe(false);
    expect(state.activeTaskId).toBeNull();
    expect(state.activeSubagentId).toBeNull();
  });
});
