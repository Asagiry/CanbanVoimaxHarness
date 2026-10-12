import { useState, useId } from 'react';
import { useCortexStore } from '../../stores/cortexStore';
import {
  ChevronDown,
  ChevronRight,
  Folder,
  FolderOpen,
  FileText,
  Settings,
  Sparkles,
  Search,
  BrainCircuit,
  Bot,
  Shield,
  Code2,
  CheckCircle2,
  Inbox,
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

  const getStageIcon = (iconName: string, color: string) => {
    const props = { className: 'h-3.5 w-3.5', style: { color } };
    switch (iconName) {
      case 'Inbox':
        return <Inbox {...props} />;
      case 'Code2':
        return <Code2 {...props} />;
      case 'ShieldCheck':
        return <Shield {...props} />;
      case 'Sparkles':
        return <Sparkles {...props} />;
      case 'CheckCircle2':
        return <CheckCircle2 {...props} />;
      default:
        return <Bot {...props} />;
    }
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
    <aside className="w-72 h-full flex flex-col bg-obsidian-900 border-r border-slate-800/90 select-none text-slate-300">
      {/* Sidebar Header */}
      <div className="flex items-center justify-between px-3.5 py-3 border-b border-slate-800 bg-obsidian-850">
        <div className="flex items-center space-x-2">
          <BrainCircuit className="h-4 w-4 text-cyber-cyan" />
          <span className="text-xs font-bold uppercase tracking-wider text-slate-200">
            Cortex Explorer
          </span>
        </div>
        <span className="px-1.5 py-0.5 rounded text-[10px] font-mono font-medium bg-obsidian-800 text-slate-400 border border-slate-750">
          {stages.length} MINDS
        </span>
      </div>

      {/* Subtitle / Mind descriptor */}
      <div className="px-3.5 pt-2 pb-1 text-[11px] text-slate-400 flex items-center justify-between">
        <span className="uppercase tracking-wider font-semibold text-[10px] text-slate-500">
          Agent Mind Trees
        </span>
        <span className="text-[10px] text-cyber-cyan/80 font-mono">Zero-Turn Primed</span>
      </div>

      {/* Filter / Search input */}
      <div className="px-3 py-1.5">
        <div className="relative flex items-center">
          <Search className="absolute left-2.5 h-3.5 w-3.5 text-slate-500 pointer-events-none" />
          <input
            id={searchInputId}
            name="cortexMindSearch"
            type="text"
            value={filterQuery}
            onChange={(e) => setFilterQuery(e.target.value)}
            placeholder="Search rules, personas, settings..."
            className="w-full pl-8 pr-2.5 py-1 text-xs rounded bg-obsidian-950/80 border border-slate-800 text-slate-200 placeholder-slate-500 focus:outline-hidden focus:border-cyan-500/50 transition font-mono"
          />
        </div>
      </div>

      {/* Stage folders tree */}
      <div className="flex-1 overflow-y-auto px-1.5 py-1 space-y-1 font-mono text-xs">
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
              className={`rounded transition-all duration-300 ${
                isFocused
                  ? 'ring-1 ring-cyber-cyan bg-cyan-950/25 shadow-sm shadow-cyan-500/10'
                  : 'hover:bg-obsidian-850/50'
              }`}
            >
              {/* Stage Root Folder Header */}
              <button
                onClick={() => toggleStageExpand(stage.stageId)}
                className="w-full flex items-center justify-between px-2 py-1.5 rounded text-left group hover:bg-obsidian-800/80 transition"
                style={{
                  borderLeft: `3px solid ${stage.color}`,
                }}
              >
                <div className="flex items-center space-x-1.5 min-w-0">
                  {stage.isExpanded ? (
                    <ChevronDown className="h-3.5 w-3.5 text-slate-400 shrink-0" />
                  ) : (
                    <ChevronRight className="h-3.5 w-3.5 text-slate-400 shrink-0" />
                  )}

                  <div className="shrink-0">{getStageIcon(stage.icon, stage.color)}</div>

                  <span className="font-sans text-xs font-semibold text-slate-200 truncate group-hover:text-white">
                    {stage.stageTitle}
                  </span>
                </div>

                <div className="flex items-center space-x-1 shrink-0 ml-1">
                  <span
                    className="text-[10px] font-mono px-1 rounded truncate max-w-[80px]"
                    style={{
                      backgroundColor: `${stage.color}15`,
                      color: stage.color,
                      border: `1px solid ${stage.color}30`,
                    }}
                  >
                    {stage.personaName}
                  </span>
                </div>
              </button>

              {/* Children (Stage Mind Tree) */}
              {stage.isExpanded && (
                <div className="pl-4 pr-1 py-1 space-y-0.5 border-l border-slate-800/80 ml-2.5 my-0.5">
                  {/* 1. Persona File (persona.md) */}
                  {personaFile && (
                    <FileTreeItem
                      file={personaFile}
                      isActive={activeFile?.id === personaFile.id}
                      icon={<Sparkles className="h-3.5 w-3.5 text-cyber-purple shrink-0" />}
                      onSelect={() => openFile(personaFile.id)}
                    />
                  )}

                  {/* 2. Rules Subfolder */}
                  {ruleFiles.length > 0 && (
                    <div className="space-y-0.5">
                      <button
                        onClick={() => toggleRulesFolder(stage.stageId)}
                        className="w-full flex items-center space-x-1 px-2 py-1 rounded text-left text-slate-400 hover:text-slate-200 hover:bg-obsidian-800/60 transition"
                      >
                        {areRulesOpen ? (
                          <FolderOpen className="h-3.5 w-3.5 text-cyber-amber shrink-0" />
                        ) : (
                          <Folder className="h-3.5 w-3.5 text-cyber-amber shrink-0" />
                        )}
                        <span className="text-[11px] font-mono">rules/</span>
                        <span className="text-[10px] text-slate-500 tabular-nums">({ruleFiles.length})</span>
                      </button>

                      {areRulesOpen && (
                        <div className="pl-4 space-y-0.5 border-l border-slate-800/60 ml-2">
                          {ruleFiles.map((ruleFile) => (
                            <FileTreeItem
                              key={ruleFile.id}
                              file={ruleFile}
                              isActive={activeFile?.id === ruleFile.id}
                              icon={<FileText className="h-3.5 w-3.5 text-cyber-cyan shrink-0" />}
                              onSelect={() => openFile(ruleFile.id)}
                            />
                          ))}
                        </div>
                      )}
                    </div>
                  )}

                  {/* 3. Settings File (settings.json) */}
                  {settingsFile && (
                    <FileTreeItem
                      file={settingsFile}
                      isActive={activeFile?.id === settingsFile.id}
                      icon={<Settings className="h-3.5 w-3.5 text-cyber-amber shrink-0" />}
                      onSelect={() => openFile(settingsFile.id)}
                    />
                  )}
                </div>
              )}
            </div>
          );
        })}
      </div>

      {/* Footer Info Pill */}
      <div className="p-2.5 border-t border-slate-800/90 bg-obsidian-850/60 text-[11px] text-slate-500 font-mono flex items-center justify-between">
        <span className="flex items-center space-x-1.5">
          <span className="h-1.5 w-1.5 rounded-full bg-cyber-emerald animate-pulse" />
          <span>Sync: Git Worktrees</span>
        </span>
        <span className="text-[10px] text-slate-400">Tauri v2 IPC</span>
      </div>
    </aside>
  );
}

function FileTreeItem({
  file,
  isActive,
  icon,
  onSelect,
}: {
  file: CortexAgentFile;
  isActive: boolean;
  icon: React.ReactNode;
  onSelect: () => void;
}) {
  return (
    <button
      onClick={onSelect}
      className={`w-full flex items-center justify-between px-2 py-1 rounded text-left transition group ${
        isActive
          ? 'bg-cyber-cyan/15 text-cyber-cyan font-medium border border-cyber-cyan/30'
          : 'text-slate-300 hover:text-white hover:bg-obsidian-800/70'
      }`}
      title={file.path}
    >
      <div className="flex items-center space-x-1.5 min-w-0">
        {icon}
        <span className="text-[11px] font-mono truncate">{file.name}</span>
      </div>

      <div className="flex items-center space-x-1 shrink-0">
        {file.isDirty && (
          <span className="h-1.5 w-1.5 rounded-full bg-cyber-amber" title="Unsaved" />
        )}
      </div>
    </button>
  );
}
