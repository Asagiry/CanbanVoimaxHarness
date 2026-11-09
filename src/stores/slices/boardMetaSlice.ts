import { StateCreator } from 'zustand';
import { BoardDef, PipelineStage, PipelineConfig } from '../../types';
import { CANONICAL_BOARD_STAGES, DEFAULT_BOARD } from '../board/boardConstants';
import { BoardMetaSlice, BoardState } from './types';

export const createBoardMetaSlice: StateCreator<
  BoardState,
  [],
  [],
  BoardMetaSlice
> = (set, get) => ({
  boards: [DEFAULT_BOARD],
  activeBoardId: 'development',
  stages: CANONICAL_BOARD_STAGES,
  get columns() {
    return get()?.stages ?? CANONICAL_BOARD_STAGES;
  },
  isLoadingBoards: false,
  isBoardsLoaded: false,
  isTauriConnected: false,

  setBoards: (boards) => set({ boards }),

  setActiveBoard: async (boardId: string) => {
    const { boards } = get();
    const targetBoard = boards.find((b) => b.id === boardId);
    if (!targetBoard) return;
    const stages = targetBoard.stages ?? targetBoard.columns ?? CANONICAL_BOARD_STAGES;
    set({
      activeBoardId: boardId,
      stages,
      columns: stages,
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
        const normalizedBoards: BoardDef[] = rawBoards.map((b) => {
          const boardStages = b.stages ?? b.columns ?? CANONICAL_BOARD_STAGES;
          return {
            ...b,
            stages: boardStages,
            columns: boardStages,
          };
        });
        const activeId = normalizedBoards.some((b) => b.id === rawActiveId)
          ? rawActiveId
          : normalizedBoards[0].id;
        const currentBoard = normalizedBoards.find((b) => b.id === activeId) ?? normalizedBoards[0];

        // Dynamic Self-Healing: Verify all tasks in store against the loaded boards
        const currentTasks = get().tasks;
        let healedTasks = currentTasks;
        if (!get().isLoadingTasks && currentTasks.length > 0) {
          healedTasks = currentTasks.map((t) => {
            const hasBoard = normalizedBoards.some((b) => b.id === t.boardId);
            const effectiveBoardId = hasBoard ? t.boardId : activeId;
            const targetBoard = normalizedBoards.find((b) => b.id === effectiveBoardId) ?? currentBoard;
            const targetStages = targetBoard.stages ?? targetBoard.columns ?? CANONICAL_BOARD_STAGES;
            const stageExists = targetStages.some((c) => c.id === t.stageId);
            const effectiveColId = stageExists
              ? t.stageId
              : (targetStages.find((c) => c.isIntake)?.id ?? targetStages[0]?.id ?? 'backlog');
            if (effectiveBoardId !== t.boardId || effectiveColId !== t.stageId) {
              return { ...t, boardId: effectiveBoardId, stageId: effectiveColId, columnId: effectiveColId };
            }
            return t;
          });
        }

        set({
          boards: normalizedBoards,
          activeBoardId: activeId,
          stages: currentBoard.stages,
          columns: currentBoard.stages,
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
      const curStages = curBoard.stages ?? curBoard.columns ?? CANONICAL_BOARD_STAGES;
      set({ stages: curStages, columns: curStages, isLoadingBoards: false });
    } else {
      set({
        boards: [DEFAULT_BOARD],
        activeBoardId: 'development',
        stages: CANONICAL_BOARD_STAGES,
        columns: CANONICAL_BOARD_STAGES,
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
      const rawBoard = await invoke<BoardDef>('create_board', {
        name: trimmed,
        template: template || 'canonical',
      });
      if (rawBoard) {
        const boardStages = rawBoard.stages ?? rawBoard.columns ?? CANONICAL_BOARD_STAGES;
        const newBoard: BoardDef = {
          ...rawBoard,
          stages: boardStages,
          columns: boardStages,
        };
        set((state) => ({
          boards: [...state.boards, newBoard],
          activeBoardId: newBoard.id,
          stages: newBoard.stages,
          columns: newBoard.stages,
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
      stages: CANONICAL_BOARD_STAGES,
      columns: CANONICAL_BOARD_STAGES,
    };
    set((state) => ({
      boards: [...state.boards, newBoard],
      activeBoardId: uniqueId,
      stages: newBoard.stages,
      columns: newBoard.stages,
    }));
    return newBoard;
  },

  saveBoard: async (board: BoardDef) => {
    const { isBoardsLoaded } = get();
    if (!isBoardsLoaded) {
      console.warn('[boardStore] saveBoard aborted: boards are not loaded yet!');
      return;
    }
    const boardStages = board.stages ?? board.columns ?? CANONICAL_BOARD_STAGES;
    const normalizedBoard: BoardDef = {
      ...board,
      stages: boardStages,
      columns: boardStages,
    };
    set((state) => {
      const nextBoards = state.boards.map((b) => (b.id === normalizedBoard.id ? normalizedBoard : b));
      const isCurrent = state.activeBoardId === normalizedBoard.id;
      // Heal tasks on this board whose stages may have been removed
      const nextTasks = state.tasks.map((t) => {
        if (t.boardId === normalizedBoard.id) {
          const stageExists = normalizedBoard.stages.some((c) => c.id === t.stageId);
          if (!stageExists) {
            const fallbackStage = normalizedBoard.stages.find((c) => c.isIntake)?.id ?? normalizedBoard.stages[0]?.id ?? 'backlog';
            return { ...t, stageId: fallbackStage, columnId: fallbackStage };
          }
        }
        return t;
      });
      const activeStages = isCurrent ? normalizedBoard.stages : state.stages;
      return {
        boards: nextBoards,
        stages: activeStages,
        columns: activeStages,
        tasks: nextTasks,
      };
    });
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const ipcBoard = {
        id: normalizedBoard.id,
        name: normalizedBoard.name,
        stages: normalizedBoard.stages,
      };
      await invoke('save_board', { board: ipcBoard });
    } catch (err) {
      console.error('save_board IPC failed:', err);
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
      const targetStages = nextBoard.stages ?? nextBoard.columns ?? CANONICAL_BOARD_STAGES;
      const stageExists = targetStages.some((c) => c.id === t.stageId);
      const healedStage = stageExists ? t.stageId : (targetStages.find((c) => c.isIntake)?.id ?? targetStages[0]?.id ?? 'backlog');
      return { ...t, boardId: nextActiveId, stageId: healedStage, columnId: healedStage };
    });

    const nextStages = nextBoard.stages ?? nextBoard.columns ?? CANONICAL_BOARD_STAGES;
    set({
      boards: filtered,
      activeBoardId: nextActiveId,
      stages: nextStages,
      columns: nextStages,
      tasks: updatedTasks,
    });

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('delete_board', { boardId });
    } catch (err) {
      console.info('delete_board IPC failed or in web mode:', err);
    }
  },

  saveCurrentBoard: async (stages: PipelineStage[]) => {
    const { activeBoardId, boards, isBoardsLoaded } = get();
    if (!isBoardsLoaded) {
      console.warn('[boardStore] saveCurrentBoard aborted: boards are not loaded yet!');
      return;
    }
    const current = boards.find((b) => b.id === activeBoardId) ?? boards[0];
    if (!current) {
      set({ stages, columns: stages });
      return;
    }
    const updated: BoardDef = { ...current, stages, columns: stages };
    await get().saveBoard(updated);
  },

  setStages: (stages) => {
    const { activeBoardId, boards } = get();
    const updatedBoards = boards.map((b) => (b.id === activeBoardId ? { ...b, stages, columns: stages } : b));
    set({ stages, columns: stages, boards: updatedBoards });
  },

  setColumns: (columns) => {
    get().setStages(columns);
  },

  updateStage: async (stageId: string, partial: Partial<PipelineStage>) => {
    const { stages, activeBoardId, boards } = get();
    const nextStages = stages.map((c) => (c.id === stageId ? { ...c, ...partial } : c));
    const updatedBoards = boards.map((b) => (b.id === activeBoardId ? { ...b, stages: nextStages, columns: nextStages } : b));
    set({ stages, columns: nextStages, boards: updatedBoards });
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await get().saveCurrentBoard(nextStages);
      let cfg: PipelineConfig | null = null;
      try {
        cfg = await invoke<PipelineConfig>('get_pipeline');
      } catch {
        cfg = null;
      }
      const pipeline = {
        id: cfg?.id ?? 'default-engineering',
        title: cfg?.title ?? 'Инженерный пайплайн',
        stages: nextStages,
      };
      await invoke('save_pipeline', { pipeline });
    } catch (err) {
      console.info('updateStage failed or in web mode:', err);
    }
  },

  loadStages: async () => {
    await get().loadBoards();
  },

  addStage: async (stage: PipelineStage) => {
    const { stages } = get();
    const nextStages = [...stages, stage];
    await get().saveCurrentBoard(nextStages);
  },

  deleteStage: async (stageId: string) => {
    const { stages } = get();
    const nextStages = stages.filter((c) => c.id !== stageId);
    await get().saveCurrentBoard(nextStages);
  },

  reorderStages: async (stages: PipelineStage[]) => {
    await get().saveCurrentBoard(stages);
  },
});
