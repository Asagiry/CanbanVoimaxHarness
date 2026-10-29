import React, { useEffect, useState, useCallback, useMemo } from 'react';
import { DragDropContext, Droppable, DropResult } from '@hello-pangea/dnd';
import { useBoardStore, isDoneColumnDef, sortDoneTasksByCompletionDateDesc } from '../../stores/boardStore';
import KanbanColumn from './KanbanColumn';
import { PipelineColumn, KanbanTask } from '../../types';
import { X } from 'lucide-react';
import BoardTabsHeader from './BoardTabsHeader';
import CreateTaskModal from './modals/CreateTaskModal';
import CreateColumnModal from './modals/CreateColumnModal';
import CreateBoardModal from './CreateBoardModal';
import { useTranslation } from '../../stores/languageStore';

export default function KanbanBoard() {
  const {
    columns,
    tasks,
    searchQuery,
    setTasks,
    reorderTask,
    tickTimers,
    setColumns,
    boards,
    activeBoardId,
    saveCurrentBoard,
  } = useBoardStore();
  const { t } = useTranslation();

  const [isNewTaskModalOpen, setIsNewTaskModalOpen] = useState(false);
  const [isAddColumnOpen, setIsAddColumnOpen] = useState(false);
  const [isCreateBoardModalOpen, setIsCreateBoardModalOpen] = useState(false);
  const [columnsError, setColumnsError] = useState<string | null>(null);

  // 1-second timer ticker interval for live cards
  useEffect(() => {
    const timer = setInterval(() => {
      tickTimers();
    }, 1000);

    return () => clearInterval(timer);
  }, [tickTimers]);

  const savePipelineConfig = useCallback(async (next: PipelineColumn[]): Promise<boolean> => {
    try {
      await saveCurrentBoard(next);
      return true;
    } catch (err) {
      console.info('saveCurrentBoard failed, rolling back:', err);
      return false;
    }
  }, [saveCurrentBoard]);

  const handleReorderColumns = async (sourceIndex: number, destinationIndex: number) => {
    if (sourceIndex === destinationIndex) return;
    if (columns.length <= 2) return;

    const prev = columns;
    const sourceCol = prev[sourceIndex];
    if (!sourceCol) return;
    // Закреплены только intake/terminal по id/флагу/индексу.
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
    // Назначение в хвост закреплённых — зажать перед хвостом.
    const terminalIdx = prev.findIndex((c) => Boolean(c.terminal));
    const readyIdx = prev.findIndex((c) => c.id === 'ready');
    const tailStart =
      terminalIdx !== -1 ? terminalIdx : readyIdx !== -1 ? readyIdx : prev.length - 1;
    const maxDest = Math.max(1, tailStart - 1);
    const safeDest = Math.max(1, Math.min(destinationIndex, maxDest));
    reordered.splice(safeDest, 0, moved);

    // Intake строго первая, terminal строго последняя, остальные — в drop-порядке.
    const intake = reordered.find((c) => c.isIntake || c.id === 'backlog') ?? prev[0];
    const terminal = reordered.slice().reverse().find((c) => c.terminal || c.id === 'ready') ?? prev[prev.length - 1];
    const middle = reordered.filter((c) => c.id !== intake.id && c.id !== terminal.id);
    const next = [intake, ...middle, terminal];

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
      // Закреплены только intake/terminal по id/флагу/позиции.
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
      // Intake строго первый.
      if (destination.index === 0) return;

      // Хвост закреплённых: первая terminal-колонка, иначе ready, иначе конец.
      const terminalIdx = columns.findIndex((c) => Boolean(c.terminal));
      const readyIdx = columns.findIndex((c) => c.id === 'ready');
      const tailStart =
        terminalIdx !== -1 ? terminalIdx : readyIdx !== -1 ? readyIdx : columns.length - 1;
      // Источник в хвосте быть не должен (закреплён выше), назначение в хвост — зажать.
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
        if (t.columnId === targetColId) return true;
        if (isFirst) {
          const colExists = columns.some((c) => c.id === t.columnId);
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
        // Done column tasks are strictly sorted by completion date (newest first).
        // Manual reordering within Done column is a no-op to preserve chronological ordering.
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

      // Backend-authoritative reordering: persist new index to SQLite position
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

    // Moving between columns with target placement:
    // @hello-pangea/dnd invariant: onDragEnd must synchronously place the card into the destination
    // column state to prevent drop animation glitches and disappearing DOM elements.
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

    const completedAt = isDone ? (movedTask.completedAt || Date.now()) : undefined;
    const totalElapsed = isDone && movedTask.createdAt
      ? Math.max(0, Math.floor((completedAt! - movedTask.createdAt) / 1000))
      : movedTask.totalElapsed;

    const updatedMovedTask: KanbanTask = {
      ...movedTask,
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

    // Synchronous state update guarantees smooth DnD transition
    setTasks(nextTasks);

    // Backend-authoritative confirmation: reorder_task moves stage and updates SQLite position
    try {
      await reorderTask(draggableId, destColId, destIndex);
    } catch (err: unknown) {
      console.warn(`[handleDragEnd] reorderTask rejected for ${draggableId}, reverting state:`, err);
      setTasks(tasks);
      const msg = err instanceof Error ? err.message : String(err);
      alert(t('board.errMoveTask', { msg }));
    }
  };

  const handleDeleteColumn = useCallback(async (columnId: string) => {
    const { columns: curCols, tasks: curTasks, setColumns: curSetCols, moveTask: curMoveTask } = useBoardStore.getState();
    const idx = curCols.findIndex((c) => c.id === columnId);
    if (idx === -1) return;
    const col = curCols[idx];
    if (col.isIntake || col.terminal || col.id === 'backlog' || col.id === 'ready' || idx === 0 || idx === curCols.length - 1) return;
    if (curCols.length <= 1) return;
    const neighbor = curCols[idx - 1] ?? curCols[idx + 1] ?? null;
    if (!neighbor) return;

    const doomedTasks = curTasks.filter((t) => t.columnId === columnId);
    const confirmed = window.confirm(
      doomedTasks.length === 0
        ? t('board.confirmDeleteColumnEmpty', { title: col.title })
        : t('board.confirmDeleteColumnWithTasks', {
            title: col.title,
            count: doomedTasks.length,
            neighbor: neighbor.title,
          })
    );
    if (!confirmed) return;

    const prev = curCols;
    const next = prev.filter((c) => c.id !== columnId);

    curSetCols(next);
    const saved = await savePipelineConfig(next);
    if (!saved) {
      curSetCols(prev);
      setColumnsError(t('board.errDeleteColumnRollback'));
      return;
    }
    for (const t of doomedTasks) {
      await curMoveTask(t.id, neighbor.id);
    }
  }, [savePipelineConfig, t]);

  // Filter tasks based on activeBoardId and searchQuery:
  // ZERO INVISIBLE TASKS (§ 2.18 & § 2.3): Every task must be visible on the board it belongs to,
  // or on the active board if orphaned, if its declared board is absent, or if its column belongs to the active board.
  const currentBoardTasks = tasks.filter((task) => {
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
    const boardHasCol = matchingBoard.columns.some(
      (c) => c.id === task.columnId || c.id.toLowerCase() === task.columnId?.toLowerCase()
    );
    const activeHasCol = columns.some(
      (c) => c.id === task.columnId || c.id.toLowerCase() === task.columnId?.toLowerCase()
    );
    if (!boardHasCol && activeHasCol) {
      return true;
    }
    return false;
  });

  const filteredTasks = currentBoardTasks.filter((task) => {
    if (!searchQuery) return true;
    const q = searchQuery.toLowerCase();
    return (
      task.id.toLowerCase().includes(q) ||
      task.title.toLowerCase().includes(q) ||
      (task.branch ? task.branch.toLowerCase().includes(q) : false) ||
      (task.description && task.description.toLowerCase().includes(q))
    );
  });

  const doneParentIds = useMemo(() => {
    const set = new Set<string>();
    for (const t of tasks) {
      if (t.status === 'done' || t.columnId === 'ready') {
        set.add(t.id);
      }
    }
    return set;
  }, [tasks]);

  const handleBoardWheel = useCallback((e: React.WheelEvent<HTMLDivElement>) => {
    // Zooming via Ctrl/Meta + wheel is handled globally by useUiZoom
    if (e.ctrlKey || e.metaKey) {
      return;
    }

    // Horizontal gestures (trackpad, tilt-wheel, or browser-remapped Shift+wheel)
    // are handled natively by the container's overflow-x-auto
    if (Math.abs(e.deltaX) > 0) {
      return;
    }

    // Explicit horizontal scroll via Shift + Wheel
    if (e.shiftKey && e.deltaY !== 0) {
      e.currentTarget.scrollLeft += e.deltaY;
      return;
    }

    // Ignore events originating inside any Kanban column (defense-in-depth)
    const target = e.target as HTMLElement | null;
    if (target?.closest('[data-kanban-column]')) {
      return;
    }

    // Scroll board horizontally only when scrolling over free canvas / board background
    if (e.deltaY !== 0) {
      e.currentTarget.scrollLeft += e.deltaY;
    }
  }, []);

  const rootTasksCount = currentBoardTasks.filter((t) => !t.parentId).length;

  return (
    <div className="flex-1 flex flex-col h-full overflow-hidden bg-[#fafafa] dark:bg-[#090a0f] transition-colors">
      <BoardTabsHeader
        tasksCount={rootTasksCount}
        onOpenAddColumn={() => setIsAddColumnOpen(true)}
        onOpenAddTask={() => setIsNewTaskModalOpen(true)}
      />

      {columnsError && (
        <div className="mx-3.5 mt-2 flex items-center justify-between gap-2 rounded-md border border-[#fecaca] dark:border-[#3b1219] bg-[#fef2f2] dark:bg-[#161b22] px-3 py-1.5 text-xs font-sans text-[#ef4444]">
          <span>{columnsError}</span>
          <button
            type="button"
            onClick={() => setColumnsError(null)}
            className="p-1 rounded-md hover:bg-red-100 dark:hover:bg-red-950/50 transition cursor-pointer"
            aria-label={t('common.close')}
          >
            <X className="h-3.5 w-3.5" />
          </button>
        </div>
      )}

      {/* Kanban Columns Horizontal Container with Mouse Wheel Scroll */}
      <DragDropContext onDragEnd={handleDragEnd}>
        <Droppable droppableId="board-columns" direction="horizontal" type="COLUMN">
          {(provided) => (
            <div
              ref={provided.innerRef}
              {...provided.droppableProps}
              onWheel={handleBoardWheel}
              className="flex-1 overflow-x-auto overflow-y-hidden p-3.5 relative"
            >
              <div className="flex items-start space-x-3.5 h-full min-w-max pb-1">
                {columns.map((column, index) => {
                  const firstColId = columns.find((c) => c.isIntake)?.id ?? columns[0]?.id ?? 'backlog';
                  const isFirstColumn = column.id === firstColId;
                  const columnTasks = filteredTasks.filter((t) => {
                    if (t.columnId === column.id) return true;
                    if (isFirstColumn) {
                      const colExists = columns.some((c) => c.id === t.columnId);
                      return !colExists;
                    }
                    return false;
                  });
                  const isPinned =
                    column.isIntake ||
                    Boolean(column.terminal) ||
                    column.id === 'backlog' ||
                    column.id === 'ready' ||
                    index === 0 ||
                    index === columns.length - 1;

                  return (
                    <KanbanColumn
                      key={`${activeBoardId}-${column.id}`}
                      column={column}
                      tasks={columnTasks}
                      index={index}
                      totalColumns={columns.length}
                      isPinned={isPinned}
                      doneParentIds={doneParentIds}
                      onDeleteColumn={columns.length > 1 && !isPinned ? handleDeleteColumn : undefined}
                    />
                  );
                })}
                {provided.placeholder}
              </div>
            </div>
          )}
        </Droppable>
      </DragDropContext>

      <CreateTaskModal
        isOpen={isNewTaskModalOpen}
        onClose={() => setIsNewTaskModalOpen(false)}
        columns={columns}
      />

      <CreateColumnModal
        isOpen={isAddColumnOpen}
        onClose={() => setIsAddColumnOpen(false)}
        columns={columns}
      />

      <CreateBoardModal
        isOpen={isCreateBoardModalOpen}
        onClose={() => setIsCreateBoardModalOpen(false)}
      />
    </div>
  );
}
