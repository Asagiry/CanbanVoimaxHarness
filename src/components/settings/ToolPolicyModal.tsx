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
  // В матрице только агенты: backlog/ready без persona не имеют тул-контекста.
  const agentColumns = columns.filter((col) => col.persona != null);
  const [activeTab, setActiveTab] = useState<'matrix' | 'global'>('matrix');

  if (!isModalOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-xs p-3">
      <div className="w-full max-w-4xl rounded-lg border border-[#ebebeb] bg-white shadow-xl overflow-hidden flex flex-col max-h-[85vh]">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-[#ebebeb] px-4 py-3 bg-white">
          <div>
            <h2 className="text-sm font-semibold text-[#171717]">
              Настройки матрицы инструментов
            </h2>
            <p className="text-[11px] text-[#666666] font-mono mt-0.5">
              Разграничение прав доступа инструментов по агентам
            </p>
          </div>
          <button
            onClick={closeModal}
            className="p-1 rounded-md text-[#666666] hover:text-[#171717] hover:bg-[#f4f4f5] transition cursor-pointer"
            title="Закрыть"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Tabs Bar */}
        <div className="flex border-b border-[#ebebeb] bg-white px-4 font-mono text-xs">
          <button
            onClick={() => setActiveTab('matrix')}
            className={`px-3.5 py-2.5 border-b-2 transition cursor-pointer ${
              activeTab === 'matrix'
                ? 'border-[#171717] text-[#171717] font-semibold'
                : 'border-transparent text-[#666666] hover:text-[#171717]'
            }`}
          >
            Матрица агентов
          </button>
          <button
            onClick={() => setActiveTab('global')}
            className={`px-3.5 py-2.5 border-b-2 transition cursor-pointer ${
              activeTab === 'global'
                ? 'border-[#171717] text-[#171717] font-semibold'
                : 'border-transparent text-[#666666] hover:text-[#171717]'
            }`}
          >
            Глобальный реестр
          </button>
        </div>

        {/* Body Content */}
        <div className="flex-1 overflow-y-auto p-4 space-y-3 font-mono text-xs">
          {activeTab === 'matrix' && (
            <div className="overflow-x-auto rounded-md border border-[#ebebeb] bg-white">
              <table className="w-full text-left border-collapse text-xs">
                <thead>
                  <tr className="border-b border-[#ebebeb] bg-[#f9f9f9] text-[#666666] text-[10px] uppercase">
                    <th className="py-2.5 px-3 font-medium">Инструмент</th>
                    <th className="py-2.5 px-2 text-right font-medium">Вес</th>
                    {agentColumns.map((col) => (
                      <th key={col.id} className="py-2.5 px-2 text-center font-medium border-l border-[#ebebeb]">
                        <span className="truncate max-w-[80px] block">
                          {col.title}
                        </span>
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody className="divide-y divide-[#ebebeb]">
                  {AVAILABLE_TOOLS.map((tool) => {
                    const isGloballyActive = globalEnabled[tool.id] ?? false;

                    return (
                      <tr key={tool.id} className="hover:bg-[#fafafa] transition text-[#171717]">
                        <td className="py-2.5 px-3">
                          <div>
                            <span className="font-medium text-[#171717]">
                              {tool.label}
                            </span>
                            <span className="text-[10px] text-[#8f8f8f] block">
                              {tool.name}
                            </span>
                          </div>
                        </td>

                        <td className="py-2.5 px-2 text-right text-[#666666] tabular-nums text-[11px]">
                          {tool.tokenWeight}
                        </td>

                        {agentColumns.map((col) => {
                          const isAllowed = isToolAllowed(tool.id, col.id);

                          return (
                            <td key={col.id} className="py-2 px-2 text-center border-l border-[#ebebeb]">
                              {!isGloballyActive ? (
                                <span className="text-[10px] text-[#8f8f8f] select-none">
                                  Выкл
                                </span>
                              ) : (
                                <button
                                  onClick={() => setStageTool(col.id, tool.id, !isAllowed)}
                                  className={`px-2 py-0.5 rounded text-[10px] font-mono transition cursor-pointer ${
                                    isAllowed
                                      ? 'text-[#15803d] bg-[#f0fdf4] border border-[#bbf7d0]'
                                      : 'text-[#666666] bg-[#f4f4f5] border border-[#ebebeb]'
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
                    className={`flex items-center justify-between p-3 rounded-md border cursor-pointer transition select-none ${
                      isEnabled
                        ? 'border-[#ebebeb] bg-white hover:border-[#d4d4d8] shadow-xs'
                        : 'border-[#ebebeb] bg-[#fafafa] opacity-60 hover:opacity-80'
                    }`}
                  >
                    <div>
                      <div className="flex items-center space-x-2">
                        <span className="font-mono font-medium text-xs text-[#171717]">
                          {tool.label}
                        </span>
                        <span className="text-[10px] text-[#8f8f8f]">
                          ({tool.name})
                        </span>
                      </div>
                      <p className="text-[11px] text-[#666666] mt-0.5 font-sans leading-snug">
                        {tool.description}
                      </p>
                    </div>

                    <div className="flex items-center space-x-3">
                      <span className={`text-xs font-mono font-medium ${isEnabled ? 'text-[#15803d]' : 'text-[#8f8f8f]'}`}>
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
        <div className="border-t border-[#ebebeb] bg-[#f9f9f9] px-4 py-2.5 flex items-center justify-end font-mono text-xs">
          <button
            onClick={closeModal}
            className="px-3.5 py-1.5 rounded-md bg-[#171717] hover:bg-[#262626] text-white font-medium transition cursor-pointer shadow-xs"
          >
            Закрыть
          </button>
        </div>
      </div>
    </div>
  );
}
