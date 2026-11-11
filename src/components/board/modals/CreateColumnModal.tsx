import React, { useState, useEffect } from 'react';
import { createPortal } from 'react-dom';
import { useShallow } from 'zustand/react/shallow';
import { X } from 'lucide-react';
import { useBoardStore } from '../../../stores/boardStore';
import { useCortexStore } from '../../../stores/cortexStore';
import { useTranslation } from '../../../stores/languageStore';
import { PipelineStage, PipelineColumn } from '../../../types';
import PersonaSelect from '../../common/PersonaSelect';
import AgentIcon from '../../common/AgentIcon';
import { slugifyTitle, uniqueColumnId } from '../../../lib/slug';
import {
  AGENT_PALETTE,
  getDeterministicAgentColor,
  isAllowedAgentColor,
} from '../../../lib/agentPalette';
import { useUiZoom, getZoomOverlayStyle } from '../../../hooks/useUiZoom';
import StageAppearancePicker from './StageAppearancePicker';

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
  stages?: PipelineStage[];
  /** @deprecated Use stages instead */
  columns?: PipelineColumn[];
}

export type CreateStageModalProps = CreateColumnModalProps;

export default function CreateColumnModal({
  isOpen,
  onClose,
  stages,
  columns,
}: CreateColumnModalProps) {
  const effectiveStages = stages ?? columns ?? [];
  const { setStages, saveCurrentBoard } = useBoardStore(
    useShallow((s) => ({ setStages: s.setStages, saveCurrentBoard: s.saveCurrentBoard }))
  );
  const { t } = useTranslation();
  const { zoom } = useUiZoom();

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
    } catch (err) {
      console.debug('[CreateColumnModal] list_agent_personas unavailable:', err);
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
        e.preventDefault();
        e.stopImmediatePropagation();
        onClose();
      }
    };
    window.addEventListener('keydown', handleKeyDown, { capture: true });
    return () => window.removeEventListener('keydown', handleKeyDown, { capture: true });
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  const targetPersona = (isNewAgentSelected ? slugifyTitle(newAgentName) : newColumnPersona) || 'developer';
  const matchedPersona = agentPersonas.find((p) => p.role === targetPersona);
  const autoColor = (() => {
    const personaColor = matchedPersona?.color || getDeterministicAgentColor(targetPersona);
    if (personaColor && isAllowedAgentColor(personaColor)) {
      return personaColor;
    }
    return AGENT_PALETTE[effectiveStages.length % AGENT_PALETTE.length];
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

    const taken = new Set(effectiveStages.map((c) => c.id));
    const id = uniqueColumnId(slugifyTitle(title), taken);
    const finalPersona = personaName || 'developer';

    const newCol: PipelineStage = {
      id,
      title,
      color: effectiveColor,
      icon: effectiveIcon,
      persona: finalPersona,
      modelOverride: null,
      hooks: {},
    };

    const prev = effectiveStages;
    const terminalIdx = prev.findIndex((c) => Boolean(c.terminal));
    const readyIdx = prev.findIndex((c) => c.id === 'ready');
    const insertIdx =
      terminalIdx !== -1 ? terminalIdx : readyIdx !== -1 ? readyIdx : prev.length;
    const next = [...prev.slice(0, insertIdx), newCol, ...prev.slice(insertIdx)];

    setStages(next);
    useCortexStore.getState().syncStagesFromColumns(next);
    try {
      await saveCurrentBoard(next);
      onClose();
    } catch (err) {
      setStages(prev);
      useCortexStore.getState().syncStagesFromColumns(prev);
      setColumnFormError(t('board.errSaveColumnRollback'));
      console.info('saveCurrentBoard failed, rolling back:', err);
    } finally {
      setIsSubmitting(false);
    }
  };

  const modalContent = (
    <div
      style={getZoomOverlayStyle(zoom)}
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 dark:bg-black/80 p-3 animate-in fade-in duration-100"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="relative flex flex-col w-[520px] max-w-[95%] max-h-[90%] rounded-lg border border-[#ebebeb] dark:border-[#212126] bg-white dark:bg-[#111114] shadow-md p-5 text-[#171717] dark:text-[#f4f4f5] transition-colors overflow-hidden">
        <div className="flex items-center justify-between pb-3 border-b border-[#ebebeb] dark:border-[#212126] shrink-0">
          <div className="flex items-center space-x-2">
            <AgentIcon name={effectiveIcon} className="h-4 w-4" style={{ color: effectiveColor }} />
            <h3 className="text-sm font-semibold text-[#171717] dark:text-[#f4f4f5]">
              {t('board.addColumnTitle')}
            </h3>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="p-1 rounded-md text-[#666666] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] hover:bg-[#f4f4f5] dark:hover:bg-[#17171b] transition cursor-pointer"
            aria-label={t('common.close')}
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        <form onSubmit={handleCreateColumn} className="mt-4 flex flex-col flex-1 min-h-0 overflow-y-auto space-y-4 text-xs pr-1">
          <div>
            <label className="block font-sans text-[#4d4d4d] dark:text-[#a1a1aa] font-medium mb-1">
              {t('board.columnTitleLabel')}
            </label>
            <input
              type="text"
              required
              autoFocus
              value={newColumnTitle}
              onChange={(e) => setNewColumnTitle(e.target.value)}
              placeholder={t('board.columnTitlePlaceholder')}
              className="w-full px-3 py-1.5 rounded-md bg-[#fafafa] dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] text-[#171717] dark:text-[#f4f4f5] placeholder:text-[#8f8f8f] dark:placeholder:text-[#71717a] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f4f4f5] focus:bg-white dark:focus:bg-[#111114] font-sans transition"
            />
          </div>

          <div>
            <label className="block font-sans text-[#4d4d4d] dark:text-[#a1a1aa] font-medium mb-1">
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
                className="w-full px-3 py-1.5 rounded-md bg-[#fafafa] dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] text-[#171717] dark:text-[#f4f4f5] placeholder:text-[#8f8f8f] dark:placeholder:text-[#71717a] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f4f4f5] focus:bg-white dark:focus:bg-[#111114] font-mono transition"
              />
            )}
          </div>

          {usePersonaSelect && isNewAgentSelected && (
            <div>
              <label className="block font-sans text-[#4d4d4d] dark:text-[#a1a1aa] font-medium mb-1">
                {t('board.newAgentNameLabel')}
              </label>
              <input
                type="text"
                required
                value={newAgentName}
                onChange={(e) => setNewAgentName(e.target.value)}
                placeholder={t('board.newAgentNamePlaceholder')}
                className="w-full px-3 py-1.5 rounded-md bg-[#fafafa] dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] text-[#171717] dark:text-[#f4f4f5] placeholder:text-[#8f8f8f] dark:placeholder:text-[#71717a] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f4f4f5] focus:bg-white dark:focus:bg-[#111114] font-sans transition"
              />
              {newAgentName.trim() !== '' && (
                <div className="mt-1 text-[10px] font-mono text-[#8f8f8f] dark:text-[#71717a]">
                  {t('board.newAgentRoleHint', {
                    role: slugifyTitle(newAgentName) === 'column' ? '—' : slugifyTitle(newAgentName),
                  })}
                </div>
              )}
            </div>
          )}

          <StageAppearancePicker
            selectedColor={selectedColor}
            autoColor={autoColor}
            effectiveColor={effectiveColor}
            onSelectColor={setSelectedColor}
            selectedIcon={selectedIcon}
            autoIcon={autoIcon}
            effectiveIcon={effectiveIcon}
            onSelectIcon={setSelectedIcon}
          />

          {columnFormError && (
            <div className="w-full text-xs font-sans text-[#ef4444] bg-[#fef2f2] dark:bg-[#17171b] border border-[#fecaca] dark:border-[#3b1219] rounded-md px-3 py-1.5">
              {columnFormError}
            </div>
          )}

          <div className="flex items-center justify-end space-x-2 pt-3 border-t border-[#ebebeb] dark:border-[#212126] shrink-0">
            <button
              type="button"
              onClick={onClose}
              disabled={isSubmitting}
              className="px-3 py-1.5 rounded-md bg-white dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] text-[#171717] dark:text-[#f4f4f5] hover:bg-[#f4f4f5] dark:hover:bg-[#232328] transition cursor-pointer font-sans disabled:opacity-50"
            >
              {t('common.cancel')}
            </button>
            <button
              type="submit"
              disabled={isSubmitting || !newColumnTitle.trim()}
              className="px-3.5 py-1.5 rounded-md bg-[#171717] dark:bg-[#232328] hover:bg-[#262626] dark:hover:bg-[#2c2c33] text-white font-medium transition cursor-pointer font-sans shadow-xs border border-transparent dark:border-[#2c2c33] disabled:opacity-50"
            >
              {t('common.create')}
            </button>
          </div>
        </form>
      </div>
    </div>
  );

  return typeof document !== 'undefined' ? createPortal(modalContent, document.body) : modalContent;
}
