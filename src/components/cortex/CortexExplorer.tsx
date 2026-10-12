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
    const props = { className: 'h-3.5 w-3.5 shrink-0', style: { color } };
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
    <aside className="w-68 shrink-0 h-full flex flex-col bg-obsidian-900 border-r border-obsidian-800 select-none text-slate-300">
      {/* Sidebar Header - VS Code Activity Style */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-obsidian-800 bg-obsidian-850">
        <div className="flex items-center space-x-1.5">
          <BrainCircuit className="h-3.5 w-3.5 text-cyber-cyan shrink-0" />
          <span className="text-xs font-semibold uppercase tracking-wider text-slate-200 font-sans">
            Cortex Explorer
          </span>
        </div>
        <span className="px-1.5 py-0.2 rounded text-[10px] font-mono font-bold bg-obsidian-950 text-slate-400 border border-obsidian-750 tabular-nums">
          {stages.length} MINDS
        </span>
      </div>

      {/* Mind Subtitle Descriptor */}
      <div className="px-3 pt-1.5 pb-1 text-[10px] font-mono text-slate-500 flex items-center justify-between">
        <span className="uppercase tracking-wider font-semibold text-[9px] text-slate-500">
          Agent Mind Trees
        </span>
        <span className="text-[9px] text-cyber-cyan/90 font-mono">Zero-Turn Primed</span>
      </div>

      {/* VS Code Filter Search Input */}
      <div className="px-2 py-1">
        <div className="relative flex items-center">
          <Search className="absolute left-2 h-3 w-3 text-slate-500 pointer-events-none" />
          <input
            id={searchInputId}
            name="cortexMindSearch"
            type="text"
            value={filterQuery}
            onChange={(e) => setFilterQuery(e.target.value)}
            placeholder="Search rules, personas..."
            className="w-full pl-6.5 pr-2 py-0.5 text-xs rounded-sm bg-obsidian-950 border border-obsidian-800 text-slate-200 placeholder-slate-600 focus:outline-hidden focus:border-cyan-500/60 transition font-mono"
          />
        </div>
      </div>

      {/* Stage Folders Tree - 12px Font & 16px VS Code Indentation */}
      <div className="flex-1 overflow-y-auto px-1 py-1 space-y-0.5 font-mono text-xs">
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
              className={`rounded-sm transition-colors duration-150 ${
                isFocused
                  ? 'bg-obsidian-800/90 border-l-2 border-cyber-cyan pl-0.5'
                  : 'hover:bg-obsidian-850/60'
              }`}
            >
              {/* Stage Root Folder Header */}
              <button
                onClick={() => toggleStageExpand(stage.stageId)}
                className="w-full flex items-center justify-between px-1.5 py-1 rounded-sm text-left group hover:bg-obsidian-800 transition cursor-pointer"
              >
                <div className="flex items-center space-x-1.5 min-w-0">
                  {stage.isExpanded ? (
                    <ChevronDown className="h-3 w-3 text-slate-400 shrink-0" />
                  ) : (
                    <ChevronRight className="h-3 w-3 text-slate-400 shrink-0" />
                  )}

                  <div className="shrink-0">{getStageIcon(stage.icon, stage.color)}</div>

                  <span className="font-sans text-xs font-medium text-slate-200 truncate group-hover:text-white">
                    {stage.stageTitle}
                  </span>
                </div>

                <div className="flex items-center space-x-1 shrink-0 ml-1">
                  <span
                    className="text-[9px] font-mono px-1 py-0.2 rounded truncate max-w-[76px]"
                    style={{
                      backgroundColor: `${stage.color}15`,
                      color: stage.color,
                      border: `1px solid ${stage.color}35`,
                    }}
                  >
                    {stage.personaName}
                  </span>
                </div>
              </button>

              {/* Children (Stage Mind Tree - 16px Indentation) */}
              {stage.isExpanded && (
                <div className="pl-4 pr-1 py-0.5 space-y-0.5 border-l border-obsidian-800 ml-2.5 my-0.5">
                  {/* 1. Persona File (persona.md) */}
                  {personaFile && (
                    <FileTreeItem
                      file={personaFile}
                      isActive={activeFile?.id === personaFile.id}
                      icon={<Sparkles className="h-3 w-3 text-cyber-purple shrink-0" />}
                      badge=".md"
                      onSelect={() => openFile(personaFile.id)}
                    />
                  )}

                  {/* 2. Rules Subfolder */}
                  {ruleFiles.length > 0 && (
                    <div className="space-y-0.5">
                      <button
                        onClick={() => toggleRulesFolder(stage.stageId)}
                        className="w-full flex items-center space-x-1.5 px-1.5 py-0.5 rounded-sm text-left text-slate-400 hover:text-slate-200 hover:bg-obsidian-800/70 transition cursor-pointer"
                      >
                        {areRulesOpen ? (
                          <FolderOpen className="h-3 w-3 text-cyber-amber shrink-0" />
                        ) : (
                          <Folder className="h-3 w-3 text-cyber-amber shrink-0" />
                        )}
                        <span className="text-xs font-mono">rules/</span>
                        <span className="text-[10px] text-slate-500 tabular-nums font-mono">({ruleFiles.length})</span>
                      </button>

                      {areRulesOpen && (
                        <div className="pl-4 space-y-0.5 border-l border-obsidian-800 ml-2">
                          {ruleFiles.map((ruleFile) => (
                            <FileTreeItem
                              key={ruleFile.id}
                              file={ruleFile}
                              isActive={activeFile?.id === ruleFile.id}
                              icon={<FileText className="h-3 w-3 text-cyber-cyan shrink-0" />}
                              badge=".md"
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
                      icon={<Settings className="h-3 w-3 text-cyber-amber shrink-0" />}
                      badge=".json"
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
      <div className="p-2 border-t border-obsidian-800 bg-obsidian-850/80 text-[10px] text-slate-400 font-mono flex items-center justify-between">
        <span className="flex items-center space-x-1.5">
          <span className="h-1.5 w-1.5 rounded-full bg-cyber-emerald animate-pulse" />
          <span>Worktree Sync</span>
        </span>
        <span className="text-slate-500">Tauri v2 IPC</span>
      </div>
    </aside>
  );
}

function FileTreeItem({
  file,
  isActive,
  icon,
  badge,
  onSelect,
}: {
  file: CortexAgentFile;
  isActive: boolean;
  icon: React.ReactNode;
  badge: string;
  onSelect: () => void;
}) {
  return (
    <button
      onClick={onSelect}
      className={`w-full flex items-center justify-between px-1.5 py-0.5 rounded-sm text-left transition cursor-pointer group ${
        isActive
          ? 'bg-cyber-cyan/15 text-cyber-cyan font-medium border-l-2 border-cyber-cyan'
          : 'text-slate-300 hover:text-white hover:bg-obsidian-800'
      }`}
      title={file.path}
    >
      <div className="flex items-center space-x-1.5 min-w-0">
        {icon}
        <span className="text-xs font-mono truncate">{file.name}</span>
      </div>

      <div className="flex items-center space-x-1 shrink-0">
        <span
          className={`px-1 py-0.2 rounded text-[9px] font-mono font-bold uppercase tracking-tight tabular-nums ${
            badge === '.json'
              ? 'bg-amber-950/40 text-cyber-amber border border-amber-500/30'
              : 'bg-obsidian-950 text-slate-400 border border-obsidian-800'
          }`}
        >
          {badge}
        </span>
        {file.isDirty && (
          <span className="h-1.5 w-1.5 rounded-full bg-cyber-amber" title="Unsaved changes" />
        )}
      </div>
    </button>
  );
}
