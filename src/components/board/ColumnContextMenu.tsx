import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import type { PipelineColumn, PipelineConfig } from '../../types';
import { useBoardStore } from '../../stores/boardStore';
import { useCortexStore } from '../../stores/cortexStore';
import { slugifyTitle } from '../../lib/slug';
import { getDefaultColumnIcon } from '../../lib/columnIcons';
import { getDeterministicAgentColor, isAllowedAgentColor } from '../../lib/agentPalette';

interface ColumnContextMenuProps {
  column: PipelineColumn;
  x: number;
  y: number;
  onClose: () => void;
  onDeleteColumn?: (columnId: string) => void;
}

interface AgentPersonaOption {
  role: string;
  hasPrompt?: boolean;
  has_prompt?: boolean;
  isBuiltin?: boolean;
  is_builtin?: boolean;
  icon?: string;
  color?: string;
}

const MENU_WIDTH = 264;
const VIEWPORT_MARGIN = 8;
const NEW_AGENT_OPTION = '__new__';

// backlog и ready — особые системные колонки: удалить их нельзя.
const PROTECTED_COLUMN_IDS = new Set(['backlog', 'ready']);

export default function ColumnContextMenu({ column, x, y, onClose, onDeleteColumn }: ColumnContextMenuProps) {
  const menuRef = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ left: x, top: y });
  const [mode, setMode] = useState<'main' | 'rename' | 'persona' | 'script'>('main');

  const { columns, setColumns, updateColumn } = useBoardStore();
  const isProtected = PROTECTED_COLUMN_IDS.has(column.id);

  // Переименование
  const [draftTitle, setDraftTitle] = useState(column.title);
  const [renameError, setRenameError] = useState<string | null>(null);
  const [renameSaving, setRenameSaving] = useState(false);

  // Скрипт перехода
  const [draftScript, setDraftScript] = useState(column.onEnterScript ?? '');
  const [scriptError, setScriptError] = useState<string | null>(null);
  const [scriptSaving, setScriptSaving] = useState(false);

  // Смена агента
  const [personas, setPersonas] = useState<AgentPersonaOption[]>([]);
  const [usePersonaSelect, setUsePersonaSelect] = useState(false);
  const [personasLoading, setPersonasLoading] = useState(false);
  const [selectedPersona, setSelectedPersona] = useState(column.persona ?? '');
  const [isNewAgentSelected, setIsNewAgentSelected] = useState(false);
  const [newAgentName, setNewAgentName] = useState('');
  const [personaError, setPersonaError] = useState<string | null>(null);
  const [personaSaving, setPersonaSaving] = useState(false);

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
  }, [x, y, mode]);

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

  const savePipelineConfig = async (next: PipelineColumn[]): Promise<boolean> => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      let cfg: PipelineConfig | null = null;
      try {
        cfg = await invoke<PipelineConfig>('get_pipeline');
      } catch {
        cfg = null;
      }
      const pipeline: PipelineConfig = {
        id: cfg?.id ?? 'default-engineering',
        title: cfg?.title ?? 'Инженерный пайплайн',
        columns: next,
      };
      await invoke('save_pipeline', { pipeline });
      return true;
    } catch (err) {
      console.info('save_pipeline недоступен, изменения колонок откачены:', err);
      return false;
    }
  };

  const handleSaveRename = async () => {
    const title = draftTitle.trim();
    if (!title) return;
    if (title === column.title) {
      setMode('main');
      return;
    }
    setRenameError(null);
    setRenameSaving(true);
    const prev = columns;
    const next = prev.map((c) => (c.id === column.id ? { ...c, title } : c));
    setColumns(next);
    const saved = await savePipelineConfig(next);
    setRenameSaving(false);
    if (!saved) {
      setColumns(prev);
      setRenameError('Не удалось сохранить. Изменения откачены.');
      return;
    }
    onClose();
  };

  const loadPersonas = async () => {
    setPersonasLoading(true);
    setPersonaError(null);
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const list = await invoke<AgentPersonaOption[]>('list_agent_personas');
      if (Array.isArray(list)) {
        setPersonas(list);
        setUsePersonaSelect(true);
      } else {
        setUsePersonaSelect(false);
      }
    } catch {
      // IPC недоступен — откат на свободный текстовый ввод.
      setUsePersonaSelect(false);
    } finally {
      setPersonasLoading(false);
    }
  };

  useEffect(() => {
    if (mode === 'persona') {
      setSelectedPersona(column.persona ?? '');
      setIsNewAgentSelected(false);
      setNewAgentName('');
      setPersonaError(null);
      void loadPersonas();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [mode]);

  const handleSavePersona = async () => {
    setPersonaError(null);
    let personaName: string;
    if (usePersonaSelect && isNewAgentSelected) {
      // Кириллица преобразуется в латиницу автоматически (бэкенд принимает только [a-z0-9-]).
      const role = slugifyTitle(newAgentName);
      if (!role || role === 'column') {
        setPersonaError('Имя должно содержать буквы или цифры.');
        return;
      }
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('create_agent_persona', { role });
        await useCortexStore.getState().loadAgentPersonas();
      } catch (err) {
        setPersonaError(err instanceof Error ? err.message : String(err));
        return;
      }
      personaName = role;
    } else {
      personaName = selectedPersona.trim();
    }

    const nextPersona = personaName || null;
    if (nextPersona === (column.persona ?? null)) {
      setMode('main');
      return;
    }

    const matchedPersona = personas.find((p) => p.role === nextPersona);
    let nextIcon = column.icon;
    if (nextPersona && matchedPersona?.icon) {
      nextIcon = matchedPersona.icon;
    } else if (!nextPersona) {
      nextIcon = getDefaultColumnIcon(column.id, column.icon);
    }

    // Автоматическое наследование цвета агента колонкой (если не backlog и не ready)
    let nextColor = column.color;
    if (column.id !== 'backlog' && column.id !== 'ready') {
      if (nextPersona) {
        const personaColor = matchedPersona?.color || getDeterministicAgentColor(nextPersona);
        if (personaColor && isAllowedAgentColor(personaColor)) {
          nextColor = personaColor;
        }
      }
    }

    setPersonaSaving(true);
    const prev = columns;
    const next = prev.map((c) =>
      c.id === column.id ? { ...c, persona: nextPersona, icon: nextIcon, color: nextColor } : c,
    );
    setColumns(next);
    useCortexStore.getState().syncStagesFromColumns(next);
    const saved = await savePipelineConfig(next);
    setPersonaSaving(false);
    if (!saved) {
      setColumns(prev);
      useCortexStore.getState().syncStagesFromColumns(prev);
      setPersonaError('Не удалось сохранить. Изменения откачены.');
      return;
    }
    onClose();
  };

  const handleSaveScript = async () => {
    const trimmed = draftScript.trim();
    const nextScript = trimmed.length > 0 ? trimmed : undefined;
    if (nextScript === (column.onEnterScript ?? undefined)) {
      setMode('main');
      return;
    }

    setScriptError(null);
    setScriptSaving(true);
    const prev = columns;
    const next = prev.map((c) => (c.id === column.id ? { ...c, onEnterScript: nextScript } : c));
    setColumns(next);
    if (updateColumn) {
      await updateColumn(column.id, { onEnterScript: nextScript });
    }
    const saved = await savePipelineConfig(next);
    setScriptSaving(false);
    if (!saved) {
      setColumns(prev);
      setScriptError('Не удалось сохранить. Изменения откачены.');
      return;
    }
    onClose();
  };

  const handleDelete = () => {
    onClose();
    onDeleteColumn?.(column.id);
  };

  const selectedKnown = personas.some((p) => p.role === selectedPersona);

  const menu = (
    <div
      ref={menuRef}
      role="menu"
      className="fixed z-50 rounded-md border border-[#ebebeb] dark:border-[#30363d] bg-white dark:bg-[#161b22] py-1 font-sans text-xs text-[#171717] dark:text-[#f0f6fc] shadow-md transition-colors"
      style={{ left: pos.left, top: pos.top, width: MENU_WIDTH }}
      onClick={(e) => e.stopPropagation()}
      onContextMenu={(e) => e.preventDefault()}
    >
      {mode === 'main' && (
        <>
          <button
            type="button"
            role="menuitem"
            onClick={() => {
              setDraftTitle(column.title);
              setRenameError(null);
              setMode('rename');
            }}
            className="w-full px-3 py-1.5 text-left hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] cursor-pointer"
          >
            Переименовать
          </button>
          <button
            type="button"
            role="menuitem"
            onClick={() => setMode('persona')}
            className="w-full px-3 py-1.5 text-left hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] cursor-pointer"
          >
            Сменить агента
          </button>
          <button
            type="button"
            role="menuitem"
            onClick={() => {
              setDraftScript(column.onEnterScript ?? '');
              setScriptError(null);
              setMode('script');
            }}
            className="w-full px-3 py-1.5 text-left hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] cursor-pointer"
          >
            Скрипт перехода (.bat, .ps1, .py, .sh)
          </button>
          <div className="my-1 border-t border-[#ebebeb] dark:border-[#21262d]" />
          <button
            type="button"
            role="menuitem"
            disabled={isProtected}
            title={isProtected ? 'Особые колонки удалить нельзя' : undefined}
            onClick={handleDelete}
            className={`w-full px-3 py-1.5 text-left ${
              isProtected
                ? 'text-[#8f8f8f] dark:text-[#8b949e] cursor-not-allowed'
                : 'text-[#b91c1c] dark:text-[#f87171] hover:bg-[#fef2f2] dark:hover:bg-[#450a0a]/40 cursor-pointer'
            }`}
          >
            Удалить
          </button>
        </>
      )}

      {mode === 'rename' && (
        <div className="px-3 py-2">
          <div className="text-[#4d4d4d] dark:text-[#c9d1d9] font-medium mb-1">Новое название</div>
          <input
            type="text"
            autoFocus
            value={draftTitle}
            onChange={(e) => setDraftTitle(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') {
                e.preventDefault();
                void handleSaveRename();
              }
            }}
            className="w-full px-2.5 py-1.5 rounded-md bg-[#fafafa] dark:bg-[#0d1117] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc] focus:bg-white dark:focus:bg-[#161b22] transition"
          />
          {renameError && (
            <div className="mt-2 text-xs text-red-700 bg-red-50 border border-red-200 rounded-md px-2.5 py-1.5">
              {renameError}
            </div>
          )}
          <div className="mt-2 flex items-center justify-end space-x-2">
            <button
              type="button"
              onClick={() => setMode('main')}
              className="px-2.5 py-1.5 rounded-md bg-white dark:bg-[#0d1117] border border-[#ebebeb] dark:border-[#30363d] hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] transition cursor-pointer text-[#171717] dark:text-[#f0f6fc]"
            >
              Отмена
            </button>
            <button
              type="button"
              disabled={!draftTitle.trim() || renameSaving}
              onClick={() => void handleSaveRename()}
              className="px-3 py-1.5 rounded-md bg-[#171717] dark:bg-[#21262d] hover:bg-[#262626] dark:hover:bg-[#30363d] text-white font-medium transition cursor-pointer disabled:opacity-50 disabled:cursor-default border border-transparent dark:border-[#30363d]"
            >
              Сохранить
            </button>
          </div>
        </div>
      )}

      {mode === 'persona' && (
        <div className="px-3 py-2">
          <div className="text-[#4d4d4d] dark:text-[#c9d1d9] font-medium mb-1">Агент колонки</div>
          <div className="mb-1.5 text-[#8f8f8f] dark:text-[#8b949e]">Текущий: {column.persona ?? 'не назначен'}</div>
          {usePersonaSelect ? (
            <select
              value={isNewAgentSelected ? NEW_AGENT_OPTION : selectedPersona}
              onChange={(e) => {
                if (e.target.value === NEW_AGENT_OPTION) {
                  setIsNewAgentSelected(true);
                } else {
                  setIsNewAgentSelected(false);
                  setSelectedPersona(e.target.value);
                }
              }}
              disabled={personasLoading || personaSaving}
              className="w-full px-2.5 py-1.5 rounded-md bg-[#fafafa] dark:bg-[#0d1117] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc] focus:bg-white dark:focus:bg-[#161b22] font-mono transition cursor-pointer"
            >
              {selectedPersona && !selectedKnown && (
                <option value={selectedPersona}>{selectedPersona}</option>
              )}
              {personas.map((p) => {
                const isBuiltin = p.role === 'cvh-dev' || Boolean(p.isBuiltin || p.is_builtin);
                const iconLabel = p.icon ? ` [${p.icon}]` : '';
                return (
                  <option key={p.role} value={p.role}>
                    {isBuiltin ? `${p.role} (Встроенный)${iconLabel}` : `${p.role}${iconLabel}`}
                  </option>
                );
              })}
              <option value={NEW_AGENT_OPTION}>+ Новый агент</option>
            </select>
          ) : (
            <input
              type="text"
              autoFocus
              value={selectedPersona}
              onChange={(e) => setSelectedPersona(e.target.value)}
              placeholder={personasLoading ? 'Загрузка агентов...' : 'developer'}
              className="w-full px-2.5 py-1.5 rounded-md bg-[#fafafa] dark:bg-[#0d1117] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] placeholder:text-[#8f8f8f] dark:placeholder:text-[#6e7681] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc] focus:bg-white dark:focus:bg-[#161b22] font-mono transition"
            />
          )}
          {usePersonaSelect && isNewAgentSelected && (
            <>
              <input
                type="text"
                autoFocus
                value={newAgentName}
                onChange={(e) => setNewAgentName(e.target.value)}
                placeholder="Имя нового агента"
                className="mt-1.5 w-full px-2.5 py-1.5 rounded-md bg-[#fafafa] dark:bg-[#0d1117] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] placeholder:text-[#8f8f8f] dark:placeholder:text-[#6e7681] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc] focus:bg-white dark:focus:bg-[#161b22] font-mono transition"
              />
              {newAgentName.trim() !== '' ? (
                <div className="mt-1 text-[10px] font-mono text-[#8f8f8f]">
                  Роль:{' '}
                  {slugifyTitle(newAgentName) === 'column'
                    ? '— нужны буквы или цифры —'
                    : slugifyTitle(newAgentName)}
                </div>
              ) : (
                <div className="mt-1 text-[10px] text-[#8f8f8f]">
                  Кириллица преобразуется в латиницу автоматически
                </div>
              )}
            </>
          )}
          {personaError && (
            <div className="mt-2 text-xs text-red-700 bg-red-50 border border-red-200 rounded-md px-2.5 py-1.5">
              {personaError}
            </div>
          )}
          <div className="mt-2 flex items-center justify-end space-x-2">
            <button
              type="button"
              onClick={() => setMode('main')}
              className="px-2.5 py-1.5 rounded-md bg-white dark:bg-[#0d1117] border border-[#ebebeb] dark:border-[#30363d] hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] text-[#171717] dark:text-[#f0f6fc] transition cursor-pointer"
            >
              Назад
            </button>
            <button
              type="button"
              disabled={personaSaving || personasLoading}
              onClick={() => void handleSavePersona()}
              className="px-3 py-1.5 rounded-md bg-[#171717] dark:bg-[#21262d] hover:bg-[#262626] dark:hover:bg-[#30363d] text-white font-medium transition cursor-pointer disabled:opacity-50 disabled:cursor-default border border-transparent dark:border-[#30363d]"
            >
              Сохранить
            </button>
          </div>
        </div>
      )}

      {mode === 'script' && (
        <div className="px-3 py-2">
          <div className="text-[#4d4d4d] dark:text-[#c9d1d9] font-medium mb-1">Скрипт перехода</div>
          <div
            className="mb-1.5 text-[#8f8f8f] dark:text-[#8b949e] truncate font-mono text-[10px]"
            title={column.onEnterScript ?? 'не задан'}
          >
            Текущий: {column.onEnterScript ?? 'не задан'}
          </div>
          <input
            type="text"
            autoFocus
            value={draftScript}
            onChange={(e) => setDraftScript(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') {
                e.preventDefault();
                void handleSaveScript();
              }
            }}
            placeholder="scripts/setup.bat, .py, .ps1, .sh"
            className="w-full px-2.5 py-1.5 rounded-md bg-[#fafafa] dark:bg-[#0d1117] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] placeholder:text-[#8f8f8f] dark:placeholder:text-[#6e7681] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc] focus:bg-white dark:focus:bg-[#161b22] font-mono text-xs transition"
          />
          <div className="mt-1 text-[10px] text-[#8f8f8f] dark:text-[#8b949e]">
            Запускается при входе задачи (.bat, .ps1, .py, .sh)
          </div>
          {scriptError && (
            <div className="mt-2 text-xs text-red-700 bg-red-50 border border-red-200 rounded-md px-2.5 py-1.5">
              {scriptError}
            </div>
          )}
          <div className="mt-2 flex items-center justify-end space-x-2">
            <button
              type="button"
              onClick={() => setMode('main')}
              className="px-2.5 py-1.5 rounded-md bg-white dark:bg-[#0d1117] border border-[#ebebeb] dark:border-[#30363d] hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] text-[#171717] dark:text-[#f0f6fc] transition cursor-pointer"
            >
              Назад
            </button>
            <button
              type="button"
              disabled={scriptSaving}
              onClick={() => void handleSaveScript()}
              className="px-3 py-1.5 rounded-md bg-[#171717] dark:bg-[#21262d] hover:bg-[#262626] dark:hover:bg-[#30363d] text-white font-medium transition cursor-pointer disabled:opacity-50 disabled:cursor-default border border-transparent dark:border-[#30363d]"
            >
              Сохранить
            </button>
          </div>
        </div>
      )}
    </div>
  );

  return createPortal(menu, document.body);
}
