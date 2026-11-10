import { useEffect, useState } from 'react';
import {
  ChevronDown,
  ChevronRight,
  Cpu,
  User,
  Settings,
  Folder,
  FolderOpen,
  FileText,
  Plus,
} from 'lucide-react';
import type { CortexAgentFile, CortexAgentStage } from '../../types';
import type { ExpandAllSignal } from './types';
import AgentIcon from '../common/AgentIcon';
import { RuntimeRuleIcon, isRuntimeRule } from '../common/RuntimeRuleIcon';
import { useTranslation } from '../../stores/languageStore';
import { useCortexStore } from '../../stores/cortexStore';
import { getDeterministicAgentColor } from '../../lib/agentPalette';
import NewCortexFileForm from './NewCortexFileForm';
import DirtyDot from './DirtyDot';
import { isAgentDirty, isFileDirty } from '../../stores/cortex/cortexGitSave';

export interface AgentMindInspectorProps {
  stage: CortexAgentStage;
  isFocused?: boolean;
  isSearching?: boolean;
  expandAllSignal?: ExpandAllSignal;
  onContextMenu?: (e: React.MouseEvent) => void;
}

/**
 * Deep component representing an autonomous Agent's Mind:
 * - Agent header with role icon, built-in cvh-dev badge, inline plus action, and context menu.
 * - Persona prompt (persona.md) with missing template restoration.
 * - Rules folder (.cvh/agents/<role>/rules/*.md) with live folder expansion and inline rule creation.
 * - Settings configuration (setting) with missing template restoration.
 * - Invariant: NEVER displays hooks/ (hooks belong strictly to pipeline columns).
 */
export function AgentMindInspector({
  stage,
  isFocused = false,
  isSearching = false,
  expandAllSignal,
  onContextMenu,
}: AgentMindInspectorProps) {
  const { t } = useTranslation();
  const activeFile = useCortexStore((s) => s.activeFile);
  const fileMissing = useCortexStore((s) => s.fileMissing);
  const openFile = useCortexStore((s) => s.openFile);
  const toggleStageExpand = useCortexStore((s) => s.toggleStageExpand);
  const createPersonaTemplate = useCortexStore((s) => s.createPersonaTemplate);
  const createSettingTemplate = useCortexStore((s) => s.createSettingTemplate);
  const cvhGitStatus = useCortexStore((s) => s.cvhGitStatus);
  const pendingDeletes = useCortexStore((s) => s.pendingDeletes);
  const restorePendingDelete = useCortexStore((s) => s.restorePendingDelete);
  const agentDirty = isAgentDirty(stage, cvhGitStatus, pendingDeletes);

  const [isRulesOpen, setIsRulesOpen] = useState(true);
  const [isCreatingRule, setIsCreatingRule] = useState(false);

  useEffect(() => {
    if (expandAllSignal && expandAllSignal.nonce > 0) {
      setIsRulesOpen(expandAllSignal.expanded);
    }
  }, [expandAllSignal]);

  const personaFile = stage.files.find((f) => f.type === 'persona');
  const ruleFiles = stage.files.filter((f) => f.type === 'rule');
  const settingsFile = stage.files.find((f) => f.type === 'settings');

  const personaIsMissing = personaFile ? Boolean(fileMissing[personaFile.id]) : false;
  const settingsIsMissing = settingsFile ? Boolean(fileMissing[settingsFile.id]) : false;
  const isBuiltin = stage.personaName === 'cvh-dev' || Boolean(stage.isBuiltin);
  const stageColor = stage.color || getDeterministicAgentColor(stage.personaName);

  const handleToggleStage = () => {
    toggleStageExpand(stage.stageId);
  };

  const handleHeaderPlus = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (!stage.isExpanded) {
      toggleStageExpand(stage.stageId);
    }
    setIsCreatingRule((prev) => !prev);
  };

  const handleRulesFolderPlus = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (!isRulesOpen) {
      setIsRulesOpen(true);
    }
    setIsCreatingRule(true);
  };

  return (
    <div
      id={`cortex-agent-${stage.personaName}`}
      onContextMenu={onContextMenu}
      className={`rounded-md transition-colors ${
        isFocused
          ? 'bg-white dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] shadow-xs'
          : 'hover:bg-[#f4f4f5] dark:hover:bg-[#1e1e23]'
      }`}
    >
      {/* Agent Mind Header */}
      <div className="group flex items-center gap-0.5 px-1">
        <button
          type="button"
          onClick={handleToggleStage}
          className="flex-1 flex items-center justify-between px-1.5 py-1.5 rounded-md text-left transition cursor-pointer min-w-0"
        >
          <div className="flex items-center space-x-2 min-w-0">
            {stage.isExpanded ? (
              <ChevronDown className="h-3.5 w-3.5 text-[#666666] dark:text-[#71717a] shrink-0" />
            ) : (
              <ChevronRight className="h-3.5 w-3.5 text-[#666666] dark:text-[#71717a] shrink-0" />
            )}

            {isBuiltin ? (
              <Cpu className="h-3.5 w-3.5 text-indigo-600 shrink-0" />
            ) : stage.icon ? (
              <AgentIcon
                name={stage.icon}
                className="h-3.5 w-3.5 shrink-0"
                style={{ color: stageColor }}
              />
            ) : (
              <span
                className="w-2 h-2 rounded-full shrink-0"
                style={{ backgroundColor: stageColor }}
              />
            )}

            <span className={`font-sans text-xs font-semibold text-[#171717] dark:text-[#f4f4f5] truncate leading-none ${stage.isDeleted ? 'line-through opacity-50' : ''}`} title={stage.isDeleted ? t('cortex.deletedUnsaved') : undefined}>
              {stage.stageTitle}
            </span>
            {agentDirty && <DirtyDot />}

            {isBuiltin && (
              <span className="px-1.5 py-0.5 rounded text-[10px] font-sans font-medium bg-indigo-50 dark:bg-indigo-950/50 text-indigo-700 dark:text-indigo-400 border border-indigo-200 dark:border-indigo-800 shrink-0 self-center leading-none">
                {t('cortex.builtinBadge')}
              </span>
            )}
          </div>
        </button>

        <button
          type="button"
          onClick={handleHeaderPlus}
          aria-label={t('cortex.newRule')}
          className="p-1 rounded-md text-[#8f8f8f] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] hover:bg-[#ebebeb]/60 dark:hover:bg-[#232328] transition cursor-pointer opacity-0 group-hover:opacity-100 focus:opacity-100 focus-visible:opacity-100 shrink-0"
        >
          <Plus className="h-3.5 w-3.5" />
        </button>
      </div>

      {/* Inline Form for creating a new agent rule */}
      {isCreatingRule && (
        <NewCortexFileForm
          stage={stage}
          initialKind="rule"
          fixedKind="rule"
          onClose={() => setIsCreatingRule(false)}
        />
      )}

      {/* Agent Mind Body */}
      {stage.isExpanded && (
        <div className="pl-4 pr-1 py-0.5 space-y-0.5 border-l border-[#ebebeb] dark:border-[#212126] ml-3 my-0.5">
          {/* 1. Persona Prompt (persona.md) */}
          {personaFile && !personaIsMissing && (
            <TreeFileRow
              file={personaFile}
              isActive={activeFile?.id === personaFile.id}
              icon={
                isBuiltin ? (
                  <Cpu className="h-3.5 w-3.5 text-indigo-600 shrink-0" />
                ) : (
                  <User className="h-3.5 w-3.5 text-[#666666] shrink-0" />
                )
              }
              onSelect={() => void openFile(personaFile.id)}
            />
          )}

          {personaFile && personaIsMissing && (
            <MissingTemplateBlock
              message={t('cortex.personaNotConfigured')}
              buttonLabel={t('cortex.createTemplate')}
              onCreate={() => void createPersonaTemplate(personaFile.id)}
            />
          )}

          {/* 2. Agent Rules Folder (rules/) */}
          {(!isSearching || ruleFiles.length > 0) && (
            <div className="space-y-0.5">
              <div className="group/folder flex items-center gap-0.5">
                <button
                  type="button"
                  onClick={() => setIsRulesOpen((prev) => !prev)}
                  className="flex-1 min-w-0 flex items-center space-x-1.5 px-2 py-1 rounded-md text-left text-[#666666] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] hover:bg-[#ebebeb]/60 dark:hover:bg-[#232328] transition cursor-pointer"
                >
                  {isRulesOpen ? (
                    <FolderOpen className="h-3.5 w-3.5 text-[#666666] dark:text-[#71717a] shrink-0" />
                  ) : (
                    <Folder className="h-3.5 w-3.5 text-[#666666] dark:text-[#71717a] shrink-0" />
                  )}
                  <span className="text-xs font-mono leading-none">{t('cortex.rules')}</span>
                  <span className="text-[10px] font-mono text-[#8f8f8f] leading-none">rules/</span>
                  <span className="text-[10px] text-[#8f8f8f] tabular-nums leading-none">({ruleFiles.length})</span>
                  {ruleFiles.some((f) => isFileDirty(f, cvhGitStatus)) && <DirtyDot />}
                </button>
                <button
                  type="button"
                  onClick={handleRulesFolderPlus}
                  aria-label={t('cortex.newRule')}
                  className="p-1 rounded-md text-[#8f8f8f] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] hover:bg-[#ebebeb]/60 dark:hover:bg-[#232328] transition cursor-pointer opacity-0 group-hover/folder:opacity-100 focus:opacity-100 focus-visible:opacity-100 shrink-0"
                >
                  <Plus className="h-3 w-3" />
                </button>
              </div>

              {isRulesOpen && ruleFiles.length > 0 && (
                <div className="pl-4 space-y-0.5 border-l border-[#ebebeb] dark:border-[#212126] ml-2">
                  {ruleFiles.map((ruleFile) => {
                    const isRuntime = isRuntimeRule(ruleFile.name);
                    return (
                      <TreeFileRow
                        key={ruleFile.id}
                        file={ruleFile}
                        isActive={activeFile?.id === ruleFile.id}
                        icon={
                          isRuntime ? (
                            <RuntimeRuleIcon className="h-3.5 w-3.5 text-indigo-500 dark:text-indigo-400 shrink-0" />
                          ) : (
                            <FileText className="h-3.5 w-3.5 text-[#666666] dark:text-[#71717a] shrink-0" />
                          )
                        }
                        badge={
                          isRuntime ? (
                            <span
                              title={t('cortex.runtimeRuleTooltip')}
                              className="px-1 py-px rounded border border-indigo-200 dark:border-indigo-800/80 bg-indigo-50 dark:bg-indigo-950/50 text-[9px] font-mono font-medium text-indigo-600 dark:text-indigo-400 shrink-0 leading-tight"
                            >
                              {t('cortex.runtimeRuleBadge')}
                            </span>
                          ) : undefined
                        }
                        title={isRuntime ? t('cortex.runtimeRuleTooltip') : ruleFile.path}
                        onSelect={() => void openFile(ruleFile.id)}
                        onContextMenu={ruleFile.isDeleted ? (e) => { e.preventDefault(); e.stopPropagation(); restorePendingDelete(`file:${ruleFile.id}`); } : undefined}
                      />
                    );
                  })}
                </div>
              )}
            </div>
          )}

          {/* 3. Agent Setting File (setting) */}
          {settingsFile && !settingsIsMissing && (
            <TreeFileRow
              file={settingsFile}
              isActive={activeFile?.id === settingsFile.id}
              icon={<Settings className="h-3.5 w-3.5 text-[#666666] shrink-0" />}
              onSelect={() => void openFile(settingsFile.id)}
            />
          )}

          {settingsFile && settingsIsMissing && (
            <MissingTemplateBlock
              message={t('cortex.settingNotConfigured')}
              buttonLabel={t('cortex.createTemplate')}
              onCreate={() => void createSettingTemplate(settingsFile.id)}
            />
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
  badge?: React.ReactNode;
  title?: string;
  onSelect: () => void;
  onContextMenu?: (e: React.MouseEvent) => void;
}

function TreeFileRow({ file, isActive, icon, badge, title, onSelect, onContextMenu }: TreeFileRowProps) {
  const { t } = useTranslation();
  const cvhGitStatus = useCortexStore((s) => s.cvhGitStatus);
  const dirty = isFileDirty(file, cvhGitStatus);
  return (
    <button
      type="button"
      onClick={onSelect}
      onContextMenu={onContextMenu}
      className={`w-full flex items-center justify-between px-2 py-1 rounded-md text-left transition cursor-pointer ${
        isActive
          ? 'bg-white dark:bg-[#232328] border border-[#ebebeb] dark:border-[#2c2c33] text-[#171717] dark:text-[#f4f4f5] font-semibold shadow-xs'
          : 'text-[#4d4d4d] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] hover:bg-[#ebebeb]/60 dark:hover:bg-[#232328]'
      }`}
      title={title ?? file.path}
    >
      <div className="flex items-center space-x-1.5 min-w-0">
        {icon}
        <span className={`text-xs font-mono truncate leading-none ${file.isDeleted ? 'line-through text-[#a1a1aa]' : ''}`} title={file.isDeleted ? t('cortex.deletedUnsaved') : undefined}>{file.name}</span>
        {typeof badge === 'string' ? (
          <span className="px-1 py-px rounded border border-[#ebebeb] dark:border-[#2c2c33] bg-[#f4f4f5] dark:bg-[#17171b] text-[10px] font-mono text-[#666666] dark:text-[#71717a] shrink-0">
            {badge}
          </span>
        ) : (
          badge
        )}
      </div>

      {dirty && (
        <span
          className="h-1.5 w-1.5 rounded-full bg-amber-500 shrink-0"
          title={t('cortex.unsavedChanges')}
        />
      )}
    </button>
  );
}

interface MissingTemplateBlockProps {
  message: string;
  buttonLabel: string;
  onCreate: () => void;
}

function MissingTemplateBlock({ message, buttonLabel, onCreate }: MissingTemplateBlockProps) {
  return (
    <div className="px-2 py-1.5 rounded-md border border-dashed border-[#d4d4d4] dark:border-[#2c2c33] bg-white dark:bg-[#17171b] space-y-1">
      <div className="text-[11px] font-sans text-[#666666] dark:text-[#71717a]">
        {message}
      </div>
      <button
        type="button"
        onClick={onCreate}
        className="flex items-center space-x-1 px-2 py-1 rounded-md text-[11px] font-mono bg-[#171717] dark:bg-[#232328] hover:bg-[#262626] dark:hover:bg-[#2c2c33] text-white dark:text-[#f4f4f5] transition cursor-pointer"
      >
        <Plus className="h-3 w-3 shrink-0" />
        <span>{buttonLabel}</span>
      </button>
    </div>
  );
}

export default AgentMindInspector;
