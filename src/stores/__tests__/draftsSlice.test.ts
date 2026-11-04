import { describe, it, expect, beforeEach, beforeAll, vi } from 'vitest';
import { useDrawerStore } from '../drawerStore';
import { DRAFTS_STORAGE_KEY, persistDrafts } from '../drawer/slices/draftsSlice';

const mockInvoke = vi.fn().mockResolvedValue([]);

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

describe('draftsSlice (Steer prompt & attachment draft preservation)', () => {
  let localStorageMock: Record<string, string> = {};

  beforeAll(() => {
    (globalThis as unknown as { alert: (msg: string) => void }).alert = vi.fn();
    if (typeof (globalThis as any).window === 'undefined') {
      (globalThis as any).window = globalThis;
    }
    (globalThis as any).__TAURI_INTERNALS__ = {
      invoke: (...args: unknown[]) => mockInvoke(...args),
    };

    // Mock localStorage
    Object.defineProperty(globalThis, 'localStorage', {
      value: {
        getItem: vi.fn((key: string) => localStorageMock[key] ?? null),
        setItem: vi.fn((key: string, val: string) => {
          localStorageMock[key] = val;
        }),
        removeItem: vi.fn((key: string) => {
          delete localStorageMock[key];
        }),
        clear: vi.fn(() => {
          localStorageMock = {};
        }),
      },
      writable: true,
    });
  });

  beforeEach(() => {
    vi.clearAllMocks();
    localStorageMock = {};
    (localStorage.setItem as any).mockImplementation((key: string, val: string) => {
      localStorageMock[key] = val;
    });
    (localStorage.getItem as any).mockImplementation((key: string) => localStorageMock[key] ?? null);
    (localStorage.removeItem as any).mockImplementation((key: string) => {
      delete localStorageMock[key];
    });
    useDrawerStore.setState({
      taskDrafts: {},
    });
  });

  it('1. Saves and retrieves draft with text and images', () => {
    const store = useDrawerStore.getState();
    const testDraft = {
      text: 'Need to refactor database schema',
      images: [{ id: 'img-1', name: 'diagram.png', dataUrl: 'data:image/png;base64,abc' }],
    };

    store.setTaskDraft('CVH-026', testDraft);

    const saved = useDrawerStore.getState().getTaskDraft('CVH-026');
    expect(saved).toBeDefined();
    expect(saved?.text).toBe('Need to refactor database schema');
    expect(saved?.images).toHaveLength(1);
    expect(saved?.images[0].name).toBe('diagram.png');

    // Also persisted to localStorage
    expect(localStorage.setItem).toHaveBeenCalledWith(
      DRAFTS_STORAGE_KEY,
      JSON.stringify({ 'CVH-026': testDraft })
    );
  });

  it('2. Automatically clears draft when text is empty/whitespace and no images', () => {
    const store = useDrawerStore.getState();
    store.setTaskDraft('CVH-026', {
      text: 'Draft to be cleared',
      images: [],
    });
    expect(useDrawerStore.getState().getTaskDraft('CVH-026')).toBeDefined();

    // User clears text
    store.setTaskDraft('CVH-026', {
      text: '   ',
      images: [],
    });

    expect(useDrawerStore.getState().getTaskDraft('CVH-026')).toBeUndefined();
    expect(useDrawerStore.getState().taskDrafts['CVH-026']).toBeUndefined();
  });

  it('3. Explicit clearTaskDraft removes draft and cleans localStorage when empty', () => {
    const store = useDrawerStore.getState();
    store.setTaskDraft('CVH-026', {
      text: 'Hello world',
      images: [],
    });
    expect(useDrawerStore.getState().getTaskDraft('CVH-026')).toBeDefined();

    store.clearTaskDraft('CVH-026');

    expect(useDrawerStore.getState().getTaskDraft('CVH-026')).toBeUndefined();
    expect(localStorage.removeItem).toHaveBeenCalledWith(DRAFTS_STORAGE_KEY);
  });

  it('4. Subagent draft isolation: parent draft and subagent drafts do not collide', () => {
    const store = useDrawerStore.getState();
    const parentDraft = { text: 'Parent prompt', images: [] };
    const subDraft1 = { text: 'Subagent prompt 1', images: [] };
    const subDraft2 = { text: 'Subagent prompt 2', images: [] };

    store.setTaskDraft('CVH-100', parentDraft);
    store.setTaskDraft('CVH-100:sub:sub-1', subDraft1);
    store.setTaskDraft('CVH-100:sub:sub-2', subDraft2);

    expect(store.getTaskDraft('CVH-100')?.text).toBe('Parent prompt');
    expect(store.getTaskDraft('CVH-100:sub:sub-1')?.text).toBe('Subagent prompt 1');
    expect(store.getTaskDraft('CVH-100:sub:sub-2')?.text).toBe('Subagent prompt 2');

    // Clearing one subagent draft leaves parent and sibling subagent intact
    store.clearTaskDraft('CVH-100:sub:sub-1');
    expect(store.getTaskDraft('CVH-100:sub:sub-1')).toBeUndefined();
    expect(store.getTaskDraft('CVH-100')?.text).toBe('Parent prompt');
    expect(store.getTaskDraft('CVH-100:sub:sub-2')?.text).toBe('Subagent prompt 2');
  });

  it('5. Clearing parent task draft also purges its subagent drafts', () => {
    const store = useDrawerStore.getState();
    store.setTaskDraft('CVH-200', { text: 'Parent 200', images: [] });
    store.setTaskDraft('CVH-200:sub:sub-1', { text: 'Child 1', images: [] });
    store.setTaskDraft('CVH-300', { text: 'Unrelated task', images: [] });

    // Clearing CVH-200 purges CVH-200 and all CVH-200:sub:*
    store.clearTaskDraft('CVH-200');

    expect(store.getTaskDraft('CVH-200')).toBeUndefined();
    expect(store.getTaskDraft('CVH-200:sub:sub-1')).toBeUndefined();
    expect(store.getTaskDraft('CVH-300')?.text).toBe('Unrelated task');
  });

  it('6. Quota fallback: if setItem throws, falls back to text-only drafts', () => {
    const testDrafts = {
      'CVH-500': {
        text: 'Crucial prompt text',
        images: [{ id: '1', name: 'huge.png', dataUrl: 'data:image/png;base64,' + 'x'.repeat(1000) }],
      },
    };

    let attempts = 0;
    (localStorage.setItem as any).mockImplementation((key: string, val: string) => {
      attempts++;
      if (attempts === 1) {
        throw new Error('QuotaExceededError');
      }
      localStorageMock[key] = val;
    });

    persistDrafts(testDrafts);

    expect(attempts).toBe(2);
    expect(localStorageMock[DRAFTS_STORAGE_KEY]).toBe(
      JSON.stringify({
        'CVH-500': { text: 'Crucial prompt text', images: [] },
      })
    );
  });

  it('7. clearTaskActivity purges task drafts', async () => {
    const store = useDrawerStore.getState();
    store.setTaskDraft('CVH-600', { text: 'Draft before clear', images: [] });
    store.setTaskDraft('CVH-600:sub:sub-1', { text: 'Sub draft', images: [] });

    await store.clearTaskActivity('CVH-600');

    expect(store.getTaskDraft('CVH-600')).toBeUndefined();
    expect(store.getTaskDraft('CVH-600:sub:sub-1')).toBeUndefined();
  });
});
