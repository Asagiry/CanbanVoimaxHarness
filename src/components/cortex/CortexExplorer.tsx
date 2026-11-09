import { useState, useId, useEffect, useRef } from 'react';
import { useCortexStore } from '../../stores/cortexStore';
import type { AgentPersonaInfo } from '../../stores/cortexStore';
import { useBoardStore } from '../../stores/boardStore';
import { useTranslation } from '../../stores/languageStore';
import {
  Search,
  Plus,
  Pencil,
  Trash2,
  RotateCw,
  RotateCcw,
  Palette,
  ChevronDown,
  ChevronRight,
} from 'lucide-react';
import type { CortexAgentFile, CortexAgentStage } from '../../types';
import { isBacklogSystemStage } from './types';
import ColumnStageCard from './ColumnStageCard';
import AgentMindInspector from './AgentMindInspector';
import GlobalRulesSection from './GlobalRulesSection';
import EditAgentPersonaModal from './EditAgentPersonaModal';
import CreateAgentModal from './CreateAgentModal';
import CvhSaveButton from './CvhSaveButton';
import DirtyDot from './DirtyDot';
import { isAgentDirty, isColumnDirty } from '../../stores/cortex/cortexGitSave';

export interface CortexExplorerProps {
  className?: string;
}

export default function CortexExplorer({ className = '' }: CortexExplorerProps = {}) {
  const {
    stages,
    agentStages,
    agentPersonas,
    globalRules,
    focusedStageId,
    expandStage,
    focusStage,
    openFile,
    loadAgentPersonas,
    loadGlobalRules,
    renameAgentPersona,
    deleteAgentPersona,
    setAgentPersonaColor,
    setAgentPersonaIcon,
  } = useCortexStore();

  const syncStagesFromColumns = useCortexStore((s) => s.syncStagesFromColumns);
  const refreshCvhGitStatus = useCortexStore((s) => s.refreshCvhGitStatus);
  const restorePendingDelete = useCortexStore((s) => s.restorePendingDelete);
  const cvhGitStatus = useCortexStore((s) => s.cvhGitStatus);
  const pendingDeletes = useCortexStore((s) => s.pendingDeletes);
  const activeFile = useCortexStore((s) => s.activeFile);
  const { columns, boards } = useBoardStore();
  const { t } = useTranslation();

  const [filterQuery, setFilterQuery] = useState('');
  const [openSections, setOpenSections] = useState({ columns: true, agents: true, globalRules: true });
  const [isRefreshingPersonas, setIsRefreshingPersonas] = useState(false);
  const [isCreateAgentOpen, setIsCreateAgentOpen] = useState(false);

  const refreshTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (refreshTimerRef.current) {
        clearTimeout(refreshTimerRef.current);
      }
    };
  }, []);

  const handleRefreshPersonas = async (e: React.MouseEvent) => {
    e.stopPropagation();
    if (isRefreshingPersonas) return;
    setIsRefreshingPersonas(true);
    try {
      await Promise.all([loadAgentPersonas(), loadGlobalRules()]);
    } finally {
      if (refreshTimerRef.current) {
        clearTimeout(refreshTimerRef.current);
      }
      refreshTimerRef.current = setTimeout(() => {
        setIsRefreshingPersonas(false);
        refreshTimerRef.current = null;
      }, 500);
    }
  };

  interface AgentContextMenuState {
    x: number;
    y: number;
    role: string;
    isBuiltin: boolean;
    persona: AgentPersonaInfo;
  }
  const [agentContextMenu, setAgentContextMenu] = useState<AgentContextMenuState | null>(null);
  const [editingAgentPersona, setEditingAgentPersona] = useState<AgentPersonaInfo | null>(null);

  useEffect(() => {
    if (!agentContextMenu) return;
    const handleClose = () => setAgentContextMenu(null);
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setAgentContextMenu(null);
    };
    window.addEventListener('click', handleClose);
    window.addEventListener('contextmenu', handleClose);
    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('click', handleClose);
      window.removeEventListener('contextmenu', handleClose);
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [agentContextMenu]);

  const handleAgentContextMenu = (e: React.MouseEvent, stage: CortexAgentStage) => {
    e.preventDefault();
    e.stopPropagation();
    const persona = agentPersonas.find((p) => p.role === stage.personaName) || {
      role: stage.personaName,
      hasPrompt: true,
      isBuiltin: stage.personaName === 'cvh-dev' || Boolean(stage.isBuiltin),
      icon: stage.icon,
      color: stage.color,
    };
    setAgentContextMenu({
      x: e.clientX,
      y: e.clientY,
      role: stage.personaName,
      isBuiltin: stage.personaName === 'cvh-dev' || Boolean(stage.isBuiltin),
      persona,
    });
  };

  const handleDeleteAgent = async (role: string) => {
    if (role === 'cvh-dev') {
      alert(t('cortex.deleteBuiltinForbidden'));
      return;
    }

    const assignedList: string[] = [];
    for (const b of boards) {
      for (const c of (b.stages ?? b.columns ?? [])) {
        if (c.persona === role) {
          assignedList.push(t('cortex.boardColumnFormat', { board: b.name, column: c.title }));
        }
      }
    }

    let confirmMsg = t('cortex.deleteAgentConfirm', { role });
    if (assignedList.length > 0) {
      confirmMsg += t('cortex.deleteAgentAssignedWarning', { list: assignedList.join('\n- ') });
    }

    if (!window.confirm(confirmMsg)) {
      return;
    }

    try {
      await deleteAgentPersona(role);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      alert(t('cortex.deleteAgentError', { msg }));
    }
  };

  const searchInputId = useId();

  // Reactive stage synchronization from board columns
  useEffect(() => {
    syncStagesFromColumns(columns);
  }, [columns, syncStagesFromColumns]);

  // Reactive agent persona list loading
  useEffect(() => {
    void loadAgentPersonas();
  }, [loadAgentPersonas]);

  // Reactive global rules loading
  useEffect(() => {
    void loadGlobalRules();
  }, [loadGlobalRules]);

  useEffect(() => {
    void refreshCvhGitStatus();
    const onFocus = () => void refreshCvhGitStatus();
    window.addEventListener('focus', onFocus);
    return () => window.removeEventListener('focus', onFocus);
  }, [refreshCvhGitStatus]);

  useEffect(() => {
    const id = setTimeout(() => void refreshCvhGitStatus(), 500);
    return () => clearTimeout(id);
  }, [activeFile, refreshCvhGitStatus]);

  const toggleSection = (key: 'columns' | 'agents' | 'globalRules') => {
    setOpenSections((prev) => ({ ...prev, [key]: !prev[key] }));
  };

  const q = filterQuery.trim().toLowerCase();
  const isSearching = q.length > 0;
  const matchFile = (f: CortexAgentFile) =>
    f.name.toLowerCase().includes(q) || f.path.toLowerCase().includes(q);

  // Filter column stages by name, persona, or files
  const filteredColumnStages = stages.map((stage) => {
    if (!isSearching) return stage;
    const matchesStage =
      stage.stageTitle.toLowerCase().includes(q) || stage.personaName.toLowerCase().includes(q);
    if (matchesStage) return { ...stage, isExpanded: true };
    const backlog = isBacklogSystemStage(stage);
    const matchingHooks = stage.files.filter((f) => f.type === 'hook' && matchFile(f));
    const settingsFile = stage.files.find((f) => f.type === 'settings');
    const matchingSetting =
      backlog && settingsFile && matchFile(settingsFile) ? settingsFile : null;
    if (matchingHooks.length === 0 && !matchingSetting) return null;
    const keepIds = new Set([
      ...matchingHooks.map((f) => f.id),
      ...(matchingSetting ? [matchingSetting.id] : []),
    ]);
    return { ...stage, isExpanded: true, files: stage.files.filter((f) => keepIds.has(f.id)) };
  }).filter(Boolean) as CortexAgentStage[];

  // Filter agent stages by role and files
  const filteredAgentStages = agentStages.map((stage) => {
    if (!isSearching) return stage;
    const matchesAgent =
      stage.stageTitle.toLowerCase().includes(q) || stage.personaName.toLowerCase().includes(q);
    if (matchesAgent) return { ...stage, isExpanded: true };
    const matchingFiles = stage.files.filter((f) => matchFile(f));
    if (matchingFiles.length === 0) return null;
    return { ...stage, isExpanded: true, files: matchingFiles };
  }).filter(Boolean) as CortexAgentStage[];

  // Filter global rules
  const filteredGlobalRules = globalRules.filter((f) => !isSearching || matchFile(f));

  return (
    <aside
      className={`w-64 shrink-0 my-1.5 ml-1.5 mr-1.5 rounded-xl border border-[#ebebeb] dark:border-[#212126] shadow-2xs bg-[#f9f9f9] dark:bg-[#111114] flex flex-col min-h-0 h-[calc(100%-0.75rem)] overflow-hidden select-none text-[#171717] dark:text-[#f4f4f5] transition-colors ${className}`}
    >
      {/* Sidebar Header */}
      <div className="h-12 shrink-0 px-3.5 border-b border-[#ebebeb] dark:border-[#212126] bg-white dark:bg-[#111114] flex items-center justify-between transition-colors">
        <span className="text-xs font-semibold text-[#171717] dark:text-[#f4f4f5]">
          {t('cortex.title')}
        </span>
        <div className="flex items-center gap-0.5">
        <CvhSaveButton />
        <button
          type="button"
          onClick={handleRefreshPersonas}
          aria-label={t('cortex.refreshAgents')}
          className="p-1 rounded hover:bg-[#e4e4e7] dark:hover:bg-[#232328] text-[#71717a] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] transition cursor-pointer"
        >
          <RotateCw className={`h-3.5 w-3.5 ${isRefreshingPersonas ? 'animate-spin' : ''}`} />
        </button>
        </div>
      </div>

      {/* Filter Search Input */}
      <div className="p-2.5 border-b border-[#ebebeb] dark:border-[#212126] bg-[#f9f9f9] dark:bg-[#111114]">
        <div className="relative flex items-center">
          <Search className="absolute left-2.5 h-3.5 w-3.5 text-[#8f8f8f] dark:text-[#71717a] pointer-events-none" />
          <input
            id={searchInputId}
            name="cortexMindSearch"
            type="text"
            value={filterQuery}
            onChange={(e) => setFilterQuery(e.target.value)}
            placeholder={t('cortex.searchPlaceholder')}
            className="w-full pl-8 pr-2.5 py-1 text-xs rounded-md bg-white dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] text-[#171717] dark:text-[#f4f4f5] placeholder:text-[#8f8f8f] dark:placeholder:text-[#71717a] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f4f4f5] font-sans transition"
          />
        </div>
      </div>

      {/* Sections Tree */}
      <div className="flex-1 min-h-0 overflow-y-auto p-2 space-y-1 font-mono text-xs">
        {/* СЕКЦИЯ 1: Колонки */}
        <section aria-label={t('cortex.columnsSection')}>
          <div className="w-full flex items-center justify-between px-1.5 py-1 rounded-md text-[#171717] dark:text-[#f4f4f5] hover:bg-[#f4f4f5] dark:hover:bg-[#1e1e23] transition group">
            <button
              type="button"
              onClick={() => toggleSection('columns')}
              aria-expanded={openSections.columns}
              className="flex-1 flex items-center gap-1.5 py-1 text-left cursor-pointer"
            >
              {openSections.columns ? (
                <ChevronDown className="h-3.5 w-3.5 text-[#666666] dark:text-[#71717a] shrink-0" />
              ) : (
                <ChevronRight className="h-3.5 w-3.5 text-[#666666] dark:text-[#71717a] shrink-0" />
              )}
              <span className="font-sans text-xs font-semibold leading-none">{t('cortex.columnsSection')}</span>
              <span className="text-[10px] font-mono text-[#8f8f8f] dark:text-[#71717a] tabular-nums leading-none">
                ({stages.length})
              </span>
              {stages.some((s) => isColumnDirty(s, columns.find((c) => c.id === s.stageId), cvhGitStatus)) && <DirtyDot />}
            </button>
          </div>

          {openSections.columns && (
            <div className="space-y-1">
              {stages.length === 0 && (
                <div className="mx-1 mt-1 rounded-md border border-dashed border-[#d4d4d4] dark:border-[#2c2c33] bg-white dark:bg-[#17171b] px-3 py-4 text-center space-y-1.5">
                  <div className="text-xs font-sans font-semibold text-[#171717] dark:text-[#f4f4f5]">
                    {t('cortex.noAgentsInColumns')}
                  </div>
                  <div className="text-[11px] font-sans text-[#666666] dark:text-[#71717a] leading-relaxed">
                    {t('cortex.createFirstColumnAgent')}
                  </div>
                  <div className="text-[10px] font-mono text-[#8f8f8f] dark:text-[#71717a]">
                    {t('cortex.freshProjectNotice')}
                  </div>
                </div>
              )}

              {stages.length > 0 && filteredColumnStages.length === 0 && (
                <div className="mx-1 mt-1 rounded-md border border-[#ebebeb] dark:border-[#2c2c33] bg-white dark:bg-[#17171b] px-3 py-4 text-center">
                  <div className="text-[11px] font-sans text-[#666666] dark:text-[#71717a]">
                    {t('cortex.columnsNothingFound')}
                  </div>
                </div>
              )}

              {filteredColumnStages.map((stage) => {
                const backlog = isBacklogSystemStage(stage);
                const column = columns.find((c) => c.id === stage.stageId);
                return (
                  <ColumnStageCard
                    key={stage.stageId}
                    stage={stage}
                    column={column}
                    isBacklogSystem={backlog}
                    isFocused={focusedStageId === stage.stageId}
                    isSearching={isSearching}
                  />
                );
              })}
            </div>
          )}
        </section>

        {/* СЕКЦИЯ 2: Агенты */}
        <section aria-label={t('cortex.agentsSection')}>
          <div className="w-full flex items-center justify-between px-1.5 py-1 rounded-md text-[#171717] dark:text-[#f4f4f5] hover:bg-[#f4f4f5] dark:hover:bg-[#1e1e23] transition group">
            <button
              type="button"
              onClick={() => toggleSection('agents')}
              aria-expanded={openSections.agents}
              className="flex-1 flex items-center gap-1.5 py-1 text-left cursor-pointer"
            >
              {openSections.agents ? (
                <ChevronDown className="h-3.5 w-3.5 text-[#666666] dark:text-[#71717a] shrink-0" />
              ) : (
                <ChevronRight className="h-3.5 w-3.5 text-[#666666] dark:text-[#71717a] shrink-0" />
              )}
              <span className="font-sans text-xs font-semibold leading-none">{t('cortex.agentsSection')}</span>
              <span className="text-[10px] font-mono text-[#8f8f8f] dark:text-[#71717a] tabular-nums leading-none">
                ({agentStages.length})
              </span>
              {agentStages.some((s) => isAgentDirty(s, cvhGitStatus, pendingDeletes)) && <DirtyDot />}
            </button>
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                setIsCreateAgentOpen(true);
              }}
              aria-label={t('cortex.createAgent')}
              className="p-1 rounded-md text-[#8f8f8f] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] hover:bg-[#ebebeb]/60 dark:hover:bg-[#232328] transition cursor-pointer shrink-0"
            >
              <Plus className="h-3.5 w-3.5" />
            </button>
          </div>

          {openSections.agents && (
            <div className="space-y-1">
              {agentStages.length === 0 && (
                <div className="mx-1 mt-1 rounded-md border border-dashed border-[#d4d4d4] dark:border-[#2c2c33] bg-white dark:bg-[#17171b] px-3 py-4 text-center space-y-1.5">
                  <div className="text-xs font-sans font-semibold text-[#171717] dark:text-[#f4f4f5]">
                    {t('cortex.noAvailableAgents')}
                  </div>
                  <div className="text-[11px] font-sans text-[#666666] dark:text-[#71717a] leading-relaxed">
                    {t('cortex.agentsFromProject')}
                  </div>
                </div>
              )}

              {agentStages.length > 0 && filteredAgentStages.length === 0 && (
                <div className="mx-1 mt-1 rounded-md border border-[#ebebeb] dark:border-[#2c2c33] bg-white dark:bg-[#17171b] px-3 py-4 text-center">
                  <div className="text-[11px] font-sans text-[#666666] dark:text-[#71717a]">
                    {t('cortex.agentsNothingFound')}
                  </div>
                </div>
              )}

              {filteredAgentStages.map((stage) => (
                <AgentMindInspector
                  key={stage.stageId}
                  stage={stage}
                  isFocused={focusedStageId === stage.stageId}
                  isSearching={isSearching}
                  onContextMenu={(e) => handleAgentContextMenu(e, stage)}
                />
              ))}
            </div>
          )}
        </section>

        {/* СЕКЦИЯ 3: Глобальные правила */}
        <GlobalRulesSection
          rules={filteredGlobalRules}
          totalCount={globalRules.length}
          isOpen={openSections.globalRules}
          onToggle={() => toggleSection('globalRules')}
          isSearching={isSearching}
        />
      </div>

      {/* Контекстное меню агента (ПКМ) */}
      {agentContextMenu && (
        <div
          className="fixed z-50 min-w-[190px] bg-white dark:bg-[#17171b] rounded-lg shadow-md border border-[#ebebeb] dark:border-[#2c2c33] p-1 text-xs font-sans animate-in fade-in zoom-in-95 duration-100"
          style={{
            top: Math.min(agentContextMenu.y, window.innerHeight - 120),
            left: Math.min(agentContextMenu.x, window.innerWidth - 210),
          }}
          onClick={(e) => e.stopPropagation()}
          onContextMenu={(e) => e.preventDefault()}
        >
          <div className="px-3 py-1.5 text-[10px] font-semibold text-[#a1a1aa] dark:text-[#71717a] uppercase tracking-wider border-b border-[#f4f4f5] dark:border-[#212126] mb-1 truncate">
            {t('cortex.agentContextMenu', { role: agentContextMenu.role })}
          </div>
          {pendingDeletes[`agent:${agentContextMenu.role}`] && (
            <button
              onClick={() => {
                const role = agentContextMenu.role;
                setAgentContextMenu(null);
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
              const persona = agentContextMenu.persona;
              setAgentContextMenu(null);
              setEditingAgentPersona(persona);
            }}
            className="w-full flex items-center space-x-2 px-3 py-1.5 text-left transition rounded-sm text-[#171717] dark:text-[#f4f4f5] hover:bg-[#f4f4f5] dark:hover:bg-[#232328] cursor-pointer"
          >
            <Palette className="h-3.5 w-3.5 text-[#71717a] dark:text-[#71717a]" />
            <span>{t('cortex.configureColorIcon')}</span>
          </button>
          <button
            onClick={() => {
              const persona = agentContextMenu.persona;
              setAgentContextMenu(null);
              setEditingAgentPersona(persona);
            }}
            disabled={agentContextMenu.isBuiltin}
            className={`w-full flex items-center space-x-2 px-3 py-1.5 text-left transition rounded-sm ${
              agentContextMenu.isBuiltin
                ? 'text-[#a1a1aa] dark:text-[#52525b] cursor-not-allowed opacity-50'
                : 'text-[#171717] dark:text-[#f4f4f5] hover:bg-[#f4f4f5] dark:hover:bg-[#232328] cursor-pointer'
            }`}
            aria-label={agentContextMenu.isBuiltin ? t('cortex.cannotRenameBuiltin') : undefined}
          >
            <Pencil className="h-3.5 w-3.5 text-[#71717a] dark:text-[#71717a]" />
            <span>{t('cortex.renameAgent')}</span>
          </button>
          <button
            onClick={() => {
              const role = agentContextMenu.role;
              setAgentContextMenu(null);
              if (agentContextMenu.isBuiltin) return;
              void handleDeleteAgent(role);
            }}
            disabled={agentContextMenu.isBuiltin}
            className={`w-full flex items-center space-x-2 px-3 py-1.5 text-left transition rounded-sm ${
              agentContextMenu.isBuiltin
                ? 'text-[#a1a1aa] dark:text-[#52525b] cursor-not-allowed opacity-50'
                : 'text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-950/30 hover:text-red-700 dark:hover:text-red-300 cursor-pointer'
            }`}
            aria-label={agentContextMenu.isBuiltin ? t('cortex.cannotDeleteBuiltin') : undefined}
          >
            <Trash2 className="h-3.5 w-3.5" />
            <span>{t('cortex.deleteAgent')}</span>
          </button>
        </div>
      )}

      {/* Диалог настройки / переименования агента */}
      <EditAgentPersonaModal
        isOpen={Boolean(editingAgentPersona)}
        persona={editingAgentPersona}
        onClose={() => setEditingAgentPersona(null)}
        onRename={async (oldRole, newRole) => {
          await renameAgentPersona(oldRole, newRole);
        }}
        onColorChange={async (role, color) => {
          await setAgentPersonaColor(role, color);
        }}
        onIconChange={async (role, icon) => {
          await setAgentPersonaIcon(role, icon);
        }}
      />

      {/* Модальное окно создания нового агента */}
      <CreateAgentModal
        isOpen={isCreateAgentOpen}
        existingRoles={agentPersonas.map((p) => p.role)}
        onClose={() => setIsCreateAgentOpen(false)}
        onCreateSuccess={async (role) => {
          setIsCreateAgentOpen(false);
          setOpenSections((prev) => ({ ...prev, agents: true }));
          expandStage(`agent:${role}`);
          focusStage(`agent:${role}`);
          await openFile(`agent:${role}-persona`);
        }}
      />
    </aside>
  );
}
