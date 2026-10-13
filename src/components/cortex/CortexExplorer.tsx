import { useState, useId, useEffect } from 'react';
import { useCortexStore } from '../../stores/cortexStore';
import { useBoardStore } from '../../stores/boardStore';
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
} from 'lucide-react';
import { CortexAgentFile, CortexAgentStage } from '../../types';

export default function CortexExplorer() {
  const { stages, activeFile, focusedStageId, toggleStageExpand, openFile, personaMissing, createPersonaTemplate } = useCortexStore();
  const syncStagesFromColumns = useCortexStore((s) => s.syncStagesFromColumns);
  const { columns } = useBoardStore();
  const [filterQuery, setFilterQuery] = useState('');
  const [openRulesFolders, setOpenRulesFolders] = useState<Record<string, boolean>>({});
  const searchInputId = useId();

  // Источник стадий — колонки доски (колонка с persona = агент).
  // Мок-стадии из стора остаются только как fallback, когда columns пуст.
  useEffect(() => {
    syncStagesFromColumns(columns);
  }, [columns, syncStagesFromColumns]);

  const toggleRulesFolder = (stageId: string) => {
    setOpenRulesFolders((prev) => ({
      ...prev,
      [stageId]: !(prev[stageId] ?? true),
    }));
  };

  const handleOpenFile = (fileId: string) => {
    void openFile(fileId);
  };

  const handleCreateTemplate = (fileId: string) => {
    void createPersonaTemplate(fileId);
  };

  const filteredStages = stages.map((stage) => {
    if (!filterQuery) return stage;
    const q = filterQuery.toLowerCase();
    const matchesStage = stage.stageTitle.toLowerCase().includes(q) || stage.personaName.toLowerCase().includes(q);
    const matchingFiles = stage.files.filter((f) => f.name.toLowerCase().includes(q) || f.path.toLowerCase().includes(q));
    if (matchesStage || matchingFiles.length > 0) {
      return {
        ...stage,
        isExpanded: true,
        files: matchingFiles.length > 0 ? matchingFiles : stage.files,
      };
    }
    return null;
  }).filter(Boolean) as CortexAgentStage[];

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
            placeholder="Поиск ролей, инструкций..."
            className="w-full pl-8 pr-2.5 py-1 text-xs rounded-md bg-white border border-[#ebebeb] text-[#171717] placeholder:text-[#8f8f8f] focus:outline-hidden focus:border-[#171717] font-sans transition"
          />
        </div>
      </div>

      {/* Stage Folders Tree */}
      <div className="flex-1 overflow-y-auto p-2 space-y-1 font-mono text-xs">
        {filteredStages.map((stage) => {
          const isFocused = focusedStageId === stage.stageId;
          const personaFile = stage.files.find((f) => f.type === 'persona');
          const ruleFiles = stage.files.filter((f) => f.type === 'rule');
          const settingsFile = stage.files.find((f) => f.type === 'settings');
          const areRulesOpen = openRulesFolders[stage.stageId] ?? true;

          return (
            <div
              key={stage.stageId}
              id={`cortex-stage-${stage.stageId}`}
              className={`rounded-md transition-colors ${
                isFocused
                  ? 'bg-white border border-[#ebebeb] shadow-xs'
                  : 'hover:bg-[#f4f4f5]'
              }`}
            >
              {/* Stage Root Folder Header */}
              <button
                onClick={() => toggleStageExpand(stage.stageId)}
                className="w-full flex items-center justify-between px-2 py-1.5 rounded-md text-left transition cursor-pointer"
              >
                <div className="flex items-center space-x-2 min-w-0">
                  {stage.isExpanded ? (
                    <ChevronDown className="h-3.5 w-3.5 text-[#666666] shrink-0" />
                  ) : (
                    <ChevronRight className="h-3.5 w-3.5 text-[#666666] shrink-0" />
                  )}

                  <span
                    className="w-2 h-2 rounded-full shrink-0"
                    style={{ backgroundColor: stage.color || '#171717' }}
                  />

                  <span className="font-sans text-xs font-semibold text-[#171717] truncate">
                    {stage.stageTitle}
                  </span>
                </div>

                <span className="text-[10px] font-mono text-[#666666] truncate ml-1">
                  {stage.personaName}
                </span>
              </button>

              {/* Children */}
              {stage.isExpanded && (
                <div className="pl-4 pr-1 py-0.5 space-y-0.5 border-l border-[#ebebeb] ml-3 my-0.5">
                  {/* 1. Роли */}
                  {personaFile && (
                    <FileTreeItem
                      file={personaFile}
                      displayName="Роли"
                      isActive={activeFile?.id === personaFile.id}
                      icon={<User className="h-3.5 w-3.5 text-[#666666] shrink-0" />}
                      onSelect={() => handleOpenFile(personaFile.id)}
                    />
                  )}

                  {/* Персона отсутствует на диске: плейсхолдер + создание шаблона */}
                  {personaFile && personaMissing[personaFile.id] && (
                    <div className="px-2 py-1.5 rounded-md border border-dashed border-[#d4d4d4] bg-white space-y-1">
                      <div className="text-[11px] font-sans text-[#666666]">
                        Персона не настроена
                      </div>
                      <div
                        className="text-[10px] font-mono text-[#8f8f8f] truncate"
                        title={personaFile.path}
                      >
                        {personaFile.path}
                      </div>
                      <button
                        onClick={() => handleCreateTemplate(personaFile.id)}
                        className="flex items-center space-x-1 px-2 py-1 rounded-md text-[11px] font-mono bg-[#171717] hover:bg-[#262626] text-white transition cursor-pointer"
                        title="Создать шаблон персоны"
                      >
                        <Plus className="h-3 w-3 shrink-0" />
                        <span>Создать шаблон</span>
                      </button>
                    </div>
                  )}

                  {/* 2. Правила пайплайна */}
                  {ruleFiles.length > 0 && (
                    <div className="space-y-0.5">
                      <button
                        onClick={() => toggleRulesFolder(stage.stageId)}
                        className="w-full flex items-center space-x-1.5 px-2 py-1 rounded-md text-left text-[#666666] hover:text-[#171717] hover:bg-[#ebebeb]/60 transition cursor-pointer"
                      >
                        {areRulesOpen ? (
                          <FolderOpen className="h-3.5 w-3.5 text-[#666666] shrink-0" />
                        ) : (
                          <Folder className="h-3.5 w-3.5 text-[#666666] shrink-0" />
                        )}
                        <span className="text-xs font-mono">Правила пайплайна</span>
                        <span className="text-[10px] text-[#8f8f8f] tabular-nums">({ruleFiles.length})</span>
                      </button>

                      {areRulesOpen && (
                        <div className="pl-4 space-y-0.5 border-l border-[#ebebeb] ml-2">
                          {ruleFiles.map((ruleFile) => (
                            <FileTreeItem
                              key={ruleFile.id}
                              file={ruleFile}
                              displayName={ruleFile.name}
                              isActive={activeFile?.id === ruleFile.id}
                              icon={<FileText className="h-3.5 w-3.5 text-[#666666] shrink-0" />}
                              onSelect={() => handleOpenFile(ruleFile.id)}
                            />
                          ))}
                        </div>
                      )}
                    </div>
                  )}

                  {/* 3. Системные инструкции */}
                  {settingsFile && (
                    <FileTreeItem
                      file={settingsFile}
                      displayName="Системные инструкции"
                      isActive={activeFile?.id === settingsFile.id}
                      icon={<Settings className="h-3.5 w-3.5 text-[#666666] shrink-0" />}
                      onSelect={() => handleOpenFile(settingsFile.id)}
                    />
                  )}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </aside>
  );
}

function FileTreeItem({
  file,
  displayName,
  isActive,
  icon,
  onSelect,
}: {
  file: CortexAgentFile;
  displayName: string;
  isActive: boolean;
  icon: React.ReactNode;
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
      </div>

      {file.isDirty && (
        <span className="h-1.5 w-1.5 rounded-full bg-amber-500 shrink-0" title="Несохраненные изменения" />
      )}
    </button>
  );
}
