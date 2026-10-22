import { useState, useEffect, useMemo, useRef } from 'react';
import { useTranslation } from '../../stores/languageStore';
import { useSettingsStore } from '../../stores/settingsStore';
import {
  Bot,
  Thermometer,
  Maximize2,
  AlertTriangle,
  Code2,
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
  hasAllowedTools: boolean;
  allowedToolsKey?: string;
  allowedTools?: unknown;
  rawExtra: Record<string, unknown>;
}

const CONTEXT_CHIPS = [
  { value: 128000, label: '128K' },
  { value: 256000, label: '256K' },
  { value: 384000, label: '384K' },
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

    const hasAllowedTools = 'allowedTools' in obj || 'allowed_tools' in obj;
    const allowedToolsKey = 'allowedTools' in obj ? 'allowedTools' : ('allowed_tools' in obj ? 'allowed_tools' : undefined);
    const allowedTools = hasAllowedTools
      ? (obj.allowedTools !== undefined ? obj.allowedTools : obj.allowed_tools)
      : undefined;

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
        hasAllowedTools,
        allowedToolsKey,
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
        hasAllowedTools: false,
        allowedToolsKey: undefined,
        allowedTools: undefined,
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

  // Сохраняем исходное значение allowedTools, если оно уже присутствовало в файле, не навязывая изменений
  if (parsed.hasAllowedTools) {
    output[parsed.allowedToolsKey || 'allowedTools'] = parsed.allowedTools;
  }

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
  const settingsModel = useSettingsStore((s) => s.settings.model);

  const lastEmittedRef = useRef<string>(content);
  const [parseError, setParseError] = useState<string | null>(null);

  const [model, setModel] = useState<string>('inherit');
  const [temperature, setTemperature] = useState<number>(0.2);
  const [contextWindow, setContextWindow] = useState<number>(200000);
  const [maxTokens, setMaxTokens] = useState<number>(8192);
  const [allowedToolsData, setAllowedToolsData] = useState<{
    has: boolean;
    key?: string;
    value?: unknown;
  }>({ has: false });
  const [rawExtra, setRawExtra] = useState<Record<string, unknown>>({});

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
    setAllowedToolsData({
      has: parsed.hasAllowedTools,
      key: parsed.allowedToolsKey,
      value: parsed.allowedTools,
    });
    setRawExtra(parsed.rawExtra);

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
    setAllowedToolsData({
      has: parsed.hasAllowedTools,
      key: parsed.allowedToolsKey,
      value: parsed.allowedTools,
    });
    setRawExtra(parsed.rawExtra);
    lastEmittedRef.current = content;
  }, []);

  // Функция эмиссии изменений в родительский activeFile.content
  const emitChanges = (
    nextModel: string,
    nextTemp: number,
    nextCw: number,
    nextMt: number,
    nextExtra: Record<string, unknown>
  ) => {
    const nextSetting: ParsedSetting = {
      model: nextModel,
      temperature: nextTemp,
      contextWindow: nextCw,
      maxTokens: nextMt,
      hasAllowedTools: allowedToolsData.has,
      allowedToolsKey: allowedToolsData.key,
      allowedTools: allowedToolsData.value,
      rawExtra: nextExtra,
    };
    const serialized = serializeSetting(nextSetting);
    lastEmittedRef.current = serialized;
    onChange(serialized);
  };

  const handleModelChange = (newModel: string) => {
    setModel(newModel);
    emitChanges(newModel, temperature, contextWindow, maxTokens, rawExtra);
  };

  const handleTemperatureChange = (newTemp: number) => {
    const clamped = Math.max(0, Math.min(1, Number(newTemp.toFixed(2))));
    setTemperature(clamped);
    emitChanges(model, clamped, contextWindow, maxTokens, rawExtra);
  };

  const handleContextWindowChange = (newCw: number) => {
    const val = Math.max(1000, newCw);
    setContextWindow(val);
    emitChanges(model, temperature, val, maxTokens, rawExtra);
  };

  const handleMaxTokensChange = (newMt: number) => {
    const val = Math.max(256, newMt);
    setMaxTokens(val);
    emitChanges(model, temperature, contextWindow, val, rawExtra);
  };

  // Динамический список моделей: строго 'inherit', затем settings.model и availableModels
  const dynamicModels = useMemo(() => {
    const list: string[] = ['inherit'];
    const seen = new Set<string>(['inherit']);

    if (settingsModel && typeof settingsModel === 'string') {
      const trimmed = settingsModel.trim();
      if (trimmed && !seen.has(trimmed)) {
        list.push(trimmed);
        seen.add(trimmed);
      }
    }

    if (Array.isArray(availableModels)) {
      for (const m of availableModels) {
        if (typeof m === 'string') {
          const trimmed = m.trim();
          if (trimmed && !seen.has(trimmed)) {
            list.push(trimmed);
            seen.add(trimmed);
          }
        }
      }
    }

    return list;
  }, [availableModels, settingsModel]);

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
                {dynamicModels.map((m) => (
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
            {dynamicModels.map((item) => {
              const isSelected =
                item === 'inherit'
                  ? isModelInherit
                  : model.toLowerCase() === item.toLowerCase();
              return (
                <button
                  key={item}
                  type="button"
                  onClick={() => handleModelChange(item)}
                  className={`px-2.5 py-1 rounded text-xs font-mono transition cursor-pointer flex items-center space-x-1 border ${
                    isSelected
                      ? 'border-[#6366f1] bg-[#6366f1]/10 text-[#6366f1] dark:text-[#818cf8] font-semibold'
                      : 'border-[#ebebeb] dark:border-[#30363d] bg-white dark:bg-[#0d1117] text-[#666666] dark:text-[#8b949e] hover:border-[#d4d4d8] dark:hover:border-[#484f58] hover:text-[#171717] dark:hover:text-[#f0f6fc]'
                  }`}
                >
                  <span>{item}</span>
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
    </div>
  );
}
