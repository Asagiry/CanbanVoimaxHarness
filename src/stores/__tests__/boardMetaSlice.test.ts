import { describe, it, expect, beforeEach, vi } from 'vitest';
import { useBoardStore } from '../boardStore';
import { CANONICAL_BOARD_COLUMNS, DEFAULT_BOARD } from '../board/boardConstants';

const mockInvoke = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

describe('boardMetaSlice rollback & stage synchronization', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockResolvedValue(null);
    useBoardStore.setState({
      boards: [
        DEFAULT_BOARD,
        { id: 'ops', name: 'Ops', stages: CANONICAL_BOARD_COLUMNS, columns: CANONICAL_BOARD_COLUMNS },
      ],
      activeBoardId: 'development',
      stages: CANONICAL_BOARD_COLUMNS,
      columns: CANONICAL_BOARD_COLUMNS,
      tasks: [],
      isBoardsLoaded: true,
      boardError: null,
    });
  });

  it('updateStage updates both stages and columns and rolls back on IPC failure', async () => {
    await useBoardStore.getState().updateStage('dev', { title: 'Engineering' });
    expect(useBoardStore.getState().stages.find((s) => s.id === 'dev')?.title).toBe('Engineering');
    expect(useBoardStore.getState().columns.find((s) => s.id === 'dev')?.title).toBe('Engineering');

    mockInvoke.mockRejectedValueOnce(new Error('save_board failed'));
    await useBoardStore.getState().updateStage('dev', { title: 'Broken Title' });

    expect(useBoardStore.getState().stages.find((s) => s.id === 'dev')?.title).toBe('Engineering');
    expect(useBoardStore.getState().columns.find((s) => s.id === 'dev')?.title).toBe('Engineering');
    expect(useBoardStore.getState().boardError).toContain('save_board failed');
  });

  it('saveBoard rolls back on IPC failure', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('disk full'));
    const modified = { ...DEFAULT_BOARD, name: 'Renamed' };
    await expect(useBoardStore.getState().saveBoard(modified)).rejects.toThrow('disk full');
    expect(useBoardStore.getState().boards[0].name).toBe(DEFAULT_BOARD.name);
    expect(useBoardStore.getState().boardError).toContain('disk full');
  });

  it('deleteBoard rolls back on IPC failure', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('cannot delete board'));
    await useBoardStore.getState().deleteBoard('ops');
    expect(useBoardStore.getState().boards.map((b) => b.id)).toEqual(['development', 'ops']);
    expect(useBoardStore.getState().boardError).toContain('cannot delete board');
  });
});
