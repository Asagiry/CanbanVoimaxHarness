import { useCallback, useEffect, useState } from 'react';
import { useToolPolicyStore, AVAILABLE_TOOLS } from '../../stores/toolPolicyStore';
import { useBoardStore } from '../../stores/boardStore';
import { X, ChevronDown, Trash2 } from 'lucide-react';

interface CustomTool {
  name: string;
  description: string;
  parameters: Record<string, unknown>;
  command: string;
}

function Toggle({
  enabled,
  onToggle,
  title,
  disabled,
}: {
  enabled: boolean;
  onToggle: () => void;
  title: string;
  disabled?: boolean;
}) {
  return (
    <button
      type="button"
      onClick={(e) => {
        e.stopPropagation();
        if (!disabled) onToggle();
      }}
      disabled={disabled}
      title={title}
      aria-pressed={enabled}
      className={`relative w-9 h-5 rounded-full transition-colors cursor-pointer shrink-0 ${
        enabled ? 'bg-[#171717]' : 'bg-[#e4e4e7]'
      } ${disabled ? 'opacity-50 cursor-not-allowed' : ''}`}
    >
      <span
        className={`absolute top-0.5 h-4 w-4 rounded-full bg-white shadow transition-all ${
          enabled ? 'left-[18px]' : 'left-0.5'
        }`}
      />
    </button>
  );
}

const NAME_PATTERN = /^[a-z0-9_]+$/;

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
  const [expandedToolId, setExpandedToolId] = useState<string | null>(null);

  const [customTools, setCustomTools] = useState<CustomTool[]>([]);
  const [customLoading, setCustomLoading] = useState(false);
  const [customListError, setCustomListError] = useState<string | null>(null);

  const [customName, setCustomName] = useState('');
  const [customDescription, setCustomDescription] = useState('');
  const [customCommand, setCustomCommand] = useState('');
  const [customParamsText, setCustomParamsText] = useState('{}');
  const [customFormError, setCustomFormError] = useState<string | null>(null);
  const [customSaving, setCustomSaving] = useState(false);

  const toggleExpanded = (id: string) => {
    setExpandedToolId((prev) => (prev === id ? null : id));
  };

  const loadCustomTools = useCallback(async () => {
    setCustomLoading(true);
    setCustomListError(null);
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const list = await invoke<CustomTool[]>('list_custom_tools');
      setCustomTools(Array.isArray(list) ? list : []);
    } catch (err) {
      setCustomTools([]);
      setCustomListError(err instanceof Error ? err.message : String(err));
    } finally {
      setCustomLoading(false);
    }
  }, []);

  useEffect(() => {
    if (isModalOpen) {
      setExpandedToolId(null);
      setCustomFormError(null);
      void loadCustomTools();
    }
  }, [isModalOpen, loadCustomTools]);

  const handleAddCustomTool = async (e: React.FormEvent) => {
    e.preventDefault();
    setCustomFormError(null);

    const name = customName.trim();
    const description = customDescription.trim();
    const command = customCommand.trim();
    const paramsRaw = customParamsText.trim() === '' ? '{}' : customParamsText.trim();

    if (!NAME_PATTERN.test(name)) {
      setCustomFormError('Имя должно содержать только строчные латинские буквы, цифры и подчёркивание: [a-z0-9_].');
      return;
    }
    if (!description) {
      setCustomFormError('Укажите описание инструмента.');
      return;
    }
    if (!command) {
      setCustomFormError('Укажите команду — шаблон shell с {args}.');
      return;
    }
    let parameters: Record<string, unknown>;
    try {
      const parsed: unknown = JSON.parse(paramsRaw);
      if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed)) {
        setCustomFormError('Параметры должны быть JSON-объектом, например: {}.');
        return;
      }
      parameters = parsed as Record<string, unknown>;
    } catch (err) {
      setCustomFormError(`Некорректный JSON в параметрах: ${err instanceof Error ? err.message : String(err)}`);
      return;
    }

    setCustomSaving(true);
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke<string>('save_custom_tool', { name, description, parameters, command });
      setCustomName('');
      setCustomDescription('');
      setCustomCommand('');
      setCustomParamsText('{}');
      await loadCustomTools();
    } catch (err) {
      setCustomFormError(err instanceof Error ? err.message : String(err));
    } finally {
      setCustomSaving(false);
    }
  };

  const handleDeleteCustomTool = async (name: string) => {
    const confirmed = window.confirm(`Удалить инструмент «${name}»?`);
    if (!confirmed) return;
    setCustomFormError(null);
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('delete_custom_tool', { name });
      await loadCustomTools();
    } catch (err) {
      setCustomFormError(err instanceof Error ? err.message : String(err));
    }
  };

  if (!isModalOpen) return null;

  return (
    <div className="fixed inset-0 z-[80] flex items-center justify-center bg-black/40 backdrop-blur-xs p-3">
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
                    const isExpanded = expandedToolId === tool.id;

                    return (
                      <tr key={tool.id} className="hover:bg-[#fafafa] transition text-[#171717]">
                        <td className="py-2.5 px-3">
                          <button
                            type="button"
                            onClick={() => toggleExpanded(tool.id)}
                            title={isExpanded ? 'Свернуть описание' : 'Показать описание'}
                            className="flex items-center gap-1.5 text-left cursor-pointer group"
                          >
                            <ChevronDown
                              className={`h-3.5 w-3.5 shrink-0 text-[#8f8f8f] transition-transform ${isExpanded ? 'rotate-180' : ''}`}
                            />
                            <span>
                              <span className="font-medium text-[#171717] group-hover:underline underline-offset-2">
                                {tool.label}
                              </span>
                              <span className="text-[10px] text-[#8f8f8f] block">
                                {tool.name}
                              </span>
                            </span>
                          </button>
                          {isExpanded && (
                            <p className="text-[11px] text-[#666666] mt-1.5 font-sans leading-snug">
                              {tool.description}
                            </p>
                          )}
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
                                <span className="inline-flex justify-center">
                                  <Toggle
                                    enabled={isAllowed}
                                    onToggle={() => setStageTool(col.id, tool.id, !isAllowed)}
                                    title={`${tool.label} для ${col.title}: ${isAllowed ? 'включено' : 'выключено'}`}
                                  />
                                </span>
                              )}
                            </td>
                          );
                        })}
                      </tr>
                    );
                  })}
                  {customTools.map((customTool) => {
                    const customId = `custom:${customTool.name}`;
                    const isCustomGloballyActive = globalEnabled[customTool.name] ?? true;
                    const isCustomExpanded = expandedToolId === customId;

                    return (
                      <tr key={customId} className="hover:bg-[#fafafa] transition text-[#171717]">
                        <td className="py-2.5 px-3">
                          <button
                            type="button"
                            onClick={() => toggleExpanded(customId)}
                            title={isCustomExpanded ? 'Свернуть описание' : 'Показать описание'}
                            className="flex items-center gap-1.5 text-left cursor-pointer group"
                          >
                            <ChevronDown
                              className={`h-3.5 w-3.5 shrink-0 text-[#8f8f8f] transition-transform ${isCustomExpanded ? 'rotate-180' : ''}`}
                            />
                            <span>
                              <span className="font-medium text-[#171717] group-hover:underline underline-offset-2">
                                {customTool.name}
                              </span>
                              <span className="ml-1.5 inline-block rounded border border-[#ebebeb] bg-[#f4f4f5] px-1 py-px text-[10px] leading-none text-[#666666] align-middle">
                                свой
                              </span>
                            </span>
                          </button>
                          {isCustomExpanded && (
                            <p className="text-[11px] text-[#666666] mt-1.5 font-sans leading-snug">
                              {customTool.description}
                            </p>
                          )}
                        </td>

                        <td className="py-2.5 px-2 text-right text-[#8f8f8f] tabular-nums text-[11px]">
                          —
                        </td>

                        {agentColumns.map((col) => {
                          const isCustomAllowed = isToolAllowed(customTool.name, col.id);

                          return (
                            <td key={col.id} className="py-2 px-2 text-center border-l border-[#ebebeb]">
                              {!isCustomGloballyActive ? (
                                <span className="text-[10px] text-[#8f8f8f] select-none">
                                  Выкл
                                </span>
                              ) : (
                                <span className="inline-flex justify-center">
                                  <Toggle
                                    enabled={isCustomAllowed}
                                    onToggle={() => setStageTool(col.id, customTool.name, !isCustomAllowed)}
                                    title={`${customTool.name} для ${col.title}: ${isCustomAllowed ? 'включено' : 'выключено'}`}
                                  />
                                </span>
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
                const isExpanded = expandedToolId === tool.id;
                return (
                  <div
                    key={tool.id}
                    className={`p-3 rounded-md border transition select-none ${
                      isEnabled
                        ? 'border-[#ebebeb] bg-white hover:border-[#d4d4d8] shadow-xs'
                        : 'border-[#ebebeb] bg-[#fafafa] opacity-70'
                    }`}
                  >
                    <div className="flex items-center justify-between gap-3">
                      <button
                        type="button"
                        onClick={() => toggleExpanded(tool.id)}
                        title={isExpanded ? 'Свернуть описание' : 'Показать описание'}
                        className="flex items-center gap-1.5 text-left cursor-pointer min-w-0"
                      >
                        <ChevronDown
                          className={`h-3.5 w-3.5 shrink-0 text-[#8f8f8f] transition-transform ${isExpanded ? 'rotate-180' : ''}`}
                        />
                        <span className="font-mono font-medium text-xs text-[#171717] truncate">
                          {tool.label}
                        </span>
                        <span className="text-[10px] text-[#8f8f8f] truncate">
                          ({tool.name})
                        </span>
                      </button>

                      <Toggle
                        enabled={isEnabled}
                        onToggle={() => toggleGlobalTool(tool.id)}
                        title={`${tool.label}: ${isEnabled ? 'включено' : 'выключено'}`}
                      />
                    </div>
                    {isExpanded && (
                      <p className="text-[11px] text-[#666666] mt-1.5 font-sans leading-snug">
                        {tool.description}
                      </p>
                    )}
                  </div>
                );
              })}

              {/* Свои инструменты */}
              <div className="pt-2">
                <h3 className="text-xs font-semibold text-[#171717] font-sans">
                  Свои инструменты
                </h3>

                {customLoading && (
                  <p className="text-[11px] text-[#666666] font-sans mt-2">Загрузка...</p>
                )}
                {customListError && (
                  <p className="text-[11px] font-sans mt-2 text-red-700 bg-red-50 border border-red-200 rounded-md px-3 py-1.5">
                    {customListError}
                  </p>
                )}

                {!customLoading && !customListError && customTools.length === 0 && (
                  <p className="text-[11px] text-[#8f8f8f] font-sans mt-2">
                    Пока нет своих инструментов.
                  </p>
                )}

                <div className="space-y-2 mt-2">
                  {customTools.map((tool) => {
                    const expandId = `custom:${tool.name}`;
                    const isExpanded = expandedToolId === expandId;
                    return (
                      <div
                        key={tool.name}
                        className="p-3 rounded-md border border-[#ebebeb] bg-white"
                      >
                        <div className="flex items-center justify-between gap-3">
                          <button
                            type="button"
                            onClick={() => toggleExpanded(expandId)}
                            title={isExpanded ? 'Свернуть описание' : 'Показать описание'}
                            className="flex items-center gap-1.5 text-left cursor-pointer min-w-0"
                          >
                            <ChevronDown
                              className={`h-3.5 w-3.5 shrink-0 text-[#8f8f8f] transition-transform ${isExpanded ? 'rotate-180' : ''}`}
                            />
                            <span className="font-mono font-medium text-xs text-[#171717] truncate">
                              {tool.name}
                            </span>
                          </button>
                          <button
                            type="button"
                            onClick={() => void handleDeleteCustomTool(tool.name)}
                            title={`Удалить ${tool.name}`}
                            className="p-1.5 rounded-md text-[#8f8f8f] hover:text-red-700 hover:bg-red-50 border border-transparent hover:border-red-200 transition cursor-pointer shrink-0"
                          >
                            <Trash2 className="h-3.5 w-3.5" />
                          </button>
                        </div>
                        <p className="text-[11px] text-[#666666] mt-1 font-sans leading-snug">
                          {tool.description}
                        </p>
                        {isExpanded && (
                          <div className="mt-2 space-y-1 text-[11px] font-sans">
                            <p className="text-[#666666]">
                              <span className="font-medium text-[#171717]">Команда: </span>
                              <code className="font-mono bg-[#f4f4f5] rounded px-1 py-0.5 break-all">
                                {tool.command}
                              </code>
                            </p>
                          </div>
                        )}
                      </div>
                    );
                  })}
                </div>

                {/* Форма добавления */}
                <form onSubmit={(e) => void handleAddCustomTool(e)} className="mt-3 p-3 rounded-md border border-[#ebebeb] bg-[#fafafa] space-y-2.5">
                  <p className="text-xs font-medium text-[#171717] font-sans">
                    Добавить инструмент
                  </p>
                  <div className="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
                    <div>
                      <label className="block font-sans text-[#4d4d4d] font-medium mb-1 text-[11px]">
                        Имя [a-z0-9_]
                      </label>
                      <input
                        type="text"
                        value={customName}
                        onChange={(e) => setCustomName(e.target.value)}
                        placeholder="my_tool"
                        className="w-full px-3 py-1.5 rounded-md bg-white border border-[#ebebeb] text-[#171717] placeholder:text-[#8f8f8f] focus:outline-hidden focus:border-[#171717] font-mono text-xs transition"
                      />
                    </div>
                    <div>
                      <label className="block font-sans text-[#4d4d4d] font-medium mb-1 text-[11px]">
                        Команда (шаблон shell с {'{args}'})
                      </label>
                      <input
                        type="text"
                        value={customCommand}
                        onChange={(e) => setCustomCommand(e.target.value)}
                        placeholder="my-script --input {args}"
                        className="w-full px-3 py-1.5 rounded-md bg-white border border-[#ebebeb] text-[#171717] placeholder:text-[#8f8f8f] focus:outline-hidden focus:border-[#171717] font-mono text-xs transition"
                      />
                    </div>
                  </div>
                  <div>
                    <label className="block font-sans text-[#4d4d4d] font-medium mb-1 text-[11px]">
                      Описание
                    </label>
                    <input
                      type="text"
                      value={customDescription}
                      onChange={(e) => setCustomDescription(e.target.value)}
                      placeholder="Что делает инструмент"
                      className="w-full px-3 py-1.5 rounded-md bg-white border border-[#ebebeb] text-[#171717] placeholder:text-[#8f8f8f] focus:outline-hidden focus:border-[#171717] font-sans text-xs transition"
                    />
                  </div>
                  <div>
                    <label className="block font-sans text-[#4d4d4d] font-medium mb-1 text-[11px]">
                      Параметры (JSON-схема)
                    </label>
                    <textarea
                      rows={3}
                      value={customParamsText}
                      onChange={(e) => setCustomParamsText(e.target.value)}
                      placeholder={'{"type": "object", "properties": {}}'}
                      spellCheck={false}
                      className="w-full px-3 py-1.5 rounded-md bg-white border border-[#ebebeb] text-[#171717] placeholder:text-[#8f8f8f] focus:outline-hidden focus:border-[#171717] font-mono text-xs resize-y transition"
                    />
                  </div>
                  {customFormError && (
                    <p className="text-[11px] font-sans text-red-700 bg-red-50 border border-red-200 rounded-md px-3 py-1.5">
                      {customFormError}
                    </p>
                  )}
                  <div className="flex justify-end">
                    <button
                      type="submit"
                      disabled={customSaving}
                      className="px-3.5 py-1.5 rounded-md bg-[#171717] hover:bg-[#262626] disabled:opacity-50 text-white font-medium transition cursor-pointer font-sans text-xs shadow-xs"
                    >
                      {customSaving ? 'Сохранение...' : 'Добавить'}
                    </button>
                  </div>
                </form>
              </div>
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
