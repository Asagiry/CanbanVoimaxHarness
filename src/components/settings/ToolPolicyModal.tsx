import { useState } from 'react';
import { useToolPolicyStore, ALL_TOOLS } from '../../stores/toolPolicyStore';
import { useBoardStore } from '../../stores/boardStore';
import {
  X,
  Shield,
  Zap,
  RotateCcw,
  Check,
  Ban,
  Sliders,
  Sparkles,
  Columns,
  Cpu,
} from 'lucide-react';

export default function ToolPolicyModal() {
  const {
    isModalOpen,
    closeModal,
    policies,
    setGlobalTool,
    setStageTool,
    resetToDefaults,
    calculateTokenSavings,
  } = useToolPolicyStore();

  const { columns } = useBoardStore();
  const [activeTab, setActiveTab] = useState<'matrix' | 'global'>('matrix');
  const [savedFeedback, setSavedFeedback] = useState(false);

  if (!isModalOpen) return null;

  const activeColumns = columns.length > 0 ? columns : [
    { id: 'backlog', title: 'Backlog', color: '#64748b' },
    { id: 'dev', title: 'In Dev', color: '#f59e0b' },
    { id: 'review', title: 'Review', color: '#8b5cf6' },
    { id: 'retro', title: 'Retro', color: '#06b6d4' },
    { id: 'done', title: 'Done', color: '#10b981' },
  ];

  const tokenSavings = calculateTokenSavings();

  const handleSave = () => {
    setSavedFeedback(true);
    setTimeout(() => {
      setSavedFeedback(false);
      closeModal();
    }, 800);
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 backdrop-blur-xs p-4 animate-in fade-in duration-150">
      <div className="relative flex flex-col w-[920px] max-w-full max-h-[90vh] rounded-xl border border-slate-750 bg-obsidian-900 shadow-2xl overflow-hidden text-slate-200">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-slate-800 bg-obsidian-850 px-6 py-4">
          <div className="flex items-center space-x-3">
            <div className="p-2 rounded-lg bg-cyber-amber/15 border border-cyber-amber/40 text-cyber-amber shadow-sm shadow-amber-950/40">
              <Shield className="h-5 w-5" />
            </div>
            <div>
              <h2 className="text-base font-bold text-white flex items-center space-x-2">
                <span>Матрица управления инструментами</span>
                <span className="font-mono text-xs px-2 py-0.5 rounded bg-obsidian-950 border border-slate-750 text-slate-400 font-semibold">
                  Zero-Trust Iron Barrier
                </span>
              </h2>
              <p className="text-xs text-slate-400 mt-0.5">
                Поколоночные политики безопасности и оптимизация контекста LLM
              </p>
            </div>
          </div>

          <div className="flex items-center space-x-2">
            {/* Token Savings Badge */}
            <div className="flex items-center space-x-1.5 px-3 py-1.5 rounded-lg bg-cyber-emerald/10 border border-cyber-emerald/40 text-cyber-emerald font-mono text-xs font-bold shadow-xs">
              <Zap className="h-4 w-4 fill-cyber-emerald animate-pulse" />
              <span>⚡ ~{tokenSavings.toLocaleString()} tokens saved per step by excluding disabled tools</span>
            </div>

            <button
              onClick={closeModal}
              className="p-1.5 rounded-lg text-slate-400 hover:text-white hover:bg-obsidian-750 transition"
              title="Close modal"
            >
              <X className="h-4 w-4" />
            </button>
          </div>
        </div>

        {/* Navigation Tabs */}
        <div className="flex border-b border-slate-800 bg-obsidian-900/90 px-6 text-xs font-medium">
          <button
            onClick={() => setActiveTab('matrix')}
            className={`flex items-center space-x-2 py-3 px-4 border-b-2 transition select-none ${
              activeTab === 'matrix'
                ? 'border-cyber-cyan text-cyber-cyan font-bold'
                : 'border-transparent text-slate-400 hover:text-slate-200'
            }`}
          >
            <Columns className="h-4 w-4" />
            <span>Level B: Поколоночные разрешения (Per-Stage Matrix)</span>
          </button>
          <button
            onClick={() => setActiveTab('global')}
            className={`flex items-center space-x-2 py-3 px-4 border-b-2 transition select-none ${
              activeTab === 'global'
                ? 'border-cyber-cyan text-cyber-cyan font-bold'
                : 'border-transparent text-slate-400 hover:text-slate-200'
            }`}
          >
            <Sliders className="h-4 w-4" />
            <span>Level A: Глобальные переключатели (Global Toggles)</span>
          </button>
        </div>

        {/* Content Body */}
        <div className="flex-1 overflow-y-auto p-6 space-y-6">
          {/* TAB 1: PER-STAGE MATRIX */}
          {activeTab === 'matrix' && (
            <div className="space-y-4">
              <div className="p-3 rounded-lg bg-obsidian-850 border border-slate-800 text-xs text-slate-300 flex items-start space-x-2.5">
                <Sparkles className="h-4 w-4 text-cyber-cyan shrink-0 mt-0.5" />
                <div className="leading-relaxed">
                  <strong className="text-white">Железный барьер Zero-Trust:</strong> Инструменты, отключенные для текущей колонки,
                  <strong> полностью исключаются</strong> из системного JSON-схемы запроса к LLM. Это гарантирует 100% изоляцию действий
                  (например, запрет <code className="text-amber-400">write_file</code> / <code className="text-amber-400">edit_file</code> в Review)
                  и экономит тысячи токенов на каждом шаге.
                </div>
              </div>

              {/* Matrix Table */}
              <div className="overflow-x-auto rounded-lg border border-slate-800 bg-obsidian-950">
                <table className="w-full text-left border-collapse text-xs">
                  <thead>
                    <tr className="border-b border-slate-800 bg-obsidian-850">
                      <th className="py-2.5 px-3.5 font-bold text-slate-300 min-w-[200px]">Инструмент (Tool)</th>
                      <th className="py-2.5 px-2 font-mono text-[11px] text-slate-400 text-center">Global</th>
                      {activeColumns.map((col) => (
                        <th
                          key={col.id}
                          className="py-2.5 px-3 font-semibold text-center uppercase font-mono text-[11px]"
                          style={{ color: col.color || '#94a3b8' }}
                        >
                          {col.title}
                        </th>
                      ))}
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-slate-800/60 font-mono text-xs">
                    {ALL_TOOLS.map((tool) => {
                      const isGloballyActive = policies.global[tool.name] !== false;

                      return (
                        <tr
                          key={tool.name}
                          className="hover:bg-obsidian-850/50 transition-colors"
                        >
                          <td className="py-2.5 px-3.5 font-sans font-medium text-slate-200">
                            <div className="flex items-center space-x-1.5">
                              <span className="font-mono text-xs text-cyber-cyan">{tool.name}</span>
                              <span className="text-[10px] text-slate-500 font-mono">
                                (~{tool.tokens} tok)
                              </span>
                            </div>
                            <div className="text-[11px] text-slate-400 font-sans line-clamp-1 mt-0.5">
                              {tool.description}
                            </div>
                          </td>

                          {/* Global status indicator */}
                          <td className="py-2.5 px-2 text-center">
                            {isGloballyActive ? (
                              <span className="inline-flex items-center px-1.5 py-0.5 rounded bg-emerald-500/15 text-emerald-400 text-[10px] font-bold">
                                ON
                              </span>
                            ) : (
                              <span className="inline-flex items-center px-1.5 py-0.5 rounded bg-rose-500/15 text-rose-400 text-[10px] font-bold">
                                OFF
                              </span>
                            )}
                          </td>

                          {/* Per-Stage Checkboxes */}
                          {activeColumns.map((col) => {
                            const stageEnabled =
                              policies.stages[col.id]?.[tool.name] ??
                              (isGloballyActive && (col.id !== 'done' && (col.id !== 'review' || (tool.name !== 'write_file' && tool.name !== 'edit_file'))));

                            const effectiveEnabled = isGloballyActive && stageEnabled;

                            return (
                              <td key={col.id} className="py-2.5 px-3 text-center">
                                <button
                                  type="button"
                                  disabled={!isGloballyActive}
                                  onClick={() => setStageTool(col.id, tool.name, !stageEnabled)}
                                  className={`h-6 w-6 rounded flex items-center justify-center transition mx-auto ${
                                    !isGloballyActive
                                      ? 'opacity-30 cursor-not-allowed bg-slate-800 text-slate-600'
                                      : effectiveEnabled
                                      ? 'bg-cyber-emerald/20 border border-cyber-emerald/50 text-cyber-emerald hover:bg-emerald-500/30'
                                      : 'bg-rose-950/30 border border-rose-900/40 text-rose-400 hover:bg-rose-900/40'
                                  }`}
                                  title={`${tool.name} in ${col.title}: ${effectiveEnabled ? 'Enabled' : 'Disabled'}`}
                                >
                                  {effectiveEnabled ? (
                                    <Check className="h-3.5 w-3.5" />
                                  ) : (
                                    <Ban className="h-3 w-3" />
                                  )}
                                </button>
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

          {/* TAB 2: GLOBAL SWITCHES */}
          {activeTab === 'global' && (
            <div className="space-y-3">
              <div className="p-3 rounded-lg bg-obsidian-850 border border-slate-800 text-xs text-slate-300 flex items-center space-x-2">
                <Cpu className="h-4 w-4 text-cyber-amber shrink-0" />
                <span>
                  <strong>Глобальное отключение (Level A):</strong> Отключение инструмента на глобальном уровне
                  деактивирует его во всех стадиях без исключения и удаляет из исполняемого реестра.
                </span>
              </div>

              <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
                {ALL_TOOLS.map((tool) => {
                  const isEnabled = policies.global[tool.name] !== false;

                  return (
                    <div
                      key={tool.name}
                      onClick={() => setGlobalTool(tool.name, !isEnabled)}
                      className={`p-3.5 rounded-lg border transition cursor-pointer select-none flex items-start justify-between ${
                        isEnabled
                          ? 'border-slate-800 bg-obsidian-850 hover:border-slate-700'
                          : 'border-rose-900/40 bg-rose-950/15 hover:border-rose-800/60'
                      }`}
                    >
                      <div className="min-w-0 pr-3">
                        <div className="flex items-center space-x-2">
                          <span className="font-mono text-xs font-bold text-white">
                            {tool.label}
                          </span>
                          <span className="font-mono text-[10px] text-cyber-cyan px-1.5 py-0.5 rounded bg-obsidian-950">
                            ~{tool.tokens} tok
                          </span>
                        </div>
                        <p className="text-[11px] text-slate-400 mt-1 leading-snug">
                          {tool.description}
                        </p>
                      </div>

                      {/* Switch Pill */}
                      <div
                        className={`h-5 w-9 rounded-full transition-colors relative shrink-0 mt-1 ${
                          isEnabled ? 'bg-cyber-emerald' : 'bg-slate-700'
                        }`}
                      >
                        <div
                          className={`h-3.5 w-3.5 rounded-full bg-white transition-transform absolute top-0.5 ${
                            isEnabled ? 'right-0.5' : 'left-0.5'
                          }`}
                        />
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          )}
        </div>

        {/* Footer Actions */}
        <div className="flex items-center justify-between border-t border-slate-800 bg-obsidian-850 px-6 py-3.5">
          <button
            type="button"
            onClick={resetToDefaults}
            className="flex items-center space-x-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold text-slate-400 hover:text-white hover:bg-obsidian-800 border border-slate-750 transition"
          >
            <RotateCcw className="h-3.5 w-3.5" />
            <span>Сбросить по умолчанию</span>
          </button>

          <div className="flex items-center space-x-2.5">
            <button
              type="button"
              onClick={closeModal}
              className="px-4 py-1.5 rounded-lg text-xs font-semibold text-slate-300 hover:text-white hover:bg-obsidian-800 transition"
            >
              Отмена
            </button>
            <button
              type="button"
              onClick={handleSave}
              className="flex items-center space-x-1.5 px-4 py-1.5 rounded-lg text-xs font-bold bg-cyber-cyan hover:bg-cyan-400 text-obsidian-950 transition shadow-sm shadow-cyan-900/40"
            >
              {savedFeedback ? (
                <>
                  <Check className="h-4 w-4" />
                  <span>Сохранено!</span>
                </>
              ) : (
                <span>Применить политики</span>
              )}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
