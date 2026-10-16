import { useState, useId, useEffect } from 'react';
import { useCortexStore, cortexAgentDir, previewCortexFileName } from '../../stores/cortexStore';
import type { CortexHookExt, CortexHookMoment, AgentPersonaInfo } from '../../stores/cortexStore';
import { useBoardStore } from '../../stores/boardStore';
import type { PipelineColumn, PipelineConfig } from '../../types';
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
  } = useCortexStore();
  const syncStagesFromColumns = useCortexStore((s) => s.syncStagesFromColumns);
  const { columns } = useBoardStore();
  const setColumns = useBoardStore((s) => s.setColumns);
  const [filterQuery, setFilterQuery] = useState('');
  const [openSections, setOpenSections] = useState({ columns: true, agents: true });
  const [openRulesFolders, setOpenRulesFolders] = useState<Record<string, boolean>>({});
  const [openHooksFolders, setOpenHooksFolders] = useState<Record<string, boolean>>({});
  const [creatingStageId, setCreatingStageId] = useState<string | null>(null);
  const [creatingKind, setCreatingKind] = useState<'rule' | 'hook'>('rule');
  const [agentSaving, setAgentSaving] = useState<Record<string, boolean>>({});
  const [agentError, setAgentError] = useState<Record<string, string | null>>({});

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
    const prev = columns;
    const next = prev.map((c) => (c.id === columnId ? { ...c, persona: nextPersona } : c));
    setColumns(next);
    const saved = await savePipelineConfig(next);
    setAgentSaving((prevMap) => ({ ...prevMap, [columnId]: false }));
    if (!saved) {
      setColumns(prev);
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
    <aside className="w-64 shrink-0 h-full flex flex-col bg-[#f9f9f9] border-r border-[#ebebeb] select-none text-[#171717]">
      {/* Sidebar Header */}
      <div className="px-3.5 py-2.5 border-b border-[#ebebeb] bg-white flex items-center justify-between">
        <span className="text-xs font-semibold text-[#171717]">
          Проводник разума агента
        </span>
      </div>

      {/* Filter Search Input */}
      <div className="p-2.5 border-b border-[#ebebeb] bg-[#f9f9f9]">
        <div className="relative flex items-center">
          <Search className="absolute left-2.5 h-3.5 w-3.5 text-[#8f8f8f] pointer-events-none" />
          <input
            id={searchInputId}
            name="cortexMindSearch"
            type="text"
            value={filterQuery}
            onChange={(e) => setFilterQuery(e.target.value)}
            placeholder="Поиск колонок, агентов, файлов..."
            className="w-full pl-8 pr-2.5 py-1 text-xs rounded-md bg-white border border-[#ebebeb] text-[#171717] placeholder:text-[#8f8f8f] focus:outline-hidden focus:border-[#171717] font-sans transition"
          />
        </div>
      </div>

      {/* Sections Tree */}
      <div className="flex-1 overflow-y-auto p-2 space-y-1 font-mono text-xs">
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
                <div className="mx-1 mt-1 rounded-md border border-dashed border-[#d4d4d4] bg-white px-3 py-4 text-center space-y-1.5">
                  <div className="text-xs font-sans font-semibold text-[#171717]">
                    Нет агентов
                  </div>
                  <div className="text-[11px] font-sans text-[#666666] leading-relaxed">
                    Создайте первую колонку-агента через + Колонка
                  </div>
                  <div className="text-[10px] font-mono text-[#8f8f8f]">
                    Свежий проект стартует только с backlog и ready
                  </div>
                </div>
              )}

              {stages.length > 0 && filteredColumnStages.length === 0 && (
                <div className="mx-1 mt-1 rounded-md border border-[#ebebeb] bg-white px-3 py-4 text-center">
                  <div className="text-[11px] font-sans text-[#666666]">
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
                <div className="mx-1 mt-1 rounded-md border border-dashed border-[#d4d4d4] bg-white px-3 py-4 text-center space-y-1.5">
                  <div className="text-xs font-sans font-semibold text-[#171717]">
                    Нет доступных агентов
                  </div>
                  <div className="text-[11px] font-sans text-[#666666] leading-relaxed">
                    Агенты берутся из списка персон проекта
                  </div>
                </div>
              )}

              {agentStages.length > 0 && filteredAgentStages.length === 0 && (
                <div className="mx-1 mt-1 rounded-md border border-[#ebebeb] bg-white px-3 py-4 text-center">
                  <div className="text-[11px] font-sans text-[#666666]">
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
                />
              ))}
            </div>
          )}
        </section>
      </div>
    </aside>
  );
}

function SectionHeader({
  title,
  count,
  open,
  onToggle,
}: {
  title: string;
  count: number;
  open: boolean;
  onToggle: () => void;
}) {
  return (
    <button
      onClick={onToggle}
      aria-expanded={open}
      className="w-full flex items-center gap-1.5 px-1.5 py-2 rounded-md text-left text-[#171717] hover:bg-[#f4f4f5] transition cursor-pointer"
    >
      {open ? (
        <ChevronDown className="h-3.5 w-3.5 text-[#666666] shrink-0" />
      ) : (
        <ChevronRight className="h-3.5 w-3.5 text-[#666666] shrink-0" />
      )}
      <span className="font-sans text-xs font-semibold">{title}</span>
      <span className="text-[10px] font-mono text-[#8f8f8f] tabular-nums">({count})</span>
    </button>
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
  onToggle,
  onPlus,
}: {
  stage: CortexAgentStage;
  hint: string;
  onToggle: () => void;
  onPlus?: () => void;
}) {
  const isBuiltin = stage.personaName === 'cvh-dev' || Boolean(stage.isBuiltin);
  return (
    <div className="group flex items-center gap-0.5 px-1">
      <button
        onClick={onToggle}
        className="flex-1 flex items-center justify-between px-1.5 py-1.5 rounded-md text-left transition cursor-pointer min-w-0"
      >
        <div className="flex items-center space-x-2 min-w-0">
          {stage.isExpanded ? (
            <ChevronDown className="h-3.5 w-3.5 text-[#666666] shrink-0" />
          ) : (
            <ChevronRight className="h-3.5 w-3.5 text-[#666666] shrink-0" />
          )}

          {isBuiltin ? (
            <Cpu className="h-3.5 w-3.5 text-indigo-600 shrink-0" />
          ) : (
            <span
              className="w-2 h-2 rounded-full shrink-0"
              style={{ backgroundColor: stage.color || '#171717' }}
            />
          )}

          <span className="font-sans text-xs font-semibold text-[#171717] truncate">
            {stage.stageTitle}
          </span>

          {isBuiltin && (
            <span className="px-1.5 py-0.5 rounded text-[10px] font-sans font-medium bg-indigo-50 text-indigo-700 border border-indigo-200 shrink-0">
              Встроенный
            </span>
          )}
        </div>

        {hint !== '' && !isBuiltin && (
          <span className="text-[10px] font-mono text-[#666666] truncate ml-1">
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
          className="p-1 rounded-md text-[#8f8f8f] hover:text-[#171717] hover:bg-[#ebebeb]/60 transition cursor-pointer opacity-0 group-hover:opacity-100 focus:opacity-100 focus-visible:opacity-100 shrink-0"
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

  return (
    <div
      id={`cortex-stage-${stage.stageId}`}
      className={`rounded-md transition-colors ${
        isFocused
          ? 'bg-white border border-[#ebebeb] shadow-xs'
          : 'hover:bg-[#f4f4f5]'
      }`}
    >
      <StageHeaderButton
        stage={stage}
        hint={stage.personaName}
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
        <div className="pl-4 pr-1 py-0.5 space-y-0.5 border-l border-[#ebebeb] ml-3 my-0.5">
          {!isBacklogSystem && column && (
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
      className={`rounded-md transition-colors ${
        isFocused
          ? 'bg-white border border-[#ebebeb] shadow-xs'
          : 'hover:bg-[#f4f4f5]'
      }`}
    >
      <StageHeaderButton
        stage={stage}
        hint=""
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
        <div className="pl-4 pr-1 py-0.5 space-y-0.5 border-l border-[#ebebeb] ml-3 my-0.5">
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
            <div className="px-2 py-1.5 rounded-md border border-dashed border-[#d4d4d4] bg-white space-y-1">
              <div className="text-[11px] font-sans text-[#666666]">
                Персона не настроена — нет файла
              </div>
              <button
                onClick={() => onCreatePersona(personaFile.id)}
                className="flex items-center space-x-1 px-2 py-1 rounded-md text-[11px] font-mono bg-[#171717] hover:bg-[#262626] text-white transition cursor-pointer"
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

  const known = personas.some((p) => p.role === currentValue);

  return (
    <div className="px-2 py-1">
      <div className="flex items-center gap-1.5">
        <span className="text-[10px] font-sans font-semibold uppercase tracking-wide text-[#8f8f8f] shrink-0">
          Агент
        </span>
        {personas.length > 0 ? (
          <select
            value={currentValue}
            disabled={saving}
            onChange={(e) => onChange(columnId, e.target.value)}
            title="Агент колонки"
            className="flex-1 min-w-0 px-1.5 py-0.5 rounded-md bg-white border border-[#ebebeb] text-[#171717] font-mono text-[11px] focus:outline-hidden focus:border-[#171717] cursor-pointer disabled:opacity-50 transition truncate"
          >
            <option value="">без агента</option>
            {currentValue && !known && (
              <option value={currentValue}>{currentValue}</option>
            )}
            {personas.map((p) => {
              const isBuiltin = p.role === 'cvh-dev' || Boolean(p.isBuiltin);
              return (
                <option key={p.role} value={p.role}>
                  {isBuiltin ? `${p.role} (Встроенный)` : p.role}
                </option>
              );
            })}
          </select>
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
            className="flex-1 min-w-0 px-1.5 py-0.5 rounded-md bg-white border border-[#ebebeb] text-[#171717] font-mono text-[11px] placeholder:text-[#8f8f8f] focus:outline-hidden focus:border-[#171717] transition disabled:opacity-50"
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
          className="flex-1 min-w-0 flex items-center space-x-1.5 px-2 py-1 rounded-md text-left text-[#666666] hover:text-[#171717] hover:bg-[#ebebeb]/60 transition cursor-pointer"
        >
          {isOpen ? (
            <FolderOpen className="h-3.5 w-3.5 text-[#666666] shrink-0" />
          ) : (
            <Folder className="h-3.5 w-3.5 text-[#666666] shrink-0" />
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
          className="p-1 rounded-md text-[#8f8f8f] hover:text-[#171717] hover:bg-[#ebebeb]/60 transition cursor-pointer opacity-0 group-hover/folder:opacity-100 focus:opacity-100 focus-visible:opacity-100 shrink-0"
        >
          <Plus className="h-3 w-3" />
        </button>
      </div>

      {isOpen && files.length > 0 && (
        <div className="pl-4 space-y-0.5 border-l border-[#ebebeb] ml-2">
          {files.map((ruleFile) => (
            <FileTreeItem
              key={ruleFile.id}
              file={ruleFile}
              displayName={ruleFile.name}
              isActive={activeFileId === ruleFile.id}
              icon={<FileText className="h-3.5 w-3.5 text-[#666666] shrink-0" />}
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
          className="flex-1 min-w-0 flex items-center space-x-1.5 px-2 py-1 rounded-md text-left text-[#666666] hover:text-[#171717] hover:bg-[#ebebeb]/60 transition cursor-pointer"
        >
          {isOpen ? (
            <FolderOpen className="h-3.5 w-3.5 text-[#666666] shrink-0" />
          ) : (
            <Folder className="h-3.5 w-3.5 text-[#666666] shrink-0" />
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
            className="p-1 rounded-md text-[#8f8f8f] hover:text-[#171717] hover:bg-[#ebebeb]/60 transition cursor-pointer opacity-0 group-hover/folder:opacity-100 focus:opacity-100 focus-visible:opacity-100 shrink-0"
          >
            <Plus className="h-3 w-3" />
          </button>
        )}
      </div>

      {isOpen && files.length > 0 && (
        <div className="pl-4 space-y-0.5 border-l border-[#ebebeb] ml-2">
          {files.map((hookFile) => {
            const isBuiltin = hookFile.path === 'builtin:create_worktree' || hookFile.path.startsWith('builtin:');
            return (
              <FileTreeItem
                key={hookFile.id}
                file={hookFile}
                displayName={hookFile.name}
                isActive={activeFileId === hookFile.id}
                badge={isBuiltin ? 'встроенный' : hookMomentOf(hookFile.name)}
                icon={<Zap className="h-3.5 w-3.5 text-[#666666] shrink-0" />}
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
    <div className="px-2 py-1.5 rounded-md border border-dashed border-[#d4d4d4] bg-white space-y-1">
      <div className="text-[11px] font-sans text-[#666666]">
        Настройки не заданы — нет файла
      </div>
      <button
        onClick={onCreate}
        className="flex items-center space-x-1 px-2 py-1 rounded-md text-[11px] font-mono bg-[#171717] hover:bg-[#262626] text-white transition cursor-pointer"
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
    <div className="mx-1 mb-1 rounded-md border border-[#ebebeb] bg-white px-2 py-2 space-y-2">
      <div className="text-[11px] font-sans font-semibold text-[#171717]">
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
        className="w-full px-2 py-1 text-xs rounded-md bg-white border border-[#ebebeb] text-[#171717] font-mono placeholder:text-[#8f8f8f] focus:outline-hidden focus:border-[#171717] transition"
      />
      <div className="text-[10px] font-mono text-[#8f8f8f]">
        Латиница, цифры, - и _ (расширение добавится автоматически)
      </div>

      {!fixedKind && (
        <div className="flex items-center gap-3 text-[11px] font-mono text-[#4d4d4d]">
          <label className="flex items-center gap-1 cursor-pointer">
            <input
              type="radio"
              name={`cortex-kind-${stage.stageId}`}
              checked={kind === 'rule'}
              onChange={() => setKind('rule')}
              className="accent-[#171717]"
            />
            <span>Правило</span>
          </label>
          <label className="flex items-center gap-1 cursor-pointer">
            <input
              type="radio"
              name={`cortex-kind-${stage.stageId}`}
              checked={kind === 'hook'}
              onChange={() => setKind('hook')}
              className="accent-[#171717]"
            />
            <span>Хук</span>
          </label>
        </div>
      )}

      {kind === 'hook' && (
        <div className="space-y-1.5 rounded-md bg-[#f9f9f9] border border-[#ebebeb] px-2 py-1.5">
          <div className="flex items-center gap-3 text-[11px] font-mono text-[#4d4d4d]">
            <label className="flex items-center gap-1 cursor-pointer">
              <input
                type="radio"
                name={`cortex-moment-${stage.stageId}`}
                checked={hookMoment === 'on_enter'}
                onChange={() => setHookMoment('on_enter')}
                className="accent-[#171717]"
              />
              <span>on_enter</span>
            </label>
            <label className="flex items-center gap-1 cursor-pointer">
              <input
                type="radio"
                name={`cortex-moment-${stage.stageId}`}
                checked={hookMoment === 'on_finish'}
                onChange={() => setHookMoment('on_finish')}
                className="accent-[#171717]"
              />
              <span>on_finish</span>
            </label>
          </div>
          <label className="flex items-center gap-1.5 text-[11px] font-mono text-[#4d4d4d]">
            <span>Тип:</span>
            <select
              value={hookExt}
              onChange={(e) => setHookExt(e.target.value as CortexHookExt)}
              className="px-1.5 py-0.5 rounded-md bg-white border border-[#ebebeb] text-[#171717] font-mono text-[11px] focus:outline-hidden focus:border-[#171717] cursor-pointer"
            >
              <option value="bat">.bat</option>
              <option value="ps1">.ps1</option>
              <option value="py">.py</option>
              <option value="sh">.sh</option>
            </select>
          </label>
        </div>
      )}

      <div className="text-[10px] font-mono text-[#8f8f8f] truncate" title={`${targetDir}${previewName}`}>
        {targetDir}{previewName}
      </div>

      {error && (
        <div className="text-[11px] font-sans text-red-600">{error}</div>
      )}

      <div className="flex items-center gap-1.5">
        <button
          onClick={() => void handleSubmit()}
          disabled={isSaving}
          className="px-2.5 py-1 rounded-md text-[11px] font-mono bg-[#171717] hover:bg-[#262626] text-white transition cursor-pointer disabled:opacity-50"
        >
          {isSaving ? 'Создание...' : 'Создать'}
        </button>
        <button
          onClick={onClose}
          disabled={isSaving}
          className="px-2.5 py-1 rounded-md text-[11px] font-mono bg-white border border-[#ebebeb] text-[#4d4d4d] hover:text-[#171717] hover:bg-[#f4f4f5] transition cursor-pointer disabled:opacity-50"
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
          ? 'bg-white border border-[#ebebeb] text-[#171717] font-semibold shadow-xs'
          : 'text-[#4d4d4d] hover:text-[#171717] hover:bg-[#ebebeb]/60'
      }`}
      title={file.path}
    >
      <div className="flex items-center space-x-1.5 min-w-0">
        {icon}
        <span className="text-xs font-mono truncate">{displayName}</span>
        {badge && (
          <span className="px-1 py-px rounded border border-[#ebebeb] bg-[#f4f4f5] text-[10px] font-mono text-[#666666] shrink-0">
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
