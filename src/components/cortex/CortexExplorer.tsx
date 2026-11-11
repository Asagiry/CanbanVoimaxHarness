import { useState, useId, useEffect, useRef, useCallback } from 'react';
import { useShallow } from 'zustand/react/shallow';
import { useCortexStore } from '../../stores/cortexStore';
import type { AgentPersonaInfo } from '../../stores/cortexStore';
import { useBoardStore } from '../../stores/boardStore';
import { useTranslation } from '../../stores/languageStore';
import { Search, RotateCw, ChevronsUpDown, ChevronsDownUp, X } from 'lucide-react';
import type { CortexAgentStage } from '../../types';
import type { ExpandAllSignal } from './types';
import GlobalRulesSection from './GlobalRulesSection';
import EditAgentPersonaModal from './EditAgentPersonaModal';
import CreateAgentModal from './CreateAgentModal';
import CvhSaveButton from './CvhSaveButton';
import {
  filterColumnStages,
  filterAgentStages,
  filterGlobalRules,
} from './explorer/cortexFilter';
import { ColumnsExplorerSection } from './explorer/ColumnsExplorerSection';
import { AgentsExplorerSection } from './explorer/AgentsExplorerSection';
import { AgentContextMenu, type AgentContextMenuState } from './explorer/AgentContextMenu';

export interface CortexExplorerProps {
  className?: string;
}

export default function CortexExplorer({ className = '' }: CortexExplorerProps = {}) {
  const cortex = useCortexStore(
    useShallow((s) => ({
      stages: s.stages,
      agentStages: s.agentStages,
      agentPersonas: s.agentPersonas,
      globalRules: s.globalRules,
      activeFile: s.activeFile,
      expandStage: s.expandStage,
      setAllStagesExpanded: s.setAllStagesExpanded,
      focusStage: s.focusStage,
      openFile: s.openFile,
      loadAgentPersonas: s.loadAgentPersonas,
      loadGlobalRules: s.loadGlobalRules,
      renameAgentPersona: s.renameAgentPersona,
      deleteAgentPersona: s.deleteAgentPersona,
      setAgentPersonaColor: s.setAgentPersonaColor,
      setAgentPersonaIcon: s.setAgentPersonaIcon,
      syncStagesFromColumns: s.syncStagesFromColumns,
      refreshCvhGitStatus: s.refreshCvhGitStatus,
    }))
  );
  const {
    stages, agentStages, agentPersonas, globalRules, activeFile,
    expandStage, setAllStagesExpanded, focusStage, openFile,
    loadAgentPersonas, loadGlobalRules, renameAgentPersona, deleteAgentPersona,
    setAgentPersonaColor, setAgentPersonaIcon, syncStagesFromColumns, refreshCvhGitStatus,
  } = cortex;
  const { columns, boards } = useBoardStore(
    useShallow((s) => ({ columns: s.columns, boards: s.boards }))
  );
  const { t } = useTranslation();

  const [filterQuery, setFilterQuery] = useState('');
  const [actionError, setActionError] = useState<string | null>(null);
  const [openSections, setOpenSections] = useState({
    columns: true,
    agents: true,
    globalRules: true,
  });
  const [expandAllSignal, setExpandAllSignal] = useState<ExpandAllSignal>({ expanded: true, nonce: 0 });
  const [isRefreshingPersonas, setIsRefreshingPersonas] = useState(false);
  const [isCreateAgentOpen, setIsCreateAgentOpen] = useState(false);
  const [agentContextMenu, setAgentContextMenu] = useState<AgentContextMenuState | null>(null);
  const [editingAgentPersona, setEditingAgentPersona] = useState<AgentPersonaInfo | null>(null);

  const refreshTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const searchInputId = useId();

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

  const handleCloseContextMenu = useCallback(() => {
    setAgentContextMenu(null);
  }, []);

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
    setActionError(null);
    if (role === 'cvh-dev') {
      setActionError(t('cortex.deleteBuiltinForbidden'));
      return;
    }

    const assignedList: string[] = [];
    for (const b of boards) {
      for (const c of b.stages ?? b.columns ?? []) {
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
      setActionError(t('cortex.deleteAgentError', { msg }));
    }
  };

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

  const handleExpandAll = () => {
    setOpenSections({ columns: true, agents: true, globalRules: true });
    setAllStagesExpanded(true);
    setExpandAllSignal((prev) => ({ expanded: true, nonce: prev.nonce + 1 }));
  };

  const handleCollapseAll = () => {
    setOpenSections({ columns: false, agents: false, globalRules: false });
    setAllStagesExpanded(false);
    setExpandAllSignal((prev) => ({ expanded: false, nonce: prev.nonce + 1 }));
  };

  const normalizedQuery = filterQuery.trim().toLowerCase();
  const isSearching = normalizedQuery.length > 0;

  const filteredColumnStages = filterColumnStages(stages, normalizedQuery);
  const filteredAgentStages = filterAgentStages(agentStages, normalizedQuery);
  const filteredGlobalRules = filterGlobalRules(globalRules, normalizedQuery);

  return (
    <aside
      className={`w-64 shrink-0 my-1.5 ml-1.5 mr-1.5 rounded-xl border border-[#ebebeb] dark:border-[#212126] shadow-2xs bg-[#f9f9f9] dark:bg-[#111114] flex flex-col min-h-0 h-[calc(100%-0.75rem)] overflow-hidden select-none text-[#171717] dark:text-[#f4f4f5] transition-colors ${className}`}
    >
      {/* Sidebar Header */}
      <div className="h-12 shrink-0 px-3.5 border-b border-[#ebebeb] dark:border-[#212126] bg-white dark:bg-[#111114] flex items-center justify-between gap-2 transition-colors">
        <span className="text-xs font-semibold text-[#171717] dark:text-[#f4f4f5] min-w-0 truncate">
          {t('cortex.title')}
        </span>
        <div className="flex items-center gap-1 shrink-0">
          <div className="flex items-center border border-[#ebebeb] dark:border-[#2c2c33] rounded bg-white dark:bg-[#17171b]">
            <button
              type="button"
              onClick={handleExpandAll}
              aria-label={t('cortex.expandAllTitle')}
              title={t('cortex.expandAllTitle')}
              className="p-1 text-[#8f8f8f] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] hover:bg-[#f4f4f5] dark:hover:bg-[#232328] transition cursor-pointer"
            >
              <ChevronsUpDown className="h-3 w-3" />
            </button>
            <div className="w-px h-3 bg-[#ebebeb] dark:bg-[#2c2c33]" />
            <button
              type="button"
              onClick={handleCollapseAll}
              aria-label={t('cortex.collapseAllTitle')}
              title={t('cortex.collapseAllTitle')}
              className="p-1 text-[#8f8f8f] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] hover:bg-[#f4f4f5] dark:hover:bg-[#232328] transition cursor-pointer"
            >
              <ChevronsDownUp className="h-3 w-3" />
            </button>
          </div>
          <CvhSaveButton />
          <button
            type="button"
            onClick={handleRefreshPersonas}
            aria-label={t('cortex.refreshAgents')}
            title={t('cortex.refreshAgents')}
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
        {actionError && (
          <div className="px-2 py-1.5 rounded border border-red-200 dark:border-red-900/60 bg-red-50 dark:bg-red-950/30 text-[11px] text-red-700 dark:text-red-400 flex items-center justify-between gap-1 font-sans">
            <span className="truncate">{actionError}</span>
            <button
              type="button"
              onClick={() => setActionError(null)}
              aria-label={t('common.close')}
              className="p-0.5 hover:opacity-75 cursor-pointer shrink-0"
            >
              <X className="h-3 w-3" />
            </button>
          </div>
        )}
        <ColumnsExplorerSection
          stages={filteredColumnStages}
          totalCount={stages.length}
          isOpen={openSections.columns}
          onToggle={() => toggleSection('columns')}
          isSearching={isSearching}
          expandAllSignal={expandAllSignal}
        />

        <AgentsExplorerSection
          stages={filteredAgentStages}
          totalCount={agentStages.length}
          isOpen={openSections.agents}
          onToggle={() => toggleSection('agents')}
          isSearching={isSearching}
          expandAllSignal={expandAllSignal}
          onCreateAgent={() => setIsCreateAgentOpen(true)}
          onAgentContextMenu={handleAgentContextMenu}
        />

        <GlobalRulesSection
          rules={filteredGlobalRules}
          totalCount={globalRules.length}
          isOpen={openSections.globalRules}
          onToggle={() => toggleSection('globalRules')}
          isSearching={isSearching}
          expandAllSignal={expandAllSignal}
        />
      </div>

      {agentContextMenu && (
        <AgentContextMenu
          menu={agentContextMenu}
          onClose={handleCloseContextMenu}
          onEditPersona={(persona) => setEditingAgentPersona(persona)}
          onDeleteAgent={(role) => void handleDeleteAgent(role)}
        />
      )}

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
