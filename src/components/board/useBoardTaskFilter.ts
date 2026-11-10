import { useMemo } from 'react';
import { KanbanTask, PipelineColumn, BoardDef } from '../../types';
import { isDoneColumnDef } from '../../stores/boardStore';

export interface UseBoardTaskFilterOptions {
  tasks: KanbanTask[];
  columns: PipelineColumn[];
  boards: BoardDef[];
  activeBoardId: string | null;
  searchQuery: string;
}

export function useBoardTaskFilter({
  tasks,
  columns,
  boards,
  activeBoardId,
  searchQuery,
}: UseBoardTaskFilterOptions) {
  // Filter tasks based on activeBoardId and searchQuery:
  // ZERO INVISIBLE TASKS (§ 2.18 & § 2.3): Every task must be visible on the board it belongs to,
  // or on the active board if orphaned, if its declared board is absent, or if its column belongs to the active board.
  const currentBoardTasks = useMemo(() => {
    return tasks.filter((task) => {
      // 1. Direct match: task belongs to activeBoardId or has no boardId assigned
      if (!task.boardId || task.boardId === activeBoardId) {
        return true;
      }
      const matchingBoard = boards.find((b) => b.id === task.boardId);
      // 2. Fallback: if declared board no longer exists, show on active board
      if (!matchingBoard) {
        return true;
      }
      // 3. Self-heal: If declared board does NOT have task's column, but active board DOES,
      // show on active board (prevents vanishing tasks when moved across boards or boardId drifted).
      const boardStages = matchingBoard.stages ?? matchingBoard.columns ?? [];
      const boardHasCol = boardStages.some(
        (c) => c.id === task.stageId || c.id.toLowerCase() === task.stageId?.toLowerCase()
      );
      const activeHasCol = columns.some(
        (c) => c.id === task.stageId || c.id.toLowerCase() === task.stageId?.toLowerCase()
      );
      if (!boardHasCol && activeHasCol) {
        return true;
      }
      return false;
    });
  }, [tasks, columns, boards, activeBoardId]);

  const filteredTasks = useMemo(() => {
    if (!searchQuery) return currentBoardTasks;
    const q = searchQuery.toLowerCase();
    return currentBoardTasks.filter((task) => {
      return (
        task.id.toLowerCase().includes(q) ||
        task.title.toLowerCase().includes(q) ||
        (task.branch ? task.branch.toLowerCase().includes(q) : false) ||
        (task.description && task.description.toLowerCase().includes(q))
      );
    });
  }, [currentBoardTasks, searchQuery]);

  const doneParentIds = useMemo(() => {
    const set = new Set<string>();
    for (const t of tasks) {
      const col = columns.find((c) => c.id === t.stageId);
      const isTerminal = isDoneColumnDef(col) || t.stageId === 'ready' || t.stageId === 'done';
      if (isTerminal) {
        set.add(t.id);
      }
    }
    return set;
  }, [tasks, columns]);

  const rootTasksCount = useMemo(
    () => currentBoardTasks.filter((t) => !t.parentId).length,
    [currentBoardTasks]
  );

  return {
    currentBoardTasks,
    filteredTasks,
    doneParentIds,
    rootTasksCount,
  };
}
