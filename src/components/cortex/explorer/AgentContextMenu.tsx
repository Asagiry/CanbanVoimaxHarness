import { useEffect } from 'react';
import { Pencil, Trash2, RotateCcw, Palette } from 'lucide-react';
import { useCortexStore } from '../../../stores/cortexStore';
import type { AgentPersonaInfo } from '../../../stores/cortexStore';
import { useTranslation } from '../../../stores/languageStore';

export interface AgentContextMenuState {
  x: number;
  y: number;
  role: string;
  isBuiltin: boolean;
  persona: AgentPersonaInfo;
}

export interface AgentContextMenuProps {
  menu: AgentContextMenuState;
  onClose: () => void;
  onEditPersona: (persona: AgentPersonaInfo) => void;
  onDeleteAgent: (role: string) => void;
}

/**
 * Right-click context menu for agent personas in CortexExplorer.
 */
export function AgentContextMenu({
  menu,
  onClose,
  onEditPersona,
  onDeleteAgent,
}: AgentContextMenuProps) {
  const { t } = useTranslation();
  const pendingDeletes = useCortexStore((s) => s.pendingDeletes);
  const restorePendingDelete = useCortexStore((s) => s.restorePendingDelete);

  useEffect(() => {
    const handleClose = () => onClose();
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('click', handleClose);
    window.addEventListener('contextmenu', handleClose);
    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('click', handleClose);
      window.removeEventListener('contextmenu', handleClose);
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [onClose]);

  return (
    <div
      className="fixed z-50 min-w-[190px] bg-white dark:bg-[#17171b] rounded-lg shadow-md border border-[#ebebeb] dark:border-[#2c2c33] p-1 text-xs font-sans animate-in fade-in zoom-in-95 duration-100"
      style={{
        top: Math.min(menu.y, window.innerHeight - 120),
        left: Math.min(menu.x, window.innerWidth - 210),
      }}
      onClick={(e) => e.stopPropagation()}
      onContextMenu={(e) => e.preventDefault()}
    >
      <div className="px-3 py-1.5 text-[10px] font-semibold text-[#a1a1aa] dark:text-[#71717a] uppercase tracking-wider border-b border-[#f4f4f5] dark:border-[#212126] mb-1 truncate">
        {t('cortex.agentContextMenu', { role: menu.role })}
      </div>
      {pendingDeletes[`agent:${menu.role}`] && (
        <button
          onClick={() => {
            const role = menu.role;
            onClose();
            restorePendingDelete(`agent:${role}`);
          }}
          className="w-full flex items-center space-x-2 px-3 py-1.5 text-left transition rounded-sm text-[#171717] dark:text-[#f4f4f5] hover:bg-[#f4f4f5] dark:hover:bg-[#232328] cursor-pointer"
        >
          <RotateCcw className="h-3.5 w-3.5 text-[#71717a] dark:text-[#71717a]" />
          <span>{t('cortex.restoreDeleted')}</span>
        </button>
      )}
      <button
        onClick={() => {
          const persona = menu.persona;
          onClose();
          onEditPersona(persona);
        }}
        className="w-full flex items-center space-x-2 px-3 py-1.5 text-left transition rounded-sm text-[#171717] dark:text-[#f4f4f5] hover:bg-[#f4f4f5] dark:hover:bg-[#232328] cursor-pointer"
      >
        <Palette className="h-3.5 w-3.5 text-[#71717a] dark:text-[#71717a]" />
        <span>{t('cortex.configureColorIcon')}</span>
      </button>
      <button
        onClick={() => {
          const persona = menu.persona;
          onClose();
          onEditPersona(persona);
        }}
        disabled={menu.isBuiltin}
        className={`w-full flex items-center space-x-2 px-3 py-1.5 text-left transition rounded-sm ${
          menu.isBuiltin
            ? 'text-[#a1a1aa] dark:text-[#52525b] cursor-not-allowed opacity-50'
            : 'text-[#171717] dark:text-[#f4f4f5] hover:bg-[#f4f4f5] dark:hover:bg-[#232328] cursor-pointer'
        }`}
        aria-label={menu.isBuiltin ? t('cortex.cannotRenameBuiltin') : undefined}
      >
        <Pencil className="h-3.5 w-3.5 text-[#71717a] dark:text-[#71717a]" />
        <span>{t('cortex.renameAgent')}</span>
      </button>
      <button
        onClick={() => {
          const role = menu.role;
          onClose();
          if (menu.isBuiltin) return;
          onDeleteAgent(role);
        }}
        disabled={menu.isBuiltin}
        className={`w-full flex items-center space-x-2 px-3 py-1.5 text-left transition rounded-sm ${
          menu.isBuiltin
            ? 'text-[#a1a1aa] dark:text-[#52525b] cursor-not-allowed opacity-50'
            : 'text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-950/30 hover:text-red-700 dark:hover:text-red-300 cursor-pointer'
        }`}
        aria-label={menu.isBuiltin ? t('cortex.cannotDeleteBuiltin') : undefined}
      >
        <Trash2 className="h-3.5 w-3.5" />
        <span>{t('cortex.deleteAgent')}</span>
      </button>
    </div>
  );
}
