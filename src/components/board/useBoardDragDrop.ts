import { useState, useCallback } from 'react';
import { DropResult } from '@hello-pangea/dnd';
import { useBoardStore, isDoneColumnDef, sortDoneTasksByCompletionDateDesc } from '../../stores/boardStore';
import { PipelineColumn, KanbanTask } from '../../types';
import { useTranslation } from '../../stores/languageStore';

export interface UseBoardDragDropOptions {
  columns: PipelineColumn[];
  tasks: KanbanTask[];
  activeBoardId: string | null;
  searchQuery: string;
  filteredTasks: KanbanTask[];
  currentBoardTasks: KanbanTask[];
}

export function useBoardDragDrop({
  columns,
  tasks,
  activeBoardId,
  searchQuery,
  filteredTasks,
  currentBoardTasks,
}: UseBoardDragDropOptions) {
  const setTasks = useBoardStore((s) => s.setTasks);
  const reorderTask = useBoardStore((s) => s.reorderTask);
  const setColumns = useBoardStore((s) => s.setColumns);
  const saveCurrentBoard = useBoardStore((s) => s.saveCurrentBoard);
  const { t } = useTranslation();
  const [columnsError, setColumnsError] = useState<string | null>(null);

  const savePipelineConfig = useCallback(async (next: PipelineColumn[]): Promise<boolean> => {
    try {
      await saveCurrentBoard(next);
      return true;
    } catch (err) {
      console.error('saveCurrentBoard failed, rolling back:', err);
      return false;
    }
  }, [saveCurrentBoard]);

  const handleReorderColumns = async (sourceIndex: number, destinationIndex: number) => {
    if (sourceIndex === destinationIndex) return;
    if (columns.length <= 2) return;

    const prev = columns;
    const sourceCol = prev[sourceIndex];
    if (!sourceCol) return;
    if (
      sourceCol.isIntake ||
      Boolean(sourceCol.terminal) ||
      sourceCol.id === 'backlog' ||
      sourceCol.id === 'ready' ||
      sourceIndex === 0 ||
      sourceIndex === prev.length - 1
    ) {
      return;
    }
    if (destinationIndex <= 0) return;

    setColumnsError(null);
    const reordered = Array.from(prev);
    const [moved] = reordered.splice(sourceIndex, 1);
    if (!moved) return;

    const terminalIdx = prev.findIndex((c) => Boolean(c.terminal));
    const readyIdx = prev.findIndex((c) => c.id === 'ready');
    const tailStart =
      terminalIdx !== -1 ? terminalIdx : readyIdx !== -1 ? readyIdx : prev.length - 1;
    const maxDest = Math.max(1, tailStart - 1);
    const safeDest = Math.max(1, Math.min(destinationIndex, maxDest));
    reordered.splice(safeDest, 0, moved);

    const intake = reordered.find((c) => c.isIntake || c.id === 'backlog') ?? prev[0];
    const terminal = reordered.slice().reverse().find((c) => c.terminal || c.id === 'ready') ?? prev[prev.length - 1];
    const middle = reordered.filter((c) => c.id !== intake.id && c.id !== terminal.id);
    const rawNext = [intake, ...middle, terminal];
    const next = rawNext.map((col, idx) => ({
      ...col,
      isIntake: idx === 0 ? true : undefined,
      terminal: idx === rawNext.length - 1 ? true : undefined,
    }));

    if (prev.length === next.length && prev.every((c, i) => c.id === next[i]?.id)) return;

    setColumns(next);
    const saved = await savePipelineConfig(next);
    if (!saved) {
      setColumns(prev);
      setColumnsError(t('board.errReorderRollback'));
    }
  };

  const handleDragEnd = async (result: DropResult) => {
    const { source, destination, draggableId, type } = result;
    if (!destination) return;
    if (source.droppableId === destination.droppableId && source.index === destination.index) {
      return;
    }

    if (type === 'COLUMN') {
      const sourceCol = columns[source.index];
      if (!sourceCol) return;
      if (
        sourceCol.isIntake ||
        Boolean(sourceCol.terminal) ||
        sourceCol.id === 'backlog' ||
        sourceCol.id === 'ready' ||
        source.index === 0 ||
        source.index === columns.length - 1
      ) {
        return;
      }
      if (destination.index === 0) return;

      const terminalIdx = columns.findIndex((c) => Boolean(c.terminal));
      const readyIdx = columns.findIndex((c) => c.id === 'ready');
      const tailStart =
        terminalIdx !== -1 ? terminalIdx : readyIdx !== -1 ? readyIdx : columns.length - 1;
      if (source.index >= tailStart) return;
      const maxDest = Math.max(1, tailStart - 1);
      const clampedDestinationIndex = Math.max(1, Math.min(destination.index, maxDest));

      void handleReorderColumns(source.index, clampedDestinationIndex);
      return;
    }

    const task = tasks.find((t) => t.id === draggableId);
    if (!task) return;

    const sourceColId = source.droppableId;
    const destColId = destination.droppableId;
    const sourceIndex = source.index;
    const destIndex = destination.index;

    const isDoneCol = (colId: string) => {
      const col = columns.find((c) => c.id === colId);
      return isDoneColumnDef(col);
    };

    const firstColId = columns.find((c) => c.isIntake)?.id ?? columns[0]?.id ?? 'backlog';
    const getColumnDisplayTasks = (targetColId: string) => {
      const isDone = isDoneCol(targetColId);
      const isFirst = targetColId === firstColId;
      const baseTasks = searchQuery ? filteredTasks : currentBoardTasks;
      const colTasks = baseTasks.filter((t) => {
        if (t.stageId === targetColId) return true;
        if (isFirst) {
          const colExists = columns.some((c) => c.id === t.stageId);
          return !colExists;
        }
        return false;
      });
      return isDone
        ? colTasks
            .filter((t) => !t.parentId)
            .slice()
            .sort(sortDoneTasksByCompletionDateDesc)
        : colTasks;
    };

    if (sourceColId === destColId) {
      if (sourceIndex === destIndex) return;
      if (isDoneCol(destColId)) {
        return;
      }

      const colDisplayTasks = getColumnDisplayTasks(destColId);
      const otherTasks = colDisplayTasks.filter((t) => t.id !== draggableId);
      const nextTasks = [...tasks];
      const taskPos = nextTasks.findIndex((t) => t.id === draggableId);
      if (taskPos === -1) return;
      const [movedTask] = nextTasks.splice(taskPos, 1);
      const effectiveBoardId = activeBoardId || 'development';
      const updatedMovedTask: KanbanTask = {
        ...movedTask,
        boardId: effectiveBoardId,
      };

      if (destIndex < otherTasks.length) {
        const refTask = otherTasks[destIndex];
        const refPos = nextTasks.findIndex((t) => t.id === refTask.id);
        if (refPos !== -1) {
          nextTasks.splice(refPos, 0, updatedMovedTask);
        } else {
          nextTasks.push(updatedMovedTask);
        }
      } else {
        if (otherTasks.length > 0) {
          const lastTask = otherTasks[otherTasks.length - 1];
          const lastPos = nextTasks.findIndex((t) => t.id === lastTask.id);
          if (lastPos !== -1) {
            nextTasks.splice(lastPos + 1, 0, updatedMovedTask);
          } else {
            nextTasks.push(updatedMovedTask);
          }
        } else {
          nextTasks.push(updatedMovedTask);
        }
      }

      setTasks(nextTasks);

      try {
        await reorderTask(draggableId, destColId, destIndex);
      } catch (err: unknown) {
        console.warn(`[handleDragEnd] reorderTask rejected for ${draggableId}, reverting state:`, err);
        setTasks(tasks);
        const msg = err instanceof Error ? err.message : String(err);
        alert(t('board.errMoveTask', { msg }));
      }
      return;
    }

    const destDisplayTasks = getColumnDisplayTasks(destColId);
    const originalTasks = [...tasks];
    const taskPos = originalTasks.findIndex((t) => t.id === draggableId);
    if (taskPos === -1) return;

    const [movedTask] = originalTasks.splice(taskPos, 1);
    const destColDef = columns.find((c) => c.id === destColId);
    const effectiveBoardId = activeBoardId || 'development';
    const isDone = isDoneCol(destColId);
    const targetStatus = isDone
      ? 'done'
      : destColDef?.persona
      ? 'running'
      : 'idle';

    const completedAt = isDone ? Date.now() : undefined;
    const totalElapsed = isDone && movedTask.createdAt && completedAt
      ? Math.max(0, Math.floor((completedAt - movedTask.createdAt) / 1000))
      : movedTask.totalElapsed;

    const updatedMovedTask: KanbanTask = {
      ...movedTask,
      stageId: destColId,
      columnId: destColId,
      boardId: effectiveBoardId,
      persona: destColDef?.persona || movedTask.persona,
      status: targetStatus,
      completedAt,
      totalElapsed,
      updatedAt: Date.now(),
      stageEnteredAt: Date.now(),
    };

    const nextTasks = [...originalTasks];
    if (destIndex < destDisplayTasks.length) {
      const refTask = destDisplayTasks[destIndex];
      const refPos = nextTasks.findIndex((t) => t.id === refTask.id);
      if (refPos !== -1) {
        nextTasks.splice(refPos, 0, updatedMovedTask);
      } else {
        nextTasks.push(updatedMovedTask);
      }
    } else {
      if (destDisplayTasks.length > 0) {
        const lastTask = destDisplayTasks[destDisplayTasks.length - 1];
        const lastPos = nextTasks.findIndex((t) => t.id === lastTask.id);
        if (lastPos !== -1) {
          nextTasks.splice(lastPos + 1, 0, updatedMovedTask);
        } else {
          nextTasks.push(updatedMovedTask);
        }
      } else {
        nextTasks.push(updatedMovedTask);
      }
    }

    setTasks(nextTasks);

    try {
      await reorderTask(draggableId, destColId, destIndex);
    } catch (err: unknown) {
      console.warn(`[handleDragEnd] reorderTask rejected for ${draggableId}, reverting state:`, err);
      setTasks(tasks);
      const msg = err instanceof Error ? err.message : String(err);
      alert(t('board.errMoveTask', { msg }));
    }
  };

  return {
    handleDragEnd,
    columnsError,
    setColumnsError,
    savePipelineConfig,
  };
}
