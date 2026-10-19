import { Fragment, useCallback, useEffect, useMemo, useState } from 'react';
import { useToolPolicyStore, AVAILABLE_TOOLS, TOOL_PARAM_DEFS } from '../../stores/toolPolicyStore';
import { useBoardStore } from '../../stores/boardStore';
import { useCortexStore } from '../../stores/cortexStore';
import AgentIcon from '../common/AgentIcon';
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
        enabled ? 'bg-[#171717] dark:bg-[#f0f6fc]' : 'bg-[#e4e4e7] dark:bg-[#30363d]'
      } ${disabled ? 'opacity-50 cursor-not-allowed' : ''}`}
    >
      <span
        className={`absolute top-0.5 h-4 w-4 rounded-full bg-white dark:bg-[#0d1117] shadow transition-all ${
          enabled ? 'left-[18px]' : 'left-0.5'
        }`}
      />
    </button>
  );
}

function ToolParamRow({
  toolId,
  param,
}: {
  toolId: string;
  param: {
    key: string;
    label: string;
    defaultValue: number;
    min: number;
    max: number;
    step: number;
    unit: string;
  };
}) {
  const { toolSettings, setToolSetting } = useToolPolicyStore();
  const currentValue = toolSettings[toolId]?.[param.key] ?? param.defaultValue;
  const [draft, setDraft] = useState(String(currentValue));

  useEffect(() => {
    setDraft(String(currentValue));
  }, [currentValue]);

  const commitValue = (val: number) => {
    const clamped = Math.max(param.min, Math.min(param.max, val));
    setDraft(String(clamped));
    setToolSetting(toolId, param.key, clamped);
  };

  const handleStep = (delta: number) => {
    commitValue(currentValue + delta);
  };

  const handleBlur = () => {
    const parsed = parseInt(draft, 10);
    if (isNaN(parsed)) {
      setDraft(String(currentValue));
    } else {
      commitValue(parsed);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      (e.target as HTMLInputElement).blur();
    }
    if (e.key === 'Escape') {
      setDraft(String(currentValue));
      (e.target as HTMLInputElement).blur();
    }
  };

  return (
    <div className="flex items-center justify-between gap-3 text-xs">
      <div className="min-w-0 flex-1">
        <span className="text-[11px] font-mono text-[#171717] dark:text-[#f0f6fc] block">
          {param.label}
        </span>
        <span className="text-[10px] text-[#8f8f8f] dark:text-[#8b949e] font-mono block">
          диапазон: {param.min}–{param.max} (шаг {param.step})
        </span>
      </div>

      <div className="flex items-center gap-1.5 shrink-0">
        <button
          type="button"
          disabled={currentValue <= param.min}
          onClick={() => handleStep(-param.step)}
          className="w-6 h-6 flex items-center justify-center rounded border border-[#d4d4d8] dark:border-[#30363d] bg-white dark:bg-[#161b22] hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] text-[#171717] dark:text-[#f0f6fc] text-xs font-mono font-bold transition select-none cursor-pointer disabled:opacity-30 disabled:cursor-not-allowed"
          title={`Уменьшить на ${param.step}`}
        >
          -
        </button>

        <input
          type="number"
          min={param.min}
          max={param.max}
          step={param.step}
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={handleBlur}
          onKeyDown={handleKeyDown}
          className="w-16 h-6 rounded border border-[#d4d4d8] dark:border-[#30363d] bg-white dark:bg-[#0d1117] text-[#171717] dark:text-[#f0f6fc] font-mono text-xs tabular-nums text-center px-1 focus:outline-hidden focus:border-[#171717] dark:focus:border-[#58a6ff]"
        />

        <button
          type="button"
          disabled={currentValue >= param.max}
          onClick={() => handleStep(param.step)}
          className="w-6 h-6 flex items-center justify-center rounded border border-[#d4d4d8] dark:border-[#30363d] bg-white dark:bg-[#161b22] hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] text-[#171717] dark:text-[#f0f6fc] text-xs font-mono font-bold transition select-none cursor-pointer disabled:opacity-30 disabled:cursor-not-allowed"
          title={`Увеличить на ${param.step}`}
        >
          +
        </button>

        <span className="text-[10px] text-[#666666] dark:text-[#8b949e] font-mono min-w-[42px] text-left">
          {param.unit}
        </span>
      </div>
    </div>
  );
}

function ToolParamsEditor({ toolId }: { toolId: string }) {
  const paramDefs = TOOL_PARAM_DEFS[toolId];

  if (!paramDefs || paramDefs.length === 0) return null;

  return (
    <div
      onClick={(e) => e.stopPropagation()}
      className="mt-2.5 p-2.5 rounded-md border border-[#ebebeb] dark:border-[#21262d] bg-[#f9f9fb] dark:bg-[#07090e] space-y-2 select-none"
    >
      <div className="flex items-center justify-between text-[10px] font-mono uppercase tracking-wider text-[#666666] dark:text-[#8b949e] font-semibold border-b border-[#ebebeb] dark:border-[#21262d] pb-1.5">
        <span>Параметры инструмента</span>
        <span className="text-[9px] text-[#8f8f8f] dark:text-[#6e7681] lowercase font-normal">
          сохраняются в политиках
        </span>
      </div>
      {paramDefs.map((param) => (
        <ToolParamRow key={param.key} toolId={toolId} param={param} />
      ))}
    </div>
  );
}

const NAME_PATTERN = /^[a-z0-9_]+$/;

const CATEGORY_ORDER: Array<'orchestration' | 'interactive' | 'workflow' | 'system' | 'fs' | 'web' | 'agent'> = [
  'orchestration',
  'interactive',
  'workflow',
  'system',
  'fs',
  'web',
  'agent',
];

const CATEGORY_LABELS: Record<string, string> = {
  orchestration: 'Оркестрация и управление задачами',
  interactive: 'Интерактивность',
  workflow: 'Маршрутизация задач',
  system: 'Команды и терминал',
  fs: 'Файловая система',
  web: 'Сеть и документация',
  agent: 'Подагенты и планирование',
};

export default function ToolPolicyModal() {
  const {
    isModalOpen,
    globalEnabled,
    closeModal,
    toggleGlobalTool,
    setStageTool,
    isToolAllowed,
  } = useToolPolicyStore();

  const { agentPersonas, loadAgentPersonas } = useCortexStore();
  const { columns } = useBoardStore();

  interface AgentMatrixItem {
    role: string;
    icon?: string;
    color?: string;
    isBuiltin: boolean;
  }

  const agents = useMemo<AgentMatrixItem[]>(() => {
    const map = new Map<string, AgentMatrixItem>();

    // 1. Персоны из реестра CortexStore
    for (const p of agentPersonas) {
      if (p.role) {
        map.set(p.role, {
          role: p.role,
          icon: p.icon,
          color: p.color,
          isBuiltin: p.role === 'cvh-dev' || Boolean(p.isBuiltin),
        });
      }
    }

    // 2. Персоны, назначенные на колонки досок
    for (const col of columns) {
      if (col.persona && !map.has(col.persona)) {
        map.set(col.persona, {
          role: col.persona,
          icon: col.icon,
          color: col.color,
          isBuiltin: col.persona === 'cvh-dev',
        });
      }
    }

    // 3. Fallback, если реестр пуст
    if (map.size === 0) {
      map.set('cvh-dev', { role: 'cvh-dev', icon: 'Cpu', isBuiltin: true });
    }

    return Array.from(map.values()).sort((a, b) => {
      if (a.isBuiltin && !b.isBuiltin) return -1;
      if (!a.isBuiltin && b.isBuiltin) return 1;
      return a.role.localeCompare(b.role);
    });
  }, [agentPersonas, columns]);

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
      void loadAgentPersonas();
    }
  }, [isModalOpen, loadCustomTools, loadAgentPersonas]);

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
    <div className="fixed inset-0 z-[80] flex items-center justify-center bg-black/60 dark:bg-black/80 p-3">
      <div className="w-full max-w-4xl rounded-lg border border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#0d1117] text-[#171717] dark:text-[#f0f6fc] shadow-md overflow-hidden flex flex-col max-h-[85vh] transition-colors">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-[#ebebeb] dark:border-[#21262d] px-4 py-3 bg-white dark:bg-[#0d1117]">
          <div>
            <h2 className="text-sm font-semibold text-[#171717] dark:text-[#f0f6fc]">
              Настройки матрицы инструментов
            </h2>
            <p className="text-[11px] text-[#666666] dark:text-[#8b949e] font-mono mt-0.5">
              Разграничение прав доступа и тонкая настройка параметров инструментов
            </p>
          </div>
          <button
            onClick={closeModal}
            className="p-1 rounded-md text-[#666666] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#f4f4f5] dark:hover:bg-[#161b22] transition cursor-pointer"
            title="Закрыть"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Tabs Bar */}
        <div className="flex border-b border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#0d1117] px-4 font-mono text-xs">
          <button
            onClick={() => setActiveTab('matrix')}
            className={`px-3.5 py-2.5 border-b-2 transition cursor-pointer ${
              activeTab === 'matrix'
                ? 'border-[#171717] dark:border-[#f0f6fc] text-[#171717] dark:text-[#f0f6fc] font-semibold'
                : 'border-transparent text-[#666666] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc]'
            }`}
          >
            Матрица агентов
          </button>
          <button
            onClick={() => setActiveTab('global')}
            className={`px-3.5 py-2.5 border-b-2 transition cursor-pointer ${
              activeTab === 'global'
                ? 'border-[#171717] dark:border-[#f0f6fc] text-[#171717] dark:text-[#f0f6fc] font-semibold'
                : 'border-transparent text-[#666666] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc]'
            }`}
          >
            Глобальный реестр
          </button>
        </div>

        {/* Body Content */}
        <div className="flex-1 min-h-0 overflow-y-auto p-4 space-y-3 font-mono text-xs">
          {activeTab === 'matrix' && (
            <div className="overflow-x-auto rounded-md border border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#0d1117]">
              <table className="w-full text-left border-collapse text-xs">
                <thead>
                  <tr className="border-b border-[#ebebeb] dark:border-[#21262d] bg-[#f9f9f9] dark:bg-[#161b22] text-[#666666] dark:text-[#8b949e] text-[10px] uppercase">
                    <th className="py-2.5 px-3 font-medium">Инструмент</th>
                    <th className="py-2.5 px-2 text-right font-medium">Вес</th>
                    {agents.map((agent) => (
                      <th key={agent.role} className="py-2.5 px-2 text-center font-medium border-l border-[#ebebeb] dark:border-[#21262d] min-w-[95px]">
                        <div className="flex flex-col items-center gap-1">
                          <div className="flex items-center gap-1.5 justify-center max-w-[110px]">
                            <AgentIcon
                              name={agent.icon || (agent.isBuiltin ? 'Cpu' : 'Bot')}
                              className={`h-3.5 w-3.5 shrink-0 ${agent.isBuiltin ? 'text-indigo-600 dark:text-indigo-400' : 'text-[#171717] dark:text-[#f0f6fc]'}`}
                            />
                            <span className="truncate block font-mono font-semibold text-[#171717] dark:text-[#f0f6fc]" title={agent.role}>
                              {agent.role}
                            </span>
                          </div>
                          {agent.isBuiltin && (
                            <span className="px-1 py-0.2 rounded text-[9px] font-sans font-medium bg-indigo-50 dark:bg-indigo-950/60 text-indigo-700 dark:text-indigo-300 border border-indigo-200 dark:border-indigo-800">
                              встроенный
                            </span>
                          )}
                        </div>
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody className="divide-y divide-[#ebebeb] dark:divide-[#21262d]">
                  {CATEGORY_ORDER.map((catKey) => {
                    const toolsInCat = AVAILABLE_TOOLS.filter((t) => t.category === catKey);
                    if (toolsInCat.length === 0) return null;

                    return (
                      <Fragment key={`cat-grp-${catKey}`}>
                        <tr className="bg-[#f4f4f5]/90 dark:bg-[#161b22] border-y border-[#ebebeb] dark:border-[#21262d]">
                          <td
                            colSpan={2 + agents.length}
                            className="py-1.5 px-3 text-[10px] font-semibold text-[#52525b] dark:text-[#8b949e] uppercase tracking-wider"
                          >
                            {CATEGORY_LABELS[catKey] || catKey}
                          </td>
                        </tr>
                        {toolsInCat.map((tool) => {
                          const isGloballyActive = globalEnabled[tool.id] ?? false;
                          const isExpanded = expandedToolId === tool.id;
                          const isChild = Boolean(tool.parentTool);

                          return (
                            <tr
                              key={tool.id}
                              className={`hover:bg-[#fafafa] dark:hover:bg-[#161b22] transition text-[#171717] dark:text-[#f0f6fc] ${
                                isChild ? 'bg-[#fafafc] dark:bg-[#090a0f]' : ''
                              }`}
                            >
                              <td className={`py-2.5 px-3 align-top ${isChild ? 'pl-7' : ''}`}>
                                <button
                                  type="button"
                                  onClick={() => toggleExpanded(tool.id)}
                                  title={isExpanded ? 'Свернуть описание' : 'Показать описание'}
                                  className="flex items-start gap-1.5 text-left cursor-pointer group w-full"
                                >
                                  {isChild && (
                                    <span className="text-[#8f8f8f] dark:text-[#6e7681] font-mono text-xs select-none shrink-0 mt-0.5 leading-none">
                                      {tool.id === 'collect_subagent' ? '└──' : '├──'}
                                    </span>
                                  )}
                                  <ChevronDown
                                    className={`w-4 h-4 shrink-0 mt-0.5 text-[#8f8f8f] dark:text-[#6e7681] transition-transform ${isExpanded ? 'rotate-180' : ''}`}
                                  />
                                  <div className="min-w-0 flex-1">
                                    <div
                                      className={`font-medium text-[#171717] dark:text-[#f0f6fc] group-hover:underline underline-offset-2 leading-tight ${
                                        tool.isSubagentMaster ? 'font-semibold' : ''
                                      }`}
                                    >
                                      {tool.label}
                                    </div>
                                    <div className="text-[10px] text-[#8f8f8f] dark:text-[#8b949e] font-mono leading-tight mt-0.5">
                                      {tool.name}
                                    </div>
                                  </div>
                                </button>
                                {isExpanded && (
                                  <div className="mt-2 space-y-2">
                                    <p className="text-[11px] text-[#666666] dark:text-[#8b949e] font-sans leading-snug">
                                      {tool.description}
                                    </p>
                                    <ToolParamsEditor toolId={tool.id} />
                                  </div>
                                )}
                              </td>

                              <td className="py-2.5 px-2 text-right text-[#666666] dark:text-[#8b949e] tabular-nums text-[11px] align-top">
                                {tool.tokenWeight}
                              </td>

                              {agents.map((agent) => {
                                const isMasterAllowed = isChild && tool.parentTool
                                  ? isToolAllowed(tool.parentTool, agent.role)
                                  : true;
                                const isAllowed = isToolAllowed(tool.id, agent.role);
                                const isDisabled = isChild && !isMasterAllowed;

                                return (
                                  <td key={agent.role} className="py-2.5 px-2 text-center border-l border-[#ebebeb] dark:border-[#21262d] align-top">
                                    {!isGloballyActive ? (
                                      <span className="text-[10px] text-[#8f8f8f] select-none block pt-0.5">
                                        Выкл
                                      </span>
                                    ) : isDisabled ? (
                                      <span className="inline-flex justify-center opacity-40 pt-0.5">
                                        <Toggle
                                          enabled={false}
                                          disabled={true}
                                          onToggle={() => {}}
                                          title={`${tool.label} отключен: мастер-тул '${tool.parentTool}' выключен для ${agent.role}`}
                                        />
                                      </span>
                                    ) : (
                                      <span className="inline-flex justify-center pt-0.5">
                                        <Toggle
                                          enabled={isAllowed}
                                          onToggle={() => {
                                            setStageTool(agent.role, tool.id, !isAllowed);
                                            for (const col of columns) {
                                              if (col.persona === agent.role && col.id !== agent.role) {
                                                setStageTool(col.id, tool.id, !isAllowed);
                                              }
                                            }
                                          }}
                                          title={`${tool.label} для агента ${agent.role}: ${isAllowed ? 'включено' : 'выключено'}`}
                                        />
                                      </span>
                                    )}
                                  </td>
                                );
                              })}
                            </tr>
                          );
                        })}
                      </Fragment>
                    );
                  })}
                  {customTools.map((customTool) => {
                    const customId = `custom:${customTool.name}`;
                    const isCustomGloballyActive = globalEnabled[customTool.name] ?? true;
                    const isCustomExpanded = expandedToolId === customId;

                    return (
                      <tr key={customId} className="hover:bg-[#fafafa] dark:hover:bg-[#161b22] transition text-[#171717] dark:text-[#f0f6fc]">
                        <td className="py-2.5 px-3 align-top">
                          <button
                            type="button"
                            onClick={() => toggleExpanded(customId)}
                            title={isCustomExpanded ? 'Свернуть описание' : 'Показать описание'}
                            className="flex items-start gap-1.5 text-left cursor-pointer group w-full"
                          >
                            <ChevronDown
                              className={`w-4 h-4 shrink-0 mt-0.5 text-[#8f8f8f] dark:text-[#8b949e] transition-transform ${isCustomExpanded ? 'rotate-180' : ''}`}
                            />
                            <div className="min-w-0 flex-1">
                              <span className="font-medium text-[#171717] dark:text-[#f0f6fc] group-hover:underline underline-offset-2 leading-tight">
                                {customTool.name}
                              </span>
                              <span className="ml-1.5 inline-block rounded border border-[#ebebeb] dark:border-[#30363d] bg-[#f4f4f5] dark:bg-[#161b22] px-1 py-px text-[10px] leading-none text-[#666666] dark:text-[#8b949e] align-middle">
                                свой
                              </span>
                            </div>
                          </button>
                          {isCustomExpanded && (
                            <p className="text-[11px] text-[#666666] dark:text-[#8b949e] mt-1.5 font-sans leading-snug">
                              {customTool.description}
                            </p>
                          )}
                        </td>

                        <td className="py-2.5 px-2 text-right text-[#8f8f8f] tabular-nums text-[11px] align-top">
                          —
                        </td>

                        {agents.map((agent) => {
                          const isCustomAllowed = isToolAllowed(customTool.name, agent.role);

                          return (
                            <td key={agent.role} className="py-2.5 px-2 text-center border-l border-[#ebebeb] dark:border-[#21262d] align-top">
                              {!isCustomGloballyActive ? (
                                <span className="text-[10px] text-[#8f8f8f] select-none block pt-0.5">
                                  Выкл
                                </span>
                              ) : (
                                <span className="inline-flex justify-center pt-0.5">
                                  <Toggle
                                    enabled={isCustomAllowed}
                                    onToggle={() => {
                                      setStageTool(agent.role, customTool.name, !isCustomAllowed);
                                      for (const col of columns) {
                                        if (col.persona === agent.role && col.id !== agent.role) {
                                          setStageTool(col.id, customTool.name, !isCustomAllowed);
                                        }
                                      }
                                    }}
                                    title={`${customTool.name} для агента ${agent.role}: ${isCustomAllowed ? 'включено' : 'выключено'}`}
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
            <div className="space-y-4">
              {CATEGORY_ORDER.map((catKey) => {
                const toolsInCat = AVAILABLE_TOOLS.filter((t) => t.category === catKey);
                if (toolsInCat.length === 0) return null;

                return (
                  <div key={`global-cat-${catKey}`} className="space-y-2">
                    <h3 className="text-[11px] font-semibold text-[#52525b] dark:text-[#8b949e] uppercase tracking-wider px-1">
                      {CATEGORY_LABELS[catKey] || catKey}
                    </h3>
                    <div className="space-y-2">
                      {toolsInCat.map((tool) => {
                        const isEnabled = globalEnabled[tool.id] ?? false;
                        const isExpanded = expandedToolId === tool.id;
                        const isChild = Boolean(tool.parentTool);
                        const isMasterGloballyActive = tool.parentTool ? (globalEnabled[tool.parentTool] ?? true) : true;
                        const isDisabled = isChild && !isMasterGloballyActive;

                        return (
                          <div
                            key={tool.id}
                            className={`p-3 rounded-md border transition select-none ${
                              isChild ? 'ml-4 bg-[#f9f9fb] dark:bg-[#090a0f]' : ''
                            } ${
                              isEnabled && !isDisabled
                                ? 'border-[#ebebeb] dark:border-[#30363d] bg-white dark:bg-[#161b22] hover:border-[#d4d4d8] dark:hover:border-[#484f58] shadow-xs'
                                : 'border-[#ebebeb] dark:border-[#21262d] bg-[#fafafa] dark:bg-[#0d1117] opacity-60'
                            }`}
                          >
                            <div className="flex items-start justify-between gap-3">
                              <button
                                type="button"
                                onClick={() => toggleExpanded(tool.id)}
                                title={isExpanded ? 'Свернуть описание' : 'Показать описание'}
                                className="flex items-start gap-1.5 text-left cursor-pointer min-w-0 flex-1 group"
                              >
                                {isChild && (
                                  <span className="text-[#8f8f8f] dark:text-[#8b949e] font-mono text-xs select-none shrink-0 mt-0.5 leading-none">
                                    {tool.id === 'collect_subagent' ? '└──' : '├──'}
                                  </span>
                                )}
                                <ChevronDown
                                  className={`w-4 h-4 shrink-0 mt-0.5 text-[#8f8f8f] dark:text-[#8b949e] transition-transform ${isExpanded ? 'rotate-180' : ''}`}
                                />
                                <div className="min-w-0 flex-1">
                                  <div className={`font-mono font-medium text-xs text-[#171717] dark:text-[#f0f6fc] truncate leading-tight group-hover:underline underline-offset-2 ${tool.isSubagentMaster ? 'font-semibold' : ''}`}>
                                    {tool.label}
                                  </div>
                                  <div className="text-[10px] text-[#8f8f8f] dark:text-[#8b949e] font-mono leading-tight mt-0.5">
                                    {tool.name}
                                  </div>
                                </div>
                              </button>

                              <div className="shrink-0 pt-0.5">
                                <Toggle
                                  enabled={isEnabled && !isDisabled}
                                  disabled={isDisabled}
                                  onToggle={() => toggleGlobalTool(tool.id)}
                                  title={
                                    isDisabled
                                      ? `${tool.label} отключен (мастер-тул '${tool.parentTool}' выключен глобально)`
                                      : `${tool.label}: ${isEnabled ? 'включено' : 'выключено'}`
                                  }
                                />
                              </div>
                            </div>
                            {isExpanded && (
                              <div className="mt-2 space-y-2">
                                <p className="text-[11px] text-[#666666] dark:text-[#8b949e] font-sans leading-snug">
                                  {tool.description}
                                </p>
                                <ToolParamsEditor toolId={tool.id} />
                              </div>
                            )}
                          </div>
                        );
                      })}
                    </div>
                  </div>
                );
              })}

              {/* Свои инструменты */}
              <div className="pt-2">
                <h3 className="text-xs font-semibold text-[#171717] dark:text-[#f0f6fc] font-sans">
                  Свои инструменты
                </h3>

                {customLoading && (
                  <p className="text-[11px] text-[#666666] dark:text-[#8b949e] font-sans mt-2">Загрузка...</p>
                )}
                {customListError && (
                  <p className="text-[11px] font-sans mt-2 text-red-700 dark:text-red-400 bg-red-50 dark:bg-red-950/30 border border-red-200 dark:border-red-900/50 rounded-md px-3 py-1.5">
                    {customListError}
                  </p>
                )}

                {!customLoading && !customListError && customTools.length === 0 && (
                  <p className="text-[11px] text-[#8f8f8f] dark:text-[#8b949e] font-sans mt-2">
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
                        className="p-3 rounded-md border border-[#ebebeb] dark:border-[#30363d] bg-white dark:bg-[#161b22]"
                      >
                        <div className="flex items-start justify-between gap-3">
                          <button
                            type="button"
                            onClick={() => toggleExpanded(expandId)}
                            title={isExpanded ? 'Свернуть описание' : 'Показать описание'}
                            className="flex items-start gap-1.5 text-left cursor-pointer min-w-0 flex-1 group"
                          >
                            <ChevronDown
                              className={`w-4 h-4 shrink-0 mt-0.5 text-[#8f8f8f] dark:text-[#8b949e] transition-transform ${isExpanded ? 'rotate-180' : ''}`}
                            />
                            <div className="min-w-0 flex-1">
                              <span className="font-mono font-medium text-xs text-[#171717] dark:text-[#f0f6fc] truncate leading-tight group-hover:underline underline-offset-2">
                                {tool.name}
                              </span>
                              <span className="ml-1.5 inline-block rounded border border-[#ebebeb] dark:border-[#30363d] bg-[#f4f4f5] dark:bg-[#161b22] px-1 py-px text-[10px] leading-none text-[#666666] dark:text-[#8b949e] align-middle">
                                свой
                              </span>
                            </div>
                          </button>
                          <div className="shrink-0 pt-0.5">
                            <button
                              type="button"
                              onClick={() => void handleDeleteCustomTool(tool.name)}
                              title={`Удалить ${tool.name}`}
                              className="p-1.5 rounded-md text-[#8f8f8f] dark:text-[#8b949e] hover:text-red-700 dark:hover:text-red-400 hover:bg-red-50 dark:hover:bg-red-950/30 border border-transparent hover:border-red-200 dark:hover:border-red-900/50 transition cursor-pointer"
                            >
                              <Trash2 className="h-3.5 w-3.5" />
                            </button>
                          </div>
                        </div>
                        <p className="text-[11px] text-[#666666] dark:text-[#8b949e] mt-1 font-sans leading-snug">
                          {tool.description}
                        </p>
                        {isExpanded && (
                          <div className="mt-2 space-y-1 text-[11px] font-sans">
                            <p className="text-[#666666] dark:text-[#8b949e]">
                              <span className="font-medium text-[#171717] dark:text-[#f0f6fc]">Команда: </span>
                              <code className="font-mono bg-[#f4f4f5] dark:bg-[#21262d] text-[#171717] dark:text-[#f0f6fc] rounded px-1 py-0.5 break-all">
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
                <form onSubmit={(e) => void handleAddCustomTool(e)} className="mt-3 p-3 rounded-md border border-[#ebebeb] dark:border-[#21262d] bg-[#fafafa] dark:bg-[#161b22] space-y-2.5">
                  <p className="text-xs font-medium text-[#171717] dark:text-[#f0f6fc] font-sans">
                    Добавить инструмент
                  </p>
                  <div className="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
                    <div>
                      <label className="block font-sans text-[#4d4d4d] dark:text-[#c9d1d9] font-medium mb-1 text-[11px]">
                        Имя [a-z0-9_]
                      </label>
                      <input
                        type="text"
                        value={customName}
                        onChange={(e) => setCustomName(e.target.value)}
                        placeholder="my_tool"
                        className="w-full px-3 py-1.5 rounded-md bg-white dark:bg-[#0d1117] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] placeholder:text-[#8f8f8f] dark:placeholder:text-[#8b949e] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc] font-mono text-xs transition"
                      />
                    </div>
                    <div>
                      <label className="block font-sans text-[#4d4d4d] dark:text-[#c9d1d9] font-medium mb-1 text-[11px]">
                        Команда (шаблон shell с {'{args}'})
                      </label>
                      <input
                        type="text"
                        value={customCommand}
                        onChange={(e) => setCustomCommand(e.target.value)}
                        placeholder="my-script --input {args}"
                        className="w-full px-3 py-1.5 rounded-md bg-white dark:bg-[#0d1117] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] placeholder:text-[#8f8f8f] dark:placeholder:text-[#8b949e] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc] font-mono text-xs transition"
                      />
                    </div>
                  </div>
                  <div>
                    <label className="block font-sans text-[#4d4d4d] dark:text-[#c9d1d9] font-medium mb-1 text-[11px]">
                      Описание
                    </label>
                    <input
                      type="text"
                      value={customDescription}
                      onChange={(e) => setCustomDescription(e.target.value)}
                      placeholder="Что делает инструмент"
                      className="w-full px-3 py-1.5 rounded-md bg-white dark:bg-[#0d1117] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] placeholder:text-[#8f8f8f] dark:placeholder:text-[#8b949e] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc] font-sans text-xs transition"
                    />
                  </div>
                  <div>
                    <label className="block font-sans text-[#4d4d4d] dark:text-[#c9d1d9] font-medium mb-1 text-[11px]">
                      Параметры (JSON-схема)
                    </label>
                    <textarea
                      rows={3}
                      value={customParamsText}
                      onChange={(e) => setCustomParamsText(e.target.value)}
                      placeholder={'{"type": "object", "properties": {}}'}
                      spellCheck={false}
                      className="w-full px-3 py-1.5 rounded-md bg-white dark:bg-[#0d1117] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] placeholder:text-[#8f8f8f] dark:placeholder:text-[#8b949e] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc] font-mono text-xs resize-y transition"
                    />
                  </div>
                  {customFormError && (
                    <p className="text-[11px] font-sans text-red-700 dark:text-red-400 bg-red-50 dark:bg-red-950/30 border border-red-200 dark:border-red-900/50 rounded-md px-3 py-1.5">
                      {customFormError}
                    </p>
                  )}
                  <div className="flex justify-end">
                    <button
                      type="submit"
                      disabled={customSaving}
                      className="px-3.5 py-1.5 rounded-md bg-[#171717] dark:bg-[#238636] hover:bg-[#262626] dark:hover:bg-[#2ea043] disabled:opacity-50 text-white font-medium transition cursor-pointer font-sans text-xs shadow-xs"
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
        <div className="border-t border-[#ebebeb] dark:border-[#21262d] bg-[#f9f9f9] dark:bg-[#161b22] px-4 py-2.5 flex items-center justify-end font-mono text-xs">
          <button
            onClick={closeModal}
            className="px-3.5 py-1.5 rounded-md bg-[#171717] dark:bg-[#21262d] hover:bg-[#262626] dark:hover:bg-[#30363d] text-white font-medium transition cursor-pointer shadow-xs"
          >
            Закрыть
          </button>
        </div>
      </div>
    </div>
  );
}
