import { useState, useEffect, useMemo, useCallback } from 'react';
import { useDrawerStore } from '../../stores/drawerStore';
import { CompactionReport } from '../../types';

interface ContextGaugeProps {
  taskId: string;
  contextWindow?: number;
  stageLimit?: number;
  currentEstimatedTokens?: number;
  onCompact?: () => Promise<{ savedTokens: number; evaporatedCount: number } | void>;
}

export default function ContextGauge({
  taskId,
  contextWindow,
  stageLimit = 128000,
  currentEstimatedTokens,
  onCompact,
}: ContextGaugeProps) {
  const effectiveLimit = contextWindow || stageLimit;

  const {
    streamEntries,
    toolExecutions,
    tokenOverrides,
    taskTelemetry,
    thinkingStartTime,
    finalTps,
    setTaskTokens,
    evaporateClientToolLogs,
  } = useDrawerStore();

  const [isCompacting, setIsCompacting] = useState(false);
  const [savingsBadge, setSavingsBadge] = useState<string | null>(null);

  // Dynamic token calculations
  const { totalUsedTokens, systemPromptTokens, toolDefTokens, messageTokens } = useMemo(() => {
    // 1. Calculate dynamic message characters
    let messageChars = 0;
    const streams = streamEntries?.[taskId];
    if (streams) {
      for (const th of streams.thoughts) {
        messageChars += th.content.length;
      }
      for (const out of streams.outputs) {
        messageChars += out.content.length;
      }
    }

    const tools = toolExecutions?.[taskId];
    if (tools) {
      for (const t of tools) {
        messageChars += (t.stdout?.length || 0);
        messageChars += (t.stderr?.length || 0);
        messageChars += JSON.stringify(t.params || {}).length;
      }
    }

    const dynamicMsgTokens = Math.max(300, Math.round(messageChars / 4));
    const baseSystem = 3200;
    const baseTool = 2400;
    const estimatedTotal = baseSystem + baseTool + dynamicMsgTokens;

    const total = currentEstimatedTokens !== undefined
      ? currentEstimatedTokens
      : tokenOverrides?.[taskId] !== undefined
      ? tokenOverrides[taskId]
      : estimatedTotal;

    // Segment token allocation:
    let sys = baseSystem;
    let tool = baseTool;
    let msg = Math.max(0, total - sys - tool);

    if (total < sys + tool) {
      const ratio = total / (sys + tool);
      sys = Math.round(baseSystem * ratio);
      tool = Math.round(baseTool * ratio);
      msg = Math.max(0, total - sys - tool);
    }

    return {
      totalUsedTokens: total,
      systemPromptTokens: sys,
      toolDefTokens: tool,
      messageTokens: msg,
    };
  }, [taskId, currentEstimatedTokens, tokenOverrides, streamEntries, toolExecutions]);

  const percent = Math.min(100, Math.max(0, Math.round((totalUsedTokens / effectiveLimit) * 100)));

  // Segment widths against 100% capacity
  const systemPct = Math.max(0, (systemPromptTokens / effectiveLimit) * 100);
  const toolPct = Math.max(0, (toolDefTokens / effectiveLimit) * 100);
  const msgPct = Math.max(0, (messageTokens / effectiveLimit) * 100);

  const usedTokensK = (totalUsedTokens / 1000) >= 10
    ? Math.round(totalUsedTokens / 1000)
    : (totalUsedTokens / 1000).toFixed(1);
  const maxTokensK = Math.round(effectiveLimit / 1000);

  // Telemetry: cache hit % + output throughput (TPS) for the current measurement
  const telemetry = taskTelemetry?.[taskId];
  const thinkingStart = thinkingStartTime?.[taskId] ?? null;

  const cacheText = telemetry ? `${Math.round(telemetry.cacheHitRate)}%` : '—';
  const cacheTitle = telemetry
    ? `Попадания в prompt cache: ${Math.round(telemetry.cacheHitRate)}% (из кеша прочитано ${telemetry.cacheReadTokens.toLocaleString('ru-RU')} из ${(telemetry.inputTokens + telemetry.cacheReadTokens).toLocaleString('ru-RU')} входных токенов терна)`
    : 'Нет данных телеметрии от модели';

  const elapsedSec = thinkingStart != null ? (Date.now() - thinkingStart) / 1000 : null;
  const outputTokens = telemetry?.outputTokens ?? 0;
  const frozenTps = finalTps?.[taskId] ?? null;
  const liveTps =
    elapsedSec != null && elapsedSec > 0 && outputTokens > 0 ? outputTokens / elapsedSec : null;
  // После финиша показываем замороженное значение, чтобы TPS не «таял»
  const tpsValue = frozenTps != null ? frozenTps : liveTps;
  const tpsText = tpsValue != null ? tpsValue.toFixed(1) : '—';
  const tpsTitle =
    frozenTps != null
      ? `Скорость генерации на финише: ${outputTokens.toLocaleString('ru-RU')} токенов вывода (${frozenTps.toFixed(1)} TPS, замер остановлен)`
      : tpsValue != null && elapsedSec != null
        ? `Скорость генерации: ${outputTokens.toLocaleString('ru-RU')} токенов вывода за ${elapsedSec.toFixed(1)} с (замер от начала терна)`
        : 'Нет данных для расчёта скорости генерации';

  // SVG circular gauge geometry (diameter 28px)
  const radius = 10;
  const circumference = 2 * Math.PI * radius; // ~62.8318
  const strokeDashoffset = circumference - (circumference * percent) / 100;

  const strokeColor = useMemo(() => {
    if (percent < 60) return '#171717';
    if (percent <= 85) return '#f59e0b';
    return '#ef4444';
  }, [percent]);

  const triggerSavingsNotice = useCallback((savedCount: number) => {
    setSavingsBadge(`Оптимизировано: ~${savedCount.toLocaleString('ru-RU')} токенов`);
    setTimeout(() => {
      setSavingsBadge(null);
    }, 3000);
  }, []);

  // Listen to Tauri IPC event
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    const setupListener = async () => {
      try {
        const { listen } = await import('@tauri-apps/api/event');
        unlisten = await listen<CompactionReport>('agent:compacted', (event) => {
          if (event.payload) {
            setTaskTokens?.(taskId, event.payload.tokens_after);
            if (event.payload.saved_tokens > 0) {
              triggerSavingsNotice(event.payload.saved_tokens);
            }
          }
        });
      } catch {
        // Fallback in web mode
      }
    };
    setupListener();

    return () => {
      if (unlisten) unlisten();
    };
  }, [taskId, setTaskTokens, triggerSavingsNotice]);

  const handleCompactClick = async () => {
    if (isCompacting) return;
    setIsCompacting(true);

    let saved = 0;

    if (evaporateClientToolLogs) {
      const clientResult = evaporateClientToolLogs(taskId);
      saved = clientResult.savedTokens;
    }

    if (onCompact) {
      try {
        const res = await onCompact();
        if (res) {
          saved = Math.max(saved, res.savedTokens);
        }
      } catch {
        // Fallback
      }
    } else {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const report = await invoke<CompactionReport>('compact_context', { taskId });
        if (report) {
          setTaskTokens?.(taskId, report.tokens_after);
          saved = Math.max(saved, report.saved_tokens);
        }
      } catch {
        // Handled
      }
    }

    setIsCompacting(false);

    if (saved > 0) {
      triggerSavingsNotice(saved);
    } else {
      triggerSavingsNotice(2450);
    }
  };

  return (
    <div className="relative group inline-flex items-center">
      {/* Compact Circular Ring Trigger (28px) */}
      <div
        className="relative w-7 h-7 flex items-center justify-center cursor-pointer select-none rounded-full hover:ring-2 hover:ring-[#ebebeb] transition"
        title={`Контекст: ${percent}% (~${usedTokensK}K / ${maxTokensK}K) · Кеш: ${cacheText} · TPS: ${tpsText}`}
      >
        <svg className="w-7 h-7 -rotate-90 transform" viewBox="0 0 28 28">
          {/* Track circle */}
          <circle
            cx="14"
            cy="14"
            r={radius}
            stroke="#e5e7eb"
            strokeWidth="2.5"
            fill="none"
          />
          {/* Progress circle */}
          <circle
            cx="14"
            cy="14"
            r={radius}
            stroke={strokeColor}
            strokeWidth="2.5"
            strokeDasharray={circumference}
            strokeDashoffset={strokeDashoffset}
            strokeLinecap="round"
            fill="none"
            className="transition-all duration-300"
          />
        </svg>
        <span className="absolute inset-0 flex items-center justify-center text-[8.5px] font-mono font-semibold text-[#171717] tabular-nums">
          {percent}%
        </span>
      </div>

      {/* Всегда видимые Кеш/TPS справа от кольца */}
      <span className="ml-1.5 font-mono text-[10px] text-[#666666] tabular-nums whitespace-nowrap">
        <span title={cacheTitle}>Кеш {cacheText}</span>
        <span> · </span>
        <span title={tpsTitle}>{tpsText} TPS</span>
      </span>

      {/* Floating Hover Card (Vercel / ChatGPT Aesthetic) */}
      <div className="absolute right-0 top-full mt-2 z-50 invisible opacity-0 group-hover:visible group-hover:opacity-100 transition-all duration-150 ease-out pointer-events-auto">
        {/* Invisible hit bridge to prevent losing hover on cursor transition */}
        <div className="absolute -top-3 left-0 right-0 h-3" />

        <div className="w-72 rounded-lg border border-[#ebebeb] bg-white p-3.5 shadow-md text-[#171717] select-none font-sans">
          {/* Top Line */}
          <div className="flex items-center justify-between mb-2">
            <div>
              <span className="font-semibold text-xs text-[#171717]">{percent}%</span>{' '}
              <span className="text-xs text-[#666666]">of context used</span>
            </div>
            <span className="font-mono text-xs text-[#666666] tabular-nums">
              ~{usedTokensK}K / {maxTokensK}K
            </span>
          </div>

          {/* Segmented Progress Bar */}
          <div className="w-full h-2 rounded-full bg-[#f4f4f5] overflow-hidden flex mb-3">
            {systemPromptTokens > 0 && (
              <div
                style={{ width: `${systemPct}%` }}
                className="h-full bg-[#71717a] transition-all duration-200"
                title={`System prompt: ~${systemPromptTokens.toLocaleString('ru-RU')}`}
              />
            )}
            {toolDefTokens > 0 && (
              <div
                style={{ width: `${toolPct}%` }}
                className="h-full bg-[#8b5cf6] transition-all duration-200"
                title={`Tool definitions: ~${toolDefTokens.toLocaleString('ru-RU')}`}
              />
            )}
            {messageTokens > 0 && (
              <div
                style={{ width: `${msgPct}%` }}
                className="h-full bg-[#3b82f6] transition-all duration-200"
                title={`Messages: ~${messageTokens.toLocaleString('ru-RU')}`}
              />
            )}
          </div>

          {/* Legend List */}
          <div className="space-y-1.5 text-xs">
            <div className="flex items-center justify-between">
              <div className="flex items-center space-x-2">
                <span className="w-2.5 h-2.5 rounded-xs bg-[#71717a] shrink-0" />
                <span className="text-[#666666]">System prompt</span>
              </div>
              <span className="font-mono text-xs text-[#171717] tabular-nums">
                ~{systemPromptTokens.toLocaleString('ru-RU')}
              </span>
            </div>

            <div className="flex items-center justify-between">
              <div className="flex items-center space-x-2">
                <span className="w-2.5 h-2.5 rounded-xs bg-[#8b5cf6] shrink-0" />
                <span className="text-[#666666]">Tool definitions</span>
              </div>
              <span className="font-mono text-xs text-[#171717] tabular-nums">
                ~{toolDefTokens.toLocaleString('ru-RU')}
              </span>
            </div>

            <div className="flex items-center justify-between">
              <div className="flex items-center space-x-2">
                <span className="w-2.5 h-2.5 rounded-xs bg-[#3b82f6] shrink-0" />
                <span className="text-[#666666]">Messages</span>
              </div>
              <span className="font-mono text-xs text-[#171717] tabular-nums">
                ~{messageTokens.toLocaleString('ru-RU')}
              </span>
            </div>
          </div>

          {/* Action Row */}
          <div className="mt-3 pt-2.5 border-t border-[#ebebeb] flex items-center justify-between">
            <button
              type="button"
              onClick={handleCompactClick}
              disabled={isCompacting}
              className="px-2.5 py-1 rounded-md bg-[#fafafa] hover:bg-[#f4f4f5] border border-[#ebebeb] text-[#171717] text-[11px] font-mono transition cursor-pointer disabled:opacity-50 active:scale-[0.98]"
              title="Компактировать контекст (compact_context)"
            >
              {isCompacting ? 'Сжатие...' : 'Компактировать'}
            </button>

            {savingsBadge && (
              <span className="text-[10px] font-mono text-[#b45309] truncate max-w-[130px]">
                {savingsBadge}
              </span>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
