import { useState } from 'react';
import { useToolPolicyStore, AVAILABLE_TOOLS, ToolMeta } from '../../stores/toolPolicyStore';
import { useBoardStore } from '../../stores/boardStore';
import {
  X,
  Shield,
  Zap,
  CheckCircle2,
  XCircle,
  Terminal,
  FileCode,
  Globe,
  GitFork,
  Cpu,
  Layers,
  Table,
} from 'lucide-react';

export default function ToolPolicyModal() {
  const {
    isModalOpen,
    globalEnabled,
    closeModal,
    toggleGlobalTool,
    setStageTool,
    isToolAllowed,
    calculateSavedTokens,
  } = useToolPolicyStore();

  const { columns } = useBoardStore();
  const [activeTab, setActiveTab] = useState<'matrix' | 'global'>('matrix');

  if (!isModalOpen) return null;

  const totalGlobalSavedTokens = calculateSavedTokens();

  const getCategoryIcon = (category: ToolMeta['category']) => {
    switch (category) {
      case 'system':
        return <Terminal className="h-3.5 w-3.5 text-cyber-cyan shrink-0" />;
      case 'fs':
        return <FileCode className="h-3.5 w-3.5 text-cyber-amber shrink-0" />;
      case 'web':
        return <Globe className="h-3.5 w-3.5 text-cyber-purple shrink-0" />;
      case 'agent':
        return <GitFork className="h-3.5 w-3.5 text-cyber-emerald shrink-0" />;
      default:
        return <Cpu className="h-3.5 w-3.5 text-slate-400 shrink-0" />;
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 p-3 animate-in fade-in duration-100">
      <div className="w-full max-w-4xl rounded-lg border border-obsidian-750 bg-obsidian-900 shadow-2xl overflow-hidden flex flex-col max-h-[88vh]">
        {/* Header - Max 16px title */}
        <div className="flex items-center justify-between border-b border-obsidian-800 px-4 py-2.5 bg-obsidian-850">
          <div className="flex items-center space-x-2.5">
            <div className="p-1.5 rounded-sm bg-obsidian-800 border border-obsidian-750 text-cyber-cyan">
              <Shield className="h-4 w-4" />
            </div>
            <div>
              <div className="flex items-center space-x-2">
                <h2 className="text-sm font-semibold text-slate-100">
                  Tool Policy Matrix
                </h2>
                <span className="text-[10px] px-1.5 py-0.2 rounded font-mono font-bold bg-cyan-950/70 border border-cyan-800/60 text-cyber-cyan tabular-nums">
                  Deterministic Prefix Barrier
                </span>
              </div>
              <p className="text-[11px] text-slate-400 font-mono mt-0.5">
                Dynamic JSON-Schema exclusion & per-stage permission grid
              </p>
            </div>
          </div>
          <button
            onClick={closeModal}
            className="p-1 rounded text-slate-400 hover:text-white hover:bg-obsidian-750 transition cursor-pointer"
            title="Close (Esc)"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Live Token Savings Banner */}
        <div className="bg-obsidian-950 border-b border-obsidian-800 px-4 py-2 flex items-center justify-between text-xs font-mono">
          <div className="flex items-center space-x-2 text-cyber-amber font-semibold tabular-nums">
            <Zap className="h-3.5 w-3.5 fill-cyber-amber shrink-0" />
            <span>⚡ ~{totalGlobalSavedTokens.toLocaleString()} tokens saved/step via deterministic JSON exclusion</span>
          </div>
          <div className="flex items-center space-x-1.5 text-[10px] text-slate-400">
            <span className="h-1.5 w-1.5 rounded-full bg-cyber-emerald animate-pulse" />
            <span>Rust Zero-Trust Boundary Active</span>
          </div>
        </div>

        {/* Tabs Bar */}
        <div className="flex border-b border-obsidian-800 bg-obsidian-850/60 px-4 pt-1 font-mono text-xs">
          <button
            onClick={() => setActiveTab('matrix')}
            className={`flex items-center space-x-1.5 px-3 py-2 font-semibold border-b-2 transition cursor-pointer ${
              activeTab === 'matrix'
                ? 'border-cyber-cyan text-cyber-cyan'
                : 'border-transparent text-slate-400 hover:text-slate-200'
            }`}
          >
            <Table className="h-3.5 w-3.5" />
            <span>Tabular Stage Matrix (Stages ➔ Tools Grid)</span>
          </button>
          <button
            onClick={() => setActiveTab('global')}
            className={`flex items-center space-x-1.5 px-3 py-2 font-semibold border-b-2 transition cursor-pointer ${
              activeTab === 'global'
                ? 'border-cyber-cyan text-cyber-cyan'
                : 'border-transparent text-slate-400 hover:text-slate-200'
            }`}
          >
            <Layers className="h-3.5 w-3.5" />
            <span>Global Tool Registry</span>
          </button>
        </div>

        {/* Body Content */}
        <div className="flex-1 overflow-y-auto p-4 space-y-3 font-mono text-xs">
          {/* TAB 1: CRISP TABULAR GRID MATRIX */}
          {activeTab === 'matrix' && (
            <div className="space-y-3">
              <div className="flex items-center justify-between text-[11px] text-slate-400 pb-1">
                <span>
                  Matrix compares tools against each pipeline stage. Click cell to toggle stage authorization.
                </span>
                <span className="text-[10px] text-cyber-amber">
                  🛡️ Review stage locks write/edit tools
                </span>
              </div>

              {/* Crisp Tabular Grid */}
              <div className="overflow-x-auto rounded-md border border-obsidian-800 bg-obsidian-950">
                <table className="w-full text-left border-collapse font-mono text-xs">
                  <thead>
                    <tr className="border-b border-obsidian-800 bg-obsidian-900 text-slate-400 text-[10px] uppercase tracking-wider">
                      <th className="py-2 px-3 font-semibold">Tool Identifier</th>
                      <th className="py-2 px-2 text-right font-semibold">Weight</th>
                      {columns.map((col) => {
                        const savedInStage = calculateSavedTokens(col.id);
                        return (
                          <th key={col.id} className="py-2 px-2 text-center font-semibold border-l border-obsidian-800/80">
                            <div className="flex flex-col items-center">
                              <span style={{ color: col.color }} className="truncate max-w-[80px]">
                                {col.title}
                              </span>
                              <span className="text-[9px] text-slate-500 tabular-nums">
                                -{savedInStage}t
                              </span>
                            </div>
                          </th>
                        );
                      })}
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-obsidian-800/80">
                    {AVAILABLE_TOOLS.map((tool) => {
                      const isGloballyActive = globalEnabled[tool.id] ?? false;

                      return (
                        <tr
                          key={tool.id}
                          className="hover:bg-obsidian-850/60 transition group text-slate-200"
                        >
                          {/* Tool Name & Category */}
                          <td className="py-2 px-3">
                            <div className="flex items-center space-x-2">
                              {getCategoryIcon(tool.category)}
                              <div>
                                <span className="font-bold text-slate-100 group-hover:text-cyber-cyan transition">
                                  {tool.name}
                                </span>
                                {!isGloballyActive && (
                                  <span className="text-[9px] text-rose-400 ml-1.5">
                                    (disabled globally)
                                  </span>
                                )}
                              </div>
                            </div>
                          </td>

                          {/* Token Weight */}
                          <td className="py-2 px-2 text-right text-slate-400 tabular-nums text-[11px]">
                            ~{tool.tokenWeight}
                          </td>

                          {/* Per-Stage Authorization Cells */}
                          {columns.map((col) => {
                            const isAllowed = isToolAllowed(tool.id, col.id);
                            const isReviewLock = col.id === 'review' && (tool.id === 'edit_file' || tool.id === 'write_file');

                            return (
                              <td
                                key={col.id}
                                className="py-1.5 px-2 text-center border-l border-obsidian-800/80"
                              >
                                {isReviewLock ? (
                                  <span
                                    className="inline-flex items-center px-1.5 py-0.5 rounded text-[9px] font-bold bg-rose-950/40 border border-rose-800/50 text-rose-400 cursor-not-allowed select-none"
                                    title="Locked by Review Invariant: Reviewer cannot modify code files"
                                  >
                                    LOCKED
                                  </span>
                                ) : !isGloballyActive ? (
                                  <span
                                    className="inline-flex items-center px-1.5 py-0.5 rounded text-[9px] font-semibold bg-obsidian-900 border border-obsidian-800 text-slate-600 select-none"
                                    title="Disabled globally"
                                  >
                                    OFF
                                  </span>
                                ) : (
                                  <button
                                    onClick={() => setStageTool(col.id, tool.id, !isAllowed)}
                                    className={`inline-flex items-center space-x-0.5 px-1.5 py-0.5 rounded text-[10px] font-bold transition active:scale-[0.98] cursor-pointer ${
                                      isAllowed
                                        ? 'bg-emerald-950/40 border border-emerald-500/40 text-cyber-emerald hover:bg-emerald-900/40'
                                        : 'bg-rose-950/30 border border-rose-800/40 text-rose-400 hover:bg-rose-900/30'
                                    }`}
                                    title={`Toggle ${tool.name} for ${col.title}`}
                                  >
                                    {isAllowed ? (
                                      <>
                                        <CheckCircle2 className="h-2.5 w-2.5" />
                                        <span>ALLOW</span>
                                      </>
                                    ) : (
                                      <>
                                        <XCircle className="h-2.5 w-2.5" />
                                        <span>DENY</span>
                                      </>
                                    )}
                                  </button>
                                )}
                              </td>
                            );
                          })}
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            </div>
          )}

          {/* TAB 2: GLOBAL TOOL REGISTRY */}
          {activeTab === 'global' && (
            <div className="space-y-2.5">
              <div className="text-[11px] text-slate-400 mb-2">
                Disabled tools are <strong className="text-slate-200">completely stripped from LLM JSON schemas</strong>, eliminating hallucinated tool calls and saving token budget.
              </div>

              {AVAILABLE_TOOLS.map((tool) => {
                const isEnabled = globalEnabled[tool.id] ?? false;
                return (
                  <div
                    key={tool.id}
                    onClick={() => toggleGlobalTool(tool.id)}
                    className={`flex items-center justify-between p-2.5 rounded-md border cursor-pointer transition select-none ${
                      isEnabled
                        ? 'border-obsidian-750 bg-obsidian-850 hover:border-obsidian-700'
                        : 'border-obsidian-800 bg-obsidian-950/60 opacity-60 hover:opacity-80'
                    }`}
                  >
                    <div className="flex items-center space-x-2.5">
                      <div className="p-1.5 rounded-sm bg-obsidian-900 border border-obsidian-800">
                        {getCategoryIcon(tool.category)}
                      </div>
                      <div>
                        <div className="flex items-center space-x-2">
                          <span className="font-mono font-bold text-xs text-slate-200">
                            {tool.label}
                          </span>
                          <span className="font-mono text-[10px] text-slate-500">
                            ({tool.name})
                          </span>
                        </div>
                        <p className="text-[11px] text-slate-400 mt-0.5 font-sans leading-snug">{tool.description}</p>
                      </div>
                    </div>

                    <div className="flex items-center space-x-3">
                      <span className="font-mono text-[10px] text-slate-500 tabular-nums">
                        ~{tool.tokenWeight} tokens
                      </span>
                      <div
                        className={`w-9 h-5 flex items-center rounded-full p-0.5 transition duration-150 ease-in-out ${
                          isEnabled ? 'bg-cyber-cyan justify-end' : 'bg-obsidian-750 justify-start'
                        }`}
                      >
                        <div
                          className={`w-4 h-4 rounded-full shadow-sm transform transition duration-150 ${
                            isEnabled ? 'bg-obsidian-950' : 'bg-slate-400'
                          }`}
                        />
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="border-t border-obsidian-800 bg-obsidian-850 px-4 py-2.5 flex items-center justify-between font-mono text-xs">
          <span className="text-[11px] text-slate-400">
            Policies enforced on-the-fly via Rust IPC barrier before SSE dispatch.
          </span>
          <button
            onClick={closeModal}
            className="px-3 py-1.5 rounded-sm bg-cyber-cyan hover:bg-cyan-400 text-obsidian-950 font-bold text-xs transition active:scale-[0.98] cursor-pointer"
          >
            Apply & Close
          </button>
        </div>
      </div>
    </div>
  );
}
