import { useState } from 'react';
import {
  ChevronDown,
  ChevronRight,
  Folder,
  FolderOpen,
  Zap,
  Settings,
  Plus,
} from 'lucide-react';
import type { PipelineColumn, CortexAgentFile, CortexAgentStage } from '../../types';
import AgentIcon from '../common/AgentIcon';
import PersonaSelect from '../common/PersonaSelect';
import { useTranslation } from '../../stores/languageStore';
import { useCortexStore } from '../../stores/cortexStore';
import { useBoardStore } from '../../stores/boardStore';
import { getDefaultColumnIcon } from '../../lib/columnIcons';
import {
  BACKLOG_COLOR,
  READY_COLOR,
  isAllowedAgentColor,
  getDeterministicAgentColor,
} from '../../lib/agentPalette';
import { hookMomentOf, savePipelineConfig } from './types';
import NewCortexFileForm from './NewCortexFileForm';

export interface ColumnStageCardProps {
  stage: CortexAgentStage;
  column?: PipelineColumn;
  isBacklogSystem?: boolean;
  isFocused?: boolean;
  isSearching?: boolean;
}

/**
 * Deep component representing a Kanban Pipeline Column Stage:
 * - Column header with reactive icon, color inheritance, stage title, and persona hint.
 * - Inlined Agent assignment selector with auto-save and rollback.
 * - Hooks folder (hooks/) with trigger moment badges (on_enter, on_leave) and inline hook creator.
 * - For system backlog: setting file with template restoration.
 */
export function ColumnStageCard({
  stage,
  column,
  isBacklogSystem = false,
  isFocused = false,
  isSearching = false,
}: ColumnStageCardProps) {
  const { t } = useTranslation();
  const agentPersonas = useCortexStore((s) => s.agentPersonas);
  const activeFile = useCortexStore((s) => s.activeFile);
  const fileMissing = useCortexStore((s) => s.fileMissing);
  const openFile = useCortexStore((s) => s.openFile);
  const toggleStageExpand = useCortexStore((s) => s.toggleStageExpand);
  const createSettingTemplate = useCortexStore((s) => s.createSettingTemplate);
  const syncStagesFromColumns = useCortexStore((s) => s.syncStagesFromColumns);

  const columns = useBoardStore((s) => s.columns);
  const setColumns = useBoardStore((s) => s.setColumns);

  const [isHooksOpen, setIsHooksOpen] = useState(true);
  const [isCreatingHook, setIsCreatingHook] = useState(false);
  const [agentSaving, setAgentSaving] = useState(false);
  const [agentSaveError, setAgentSaveError] = useState<string | null>(null);

  const hookFiles = stage.files.filter((f) => f.type === 'hook');
  const settingsFile = stage.files.find((f) => f.type === 'settings');
  const settingsIsMissing = settingsFile ? Boolean(fileMissing[settingsFile.id]) : false;

  const isBacklog = Boolean(column?.isIntake || stage.stageId === 'backlog');
  const isReady = Boolean(column?.terminal || stage.stageId === 'ready');
  const assignedPersona = column?.persona
    ? agentPersonas.find((p) => p.role === column.persona)
    : undefined;

  const effectiveIcon = isBacklog
    ? column?.icon || stage.icon || getDefaultColumnIcon('backlog')
    : isReady
    ? column?.icon || stage.icon || getDefaultColumnIcon('ready')
    : assignedPersona?.icon || column?.icon || stage.icon || getDefaultColumnIcon(stage.stageId);

  const effectiveColor = isBacklog
    ? BACKLOG_COLOR
    : isReady
    ? READY_COLOR
    : assignedPersona?.color ||
      (column?.color && isAllowedAgentColor(column.color)
        ? column.color
        : column?.persona
        ? getDeterministicAgentColor(column.persona)
        : stage.color);

  const handleAgentChange = async (newPersonaRaw: string) => {
    if (!column) return;
    const nextPersona = newPersonaRaw.trim() || null;
    if ((column.persona ?? null) === nextPersona) return;

    setAgentSaveError(null);
    setAgentSaving(true);

    const matchedPersona = agentPersonas.find((p) => p.role === nextPersona);
    let nextIcon = column.icon;
    if (nextPersona && matchedPersona?.icon) {
      nextIcon = matchedPersona.icon;
    } else if (!nextPersona) {
      nextIcon = getDefaultColumnIcon(column.id, column.icon);
    }

    let nextColor = column.color;
    if (column.id !== 'backlog' && column.id !== 'ready') {
      if (nextPersona) {
        const personaColor = matchedPersona?.color || getDeterministicAgentColor(nextPersona);
        if (personaColor && isAllowedAgentColor(personaColor)) {
          nextColor = personaColor;
        }
      }
    }

    const prevColumns = columns;
    const nextColumns = prevColumns.map((c) =>
      c.id === column.id
        ? { ...c, persona: nextPersona, icon: nextIcon, color: nextColor }
        : c,
    );

    setColumns(nextColumns);
    syncStagesFromColumns(nextColumns);

    const saved = await savePipelineConfig(nextColumns);
    setAgentSaving(false);

    if (!saved) {
      setColumns(prevColumns);
      syncStagesFromColumns(prevColumns);
      setAgentSaveError(t('columnMenu.saveErrorRollback'));
    }
  };

  const handleHooksFolderPlus = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (!stage.isExpanded) {
      toggleStageExpand(stage.stageId);
    }
    if (!isHooksOpen) {
      setIsHooksOpen(true);
    }
    setIsCreatingHook(true);
  };

  return (
    <div
      id={`cortex-stage-${stage.stageId}`}
      className={`rounded-md transition-colors ${
        isFocused
          ? 'bg-white dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] shadow-xs'
          : 'hover:bg-[#f4f4f5] dark:hover:bg-[#161b22]'
      }`}
    >
      {/* Column Stage Header */}
      <div className="group flex items-center gap-0.5 px-1">
        <button
          type="button"
          onClick={() => toggleStageExpand(stage.stageId)}
          className="flex-1 flex items-center justify-between px-1.5 py-1.5 rounded-md text-left transition cursor-pointer min-w-0"
        >
          <div className="flex items-center space-x-2 min-w-0">
            {stage.isExpanded ? (
              <ChevronDown className="h-3.5 w-3.5 text-[#666666] dark:text-[#8b949e] shrink-0" />
            ) : (
              <ChevronRight className="h-3.5 w-3.5 text-[#666666] dark:text-[#8b949e] shrink-0" />
            )}

            {effectiveIcon ? (
              <AgentIcon
                name={effectiveIcon}
                className="h-3.5 w-3.5 shrink-0"
                style={{ color: effectiveColor }}
              />
            ) : (
              <span
                className="w-2 h-2 rounded-full shrink-0"
                style={{ backgroundColor: effectiveColor }}
              />
            )}

            <span className="font-sans text-xs font-semibold text-[#171717] dark:text-[#f0f6fc] truncate">
              {stage.stageTitle}
            </span>
          </div>

          {stage.personaName !== '' && (
            <span className="text-[10px] font-mono text-[#666666] dark:text-[#8b949e] truncate ml-1">
              {stage.personaName}
            </span>
          )}
        </button>
      </div>

      {/* Inline Form for creating a new stage hook */}
      {isCreatingHook && (
        <NewCortexFileForm
          stage={stage}
          initialKind="hook"
          fixedKind="hook"
          onClose={() => setIsCreatingHook(false)}
        />
      )}

      {/* Column Stage Body */}
      {stage.isExpanded && (
        <div className="pl-2.5 pr-1 py-0.5 space-y-0.5 border-l border-[#ebebeb] dark:border-[#21262d] ml-2 my-0.5">
          {/* Agent Persona Selector */}
          {!isBacklogSystem && !isReady && !column?.terminal && column && (
            <div className="px-0.5 py-0.5 min-w-0 w-full">
              <div className="flex items-center gap-1.5 min-w-0 w-full">
                <span className="text-[10px] font-sans font-semibold uppercase tracking-wide text-[#8f8f8f] shrink-0">
                  {t('cortex.agentField')}
                </span>
                {agentPersonas.length > 0 ? (
                  <PersonaSelect
                    value={column.persona ?? ''}
                    disabled={agentSaving}
                    onChange={handleAgentChange}
                    allowEmpty={true}
                    emptyLabel={t('cortex.noAgent')}
                    personas={agentPersonas}
                    compact={true}
                    className="flex-1 min-w-0"
                  />
                ) : (
                  <input
                    type="text"
                    defaultValue={column.persona ?? ''}
                    disabled={agentSaving}
                    onBlur={(e) => {
                      if (e.target.value !== (column.persona ?? '')) {
                        void handleAgentChange(e.target.value);
                      }
                    }}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') (e.target as HTMLInputElement).blur();
                      if (e.key === 'Escape') {
                        (e.target as HTMLInputElement).value = column.persona ?? '';
                        (e.target as HTMLInputElement).blur();
                      }
                    }}
                    placeholder="developer"
                    title={t('cortex.columnAgent')}
                    className="flex-1 min-w-0 px-1.5 py-0.5 rounded-md bg-white dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] font-mono text-[11px] placeholder:text-[#8f8f8f] dark:placeholder:text-[#8b949e] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc] transition disabled:opacity-50"
                  />
                )}
                {agentSaving && (
                  <span className="text-[10px] font-mono text-[#8f8f8f] shrink-0">...</span>
                )}
              </div>
              {agentSaveError && (
                <div className="mt-1 text-[11px] font-sans text-red-600">{agentSaveError}</div>
              )}
            </div>
          )}

          {/* Hooks Folder (hooks/) */}
          {(!isSearching || hookFiles.length > 0) && (
            <div className="space-y-0.5">
              <div className="group/folder flex items-center gap-0.5">
                <button
                  type="button"
                  onClick={() => setIsHooksOpen((prev) => !prev)}
                  className="flex-1 min-w-0 flex items-center space-x-1.5 px-2 py-1 rounded-md text-left text-[#666666] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#ebebeb]/60 dark:hover:bg-[#21262d] transition cursor-pointer"
                >
                  {isHooksOpen ? (
                    <FolderOpen className="h-3.5 w-3.5 text-[#666666] dark:text-[#8b949e] shrink-0" />
                  ) : (
                    <Folder className="h-3.5 w-3.5 text-[#666666] dark:text-[#8b949e] shrink-0" />
                  )}
                  <span className="text-xs font-mono">{t('cortex.hooks')}</span>
                  <span className="text-[10px] font-mono text-[#8f8f8f]">hooks/</span>
                  <span className="text-[10px] text-[#8f8f8f] tabular-nums leading-none translate-y-px">({hookFiles.length})</span>
                </button>
                <button
                  type="button"
                  onClick={handleHooksFolderPlus}
                  aria-label={t('cortex.newHook')}
                  className="p-1 rounded-md text-[#8f8f8f] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#ebebeb]/60 dark:hover:bg-[#21262d] transition cursor-pointer opacity-0 group-hover/folder:opacity-100 focus:opacity-100 focus-visible:opacity-100 shrink-0"
                >
                  <Plus className="h-3 w-3" />
                </button>
              </div>

              {isHooksOpen && hookFiles.length > 0 && (
                <div className="pl-4 space-y-0.5 border-l border-[#ebebeb] dark:border-[#21262d] ml-2">
                  {hookFiles.map((hookFile) => {
                    const isBuiltinHook = hookFile.path.startsWith('builtin:');
                    return (
                      <TreeFileRow
                        key={hookFile.id}
                        file={hookFile}
                        isActive={activeFile?.id === hookFile.id}
                        badge={isBuiltinHook ? t('personaSelect.builtin') : hookMomentOf(hookFile.name)}
                        icon={<Zap className="h-3.5 w-3.5 text-[#666666] dark:text-[#8b949e] shrink-0" />}
                        onSelect={() => void openFile(hookFile.id)}
                      />
                    );
                  })}
                </div>
              )}
            </div>
          )}

          {/* Backlog System Stage Settings File */}
          {isBacklogSystem && settingsFile && !settingsIsMissing && (
            <TreeFileRow
              file={settingsFile}
              isActive={activeFile?.id === settingsFile.id}
              icon={<Settings className="h-3.5 w-3.5 text-[#666666] shrink-0" />}
              onSelect={() => void openFile(settingsFile.id)}
            />
          )}

          {isBacklogSystem && settingsFile && settingsIsMissing && (
            <div className="px-2 py-1.5 rounded-md border border-dashed border-[#d4d4d4] dark:border-[#30363d] bg-white dark:bg-[#161b22] space-y-1">
              <div className="text-[11px] font-sans text-[#666666] dark:text-[#8b949e]">
                {t('cortex.settingNotConfigured')}
              </div>
              <button
                type="button"
                onClick={() => void createSettingTemplate(settingsFile.id)}
                className="flex items-center space-x-1 px-2 py-1 rounded-md text-[11px] font-mono bg-[#171717] dark:bg-[#21262d] hover:bg-[#262626] dark:hover:bg-[#30363d] text-white dark:text-[#f0f6fc] transition cursor-pointer"
                aria-label={t('cortex.createSettingTemplate')}
              >
                <Plus className="h-3 w-3 shrink-0" />
                <span>{t('cortex.createTemplate')}</span>
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

interface TreeFileRowProps {
  file: CortexAgentFile;
  isActive: boolean;
  icon: React.ReactNode;
  badge?: string | null;
  onSelect: () => void;
}

function TreeFileRow({ file, isActive, icon, badge, onSelect }: TreeFileRowProps) {
  const { t } = useTranslation();
  return (
    <button
      type="button"
      onClick={onSelect}
      className={`w-full flex items-center justify-between px-2 py-1 rounded-md text-left transition cursor-pointer ${
        isActive
          ? 'bg-white dark:bg-[#21262d] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] font-semibold shadow-xs'
          : 'text-[#4d4d4d] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#ebebeb]/60 dark:hover:bg-[#21262d]'
      }`}
      title={file.path}
    >
      <div className="flex items-center space-x-1.5 min-w-0">
        {icon}
        <span className="text-xs font-mono truncate">{file.name}</span>
        {badge && (
          <span className="px-1 py-px rounded border border-[#ebebeb] dark:border-[#30363d] bg-[#f4f4f5] dark:bg-[#161b22] text-[10px] font-mono text-[#666666] dark:text-[#8b949e] shrink-0">
            {badge}
          </span>
        )}
      </div>

      {file.isDirty && (
        <span
          className="h-1.5 w-1.5 rounded-full bg-amber-500 shrink-0"
          title={t('cortex.unsavedChanges')}
        />
      )}
    </button>
  );
}

export default ColumnStageCard;
