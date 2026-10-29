import { useEffect, useLayoutEffect, useRef, useState, memo } from 'react';
import { createPortal } from 'react-dom';
import type { KanbanTask } from '../../types';
import { useBoardStore } from '../../stores/boardStore';
import { useDrawerStore } from '../../stores/drawerStore';
import { useCortexStore } from '../../stores/cortexStore';
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
  const [pos, setPos] = useState({ left: x, top: y });
  const [showRoleSelector, setShowRoleSelector] = useState(false);

  const columns = useBoardStore(useShallow((s) => s.columns));
  const agentPersonas = useCortexStore(useShallow((s) => s.agentPersonas));
  const openDrawer = useDrawerStore((s) => s.openDrawer);
  const { t, language } = useTranslation();

  // Clamp coordinates within viewport
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
  }, [x, y, showRoleSelector]);

  // Close handlers: outside click, Escape, scroll, resize
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
    await useBoardStore.getState().runTaskAgent(task.id);
  };

  const handleCreateSubtask = async () => {
    onClose();
    await useBoardStore.getState().createSubtask(task.id);
  };

  const handleMove = async (targetColumnId: string) => {
    onClose();
    if (targetColumnId === task.columnId) return;
    try {
      await useBoardStore.getState().moveTask(task.id, targetColumnId);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      alert(`Не удалось переместить задачу:\n${msg}`);
    }
  };

  const handleSelectPersona = (role: string) => {
    onClose();
    if (task.persona === role) return;
    useBoardStore.getState().updateTask(task.id, { persona: role });
  };

  const handleDelete = async () => {
    const confirmed = window.confirm(t('taskMenu.confirmDelete', { title: task.title }));
    onClose();
    if (!confirmed) return;
    await useBoardStore.getState().deleteTask(task.id);
  };

  const isRu = language === 'ru';

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
        {t('taskMenu.open')}
      </button>

      <button
        type="button"
        role="menuitem"
        onClick={handleRunAgent}
        className="w-full px-3 py-1.5 text-left hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] cursor-pointer"
      >
        {t('taskMenu.runAgent')}
      </button>

      <button
        type="button"
        role="menuitem"
        onClick={handleCreateSubtask}
        className="w-full px-3 py-1.5 text-left hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] cursor-pointer text-[#171717] dark:text-[#f0f6fc]"
      >
        {t('taskMenu.createSubtask')}
      </button>

      {/* Role / Persona selector */}
      {agentPersonas.length > 0 && (
        <>
          <button
            type="button"
            role="menuitem"
            onClick={() => setShowRoleSelector(!showRoleSelector)}
            className="w-full px-3 py-1.5 text-left hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] cursor-pointer flex items-center justify-between"
          >
            <span>{isRu ? 'Сменить роль' : 'Change Role'}</span>
            <span className="text-[10px] text-[#8f8f8f] dark:text-[#8b949e]">
              {task.persona || 'auto'}
            </span>
          </button>

          {showRoleSelector && (
            <div className="pl-3 pr-1 py-1 max-h-36 overflow-y-auto space-y-0.5 border-l-2 border-purple-500/30 ml-3 my-0.5">
              {agentPersonas.map((p) => {
                const isCurrentRole = task.persona === p.role;
                return (
                  <button
                    key={p.role}
                    type="button"
                    role="menuitem"
                    disabled={isCurrentRole}
                    onClick={() => handleSelectPersona(p.role)}
                    className={`w-full px-2 py-1 text-left rounded text-[11px] truncate flex items-center justify-between ${
                      isCurrentRole
                        ? 'text-[#8f8f8f] dark:text-[#8b949e] font-semibold cursor-default bg-[#f4f4f5]/50 dark:bg-[#21262d]/50'
                        : 'hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] cursor-pointer'
                    }`}
                  >
                    <span className="truncate">{p.role}</span>
                    {isCurrentRole && <span className="text-[9px] text-purple-400">✓</span>}
                  </button>
                );
              })}
            </div>
          )}
        </>
      )}

      <div className="my-1 border-t border-[#ebebeb] dark:border-[#21262d]" />

      <div className="px-3 py-1 text-[#8f8f8f] dark:text-[#8b949e] select-none">{t('taskMenu.moveTo')}</div>
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
            {isCurrent ? t('taskMenu.current', { title: col.title }) : col.title}
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
        {t('taskMenu.deleteTask')}
      </button>
    </div>
  );

  return createPortal(menu, document.body);
});

export default CardContextMenu;
