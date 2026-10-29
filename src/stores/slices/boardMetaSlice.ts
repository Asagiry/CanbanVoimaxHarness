import { StateCreator } from 'zustand';
import { BoardDef, PipelineColumn, PipelineConfig } from '../../types';
import { CANONICAL_BOARD_COLUMNS, DEFAULT_BOARD } from '../board/boardConstants';
import { BoardMetaSlice, BoardState } from './types';

export const createBoardMetaSlice: StateCreator<
  BoardState,
  [],
  [],
  BoardMetaSlice
> = (set, get) => ({
  boards: [DEFAULT_BOARD],
  activeBoardId: 'development',
  columns: CANONICAL_BOARD_COLUMNS,
  isLoadingBoards: false,
  isBoardsLoaded: false,
  isTauriConnected: false,

  setBoards: (boards) => set({ boards }),

  setActiveBoard: async (boardId: string) => {
    const { boards } = get();
    const targetBoard = boards.find((b) => b.id === boardId);
    if (!targetBoard) return;
    set({
      activeBoardId: boardId,
      columns: targetBoard.columns,
    });
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('set_active_board', { boardId });
    } catch (err) {
      console.info('set_active_board IPC failed or in web mode:', err);
    }
  },

  loadBoards: async () => {
    set({ isLoadingBoards: true });
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const [rawBoards, rawActiveId] = await Promise.all([
        invoke<BoardDef[]>('list_boards'),
        invoke<string>('get_active_board_id').catch(() => 'development'),
      ]);
      if (Array.isArray(rawBoards) && rawBoards.length > 0) {
        const activeId = rawBoards.some((b) => b.id === rawActiveId)
          ? rawActiveId
          : rawBoards[0].id;
        const currentBoard = rawBoards.find((b) => b.id === activeId) ?? rawBoards[0];

        // Dynamic Self-Healing: Verify all tasks in store against the loaded boards
        const currentTasks = get().tasks;
        const healedTasks = currentTasks.map((t) => {
          const hasBoard = rawBoards.some((b) => b.id === t.boardId);
          const effectiveBoardId = hasBoard ? t.boardId : activeId;
          const targetBoard = rawBoards.find((b) => b.id === effectiveBoardId) ?? currentBoard;
          const colExists = targetBoard.columns.some((c) => c.id === t.columnId);
          const effectiveColId = colExists ? t.columnId : (targetBoard.columns.find((c) => c.isIntake)?.id ?? targetBoard.columns[0]?.id ?? 'backlog');
          if (effectiveBoardId !== t.boardId || effectiveColId !== t.columnId) {
            return { ...t, boardId: effectiveBoardId, columnId: effectiveColId };
          }
          return t;
        });

        set({
          boards: rawBoards,
          activeBoardId: activeId,
          columns: currentBoard.columns,
          tasks: healedTasks,
          isLoadingBoards: false,
          isBoardsLoaded: true,
        });
        return;
      }
    } catch (err) {
      console.info('loadBoards IPC not available or in web mode:', err);
    }

    const current = get();
    if (current.boards.length > 0 && current.isBoardsLoaded) {
      const curBoard = current.boards.find((b) => b.id === current.activeBoardId) ?? current.boards[0];
      set({ columns: curBoard.columns, isLoadingBoards: false });
    } else {
      set({
        boards: [DEFAULT_BOARD],
        activeBoardId: 'development',
        columns: CANONICAL_BOARD_COLUMNS,
        isLoadingBoards: false,
        isBoardsLoaded: true,
      });
    }
  },

  createBoard: async (name: string, template?: string) => {
    const trimmed = name.trim();
    if (!trimmed) return null;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const newBoard = await invoke<BoardDef>('create_board', {
        name: trimmed,
        template: template || 'canonical',
      });
      if (newBoard) {
        set((state) => ({
          boards: [...state.boards, newBoard],
          activeBoardId: newBoard.id,
          columns: newBoard.columns,
        }));
        return newBoard;
      }
    } catch (err) {
      console.info('create_board IPC failed, falling back to local state:', err);
    }

    const slug = trimmed.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '') || 'board';
    const uniqueId = `${slug}-${Date.now().toString(36)}`;
    const newBoard: BoardDef = {
      id: uniqueId,
      name: trimmed,
      columns: CANONICAL_BOARD_COLUMNS,
    };
    set((state) => ({
      boards: [...state.boards, newBoard],
      activeBoardId: uniqueId,
      columns: newBoard.columns,
    }));
    return newBoard;
  },

  saveBoard: async (board: BoardDef) => {
    const { isBoardsLoaded } = get();
    if (!isBoardsLoaded) {
      console.warn('[boardStore] saveBoard aborted: boards are not loaded yet!');
      return;
    }
    set((state) => {
      const nextBoards = state.boards.map((b) => (b.id === board.id ? board : b));
      const isCurrent = state.activeBoardId === board.id;
      // Heal tasks on this board whose columns may have been removed
      const nextTasks = state.tasks.map((t) => {
        if (t.boardId === board.id) {
          const colExists = board.columns.some((c) => c.id === t.columnId);
          if (!colExists) {
            return { ...t, columnId: board.columns.find((c) => c.isIntake)?.id ?? board.columns[0]?.id ?? 'backlog' };
          }
        }
        return t;
      });
      return {
        boards: nextBoards,
        columns: isCurrent ? board.columns : state.columns,
        tasks: nextTasks,
      };
    });
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('save_board', { board });
    } catch (err) {
      console.info('save_board IPC failed or in web mode:', err);
      throw err instanceof Error ? err : new Error(String(err));
    }
  },

  deleteBoard: async (boardId: string) => {
    const { boards, activeBoardId, tasks } = get();
    if (boards.length <= 1) return;

    const filtered = boards.filter((b) => b.id !== boardId);
    const nextActiveId = activeBoardId === boardId ? filtered[0].id : activeBoardId;
    const nextBoard = filtered.find((b) => b.id === nextActiveId) ?? filtered[0];

    const updatedTasks = tasks.map((t) => {
      const isTarget = (t.boardId || 'development') === boardId;
      if (!isTarget) return t;
      const targetCols = nextBoard.columns;
      const colExists = targetCols.some((c) => c.id === t.columnId);
      const healedCol = colExists ? t.columnId : (targetCols.find((c) => c.isIntake)?.id ?? targetCols[0]?.id ?? 'backlog');
      return { ...t, boardId: nextActiveId, columnId: healedCol };
    });

    set({
      boards: filtered,
      activeBoardId: nextActiveId,
      columns: nextBoard.columns,
      tasks: updatedTasks,
    });

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('delete_board', { boardId });
    } catch (err) {
      console.info('delete_board IPC failed or in web mode:', err);
    }
  },

  saveCurrentBoard: async (columns: PipelineColumn[]) => {
    const { activeBoardId, boards, isBoardsLoaded } = get();
    if (!isBoardsLoaded) {
      console.warn('[boardStore] saveCurrentBoard aborted: boards are not loaded yet!');
      return;
    }
    const current = boards.find((b) => b.id === activeBoardId) ?? boards[0];
    if (!current) {
      set({ columns });
      return;
    }
    const updated: BoardDef = { ...current, columns };
    await get().saveBoard(updated);
  },

  setColumns: (columns) => {
    const { activeBoardId, boards } = get();
    const updatedBoards = boards.map((b) => (b.id === activeBoardId ? { ...b, columns } : b));
    set({ columns, boards: updatedBoards });
  },

  updateColumn: async (columnId: string, partial: Partial<PipelineColumn>) => {
    const { columns, activeBoardId, boards } = get();
    const nextColumns = columns.map((c) => (c.id === columnId ? { ...c, ...partial } : c));
    const updatedBoards = boards.map((b) => (b.id === activeBoardId ? { ...b, columns: nextColumns } : b));
    set({ columns: nextColumns, boards: updatedBoards });
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await get().saveCurrentBoard(nextColumns);
      let cfg: PipelineConfig | null = null;
      try {
        cfg = await invoke<PipelineConfig>('get_pipeline');
      } catch {
        cfg = null;
      }
      const pipeline: PipelineConfig = {
        id: cfg?.id ?? 'default-engineering',
        title: cfg?.title ?? 'Инженерный пайплайн',
        columns: nextColumns,
      };
      await invoke('save_pipeline', { pipeline });
    } catch (err) {
      console.info('updateColumn failed or in web mode:', err);
    }
  },

  loadColumns: async () => {
    await get().loadBoards();
  },

  addColumn: async (column: PipelineColumn) => {
    const { columns } = get();
    const nextColumns = [...columns, column];
    await get().saveCurrentBoard(nextColumns);
  },

  deleteColumn: async (columnId: string) => {
    const { columns } = get();
    const nextColumns = columns.filter((c) => c.id !== columnId);
    await get().saveCurrentBoard(nextColumns);
  },

  reorderColumns: async (columns: PipelineColumn[]) => {
    await get().saveCurrentBoard(columns);
  },
});
