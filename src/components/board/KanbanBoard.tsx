import React, { useState, useCallback } from 'react';
import { DragDropContext, Droppable } from '@hello-pangea/dnd';
import { useBoardStore } from '../../stores/boardStore';
import { useShallow } from 'zustand/react/shallow';
import KanbanColumn from './KanbanColumn';
import { X } from 'lucide-react';
import BoardTabsHeader from './BoardTabsHeader';
import CreateTaskModal from './modals/CreateTaskModal';
import CreateColumnModal from './modals/CreateColumnModal';
import CreateBoardModal from './CreateBoardModal';
import { useTranslation } from '../../stores/languageStore';
import { useBoardDragDrop } from './useBoardDragDrop';
import { useBoardTaskFilter } from './useBoardTaskFilter';

export { useBoardDragDrop } from './useBoardDragDrop';
export { useBoardTaskFilter } from './useBoardTaskFilter';

export default function KanbanBoard() {
  const columns = useBoardStore(useShallow((s) => s.columns));
  const tasks = useBoardStore(useShallow((s) => s.tasks));
  const searchQuery = useBoardStore((s) => s.searchQuery);
  const boards = useBoardStore(useShallow((s) => s.boards));
  const activeBoardId = useBoardStore((s) => s.activeBoardId);
  const { t } = useTranslation();

  const [isNewTaskModalOpen, setIsNewTaskModalOpen] = useState(false);
  const [isAddColumnOpen, setIsAddColumnOpen] = useState(false);
  const [isCreateBoardModalOpen, setIsCreateBoardModalOpen] = useState(false);

  const {
    currentBoardTasks,
    filteredTasks,
    doneParentIds,
    rootTasksCount,
  } = useBoardTaskFilter({
    tasks,
    columns,
    boards,
    activeBoardId,
    searchQuery,
  });

  const {
    handleDragEnd,
    columnsError,
    setColumnsError,
    savePipelineConfig,
  } = useBoardDragDrop({
    columns,
    tasks,
    activeBoardId,
    searchQuery,
    filteredTasks,
    currentBoardTasks,
  });

  const handleDeleteColumn = useCallback(async (columnId: string) => {
    const { columns: curCols, tasks: curTasks, setColumns: curSetCols, moveTask: curMoveTask } = useBoardStore.getState();
    const idx = curCols.findIndex((c) => c.id === columnId);
    if (idx === -1) return;
    const col = curCols[idx];
    if (col.isIntake || col.terminal || col.id === 'backlog' || col.id === 'ready' || idx === 0 || idx === curCols.length - 1) return;
    if (curCols.length <= 1) return;
    const neighbor = curCols[idx - 1] ?? curCols[idx + 1] ?? null;
    if (!neighbor) return;

    const doomedTasks = curTasks.filter((task) => task.stageId === columnId);
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
    for (const doomedTask of doomedTasks) {
      await curMoveTask(doomedTask.id, neighbor.id);
    }
  }, [savePipelineConfig, setColumnsError, t]);

  const handleBoardWheel = useCallback((e: React.WheelEvent<HTMLDivElement>) => {
    if (e.ctrlKey || e.metaKey) return;
    if (Math.abs(e.deltaX) > 0) return;
    if (e.shiftKey && e.deltaY !== 0) {
      e.currentTarget.scrollLeft += e.deltaY;
      return;
    }
    const target = e.target as HTMLElement | null;
    if (target?.closest('[data-kanban-column]')) return;
    if (e.deltaY !== 0) {
      e.currentTarget.scrollLeft += e.deltaY;
    }
  }, []);

  return (
    <div className="flex-1 flex flex-col h-full overflow-hidden bg-[#fafafa] dark:bg-[#0b0b0e] transition-colors">
      <BoardTabsHeader
        tasksCount={rootTasksCount}
        onOpenAddColumn={() => setIsAddColumnOpen(true)}
        onOpenAddTask={() => setIsNewTaskModalOpen(true)}
      />

      {columnsError && (
        <div className="mx-3.5 mt-2 flex items-center justify-between gap-2 rounded-md border border-[#fecaca] dark:border-[#3b1219] bg-[#fef2f2] dark:bg-[#17171b] px-3 py-1.5 text-xs font-sans text-[#ef4444]">
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
                  const columnTasks = filteredTasks.filter((task) => {
                    if (task.stageId === column.id) return true;
                    if (isFirstColumn) {
                      const colExists = columns.some((c) => c.id === task.stageId);
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
                      stage={column}
                      column={column}
                      tasks={columnTasks}
                      index={index}
                      totalColumns={columns.length}
                      isPinned={isPinned}
                      doneParentIds={doneParentIds}
                      onDeleteStage={columns.length > 1 && !isPinned ? handleDeleteColumn : undefined}
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
