import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { ArrowRightToLine, ArrowRightFromLine } from 'lucide-react';
import type { PipelineStage, PipelineConfig, HookDefinition } from '../../types';
import { getHookScript } from '../../types';
import { useBoardStore } from '../../stores/boardStore';
import { useCortexStore } from '../../stores/cortexStore';
import { slugifyTitle } from '../../lib/slug';
import { getDefaultColumnIcon } from '../../lib/columnIcons';
import { getDeterministicAgentColor, isAllowedAgentColor } from '../../lib/agentPalette';
import PersonaSelect from '../common/PersonaSelect';
import { useTranslation } from '../../stores/languageStore';

export interface StageContextMenuProps {
  stage?: PipelineStage;
  /** @deprecated Use stage instead */
  column?: PipelineStage;
  x: number;
  y: number;
  onClose: () => void;
  onDeleteStage?: (stageId: string) => void;
  /** @deprecated Use onDeleteStage instead */
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

// backlog и ready — особые системные стадии: удалить их нельзя.
const PROTECTED_STAGE_IDS = new Set(['backlog', 'ready']);

export default function StageContextMenu({
  stage,
  column,
  x,
  y,
  onClose,
  onDeleteStage,
  onDeleteColumn,
}: StageContextMenuProps) {
  const { stages, setStages, updateStage } = useBoardStore();
  const currentStage = stages.find((s) => s.id === (stage?.id ?? column?.id)) ?? stage ?? column!;
  const menuRef = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ left: x, top: y });
  const [mode, setMode] = useState<'main' | 'rename' | 'persona' | 'script'>('main');
  const { t } = useTranslation();

  const isProtected =
    PROTECTED_STAGE_IDS.has(currentStage.id) ||
    Boolean(currentStage.isIntake) ||
    Boolean(currentStage.terminal);

  const canChangeAgent =
    currentStage.id !== 'backlog' &&
    currentStage.id !== 'ready' &&
    !currentStage.isIntake &&
    !currentStage.terminal;

  const enterHookDef =
    currentStage.hooks?.['on_enter'] ??
    (currentStage.onEnterScript ? { script: currentStage.onEnterScript } : undefined);
  const leaveHookDef =
    currentStage.hooks?.['on_leave'] ??
    currentStage.hooks?.['on_exit'] ??
    currentStage.hooks?.['on_finish'];

  const currentEnterScript = getHookScript(enterHookDef) ?? '';
  const currentLeaveScript = getHookScript(leaveHookDef) ?? '';

  // Переименование
  const [draftTitle, setDraftTitle] = useState(currentStage.title);
  const [renameError, setRenameError] = useState<string | null>(null);
  const [renameSaving, setRenameSaving] = useState(false);

  // Скрипт перехода
  const [scriptType, setScriptType] = useState<'on_enter' | 'on_leave'>('on_enter');
  const [draftScript, setDraftScript] = useState('');
  const [scriptError, setScriptError] = useState<string | null>(null);
  const [scriptSaving, setScriptSaving] = useState(false);

  // Смена агента
  const [personas, setPersonas] = useState<AgentPersonaOption[]>([]);
  const [usePersonaSelect, setUsePersonaSelect] = useState(false);
  const [personasLoading, setPersonasLoading] = useState(false);
  const [selectedPersona, setSelectedPersona] = useState(currentStage.persona ?? '');
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

  const savePipelineConfig = async (next: PipelineStage[]): Promise<boolean> => {
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
        stages: next,
        columns: next,
      };
      await useBoardStore.getState().saveCurrentBoard(next);
      await invoke('save_pipeline', { pipeline });
      return true;
    } catch (err) {
      console.info('save_pipeline недоступен, изменения стадий откачены:', err);
      return false;
    }
  };

  const handleSaveRename = async () => {
    const title = draftTitle.trim();
    if (!title) return;
    if (title === currentStage.title) {
      setMode('main');
      return;
    }
    setRenameError(null);
    setRenameSaving(true);
    const prev = stages;
    const next = prev.map((c) => (c.id === currentStage.id ? { ...c, title } : c));
    setStages(next);
    if (updateStage) {
      await updateStage(currentStage.id, { title });
    }
    const saved = await savePipelineConfig(next);
    setRenameSaving(false);
    if (!saved) {
      setStages(prev);
      setRenameError(t('columnMenu.saveErrorRollback'));
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
      setSelectedPersona(currentStage.persona ?? '');
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
        setPersonaError(t('columnMenu.errNameLettersOrDigits'));
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
    if (nextPersona === (currentStage.persona ?? null)) {
      setMode('main');
      return;
    }

    const matchedPersona = personas.find((p) => p.role === nextPersona);
    let nextIcon = currentStage.icon;
    if (nextPersona && matchedPersona?.icon) {
      nextIcon = matchedPersona.icon;
    } else if (!nextPersona) {
      nextIcon = getDefaultColumnIcon(currentStage.id, currentStage.icon);
    }

    // Автоматическое наследование цвета агента стадией (если не backlog и не ready)
    let nextColor = currentStage.color;
    if (currentStage.id !== 'backlog' && currentStage.id !== 'ready' && !currentStage.isIntake && !currentStage.terminal) {
      if (nextPersona) {
        const personaColor = matchedPersona?.color || getDeterministicAgentColor(nextPersona);
        if (personaColor && isAllowedAgentColor(personaColor)) {
          nextColor = personaColor;
        }
      }
    }

    setPersonaSaving(true);
    const prev = stages;
    const next = prev.map((c) =>
      c.id === currentStage.id ? { ...c, persona: nextPersona, icon: nextIcon, color: nextColor } : c,
    );
    setStages(next);
    useCortexStore.getState().syncStagesFromColumns(next);
    if (updateStage) {
      await updateStage(currentStage.id, { persona: nextPersona, icon: nextIcon, color: nextColor });
    }
    const saved = await savePipelineConfig(next);
    setPersonaSaving(false);
    if (!saved) {
      setStages(prev);
      useCortexStore.getState().syncStagesFromColumns(prev);
      setPersonaError(t('columnMenu.saveErrorRollback'));
      return;
    }
    onClose();
  };

  const handleSaveScript = async () => {
    const trimmed = draftScript.trim();
    const currentVal = scriptType === 'on_enter' ? currentEnterScript : currentLeaveScript;
    if (trimmed === currentVal) {
      setMode('main');
      return;
    }

    setScriptError(null);
    setScriptSaving(true);
    const prev = stages;

    const existingHooks = { ...currentStage.hooks };
    const updatedHooks: Record<string, HookDefinition> = { ...existingHooks };
    let updatedOnEnterScript: string | null | undefined = currentStage.onEnterScript;

    if (scriptType === 'on_enter') {
      if (trimmed) {
        const existingEnter = existingHooks['on_enter'];
        const desc = typeof existingEnter === 'object' && existingEnter !== null ? existingEnter.description : undefined;
        updatedHooks['on_enter'] = desc ? { script: trimmed, description: desc } : { script: trimmed };
        updatedOnEnterScript = trimmed;
      } else {
        delete updatedHooks['on_enter'];
        updatedOnEnterScript = null;
      }
    } else {
      if (trimmed) {
        const existingLeave = existingHooks['on_leave'] ?? existingHooks['on_exit'] ?? existingHooks['on_finish'];
        const desc = typeof existingLeave === 'object' && existingLeave !== null ? existingLeave.description : undefined;
        updatedHooks['on_leave'] = desc ? { script: trimmed, description: desc } : { script: trimmed };
      } else {
        delete updatedHooks['on_leave'];
      }
      delete updatedHooks['on_exit'];
      delete updatedHooks['on_finish'];
    }

    const finalHooks = Object.keys(updatedHooks).length > 0 ? updatedHooks : undefined;
    const updatedFields: Partial<PipelineStage> = {
      hooks: finalHooks,
      ...(scriptType === 'on_enter' ? { onEnterScript: updatedOnEnterScript } : {}),
    };

    const next = prev.map((c) => (c.id === currentStage.id ? { ...c, ...updatedFields } : c));
    setStages(next);
    if (updateStage) {
      await updateStage(currentStage.id, updatedFields);
    }
    const saved = await savePipelineConfig(next);
    setScriptSaving(false);
    if (!saved) {
      setStages(prev);
      setScriptError(t('columnMenu.saveErrorRollback'));
      return;
    }
    onClose();
  };

  const handleDelete = () => {
    onClose();
    if (onDeleteStage) {
      onDeleteStage(currentStage.id);
    } else if (onDeleteColumn) {
      onDeleteColumn(currentStage.id);
    }
  };

  const menu = (
    <div
      ref={menuRef}
      role="menu"
      className="fixed z-50 rounded-md border border-[#ebebeb] dark:border-[#2c2c33] bg-white dark:bg-[#17171b] py-1 font-sans text-xs text-[#171717] dark:text-[#f4f4f5] shadow-md transition-colors"
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
              setDraftTitle(currentStage.title);
              setRenameError(null);
              setMode('rename');
            }}
            className="w-full px-3 py-1.5 text-left hover:bg-[#f4f4f5] dark:hover:bg-[#232328] cursor-pointer"
          >
            {t('columnMenu.rename')}
          </button>
          {canChangeAgent && (
            <button
              type="button"
              role="menuitem"
              onClick={() => setMode('persona')}
              className="w-full px-3 py-1.5 text-left hover:bg-[#f4f4f5] dark:hover:bg-[#232328] cursor-pointer"
            >
              {t('columnMenu.changeAgent')}
            </button>
          )}
          <button
            type="button"
            role="menuitem"
            onClick={() => {
              setDraftScript(currentEnterScript);
              setScriptType('on_enter');
              setScriptError(null);
              setMode('script');
            }}
            className="w-full px-3 py-1.5 text-left hover:bg-[#f4f4f5] dark:hover:bg-[#232328] cursor-pointer flex items-center justify-between"
          >
            <span className="flex items-center gap-1.5 min-w-0">
              <ArrowRightToLine className="h-3.5 w-3.5 text-emerald-600 dark:text-emerald-400 shrink-0 stroke-[2.2]" />
              <span>{t('columnMenu.enterScript')}</span>
            </span>
            {currentEnterScript && (
              <span className="ml-2 font-mono text-[10px] text-[#8f8f8f] dark:text-[#71717a] truncate max-w-[120px]">
                {currentEnterScript.split(/[\\/]/).pop()}
              </span>
            )}
          </button>
          <button
            type="button"
            role="menuitem"
            onClick={() => {
              setDraftScript(currentLeaveScript);
              setScriptType('on_leave');
              setScriptError(null);
              setMode('script');
            }}
            className="w-full px-3 py-1.5 text-left hover:bg-[#f4f4f5] dark:hover:bg-[#232328] cursor-pointer flex items-center justify-between"
          >
            <span className="flex items-center gap-1.5 min-w-0">
              <ArrowRightFromLine className="h-3.5 w-3.5 text-amber-600 dark:text-amber-400 shrink-0 stroke-[2.2]" />
              <span>{t('columnMenu.leaveScript')}</span>
            </span>
            {currentLeaveScript && (
              <span className="ml-2 font-mono text-[10px] text-[#8f8f8f] dark:text-[#71717a] truncate max-w-[120px]">
                {currentLeaveScript.split(/[\\/]/).pop()}
              </span>
            )}
          </button>
          <div className="my-1 border-t border-[#ebebeb] dark:border-[#212126]" />
          <button
            type="button"
            role="menuitem"
            disabled={isProtected}
            title={isProtected ? t('columnMenu.protectedNotice') : undefined}
            onClick={handleDelete}
            className={`w-full px-3 py-1.5 text-left ${
              isProtected
                ? 'text-[#8f8f8f] dark:text-[#71717a] cursor-not-allowed'
                : 'text-[#b91c1c] dark:text-[#f87171] hover:bg-[#fef2f2] dark:hover:bg-[#450a0a]/40 cursor-pointer'
            }`}
          >
            {t('columnMenu.delete')}
          </button>
        </>
      )}

      {mode === 'rename' && (
        <div className="px-3 py-2">
          <div className="text-[#4d4d4d] dark:text-[#a1a1aa] font-medium mb-1">{t('columnMenu.newTitle')}</div>
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
            className="w-full px-2.5 py-1.5 rounded-md bg-[#fafafa] dark:bg-[#111114] border border-[#ebebeb] dark:border-[#2c2c33] text-[#171717] dark:text-[#f4f4f5] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f4f4f5] focus:bg-white dark:focus:bg-[#17171b] transition"
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
              className="px-2.5 py-1.5 rounded-md bg-white dark:bg-[#111114] border border-[#ebebeb] dark:border-[#2c2c33] hover:bg-[#f4f4f5] dark:hover:bg-[#232328] transition cursor-pointer text-[#171717] dark:text-[#f4f4f5]"
            >
              {t('common.cancel')}
            </button>
            <button
              type="button"
              disabled={!draftTitle.trim() || renameSaving}
              onClick={() => void handleSaveRename()}
              className="px-3 py-1.5 rounded-md bg-[#171717] dark:bg-[#232328] hover:bg-[#262626] dark:hover:bg-[#2c2c33] text-white font-medium transition cursor-pointer disabled:opacity-50 disabled:cursor-default border border-transparent dark:border-[#2c2c33]"
            >
              {t('common.save')}
            </button>
          </div>
        </div>
      )}

      {mode === 'persona' && (
        <div className="px-3 py-2">
          <div className="text-[#4d4d4d] dark:text-[#a1a1aa] font-medium mb-1">{t('columnMenu.agentColumn')}</div>
          <div className="mb-1.5 text-[#8f8f8f] dark:text-[#71717a]">
            {t('columnMenu.currentAgent', { agent: currentStage.persona ?? t('columnMenu.notAssigned') })}
          </div>
          {usePersonaSelect ? (
            <PersonaSelect
              value={isNewAgentSelected ? NEW_AGENT_OPTION : selectedPersona}
              onChange={(val) => {
                if (val === NEW_AGENT_OPTION) {
                  setIsNewAgentSelected(true);
                } else {
                  setIsNewAgentSelected(false);
                  setSelectedPersona(val);
                }
              }}
              disabled={personasLoading || personaSaving}
              personas={personas}
              allowNewAgent={true}
              newAgentOptionValue={NEW_AGENT_OPTION}
            />
          ) : (
            <input
              type="text"
              autoFocus
              value={selectedPersona}
              onChange={(e) => setSelectedPersona(e.target.value)}
              placeholder={personasLoading ? t('board.loadingAgents') : 'developer'}
              className="w-full px-2.5 py-1.5 rounded-md bg-[#fafafa] dark:bg-[#111114] border border-[#ebebeb] dark:border-[#2c2c33] text-[#171717] dark:text-[#f4f4f5] placeholder:text-[#8f8f8f] dark:placeholder:text-[#52525b] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f4f4f5] focus:bg-white dark:focus:bg-[#17171b] font-mono transition"
            />
          )}
          {usePersonaSelect && isNewAgentSelected && (
            <>
              <input
                type="text"
                autoFocus
                value={newAgentName}
                onChange={(e) => setNewAgentName(e.target.value)}
                placeholder={t('board.newAgentNamePlaceholder')}
                className="mt-1.5 w-full px-2.5 py-1.5 rounded-md bg-[#fafafa] dark:bg-[#111114] border border-[#ebebeb] dark:border-[#2c2c33] text-[#171717] dark:text-[#f4f4f5] placeholder:text-[#8f8f8f] dark:placeholder:text-[#52525b] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f4f4f5] focus:bg-white dark:focus:bg-[#17171b] font-mono transition"
              />
              {newAgentName.trim() !== '' ? (
                <div className="mt-1 text-[10px] font-mono text-[#8f8f8f]">
                  {t('columnMenu.roleNameHint', {
                    role: slugifyTitle(newAgentName) === 'column'
                      ? t('columnMenu.lettersOrDigitsRequired')
                      : slugifyTitle(newAgentName)
                  })}
                </div>
              ) : (
                <div className="mt-1 text-[10px] text-[#8f8f8f]">
                  {t('columnMenu.translitHint')}
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
              className="px-2.5 py-1.5 rounded-md bg-white dark:bg-[#111114] border border-[#ebebeb] dark:border-[#2c2c33] hover:bg-[#f4f4f5] dark:hover:bg-[#232328] text-[#171717] dark:text-[#f4f4f5] transition cursor-pointer"
            >
              {t('common.back')}
            </button>
            <button
              type="button"
              disabled={personaSaving || personasLoading}
              onClick={() => void handleSavePersona()}
              className="px-3 py-1.5 rounded-md bg-[#171717] dark:bg-[#232328] hover:bg-[#262626] dark:hover:bg-[#2c2c33] text-white font-medium transition cursor-pointer disabled:opacity-50 disabled:cursor-default border border-transparent dark:border-[#2c2c33]"
            >
              {t('common.save')}
            </button>
          </div>
        </div>
      )}

      {mode === 'script' && (
        <div className="px-3 py-2">
          <div className="text-[#4d4d4d] dark:text-[#a1a1aa] font-medium mb-1 flex items-center gap-1.5">
            {scriptType === 'on_enter' ? (
              <ArrowRightToLine className="h-3.5 w-3.5 text-emerald-600 dark:text-emerald-400 shrink-0 stroke-[2.2]" />
            ) : (
              <ArrowRightFromLine className="h-3.5 w-3.5 text-amber-600 dark:text-amber-400 shrink-0 stroke-[2.2]" />
            )}
            <span>{scriptType === 'on_enter' ? t('columnMenu.enterScript') : t('columnMenu.leaveScript')}</span>
          </div>
          <div
            className="mb-1.5 text-[#8f8f8f] dark:text-[#71717a] truncate font-mono text-[10px]"
            title={(scriptType === 'on_enter' ? currentEnterScript : currentLeaveScript) || t('columnMenu.notSet')}
          >
            {t('columnMenu.currentScript', {
              script: (scriptType === 'on_enter' ? currentEnterScript : currentLeaveScript) || t('columnMenu.notSet'),
            })}
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
            className="w-full px-2.5 py-1.5 rounded-md bg-[#fafafa] dark:bg-[#111114] border border-[#ebebeb] dark:border-[#2c2c33] text-[#171717] dark:text-[#f4f4f5] placeholder:text-[#8f8f8f] dark:placeholder:text-[#52525b] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f4f4f5] focus:bg-white dark:focus:bg-[#17171b] font-mono text-xs transition"
          />
          <div className="mt-1 text-[10px] text-[#8f8f8f] dark:text-[#71717a]">
            {scriptType === 'on_enter' ? t('columnMenu.enterScriptHint') : t('columnMenu.leaveScriptHint')}
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
              className="px-2.5 py-1.5 rounded-md bg-white dark:bg-[#111114] border border-[#ebebeb] dark:border-[#2c2c33] hover:bg-[#f4f4f5] dark:hover:bg-[#232328] text-[#171717] dark:text-[#f4f4f5] transition cursor-pointer"
            >
              {t('common.back')}
            </button>
            <button
              type="button"
              disabled={scriptSaving}
              onClick={() => void handleSaveScript()}
              className="px-3 py-1.5 rounded-md bg-[#171717] dark:bg-[#232328] hover:bg-[#262626] dark:hover:bg-[#2c2c33] text-white font-medium transition cursor-pointer disabled:opacity-50 disabled:cursor-default border border-transparent dark:border-[#2c2c33]"
            >
              {t('common.save')}
            </button>
          </div>
        </div>
      )}
    </div>
  );

  if (typeof document === 'undefined') {
    return menu;
  }
  return createPortal(menu, document.body);
}
