import { useState, useEffect, useCallback, useRef } from 'react';
import { useDrawerStore } from '../../../stores/drawerStore';
import { TaskTelemetrySummary } from '../../../stores/drawer/types';
import { useBoardStore } from '../../../stores/boardStore';
import { useCortexStore } from '../../../stores/cortexStore';
import { listenToEvent } from '../../../services/tauriEventGateway';
import { t, useLanguageStore } from '../../../stores/languageStore';
import { UseContextGaugeMetricsOptions } from './types';

export function usePersonaContextLimit(
  taskId: string,
  persona?: string,
  contextWindow?: number
): number | null {
  const [resolvedPersonaLimit, setResolvedPersonaLimit] = useState<number | null>(null);

  useEffect(() => {
    let cancelled = false;
    import('@tauri-apps/api/core')
      .then(({ invoke }) => invoke<number>('get_task_context_window', { taskId, persona }))
      .then((cw) => {
        if (!cancelled && typeof cw === 'number' && cw > 0) {
          const pInfo = useCortexStore
            .getState()
            .agentPersonas?.find((p) => p.role.toLowerCase() === (persona || '').toLowerCase());
          setResolvedPersonaLimit(
            pInfo?.contextWindow ?? (contextWindow && cw === 128000 ? contextWindow : cw)
          );
        }
      })
      .catch((err) => {
        console.debug('[useGaugeEffects] get_agent_context_window skipped:', err);
        const role =
          persona ||
          useBoardStore
            .getState()
            .columns.find(
              (c) =>
                c.id === taskId ||
                c.id === useBoardStore.getState().tasks.find((t) => t.id === taskId)?.stageId
            )?.persona ||
          'general';
        const pInfo = useCortexStore
          .getState()
          .agentPersonas?.find((p) => p.role.toLowerCase() === role.toLowerCase());
        if (!cancelled && (pInfo?.contextWindow || contextWindow)) {
          setResolvedPersonaLimit(pInfo?.contextWindow ?? contextWindow ?? null);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [taskId, persona, contextWindow]);

  return resolvedPersonaLimit;
}

export function useTelemetrySummarySync(
  taskId: string,
  setTaskTelemetrySummary: ((taskId: string, summary: TaskTelemetrySummary) => void) | undefined
): void {
  const refreshTelemetrySummary = useCallback(async () => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const summary = await invoke<TaskTelemetrySummary>('get_task_telemetry_summary', {
        taskId,
      });
      if (summary && typeof summary.totalTokens === 'number') {
        setTaskTelemetrySummary?.(taskId, summary);
      }
    } catch (err) {
      // Web fallback: cumulative stays unset, UI falls back to last-turn snapshot.
      console.debug('[useGaugeEffects] get_task_telemetry_summary skipped:', err);
    }
  }, [taskId, setTaskTelemetrySummary]);

  useEffect(() => {
    void refreshTelemetrySummary();
  }, [refreshTelemetrySummary]);

  useEffect(() => {
    let isDisposed = false;
    let unlisten: (() => void) | undefined;
    let debounce: ReturnType<typeof setTimeout> | null = null;
    const setupListener = async () => {
      try {
        if (isDisposed) return;
        const u = await listenToEvent('agent:telemetry', (payload) => {
          if (payload?.taskId && payload.taskId !== taskId) return;
          if (debounce) clearTimeout(debounce);
          debounce = setTimeout(() => {
            void refreshTelemetrySummary();
          }, 50);
        });
        if (isDisposed) u();
        else unlisten = u;
      } catch (err) {
        console.debug('[useGaugeEffects] agent:telemetry listener skipped:', err);
      }
    };
    setupListener();
    return () => {
      isDisposed = true;
      if (unlisten) unlisten();
      if (debounce) clearTimeout(debounce);
    };
  }, [taskId, refreshTelemetrySummary]);
}

export function useCompactionActions(params: {
  taskId: string;
  onCompact?: UseContextGaugeMetricsOptions['onCompact'];
  setTaskTokens: ((taskId: string, tokens: number) => void) | undefined;
  evaporateClientToolLogs:
    | ((taskId: string) => { evaporatedCount: number; savedTokens: number })
    | undefined;
}) {
  const { taskId, onCompact, setTaskTokens, evaporateClientToolLogs } = params;
  const [isCompacting, setIsCompacting] = useState(false);
  const [savingsBadge, setSavingsBadge] = useState<string | null>(null);
  const savingsTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const triggerSavingsNotice = useCallback((savedCount: number) => {
    const locale = useLanguageStore.getState().language === 'ru' ? 'ru-RU' : 'en-US';
    setSavingsBadge(t('gauge.compactedTokens', { count: savedCount.toLocaleString(locale) }));
    if (savingsTimerRef.current) clearTimeout(savingsTimerRef.current);
    savingsTimerRef.current = setTimeout(() => {
      setSavingsBadge(null);
      savingsTimerRef.current = null;
    }, 3000);
  }, []);

  useEffect(() => {
    let isDisposed = false;
    let unlisten: (() => void) | undefined;
    const setupListener = async () => {
      try {
        if (isDisposed) return;
        const u = await listenToEvent('agent:compacted', (payload) => {
          if (!payload) return;
          if (payload.taskId && payload.taskId !== taskId) return;
          const after = payload.tokens_after ?? payload.tokensAfter;
          const saved =
            payload.saved_tokens ?? payload.estimatedSavedTokens ?? payload.savedTokens ?? 0;
          if (typeof after === 'number' && Number.isFinite(after)) {
            setTaskTokens?.(taskId, after);
          }
          if (saved > 0) triggerSavingsNotice(saved);
        });
        if (isDisposed) u();
        else unlisten = u;
      } catch (err) {
        console.debug('[useGaugeEffects] agent:compacted listener fallback:', err);
      }
    };
    setupListener();
    return () => {
      isDisposed = true;
      if (unlisten) unlisten();
      if (savingsTimerRef.current) clearTimeout(savingsTimerRef.current);
    };
  }, [taskId, setTaskTokens, triggerSavingsNotice]);

  const handleCompactClick = async () => {
    if (isCompacting) return;
    setIsCompacting(true);
    let saved = 0;
    if (evaporateClientToolLogs) {
      saved = evaporateClientToolLogs(taskId).savedTokens;
    }
    if (onCompact) {
      try {
        const res = await onCompact();
        if (res) saved = Math.max(saved, res.savedTokens);
      } catch (err) {
        console.warn('[useGaugeEffects] onCompact failed:', err);
      }
    } else {
      try {
        const res = await useDrawerStore.getState().compactTaskContext(taskId);
        if (res) saved = Math.max(saved, res.savedTokens);
      } catch (err) {
        console.warn('[useGaugeEffects] compactTaskContext failed:', err);
      }
    }
    setIsCompacting(false);
    if (saved > 0) {
      triggerSavingsNotice(saved);
    }
  };

  return { isCompacting, savingsBadge, handleCompactClick };
}
