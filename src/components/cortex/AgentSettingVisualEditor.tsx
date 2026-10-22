import React, { useState, useEffect, useMemo, useRef } from 'react';
import { useTranslation } from '../../stores/languageStore';
import { useSettingsStore } from '../../stores/settingsStore';
import {
  Bot,
  Cpu,
  Thermometer,
  Layers,
  Maximize2,
  Wrench,
  CheckCircle2,
  ShieldCheck,
  Search,
  Check,
  AlertTriangle,
  Code2,
  Terminal,
  FileText,
  Edit3,
  FileCode,
  Compass,
  Globe,
  Download,
  CheckSquare,
  ArrowRight,
  RotateCcw,
  HelpCircle,
  PlusSquare,
  List,
  History,
  BookOpen,
} from 'lucide-react';

export interface AgentSettingVisualEditorProps {
  content: string;
  onChange: (newContent: string) => void;
  onSwitchToCode?: () => void;
}

interface ParsedSetting {
  model: string | null;
  temperature: number;
  contextWindow: number;
  maxTokens: number;
  allowedTools: string[] | null;
  rawExtra: Record<string, unknown>;
}

interface ToolDefinition {
  name: string;
  label: string;
  description: string;
  category: string;
  icon: React.ComponentType<{ className?: string }>;
}

const CANONICAL_TOOLS: ToolDefinition[] = [
  {
    name: 'execute_command',
    label: 'Команды терминала',
    description: 'Выполнение команд в изолированном worktree',
    category: 'system',
    icon: Terminal,
  },
  {
    name: 'read_file',
    label: 'Чтение файла',
    description: 'Чтение текстовых файлов UTF-8 с пагинацией строк',
    category: 'fs',
    icon: FileText,
  },
  {
    name: 'edit_file',
    label: 'Целевое редактирование',
    description: 'Точечная замена строк и блоков кода (str_replace)',
    category: 'fs',
    icon: Edit3,
  },
  {
    name: 'write_file',
    label: 'Запись файла',
    description: 'Атомарная запись полного содержимого файла',
    category: 'fs',
    icon: FileCode,
  },
  {
    name: 'glob',
    label: 'Поиск по маске',
    description: 'Поиск файлов по glob-паттерну в репозитории',
    category: 'fs',
    icon: Compass,
  },
  {
    name: 'grep',
    label: 'Поиск по тексту',
    description: 'Поиск по регулярным выражениям с номерами строк',
    category: 'fs',
    icon: Search,
  },
  {
    name: 'web_search',
    label: 'Поиск в сети',
    description: 'Поиск документации и внешних решений в сети',
    category: 'web',
    icon: Globe,
  },
  {
    name: 'web_fetch',
    label: 'Загрузка веб-страниц',
    description: 'Загрузка HTML/страниц с преобразованием в Markdown',
    category: 'web',
    icon: Download,
  },
  {
    name: 'subagent',
    label: 'Вызов субагентов',
    description: 'Синхронный запуск специализированного дочернего агента',
    category: 'agent',
    icon: Bot,
  },
  {
    name: 'subagent_bg',
    label: 'Фоновый субагент',
    description: 'Асинхронный запуск субагента в фоне',
    category: 'agent',
    icon: Cpu,
  },
  {
    name: 'collect_subagent',
    label: 'Собрать субагентов',
    description: 'Ожидание отчётов фоновых субагентов без поллинга',
    category: 'agent',
    icon: Layers,
  },
  {
    name: 'todo_write',
    label: 'План выполнения',
    description: 'Фиксация и обновление тактического чек-листа шагов',
    category: 'agent',
    icon: CheckSquare,
  },
  {
    name: 'advance_task',
    label: 'Передать этап',
    description: 'Передача задачи на следующий этап с эстафетой',
    category: 'workflow',
    icon: ArrowRight,
  },
  {
    name: 'reject_task',
    label: 'Вернуть на доработку',
    description: 'Возврат задачи на предыдущий этап с замечаниями',
    category: 'workflow',
    icon: RotateCcw,
  },
  {
    name: 'question',
    label: 'Вопрос пользователю',
    description: 'Интерактивный вопрос оператору в интерфейсе',
    category: 'interactive',
    icon: HelpCircle,
  },
  {
    name: 'create_task',
    label: 'Создать задачу',
    description: 'Создание карточки задачи на Канбан-доске',
    category: 'orchestration',
    icon: PlusSquare,
  },
  {
    name: 'complete_task',
    label: 'Завершить задачу',
    description: 'Финализация задачи и перевод в статус Готово',
    category: 'orchestration',
    icon: CheckCircle2,
  },
  {
    name: 'list_tasks',
    label: 'Реестр задач',
    description: 'Компактный список задач с Канбан-доски',
    category: 'workflow',
    icon: List,
  },
  {
    name: 'get_task_history',
    label: 'История задачи',
    description: 'Пошаговый рентген выполнения задачи',
    category: 'workflow',
    icon: History,
  },
  {
    name: 'load_rule',
    label: 'Загрузка правил',
    description: 'Динамическая JIT-загрузка стандартов проекта',
    category: 'system',
    icon: BookOpen,
  },
];

const MODEL_PRESETS = [
  { id: 'inherit', label: 'inherit', desc: 'глобальные' },
  { id: 'gemini-3.8-flash-high', label: 'gemini-3.8-flash-high', desc: 'Google' },
  { id: 'claude-3-7-sonnet', label: 'claude-3-7-sonnet', desc: 'Anthropic' },
  { id: 'deepseek-chat', label: 'deepseek-chat', desc: 'DeepSeek' },
];

const CONTEXT_CHIPS = [
  { value: 128000, label: '128K' },
  { value: 200000, label: '200K' },
  { value: 1000000, label: '1M' },
];

const MAX_TOKENS_CHIPS = [
  { value: 4096, label: '4K' },
  { value: 8192, label: '8K' },
  { value: 16384, label: '16K' },
  { value: 65536, label: '64K' },
];

function parseSettingJson(raw: string): { parsed: ParsedSetting; error: string | null } {
  try {
    const trimmed = raw.trim();
    const obj = trimmed ? JSON.parse(trimmed) : {};
    if (typeof obj !== 'object' || obj === null || Array.isArray(obj)) {
      throw new Error('Ожидался JSON объект {...}');
    }

    const rawModel = obj.model;
    const model =
      rawModel === undefined || rawModel === null || rawModel === '' || rawModel === 'inherit'
        ? 'inherit'
        : String(rawModel);

    const temperature =
      typeof obj.temperature === 'number' && !isNaN(obj.temperature)
        ? Math.max(0, Math.min(1, obj.temperature))
        : 0.2;

    const rawCw = obj.contextWindow ?? obj.context_window;
    const contextWindow =
      typeof rawCw === 'number' && !isNaN(rawCw) && rawCw > 0 ? Math.round(rawCw) : 200000;

    const rawMt = obj.maxTokens ?? obj.max_tokens;
    const maxTokens =
      typeof rawMt === 'number' && !isNaN(rawMt) && rawMt > 0 ? Math.round(rawMt) : 8192;

    const rawTools = obj.allowedTools ?? obj.allowed_tools;
    let allowedTools: string[] | null = null;
    if (Array.isArray(rawTools)) {
      allowedTools = rawTools.map((t) => String(t).trim()).filter(Boolean);
    }

    const rawExtra: Record<string, unknown> = {};
    for (const [key, val] of Object.entries(obj)) {
      if (
        ![
          'model',
          'temperature',
          'contextWindow',
          'context_window',
          'maxTokens',
          'max_tokens',
          'allowedTools',
          'allowed_tools',
        ].includes(key)
      ) {
        rawExtra[key] = val;
      }
    }

    return {
      parsed: {
        model,
        temperature,
        contextWindow,
        maxTokens,
        allowedTools,
        rawExtra,
      },
      error: null,
    };
  } catch (err) {
    return {
      parsed: {
        model: 'inherit',
        temperature: 0.2,
        contextWindow: 200000,
        maxTokens: 8192,
        allowedTools: null,
        rawExtra: {},
      },
      error: err instanceof Error ? err.message : String(err),
    };
  }
}

function serializeSetting(parsed: ParsedSetting): string {
  const output: Record<string, unknown> = {};

  if (parsed.rawExtra._comment) {
    output._comment = parsed.rawExtra._comment;
  }

  // Model: если inherit или пусто — пишем 'inherit'
  output.model =
    !parsed.model || parsed.model === 'inherit' ? 'inherit' : parsed.model.trim();

  output.temperature = Number(parsed.temperature.toFixed(2));
  output.contextWindow = parsed.contextWindow;
  output.maxTokens = parsed.maxTokens;
  output.allowedTools = parsed.allowedTools;

  for (const [key, val] of Object.entries(parsed.rawExtra)) {
    if (key !== '_comment') {
      output[key] = val;
    }
  }

  return JSON.stringify(output, null, 2);
}

export default function AgentSettingVisualEditor({
  content,
  onChange,
  onSwitchToCode,
}: AgentSettingVisualEditorProps) {
  const { t } = useTranslation();
  const availableModels = useSettingsStore((s) => s.availableModels);

  const lastEmittedRef = useRef<string>(content);
  const [parseError, setParseError] = useState<string | null>(null);

  const [model, setModel] = useState<string>('inherit');
  const [temperature, setTemperature] = useState<number>(0.2);
  const [contextWindow, setContextWindow] = useState<number>(200000);
  const [maxTokens, setMaxTokens] = useState<number>(8192);
  const [isRestricted, setIsRestricted] = useState<boolean>(false);
  const [allowedTools, setAllowedTools] = useState<string[]>([]);
  const [rawExtra, setRawExtra] = useState<Record<string, unknown>>({});

  const [toolSearch, setToolSearch] = useState<string>('');

  // Синхронизация при изменении внешнего content
  useEffect(() => {
    if (content === lastEmittedRef.current) return;

    const { parsed, error } = parseSettingJson(content);
    if (error) {
      setParseError(error);
      return;
    }

    setParseError(null);
    setModel(parsed.model ?? 'inherit');
    setTemperature(parsed.temperature);
    setContextWindow(parsed.contextWindow);
    setMaxTokens(parsed.maxTokens);
    setRawExtra(parsed.rawExtra);

    if (parsed.allowedTools === null) {
      setIsRestricted(false);
      setAllowedTools(CANONICAL_TOOLS.map((t) => t.name));
    } else {
      setIsRestricted(true);
      setAllowedTools(parsed.allowedTools);
    }

    lastEmittedRef.current = content;
  }, [content]);

  // Первоначальный разбор при монтировании
  useEffect(() => {
    const { parsed, error } = parseSettingJson(content);
    if (error) {
      setParseError(error);
      return;
    }
    setParseError(null);
    setModel(parsed.model ?? 'inherit');
    setTemperature(parsed.temperature);
    setContextWindow(parsed.contextWindow);
    setMaxTokens(parsed.maxTokens);
    setRawExtra(parsed.rawExtra);

    if (parsed.allowedTools === null) {
      setIsRestricted(false);
      setAllowedTools(CANONICAL_TOOLS.map((t) => t.name));
    } else {
      setIsRestricted(true);
      setAllowedTools(parsed.allowedTools);
    }
    lastEmittedRef.current = content;
  }, []);

  // Функция эмиссии изменений в родительский activeFile.content
  const emitChanges = (
    nextModel: string,
    nextTemp: number,
    nextCw: number,
    nextMt: number,
    nextRestricted: boolean,
    nextTools: string[],
    nextExtra: Record<string, unknown>
  ) => {
    const nextSetting: ParsedSetting = {
      model: nextModel,
      temperature: nextTemp,
      contextWindow: nextCw,
      maxTokens: nextMt,
      allowedTools: nextRestricted ? nextTools : null,
      rawExtra: nextExtra,
    };
    const serialized = serializeSetting(nextSetting);
    lastEmittedRef.current = serialized;
    onChange(serialized);
  };

  const handleModelChange = (newModel: string) => {
    setModel(newModel);
    emitChanges(newModel, temperature, contextWindow, maxTokens, isRestricted, allowedTools, rawExtra);
  };

  const handleTemperatureChange = (newTemp: number) => {
    const clamped = Math.max(0, Math.min(1, Number(newTemp.toFixed(2))));
    setTemperature(clamped);
    emitChanges(model, clamped, contextWindow, maxTokens, isRestricted, allowedTools, rawExtra);
  };

  const handleContextWindowChange = (newCw: number) => {
    const val = Math.max(1000, newCw);
    setContextWindow(val);
    emitChanges(model, temperature, val, maxTokens, isRestricted, allowedTools, rawExtra);
  };

  const handleMaxTokensChange = (newMt: number) => {
    const val = Math.max(256, newMt);
    setMaxTokens(val);
    emitChanges(model, temperature, contextWindow, val, isRestricted, allowedTools, rawExtra);
  };

  const handleToggleRestricted = (restricted: boolean) => {
    setIsRestricted(restricted);
    let tools = allowedTools;
    if (restricted && tools.length === 0) {
      tools = CANONICAL_TOOLS.map((t) => t.name);
      setAllowedTools(tools);
    }
    emitChanges(model, temperature, contextWindow, maxTokens, restricted, tools, rawExtra);
  };

  const handleToggleTool = (toolName: string) => {
    const isCurrentlyActive = allowedTools.includes(toolName);
    const updated = isCurrentlyActive
      ? allowedTools.filter((t) => t !== toolName)
      : [...allowedTools, toolName];

    setAllowedTools(updated);
    emitChanges(model, temperature, contextWindow, maxTokens, isRestricted, updated, rawExtra);
  };

  const handleSelectAllTools = () => {
    const all = allTools.map((t) => t.name);
    setAllowedTools(all);
    emitChanges(model, temperature, contextWindow, maxTokens, isRestricted, all, rawExtra);
  };

  const handleClearAllTools = () => {
    setAllowedTools([]);
    emitChanges(model, temperature, contextWindow, maxTokens, isRestricted, [], rawExtra);
  };

  // Список всех уникальных моделей для выпадающего списка
  const allAvailableModels = useMemo(() => {
    const set = new Set<string>();
    for (const p of MODEL_PRESETS) {
      if (p.id !== 'inherit') set.add(p.id);
    }
    if (Array.isArray(availableModels)) {
      for (const m of availableModels) {
        if (m) set.add(m);
      }
    }
    return Array.from(set);
  }, [availableModels]);

  // Объединяем стандартные инструменты и любые кастомные из allowedTools
  const allTools = useMemo<ToolDefinition[]>(() => {
    const existing = new Set(CANONICAL_TOOLS.map((t) => t.name));
    const extras: ToolDefinition[] = [];
    for (const name of allowedTools) {
      if (!existing.has(name)) {
        extras.push({
          name,
          label: name,
          description: 'Пользовательский инструмент',
          category: 'custom',
          icon: Wrench,
        });
        existing.add(name);
      }
    }
    return [...CANONICAL_TOOLS, ...extras];
  }, [allowedTools]);

  // Фильтрация инструментов по поиску
  const filteredTools = useMemo(() => {
    if (!toolSearch.trim()) return allTools;
    const q = toolSearch.toLowerCase().trim();
    return allTools.filter(
      (t) =>
        t.name.toLowerCase().includes(q) ||
        t.label.toLowerCase().includes(q) ||
        t.description.toLowerCase().includes(q)
    );
  }, [toolSearch, allTools]);

  if (parseError) {
    return (
      <div className="p-6 space-y-4 max-w-xl mx-auto my-auto text-center font-sans">
        <div className="inline-flex p-3 rounded-full bg-amber-500/10 text-amber-500">
          <AlertTriangle className="h-8 w-8" />
        </div>
        <h3 className="text-sm font-semibold text-[#171717] dark:text-[#f0f6fc]">
          {t('ruleEditor.jsonParseError')}
        </h3>
        <p className="text-xs text-[#666666] dark:text-[#8b949e] font-mono bg-[#f4f4f5] dark:bg-[#161b22] p-3 rounded-md border border-[#ebebeb] dark:border-[#30363d] text-left break-all">
          {parseError}
        </p>
        {onSwitchToCode && (
          <button
            type="button"
            onClick={onSwitchToCode}
            className="inline-flex items-center space-x-2 px-4 py-2 rounded-md bg-[#171717] hover:bg-[#262626] dark:bg-[#238636] dark:hover:bg-[#2ea043] text-white text-xs font-mono font-medium transition cursor-pointer"
          >
            <Code2 className="h-4 w-4" />
            <span>{t('ruleEditor.switchToCode')}</span>
          </button>
        )}
      </div>
    );
  }

  const isModelInherit = !model || model === 'inherit';

  return (
    <div className="w-full h-full overflow-y-auto px-5 py-4 space-y-5 font-sans select-none text-[#171717] dark:text-[#f0f6fc]">
      {/* СЕКЦИЯ 1: МОДЕЛЬ И ТЕМПЕРАТУРА */}
      <section className="rounded-lg border border-[#ebebeb] dark:border-[#21262d] bg-[#fdfdfd] dark:bg-[#161b22]/70 p-4 space-y-4 shadow-2xs">
        <div className="flex items-center space-x-2 border-b border-[#ebebeb] dark:border-[#21262d] pb-2.5">
          <Bot className="h-4 w-4 text-[#6366f1] dark:text-[#818cf8]" />
          <h2 className="text-xs font-bold font-mono uppercase tracking-wider text-[#171717] dark:text-[#f0f6fc]">
            {t('ruleEditor.modelSection')}
          </h2>
        </div>

        {/* Выбор модели */}
        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <label className="text-xs font-medium text-[#171717] dark:text-[#f0f6fc] flex items-center space-x-1.5">
              <span>{t('ruleEditor.modelLabel')}</span>
              {isModelInherit && (
                <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-[#f4f4f5] dark:bg-[#21262d] border border-[#ebebeb] dark:border-[#30363d] text-[#666666] dark:text-[#8b949e]">
                  {t('ruleEditor.modelInherit')}
                </span>
              )}
            </label>
          </div>

          <div className="flex items-center space-x-2">
            <div className="relative flex-1">
              <input
                type="text"
                list="model-options"
                value={model === 'inherit' ? '' : model}
                onChange={(e) => handleModelChange(e.target.value || 'inherit')}
                placeholder={t('ruleEditor.modelPlaceholder')}
                className="w-full px-3 py-1.5 rounded-md border border-[#d4d4d8] dark:border-[#30363d] bg-white dark:bg-[#0d1117] text-[#171717] dark:text-[#f0f6fc] font-mono text-xs focus:outline-hidden focus:border-[#6366f1] dark:focus:border-[#818cf8] transition"
              />
              <datalist id="model-options">
                {allAvailableModels.map((m) => (
                  <option key={m} value={m} />
                ))}
              </datalist>
            </div>

            {model !== 'inherit' && (
              <button
                type="button"
                onClick={() => handleModelChange('inherit')}
                className="px-2.5 py-1.5 rounded-md text-xs font-mono border border-[#ebebeb] dark:border-[#30363d] bg-white dark:bg-[#0d1117] hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] text-[#666666] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] transition cursor-pointer"
              >
                Сбросить
              </button>
            )}
          </div>

          {/* Быстрые бейджи модели */}
          <div className="flex flex-wrap gap-1.5 pt-1">
            {MODEL_PRESETS.map((preset) => {
              const isSelected =
                preset.id === 'inherit'
                  ? isModelInherit
                  : model.toLowerCase() === preset.id.toLowerCase();
              return (
                <button
                  key={preset.id}
                  type="button"
                  onClick={() => handleModelChange(preset.id)}
                  className={`px-2.5 py-1 rounded text-xs font-mono transition cursor-pointer flex items-center space-x-1 border ${
                    isSelected
                      ? 'border-[#6366f1] bg-[#6366f1]/10 text-[#6366f1] dark:text-[#818cf8] font-semibold'
                      : 'border-[#ebebeb] dark:border-[#30363d] bg-white dark:bg-[#0d1117] text-[#666666] dark:text-[#8b949e] hover:border-[#d4d4d8] dark:hover:border-[#484f58] hover:text-[#171717] dark:hover:text-[#f0f6fc]'
                  }`}
                >
                  <span>{preset.label}</span>
                </button>
              );
            })}
          </div>
        </div>

        {/* Температура */}
        <div className="space-y-2 pt-2 border-t border-[#f4f4f5] dark:border-[#21262d]">
          <div className="flex items-center justify-between">
            <label className="text-xs font-medium text-[#171717] dark:text-[#f0f6fc] flex items-center space-x-1.5">
              <Thermometer className="h-3.5 w-3.5 text-[#ef4444]" />
              <span>{t('ruleEditor.temperatureLabel')}</span>
            </label>
            <div className="flex items-center space-x-1.5">
              <span className="font-mono text-xs font-bold tabular-nums px-2 py-0.5 rounded bg-white dark:bg-[#0d1117] border border-[#d4d4d8] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc]">
                {temperature.toFixed(2)}
              </span>
            </div>
          </div>

          <div className="flex items-center space-x-3">
            <span className="text-[11px] font-mono text-[#8f8f8f] dark:text-[#8b949e]">0.0</span>
            <input
              type="range"
              min="0"
              max="1"
              step="0.05"
              value={temperature}
              onChange={(e) => handleTemperatureChange(parseFloat(e.target.value))}
              className="flex-1 h-1.5 bg-[#e4e4e7] dark:bg-[#30363d] rounded-lg appearance-none cursor-pointer accent-[#6366f1] dark:accent-[#818cf8]"
            />
            <span className="text-[11px] font-mono text-[#8f8f8f] dark:text-[#8b949e]">1.0</span>
          </div>

          {/* Быстрые кнопки температуры */}
          <div className="flex flex-wrap gap-1.5 pt-0.5">
            <button
              type="button"
              onClick={() => handleTemperatureChange(0.0)}
              className={`px-2.5 py-1 rounded text-xs font-mono transition cursor-pointer border ${
                temperature === 0.0
                  ? 'border-[#6366f1] bg-[#6366f1]/10 text-[#6366f1] dark:text-[#818cf8] font-semibold'
                  : 'border-[#ebebeb] dark:border-[#30363d] bg-white dark:bg-[#0d1117] text-[#666666] dark:text-[#8b949e] hover:border-[#d4d4d8] dark:hover:border-[#484f58]'
              }`}
            >
              {t('ruleEditor.tempAccurate')}
            </button>
            <button
              type="button"
              onClick={() => handleTemperatureChange(0.2)}
              className={`px-2.5 py-1 rounded text-xs font-mono transition cursor-pointer border ${
                temperature === 0.2
                  ? 'border-[#6366f1] bg-[#6366f1]/10 text-[#6366f1] dark:text-[#818cf8] font-semibold'
                  : 'border-[#ebebeb] dark:border-[#30363d] bg-white dark:bg-[#0d1117] text-[#666666] dark:text-[#8b949e] hover:border-[#d4d4d8] dark:hover:border-[#484f58]'
              }`}
            >
              {t('ruleEditor.tempBalanced')}
            </button>
            <button
              type="button"
              onClick={() => handleTemperatureChange(0.7)}
              className={`px-2.5 py-1 rounded text-xs font-mono transition cursor-pointer border ${
                temperature === 0.7
                  ? 'border-[#6366f1] bg-[#6366f1]/10 text-[#6366f1] dark:text-[#818cf8] font-semibold'
                  : 'border-[#ebebeb] dark:border-[#30363d] bg-white dark:bg-[#0d1117] text-[#666666] dark:text-[#8b949e] hover:border-[#d4d4d8] dark:hover:border-[#484f58]'
              }`}
            >
              {t('ruleEditor.tempCreative')}
            </button>
          </div>
        </div>
      </section>

      {/* СЕКЦИЯ 2: ЛИМИТЫ КОНТЕКСТА И ТОКЕНОВ */}
      <section className="rounded-lg border border-[#ebebeb] dark:border-[#21262d] bg-[#fdfdfd] dark:bg-[#161b22]/70 p-4 space-y-4 shadow-2xs">
        <div className="flex items-center space-x-2 border-b border-[#ebebeb] dark:border-[#21262d] pb-2.5">
          <Maximize2 className="h-4 w-4 text-[#10b981] dark:text-[#34d399]" />
          <h2 className="text-xs font-bold font-mono uppercase tracking-wider text-[#171717] dark:text-[#f0f6fc]">
            {t('ruleEditor.limitsSection')}
          </h2>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {/* Размер контекстного окна */}
          <div className="space-y-2 p-3 rounded-md border border-[#f4f4f5] dark:border-[#21262d] bg-white dark:bg-[#0d1117]">
            <div className="flex items-center justify-between">
              <label className="text-xs font-medium text-[#171717] dark:text-[#f0f6fc]">
                {t('ruleEditor.contextWindowLabel')}
              </label>
              <span className="font-mono text-xs font-semibold tabular-nums text-[#6366f1] dark:text-[#818cf8]">
                {contextWindow.toLocaleString()}
              </span>
            </div>

            <input
              type="number"
              min="1000"
              max="2000000"
              step="1000"
              value={contextWindow}
              onChange={(e) => handleContextWindowChange(parseInt(e.target.value, 10) || 128000)}
              className="w-full px-2.5 py-1.5 rounded border border-[#d4d4d8] dark:border-[#30363d] bg-white dark:bg-[#161b22] text-[#171717] dark:text-[#f0f6fc] font-mono text-xs tabular-nums focus:outline-hidden focus:border-[#10b981] dark:focus:border-[#34d399]"
            />

            {/* Быстрые чипы */}
            <div className="flex flex-wrap gap-1.5 pt-0.5">
              {CONTEXT_CHIPS.map((chip) => {
                const isActive = contextWindow === chip.value;
                return (
                  <button
                    key={chip.value}
                    type="button"
                    onClick={() => handleContextWindowChange(chip.value)}
                    className={`px-2 py-0.5 rounded text-[11px] font-mono transition cursor-pointer border ${
                      isActive
                        ? 'border-[#10b981] bg-[#10b981]/10 text-[#059669] dark:text-[#34d399] font-bold'
                        : 'border-[#ebebeb] dark:border-[#30363d] bg-[#f9f9fb] dark:bg-[#161b22] text-[#666666] dark:text-[#8b949e] hover:border-[#d4d4d8]'
                    }`}
                  >
                    {chip.label}
                  </button>
                );
              })}
            </div>
            <p className="text-[10px] text-[#8f8f8f] dark:text-[#8b949e] font-sans leading-tight pt-1">
              {t('ruleEditor.contextWindowHint')}
            </p>
          </div>

          {/* Максимально токенов вывода */}
          <div className="space-y-2 p-3 rounded-md border border-[#f4f4f5] dark:border-[#21262d] bg-white dark:bg-[#0d1117]">
            <div className="flex items-center justify-between">
              <label className="text-xs font-medium text-[#171717] dark:text-[#f0f6fc]">
                {t('ruleEditor.maxTokensLabel')}
              </label>
              <span className="font-mono text-xs font-semibold tabular-nums text-[#10b981] dark:text-[#34d399]">
                {maxTokens.toLocaleString()}
              </span>
            </div>

            <input
              type="number"
              min="256"
              max="131072"
              step="512"
              value={maxTokens}
              onChange={(e) => handleMaxTokensChange(parseInt(e.target.value, 10) || 4096)}
              className="w-full px-2.5 py-1.5 rounded border border-[#d4d4d8] dark:border-[#30363d] bg-white dark:bg-[#161b22] text-[#171717] dark:text-[#f0f6fc] font-mono text-xs tabular-nums focus:outline-hidden focus:border-[#10b981] dark:focus:border-[#34d399]"
            />

            {/* Быстрые чипы */}
            <div className="flex flex-wrap gap-1.5 pt-0.5">
              {MAX_TOKENS_CHIPS.map((chip) => {
                const isActive = maxTokens === chip.value;
                return (
                  <button
                    key={chip.value}
                    type="button"
                    onClick={() => handleMaxTokensChange(chip.value)}
                    className={`px-2 py-0.5 rounded text-[11px] font-mono transition cursor-pointer border ${
                      isActive
                        ? 'border-[#10b981] bg-[#10b981]/10 text-[#059669] dark:text-[#34d399] font-bold'
                        : 'border-[#ebebeb] dark:border-[#30363d] bg-[#f9f9fb] dark:bg-[#161b22] text-[#666666] dark:text-[#8b949e] hover:border-[#d4d4d8]'
                    }`}
                  >
                    {chip.label}
                  </button>
                );
              })}
            </div>
            <p className="text-[10px] text-[#8f8f8f] dark:text-[#8b949e] font-sans leading-tight pt-1">
              {t('ruleEditor.maxTokensHint')}
            </p>
          </div>
        </div>
      </section>

      {/* СЕКЦИЯ 3: РАЗРЕШЕННЫЕ ИНСТРУМЕНТЫ */}
      <section className="rounded-lg border border-[#ebebeb] dark:border-[#21262d] bg-[#fdfdfd] dark:bg-[#161b22]/70 p-4 space-y-4 shadow-2xs">
        <div className="flex items-center justify-between border-b border-[#ebebeb] dark:border-[#21262d] pb-2.5">
          <div className="flex items-center space-x-2">
            <Wrench className="h-4 w-4 text-[#f59e0b]" />
            <h2 className="text-xs font-bold font-mono uppercase tracking-wider text-[#171717] dark:text-[#f0f6fc]">
              {t('ruleEditor.toolsSection')}
            </h2>
          </div>

          {isRestricted && (
            <span className="font-mono text-[11px] text-[#666666] dark:text-[#8b949e] tabular-nums">
              {t('ruleEditor.toolsCount', {
                active: allowedTools.length,
                total: allTools.length,
              })}
            </span>
          )}
        </div>

        {/* Переключатель: Все разрешены / Ограничить набор */}
        <div className="flex items-center rounded-lg border border-[#ebebeb] dark:border-[#30363d] bg-white dark:bg-[#0d1117] p-1 text-xs font-mono">
          <button
            type="button"
            onClick={() => handleToggleRestricted(false)}
            className={`flex-1 py-1.5 px-3 rounded-md transition cursor-pointer flex items-center justify-center space-x-1.5 ${
              !isRestricted
                ? 'bg-[#171717] text-white dark:bg-[#238636] font-medium shadow-2xs'
                : 'text-[#666666] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc]'
            }`}
          >
            <CheckCircle2 className="h-3.5 w-3.5" />
            <span>{t('ruleEditor.toolsAll')}</span>
          </button>
          <button
            type="button"
            onClick={() => handleToggleRestricted(true)}
            className={`flex-1 py-1.5 px-3 rounded-md transition cursor-pointer flex items-center justify-center space-x-1.5 ${
              isRestricted
                ? 'bg-[#171717] text-white dark:bg-[#238636] font-medium shadow-2xs'
                : 'text-[#666666] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc]'
            }`}
          >
            <ShieldCheck className="h-3.5 w-3.5" />
            <span>{t('ruleEditor.toolsRestricted')}</span>
          </button>
        </div>

        {!isRestricted ? (
          <div className="p-3 rounded-md border border-[#ebebeb] dark:border-[#21262d] bg-[#f9f9fb] dark:bg-[#0d1117] flex items-center space-x-2.5 text-xs text-[#666666] dark:text-[#8b949e]">
            <Check className="h-4 w-4 text-[#10b981] shrink-0" />
            <span>{t('ruleEditor.toolsAllNotice')}</span>
          </div>
        ) : (
          <div className="space-y-3 pt-1">
            {/* Панель действий: поиск + кнопки "Выбрать все" / "Очистить все" */}
            <div className="flex items-center justify-between gap-2 flex-wrap">
              <div className="relative flex-1 min-w-[180px]">
                <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-[#8f8f8f] dark:text-[#8b949e]" />
                <input
                  type="text"
                  value={toolSearch}
                  onChange={(e) => setToolSearch(e.target.value)}
                  placeholder={t('ruleEditor.toolsSearch')}
                  className="w-full pl-8 pr-2.5 py-1 rounded-md border border-[#d4d4d8] dark:border-[#30363d] bg-white dark:bg-[#0d1117] text-[#171717] dark:text-[#f0f6fc] font-mono text-xs focus:outline-hidden focus:border-[#6366f1] dark:focus:border-[#818cf8]"
                />
              </div>

              <div className="flex items-center space-x-1.5 shrink-0">
                <button
                  type="button"
                  onClick={handleSelectAllTools}
                  className="px-2.5 py-1 rounded text-xs font-mono border border-[#ebebeb] dark:border-[#30363d] bg-white dark:bg-[#0d1117] hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] text-[#171717] dark:text-[#f0f6fc] transition cursor-pointer"
                >
                  {t('ruleEditor.toolsSelectAll')}
                </button>
                <button
                  type="button"
                  onClick={handleClearAllTools}
                  className="px-2.5 py-1 rounded text-xs font-mono border border-[#ebebeb] dark:border-[#30363d] bg-white dark:bg-[#0d1117] hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] text-[#666666] dark:text-[#8b949e] hover:text-[#ef4444] dark:hover:text-[#f87171] transition cursor-pointer"
                >
                  {t('ruleEditor.toolsClearAll')}
                </button>
              </div>
            </div>

            {/* Компактная сетка тулов */}
            <div className="grid grid-cols-1 md:grid-cols-2 gap-2 max-h-[300px] overflow-y-auto pr-1">
              {filteredTools.map((tool) => {
                const isEnabled = allowedTools.includes(tool.name);
                const IconComponent = tool.icon;

                return (
                  <div
                    key={tool.name}
                    onClick={() => handleToggleTool(tool.name)}
                    className={`flex items-center justify-between p-2.5 rounded-lg border transition-all cursor-pointer select-none ${
                      isEnabled
                        ? 'border-[#171717]/20 dark:border-[#388bfd]/40 bg-white dark:bg-[#0d1117] shadow-2xs'
                        : 'border-[#ebebeb] dark:border-[#21262d] bg-[#f9f9fb] dark:bg-[#161b22]/40 opacity-60 hover:opacity-90'
                    } hover:border-[#171717]/40 dark:hover:border-[#58a6ff]/60`}
                  >
                    <div className="flex items-center space-x-2.5 min-w-0 pr-2">
                      <div
                        className={`p-1.5 rounded-md shrink-0 ${
                          isEnabled
                            ? 'bg-[#f4f4f5] dark:bg-[#21262d] text-[#171717] dark:text-[#f0f6fc]'
                            : 'bg-[#ebebeb] dark:bg-[#1c2128] text-[#8f8f8f] dark:text-[#6e7681]'
                        }`}
                      >
                        <IconComponent className="h-4 w-4" />
                      </div>
                      <div className="min-w-0">
                        <div className="flex items-center space-x-1.5">
                          <span className="font-mono text-xs font-semibold text-[#171717] dark:text-[#f0f6fc] truncate">
                            {tool.name}
                          </span>
                        </div>
                        <p className="text-[11px] text-[#666666] dark:text-[#8b949e] truncate leading-tight">
                          {tool.label}
                        </p>
                      </div>
                    </div>

                    {/* Тумблер переключателя */}
                    <div
                      role="switch"
                      aria-checked={isEnabled}
                      className={`relative w-8 h-4.5 rounded-full transition-colors shrink-0 ${
                        isEnabled
                          ? 'bg-[#171717] dark:bg-[#238636]'
                          : 'bg-[#e4e4e7] dark:bg-[#30363d]'
                      }`}
                    >
                      <span
                        className={`absolute top-0.5 h-3.5 w-3.5 rounded-full bg-white dark:bg-[#0d1117] shadow-xs transition-transform ${
                          isEnabled ? 'translate-x-3.5' : 'translate-x-0.5'
                        }`}
                      />
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        )}
      </section>
    </div>
  );
}
