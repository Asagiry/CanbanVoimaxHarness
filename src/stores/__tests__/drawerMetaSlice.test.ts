import { describe, it, expect, beforeEach, beforeAll, vi } from 'vitest';
import { useDrawerStore } from '../drawerStore';

const mockInvoke = vi.fn().mockResolvedValue([]);

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

describe('drawerMetaSlice (Task History & Back Navigation)', () => {
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
      taskHistory: [],
      activeSubagentId: null,
      activeSubagentTitle: null,
      activeSubagentRole: null,
      activeSubagentPrompt: null,
    });
  });

  it('1. Initial state: taskHistory is empty', () => {
    const state = useDrawerStore.getState();
    expect(state.taskHistory).toEqual([]);
    expect(state.isOpen).toBe(false);
    expect(state.activeTaskId).toBeNull();
  });

  it('2. Sequential navigation: openDrawer("CVH-010") -> openDrawer("CVH-010.1") records history', () => {
    const store = useDrawerStore.getState();
    store.openDrawer('CVH-010');

    expect(useDrawerStore.getState().activeTaskId).toBe('CVH-010');
    expect(useDrawerStore.getState().taskHistory).toEqual([]);

    store.openDrawer('CVH-010.1');

    expect(useDrawerStore.getState().activeTaskId).toBe('CVH-010.1');
    expect(useDrawerStore.getState().taskHistory).toEqual(['CVH-010']);
  });

  it('3. goBackDrawer() pops CVH-010 and reloads dialog', () => {
    const store = useDrawerStore.getState();
    const loadTaskDialogSpy = vi.spyOn(store, 'loadTaskDialog');
    const loadSubagentsSpy = vi.spyOn(store, 'loadSubagents');

    store.openDrawer('CVH-010');
    store.openDrawer('CVH-010.1');

    expect(useDrawerStore.getState().activeTaskId).toBe('CVH-010.1');
    expect(useDrawerStore.getState().taskHistory).toEqual(['CVH-010']);

    loadTaskDialogSpy.mockClear();
    loadSubagentsSpy.mockClear();

    store.goBackDrawer();

    const state = useDrawerStore.getState();
    expect(state.activeTaskId).toBe('CVH-010');
    expect(state.taskHistory).toEqual([]);
    expect(state.isOpen).toBe(true);
    expect(loadTaskDialogSpy).toHaveBeenCalledWith('CVH-010');
    expect(loadSubagentsSpy).toHaveBeenCalledWith('CVH-010');
  });

  it('4. goBackDrawer() on root closes drawer', () => {
    const store = useDrawerStore.getState();
    store.openDrawer('CVH-010');

    expect(useDrawerStore.getState().activeTaskId).toBe('CVH-010');
    expect(useDrawerStore.getState().taskHistory).toEqual([]);

    store.goBackDrawer();

    const state = useDrawerStore.getState();
    expect(state.isOpen).toBe(false);
    expect(state.activeTaskId).toBeNull();
    expect(state.taskHistory).toEqual([]);
  });

  it('5. Fallback when subtask is opened directly without history: goBackDrawer() navigates to CVH-010', () => {
    const store = useDrawerStore.getState();
    const loadTaskDialogSpy = vi.spyOn(store, 'loadTaskDialog');
    const loadSubagentsSpy = vi.spyOn(store, 'loadSubagents');

    // Simulate direct opening of subtask when drawer was closed (history empty)
    store.openDrawer('CVH-010.1');

    expect(useDrawerStore.getState().activeTaskId).toBe('CVH-010.1');
    expect(useDrawerStore.getState().taskHistory).toEqual([]);

    loadTaskDialogSpy.mockClear();
    loadSubagentsSpy.mockClear();

    store.goBackDrawer();

    const state = useDrawerStore.getState();
    expect(state.activeTaskId).toBe('CVH-010');
    expect(state.taskHistory).toEqual([]);
    expect(state.isOpen).toBe(true);
    expect(loadTaskDialogSpy).toHaveBeenCalledWith('CVH-010');
    expect(loadSubagentsSpy).toHaveBeenCalledWith('CVH-010');

    // Navigating back once more from the root task closes the drawer
    store.goBackDrawer();
    expect(useDrawerStore.getState().isOpen).toBe(false);
    expect(useDrawerStore.getState().activeTaskId).toBeNull();
  });

  it('6. When activeSubagentId is set, goBackDrawer() closes the subagent first', () => {
    const store = useDrawerStore.getState();
    store.openDrawer('CVH-010');
    store.openDrawer('CVH-010.1');

    // Open subagent inside subtask
    useDrawerStore.setState({
      activeSubagentId: 'sub-agent-123',
      activeSubagentTitle: 'Audit subagent',
    });

    expect(useDrawerStore.getState().activeSubagentId).toBe('sub-agent-123');

    // First goBackDrawer should close subagent, retaining task and history
    store.goBackDrawer();

    expect(useDrawerStore.getState().activeSubagentId).toBeNull();
    expect(useDrawerStore.getState().activeTaskId).toBe('CVH-010.1');
    expect(useDrawerStore.getState().taskHistory).toEqual(['CVH-010']);

    // Second goBackDrawer should pop history back to parent task
    store.goBackDrawer();
    expect(useDrawerStore.getState().activeTaskId).toBe('CVH-010');
    expect(useDrawerStore.getState().taskHistory).toEqual([]);

    // Third goBackDrawer closes drawer
    store.goBackDrawer();
    expect(useDrawerStore.getState().isOpen).toBe(false);
  });

  it('7. Re-opening the exact same task does not push duplicate to history', () => {
    const store = useDrawerStore.getState();
    store.openDrawer('CVH-010');
    store.openDrawer('CVH-010');

    expect(useDrawerStore.getState().activeTaskId).toBe('CVH-010');
    expect(useDrawerStore.getState().taskHistory).toEqual([]);
  });

  it('8. closeDrawer() resets taskHistory and all transient state', () => {
    const store = useDrawerStore.getState();
    store.openDrawer('CVH-010');
    store.openDrawer('CVH-010.1');
    expect(useDrawerStore.getState().taskHistory).toEqual(['CVH-010']);

    store.closeDrawer();

    const state = useDrawerStore.getState();
    expect(state.isOpen).toBe(false);
    expect(state.activeTaskId).toBeNull();
    expect(state.taskHistory).toEqual([]);
    expect(state.activeSubagentId).toBeNull();
  });
});
