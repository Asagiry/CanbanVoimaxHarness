import { useState } from 'react';
import { useToolPolicyStore, AVAILABLE_TOOLS } from '../../stores/toolPolicyStore';
import { useBoardStore } from '../../stores/boardStore';
import {
  ShieldAlert,
  ShieldCheck,
  X,
  Zap,
  Terminal,
  FileCode,
  Globe,
  Bot,
  RotateCcw,
  Lock,
} from 'lucide-react';

export default function ToolPolicyModal() {
  const {
    isModalOpen,
    closeModal,
    globalEnabled,
    stageOverrides,
    toggleGlobalTool,
    setStageTool,
    resetDefaults,
    calculateSavedTokens,
  } = useToolPolicyStore();

  const { columns } = useBoardStore();
  const [selectedStageTab, setSelectedStageTab] = useState<string>('global');

  if (!isModalOpen) return null;

  const getToolIcon = (category: string) => {
    switch (category) {
      case 'system':
        return <Terminal className="h-4 w-4 text-cyber-cyan" />;
      case 'fs':
        return <FileCode className="h-4 w-4 text-cyber-amber" />;
      case 'web':
        return <Globe className="h-4 w-4 text-cyber-purple" />;
      case 'agent':
        return <Bot className="h-4 w-4 text-cyber-emerald" />;
      default:
        return <Terminal className="h-4 w-4 text-slate-400" />;
    }
  };

  const isGlobalTab = selectedStageTab === 'global';
  const savedTokens = calculateSavedTokens(isGlobalTab ? undefined : selectedStageTab);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-xs p-4 animate-in fade-in duration-150">
      <div className="relative w-full max-w-3xl rounded-xl border border-slate-800 bg-obsidian-900 shadow-2xl flex flex-col max-h-[90vh] overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-slate-800 bg-obsidian-850 px-5 py-3.5">
          <div className="flex items-center space-x-2.5">
            <div className="p-1.5 rounded-lg bg-cyber-cyan/15 border border-cyber-cyan/40 text-cyber-cyan">
              <ShieldAlert className="h-4 w-4" />
            </div>
            <div>
              <h3 className="text-sm font-bold text-slate-100 flex items-center space-x-2">
                <span>Tool Policy Matrix</span>
                <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-cyan-950 text-cyber-cyan border border-cyan-800/60">
                  Zero-Trust Barrier
                </span>
              </h3>
              <p className="text-[11px] text-slate-400">
                Level A (Global) and Level B (Per-Stage) agent tool execution permissions
              </p>
            </div>
          </div>

          <button
            onClick={closeModal}
            className="p-1 rounded-md text-slate-400 hover:text-white hover:bg-obsidian-750 transition"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Token Savings Telemetry Banner */}
        <div className="flex items-center justify-between bg-cyan-950/40 border-b border-cyan-900/40 px-5 py-2 text-xs font-mono">
          <div className="flex items-center space-x-2 text-cyber-cyan">
            <Zap className="h-3.5 w-3.5 fill-cyber-cyan" />
            <span className="font-bold">
              ⚡ ~{savedTokens.toLocaleString()} tokens saved per step by excluding disabled tools
            </span>
          </div>
          <span className="text-[10px] text-slate-400 hidden sm:inline">
            Deterministic Prefix Caching enabled
          </span>
        </div>

        {/* Stage Tabs (Level A Global + Level B Columns) */}
        <div className="flex items-center space-x-1 border-b border-slate-800 bg-obsidian-950 px-5 pt-2 text-xs font-medium overflow-x-auto">
          <button
            onClick={() => setSelectedStageTab('global')}
            className={`flex items-center space-x-1.5 px-3 py-2 border-b-2 transition select-none ${
              isGlobalTab
                ? 'border-cyber-cyan text-cyber-cyan font-bold'
                : 'border-transparent text-slate-400 hover:text-slate-200'
            }`}
          >
            <Lock className="h-3.5 w-3.5" />
            <span>Level A: Global</span>
          </button>

          {columns.map((col) => {
            const isSelected = selectedStageTab === col.id;
            return (
              <button
                key={col.id}
                onClick={() => setSelectedStageTab(col.id)}
                className={`flex items-center space-x-1.5 px-3 py-2 border-b-2 transition select-none ${
                  isSelected
                    ? 'border-cyber-cyan text-cyber-cyan font-bold'
                    : 'border-transparent text-slate-400 hover:text-slate-200'
                }`}
              >
                <span
                  className="h-2 w-2 rounded-full"
                  style={{ backgroundColor: col.color || '#64748b' }}
                />
                <span>Stage: {col.title}</span>
              </button>
            );
          })}
        </div>

        {/* Tools Matrix List */}
        <div className="flex-1 overflow-y-auto p-5 space-y-2.5 bg-obsidian-900/90 text-xs">
          <div className="flex items-center justify-between pb-1 text-[11px] font-mono text-slate-400 uppercase tracking-wider">
            <span>Tool Specification</span>
            <span>Security &amp; Token Status</span>
          </div>

          {AVAILABLE_TOOLS.map((tool) => {
            let isAllowed = globalEnabled[tool.id] ?? true;
            if (!isGlobalTab) {
              const stageMap = stageOverrides[selectedStageTab];
              if (stageMap && typeof stageMap[tool.id] === 'boolean') {
                isAllowed = stageMap[tool.id];
              } else {
                isAllowed = globalEnabled[tool.id] ?? true;
              }
            }

            const handleToggle = () => {
              if (isGlobalTab) {
                toggleGlobalTool(tool.id);
              } else {
                setStageTool(selectedStageTab, tool.id, !isAllowed);
              }
            };

            return (
              <div
                key={tool.id}
                onClick={handleToggle}
                className={`flex items-center justify-between p-3 rounded-lg border transition cursor-pointer select-none ${
                  isAllowed
                    ? 'border-slate-800 bg-obsidian-850 hover:bg-obsidian-800/90 hover:border-slate-700'
                    : 'border-rose-950/40 bg-rose-950/10 hover:bg-rose-950/20'
                }`}
              >
                <div className="flex items-start space-x-3 min-w-0 pr-4">
                  <div className="p-2 rounded bg-obsidian-800 border border-slate-750 shrink-0 mt-0.5">
                    {getToolIcon(tool.category)}
                  </div>
                  <div>
                    <div className="flex items-center space-x-2">
                      <span className="font-mono font-bold text-slate-200">{tool.label}</span>
                      <code className="text-[10px] text-slate-400 bg-obsidian-950 px-1.5 py-0.5 rounded border border-slate-800">
                        {tool.name}
                      </code>
                    </div>
                    <p className="text-[11px] text-slate-400 mt-0.5 leading-snug">
                      {tool.description}
                    </p>
                  </div>
                </div>

                <div className="flex items-center space-x-3 shrink-0">
                  <span className="font-mono text-[10px] text-slate-500 tabular-nums">
                    ~{tool.tokenWeight} tokens
                  </span>

                  <button
                    type="button"
                    onClick={(e) => {
                      e.stopPropagation();
                      handleToggle();
                    }}
                    className={`relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-hidden ${
                      isAllowed ? 'bg-cyber-emerald' : 'bg-slate-700'
                    }`}
                  >
                    <span
                      aria-hidden="true"
                      className={`pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow-sm ring-0 transition duration-200 ease-in-out ${
                        isAllowed ? 'translate-x-4' : 'translate-x-0'
                      }`}
                    />
                  </button>
                </div>
              </div>
            );
          })}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between border-t border-slate-800 bg-obsidian-850 px-5 py-3 text-xs">
          <button
            onClick={resetDefaults}
            className="flex items-center space-x-1.5 text-slate-400 hover:text-slate-200 transition"
          >
            <RotateCcw className="h-3.5 w-3.5" />
            <span>Reset Defaults</span>
          </button>

          <div className="flex items-center space-x-2">
            <button
              onClick={closeModal}
              className="px-4 py-1.5 rounded-lg bg-cyber-cyan hover:bg-cyan-400 text-obsidian-950 font-bold transition flex items-center space-x-1.5 shadow-sm shadow-cyan-900/40"
            >
              <ShieldCheck className="h-4 w-4" />
              <span>Apply Policies</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
