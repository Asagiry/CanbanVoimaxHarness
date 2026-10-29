import { useState, useId, useEffect, useRef } from 'react';
import { useCortexStore } from '../../stores/cortexStore';
import type { AgentPersonaInfo } from '../../stores/cortexStore';
import { useBoardStore } from '../../stores/boardStore';
import { getDefaultColumnIcon } from '../../lib/columnIcons';
import { useTranslation } from '../../stores/languageStore';
import {
  getDeterministicAgentColor,
  isAllowedAgentColor,
} from '../../lib/agentPalette';
import {
  Search,
  Plus,
  Pencil,
  Trash2,
  RotateCw,
  Palette,
} from 'lucide-react';
import type { CortexAgentFile, CortexAgentStage } from '../../types';
import {
  isBacklogSystemStage,
  savePipelineConfig,
} from './types';
import SectionHeader from './tree/SectionHeader';
import ColumnStageCard from './cards/ColumnStageCard';
import AgentStageCard from './cards/AgentStageCard';
import EditAgentPersonaModal from './modals/EditAgentPersonaModal';
import CreateAgentModal from './modals/CreateAgentModal';

export default function CortexExplorer() {
  const {
    stages,
    agentStages,
    agentPersonas,
    activeFile,
    focusedStageId,
    toggleStageExpand,
    expandStage,
    focusStage,
    openFile,
    fileMissing,
    createPersonaTemplate,
    createSettingTemplate,
    loadAgentPersonas,
    renameAgentPersona,
    deleteAgentPersona,
    setAgentPersonaColor,
    setAgentPersonaIcon,
  } = useCortexStore();
  const syncStagesFromColumns = useCortexStore((s) => s.syncStagesFromColumns);
  const { columns, boards } = useBoardStore();
  const setColumns = useBoardStore((s) => s.setColumns);
  const { t } = useTranslation();
  const [filterQuery, setFilterQuery] = useState('');
  const [openSections, setOpenSections] = useState({ columns: true, agents: true });
  const [openRulesFolders, setOpenRulesFolders] = useState<Record<string, boolean>>({});
  const [openHooksFolders, setOpenHooksFolders] = useState<Record<string, boolean>>({});
  const [creatingStageId, setCreatingStageId] = useState<string | null>(null);
  const [creatingKind, setCreatingKind] = useState<'rule' | 'hook'>('rule');
  const [agentSaving, setAgentSaving] = useState<Record<string, boolean>>({});
  const [agentError, setAgentError] = useState<Record<string, string | null>>({});
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
      await loadAgentPersonas();
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
      for (const c of b.columns) {
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

  const openCreateForm = (stageId: string, kind: 'rule' | 'hook', isExpanded: boolean, expand: () => void) => {
    if (!isExpanded) expand();
    setCreatingKind(kind);
    setCreatingStageId(stageId);
  };
  const searchInputId = useId();

  // Источник стадий — колонки доски (колонка с persona = агент).
  // Мок-стадии из стора остаются только как fallback, когда columns пуст.
  // Свежий проект (только backlog+ready без persona) даёт stages = [] — пустое состояние ниже.
  useEffect(() => {
    syncStagesFromColumns(columns);
  }, [columns, syncStagesFromColumns]);

  // Источник секции агентов — invoke('list_agent_personas'); ошибка → пусто.
  useEffect(() => {
    void loadAgentPersonas();
  }, [loadAgentPersonas]);

  const toggleSection = (key: 'columns' | 'agents') => {
    setOpenSections((prev) => ({ ...prev, [key]: !prev[key] }));
  };

  const toggleRulesFolder = (stageId: string) => {
    setOpenRulesFolders((prev) => ({
      ...prev,
      [stageId]: !(prev[stageId] ?? true),
    }));
  };

  const toggleHooksFolder = (stageId: string) => {
    setOpenHooksFolders((prev) => ({
      ...prev,
      [stageId]: !(prev[stageId] ?? true),
    }));
  };

  const handleOpenFile = (fileId: string) => {
    void openFile(fileId);
  };

  const handleCreatePersona = (fileId: string) => {
    void createPersonaTemplate(fileId);
  };

  const handleCreateSetting = (fileId: string) => {
    void createSettingTemplate(fileId);
  };

  // Смена персоны колонки — флоу ColumnContextMenu: оптимистичный setColumns,
  // save_pipeline, при неудаче откат. Без создания новых персон (только выбор).
  const handleAgentChange = async (columnId: string, rawValue: string) => {
    const nextPersona = rawValue.trim() || null;
    const col = columns.find((c) => c.id === columnId);
    if (!col) return;
    if ((col.persona ?? null) === nextPersona) return;
    setAgentError((prev) => ({ ...prev, [columnId]: null }));
    setAgentSaving((prev) => ({ ...prev, [columnId]: true }));
    const matchedPersona = agentPersonas.find((p) => p.role === nextPersona);
    let nextIcon = col.icon;
    if (nextPersona && matchedPersona?.icon) {
      nextIcon = matchedPersona.icon;
    } else if (!nextPersona) {
      nextIcon = getDefaultColumnIcon(col.id, col.icon);
    }
    // Автоматическое наследование цвета агента колонкой (если не backlog и не ready)
    let nextColor = col.color;
    if (col.id !== 'backlog' && col.id !== 'ready') {
      if (nextPersona) {
        const personaColor = matchedPersona?.color || getDeterministicAgentColor(nextPersona);
        if (personaColor && isAllowedAgentColor(personaColor)) {
          nextColor = personaColor;
        }
      }
    }
    const prev = columns;
    const next = prev.map((c) =>
      c.id === columnId ? { ...c, persona: nextPersona, icon: nextIcon, color: nextColor } : c,
    );
    setColumns(next);
    syncStagesFromColumns(next);
    const saved = await savePipelineConfig(next);
    setAgentSaving((prevMap) => ({ ...prevMap, [columnId]: false }));
    if (!saved) {
      setColumns(prev);
      syncStagesFromColumns(prev);
      setAgentError((prevMap) => ({ ...prevMap, [columnId]: t('columnMenu.saveErrorRollback') }));
    }
  };

  const q = filterQuery.trim().toLowerCase();
  const isSearching = q.length > 0;
  const matchFile = (f: CortexAgentFile) =>
    f.name.toLowerCase().includes(q) || f.path.toLowerCase().includes(q);

  // Секция 1: поиск по названию стадии, персоне и хукам
  // (setting — только для системного backlog-узла).
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

  // Секция 2: поиск по роли и всем файлам агента.
  const filteredAgentStages = agentStages.map((stage) => {
    if (!isSearching) return stage;
    const matchesAgent =
      stage.stageTitle.toLowerCase().includes(q) || stage.personaName.toLowerCase().includes(q);
    if (matchesAgent) return { ...stage, isExpanded: true };
    const matchingFiles = stage.files.filter((f) => matchFile(f));
    if (matchingFiles.length === 0) return null;
    return { ...stage, isExpanded: true, files: matchingFiles };
  }).filter(Boolean) as CortexAgentStage[];

  const sharedFolders = {
    openRulesFolders,
    openHooksFolders,
    creatingStageId,
    creatingKind,
  };

  return (
    <aside className="w-64 shrink-0 h-full flex flex-col bg-[#f9f9f9] dark:bg-[#0d1117] border-r border-[#ebebeb] dark:border-[#21262d] select-none text-[#171717] dark:text-[#f0f6fc]">
      {/* Sidebar Header */}
      <div className="px-3.5 py-2.5 border-b border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#161b22] flex items-center justify-between">
        <span className="text-xs font-semibold text-[#171717] dark:text-[#f0f6fc]">
          {t('cortex.title')}
        </span>
        <button
          type="button"
          onClick={handleRefreshPersonas}
          aria-label={t('cortex.refreshAgents')}
          className="p-1 rounded hover:bg-[#e4e4e7] dark:hover:bg-[#21262d] text-[#71717a] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] transition cursor-pointer"
        >
          <RotateCw className={`h-3.5 w-3.5 ${isRefreshingPersonas ? 'animate-spin' : ''}`} />
        </button>
      </div>

      {/* Filter Search Input */}
      <div className="p-2.5 border-b border-[#ebebeb] dark:border-[#21262d] bg-[#f9f9f9] dark:bg-[#0d1117]">
        <div className="relative flex items-center">
          <Search className="absolute left-2.5 h-3.5 w-3.5 text-[#8f8f8f] dark:text-[#8b949e] pointer-events-none" />
          <input
            id={searchInputId}
            name="cortexMindSearch"
            type="text"
            value={filterQuery}
            onChange={(e) => setFilterQuery(e.target.value)}
            placeholder={t('cortex.searchPlaceholder')}
            className="w-full pl-8 pr-2.5 py-1 text-xs rounded-md bg-white dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] placeholder:text-[#8f8f8f] dark:placeholder:text-[#8b949e] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc] font-sans transition"
          />
        </div>
      </div>

      {/* Sections Tree */}
      <div className="flex-1 min-h-0 overflow-y-auto p-2 space-y-1 font-mono text-xs">
        {/* СЕКЦИЯ 1: Колонки */}
        <section aria-label={t('cortex.columnsSection')}>
          <SectionHeader
            title={t('cortex.columnsSection')}
            count={stages.length}
            open={openSections.columns}
            onToggle={() => toggleSection('columns')}
          />
          {openSections.columns && (
            <div className="space-y-1">
              {stages.length === 0 && (
                <div className="mx-1 mt-1 rounded-md border border-dashed border-[#d4d4d4] dark:border-[#30363d] bg-white dark:bg-[#161b22] px-3 py-4 text-center space-y-1.5">
                  <div className="text-xs font-sans font-semibold text-[#171717] dark:text-[#f0f6fc]">
                    {t('cortex.noAgentsInColumns')}
                  </div>
                  <div className="text-[11px] font-sans text-[#666666] dark:text-[#8b949e] leading-relaxed">
                    {t('cortex.createFirstColumnAgent')}
                  </div>
                  <div className="text-[10px] font-mono text-[#8f8f8f] dark:text-[#8b949e]">
                    {t('cortex.freshProjectNotice')}
                  </div>
                </div>
              )}

              {stages.length > 0 && filteredColumnStages.length === 0 && (
                <div className="mx-1 mt-1 rounded-md border border-[#ebebeb] dark:border-[#30363d] bg-white dark:bg-[#161b22] px-3 py-4 text-center">
                  <div className="text-[11px] font-sans text-[#666666] dark:text-[#8b949e]">
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
                    isBacklogSystem={backlog}
                    column={column}
                    personas={agentPersonas}
                    agentSaving={!!agentSaving[stage.stageId]}
                    agentSaveError={agentError[stage.stageId] ?? null}
                    onAgentChange={handleAgentChange}
                    shared={sharedFolders}
                    activeFileId={activeFile?.id ?? null}
                    fileMissing={fileMissing}
                    isFocused={focusedStageId === stage.stageId}
                    isSearching={isSearching}
                    onToggleStage={() => toggleStageExpand(stage.stageId)}
                    onToggleHooks={() => toggleHooksFolder(stage.stageId)}
                    onOpenFile={handleOpenFile}
                    onCreateSetting={handleCreateSetting}
                    onOpenCreateForm={(kind) =>
                      openCreateForm(stage.stageId, kind, stage.isExpanded, () => toggleStageExpand(stage.stageId))
                    }
                    onCloseCreate={() => setCreatingStageId(null)}
                  />
                );
              })}
            </div>
          )}
        </section>

        {/* СЕКЦИЯ 2: Агенты */}
        <section aria-label={t('cortex.agentsSection')}>
          <SectionHeader
            title={t('cortex.agentsSection')}
            count={agentStages.length}
            open={openSections.agents}
            onToggle={() => toggleSection('agents')}
            action={
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  setIsCreateAgentOpen(true);
                }}
                aria-label={t('cortex.createAgent')}
                className="p-1 rounded-md text-[#8f8f8f] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#ebebeb]/60 dark:hover:bg-[#21262d] transition cursor-pointer"
              >
                <Plus className="h-3.5 w-3.5" />
              </button>
            }
          />
          {openSections.agents && (
            <div className="space-y-1">
              {agentStages.length === 0 && (
                <div className="mx-1 mt-1 rounded-md border border-dashed border-[#d4d4d4] dark:border-[#30363d] bg-white dark:bg-[#161b22] px-3 py-4 text-center space-y-1.5">
                  <div className="text-xs font-sans font-semibold text-[#171717] dark:text-[#f0f6fc]">
                    {t('cortex.noAvailableAgents')}
                  </div>
                  <div className="text-[11px] font-sans text-[#666666] dark:text-[#8b949e] leading-relaxed">
                    {t('cortex.agentsFromProject')}
                  </div>
                </div>
              )}

              {agentStages.length > 0 && filteredAgentStages.length === 0 && (
                <div className="mx-1 mt-1 rounded-md border border-[#ebebeb] dark:border-[#30363d] bg-white dark:bg-[#161b22] px-3 py-4 text-center">
                  <div className="text-[11px] font-sans text-[#666666] dark:text-[#8b949e]">
                    {t('cortex.agentsNothingFound')}
                  </div>
                </div>
              )}

              {filteredAgentStages.map((stage) => (
                <AgentStageCard
                  key={stage.stageId}
                  stage={stage}
                  shared={sharedFolders}
                  activeFileId={activeFile?.id ?? null}
                  fileMissing={fileMissing}
                  isFocused={focusedStageId === stage.stageId}
                  isSearching={isSearching}
                  onToggleStage={() => toggleStageExpand(stage.stageId)}
                  onToggleRules={() => toggleRulesFolder(stage.stageId)}
                  onOpenFile={handleOpenFile}
                  onCreatePersona={handleCreatePersona}
                  onCreateSetting={handleCreateSetting}
                  onOpenCreateForm={(kind) =>
                    openCreateForm(stage.stageId, kind, stage.isExpanded, () => toggleStageExpand(stage.stageId))
                  }
                  onPlusHeader={() => {
                    if (!stage.isExpanded) toggleStageExpand(stage.stageId);
                    setCreatingKind('rule');
                    setCreatingStageId((prev) => (prev === stage.stageId ? null : stage.stageId));
                  }}
                  onCloseCreate={() => setCreatingStageId(null)}
                  onContextMenu={(e) => handleAgentContextMenu(e, stage)}
                />
              ))}
            </div>
          )}
        </section>
      </div>

      {/* Контекстное меню агента (ПКМ) */}
      {agentContextMenu && (
        <div
          className="fixed z-50 min-w-[190px] bg-white dark:bg-[#161b22] rounded-lg shadow-md border border-[#ebebeb] dark:border-[#30363d] p-1 text-xs font-sans animate-in fade-in zoom-in-95 duration-100"
          style={{
            top: Math.min(agentContextMenu.y, window.innerHeight - 120),
            left: Math.min(agentContextMenu.x, window.innerWidth - 210),
          }}
          onClick={(e) => e.stopPropagation()}
          onContextMenu={(e) => e.preventDefault()}
        >
          <div className="px-3 py-1.5 text-[10px] font-semibold text-[#a1a1aa] dark:text-[#8b949e] uppercase tracking-wider border-b border-[#f4f4f5] dark:border-[#21262d] mb-1 truncate">
            {t('cortex.agentContextMenu', { role: agentContextMenu.role })}
          </div>
          <button
            onClick={() => {
              const persona = agentContextMenu.persona;
              setAgentContextMenu(null);
              setEditingAgentPersona(persona);
            }}
            className="w-full flex items-center space-x-2 px-3 py-1.5 text-left transition rounded-sm text-[#171717] dark:text-[#f0f6fc] hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] cursor-pointer"
          >
            <Palette className="h-3.5 w-3.5 text-[#71717a] dark:text-[#8b949e]" />
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
                ? 'text-[#a1a1aa] dark:text-[#6e7681] cursor-not-allowed opacity-50'
                : 'text-[#171717] dark:text-[#f0f6fc] hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] cursor-pointer'
            }`}
            aria-label={agentContextMenu.isBuiltin ? t('cortex.cannotRenameBuiltin') : undefined}
          >
            <Pencil className="h-3.5 w-3.5 text-[#71717a] dark:text-[#8b949e]" />
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
                ? 'text-[#a1a1aa] dark:text-[#6e7681] cursor-not-allowed opacity-50'
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
          handleOpenFile(`agent:${role}-persona`);
        }}
      />
    </aside>
  );
}
