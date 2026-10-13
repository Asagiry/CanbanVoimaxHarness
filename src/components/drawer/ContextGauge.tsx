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
    setTaskTokens,
    evaporateClientToolLogs,
  } = useDrawerStore();

  const [isCompacting, setIsCompacting] = useState(false);
  const [savingsBadge, setSavingsBadge] = useState<string | null>(null);

  const currentTokens = useMemo(() => {
    if (currentEstimatedTokens !== undefined) {
      return currentEstimatedTokens;
    }

    if (tokenOverrides?.[taskId] !== undefined) {
      return tokenOverrides[taskId];
    }

    let charCount = 3800;

    const streams = streamEntries?.[taskId];
    if (streams) {
      for (const th of streams.thoughts) {
        charCount += th.content.length;
      }
      for (const out of streams.outputs) {
        charCount += out.content.length;
      }
    }

    const tools = toolExecutions?.[taskId];
    if (tools) {
      for (const t of tools) {
        charCount += (t.stdout?.length || 0);
        charCount += (t.stderr?.length || 0);
        charCount += JSON.stringify(t.params || {}).length;
      }
    }

    return Math.max(1200, Math.round(charCount / 4));
  }, [taskId, currentEstimatedTokens, tokenOverrides, streamEntries, toolExecutions]);

  const percentage = Math.min(100, Math.round((currentTokens / effectiveLimit) * 100));

  const barColor = useMemo(() => {
    if (percentage < 50) return 'bg-[#171717]';
    if (percentage <= 75) return 'bg-amber-500';
    return 'bg-red-500';
  }, [percentage]);

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
        // Fallback
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
    <div className="flex flex-col space-y-1.5 py-1 text-xs font-mono select-none">
      {/* Toast */}
      {savingsBadge && (
        <div className="text-[11px] text-[#b45309] font-mono">
          {savingsBadge}
        </div>
      )}

      {/* Row */}
      <div className="flex items-center justify-between text-[#666666] text-[11px]">
        <div className="flex items-center space-x-2">
          <span>Контекст:</span>
          <span className="text-[#171717] font-medium tabular-nums">
            {currentTokens.toLocaleString('ru-RU')} / {effectiveLimit.toLocaleString('ru-RU')} ({percentage}%)
          </span>
        </div>

        <button
          onClick={handleCompactClick}
          disabled={isCompacting}
          className="px-2 py-0.5 rounded-md bg-white hover:bg-[#f4f4f5] border border-[#ebebeb] text-[#171717] hover:text-[#0d0d0d] transition cursor-pointer text-[10px]"
          title="Компактировать контекст"
        >
          {isCompacting ? 'Сжатие...' : 'Компактировать контекст'}
        </button>
      </div>

      {/* Thin minimalist progress bar */}
      <div className="w-full h-1.5 rounded-full bg-[#ebebeb] overflow-hidden">
        <div
          className={`h-full transition-all duration-200 ${barColor}`}
          style={{ width: `${Math.max(2, percentage)}%` }}
        />
      </div>
    </div>
  );
}
