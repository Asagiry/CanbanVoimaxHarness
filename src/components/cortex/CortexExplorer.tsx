import { useState, useId, useEffect, useRef } from 'react';
import { useCortexStore, cortexAgentDir, previewCortexFileName } from '../../stores/cortexStore';
import type { CortexHookExt, CortexHookMoment, AgentPersonaInfo } from '../../stores/cortexStore';
import { useBoardStore } from '../../stores/boardStore';
import type { PipelineColumn, PipelineConfig } from '../../types';
import AgentIcon from '../common/AgentIcon';
import PersonaSelect from '../common/PersonaSelect';
import { getDefaultColumnIcon } from '../../lib/columnIcons';
import {
  BACKLOG_COLOR,
  READY_COLOR,
  AGENT_PALETTE,
  AGENT_ICONS,
  getDeterministicAgentColor,
  isAllowedAgentColor,
} from '../../lib/agentPalette';
import {
  ChevronDown,
  ChevronRight,
  Folder,
  FolderOpen,
  FileText,
  Settings,
  Search,
  User,
  Plus,
  Zap,
  Cpu,
  Pencil,
  Trash2,
  X,
  RotateCw,
  Palette,
  Check,
} from 'lucide-react';
import type { CortexAgentFile, CortexAgentStage } from '../../types';

function hookMomentOf(fileName: string): CortexHookMoment | null {
  if (fileName.startsWith('on_enter')) return 'on_enter';
  if (fileName.startsWith('on_finish')) return 'on_finish';
  return null;
}

// Системный backlog-узел («система») — исключение секции колонок: без поля АГЕНТ,
// рендерится как раньше (хуки колонки + setting).
function isBacklogSystemStage(stage: CortexAgentStage): boolean {
  if (stage.stageId !== 'backlog') return false;
  const persona = (stage.personaName ?? '').trim();
  return persona === 'система' || persona.toLowerCase() === 'system';
}

// Сохранение пайплайна — тот же флоу, что в ColumnContextMenu:
// оптимистичный setColumns + save_pipeline, при ошибке откат вызывает caller.
async function savePipelineConfig(next: PipelineColumn[]): Promise<boolean> {
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
}

export default function CortexExplorer() {
  const {
    stages,
    agentStages,
    agentPersonas,
    activeFile,
    focusedStageId,
    toggleStageExpand,
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
  const [filterQuery, setFilterQuery] = useState('');
  const [openSections, setOpenSections] = useState({ columns: true, agents: true });
  const [openRulesFolders, setOpenRulesFolders] = useState<Record<string, boolean>>({});
  const [openHooksFolders, setOpenHooksFolders] = useState<Record<string, boolean>>({});
  const [creatingStageId, setCreatingStageId] = useState<string | null>(null);
  const [creatingKind, setCreatingKind] = useState<'rule' | 'hook'>('rule');
  const [agentSaving, setAgentSaving] = useState<Record<string, boolean>>({});
  const [agentError, setAgentError] = useState<Record<string, string | null>>({});
  const [isRefreshingPersonas, setIsRefreshingPersonas] = useState(false);

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
      alert("Запрещено удалять встроенную персону 'cvh-dev'");
      return;
    }

    const assignedList: string[] = [];
    for (const b of boards) {
      for (const c of b.columns) {
        if (c.persona === role) {
          assignedList.push(`Доска «${b.name}» → колонка «${c.title}»`);
        }
      }
    }

    let confirmMsg = `Удалить агента «${role}»?`;
    if (assignedList.length > 0) {
      confirmMsg += `\n\nВнимание: агент назначен на колонки:\n- ${assignedList.join('\n- ')}\n\nНазначение персоны у этих колонок будет сброшено.`;
    }

    if (!window.confirm(confirmMsg)) {
      return;
    }

    try {
      await deleteAgentPersona(role);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      alert(`Ошибка при удалении агента: ${msg}`);
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
      setAgentError((prevMap) => ({ ...prevMap, [columnId]: 'Не удалось сохранить. Изменения откачены.' }));
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
          Проводник разума агента
        </span>
        <button
          type="button"
          onClick={handleRefreshPersonas}
          title="Обновить агентов"
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
            placeholder="Поиск колонок, агентов, файлов..."
            className="w-full pl-8 pr-2.5 py-1 text-xs rounded-md bg-white dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] placeholder:text-[#8f8f8f] dark:placeholder:text-[#8b949e] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc] font-sans transition"
          />
        </div>
      </div>

      {/* Sections Tree */}
      <div className="flex-1 min-h-0 overflow-y-auto p-2 space-y-1 font-mono text-xs">
        {/* СЕКЦИЯ 1: Колонки */}
        <section aria-label="Колонки">
          <SectionHeader
            title="Колонки"
            count={stages.length}
            open={openSections.columns}
            onToggle={() => toggleSection('columns')}
          />
          {openSections.columns && (
            <div className="space-y-1">
              {stages.length === 0 && (
                <div className="mx-1 mt-1 rounded-md border border-dashed border-[#d4d4d4] dark:border-[#30363d] bg-white dark:bg-[#161b22] px-3 py-4 text-center space-y-1.5">
                  <div className="text-xs font-sans font-semibold text-[#171717] dark:text-[#f0f6fc]">
                    Нет агентов
                  </div>
                  <div className="text-[11px] font-sans text-[#666666] dark:text-[#8b949e] leading-relaxed">
                    Создайте первую колонку-агента через + Колонка
                  </div>
                  <div className="text-[10px] font-mono text-[#8f8f8f] dark:text-[#8b949e]">
                    Свежий проект стартует только с backlog и ready
                  </div>
                </div>
              )}

              {stages.length > 0 && filteredColumnStages.length === 0 && (
                <div className="mx-1 mt-1 rounded-md border border-[#ebebeb] dark:border-[#30363d] bg-white dark:bg-[#161b22] px-3 py-4 text-center">
                  <div className="text-[11px] font-sans text-[#666666] dark:text-[#8b949e]">
                    В колонках ничего не найдено
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
        <section aria-label="Агенты">
          <SectionHeader
            title="Агенты"
            count={agentStages.length}
            open={openSections.agents}
            onToggle={() => toggleSection('agents')}
          />
          {openSections.agents && (
            <div className="space-y-1">
              {agentStages.length === 0 && (
                <div className="mx-1 mt-1 rounded-md border border-dashed border-[#d4d4d4] dark:border-[#30363d] bg-white dark:bg-[#161b22] px-3 py-4 text-center space-y-1.5">
                  <div className="text-xs font-sans font-semibold text-[#171717] dark:text-[#f0f6fc]">
                    Нет доступных агентов
                  </div>
                  <div className="text-[11px] font-sans text-[#666666] dark:text-[#8b949e] leading-relaxed">
                    Агенты берутся из списка персон проекта
                  </div>
                </div>
              )}

              {agentStages.length > 0 && filteredAgentStages.length === 0 && (
                <div className="mx-1 mt-1 rounded-md border border-[#ebebeb] dark:border-[#30363d] bg-white dark:bg-[#161b22] px-3 py-4 text-center">
                  <div className="text-[11px] font-sans text-[#666666] dark:text-[#8b949e]">
                    Среди агентов ничего не найдено
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
          className="fixed z-50 min-w-[190px] bg-white dark:bg-[#161b22] rounded-lg shadow-xl border border-[#ebebeb] dark:border-[#30363d] p-1 text-xs font-sans animate-in fade-in zoom-in-95 duration-100"
          style={{
            top: Math.min(agentContextMenu.y, window.innerHeight - 120),
            left: Math.min(agentContextMenu.x, window.innerWidth - 210),
          }}
          onClick={(e) => e.stopPropagation()}
          onContextMenu={(e) => e.preventDefault()}
        >
          <div className="px-3 py-1.5 text-[10px] font-semibold text-[#a1a1aa] dark:text-[#8b949e] uppercase tracking-wider border-b border-[#f4f4f5] dark:border-[#21262d] mb-1 truncate">
            Агент: {agentContextMenu.role}
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
            <span>Настроить агента (цвет и иконка)</span>
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
            title={agentContextMenu.isBuiltin ? 'Нельзя переименовать встроенного агента' : undefined}
          >
            <Pencil className="h-3.5 w-3.5 text-[#71717a] dark:text-[#8b949e]" />
            <span>Переименовать агента</span>
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
            title={agentContextMenu.isBuiltin ? 'Нельзя удалить встроенного агента' : undefined}
          >
            <Trash2 className="h-3.5 w-3.5" />
            <span>Удалить агента</span>
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
    </aside>
  );
}

function SectionHeader({
  title,
  count,
  open,
  onToggle,
  action,
}: {
  title: string;
  count: number;
  open: boolean;
  onToggle: () => void;
  action?: React.ReactNode;
}) {
  return (
    <div className="w-full flex items-center justify-between px-1.5 py-1 rounded-md text-[#171717] dark:text-[#f0f6fc] hover:bg-[#f4f4f5] dark:hover:bg-[#161b22] transition group">
      <button
        onClick={onToggle}
        aria-expanded={open}
        className="flex-1 flex items-center gap-1.5 py-1 text-left cursor-pointer"
      >
        {open ? (
          <ChevronDown className="h-3.5 w-3.5 text-[#666666] dark:text-[#8b949e] shrink-0" />
        ) : (
          <ChevronRight className="h-3.5 w-3.5 text-[#666666] dark:text-[#8b949e] shrink-0" />
        )}
        <span className="font-sans text-xs font-semibold">{title}</span>
        <span className="text-[10px] font-mono text-[#8f8f8f] dark:text-[#8b949e] tabular-nums">({count})</span>
      </button>
      {action && (
        <div className="shrink-0 flex items-center" onClick={(e) => e.stopPropagation()}>
          {action}
        </div>
      )}
    </div>
  );
}

interface SharedFolderState {
  openRulesFolders: Record<string, boolean>;
  openHooksFolders: Record<string, boolean>;
  creatingStageId: string | null;
  creatingKind: 'rule' | 'hook';
}

function StageHeaderButton({
  stage,
  hint,
  overrideIcon,
  overrideColor,
  onToggle,
  onPlus,
}: {
  stage: CortexAgentStage;
  hint: string;
  overrideIcon?: string;
  overrideColor?: string;
  onToggle: () => void;
  onPlus?: () => void;
}) {
  const isBuiltin = stage.personaName === 'cvh-dev' || Boolean(stage.isBuiltin);
  const isBacklog = stage.stageId === 'backlog';
  const isReady = stage.stageId === 'ready';
  const iconName = overrideIcon || stage.icon || getDefaultColumnIcon(stage.stageId);
  const stageColor = isBacklog
    ? BACKLOG_COLOR
    : isReady
    ? READY_COLOR
    : (overrideColor || stage.color || (stage.personaName ? getDeterministicAgentColor(stage.personaName) : '#171717'));

  return (
    <div className="group flex items-center gap-0.5 px-1">
      <button
        onClick={onToggle}
        className="flex-1 flex items-center justify-between px-1.5 py-1.5 rounded-md text-left transition cursor-pointer min-w-0"
      >
        <div className="flex items-center space-x-2 min-w-0">
          {stage.isExpanded ? (
            <ChevronDown className="h-3.5 w-3.5 text-[#666666] dark:text-[#8b949e] shrink-0" />
          ) : (
            <ChevronRight className="h-3.5 w-3.5 text-[#666666] dark:text-[#8b949e] shrink-0" />
          )}

          {iconName ? (
            <AgentIcon
              name={iconName}
              className={`h-3.5 w-3.5 shrink-0 ${isBuiltin ? 'text-indigo-600' : ''}`}
              style={!isBuiltin ? { color: stageColor } : undefined}
            />
          ) : isBuiltin ? (
            <Cpu className="h-3.5 w-3.5 text-indigo-600 shrink-0" />
          ) : (
            <span
              className="w-2 h-2 rounded-full shrink-0"
              style={{ backgroundColor: stageColor }}
            />
          )}

          <span className="font-sans text-xs font-semibold text-[#171717] dark:text-[#f0f6fc] truncate">
            {stage.stageTitle}
          </span>

          {isBuiltin && (
            <span className="px-1.5 py-0.5 rounded text-[10px] font-sans font-medium bg-indigo-50 dark:bg-indigo-950/50 text-indigo-700 dark:text-indigo-400 border border-indigo-200 dark:border-indigo-800 shrink-0 self-center translate-y-[0.5px] leading-tight">
              Встроенный
            </span>
          )}
        </div>

        {hint !== '' && !isBuiltin && (
          <span className="text-[10px] font-mono text-[#666666] dark:text-[#8b949e] truncate ml-1">
            {hint}
          </span>
        )}
      </button>

      {onPlus && (
        <button
          onClick={(e) => {
            e.stopPropagation();
            onPlus();
          }}
          title="Новое правило"
          aria-label="Новое правило"
          className="p-1 rounded-md text-[#8f8f8f] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#ebebeb]/60 dark:hover:bg-[#21262d] transition cursor-pointer opacity-0 group-hover:opacity-100 focus:opacity-100 focus-visible:opacity-100 shrink-0"
        >
          <Plus className="h-3.5 w-3.5" />
        </button>
      )}
    </div>
  );
}

// Секция 1: стадия колонки — только хуки (правил и setting нет,
// кроме системного backlog-узла, который рендерится как раньше).
// Кнопки + на заголовке стадии нет: создание только через + папки Хуки.
// Поле АГЕНТ — только у колонок с persona, внутри сворачиваемого контента;
// смена агента у остальных — через ПКМ-меню колонки на доске.
function ColumnStageCard({
  stage,
  isBacklogSystem,
  column,
  personas,
  agentSaving,
  agentSaveError,
  onAgentChange,
  shared,
  activeFileId,
  fileMissing,
  isFocused,
  isSearching,
  onToggleStage,
  onToggleHooks,
  onOpenFile,
  onCreateSetting,
  onOpenCreateForm,
  onCloseCreate,
}: {
  stage: CortexAgentStage;
  isBacklogSystem: boolean;
  column: PipelineColumn | undefined;
  personas: AgentPersonaInfo[];
  agentSaving: boolean;
  agentSaveError: string | null;
  onAgentChange: (columnId: string, value: string) => void;
  shared: SharedFolderState;
  activeFileId: string | null;
  fileMissing: Record<string, boolean>;
  isFocused: boolean;
  isSearching: boolean;
  onToggleStage: () => void;
  onToggleHooks: () => void;
  onOpenFile: (fileId: string) => void;
  onCreateSetting: (fileId: string) => void;
  onOpenCreateForm: (kind: 'rule' | 'hook') => void;
  onCloseCreate: () => void;
}) {
  const hookFiles = stage.files.filter((f) => f.type === 'hook');
  const settingsFile = stage.files.find((f) => f.type === 'settings');
  const areHooksOpen = shared.openHooksFolders[stage.stageId] ?? true;
  const settingsIsMissing = settingsFile ? !!fileMissing[settingsFile.id] : false;

  const isBacklog = stage.stageId === 'backlog';
  const isReady = stage.stageId === 'ready';
  const assignedPersona = column?.persona ? personas.find((p) => p.role === column.persona) : undefined;
  const effectiveIcon = isBacklog
    ? (column?.icon || stage.icon || getDefaultColumnIcon('backlog'))
    : isReady
    ? (column?.icon || stage.icon || getDefaultColumnIcon('ready'))
    : (assignedPersona?.icon || column?.icon || stage.icon || getDefaultColumnIcon(stage.stageId));
  const effectiveColor = isBacklog
    ? BACKLOG_COLOR
    : isReady
    ? READY_COLOR
    : (assignedPersona?.color || (column?.color && isAllowedAgentColor(column.color) ? column.color : (column?.persona ? getDeterministicAgentColor(column.persona) : stage.color)));

  return (
    <div
      id={`cortex-stage-${stage.stageId}`}
      className={`rounded-md transition-colors ${
        isFocused
          ? 'bg-white dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] shadow-xs'
          : 'hover:bg-[#f4f4f5] dark:hover:bg-[#161b22]'
      }`}
    >
      <StageHeaderButton
        stage={stage}
        hint={stage.personaName}
        overrideIcon={effectiveIcon}
        overrideColor={effectiveColor}
        onToggle={onToggleStage}
      />

      {/* Инлайн-форма создания файла новой схемы */}
      {shared.creatingStageId === stage.stageId && (
        <NewCortexFileForm
          stage={stage}
          initialKind={shared.creatingKind}
          fixedKind={isBacklogSystem ? undefined : 'hook'}
          onClose={onCloseCreate}
        />
      )}

      {/* Children */}
      {stage.isExpanded && (
        <div className="pl-4 pr-1 py-0.5 space-y-0.5 border-l border-[#ebebeb] dark:border-[#21262d] ml-3 my-0.5">
          {!isBacklogSystem && !isReady && !column?.terminal && column && (
            <AgentField
              columnId={column.id}
              current={column.persona}
              personas={personas}
              saving={agentSaving}
              error={agentSaveError}
              onChange={onAgentChange}
            />
          )}
          <HooksFolderBlock
            files={hookFiles}
            isOpen={areHooksOpen}
            isSearching={isSearching}
            activeFileId={activeFileId}
            onToggle={onToggleHooks}
            onNew={() => onOpenCreateForm('hook')}
            onOpen={onOpenFile}
          />

          {isBacklogSystem && settingsFile && !settingsIsMissing && (
            <FileTreeItem
              file={settingsFile}
              displayName="setting"
              isActive={activeFileId === settingsFile.id}
              icon={<Settings className="h-3.5 w-3.5 text-[#666666] shrink-0" />}
              onSelect={() => onOpenFile(settingsFile.id)}
            />
          )}

          {isBacklogSystem && settingsFile && settingsIsMissing && (
            <SettingMissingBlock onCreate={() => onCreateSetting(settingsFile.id)} />
          )}
        </div>
      )}
    </div>
  );
}

// Секция 2: агент целиком — agent.md, rules/, setting (хуки строго в колонках).
function AgentStageCard({
  stage,
  shared,
  activeFileId,
  fileMissing,
  isFocused,
  isSearching,
  onToggleStage,
  onToggleRules,
  onOpenFile,
  onCreatePersona,
  onCreateSetting,
  onOpenCreateForm,
  onPlusHeader,
  onCloseCreate,
  onContextMenu,
}: {
  stage: CortexAgentStage;
  shared: SharedFolderState;
  activeFileId: string | null;
  fileMissing: Record<string, boolean>;
  isFocused: boolean;
  isSearching: boolean;
  onToggleStage: () => void;
  onToggleRules: () => void;
  onOpenFile: (fileId: string) => void;
  onCreatePersona: (fileId: string) => void;
  onCreateSetting: (fileId: string) => void;
  onOpenCreateForm: (kind: 'rule') => void;
  onPlusHeader: () => void;
  onCloseCreate: () => void;
  onContextMenu: (e: React.MouseEvent) => void;
}) {
  const personaFile = stage.files.find((f) => f.type === 'persona');
  const ruleFiles = stage.files.filter((f) => f.type === 'rule');
  const settingsFile = stage.files.find((f) => f.type === 'settings');
  const areRulesOpen = shared.openRulesFolders[stage.stageId] ?? true;
  const personaIsMissing = personaFile ? !!fileMissing[personaFile.id] : false;
  const settingsIsMissing = settingsFile ? !!fileMissing[settingsFile.id] : false;
  const isBuiltin = stage.personaName === 'cvh-dev' || Boolean(stage.isBuiltin);

  return (
    <div
      id={`cortex-agent-${stage.personaName}`}
      onContextMenu={onContextMenu}
      className={`rounded-md transition-colors ${
        isFocused
          ? 'bg-white dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] shadow-xs'
          : 'hover:bg-[#f4f4f5] dark:hover:bg-[#161b22]'
      }`}
    >
      <StageHeaderButton
        stage={stage}
        hint=""
        overrideIcon={stage.icon}
        overrideColor={stage.color || getDeterministicAgentColor(stage.personaName)}
        onToggle={onToggleStage}
        onPlus={onPlusHeader}
      />

      {/* Инлайн-форма создания файла новой схемы (у агентов только правила) */}
      {shared.creatingStageId === stage.stageId && (
        <NewCortexFileForm
          stage={stage}
          initialKind="rule"
          fixedKind="rule"
          onClose={onCloseCreate}
        />
      )}

      {/* Children */}
      {stage.isExpanded && (
        <div className="pl-4 pr-1 py-0.5 space-y-0.5 border-l border-[#ebebeb] dark:border-[#21262d] ml-3 my-0.5">
          {/* 1. Персона: .cvh/agents/<role>/persona.md */}
          {personaFile && !personaIsMissing && (
            <FileTreeItem
              file={personaFile}
              displayName={personaFile.name}
              isActive={activeFileId === personaFile.id}
              icon={
                isBuiltin ? (
                  <Cpu className="h-3.5 w-3.5 text-indigo-600 shrink-0" />
                ) : (
                  <User className="h-3.5 w-3.5 text-[#666666] shrink-0" />
                )
              }
              onSelect={() => onOpenFile(personaFile.id)}
            />
          )}

          {/* Персона отсутствует на диске: плейсхолдер + создание шаблона */}
          {personaFile && personaIsMissing && (
            <div className="px-2 py-1.5 rounded-md border border-dashed border-[#d4d4d4] dark:border-[#30363d] bg-white dark:bg-[#161b22] space-y-1">
              <div className="text-[11px] font-sans text-[#666666] dark:text-[#8b949e]">
                Персона не настроена — нет файла
              </div>
              <button
                onClick={() => onCreatePersona(personaFile.id)}
                className="flex items-center space-x-1 px-2 py-1 rounded-md text-[11px] font-mono bg-[#171717] dark:bg-[#21262d] hover:bg-[#262626] dark:hover:bg-[#30363d] text-white dark:text-[#f0f6fc] transition cursor-pointer"
                title="Создать шаблон персоны"
              >
                <Plus className="h-3 w-3 shrink-0" />
                <span>Создать шаблон</span>
              </button>
            </div>
          )}

          {/* 2. Правила: .cvh/agents/<role>/rules/ */}
          <RulesFolderBlock
            files={ruleFiles}
            isOpen={areRulesOpen}
            isSearching={isSearching}
            activeFileId={activeFileId}
            onToggle={onToggleRules}
            onNew={() => onOpenCreateForm('rule')}
            onOpen={onOpenFile}
          />

          {/* 3. Настройки: .cvh/agents/<role>/setting */}
          {settingsFile && !settingsIsMissing && (
            <FileTreeItem
              file={settingsFile}
              displayName="setting"
              isActive={activeFileId === settingsFile.id}
              icon={<Settings className="h-3.5 w-3.5 text-[#666666] shrink-0" />}
              onSelect={() => onOpenFile(settingsFile.id)}
            />
          )}

          {settingsFile && settingsIsMissing && (
            <SettingMissingBlock onCreate={() => onCreateSetting(settingsFile.id)} />
          )}
        </div>
      )}
    </div>
  );
}

// Поле АГЕНТ колонки: select из доступных персон, при пустом списке — свободный ввод.
// Сохранение — через флоу setColumns + save_pipeline (см. handleAgentChange).
function AgentField({
  columnId,
  current,
  personas,
  saving,
  error,
  onChange,
}: {
  columnId: string;
  current: string | null;
  personas: AgentPersonaInfo[];
  saving: boolean;
  error: string | null;
  onChange: (columnId: string, value: string) => void;
}) {
  const currentValue = current ?? '';
  const [draft, setDraft] = useState(currentValue);

  useEffect(() => {
    setDraft(current ?? '');
  }, [current]);

  return (
    <div className="px-2 py-1">
      <div className="flex items-center gap-1.5">
        <span className="text-[10px] font-sans font-semibold uppercase tracking-wide text-[#8f8f8f] shrink-0">
          Агент
        </span>
        {personas.length > 0 ? (
          <PersonaSelect
            value={currentValue}
            disabled={saving}
            onChange={(role) => onChange(columnId, role)}
            allowEmpty={true}
            emptyLabel="без агента"
            personas={personas}
            className="flex-1 min-w-0"
          />
        ) : (
          <input
            type="text"
            value={draft}
            disabled={saving}
            onChange={(e) => setDraft(e.target.value)}
            onBlur={() => {
              if (draft !== currentValue) onChange(columnId, draft);
            }}
            onKeyDown={(e) => {
              if (e.key === 'Enter') (e.target as HTMLInputElement).blur();
              if (e.key === 'Escape') {
                setDraft(currentValue);
                (e.target as HTMLInputElement).blur();
              }
            }}
            placeholder="developer"
            title="Агент колонки"
            className="flex-1 min-w-0 px-1.5 py-0.5 rounded-md bg-white dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] font-mono text-[11px] placeholder:text-[#8f8f8f] dark:placeholder:text-[#8b949e] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc] transition disabled:opacity-50"
          />
        )}
        {saving && (
          <span className="text-[10px] font-mono text-[#8f8f8f] shrink-0">...</span>
        )}
      </div>
      {error && (
        <div className="mt-1 text-[11px] font-sans text-red-600">{error}</div>
      )}
    </div>
  );
}

function RulesFolderBlock({
  files,
  isOpen,
  isSearching,
  activeFileId,
  onToggle,
  onNew,
  onOpen,
}: {
  files: CortexAgentFile[];
  isOpen: boolean;
  isSearching: boolean;
  activeFileId: string | null;
  onToggle: () => void;
  onNew: () => void;
  onOpen: (fileId: string) => void;
}) {
  if (files.length === 0 && isSearching) return null;
  return (
    <div className="space-y-0.5">
      <div className="group/folder flex items-center gap-0.5">
        <button
          onClick={onToggle}
          className="flex-1 min-w-0 flex items-center space-x-1.5 px-2 py-1 rounded-md text-left text-[#666666] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#ebebeb]/60 dark:hover:bg-[#21262d] transition cursor-pointer"
        >
          {isOpen ? (
            <FolderOpen className="h-3.5 w-3.5 text-[#666666] dark:text-[#8b949e] shrink-0" />
          ) : (
            <Folder className="h-3.5 w-3.5 text-[#666666] dark:text-[#8b949e] shrink-0" />
          )}
          <span className="text-xs font-mono">Правила</span>
          <span className="text-[10px] font-mono text-[#8f8f8f]">rules/</span>
          <span className="text-[10px] text-[#8f8f8f] tabular-nums">({files.length})</span>
        </button>
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            onNew();
          }}
          title="Новое правило"
          aria-label="Новое правило"
          className="p-1 rounded-md text-[#8f8f8f] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#ebebeb]/60 dark:hover:bg-[#21262d] transition cursor-pointer opacity-0 group-hover/folder:opacity-100 focus:opacity-100 focus-visible:opacity-100 shrink-0"
        >
          <Plus className="h-3 w-3" />
        </button>
      </div>

      {isOpen && files.length > 0 && (
        <div className="pl-4 space-y-0.5 border-l border-[#ebebeb] dark:border-[#21262d] ml-2">
          {files.map((ruleFile) => (
            <FileTreeItem
              key={ruleFile.id}
              file={ruleFile}
              displayName={ruleFile.name}
              isActive={activeFileId === ruleFile.id}
              icon={<FileText className="h-3.5 w-3.5 text-[#666666] dark:text-[#8b949e] shrink-0" />}
              onSelect={() => onOpen(ruleFile.id)}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function HooksFolderBlock({
  files,
  isOpen,
  isSearching,
  activeFileId,
  onToggle,
  onNew,
  onOpen,
}: {
  files: CortexAgentFile[];
  isOpen: boolean;
  isSearching: boolean;
  activeFileId: string | null;
  onToggle: () => void;
  onNew?: () => void;
  onOpen: (fileId: string) => void;
}) {
  if (files.length === 0 && isSearching) return null;
  return (
    <div className="space-y-0.5">
      <div className="group/folder flex items-center gap-0.5">
        <button
          onClick={onToggle}
          className="flex-1 min-w-0 flex items-center space-x-1.5 px-2 py-1 rounded-md text-left text-[#666666] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#ebebeb]/60 dark:hover:bg-[#21262d] transition cursor-pointer"
        >
          {isOpen ? (
            <FolderOpen className="h-3.5 w-3.5 text-[#666666] dark:text-[#8b949e] shrink-0" />
          ) : (
            <Folder className="h-3.5 w-3.5 text-[#666666] dark:text-[#8b949e] shrink-0" />
          )}
          <span className="text-xs font-mono">Хуки</span>
          <span className="text-[10px] font-mono text-[#8f8f8f]">hooks/</span>
          <span className="text-[10px] text-[#8f8f8f] tabular-nums">({files.length})</span>
        </button>
        {onNew && (
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onNew();
            }}
            title="Новый хук"
            aria-label="Новый хук"
            className="p-1 rounded-md text-[#8f8f8f] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#ebebeb]/60 dark:hover:bg-[#21262d] transition cursor-pointer opacity-0 group-hover/folder:opacity-100 focus:opacity-100 focus-visible:opacity-100 shrink-0"
          >
            <Plus className="h-3 w-3" />
          </button>
        )}
      </div>

      {isOpen && files.length > 0 && (
        <div className="pl-4 space-y-0.5 border-l border-[#ebebeb] dark:border-[#21262d] ml-2">
          {files.map((hookFile) => {
            const isBuiltin = hookFile.path === 'builtin:create_worktree' || hookFile.path.startsWith('builtin:');
            return (
              <FileTreeItem
                key={hookFile.id}
                file={hookFile}
                displayName={hookFile.name}
                isActive={activeFileId === hookFile.id}
                badge={isBuiltin ? 'встроенный' : hookMomentOf(hookFile.name)}
                icon={<Zap className="h-3.5 w-3.5 text-[#666666] dark:text-[#8b949e] shrink-0" />}
                onSelect={() => onOpen(hookFile.id)}
              />
            );
          })}
        </div>
      )}
    </div>
  );
}

function SettingMissingBlock({
  onCreate,
}: {
  onCreate: () => void;
}) {
  return (
    <div className="px-2 py-1.5 rounded-md border border-dashed border-[#d4d4d4] dark:border-[#30363d] bg-white dark:bg-[#161b22] space-y-1">
      <div className="text-[11px] font-sans text-[#666666] dark:text-[#8b949e]">
        Настройки не заданы — нет файла
      </div>
      <button
        onClick={onCreate}
        className="flex items-center space-x-1 px-2 py-1 rounded-md text-[11px] font-mono bg-[#171717] dark:bg-[#21262d] hover:bg-[#262626] dark:hover:bg-[#30363d] text-white dark:text-[#f0f6fc] transition cursor-pointer"
        title="Создать шаблон настроек"
      >
        <Plus className="h-3 w-3 shrink-0" />
        <span>Создать шаблон</span>
      </button>
    </div>
  );
}

function NewCortexFileForm({
  stage,
  onClose,
  initialKind = 'rule',
  fixedKind,
}: {
  stage: CortexAgentStage;
  onClose: () => void;
  initialKind?: 'rule' | 'hook';
  // Секция колонок создаёт только хуки: fixedKind прячет переключатель вида.
  fixedKind?: 'rule' | 'hook';
}) {
  const createCortexFile = useCortexStore((s) => s.createCortexFile);
  const [rawName, setRawName] = useState('');
  const [kind, setKind] = useState<'rule' | 'hook'>(fixedKind ?? initialKind);
  const [hookMoment, setHookMoment] = useState<CortexHookMoment>('on_enter');
  const [hookExt, setHookExt] = useState<CortexHookExt>('bat');
  const [error, setError] = useState<string | null>(null);
  const [isSaving, setIsSaving] = useState(false);

  const agentDir = cortexAgentDir(stage.personaName, stage.stageId);
  const targetDir = kind === 'rule' ? `${agentDir}/rules/` : `${agentDir}/hooks/`;
  const previewName =
    rawName.trim().length === 0
      ? kind === 'rule'
        ? 'имя-правила.md'
        : `${hookMoment}.${hookExt}`
      : previewCortexFileName(rawName, kind, { hookMoment, hookExt });

  const handleSubmit = async () => {
    try {
      if (kind === 'rule') {
        const trimmed = rawName.trim();
        const base = /\.md$/i.test(trimmed) ? trimmed.slice(0, -3).trim() : trimmed;
        if (!base) {
          setError('Введите имя файла');
          return;
        }
        if (!/^[A-Za-z0-9-_]+$/.test(base)) {
          setError('Только латиница, цифры, - и _');
          return;
        }
        const fileName = `${base}.md`;
        const duplicate = stage.files.some(
          (f) => f.name.toLowerCase() === fileName.toLowerCase()
        );
        if (duplicate) {
          setError(`Файл ${fileName} уже есть в стадии`);
          return;
        }
      } else {
        const typed = rawName.trim().replace(/\.[A-Za-z0-9]+$/, '').trim();
        if (typed && !/^[A-Za-z0-9-_]+$/.test(typed)) {
          setError('Только латиница, цифры, - и _');
          return;
        }
        const fileName = previewCortexFileName(rawName, 'hook', { hookMoment, hookExt });
        const duplicate = stage.files.some(
          (f) => f.name.toLowerCase() === fileName.toLowerCase()
        );
        if (duplicate) {
          setError(`Файл ${fileName} уже есть в стадии`);
          return;
        }
      }
      setError(null);
      setIsSaving(true);
      await createCortexFile(
        stage.stageId,
        rawName,
        kind,
        kind === 'hook' ? { hookMoment, hookExt } : undefined,
      );
      onClose();
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message || 'Не удалось создать файл');
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <div className="mx-1 mb-1 rounded-md border border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#161b22] px-2 py-2 space-y-2">
      <div className="text-[11px] font-sans font-semibold text-[#171717] dark:text-[#f0f6fc]">
        Новый файл агента
      </div>

      <input
        type="text"
        value={rawName}
        onChange={(e) => {
          setRawName(e.target.value);
          if (error) setError(null);
        }}
        onKeyDown={(e) => {
          if (e.key === 'Enter') void handleSubmit();
          if (e.key === 'Escape') onClose();
        }}
        placeholder={kind === 'rule' ? 'имя-правила' : 'имя-хука (можно пусто)'}
        autoFocus
        className="w-full px-2 py-1 text-xs rounded-md bg-white dark:bg-[#0d1117] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] font-mono placeholder:text-[#8f8f8f] dark:placeholder:text-[#8b949e] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc] transition"
      />
      <div className="text-[10px] font-mono text-[#8f8f8f] dark:text-[#8b949e]">
        Латиница, цифры, - и _ (расширение добавится автоматически)
      </div>

      {!fixedKind && (
        <div className="flex items-center gap-3 text-[11px] font-mono text-[#4d4d4d] dark:text-[#c9d1d9]">
          <label className="flex items-center gap-1 cursor-pointer">
            <input
              type="radio"
              name={`cortex-kind-${stage.stageId}`}
              checked={kind === 'rule'}
              onChange={() => setKind('rule')}
              className="accent-[#171717] dark:accent-[#f0f6fc]"
            />
            <span>Правило</span>
          </label>
          <label className="flex items-center gap-1 cursor-pointer">
            <input
              type="radio"
              name={`cortex-kind-${stage.stageId}`}
              checked={kind === 'hook'}
              onChange={() => setKind('hook')}
              className="accent-[#171717] dark:accent-[#f0f6fc]"
            />
            <span>Хук</span>
          </label>
        </div>
      )}

      {kind === 'hook' && (
        <div className="space-y-1.5 rounded-md bg-[#f9f9f9] dark:bg-[#0d1117] border border-[#ebebeb] dark:border-[#30363d] px-2 py-1.5">
          <div className="flex items-center gap-3 text-[11px] font-mono text-[#4d4d4d] dark:text-[#c9d1d9]">
            <label className="flex items-center gap-1 cursor-pointer">
              <input
                type="radio"
                name={`cortex-moment-${stage.stageId}`}
                checked={hookMoment === 'on_enter'}
                onChange={() => setHookMoment('on_enter')}
                className="accent-[#171717] dark:accent-[#f0f6fc]"
              />
              <span>on_enter</span>
            </label>
            <label className="flex items-center gap-1 cursor-pointer">
              <input
                type="radio"
                name={`cortex-moment-${stage.stageId}`}
                checked={hookMoment === 'on_finish'}
                onChange={() => setHookMoment('on_finish')}
                className="accent-[#171717] dark:accent-[#f0f6fc]"
              />
              <span>on_finish</span>
            </label>
          </div>
          <label className="flex items-center gap-1.5 text-[11px] font-mono text-[#4d4d4d] dark:text-[#c9d1d9]">
            <span>Тип:</span>
            <select
              value={hookExt}
              onChange={(e) => setHookExt(e.target.value as CortexHookExt)}
              className="px-1.5 py-0.5 rounded-md bg-white dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] font-mono text-[11px] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc] cursor-pointer"
            >
              <option value="bat">.bat</option>
              <option value="ps1">.ps1</option>
              <option value="py">.py</option>
              <option value="sh">.sh</option>
            </select>
          </label>
        </div>
      )}

      <div className="text-[10px] font-mono text-[#8f8f8f] dark:text-[#8b949e] truncate" title={`${targetDir}${previewName}`}>
        {targetDir}{previewName}
      </div>

      {error && (
        <div className="text-[11px] font-sans text-red-600 dark:text-red-400">{error}</div>
      )}

      <div className="flex items-center gap-1.5">
        <button
          onClick={() => void handleSubmit()}
          disabled={isSaving}
          className="px-2.5 py-1 rounded-md text-[11px] font-mono bg-[#171717] dark:bg-[#238636] hover:bg-[#262626] dark:hover:bg-[#2ea043] text-white transition cursor-pointer disabled:opacity-50"
        >
          {isSaving ? 'Создание...' : 'Создать'}
        </button>
        <button
          onClick={onClose}
          disabled={isSaving}
          className="px-2.5 py-1 rounded-md text-[11px] font-mono bg-white dark:bg-[#21262d] border border-[#ebebeb] dark:border-[#30363d] text-[#4d4d4d] dark:text-[#c9d1d9] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#f4f4f5] dark:hover:bg-[#30363d] transition cursor-pointer disabled:opacity-50"
        >
          Отмена
        </button>
      </div>
    </div>
  );
}

function FileTreeItem({
  file,
  displayName,
  isActive,
  icon,
  badge,
  onSelect,
}: {
  file: CortexAgentFile;
  displayName: string;
  isActive: boolean;
  icon: React.ReactNode;
  badge?: string | null;
  onSelect: () => void;
}) {
  return (
    <button
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
        <span className="text-xs font-mono truncate">{displayName}</span>
        {badge && (
          <span className="px-1 py-px rounded border border-[#ebebeb] dark:border-[#30363d] bg-[#f4f4f5] dark:bg-[#161b22] text-[10px] font-mono text-[#666666] dark:text-[#8b949e] shrink-0">
            {badge}
          </span>
        )}
      </div>

      {file.isDirty && (
        <span className="h-1.5 w-1.5 rounded-full bg-amber-500 shrink-0" title="Несохраненные изменения" />
      )}
    </button>
  );
}

interface EditAgentPersonaModalProps {
  isOpen: boolean;
  persona: AgentPersonaInfo | null;
  onClose: () => void;
  onRename: (oldRole: string, newRole: string) => Promise<void>;
  onColorChange: (role: string, color: string) => Promise<void>;
  onIconChange: (role: string, icon: string) => Promise<void>;
}

function EditAgentPersonaModal({
  isOpen,
  persona,
  onClose,
  onRename,
  onColorChange,
  onIconChange,
}: EditAgentPersonaModalProps) {
  const role = persona?.role ?? '';
  const isBuiltin = role === 'cvh-dev' || Boolean(persona?.isBuiltin);
  const [newName, setNewName] = useState(role);
  const [selectedColor, setSelectedColor] = useState(
    persona?.color && isAllowedAgentColor(persona.color)
      ? persona.color
      : getDeterministicAgentColor(role)
  );
  const [selectedIcon, setSelectedIcon] = useState(
    persona?.icon || (isBuiltin ? 'Cpu' : 'Bot')
  );
  const [error, setError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  useEffect(() => {
    if (isOpen && persona) {
      setNewName(persona.role);
      setSelectedColor(
        persona.color && isAllowedAgentColor(persona.color)
          ? persona.color
          : getDeterministicAgentColor(persona.role)
      );
      setSelectedIcon(persona.icon || (persona.isBuiltin ? 'Cpu' : 'Bot'));
      setError(null);
      setIsSubmitting(false);
    }
  }, [isOpen, persona]);

  if (!isOpen || !persona) return null;

  const handleSubmit = async (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    const trimmed = newName.trim().toLowerCase();
    if (!trimmed) {
      setError('Имя агента не может быть пустым');
      return;
    }
    if (!/^[a-z0-9][a-z0-9-]*$/.test(trimmed)) {
      setError('Разрешены только строчные латинские буквы, цифры и дефис (^[a-z0-9][a-z0-9-]*$)');
      return;
    }

    if (!isAllowedAgentColor(selectedColor)) {
      setError('Серый и зеленый цвета зарезервированы за Backlog и Ready и запрещены для агентов');
      return;
    }

    setIsSubmitting(true);
    setError(null);
    try {
      let currentRole = role;
      if (trimmed !== role && !isBuiltin) {
        await onRename(role, trimmed);
        currentRole = trimmed;
      }
      if (selectedColor !== (persona.color ?? '')) {
        await onColorChange(currentRole, selectedColor);
      }
      if (selectedIcon !== (persona.icon ?? '')) {
        await onIconChange(currentRole, selectedIcon);
      }
      onClose();
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setError(msg);
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 dark:bg-black/75 backdrop-blur-xs p-4 animate-in fade-in duration-150"
      onClick={onClose}
    >
      <div
        className="w-full max-w-md rounded-xl border border-[#e4e4e7] dark:border-[#21262d] bg-white dark:bg-[#0d1117] text-[#171717] dark:text-[#f0f6fc] p-5 shadow-2xl space-y-4 max-h-[90vh] overflow-y-auto"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between border-b border-[#f4f4f5] dark:border-[#21262d] pb-3">
          <div className="flex items-center space-x-2">
            <Palette className="h-4 w-4 text-[#171717] dark:text-[#f0f6fc]" />
            <h3 className="font-semibold text-sm text-[#171717] dark:text-[#f0f6fc]">Настройка агента: {role}</h3>
          </div>
          <button
            onClick={onClose}
            className="p-1 rounded-md text-[#71717a] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#f4f4f5] dark:hover:bg-[#161b22] transition cursor-pointer"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="space-y-4">
          {/* Имя роли */}
          <div>
            <label className="block text-xs font-medium text-[#52525b] dark:text-[#8b949e] mb-1">
              Имя агента (роль)
            </label>
            <input
              type="text"
              disabled={isBuiltin}
              value={newName}
              onChange={(e) => {
                setNewName(e.target.value);
                setError(null);
              }}
              placeholder="имя-агента"
              className="w-full px-3 py-1.5 rounded-lg border border-[#d4d4d8] dark:border-[#30363d] bg-white dark:bg-[#161b22] text-[#171717] dark:text-[#f0f6fc] text-xs font-mono focus:border-[#171717] dark:focus:border-[#f0f6fc] focus:outline-hidden transition disabled:opacity-50 disabled:bg-[#f4f4f5] dark:disabled:bg-[#0d1117] disabled:text-[#8f8f8f] dark:disabled:text-[#6e7681]"
            />
            {isBuiltin && (
              <p className="mt-1 text-[11px] text-[#71717a] dark:text-[#8b949e]">
                Встроенного агента 'cvh-dev' нельзя переименовать
              </p>
            )}
          </div>

          {/* Палитра цветов агента */}
          <div>
            <div className="flex items-center justify-between mb-1.5">
              <label className="block text-xs font-medium text-[#52525b] dark:text-[#8b949e]">
                Цвет агента
              </label>
              <span className="text-[10px] font-mono text-[#71717a] dark:text-[#8b949e]">
                {selectedColor}
              </span>
            </div>
            <div className="p-2.5 rounded-lg border border-[#ebebeb] dark:border-[#30363d] bg-[#fafafa] dark:bg-[#161b22] space-y-2">
              <div className="flex flex-wrap gap-2 items-center">
                {AGENT_PALETTE.map((c) => {
                  const isSelected = selectedColor.toLowerCase() === c.toLowerCase();
                  return (
                    <button
                      key={c}
                      type="button"
                      onClick={() => setSelectedColor(c)}
                      title={c}
                      className={`h-7 w-7 rounded-full transition-transform cursor-pointer flex items-center justify-center shadow-xs ${
                        isSelected ? 'scale-110 ring-2 ring-offset-2 ring-[#171717] dark:ring-[#f0f6fc]' : 'hover:scale-105'
                      }`}
                      style={{ backgroundColor: c }}
                    >
                      {isSelected && <Check className="h-3.5 w-3.5 text-white drop-shadow" />}
                    </button>
                  );
                })}
              </div>
              <p className="text-[10px] font-sans text-[#71717a] dark:text-[#8b949e] leading-tight">
                Серый (#64748b) и зеленый (#10b981) зарезервированы за Backlog и Ready и исключены из палитры агентов.
              </p>
            </div>
          </div>

          {/* Выбор SVG иконки */}
          <div>
            <div className="flex items-center justify-between mb-1.5">
              <label className="block text-xs font-medium text-[#52525b] dark:text-[#8b949e]">
                SVG Иконка агента
              </label>
              <span className="text-[10px] font-mono text-[#71717a] dark:text-[#8b949e]">
                [{selectedIcon}]
              </span>
            </div>
            <div className="p-2 rounded-lg border border-[#ebebeb] dark:border-[#30363d] bg-[#fafafa] dark:bg-[#161b22]">
              <div className="grid grid-cols-6 gap-1.5 max-h-36 overflow-y-auto p-1">
                {AGENT_ICONS.map((ic) => {
                  const isSelected = selectedIcon === ic;
                  return (
                    <button
                      key={ic}
                      type="button"
                      onClick={() => setSelectedIcon(ic)}
                      title={ic}
                      className={`flex flex-col items-center justify-center p-2 rounded-md transition cursor-pointer ${
                        isSelected
                          ? 'bg-white dark:bg-[#21262d] border-2 border-[#171717] dark:border-[#f0f6fc] shadow-xs'
                          : 'hover:bg-white/80 dark:hover:bg-[#21262d]/80 border border-transparent text-[#71717a] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc]'
                      }`}
                    >
                      <AgentIcon
                        name={ic}
                        className="h-4 w-4"
                        style={{ color: isSelected ? selectedColor : undefined }}
                      />
                    </button>
                  );
                })}
              </div>
            </div>
          </div>

          {error && (
            <div className="text-xs text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-950/30 p-2 rounded-md border border-red-200 dark:border-red-900/50">
              {error}
            </div>
          )}

          <div className="flex items-center justify-end space-x-2 pt-2 border-t border-[#f4f4f5] dark:border-[#21262d]">
            <button
              type="button"
              onClick={onClose}
              disabled={isSubmitting}
              className="px-3 py-1.5 rounded-md text-xs font-medium text-[#71717a] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#f4f4f5] dark:hover:bg-[#161b22] transition cursor-pointer"
            >
              Отмена
            </button>
            <button
              type="submit"
              disabled={isSubmitting}
              className="px-3.5 py-1.5 rounded-md text-xs font-medium bg-[#171717] dark:bg-[#238636] hover:bg-[#262626] dark:hover:bg-[#2ea043] text-white transition cursor-pointer disabled:opacity-50 flex items-center space-x-1.5"
            >
              {isSubmitting ? <span>Сохранение...</span> : <span>Сохранить</span>}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
