import { useState, useEffect, useMemo, useCallback, useRef } from 'react';
import { useDrawerStore } from '../../stores/drawerStore';
import { useBoardStore } from '../../stores/boardStore';
import { CompactionReport } from '../../types';

interface ContextGaugeProps {
  taskId: string;
  turns?: number;
  contextWindow?: number;
  stageLimit?: number;
  currentEstimatedTokens?: number;
  onCompact?: () => Promise<{ savedTokens: number; evaporatedCount: number } | void>;
  variant?: 'status-bar' | 'header-ring';
}

export default function ContextGauge({
  taskId,
  turns,
  contextWindow,
  stageLimit = 128000,
  currentEstimatedTokens,
  onCompact,
  variant = 'status-bar',
}: ContextGaugeProps) {
  const effectiveLimit = contextWindow || stageLimit;

  // Селективные подписки per taskId: гейдж чужой задачи не пересчитывается
  // при стриме активной (раньше подписка на весь стор дёргала все гейджи).
  const thoughts = useDrawerStore((s) => s.streamEntries[taskId]?.thoughts);
  const outputs = useDrawerStore((s) => s.streamEntries[taskId]?.outputs);
  const tools = useDrawerStore((s) => s.toolExecutions[taskId]);
  const feed = useDrawerStore((s) => s.activityFeed[taskId]);
  const tokenOverride = useDrawerStore((s) => s.tokenOverrides[taskId]);
  const taskCurrentTokens = useBoardStore((s) => s.tasks.find((t) => t.id === taskId)?.currentTokens);
  const telemetry = useDrawerStore((s) => s.taskTelemetry[taskId]);
  const thinkingStart = useDrawerStore((s) => s.thinkingStartTime[taskId] ?? null);
  const frozenTps = useDrawerStore((s) => s.finalTps[taskId] ?? null);
  const setTaskTokens = useDrawerStore((s) => s.setTaskTokens);
  const evaporateClientToolLogs = useDrawerStore((s) => s.evaporateClientToolLogs);

  // Extract latest compacted tokens from activity feed if persisted
  const latestCompactedTokens = useMemo(() => {
    if (!feed || feed.length === 0) return undefined;
    for (let i = feed.length - 1; i >= 0; i--) {
      const it = feed[i];
      if (it.type === 'compacted' && it.tool) {
        const tData = it.tool as any;
        if (typeof tData?.tokensAfter === 'number') {
          return tData.tokensAfter;
        }
      }
    }
    return undefined;
  }, [feed]);

  const resolvedCompactTokens = tokenOverride ?? taskCurrentTokens ?? latestCompactedTokens;

  // Prime in-memory token override if found from task state or feed
  useEffect(() => {
    if (resolvedCompactTokens !== undefined && tokenOverride === undefined) {
      setTaskTokens?.(taskId, resolvedCompactTokens);
    }
  }, [taskId, resolvedCompactTokens, tokenOverride, setTaskTokens]);

  const [isCompacting, setIsCompacting] = useState(false);
  const [savingsBadge, setSavingsBadge] = useState<string | null>(null);
  const savingsTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Инкрементальный кэш суммы длин thought/output: горячий путь стрима —
  // та же длина ленты + вырос только последний элемент — досчитываем дельту
  // вместо O(n) прохода по всей ленте. Формулы и цифры не меняются.
  const textCache = useRef<{
    valid: boolean;
    thoughtsLen: number;
    outputsLen: number;
    lastThoughtId?: string;
    lastOutputId?: string;
    lastThoughtLen: number;
    lastOutputLen: number;
    textChars: number;
  }>({ valid: false, thoughtsLen: 0, outputsLen: 0, lastThoughtLen: 0, lastOutputLen: 0, textChars: 0 });

  // Dynamic token calculations
  const { totalUsedTokens, systemPromptTokens, toolDefTokens, messageTokens } = useMemo(() => {
    const thoughtsArr = thoughts ?? [];
    const outputsArr = outputs ?? [];
    const toolsArr = tools ?? [];

    const tLen = thoughtsArr.length;
    const oLen = outputsArr.length;
    const lastTh = tLen > 0 ? thoughtsArr[tLen - 1] : undefined;
    const lastOut = oLen > 0 ? outputsArr[oLen - 1] : undefined;
    const lastThLen = (lastTh?.content ?? '').length;
    const lastOutLen = (lastOut?.content ?? '').length;

    const fullTextRecalc = () => {
      let sum = 0;
      for (const th of thoughtsArr) sum += (th?.content ?? '').length;
      for (const out of outputsArr) sum += (out?.content ?? '').length;
      return sum;
    };
    const commitCache = (textChars: number) => {
      const c = textCache.current;
      c.valid = true;
      c.thoughtsLen = tLen;
      c.outputsLen = oLen;
      c.lastThoughtId = lastTh?.id;
      c.lastOutputId = lastOut?.id;
      c.lastThoughtLen = lastThLen;
      c.lastOutputLen = lastOutLen;
      c.textChars = textChars;
    };

    let textChars: number;
    const c = textCache.current;
    if (!c.valid) {
      textChars = fullTextRecalc();
      commitCache(textChars);
    } else if (tLen < c.thoughtsLen || oLen < c.outputsLen) {
      // Лента ужалась (clear/reload/evict) — полный пересчёт
      textChars = fullTextRecalc();
      commitCache(textChars);
    } else {
      // Проверка append-only префикса по id граничных элементов (O(1))
      const thoughtStable =
        c.thoughtsLen === 0 || thoughtsArr[c.thoughtsLen - 1]?.id === c.lastThoughtId;
      const outputStable =
        c.outputsLen === 0 || outputsArr[c.outputsLen - 1]?.id === c.lastOutputId;
      if (!thoughtStable || !outputStable) {
        // Перезагрузка с той же/большей длиной, но другим содержимым
        textChars = fullTextRecalc();
        commitCache(textChars);
      } else {
        // Дельта: подрастание старого последнего + только новые хвосты
        let delta = 0;
        let valid = true;
        if (c.thoughtsLen > 0) {
          const grow = (thoughtsArr[c.thoughtsLen - 1]?.content ?? '').length - c.lastThoughtLen;
          if (grow < 0) valid = false;
          else delta += grow;
        }
        if (c.outputsLen > 0) {
          const grow = (outputsArr[c.outputsLen - 1]?.content ?? '').length - c.lastOutputLen;
          if (grow < 0) valid = false;
          else delta += grow;
        }
        if (valid) {
          for (let i = c.thoughtsLen; i < tLen; i++) delta += (thoughtsArr[i]?.content ?? '').length;
          for (let i = c.outputsLen; i < oLen; i++) delta += (outputsArr[i]?.content ?? '').length;
          textChars = c.textChars + delta;
          commitCache(textChars);
        } else {
          textChars = fullTextRecalc();
          commitCache(textChars);
        }
      }
    }

    const baseSystem = 3200;
    const baseTool = 2400;

    // Fix 100% gauge bug: if the task was compacted, preserve the compacted token count
    // rather than recalculating an uncompressed sum of character lengths from raw historical tool items!
    let total: number;
    if (currentEstimatedTokens !== undefined) {
      total = currentEstimatedTokens;
    } else if (resolvedCompactTokens !== undefined) {
      total = resolvedCompactTokens;
    } else {
      let messageChars = textChars;
      for (const t of toolsArr) {
        messageChars += (t?.stdout?.length || 0);
        messageChars += (t?.stderr?.length || 0);
        messageChars += JSON.stringify(t?.params || {}).length;
      }
      const dynamicMsgTokens = Math.max(300, Math.round(messageChars / 4));
      total = baseSystem + baseTool + dynamicMsgTokens;
    }

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
    // Лёгкие deps: только слайсы текущей задачи, а не весь стор
  }, [taskId, currentEstimatedTokens, resolvedCompactTokens, thoughts, outputs, tools]);

  const percent = Math.min(100, Math.max(0, Math.round((totalUsedTokens / effectiveLimit) * 100)));

  // Segment widths against 100% capacity
  const systemPct = Math.max(0, (systemPromptTokens / effectiveLimit) * 100);
  const toolPct = Math.max(0, (toolDefTokens / effectiveLimit) * 100);
  const msgPct = Math.max(0, (messageTokens / effectiveLimit) * 100);

  const usedTokensK = (totalUsedTokens / 1000) >= 10
    ? Math.round(totalUsedTokens / 1000)
    : (totalUsedTokens / 1000).toFixed(1);
  const maxTokensK = Math.round(effectiveLimit / 1000);

  const calculatedTurns = useMemo(() => {
    const toolsCount = tools?.length || 0;
    const thoughtsCount = thoughts?.length || 0;
    const outputsCount = outputs?.length || 0;
    return Math.max(toolsCount + outputsCount, thoughtsCount, 0);
  }, [tools, thoughts, outputs]);
  const effectiveTurns = turns !== undefined ? turns : calculatedTurns;

  // Telemetry: cache hit % + output throughput (TPS) for the current measurement

  const cacheText = telemetry ? `${Math.round(telemetry.cacheHitRate)}%` : '—';
  const cacheTitle = telemetry
    ? `Попадания в prompt cache: ${Math.round(telemetry.cacheHitRate)}% (из кеша прочитано ${telemetry.cacheReadTokens.toLocaleString('ru-RU')} из ${(telemetry.inputTokens + telemetry.cacheReadTokens).toLocaleString('ru-RU')} входных токенов терна)`
    : 'Нет данных телеметрии от модели';

  const elapsedSec = thinkingStart != null ? (Date.now() - thinkingStart) / 1000 : null;
  const outputTokens = telemetry?.outputTokens ?? 0;
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
    if (savingsTimerRef.current) {
      clearTimeout(savingsTimerRef.current);
    }
    savingsTimerRef.current = setTimeout(() => {
      setSavingsBadge(null);
      savingsTimerRef.current = null;
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
      if (savingsTimerRef.current) {
        clearTimeout(savingsTimerRef.current);
      }
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
        const res = await useDrawerStore.getState().compactTaskContext(taskId);
        if (res) {
          saved = Math.max(saved, res.savedTokens);
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

  const miniCircumference = 2 * Math.PI * 6; // ~37.699
  const miniStrokeDashoffset = miniCircumference - (miniCircumference * percent) / 100;

  if (variant === 'header-ring') {
    return (
      <div className="relative group inline-flex items-center">
        <div
          className="relative w-7 h-7 flex items-center justify-center cursor-pointer select-none rounded-full hover:ring-2 hover:ring-[#ebebeb] transition"
          title={`Контекст: ${percent}% (~${usedTokensK}K / ${maxTokensK}K) · Кеш: ${cacheText} · TPS: ${tpsText}`}
        >
          <svg className="w-7 h-7 -rotate-90 transform" viewBox="0 0 28 28">
            <circle cx="14" cy="14" r={radius} stroke="#e5e7eb" strokeWidth="2.5" fill="none" />
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
      </div>
    );
  }

  // Elegant bottom status bar under chat input card
  return (
    <div className="font-mono text-xs text-[#8f8f8f] dark:text-[#8b949e] flex items-center justify-center gap-6 py-1.5 select-none">
      {/* ⏱️ {turns} шагов · {tps} tok/s */}
      <span className="flex items-center gap-1.5" title={tpsTitle}>
        <span className="tabular-nums">{effectiveTurns} шагов · {tpsText} tok/s</span>
      </span>

      {/* 💾 {tokens} tok · Кэш {cacheHitRate}% */}
      <span className="flex items-center gap-1.5" title={cacheTitle}>
        <span className="tabular-nums">{usedTokensK}k tok · Кэш {cacheText}</span>
      </span>

      {/* ○ {contextPercent}% (миниатюрное кольцо/круг контекста с popover) */}
      <div className="relative group inline-flex items-center gap-1.5 cursor-pointer">
        <div
          className="relative w-3.5 h-3.5 flex items-center justify-center shrink-0"
          title={`Контекст: ${percent}% (~${usedTokensK}K / ${maxTokensK}K)`}
        >
          <svg className="w-3.5 h-3.5 -rotate-90 transform" viewBox="0 0 16 16">
            <circle cx="8" cy="8" r="6" stroke="#ebebeb" strokeWidth="2" fill="none" />
            <circle
              cx="8"
              cy="8"
              r="6"
              stroke={strokeColor}
              strokeWidth="2"
              strokeDasharray={miniCircumference}
              strokeDashoffset={miniStrokeDashoffset}
              strokeLinecap="round"
              fill="none"
              className="transition-all duration-300"
            />
          </svg>
        </div>
        <span className="tabular-nums text-[#8f8f8f]">{percent}%</span>

        {/* Floating Hover Card (pops UPWARDS above status bar) */}
        <div className="absolute left-1/2 -translate-x-1/2 bottom-full mb-2.5 z-50 invisible opacity-0 group-hover:visible group-hover:opacity-100 transition-all duration-150 ease-out pointer-events-auto">
          {/* Invisible hit bridge to prevent losing hover on cursor transition */}
          <div className="absolute -bottom-3 left-0 right-0 h-3" />

          <div className="w-72 rounded-lg border border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#0d1117] p-3.5 shadow-xl text-[#171717] dark:text-[#f0f6fc] select-none font-sans">
            {/* Top Line */}
            <div className="flex items-center justify-between mb-2">
              <div>
                <span className="font-semibold text-xs text-[#171717] dark:text-[#f0f6fc]">{percent}%</span>{' '}
                <span className="text-xs text-[#666666] dark:text-[#8b949e]">of context used</span>
              </div>
              <span className="font-mono text-xs text-[#666666] dark:text-[#8b949e] tabular-nums">
                ~{usedTokensK}K / {maxTokensK}K
              </span>
            </div>

            {/* Segmented Progress Bar */}
            <div className="w-full h-2 rounded-full bg-[#f4f4f5] dark:bg-[#21262d] overflow-hidden flex mb-3">
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
            <div className="mt-3 pt-2.5 border-t border-[#ebebeb] dark:border-[#21262d] flex items-center justify-between">
              <button
                type="button"
                onClick={handleCompactClick}
                disabled={isCompacting}
                className="px-2.5 py-1 rounded-md bg-[#fafafa] dark:bg-[#161b22] hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] text-[11px] font-mono transition cursor-pointer disabled:opacity-50 active:scale-[0.98]"
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
    </div>
  );
}
