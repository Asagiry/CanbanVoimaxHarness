import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import type { KanbanTask } from '../../types';
import { useBoardStore } from '../../stores/boardStore';
import { useDrawerStore } from '../../stores/drawerStore';

interface TaskContextMenuProps {
  task: KanbanTask;
  x: number;
  y: number;
  onClose: () => void;
}

const MENU_WIDTH = 224;
const VIEWPORT_MARGIN = 8;

export default function TaskContextMenu({ task, x, y, onClose }: TaskContextMenuProps) {
  const menuRef = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ left: x, top: y });

  const { columns, moveTask, deleteTask, runTaskAgent, createSubtask } = useBoardStore();
  const { openDrawer } = useDrawerStore();

  // Поджать координаты, чтобы меню не вылезало за край viewport.
  useLayoutEffect(() => {
    const el = menuRef.current;
    if (!el) {
      setPos({
        left: Math.min(x, Math.max(VIEWPORT_MARGIN, window.innerWidth - MENU_WIDTH - VIEWPORT_MARGIN)),
        top: Math.min(y, Math.max(VIEWPORT_MARGIN, window.innerHeight - VIEWPORT_MARGIN)),
      });
      return;
    }
    const rect = el.getBoundingClientRect();
    let left = x;
    let top = y;
    if (left + rect.width > window.innerWidth - VIEWPORT_MARGIN) {
      left = Math.max(VIEWPORT_MARGIN, window.innerWidth - rect.width - VIEWPORT_MARGIN);
    }
    if (top + rect.height > window.innerHeight - VIEWPORT_MARGIN) {
      top = Math.max(VIEWPORT_MARGIN, window.innerHeight - rect.height - VIEWPORT_MARGIN);
    }
    if (left !== x || top !== y) {
      setPos({ left, top });
    }
  }, [x, y]);

  // Вся логика закрытия внутри: клик вне, Esc, скролл/ресайз.
  useEffect(() => {
    const handlePointerDown = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose();
      }
    };
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      }
    };
    const handleScroll = () => onClose();
    const handleResize = () => onClose();
    const handleContextMenu = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose();
      }
    };

    document.addEventListener('mousedown', handlePointerDown);
    document.addEventListener('keydown', handleKeyDown);
    document.addEventListener('contextmenu', handleContextMenu);
    window.addEventListener('scroll', handleScroll, true);
    window.addEventListener('resize', handleResize);

    return () => {
      document.removeEventListener('mousedown', handlePointerDown);
      document.removeEventListener('keydown', handleKeyDown);
      document.removeEventListener('contextmenu', handleContextMenu);
      window.removeEventListener('scroll', handleScroll, true);
      window.removeEventListener('resize', handleResize);
    };
  }, [onClose]);

  const handleOpen = () => {
    openDrawer(task.id);
    onClose();
  };

  const handleRunAgent = async () => {
    onClose();
    await runTaskAgent(task.id);
  };

  const handleCreateSubtask = async () => {
    onClose();
    await createSubtask(task.id);
  };

  const handleMove = async (targetColumnId: string) => {
    onClose();
    if (targetColumnId === task.columnId) return;
    await moveTask(task.id, targetColumnId);
  };

  const handleDelete = async () => {
    const confirmed = window.confirm(`Удалить задачу "${task.title}"?`);
    onClose();
    if (!confirmed) return;
    await deleteTask(task.id);
  };

  const menu = (
    <div
      ref={menuRef}
      role="menu"
      className="fixed z-50 w-56 rounded-md border border-[#ebebeb] dark:border-[#30363d] bg-white dark:bg-[#161b22] py-1 font-mono text-xs text-[#171717] dark:text-[#f0f6fc] shadow-md transition-colors"
      style={{ left: pos.left, top: pos.top }}
      onClick={(e) => e.stopPropagation()}
      onContextMenu={(e) => e.preventDefault()}
    >
      <button
        type="button"
        role="menuitem"
        onClick={handleOpen}
        className="w-full px-3 py-1.5 text-left hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] cursor-pointer"
      >
        Открыть
      </button>
      <button
        type="button"
        role="menuitem"
        onClick={handleRunAgent}
        className="w-full px-3 py-1.5 text-left hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] cursor-pointer"
      >
        Запустить агента
      </button>
      <button
        type="button"
        role="menuitem"
        onClick={handleCreateSubtask}
        className="w-full px-3 py-1.5 text-left hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] cursor-pointer text-[#171717] dark:text-[#f0f6fc]"
      >
        + Создать подзадачу
      </button>

      <div className="my-1 border-t border-[#ebebeb] dark:border-[#21262d]" />

      <div className="px-3 py-1 text-[#8f8f8f] dark:text-[#8b949e] select-none">Переместить в:</div>
      {columns.map((col) => {
        const isCurrent = col.id === task.columnId;
        return (
          <button
            key={col.id}
            type="button"
            role="menuitem"
            disabled={isCurrent}
            onClick={() => handleMove(col.id)}
            className={`w-full px-3 py-1.5 text-left ${
              isCurrent
                ? 'text-[#8f8f8f] dark:text-[#8b949e] cursor-default'
                : 'hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] cursor-pointer'
            }`}
          >
            {isCurrent ? `${col.title} (текущая)` : col.title}
          </button>
        );
      })}

      <div className="my-1 border-t border-[#ebebeb] dark:border-[#21262d]" />

      <button
        type="button"
        role="menuitem"
        onClick={() => void handleDelete()}
        className="w-full px-3 py-1.5 text-left text-[#b91c1c] dark:text-[#f87171] hover:bg-[#fef2f2] dark:hover:bg-[#450a0a]/40 cursor-pointer"
      >
        Удалить задачу
      </button>
    </div>
  );

  return createPortal(menu, document.body);
}
