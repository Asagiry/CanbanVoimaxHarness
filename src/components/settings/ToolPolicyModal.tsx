import { useState } from 'react';
import { useToolPolicyStore, AVAILABLE_TOOLS } from '../../stores/toolPolicyStore';
import { useBoardStore } from '../../stores/boardStore';
import { X } from 'lucide-react';

export default function ToolPolicyModal() {
  const {
    isModalOpen,
    globalEnabled,
    closeModal,
    toggleGlobalTool,
    setStageTool,
    isToolAllowed,
  } = useToolPolicyStore();

  const { columns } = useBoardStore();
  const [activeTab, setActiveTab] = useState<'matrix' | 'global'>('matrix');

  if (!isModalOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-3">
      <div className="w-full max-w-4xl rounded border border-obsidian-750 bg-obsidian-900 shadow-xl overflow-hidden flex flex-col max-h-[85vh]">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-obsidian-800 px-4 py-2.5 bg-obsidian-850">
          <div>
            <h2 className="text-xs font-semibold text-slate-100">
              Настройки матрицы инструментов
            </h2>
            <p className="text-[11px] text-slate-400 font-mono mt-0.5">
              Разграничение прав доступа инструментов по этапам
            </p>
          </div>
          <button
            onClick={closeModal}
            className="p-1 rounded text-slate-400 hover:text-white transition cursor-pointer"
            title="Закрыть"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Tabs Bar */}
        <div className="flex border-b border-obsidian-800 bg-obsidian-900 px-4 font-mono text-xs">
          <button
            onClick={() => setActiveTab('matrix')}
            className={`px-3 py-2 border-b-2 transition cursor-pointer ${
              activeTab === 'matrix'
                ? 'border-slate-300 text-slate-100 font-medium'
                : 'border-transparent text-slate-500 hover:text-slate-300'
            }`}
          >
            Матрица этапов
          </button>
          <button
            onClick={() => setActiveTab('global')}
            className={`px-3 py-2 border-b-2 transition cursor-pointer ${
              activeTab === 'global'
                ? 'border-slate-300 text-slate-100 font-medium'
                : 'border-transparent text-slate-500 hover:text-slate-300'
            }`}
          >
            Глобальный реестр
          </button>
        </div>

        {/* Body Content */}
        <div className="flex-1 overflow-y-auto p-4 space-y-3 font-mono text-xs">
          {activeTab === 'matrix' && (
            <div className="overflow-x-auto rounded border border-obsidian-800 bg-obsidian-950">
              <table className="w-full text-left border-collapse text-xs">
                <thead>
                  <tr className="border-b border-obsidian-800 bg-obsidian-900 text-slate-400 text-[10px] uppercase">
                    <th className="py-2 px-3 font-medium">Инструмент</th>
                    <th className="py-2 px-2 text-right font-medium">Вес</th>
                    {columns.map((col) => (
                      <th key={col.id} className="py-2 px-2 text-center font-medium border-l border-obsidian-800/80">
                        <span className="truncate max-w-[80px] block">
                          {col.title}
                        </span>
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody className="divide-y divide-obsidian-800/80">
                  {AVAILABLE_TOOLS.map((tool) => {
                    const isGloballyActive = globalEnabled[tool.id] ?? false;

                    return (
                      <tr key={tool.id} className="hover:bg-obsidian-850/40 transition text-slate-200">
                        <td className="py-2 px-3">
                          <div>
                            <span className="font-medium text-slate-200">
                              {tool.label}
                            </span>
                            <span className="text-[10px] text-slate-500 block">
                              {tool.name}
                            </span>
                          </div>
                        </td>

                        <td className="py-2 px-2 text-right text-slate-500 tabular-nums text-[11px]">
                          {tool.tokenWeight}
                        </td>

                        {columns.map((col) => {
                          const isAllowed = isToolAllowed(tool.id, col.id);
                          const isReviewLock = col.id === 'review' && (tool.id === 'edit_file' || tool.id === 'write_file');

                          return (
                            <td key={col.id} className="py-1.5 px-2 text-center border-l border-obsidian-800/80">
                              {isReviewLock ? (
                                <span className="text-[10px] text-slate-600 select-none">
                                  Блок
                                </span>
                              ) : !isGloballyActive ? (
                                <span className="text-[10px] text-slate-600 select-none">
                                  Выкл
                                </span>
                              ) : (
                                <button
                                  onClick={() => setStageTool(col.id, tool.id, !isAllowed)}
                                  className={`px-2 py-0.5 rounded text-[10px] font-mono transition cursor-pointer ${
                                    isAllowed
                                      ? 'text-emerald-400 bg-emerald-950/30 border border-emerald-800/40'
                                      : 'text-slate-500 bg-obsidian-900 border border-obsidian-800'
                                  }`}
                                  title={`${tool.label} для ${col.title}`}
                                >
                                  {isAllowed ? 'ВКЛ' : 'ВЫКЛ'}
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
          )}

          {activeTab === 'global' && (
            <div className="space-y-2">
              {AVAILABLE_TOOLS.map((tool) => {
                const isEnabled = globalEnabled[tool.id] ?? false;
                return (
                  <div
                    key={tool.id}
                    onClick={() => toggleGlobalTool(tool.id)}
                    className={`flex items-center justify-between p-2.5 rounded border cursor-pointer transition select-none ${
                      isEnabled
                        ? 'border-obsidian-750 bg-obsidian-850 hover:border-obsidian-700'
                        : 'border-obsidian-800 bg-obsidian-950/60 opacity-60 hover:opacity-80'
                    }`}
                  >
                    <div>
                      <div className="flex items-center space-x-2">
                        <span className="font-mono font-medium text-xs text-slate-200">
                          {tool.label}
                        </span>
                        <span className="text-[10px] text-slate-500">
                          ({tool.name})
                        </span>
                      </div>
                      <p className="text-[11px] text-slate-400 mt-0.5 font-sans leading-snug">
                        {tool.description}
                      </p>
                    </div>

                    <div className="flex items-center space-x-3">
                      <span className={`text-xs font-mono ${isEnabled ? 'text-emerald-400' : 'text-slate-500'}`}>
                        {isEnabled ? 'Активен' : 'Отключен'}
                      </span>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="border-t border-obsidian-800 bg-obsidian-850 px-4 py-2 flex items-center justify-end font-mono text-xs">
          <button
            onClick={closeModal}
            className="px-3 py-1 rounded bg-slate-200 hover:bg-white text-obsidian-950 font-medium transition cursor-pointer"
          >
            Закрыть
          </button>
        </div>
      </div>
    </div>
  );
}
