import { useState } from 'react';
import { useToolPolicyStore, ALL_TOOLS } from '../../stores/toolPolicyStore';
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
  Sparkles,
} from 'lucide-react';

export default function ToolPolicyModal() {
  const {
    isModalOpen,
    globalEnabled,
    stagePermissions,
    closeModal,
    toggleGlobalTool,
    toggleStageTool,
    calculateSavedTokensForStage,
  } = useToolPolicyStore();

  const { columns } = useBoardStore();
  const [activeTab, setActiveTab] = useState<'global' | 'stages'>('global');
  const [selectedStageId, setSelectedStageId] = useState<string>('dev');

  if (!isModalOpen) return null;

  const currentStage = columns.find((c) => c.id === selectedStageId) || columns[0];
  const stageSavedTokens = currentStage ? calculateSavedTokensForStage(currentStage.id) : 0;

  // Total global savings
  const totalGlobalSavedTokens = ALL_TOOLS.reduce((sum, tool) => {
    return sum + (!globalEnabled[tool.id] ? tool.tokenWeight : 0);
  }, 0);

  const getCategoryIcon = (category: string) => {
    switch (category) {
      case 'execution':
        return <Terminal className="h-4 w-4 text-cyber-cyan" />;
      case 'filesystem':
        return <FileCode className="h-4 w-4 text-cyber-amber" />;
      case 'web':
        return <Globe className="h-4 w-4 text-cyber-purple" />;
      case 'orchestration':
        return <GitFork className="h-4 w-4 text-cyber-emerald" />;
      default:
        return <Cpu className="h-4 w-4 text-slate-400" />;
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-xs p-4">
      <div className="w-full max-w-3xl rounded-xl border border-slate-800 bg-obsidian-900 shadow-2xl overflow-hidden flex flex-col max-h-[85vh]">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-slate-800 px-6 py-4 bg-obsidian-850">
          <div className="flex items-center space-x-3">
            <div className="p-2 rounded-lg bg-obsidian-800 border border-slate-750 text-cyber-cyan">
              <Shield className="h-5 w-5" />
            </div>
            <div>
              <h2 className="text-base font-bold text-slate-100 flex items-center space-x-2">
                <span>Матрица управления инструментами</span>
                <span className="text-xs px-2 py-0.5 rounded-full bg-cyan-950/70 border border-cyan-800/60 text-cyber-cyan font-mono">
                  Tool Policy Matrix
                </span>
              </h2>
              <p className="text-xs text-slate-400 mt-0.5">
                Глобальные переключатели и разграничение прав по колонкам канбана
              </p>
            </div>
          </div>
          <button
            onClick={closeModal}
            className="p-1.5 rounded-lg text-slate-400 hover:text-white hover:bg-obsidian-750 transition"
          >
            <X className="h-5 w-5" />
          </button>
        </div>

        {/* Live Token Savings Banner */}
        <div className="bg-obsidian-950/90 border-b border-slate-800 px-6 py-3 flex items-center justify-between text-xs">
          <div className="flex items-center space-x-2 text-cyber-amber font-mono font-semibold">
            <Zap className="h-4 w-4 fill-cyber-amber" />
            <span>
              {activeTab === 'global'
                ? `⚡ ~${totalGlobalSavedTokens.toLocaleString()} токенов экономится на каждом шаге`
                : `⚡ ~${stageSavedTokens.toLocaleString()} токенов экономится в колонке «${currentStage?.title}»`}
            </span>
          </div>
          <div className="flex items-center space-x-1.5 text-[11px] text-slate-400 font-mono">
            <span className="h-2 w-2 rounded-full bg-cyber-emerald animate-pulse" />
            <span>Rust Zero-Trust Barrier Active</span>
          </div>
        </div>

        {/* Tabs Bar */}
        <div className="flex border-b border-slate-800 bg-obsidian-850/50 px-6 pt-2">
          <button
            onClick={() => setActiveTab('global')}
            className={`flex items-center space-x-2 px-4 py-2.5 text-xs font-semibold border-b-2 transition ${
              activeTab === 'global'
                ? 'border-cyber-cyan text-cyber-cyan'
                : 'border-transparent text-slate-400 hover:text-slate-200'
            }`}
          >
            <Shield className="h-3.5 w-3.5" />
            <span>Глобальный выключатель (Settings ➔ Tools)</span>
          </button>
          <button
            onClick={() => setActiveTab('stages')}
            className={`flex items-center space-x-2 px-4 py-2.5 text-xs font-semibold border-b-2 transition ${
              activeTab === 'stages'
                ? 'border-cyber-cyan text-cyber-cyan'
                : 'border-transparent text-slate-400 hover:text-slate-200'
            }`}
          >
            <Layers className="h-3.5 w-3.5" />
            <span>Права по колонкам (Per-Stage Permissions)</span>
          </button>
        </div>

        {/* Body */}
        <div className="flex-1 overflow-y-auto p-6 space-y-4">
          {/* TAB 1: GLOBAL */}
          {activeTab === 'global' && (
            <div className="space-y-3">
              <div className="text-xs text-slate-400 mb-2">
                Отключенные инструменты <strong className="text-slate-200">полностью исключаются из JSON-схем</strong> запроса к LLM. Это гарантирует 0% галлюцинаций и сохраняет до 3 000 токенов контекста за ход.
              </div>

              {ALL_TOOLS.map((tool) => {
                const isEnabled = globalEnabled[tool.id] ?? false;
                return (
                  <div
                    key={tool.id}
                    onClick={() => toggleGlobalTool(tool.id)}
                    className={`flex items-center justify-between p-3.5 rounded-lg border cursor-pointer transition select-none ${
                      isEnabled
                        ? 'border-slate-750 bg-obsidian-850 hover:border-slate-600'
                        : 'border-slate-850 bg-obsidian-950/60 opacity-60 hover:opacity-80'
                    }`}
                  >
                    <div className="flex items-center space-x-3.5">
                      <div className="p-2 rounded bg-obsidian-900 border border-slate-800">
                        {getCategoryIcon(tool.category)}
                      </div>
                      <div>
                        <div className="flex items-center space-x-2">
                          <span className="font-mono font-bold text-sm text-slate-200">
                            {tool.name}
                          </span>
                          {tool.aliases.length > 0 && (
                            <span className="font-mono text-[10px] text-slate-500">
                              (алиасы: {tool.aliases.join(', ')})
                            </span>
                          )}
                        </div>
                        <p className="text-xs text-slate-400 mt-0.5">{tool.description}</p>
                      </div>
                    </div>

                    <div className="flex items-center space-x-3">
                      <span className="font-mono text-[11px] text-slate-500 tabular-nums">
                        ~{tool.tokenWeight} tokens
                      </span>
                      <div
                        className={`w-11 h-6 flex items-center rounded-full p-1 transition duration-200 ease-in-out ${
                          isEnabled ? 'bg-cyber-cyan justify-end' : 'bg-slate-750 justify-start'
                        }`}
                      >
                        <div
                          className={`bg-white w-4 h-4 rounded-full shadow-md transform transition duration-200 ease-in-out ${
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

          {/* TAB 2: PER-STAGE */}
          {activeTab === 'stages' && (
            <div className="space-y-4">
              {/* Stage selector pills */}
              <div className="flex items-center space-x-2 overflow-x-auto pb-1">
                {columns.map((col) => (
                  <button
                    key={col.id}
                    onClick={() => setSelectedStageId(col.id)}
                    className={`px-3 py-1.5 rounded-lg font-mono text-xs font-semibold whitespace-nowrap transition flex items-center space-x-1.5 ${
                      selectedStageId === col.id
                        ? 'bg-cyber-cyan text-obsidian-950 shadow-xs'
                        : 'bg-obsidian-800 text-slate-400 hover:text-slate-200 border border-slate-750'
                    }`}
                  >
                    <span>{col.title}</span>
                    <span className="text-[10px] opacity-75">({col.persona})</span>
                  </button>
                ))}
              </div>

              {/* Stage info notice */}
              <div className="p-3 rounded-lg bg-obsidian-950 border border-slate-800 text-xs text-slate-300">
                Настройки прав для колонки <strong className="text-cyber-cyan">{currentStage?.title}</strong>. 
                {currentStage?.id === 'review' && (
                  <span className="text-cyber-amber block mt-1">
                    🛡️ В колонке Review Gate инструменты модификации файлов (edit_file, write_file) заблокированы намертво. Ревьюер только анализирует код!
                  </span>
                )}
              </div>

              {/* Tools for selected stage */}
              <div className="space-y-2.5">
                {ALL_TOOLS.map((tool) => {
                  const isGloballyActive = globalEnabled[tool.id] ?? false;
                  const stageMap = stagePermissions[selectedStageId] || {};
                  const isStageAllowed = stageMap[tool.id] !== undefined ? stageMap[tool.id] : isGloballyActive;

                  return (
                    <div
                      key={tool.id}
                      onClick={() => isGloballyActive && toggleStageTool(selectedStageId, tool.id)}
                      className={`flex items-center justify-between p-3 rounded-lg border transition ${
                        !isGloballyActive
                          ? 'border-slate-850 bg-obsidian-950/40 opacity-40 cursor-not-allowed'
                          : isStageAllowed
                          ? 'border-slate-750 bg-obsidian-850 cursor-pointer hover:border-slate-600'
                          : 'border-rose-950/50 bg-rose-950/10 cursor-pointer opacity-70 hover:opacity-90'
                      }`}
                    >
                      <div className="flex items-center space-x-3">
                        <div className="p-1.5 rounded bg-obsidian-900 border border-slate-800">
                          {getCategoryIcon(tool.category)}
                        </div>
                        <div>
                          <span className="font-mono font-bold text-xs text-slate-200">
                            {tool.name}
                          </span>
                          {!isGloballyActive && (
                            <span className="text-[10px] text-rose-400 ml-2 font-mono">
                              (Выключен глобально)
                            </span>
                          )}
                        </div>
                      </div>

                      <div className="flex items-center space-x-2">
                        {isStageAllowed && isGloballyActive ? (
                          <span className="flex items-center space-x-1 px-2 py-0.5 rounded text-[11px] font-mono font-semibold bg-emerald-500/15 border border-emerald-500/40 text-emerald-400">
                            <CheckCircle2 className="h-3 w-3" />
                            <span>РАЗРЕШЁН</span>
                          </span>
                        ) : (
                          <span className="flex items-center space-x-1 px-2 py-0.5 rounded text-[11px] font-mono font-semibold bg-rose-500/15 border border-rose-500/40 text-rose-400">
                            <XCircle className="h-3 w-3" />
                            <span>ЗАБЛОКИРОВАН</span>
                          </span>
                        )}
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="border-t border-slate-800 bg-obsidian-850 px-6 py-3.5 flex items-center justify-between">
          <span className="text-xs text-slate-400">
            Политика применяется динамически на лету перед каждым вызовом LLM.
          </span>
          <button
            onClick={closeModal}
            className="px-4 py-2 rounded-lg bg-cyber-cyan hover:bg-cyan-400 text-obsidian-950 font-bold text-xs transition active:scale-95"
          >
            Применить и Закрыть
          </button>
        </div>
      </div>
    </div>
  );
}
