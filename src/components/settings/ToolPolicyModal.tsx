import { Fragment, useCallback, useEffect, useMemo, useState } from 'react';
import { useToolPolicyStore, AVAILABLE_TOOLS, TOOL_PARAM_DEFS, getToolTranslationKey, type ToolMeta } from '../../stores/toolPolicyStore';
import { useBoardStore } from '../../stores/boardStore';
import { useCortexStore } from '../../stores/cortexStore';
import AgentIcon from '../common/AgentIcon';
import { X, ChevronDown, Trash2 } from 'lucide-react';
import { useTranslation, type TranslationKey } from '../../stores/languageStore';
import { useUiZoom, getZoomOverlayStyle } from '../../hooks/useUiZoom';

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
      aria-label={title}
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
  const { t } = useTranslation();
  const currentValue = toolSettings[toolId]?.[param.key] ?? param.defaultValue;
  const [draft, setDraft] = useState(String(currentValue));

  const paramLabelKey = `toolParam.${param.key}.label` as TranslationKey;
  const paramUnitKey = `toolParam.${param.key}.unit` as TranslationKey;
  const paramLabel = t(paramLabelKey) !== paramLabelKey ? t(paramLabelKey) : param.label;
  const paramUnit = t(paramUnitKey) !== paramUnitKey ? t(paramUnitKey) : param.unit;

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
          {paramLabel}
        </span>
        <span className="text-[10px] text-[#8f8f8f] dark:text-[#8b949e] font-mono block">
          {t('toolPolicy.rangeStep', { min: param.min, max: param.max, step: param.step })}
        </span>
      </div>

      <div className="flex items-center gap-1.5 shrink-0">
        <button
          type="button"
          disabled={currentValue <= param.min}
          onClick={() => handleStep(-param.step)}
          className="w-6 h-6 flex items-center justify-center rounded border border-[#d4d4d8] dark:border-[#30363d] bg-white dark:bg-[#161b22] hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] text-[#171717] dark:text-[#f0f6fc] text-xs font-mono font-bold transition select-none cursor-pointer disabled:opacity-30 disabled:cursor-not-allowed"
          aria-label={t('toolPolicy.decreaseBy', { step: param.step })}
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
          aria-label={t('toolPolicy.increaseBy', { step: param.step })}
        >
          +
        </button>

        <span className="text-[10px] text-[#666666] dark:text-[#8b949e] font-mono min-w-[42px] text-left">
          {paramUnit}
        </span>
      </div>
    </div>
  );
}

function ToolParamsEditor({ toolId }: { toolId: string }) {
  const paramDefs = TOOL_PARAM_DEFS[toolId];
  const { t } = useTranslation();

  if (!paramDefs || paramDefs.length === 0) return null;

  return (
    <div
      onClick={(e) => e.stopPropagation()}
      className="mt-2.5 p-2.5 rounded-md border border-[#ebebeb] dark:border-[#21262d] bg-[#f9f9fb] dark:bg-[#07090e] space-y-2 select-none"
    >
      <div className="flex items-center justify-between text-[10px] font-mono uppercase tracking-wider text-[#666666] dark:text-[#8b949e] font-semibold border-b border-[#ebebeb] dark:border-[#21262d] pb-1.5">
        <span>{t('toolPolicy.toolParams')}</span>
        <span className="text-[9px] text-[#8f8f8f] dark:text-[#6e7681] lowercase font-normal">
          {t('toolPolicy.savedInPolicies')}
        </span>
      </div>
      {paramDefs.map((param) => (
        <ToolParamRow key={param.key} toolId={toolId} param={param} />
      ))}
    </div>
  );
}

function ToolSchemaEditor({ tool }: { tool: ToolMeta }) {
  const { toolCustomSchemas, setToolCustomSchema, resetToolCustomSchema } = useToolPolicyStore();

  const defaultSchema = useMemo(() => ({
    type: 'function',
    function: {
      name: tool.name,
      description: tool.description,
      parameters: tool.parameters || { type: 'object', properties: {} },
    },
  }), [tool.name, tool.description, tool.parameters]);

  const defaultJsonString = useMemo(() => {
    return JSON.stringify(defaultSchema, null, 2);
  }, [defaultSchema]);

  const customSchema = toolCustomSchemas[tool.id];
  const isCustomized = customSchema !== undefined;

  const activeJsonString = useMemo(() => {
    if (customSchema !== undefined) {
      try {
        return typeof customSchema === 'string'
          ? JSON.stringify(JSON.parse(customSchema), null, 2)
          : JSON.stringify(customSchema, null, 2);
      } catch {
        return typeof customSchema === 'string' ? customSchema : JSON.stringify(customSchema, null, 2);
      }
    }
    return defaultJsonString;
  }, [customSchema, defaultJsonString]);

  const [jsonText, setJsonText] = useState(activeJsonString);
  const [parseError, setParseError] = useState<string | null>(null);
  const [savedFeedback, setSavedFeedback] = useState(false);

  useEffect(() => {
    setJsonText(activeJsonString);
    setParseError(null);
  }, [activeJsonString]);

  const handleFormat = (e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      const parsed = JSON.parse(jsonText);
      const formatted = JSON.stringify(parsed, null, 2);
      setJsonText(formatted);
      setParseError(null);
    } catch (err) {
      setParseError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleSave = (e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      const parsed = JSON.parse(jsonText);
      if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed)) {
        setParseError('JSON должен быть объектом { "type": "function", "function": { ... } }');
        return;
      }
      setToolCustomSchema(tool.id, parsed);
      setParseError(null);
      setSavedFeedback(true);
      setTimeout(() => setSavedFeedback(false), 2000);
    } catch (err) {
      setParseError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleReset = (e: React.MouseEvent) => {
    e.stopPropagation();
    resetToolCustomSchema(tool.id);
    setJsonText(defaultJsonString);
    setParseError(null);
  };

  return (
    <div
      onClick={(e) => e.stopPropagation()}
      className="mt-2.5 p-2.5 rounded-md border border-[#ebebeb] dark:border-[#21262d] bg-[#f9f9fb] dark:bg-[#07090e] space-y-2 select-none"
    >
      <div className="flex items-center justify-between gap-2 border-b border-[#ebebeb] dark:border-[#21262d] pb-1.5 flex-wrap">
        <div className="flex items-center gap-2">
          <span className="text-[10px] font-mono uppercase tracking-wider text-[#666666] dark:text-[#8b949e] font-semibold">
            Схема инструмента (JSON)
          </span>
          {isCustomized && (
            <span className="px-1.5 py-0.5 rounded text-[9px] font-mono bg-amber-500/10 text-amber-600 dark:text-amber-400 border border-amber-500/30">
              Кастомная схема
            </span>
          )}
          {savedFeedback && (
            <span className="px-1.5 py-0.5 rounded text-[9px] font-mono bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 border border-emerald-500/30 animate-pulse">
              Сохранено
            </span>
          )}
        </div>

        <div className="flex items-center gap-1.5 flex-wrap">
          <button
            type="button"
            onClick={handleFormat}
            className="px-2 py-0.5 rounded border border-[#d4d4d8] dark:border-[#30363d] bg-white dark:bg-[#161b22] text-[#171717] dark:text-[#f0f6fc] text-[10px] font-mono hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] transition cursor-pointer"
          >
            Форматировать
          </button>
          <button
            type="button"
            onClick={handleSave}
            className="px-2 py-0.5 rounded border border-[#171717] dark:border-[#f0f6fc] bg-[#171717] dark:bg-[#f0f6fc] text-white dark:text-[#0d1117] text-[10px] font-mono font-medium hover:opacity-90 transition cursor-pointer"
          >
            Сохранить JSON
          </button>
          {isCustomized && (
            <button
              type="button"
              onClick={handleReset}
              className="px-2 py-0.5 rounded border border-[#fecaca] dark:border-[#da3633]/40 bg-[#fef2f2] dark:bg-[#3b1219] text-[#991b1b] dark:text-[#ff7b72] text-[10px] font-mono hover:opacity-90 transition cursor-pointer"
            >
              Сбросить по умолчанию
            </button>
          )}
        </div>
      </div>

      <div className="pt-1 space-y-1.5">
        <textarea
          value={jsonText}
          onChange={(e) => {
            setJsonText(e.target.value);
            if (parseError) setParseError(null);
          }}
          rows={11}
          spellCheck={false}
          className="w-full rounded border border-[#d4d4d8] dark:border-[#30363d] bg-white dark:bg-[#0d1117] text-[#24292f] dark:text-[#c9d1d9] font-mono text-xs p-2.5 focus:outline-hidden focus:border-[#171717] dark:focus:border-[#58a6ff] leading-relaxed resize-y select-text"
          placeholder='{ "type": "function", "function": { ... } }'
        />

        {parseError && (
          <p className="text-[11px] font-mono text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-950/30 border border-red-200 dark:border-red-900/50 rounded px-2.5 py-1.5">
            Ошибка: {parseError}
          </p>
        )}
      </div>
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

const getCategoryLabel = (catKey: string, t: (key: TranslationKey) => string) => {
  const catTransKey = `toolPolicy.category.${catKey}` as TranslationKey;
  const label = t(catTransKey);
  return label !== catTransKey ? label : catKey;
};

export default function ToolPolicyModal() {
  const { zoom } = useUiZoom();
  const {
    isModalOpen,
    closeModal,
    setStageTool,
    isToolAllowed,
    toolDescriptions,
    toolCustomSchemas,
  } = useToolPolicyStore();
  const { t } = useTranslation();

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
      setCustomFormError(t('toolPolicy.errNamePattern'));
      return;
    }
    if (!description) {
      setCustomFormError(t('toolPolicy.errDescRequired'));
      return;
    }
    if (!command) {
      setCustomFormError(t('toolPolicy.errCommandRequired'));
      return;
    }
    let parameters: Record<string, unknown>;
    try {
      const parsed: unknown = JSON.parse(paramsRaw);
      if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed)) {
        setCustomFormError(t('toolPolicy.errParamsObject'));
        return;
      }
      parameters = parsed as Record<string, unknown>;
    } catch (err) {
      setCustomFormError(t('toolPolicy.errParamsJson', { error: err instanceof Error ? err.message : String(err) }));
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
    const confirmed = window.confirm(t('toolPolicy.deleteToolConfirm', { name }));
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
    <div style={getZoomOverlayStyle(zoom)} className="fixed inset-0 z-[80] flex items-center justify-center bg-black/60 dark:bg-black/80 p-3">
      <div className="w-full max-w-4xl rounded-lg border border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#0d1117] text-[#171717] dark:text-[#f0f6fc] shadow-md overflow-hidden flex flex-col max-h-[85vh] transition-colors">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-[#ebebeb] dark:border-[#21262d] px-4 py-3 bg-white dark:bg-[#0d1117]">
          <div>
            <h2 className="text-sm font-semibold text-[#171717] dark:text-[#f0f6fc]">
              {t('toolPolicy.modalTitle')}
            </h2>
            <p className="text-[11px] text-[#666666] dark:text-[#8b949e] font-mono mt-0.5">
              {t('toolPolicy.modalSubtitle')}
            </p>
          </div>
          <button
            onClick={closeModal}
            className="p-1 rounded-md text-[#666666] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#f4f4f5] dark:hover:bg-[#161b22] transition cursor-pointer"
            aria-label={t('common.close')}
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Body Content */}
        <div className="flex-1 min-h-0 overflow-y-auto p-4 space-y-4 font-mono text-xs">
          <div className="overflow-x-auto rounded-md border border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#0d1117]">
            <table className="w-full text-left border-collapse text-xs">
              <thead>
                <tr className="border-b border-[#ebebeb] dark:border-[#21262d] bg-[#f9f9f9] dark:bg-[#161b22] text-[#666666] dark:text-[#8b949e] text-[10px] uppercase">
                  <th className="py-2.5 px-3 font-medium">{t('toolPolicy.toolHeader')}</th>
                  <th className="py-2.5 px-2 text-right font-medium">{t('toolPolicy.weightHeader')}</th>
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
                            {t('toolPolicy.builtin')}
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
                          {getCategoryLabel(catKey, t)}
                        </td>
                      </tr>
                      {toolsInCat.map((tool) => {
                        const isExpanded = expandedToolId === tool.id;
                        const isChild = Boolean(tool.parentTool);
                        const toolLabelKey = getToolTranslationKey(tool.id, 'label') as TranslationKey;
                        const toolLabel = t(toolLabelKey) !== toolLabelKey ? t(toolLabelKey) : tool.label;

                        return (
                          <Fragment key={tool.id}>
                            <tr
                              className={`hover:bg-[#fafafa] dark:hover:bg-[#161b22] transition text-[#171717] dark:text-[#f0f6fc] ${
                                isChild ? 'bg-[#fafafc] dark:bg-[#090a0f]' : ''
                              }`}
                            >
                              <td className={`py-2 px-3 align-middle ${isChild ? 'pl-7' : ''}`}>
                                <div className="flex items-start gap-1.5 text-left w-full">
                                  {isChild && (
                                    <span className="text-[#8f8f8f] dark:text-[#6e7681] font-mono text-xs select-none shrink-0 mt-0.5 leading-none">
                                      {tool.id === 'subagent_list' ? '└──' : '├──'}
                                    </span>
                                  )}
                                  <button
                                    type="button"
                                    onClick={() => toggleExpanded(tool.id)}
                                    className="flex items-start gap-1.5 text-left cursor-pointer min-w-0 flex-1 group"
                                  >
                                    <ChevronDown
                                      className={`w-3.5 h-3.5 shrink-0 mt-0.5 text-[#8f8f8f] dark:text-[#8b949e] transition-transform ${isExpanded ? 'rotate-180' : ''}`}
                                    />
                                    <div className="min-w-0 flex-1">
                                      <div className="flex items-center gap-1.5 flex-wrap">
                                        <span
                                          className={`font-medium text-[#171717] dark:text-[#f0f6fc] leading-tight group-hover:underline underline-offset-2 ${
                                            tool.isSubagentMaster ? 'font-semibold' : ''
                                          }`}
                                        >
                                          {toolLabel}
                                        </span>
                                        {toolCustomSchemas[tool.id] !== undefined ? (
                                          <span className="px-1.5 py-0.2 rounded text-[9px] font-mono bg-amber-500/10 text-amber-600 dark:text-amber-400 border border-amber-500/30">
                                            Кастомная схема
                                          </span>
                                        ) : toolDescriptions[tool.id] !== undefined && toolDescriptions[tool.id] !== tool.description ? (
                                          <span className="px-1.5 py-0.2 rounded text-[9px] font-mono bg-amber-500/10 text-amber-600 dark:text-amber-400 border border-amber-500/30">
                                            Кастомное описание
                                          </span>
                                        ) : null}
                                      </div>
                                      <div className="text-[10px] text-[#8f8f8f] dark:text-[#8b949e] font-mono leading-tight mt-0.5">
                                        {tool.name}
                                      </div>
                                    </div>
                                  </button>
                                </div>
                              </td>

                              <td className="py-2 px-2 text-right text-[#666666] dark:text-[#8b949e] tabular-nums text-[11px] align-middle">
                                {tool.tokenWeight}
                              </td>

                              {agents.map((agent) => {
                                const isMasterAllowed = isChild && tool.parentTool
                                  ? isToolAllowed(tool.parentTool, agent.role)
                                  : true;
                                const isAllowed = isToolAllowed(tool.id, agent.role);
                                const isDisabled = isChild && !isMasterAllowed;
                                const masterLabel = tool.parentTool
                                  ? (t(getToolTranslationKey(tool.parentTool, 'label') as TranslationKey) !== getToolTranslationKey(tool.parentTool, 'label')
                                      ? t(getToolTranslationKey(tool.parentTool, 'label') as TranslationKey)
                                      : tool.parentTool)
                                  : '';

                                return (
                                  <td key={agent.role} className="py-2 px-2 text-center border-l border-[#ebebeb] dark:border-[#21262d] align-middle">
                                    {isDisabled ? (
                                      <span className="inline-flex justify-center opacity-40 pt-0.5">
                                        <Toggle
                                          enabled={false}
                                          disabled={true}
                                          onToggle={() => {}}
                                          title={t('toolPolicy.masterDisabledNotice', {
                                            tool: toolLabel,
                                            master: masterLabel,
                                            agent: agent.role,
                                          })}
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
                                          title={t('toolPolicy.toolAgentStatus', {
                                            tool: toolLabel,
                                            agent: agent.role,
                                            status: isAllowed ? t('toolPolicy.statusEnabled') : t('toolPolicy.statusDisabled'),
                                          })}
                                        />
                                      </span>
                                    )}
                                  </td>
                                );
                              })}
                            </tr>
                            {isExpanded && (
                              <tr className="bg-[#f9f9fb] dark:bg-[#07090e] border-b border-[#ebebeb] dark:border-[#21262d]">
                                <td colSpan={2 + agents.length} className="p-3">
                                  <div className="space-y-2 max-w-3xl">
                                    <ToolParamsEditor toolId={tool.id} />
                                    <ToolSchemaEditor tool={tool} />
                                  </div>
                                </td>
                              </tr>
                            )}
                          </Fragment>
                        );
                      })}
                    </Fragment>
                  );
                })}

                {customTools.map((customTool) => {
                  const customId = `custom:${customTool.name}`;
                  const isExpanded = expandedToolId === customId;

                  return (
                    <Fragment key={customId}>
                      <tr className="hover:bg-[#fafafa] dark:hover:bg-[#161b22] transition text-[#171717] dark:text-[#f0f6fc]">
                        <td className="py-2 px-3 align-middle">
                          <div className="flex items-start gap-1.5 text-left w-full">
                            <button
                              type="button"
                              onClick={() => toggleExpanded(customId)}
                              className="flex items-start gap-1.5 text-left cursor-pointer min-w-0 flex-1 group"
                            >
                              <ChevronDown
                                className={`w-3.5 h-3.5 shrink-0 mt-0.5 text-[#8f8f8f] dark:text-[#8b949e] transition-transform ${isExpanded ? 'rotate-180' : ''}`}
                              />
                              <div className="min-w-0 flex-1">
                                <div className="flex items-center gap-1.5">
                                  <span className="font-medium text-[#171717] dark:text-[#f0f6fc] leading-tight group-hover:underline underline-offset-2">
                                    {customTool.name}
                                  </span>
                                  <span className="rounded border border-[#ebebeb] dark:border-[#30363d] bg-[#f4f4f5] dark:bg-[#161b22] px-1 py-px text-[10px] leading-none text-[#666666] dark:text-[#8b949e] align-middle">
                                    {t('toolPolicy.customBadge')}
                                  </span>
                                </div>
                                <div className="text-[10px] text-[#8f8f8f] dark:text-[#8b949e] font-mono leading-tight mt-0.5 truncate">
                                  {customTool.description}
                                </div>
                              </div>
                            </button>
                          </div>
                        </td>

                        <td className="py-2 px-2 text-right text-[#8f8f8f] tabular-nums text-[11px] align-middle">
                          —
                        </td>

                        {agents.map((agent) => {
                          const isCustomAllowed = isToolAllowed(customTool.name, agent.role);

                          return (
                            <td key={agent.role} className="py-2 px-2 text-center border-l border-[#ebebeb] dark:border-[#21262d] align-middle">
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
                                  title={t('toolPolicy.toolAgentStatus', {
                                    tool: customTool.name,
                                    agent: agent.role,
                                    status: isCustomAllowed ? t('toolPolicy.statusEnabled') : t('toolPolicy.statusDisabled'),
                                  })}
                                />
                              </span>
                            </td>
                          );
                        })}
                      </tr>

                      {isExpanded && (
                        <tr className="bg-[#f9f9fb] dark:bg-[#07090e] border-b border-[#ebebeb] dark:border-[#21262d]">
                          <td colSpan={2 + agents.length} className="p-3">
                            <div className="space-y-1.5 text-xs font-mono">
                              <div className="flex items-center justify-between">
                                <span className="text-[#666666] dark:text-[#8b949e]">{t('toolPolicy.commandPrefix')} <code className="bg-[#f4f4f5] dark:bg-[#21262d] px-1 py-0.5 rounded">{customTool.command}</code></span>
                                <button
                                  type="button"
                                  onClick={() => void handleDeleteCustomTool(customTool.name)}
                                  className="px-2 py-1 rounded text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-950/40 text-xs flex items-center gap-1 cursor-pointer"
                                >
                                  <Trash2 className="h-3 w-3" />
                                  {t('common.delete')}
                                </button>
                              </div>
                            </div>
                          </td>
                        </tr>
                      )}
                    </Fragment>
                  );
                })}
              </tbody>
            </table>
          </div>

          {/* Custom Tools Creation Form */}
          <div className="pt-2">
            <h3 className="text-xs font-semibold text-[#171717] dark:text-[#f0f6fc] font-sans">
              {t('toolPolicy.customTools')}
            </h3>

            {customLoading && (
              <p className="text-[11px] text-[#666666] dark:text-[#8b949e] font-sans mt-2">{t('common.loading')}</p>
            )}
            {customListError && (
              <p className="text-[11px] font-sans mt-2 text-red-700 dark:text-red-400 bg-red-50 dark:bg-red-950/30 border border-red-200 dark:border-red-900/50 rounded-md px-3 py-1.5">
                {customListError}
              </p>
            )}

            <form onSubmit={(e) => void handleAddCustomTool(e)} className="mt-3 p-3 rounded-md border border-[#ebebeb] dark:border-[#21262d] bg-[#fafafa] dark:bg-[#161b22] space-y-2.5">
              <p className="text-xs font-medium text-[#171717] dark:text-[#f0f6fc] font-sans">
                {t('toolPolicy.addTool')}
              </p>
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
                <div>
                  <label className="block font-sans text-[#4d4d4d] dark:text-[#c9d1d9] font-medium mb-1 text-[11px]">
                    {t('toolPolicy.nameLabel')}
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
                    {t('toolPolicy.commandLabel')}
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
                  {t('toolPolicy.descLabel')}
                </label>
                <input
                  type="text"
                  value={customDescription}
                  onChange={(e) => setCustomDescription(e.target.value)}
                  placeholder={t('toolPolicy.descPlaceholder')}
                  className="w-full px-3 py-1.5 rounded-md bg-white dark:bg-[#0d1117] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] placeholder:text-[#8f8f8f] dark:placeholder:text-[#8b949e] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc] font-sans text-xs transition"
                />
              </div>
              <div>
                <label className="block font-sans text-[#4d4d4d] dark:text-[#c9d1d9] font-medium mb-1 text-[11px]">
                  {t('toolPolicy.paramsLabel')}
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
                  {customSaving ? t('common.saving') : t('toolPolicy.add')}
                </button>
              </div>
            </form>
          </div>
        </div>

        {/* Footer */}
        <div className="border-t border-[#ebebeb] dark:border-[#21262d] bg-[#f9f9f9] dark:bg-[#161b22] px-4 py-2.5 flex items-center justify-end font-mono text-xs">
          <button
            onClick={closeModal}
            className="px-3.5 py-1.5 rounded-md bg-[#171717] dark:bg-[#21262d] hover:bg-[#262626] dark:hover:bg-[#30363d] text-white font-medium transition cursor-pointer shadow-xs"
          >
            {t('common.close')}
          </button>
        </div>
      </div>
    </div>
  );
}
