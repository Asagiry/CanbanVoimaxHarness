import { useState, useId } from 'react';
import { useCortexStore } from '../../stores/cortexStore';
import {
  ChevronDown,
  ChevronRight,
  Folder,
  FolderOpen,
  FileText,
  Settings,
  Search,
  User,
} from 'lucide-react';
import { CortexAgentFile, CortexAgentStage } from '../../types';

export default function CortexExplorer() {
  const { stages, activeFile, focusedStageId, toggleStageExpand, openFile } = useCortexStore();
  const [filterQuery, setFilterQuery] = useState('');
  const [openRulesFolders, setOpenRulesFolders] = useState<Record<string, boolean>>({
    dev: true,
    review: true,
  });
  const searchInputId = useId();

  const toggleRulesFolder = (stageId: string) => {
    setOpenRulesFolders((prev) => ({
      ...prev,
      [stageId]: !prev[stageId],
    }));
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
    <aside className="w-64 shrink-0 h-full flex flex-col bg-obsidian-900 border-r border-obsidian-800 select-none text-slate-300">
      {/* Sidebar Header */}
      <div className="px-3 py-2 border-b border-obsidian-800 bg-obsidian-850 flex items-center justify-between">
        <span className="text-xs font-semibold text-slate-200">
          Проводник разума агента
        </span>
      </div>

      {/* Filter Search Input */}
      <div className="p-2 border-b border-obsidian-800/80">
        <div className="relative flex items-center">
          <Search className="absolute left-2 h-3 w-3 text-slate-500 pointer-events-none" />
          <input
            id={searchInputId}
            name="cortexMindSearch"
            type="text"
            value={filterQuery}
            onChange={(e) => setFilterQuery(e.target.value)}
            placeholder="Поиск ролей, инструкций..."
            className="w-full pl-6 pr-2 py-1 text-xs rounded bg-obsidian-950 border border-obsidian-800 text-slate-200 placeholder-slate-600 focus:outline-hidden focus:border-slate-600 font-mono"
          />
        </div>
      </div>

      {/* Stage Folders Tree */}
      <div className="flex-1 overflow-y-auto p-1.5 space-y-0.5 font-mono text-xs">
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
              className={`rounded transition-colors ${
                isFocused
                  ? 'bg-obsidian-800'
                  : 'hover:bg-obsidian-850/60'
              }`}
            >
              {/* Stage Root Folder Header */}
              <button
                onClick={() => toggleStageExpand(stage.stageId)}
                className="w-full flex items-center justify-between px-1.5 py-1 rounded text-left hover:bg-obsidian-800 transition cursor-pointer"
              >
                <div className="flex items-center space-x-1.5 min-w-0">
                  {stage.isExpanded ? (
                    <ChevronDown className="h-3 w-3 text-slate-500 shrink-0" />
                  ) : (
                    <ChevronRight className="h-3 w-3 text-slate-500 shrink-0" />
                  )}

                  <span
                    className="w-2 h-2 rounded-full shrink-0"
                    style={{ backgroundColor: stage.color }}
                  />

                  <span className="font-sans text-xs font-medium text-slate-200 truncate">
                    {stage.stageTitle}
                  </span>
                </div>

                <span className="text-[10px] font-mono text-slate-500 truncate ml-1">
                  {stage.personaName}
                </span>
              </button>

              {/* Children */}
              {stage.isExpanded && (
                <div className="pl-4 pr-1 py-0.5 space-y-0.5 border-l border-obsidian-800/80 ml-2.5 my-0.5">
                  {/* 1. Роли */}
                  {personaFile && (
                    <FileTreeItem
                      file={personaFile}
                      displayName="Роли"
                      isActive={activeFile?.id === personaFile.id}
                      icon={<User className="h-3 w-3 text-slate-400 shrink-0" />}
                      onSelect={() => openFile(personaFile.id)}
                    />
                  )}

                  {/* 2. Правила пайплайна */}
                  {ruleFiles.length > 0 && (
                    <div className="space-y-0.5">
                      <button
                        onClick={() => toggleRulesFolder(stage.stageId)}
                        className="w-full flex items-center space-x-1.5 px-1.5 py-0.5 rounded text-left text-slate-400 hover:text-slate-200 hover:bg-obsidian-800 transition cursor-pointer"
                      >
                        {areRulesOpen ? (
                          <FolderOpen className="h-3 w-3 text-slate-500 shrink-0" />
                        ) : (
                          <Folder className="h-3 w-3 text-slate-500 shrink-0" />
                        )}
                        <span className="text-xs font-mono">Правила пайплайна</span>
                        <span className="text-[10px] text-slate-500 tabular-nums">({ruleFiles.length})</span>
                      </button>

                      {areRulesOpen && (
                        <div className="pl-4 space-y-0.5 border-l border-obsidian-800/80 ml-2">
                          {ruleFiles.map((ruleFile) => (
                            <FileTreeItem
                              key={ruleFile.id}
                              file={ruleFile}
                              displayName={ruleFile.name}
                              isActive={activeFile?.id === ruleFile.id}
                              icon={<FileText className="h-3 w-3 text-slate-500 shrink-0" />}
                              onSelect={() => openFile(ruleFile.id)}
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
                      icon={<Settings className="h-3 w-3 text-slate-400 shrink-0" />}
                      onSelect={() => openFile(settingsFile.id)}
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
      className={`w-full flex items-center justify-between px-1.5 py-0.5 rounded text-left transition cursor-pointer ${
        isActive
          ? 'bg-obsidian-800 text-slate-100 font-medium'
          : 'text-slate-400 hover:text-slate-200 hover:bg-obsidian-800/60'
      }`}
      title={file.path}
    >
      <div className="flex items-center space-x-1.5 min-w-0">
        {icon}
        <span className="text-xs font-mono truncate">{displayName}</span>
      </div>

      {file.isDirty && (
        <span className="h-1.5 w-1.5 rounded-full bg-amber-400 shrink-0" title="Несохраненные изменения" />
      )}
    </button>
  );
}
