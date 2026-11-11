import { useRef, useState, memo } from 'react';
import { useContextMenuPosition } from '../../hooks/useContextMenuPosition';
import { createPortal } from 'react-dom';
import type { KanbanTask } from '../../types';
import { useBoardStore } from '../../stores/boardStore';
import { useDrawerStore } from '../../stores/drawerStore';
import { useTranslation } from '../../stores/languageStore';
import { useShallow } from 'zustand/react/shallow';

export interface CardContextMenuProps {
  task: KanbanTask;
  x: number;
  y: number;
  onClose: () => void;
}

const MENU_WIDTH = 224;
const VIEWPORT_MARGIN = 8;

export const CardContextMenu = memo(function CardContextMenu({
  task,
  x,
  y,
  onClose,
}: CardContextMenuProps) {
  const menuRef = useRef<HTMLDivElement>(null);
  const pos = useContextMenuPosition({ x, y, menuRef, onClose, defaultWidth: MENU_WIDTH, viewportMargin: VIEWPORT_MARGIN });
  const columns = useBoardStore(useShallow((s) => s.columns));
  const openDrawer = useDrawerStore((s) => s.openDrawer);
  const { t } = useTranslation();
  const [moveError, setMoveError] = useState<string | null>(null);

  const handleOpen = () => {
    openDrawer(task.id);
    onClose();
  };

  const handleRunAgent = async () => {
    onClose();
    await useBoardStore.getState().runTaskAgent(task.id);
  };

  const handleCreateSubtask = async () => {
    onClose();
    await useBoardStore.getState().createSubtask(task.id);
  };

  const handleMove = async (targetStageId: string) => {
    if (targetStageId === task.stageId) {
      onClose();
      return;
    }
    setMoveError(null);
    try {
      await useBoardStore.getState().moveTask(task.id, targetStageId);
      onClose();
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      console.warn('[CardContextMenu] Failed to move task:', err);
      setMoveError(t('taskMenu.moveError', { msg }));
    }
  };

  const handleDelete = async () => {
    const confirmed = window.confirm(t('taskMenu.confirmDelete', { title: task.title }));
    onClose();
    if (!confirmed) return;
    await useBoardStore.getState().deleteTask(task.id);
  };

  const menu = (
    <div
      ref={menuRef}
      role="menu"
      className="fixed z-50 w-56 rounded-md border border-[#ebebeb] dark:border-[#2c2c33] bg-white dark:bg-[#17171b] py-1 font-mono text-xs text-[#171717] dark:text-[#f4f4f5] shadow-md transition-colors"
      style={{ left: pos.left, top: pos.top }}
      onClick={(e) => e.stopPropagation()}
      onContextMenu={(e) => e.preventDefault()}
    >
      <button
        type="button"
        role="menuitem"
        onClick={handleOpen}
        className="w-full px-3 py-1.5 text-left hover:bg-[#f4f4f5] dark:hover:bg-[#232328] cursor-pointer"
      >
        {t('taskMenu.open')}
      </button>

      <button
        type="button"
        role="menuitem"
        onClick={handleRunAgent}
        className="w-full px-3 py-1.5 text-left hover:bg-[#f4f4f5] dark:hover:bg-[#232328] cursor-pointer"
      >
        {t('taskMenu.runAgent')}
      </button>

      <button
        type="button"
        role="menuitem"
        onClick={handleCreateSubtask}
        className="w-full px-3 py-1.5 text-left hover:bg-[#f4f4f5] dark:hover:bg-[#232328] cursor-pointer text-[#171717] dark:text-[#f4f4f5]"
      >
        {t('taskMenu.createSubtask')}
      </button>

      <div className="my-1 border-t border-[#ebebeb] dark:border-[#212126]" />

      <div className="px-3 py-1 text-[#8f8f8f] dark:text-[#71717a] select-none">{t('taskMenu.moveTo')}</div>
      {columns.map((col) => {
        const isCurrent = col.id === task.stageId;
        return (
          <button
            key={col.id}
            type="button"
            role="menuitem"
            disabled={isCurrent}
            onClick={() => handleMove(col.id)}
            className={`w-full px-3 py-1.5 text-left ${
              isCurrent
                ? 'text-[#8f8f8f] dark:text-[#71717a] cursor-default'
                : 'hover:bg-[#f4f4f5] dark:hover:bg-[#232328] cursor-pointer'
            }`}
          >
            {isCurrent ? t('taskMenu.current', { title: col.title }) : col.title}
          </button>
        );
      })}

      <div className="my-1 border-t border-[#ebebeb] dark:border-[#212126]" />

      {moveError && (
        <div
          role="alert"
          className="mx-2 mb-1.5 rounded border border-red-200 dark:border-red-900/50 bg-red-50 dark:bg-red-950/40 px-2 py-1 text-[11px] font-sans text-red-700 dark:text-red-300 break-words"
        >
          {moveError}
        </div>
      )}

      <button
        type="button"
        role="menuitem"
        onClick={() => void handleDelete()}
        className="w-full px-3 py-1.5 text-left text-[#b91c1c] dark:text-[#f87171] hover:bg-[#fef2f2] dark:hover:bg-[#450a0a]/40 cursor-pointer"
      >
        {t('taskMenu.deleteTask')}
      </button>
    </div>
  );

  return createPortal(menu, document.body);
});

export default CardContextMenu;
