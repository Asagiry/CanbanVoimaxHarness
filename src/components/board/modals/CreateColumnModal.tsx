import React, { useState, useEffect } from 'react';
import { X, Check } from 'lucide-react';
import { useBoardStore } from '../../../stores/boardStore';
import { useCortexStore } from '../../../stores/cortexStore';
import { useTranslation } from '../../../stores/languageStore';
import { PipelineColumn } from '../../../types';
import PersonaSelect from '../../common/PersonaSelect';
import AgentIcon from '../../common/AgentIcon';
import { slugifyTitle, uniqueColumnId } from '../../../lib/slug';
import {
  AGENT_PALETTE,
  AGENT_ICONS,
  getDeterministicAgentColor,
  isAllowedAgentColor,
} from '../../../lib/agentPalette';

interface AgentPersonaOption {
  role: string;
  hasPrompt?: boolean;
  has_prompt?: boolean;
  isBuiltin?: boolean;
  is_builtin?: boolean;
  icon?: string;
  color?: string;
}

const NEW_AGENT_OPTION = '__new__';

export interface CreateColumnModalProps {
  isOpen: boolean;
  onClose: () => void;
  columns: PipelineColumn[];
}

export default function CreateColumnModal({
  isOpen,
  onClose,
  columns,
}: CreateColumnModalProps) {
  const { setColumns, saveCurrentBoard } = useBoardStore();
  const { t } = useTranslation();

  const [newColumnTitle, setNewColumnTitle] = useState('');
  const [newColumnPersona, setNewColumnPersona] = useState('developer');
  const [agentPersonas, setAgentPersonas] = useState<AgentPersonaOption[]>([]);
  const [usePersonaSelect, setUsePersonaSelect] = useState(false);
  const [personasLoading, setPersonasLoading] = useState(false);
  const [isNewAgentSelected, setIsNewAgentSelected] = useState(false);
  const [newAgentName, setNewAgentName] = useState('');
  const [selectedColor, setSelectedColor] = useState<string | null>(null);
  const [selectedIcon, setSelectedIcon] = useState<string | null>(null);
  const [columnFormError, setColumnFormError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  const loadAgentPersonas = async () => {
    setPersonasLoading(true);
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const list = await invoke<AgentPersonaOption[]>('list_agent_personas');
      if (Array.isArray(list)) {
        setAgentPersonas(list);
        setUsePersonaSelect(true);
        const roles = new Set(list.map((p) => p.role));
        setNewColumnPersona(roles.has('developer') ? 'developer' : (list[0]?.role ?? ''));
      } else {
        setUsePersonaSelect(false);
      }
    } catch {
      setUsePersonaSelect(false);
    } finally {
      setPersonasLoading(false);
    }
  };

  useEffect(() => {
    if (isOpen) {
      setNewColumnTitle('');
      setNewColumnPersona('developer');
      setIsNewAgentSelected(false);
      setNewAgentName('');
      setSelectedColor(null);
      setSelectedIcon(null);
      setColumnFormError(null);
      setIsSubmitting(false);
      void loadAgentPersonas();
    }
  }, [isOpen]);

  useEffect(() => {
    if (!isOpen) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  // Determine active preview / auto color & icon
  const targetPersona = (isNewAgentSelected ? slugifyTitle(newAgentName) : newColumnPersona) || 'developer';
  const matchedPersona = agentPersonas.find((p) => p.role === targetPersona);
  const autoColor = (() => {
    const personaColor = matchedPersona?.color || getDeterministicAgentColor(targetPersona);
    if (personaColor && isAllowedAgentColor(personaColor)) {
      return personaColor;
    }
    return AGENT_PALETTE[columns.length % AGENT_PALETTE.length];
  })();
  const effectiveColor = selectedColor || autoColor;

  const autoIcon = matchedPersona?.icon || 'Layers';
  const effectiveIcon = selectedIcon || autoIcon;

  const handleCreateColumn = async (e: React.FormEvent) => {
    e.preventDefault();
    const title = newColumnTitle.trim();
    if (!title || isSubmitting) return;
    setColumnFormError(null);
    setIsSubmitting(true);

    let personaName: string;
    if (usePersonaSelect && isNewAgentSelected) {
      const role = slugifyTitle(newAgentName);
      if (!role || role === 'column') {
        setColumnFormError(t('board.errNameLettersOrDigits'));
        setIsSubmitting(false);
        return;
      }
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('create_agent_persona', { role });
        await useCortexStore.getState().loadAgentPersonas();
        await loadAgentPersonas();
      } catch (err) {
        setColumnFormError(err instanceof Error ? err.message : String(err));
        setIsSubmitting(false);
        return;
      }
      personaName = role;
    } else {
      personaName = newColumnPersona.trim();
    }

    const taken = new Set(columns.map((c) => c.id));
    const id = uniqueColumnId(slugifyTitle(title), taken);
    const finalPersona = personaName || 'developer';

    const newCol: PipelineColumn = {
      id,
      title,
      color: effectiveColor,
      icon: effectiveIcon,
      persona: finalPersona,
      modelOverride: null,
      hooks: {},
    };

    const prev = columns;
    const terminalIdx = prev.findIndex((c) => Boolean(c.terminal));
    const readyIdx = prev.findIndex((c) => c.id === 'ready');
    const insertIdx =
      terminalIdx !== -1 ? terminalIdx : readyIdx !== -1 ? readyIdx : prev.length;
    const next = [...prev.slice(0, insertIdx), newCol, ...prev.slice(insertIdx)];

    setColumns(next);
    useCortexStore.getState().syncStagesFromColumns(next);
    try {
      await saveCurrentBoard(next);
      onClose();
    } catch (err) {
      setColumns(prev);
      useCortexStore.getState().syncStagesFromColumns(prev);
      setColumnFormError(t('board.errSaveColumnRollback'));
      console.info('saveCurrentBoard failed, rolling back:', err);
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 dark:bg-black/80 p-3 animate-in fade-in duration-100"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="relative flex flex-col w-[520px] max-w-[95vw] max-h-[90vh] rounded-lg border border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#0d1117] shadow-md p-5 text-[#171717] dark:text-[#f0f6fc] transition-colors overflow-hidden">
        <div className="flex items-center justify-between pb-3 border-b border-[#ebebeb] dark:border-[#21262d] shrink-0">
          <div className="flex items-center space-x-2">
            <AgentIcon name={effectiveIcon} className="h-4 w-4" style={{ color: effectiveColor }} />
            <h3 className="text-sm font-semibold text-[#171717] dark:text-[#f0f6fc]">
              {t('board.addColumnTitle')}
            </h3>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="p-1 rounded-md text-[#666666] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#f4f4f5] dark:hover:bg-[#161b22] transition cursor-pointer"
            aria-label={t('common.close')}
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        <form onSubmit={handleCreateColumn} className="mt-4 flex flex-col flex-1 min-h-0 overflow-y-auto space-y-4 text-xs pr-1">
          <div>
            <label className="block font-sans text-[#4d4d4d] dark:text-[#c9d1d9] font-medium mb-1">
              {t('board.columnTitleLabel')}
            </label>
            <input
              type="text"
              required
              autoFocus
              value={newColumnTitle}
              onChange={(e) => setNewColumnTitle(e.target.value)}
              placeholder={t('board.columnTitlePlaceholder')}
              className="w-full px-3 py-1.5 rounded-md bg-[#fafafa] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] placeholder:text-[#8f8f8f] dark:placeholder:text-[#8b949e] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc] focus:bg-white dark:focus:bg-[#0d1117] font-sans transition"
            />
          </div>

          <div>
            <label className="block font-sans text-[#4d4d4d] dark:text-[#c9d1d9] font-medium mb-1">
              {t('board.agentLabel')}
            </label>
            {usePersonaSelect ? (
              <PersonaSelect
                value={isNewAgentSelected ? NEW_AGENT_OPTION : newColumnPersona}
                onChange={(val) => {
                  if (val === NEW_AGENT_OPTION) {
                    setIsNewAgentSelected(true);
                  } else {
                    setIsNewAgentSelected(false);
                    setNewColumnPersona(val);
                  }
                }}
                disabled={personasLoading}
                personas={agentPersonas}
                allowNewAgent={true}
                newAgentOptionValue={NEW_AGENT_OPTION}
                className="w-full"
              />
            ) : (
              <input
                type="text"
                value={newColumnPersona}
                onChange={(e) => setNewColumnPersona(e.target.value)}
                placeholder={personasLoading ? t('board.loadingAgents') : 'developer'}
                className="w-full px-3 py-1.5 rounded-md bg-[#fafafa] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] placeholder:text-[#8f8f8f] dark:placeholder:text-[#8b949e] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc] focus:bg-white dark:focus:bg-[#0d1117] font-mono transition"
              />
            )}
          </div>

          {usePersonaSelect && isNewAgentSelected && (
            <div>
              <label className="block font-sans text-[#4d4d4d] dark:text-[#c9d1d9] font-medium mb-1">
                {t('board.newAgentNameLabel')}
              </label>
              <input
                type="text"
                required
                value={newAgentName}
                onChange={(e) => setNewAgentName(e.target.value)}
                placeholder={t('board.newAgentNamePlaceholder')}
                className="w-full px-3 py-1.5 rounded-md bg-[#fafafa] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] placeholder:text-[#8f8f8f] dark:placeholder:text-[#8b949e] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc] focus:bg-white dark:focus:bg-[#0d1117] font-sans transition"
              />
              {newAgentName.trim() !== '' && (
                <div className="mt-1 text-[10px] font-mono text-[#8f8f8f] dark:text-[#8b949e]">
                  {t('board.newAgentRoleHint', {
                    role: slugifyTitle(newAgentName) === 'column' ? '—' : slugifyTitle(newAgentName),
                  })}
                </div>
              )}
            </div>
          )}

          {/* Color Selection Palette */}
          <div>
            <div className="flex items-center justify-between mb-1.5">
              <label className="block font-sans text-[#4d4d4d] dark:text-[#c9d1d9] font-medium">
                {t('board.autoColor')}
              </label>
              {selectedColor && (
                <button
                  type="button"
                  onClick={() => setSelectedColor(null)}
                  className="text-[10px] text-[#71717a] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] underline cursor-pointer"
                >
                  Авто ({autoColor})
                </button>
              )}
            </div>
            <div className="flex flex-wrap items-center gap-2 p-2 rounded-md bg-[#fafafa] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d]">
              {AGENT_PALETTE.map((c) => {
                const isSelected = effectiveColor === c;
                return (
                  <button
                    key={c}
                    type="button"
                    onClick={() => setSelectedColor(c)}
                    className="relative w-6 h-6 rounded-full flex items-center justify-center transition-transform hover:scale-110 cursor-pointer shadow-2xs border border-black/10 dark:border-white/10"
                    style={{ backgroundColor: c }}
                    aria-label={c}
                  >
                    {isSelected && <Check className="h-3.5 w-3.5 text-white drop-shadow" />}
                  </button>
                );
              })}
            </div>
          </div>

          {/* Icon Selection Grid */}
          <div>
            <div className="flex items-center justify-between mb-1.5">
              <label className="block font-sans text-[#4d4d4d] dark:text-[#c9d1d9] font-medium">
                Иконка [{effectiveIcon}]
              </label>
              {selectedIcon && (
                <button
                  type="button"
                  onClick={() => setSelectedIcon(null)}
                  className="text-[10px] text-[#71717a] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] underline cursor-pointer"
                >
                  Авто ({autoIcon})
                </button>
              )}
            </div>
            <div className="p-2 rounded-md bg-[#fafafa] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d]">
              <div className="grid grid-cols-6 gap-1.5 max-h-32 overflow-y-auto p-0.5">
                {AGENT_ICONS.map((ic) => {
                  const isSelected = effectiveIcon === ic;
                  return (
                    <button
                      key={ic}
                      type="button"
                      onClick={() => setSelectedIcon(ic)}
                      className={`flex items-center justify-center p-1.5 rounded-md transition cursor-pointer ${
                        isSelected
                          ? 'bg-white dark:bg-[#21262d] border-2 border-[#171717] dark:border-[#f0f6fc] shadow-xs'
                          : 'hover:bg-white/80 dark:hover:bg-[#21262d]/80 border border-transparent text-[#71717a] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc]'
                      }`}
                      aria-label={ic}
                    >
                      <AgentIcon
                        name={ic}
                        className="h-4 w-4"
                        style={{ color: isSelected ? effectiveColor : undefined }}
                      />
                    </button>
                  );
                })}
              </div>
            </div>
          </div>

          {columnFormError && (
            <div className="w-full text-xs font-sans text-[#ef4444] bg-[#fef2f2] dark:bg-[#161b22] border border-[#fecaca] dark:border-[#3b1219] rounded-md px-3 py-1.5">
              {columnFormError}
            </div>
          )}

          <div className="flex items-center justify-end space-x-2 pt-3 border-t border-[#ebebeb] dark:border-[#21262d] shrink-0">
            <button
              type="button"
              onClick={onClose}
              disabled={isSubmitting}
              className="px-3 py-1.5 rounded-md bg-white dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] transition cursor-pointer font-sans disabled:opacity-50"
            >
              {t('common.cancel')}
            </button>
            <button
              type="submit"
              disabled={isSubmitting || !newColumnTitle.trim()}
              className="px-3.5 py-1.5 rounded-md bg-[#171717] dark:bg-[#21262d] hover:bg-[#262626] dark:hover:bg-[#30363d] text-white font-medium transition cursor-pointer font-sans shadow-xs border border-transparent dark:border-[#30363d] disabled:opacity-50"
            >
              {t('common.create')}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
