import { useRef, useState, useEffect } from 'react';
import { useContextMenuPosition } from '../../hooks/useContextMenuPosition';
import { createPortal } from 'react-dom';
import type { PipelineStage, HookDefinition } from '../../types';
import { getHookScript } from '../../types';
import { useBoardStore } from '../../stores/boardStore';
import { useCortexStore } from '../../stores/cortexStore';
import { slugifyTitle } from '../../lib/slug';
import { getDefaultColumnIcon } from '../../lib/columnIcons';
import { getDeterministicAgentColor, isAllowedAgentColor } from '../../lib/agentPalette';
import { useTranslation } from '../../stores/languageStore';
import {
  StageMainMenu,
  StageRenameView,
  StagePersonaView,
  StageScriptView,
  AgentPersonaOption,
  savePipelineConfig,
  MENU_WIDTH,
  VIEWPORT_MARGIN,
  isStageProtected,
  canStageChangeAgent,
} from './stageMenu';

export * from './stageMenu';

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
  const pos = useContextMenuPosition({ x, y, menuRef, onClose, defaultWidth: MENU_WIDTH, viewportMargin: VIEWPORT_MARGIN });
  const [mode, setMode] = useState<'main' | 'rename' | 'persona' | 'script'>('main');
  const { t } = useTranslation();

  const isProtected = isStageProtected(currentStage);
  const canChangeAgent = canStageChangeAgent(currentStage);

  const enterHookDef =
    currentStage.hooks?.['on_enter'] ??
    (currentStage.onEnterScript ? { script: currentStage.onEnterScript } : undefined);
  const leaveHookDef =
    currentStage.hooks?.['on_leave'] ??
    currentStage.hooks?.['on_exit'] ??
    currentStage.hooks?.['on_finish'];

  const currentEnterScript = getHookScript(enterHookDef) ?? '';
  const currentLeaveScript = getHookScript(leaveHookDef) ?? '';

  const [draftTitle, setDraftTitle] = useState(currentStage.title);
  const [renameError, setRenameError] = useState<string | null>(null);
  const [renameSaving, setRenameSaving] = useState(false);

  const [scriptType, setScriptType] = useState<'on_enter' | 'on_leave'>('on_enter');
  const [draftScript, setDraftScript] = useState('');
  const [scriptError, setScriptError] = useState<string | null>(null);
  const [scriptSaving, setScriptSaving] = useState(false);

  const [personas, setPersonas] = useState<AgentPersonaOption[]>([]);
  const [usePersonaSelect, setUsePersonaSelect] = useState(false);
  const [personasLoading, setPersonasLoading] = useState(false);
  const [selectedPersona, setSelectedPersona] = useState(currentStage.persona ?? '');
  const [isNewAgentSelected, setIsNewAgentSelected] = useState(false);
  const [newAgentName, setNewAgentName] = useState('');
  const [personaError, setPersonaError] = useState<string | null>(null);
  const [personaSaving, setPersonaSaving] = useState(false);

  useEffect(() => {
    let cancelled = false;
    setPersonasLoading(true);
    void (async () => {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const list = await invoke<AgentPersonaOption[]>('list_agent_personas');
        if (!cancelled && Array.isArray(list)) {
          setPersonas(list);
          setUsePersonaSelect(true);
        }
      } catch {
        if (!cancelled) setUsePersonaSelect(false);
      } finally {
        if (!cancelled) setPersonasLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

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

  const handleSavePersona = async () => {
    let personaToSave = selectedPersona.trim();
    let newIconToSave: string | undefined = undefined;
    let newColorToSave: string | undefined = undefined;

    if (usePersonaSelect && isNewAgentSelected) {
      const agentTitle = newAgentName.trim();
      if (!agentTitle) {
        setPersonaError(t('columnMenu.errNameLettersOrDigits'));
        return;
      }
      const roleSlug = slugifyTitle(agentTitle);
      if (!roleSlug || roleSlug === 'column') {
        setPersonaError(t('columnMenu.errNameLettersOrDigits'));
        return;
      }
      setPersonaError(null);
      setPersonaSaving(true);
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        newIconToSave = getDefaultColumnIcon(roleSlug);
        const autoColor = getDeterministicAgentColor(roleSlug);
        newColorToSave = isAllowedAgentColor(autoColor) ? autoColor : undefined;

        await invoke('create_agent_persona', {
          role: roleSlug,
          persona: `# ${agentTitle}\n\nОписание роли и цели агента.\n`,
          icon: newIconToSave,
          color: newColorToSave,
        });

        await useCortexStore.getState().loadAgentPersonas();
        personaToSave = roleSlug;
      } catch (err) {
        setPersonaSaving(false);
        setPersonaError(err instanceof Error ? err.message : String(err));
        return;
      }
    }

    setPersonaError(null);
    setPersonaSaving(true);
    const finalPersona = personaToSave || null;
    const finalColor = newColorToSave || currentStage.color;
    const finalIcon = newIconToSave || currentStage.icon;

    const prev = stages;
    const next = prev.map((c) => (c.id === currentStage.id ? { ...c, persona: finalPersona, color: finalColor, icon: finalIcon } : c));
    setStages(next);
    if (updateStage) {
      await updateStage(currentStage.id, { persona: finalPersona, color: finalColor, icon: finalIcon });
    }
    const saved = await savePipelineConfig(next);
    setPersonaSaving(false);
    if (!saved) {
      setStages(prev);
      setPersonaError(t('columnMenu.saveErrorRollback'));
      return;
    }
    onClose();
  };

  const handleSaveScript = async () => {
    const raw = draftScript.trim();
    setScriptError(null);
    setScriptSaving(true);

    const prevHooks = currentStage.hooks ?? {};
    const nextHooks: Record<string, HookDefinition> = { ...prevHooks };

    if (!raw) {
      delete nextHooks[scriptType];
      if (scriptType === 'on_leave') {
        delete nextHooks['on_exit'];
        delete nextHooks['on_finish'];
      }
    } else {
      nextHooks[scriptType] = { script: raw };
    }

    const prev = stages;
    const next = prev.map((c) =>
      c.id === currentStage.id
        ? {
            ...c,
            hooks: Object.keys(nextHooks).length > 0 ? nextHooks : undefined,
            onEnterScript: scriptType === 'on_enter' ? (raw || undefined) : c.onEnterScript,
          }
        : c
    );

    setStages(next);
    if (updateStage) {
      await updateStage(currentStage.id, {
        hooks: Object.keys(nextHooks).length > 0 ? nextHooks : undefined,
        onEnterScript: scriptType === 'on_enter' ? (raw || undefined) : currentStage.onEnterScript,
      });
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
    if (isProtected) return;
    const handler = onDeleteStage ?? onDeleteColumn;
    if (handler) {
      handler(currentStage.id);
    } else {
      const prev = stages;
      const next = prev.filter((c) => c.id !== currentStage.id);
      setStages(next);
      void savePipelineConfig(next);
    }
    onClose();
  };

  const menu = (
    <div
      ref={menuRef}
      style={{ left: `${pos.left}px`, top: `${pos.top}px`, width: `${MENU_WIDTH}px` }}
      role="menu"
      aria-label={currentStage.title}
      className="fixed z-50 rounded-lg bg-white dark:bg-[#1c1c21] border border-[#ebebeb] dark:border-[#2c2c33] shadow-lg py-1 text-xs select-none animate-in fade-in zoom-in-95 duration-100"
    >
      <div className="px-3 py-1.5 text-[11px] font-mono text-[#8f8f8f] dark:text-[#71717a] border-b border-[#ebebeb] dark:border-[#212126] flex items-center justify-between">
        <span className="truncate">{currentStage.title}</span>
        <span className="font-semibold text-[#666666] dark:text-[#a1a1aa]">[{currentStage.id}]</span>
      </div>

      {mode === 'main' && (
        <StageMainMenu
          isProtected={isProtected}
          canChangeAgent={canChangeAgent}
          currentEnterScript={currentEnterScript}
          currentLeaveScript={currentLeaveScript}
          onOpenRename={() => setMode('rename')}
          onOpenPersona={() => setMode('persona')}
          onOpenEnterScript={() => {
            setDraftScript(currentEnterScript);
            setScriptType('on_enter');
            setScriptError(null);
            setMode('script');
          }}
          onOpenLeaveScript={() => {
            setDraftScript(currentLeaveScript);
            setScriptType('on_leave');
            setScriptError(null);
            setMode('script');
          }}
          onDelete={handleDelete}
        />
      )}

      {mode === 'rename' && (
        <StageRenameView
          draftTitle={draftTitle}
          setDraftTitle={setDraftTitle}
          renameError={renameError}
          renameSaving={renameSaving}
          onSave={() => void handleSaveRename()}
          onCancel={() => setMode('main')}
        />
      )}

      {mode === 'persona' && (
        <StagePersonaView
          currentPersona={currentStage.persona}
          selectedPersona={selectedPersona}
          setSelectedPersona={setSelectedPersona}
          usePersonaSelect={usePersonaSelect}
          personas={personas}
          personasLoading={personasLoading}
          personaSaving={personaSaving}
          isNewAgentSelected={isNewAgentSelected}
          setIsNewAgentSelected={setIsNewAgentSelected}
          newAgentName={newAgentName}
          setNewAgentName={setNewAgentName}
          personaError={personaError}
          onSave={() => void handleSavePersona()}
          onCancel={() => setMode('main')}
        />
      )}

      {mode === 'script' && (
        <StageScriptView
          scriptType={scriptType}
          currentEnterScript={currentEnterScript}
          currentLeaveScript={currentLeaveScript}
          draftScript={draftScript}
          setDraftScript={setDraftScript}
          scriptError={scriptError}
          scriptSaving={scriptSaving}
          onSave={() => void handleSaveScript()}
          onCancel={() => setMode('main')}
        />
      )}
    </div>
  );

  if (typeof document === 'undefined') {
    return menu;
  }
  return createPortal(menu, document.body);
}
