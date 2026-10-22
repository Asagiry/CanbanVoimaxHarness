import { useState, useRef, useEffect } from 'react';
import { ToolExecution } from '../../types';
import { useDrawerStore } from '../../stores/drawerStore';
import { useBoardStore } from '../../stores/boardStore';
import { useCortexStore } from '../../stores/cortexStore';
import { getRoleBadgeStyle } from '../../lib/agentPalette';
import { formatSubagentDisplayTitle, formatSubagentRoleDisplay, getSubagentOrderMap } from './SubagentsSidebar';
import { formatDurationCompact } from './ThinkingCard';
import { FileExtensionBadge, getLanguageBadgeConfig } from '../common/BrandFileIcons';
import {
  ChevronRight,
  Copy,
  Check,
  X,
  Wrench,
  FileCode2,
  FileText,
  Minimize2,
  Layers,
  Zap,
  Users,
  FolderDown,
} from 'lucide-react';

interface ToolExecutionCardProps {
  tool: ToolExecution;
}

const getLanguageBadge = getLanguageBadgeConfig;

function CompactContextExecutionCard({ tool }: { tool: ToolExecution }) {
  const [isExpanded, setIsExpanded] = useState(false);
  const [copied, setCopied] = useState(false);
  const copiedTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [activeStep, setActiveStep] = useState(1);
  const [progressPct, setProgressPct] = useState(25);

  const isRunning = tool.status === 'running';
  const isSuccess = tool.status === 'success';
  const isFailed =
    tool.status === 'failed' ||
    (tool.status as string) === 'error' ||
    Boolean(tool.stderr && tool.stderr.trim().length > 0) ||
    (tool.exitCode !== undefined && tool.exitCode !== null && tool.exitCode !== 0);

  const params = (tool.params && typeof tool.params === 'object' ? tool.params : {}) as Record<string, unknown>;
  const savedTokens = typeof params.savedTokens === 'number'
    ? params.savedTokens
    : typeof (tool as any).savedTokens === 'number'
    ? (tool as any).savedTokens
    : 0;
  const formattedSaved = savedTokens > 0 ? savedTokens.toLocaleString('ru-RU') : '';

  // Animated stages progression when running:
  // 1. Анализ диалога → 2. Испарение логов тулов → 3. Сборка выжимки
  useEffect(() => {
    if (!isRunning) {
      if (isSuccess) {
        setActiveStep(3);
        setProgressPct(100);
      }
      return;
    }

    setActiveStep(1);
    setProgressPct(25);

    const t1 = setTimeout(() => {
      setActiveStep(2);
      setProgressPct(62);
    }, 1100);

    const t2 = setTimeout(() => {
      setActiveStep(3);
      setProgressPct(89);
    }, 2400);

    return () => {
      clearTimeout(t1);
      clearTimeout(t2);
    };
  }, [isRunning, isSuccess]);

  useEffect(() => {
    return () => {
      if (copiedTimeoutRef.current) {
        clearTimeout(copiedTimeoutRef.current);
      }
    };
  }, []);

  const handleCopy = (e: React.MouseEvent) => {
    e.stopPropagation();
    const content = tool.stdout || tool.stderr || 'compact_context';
    navigator.clipboard.writeText(content);
    setCopied(true);
    if (copiedTimeoutRef.current) clearTimeout(copiedTimeoutRef.current);
    copiedTimeoutRef.current = setTimeout(() => {
      setCopied(false);
      copiedTimeoutRef.current = null;
    }, 2000);
  };

  const stages = [
    { num: 1, label: '1. Анализ диалога' },
    { num: 2, label: '2. Испарение логов тулов' },
    { num: 3, label: '3. Сборка выжимки' },
  ];

  const currentStageLabel =
    activeStep === 1
      ? 'Анализ диалога...'
      : activeStep === 2
      ? 'Испарение логов тулов...'
      : 'Сборка выжимки...';

  const durationText = formatDurationCompact(tool.durationMs);

  return (
    <div
      className={`group rounded-md border text-xs shadow-2xs hover:shadow-xs overflow-hidden my-1 animate-silk-entrance transition-colors bg-white dark:bg-[#161b22] ${
        isRunning
          ? 'border-amber-500/40 dark:border-amber-500/30 bg-amber-500/[0.02]'
          : 'border-[#ebebeb] dark:border-[#21262d]'
      }`}
    >
      {/* Header Strip */}
      <div
        onClick={() => setIsExpanded(!isExpanded)}
        className={`flex items-center justify-between px-2.5 py-1.5 bg-[#f8f8f9] dark:bg-[#161b22] hover:bg-[#f2f2f3] dark:hover:bg-[#1c2128] transition-colors cursor-pointer select-none gap-2 text-xs min-h-[30px] ${
          isExpanded ? 'border-b border-[#ebebeb] dark:border-[#21262d]' : ''
        }`}
      >
        <div className="flex items-center space-x-2 min-w-0 flex-1 overflow-hidden">
          <button
            type="button"
            className="p-0.5 text-[#8f8f8f] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] rounded shrink-0 transition cursor-pointer"
            onClick={(e) => {
              e.stopPropagation();
              setIsExpanded(!isExpanded);
            }}
            aria-label={isExpanded ? 'Свернуть подробности' : 'Развернуть подробности'}
          >
            <ChevronRight
              className={`h-3 w-3 transition-transform duration-150 ${
                isExpanded ? 'rotate-90 text-[#171717] dark:text-[#f0f6fc]' : ''
              }`}
            />
          </button>

          {/* Icon */}
          <div className="p-1 rounded-sm bg-indigo-50 dark:bg-indigo-950/40 text-indigo-600 dark:text-indigo-400 border border-indigo-200 dark:border-indigo-800/60 shrink-0">
            <Minimize2 className="h-3 w-3" />
          </div>

          {/* Badge */}
          <span className="px-1.5 py-0.5 rounded text-[10px] font-mono font-semibold uppercase tracking-wider shrink-0 select-none bg-indigo-50 dark:bg-indigo-950/40 text-indigo-700 dark:text-indigo-300 border border-indigo-200 dark:border-indigo-800/60">
            Оптимизация
          </span>

          {/* Title */}
          <span className="font-semibold text-[#171717] dark:text-[#f0f6fc] shrink-0 font-sans">
            Оптимизация контекста
          </span>

          {/* Saved tokens badge when finished */}
          {isSuccess && formattedSaved && (
            <span className="font-mono tabular-nums text-[#15803d] dark:text-[#7ee787] text-[11px] bg-[#ecfdf5] dark:bg-[#12281e] px-1.5 py-0.2 rounded border border-[#a7f3d0] dark:border-[#238636] font-medium shrink-0">
              Освобождено -{formattedSaved} токенов
            </span>
          )}

          {/* Duration */}
          {durationText && (
            <>
              <span className="text-[#a1a1aa] dark:text-[#6e7681] select-none text-[10px]">•</span>
              <span className="font-mono text-[#71717a] dark:text-[#8b949e] shrink-0 tabular-nums text-[11px]">
                {durationText}
              </span>
            </>
          )}
        </div>

        {/* Right side status indicator & copy */}
        <div className="flex items-center space-x-2 shrink-0">
          <button
            type="button"
            onClick={handleCopy}
            className={`p-1 rounded text-[#8f8f8f] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-white dark:hover:bg-[#161b22] transition cursor-pointer ${
              isExpanded ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'
            }`}
            aria-label="Скопировать вывод"
          >
            {copied ? (
              <Check className="h-3 w-3 text-emerald-600 dark:text-emerald-400" />
            ) : (
              <Copy className="h-3 w-3" />
            )}
          </button>

          {isRunning ? (
            <div className="flex items-center space-x-1.5">
              <span className="inline-flex rounded-full h-2 w-2 bg-indigo-500 animate-pulse shrink-0" />
              <span className="text-[11px] font-mono text-indigo-600 dark:text-indigo-400">В процессе</span>
            </div>
          ) : isSuccess ? (
            <div className="flex items-center space-x-1.5">
              <Check className="h-3.5 w-3.5 text-emerald-600 dark:text-emerald-400 stroke-[2.5]" />
              <span className="text-[11px] font-mono font-medium text-emerald-600 dark:text-emerald-400">Готово</span>
            </div>
          ) : (
            <div className="flex items-center space-x-1.5">
              <X className="h-3.5 w-3.5 text-rose-600 dark:text-rose-400 stroke-[2.5]" />
              <span className="text-[11px] font-mono font-medium text-rose-600 dark:text-rose-400">Ошибка</span>
            </div>
          )}
        </div>
      </div>

      {/* Running State: Animated Progress Bar & Stages Stepper */}
      {isRunning && (
        <div className="px-3 pb-3 pt-1 border-t border-[#ebebeb]/70 dark:border-[#21262d]/70 space-y-2.5">
          {/* Progress bar with percentage */}
          <div className="space-y-1">
            <div className="flex items-center justify-between text-[11px] font-mono">
              <span className="text-[#52525b] dark:text-[#8b949e] flex items-center gap-1.5">
                <span className="inline-block h-1.5 w-1.5 rounded-full bg-indigo-500 animate-ping" />
                {currentStageLabel}
              </span>
              <span className="text-indigo-600 dark:text-indigo-400 font-semibold tabular-nums">
                {progressPct}%
              </span>
            </div>
            <div className="h-1.5 w-full bg-[#f4f4f5] dark:bg-[#21262d] rounded-full overflow-hidden">
              <div
                className="h-full bg-gradient-to-r from-indigo-500 via-purple-500 to-indigo-400 transition-all duration-300 ease-out rounded-full"
                style={{ width: `${progressPct}%` }}
              />
            </div>
          </div>

          {/* 3 Stages pill indicator */}
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-1.5 text-[10px] font-mono">
            {stages.map((st) => {
              const isPassed = isSuccess || activeStep > st.num;
              const isCurrent = isRunning && activeStep === st.num;
              return (
                <div
                  key={st.num}
                  className={`flex items-center space-x-1.5 px-2 py-1 rounded transition-colors ${
                    isCurrent
                      ? 'bg-indigo-50 dark:bg-indigo-950/60 text-indigo-700 dark:text-indigo-300 border border-indigo-200 dark:border-indigo-800/80 font-semibold'
                      : isPassed
                      ? 'bg-emerald-50/50 dark:bg-emerald-950/30 text-emerald-600 dark:text-emerald-400 font-medium border border-emerald-200/50 dark:border-emerald-800/40'
                      : 'text-[#8f8f8f] dark:text-[#6e7681] bg-[#f4f4f5]/60 dark:bg-[#161b22]/60 border border-transparent'
                  }`}
                >
                  {isPassed ? (
                    <Check className="h-3 w-3 shrink-0 stroke-[2.5] text-emerald-600 dark:text-emerald-400" />
                  ) : isCurrent ? (
                    <span className="h-1.5 w-1.5 rounded-full bg-indigo-500 animate-pulse shrink-0" />
                  ) : (
                    <span className="h-1.5 w-1.5 rounded-full bg-[#d4d4d8] dark:bg-[#30363d] shrink-0" />
                  )}
                  <span className="truncate">{st.label}</span>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* Completed Success State: Compact Stages Completed Strip */}
      {!isRunning && isSuccess && (
        <div className="px-3 py-1.5 border-t border-[#ebebeb]/60 dark:border-[#21262d]/60 flex items-center justify-between text-[11px] font-mono text-[#52525b] dark:text-[#8b949e]">
          <div className="flex items-center space-x-1.5">
            <Check className="h-3 w-3 text-emerald-600 dark:text-emerald-400 stroke-[2.5]" />
            <span>1. Анализ диалога → 2. Испарение логов тулов → 3. Сборка выжимки</span>
          </div>
          <span className="text-emerald-600 dark:text-emerald-400 font-medium tabular-nums">
            готово
          </span>
        </div>
      )}

      {/* Expanded Details */}
      {isExpanded && (
        <div className="bg-white dark:bg-[#0d1117] p-3 border-t border-[#ebebeb] dark:border-[#21262d] space-y-2 font-mono text-xs">
          {tool.stdout && (
            <div>
              <span className="text-[10px] text-[#666666] dark:text-[#8b949e] block mb-1">
                Результат оптимизации
              </span>
              <pre className="p-2.5 rounded-md bg-[#fafafa] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#21262d] text-[11px] text-[#171717] dark:text-[#f0f6fc] overflow-x-auto leading-relaxed max-h-48 whitespace-pre-wrap select-text">
                {tool.stdout}
              </pre>
            </div>
          )}

          {tool.stderr && (
            <div>
              <span className="text-[10px] text-[#b91c1c] dark:text-[#ff7b72] block mb-1">
                Ошибки оптимизации
              </span>
              <pre className="p-2.5 rounded-md bg-[#fef2f2] dark:bg-[#3b1219] border border-[#fecaca] dark:border-[#da3633] text-[11px] text-[#991b1b] dark:text-[#ff7b72] overflow-x-auto leading-relaxed max-h-48 whitespace-pre-wrap select-text">
                {tool.stderr}
              </pre>
            </div>
          )}

          <div className="flex items-center justify-between pt-1 border-t border-[#f4f4f5] dark:border-[#21262d] text-[10px] text-[#8f8f8f] dark:text-[#8b949e]">
            <span>{isFailed ? `failed (код ${tool.exitCode ?? 1})` : `Этапы завершены`}</span>
            {durationText && <span>Длительность: {durationText}</span>}
          </div>
        </div>
      )}
    </div>
  );
}

export default function ToolExecutionCard({ tool }: ToolExecutionCardProps) {
  const [isExpanded, setIsExpanded] = useState(false);
  const [copied, setCopied] = useState(false);
  const copiedTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const openSubagent = useDrawerStore((s) => s.openSubagent);
  const agentPersonas = useCortexStore((s) => s.agentPersonas);
  const activeTaskId = useDrawerStore((s) => s.activeTaskId);
  const storeSubagents = useDrawerStore((s) => s.subagents);
  const toolExecutions = useDrawerStore((s) => s.toolExecutions);
  const toolIdToSubagentId = useDrawerStore((s) => s.toolIdToSubagentId);
  const boardTasks = useBoardStore((s) => s.tasks);

  useEffect(() => {
    return () => {
      if (copiedTimeoutRef.current) {
        clearTimeout(copiedTimeoutRef.current);
      }
    };
  }, []);

  if (!tool) return null;

  const toolName: string = (tool as ToolExecution)?.toolName ?? 'tool';
  const params: Record<string, unknown> =
    (tool as ToolExecution)?.params && typeof (tool as ToolExecution).params === 'object'
      ? ((tool as ToolExecution).params as Record<string, unknown>)
      : {};

  // Subagent interactive card (subagent and subagent_bg)
  if (toolName === 'subagent' || toolName === 'subagent_bg') {
    const rawRole = (params?.role as string) || 'general';
    const title = (params?.title as string) || (params?.prompt as string)?.slice(0, 60) || 'Делегированная задача субагента';
    const prompt = (params?.prompt as string) || '';
    const isBg = toolName === 'subagent_bg' || (params?.is_bg === true) || (params?.isBg === true);

    // Extract subagent_id from params, or drawerStore toolIdToSubagentId map, or parsed JSON stdout (subagent_bg), or stdout regex (subagent)
    let subagentId = typeof params?.subagent_id === 'string' && params.subagent_id ? params.subagent_id : '';
    if (!subagentId) {
      const mapped = useDrawerStore.getState().toolIdToSubagentId[tool.id];
      if (mapped) subagentId = mapped;
    }
    if (!subagentId && tool.stdout) {
      try {
        const parsed = JSON.parse(tool.stdout);
        if (parsed.subagent_id) subagentId = parsed.subagent_id;
      } catch {}
      if (!subagentId) {
        const match = tool.stdout.match(/\[Subagent ID:\s*([^\s\]]+)\]/);
        if (match && match[1]) subagentId = match[1];
      }
    }
    if (!subagentId) {
      subagentId = tool.id;
    }

    const persona = agentPersonas.find(
      (p) => p.role.toLowerCase() === rawRole.toLowerCase()
    );
    const roleBadgeStyle = getRoleBadgeStyle(rawRole, persona?.color);
    const displayTitle = formatSubagentDisplayTitle(title);

    const subagentNumber = (() => {
      if (!activeTaskId) return 1;
      const map = getSubagentOrderMap(activeTaskId, storeSubagents, toolExecutions, toolIdToSubagentId, boardTasks);
      return map.get(subagentId) ?? map.get(tool.id) ?? (toolIdToSubagentId[subagentId] ? map.get(toolIdToSubagentId[subagentId]) : null) ?? 1;
    })();

    // Find subagent in storeSubagents
    const list = storeSubagents[activeTaskId || ''] || [];
    const sub = list.find(
      (s) =>
        s.id === subagentId ||
        s.toolCallId === tool.id ||
        (toolIdToSubagentId[tool.id] && s.id === toolIdToSubagentId[tool.id]) ||
        (s.toolCallId && s.toolCallId === subagentId)
    );

    // Compute effective status for sync vs background subagent
    let effectiveStatus: 'running' | 'success' | 'aborted' | 'error';
    if (isBg) {
      if (sub) {
        if (sub.status === 'running') {
          effectiveStatus = 'running';
        } else if (sub.status === 'finished' || (sub.status as string) === 'done') {
          effectiveStatus = 'success';
        } else if (sub.status === 'stopped' || (sub.status as string) === 'aborted') {
          effectiveStatus = 'aborted';
        } else if (sub.status === 'error' || (sub.status as string) === 'failed') {
          effectiveStatus = 'error';
        } else {
          effectiveStatus = 'running';
        }
      } else {
        if (tool.status === 'failed' || (tool.status as string) === 'error') {
          effectiveStatus = 'error';
        } else {
          // Tool spawn finished, subagent is running in background until subagent:finished
          effectiveStatus = 'running';
        }
      }
    } else {
      // Sync subagent
      if (tool.status === 'running') {
        effectiveStatus = 'running';
      } else if (tool.status === 'failed' || (tool.status as string) === 'error') {
        effectiveStatus = 'error';
      } else if (sub && (sub.status === 'stopped' || (sub.status as string) === 'aborted')) {
        effectiveStatus = 'aborted';
      } else if (sub && (sub.status === 'error' || (sub.status as string) === 'failed')) {
        effectiveStatus = 'error';
      } else if (sub && sub.status === 'running') {
        effectiveStatus = 'running';
      } else if (tool.status === 'success') {
        effectiveStatus = 'success';
      } else {
        effectiveStatus = 'error';
      }
    }

    const isRunning = effectiveStatus === 'running';
    const isSuccess = effectiveStatus === 'success';
    const isAborted = effectiveStatus === 'aborted';

    const subDurationText = (() => {
      if (isRunning) return null;
      if (isBg) {
        if (sub?.durationMs && (!sub.isBg || sub.durationMs >= 1000)) {
          return formatDurationCompact(sub.durationMs);
        }
        if (sub?.createdAt && sub?.finishedAt) {
          return formatDurationCompact(sub.finishedAt - sub.createdAt);
        }
        return null;
      }
      return formatDurationCompact(tool.durationMs);
    })();

    return (
      <div
        onClick={() => openSubagent(subagentId, title, rawRole, prompt)}
        className="group rounded-lg border border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#161b22] hover:bg-[#f9f9f9] dark:hover:bg-[#1c2128] hover:border-[#171717]/40 dark:hover:border-[#30363d] p-3 shadow-xs transition-all duration-150 cursor-pointer select-none my-2"
      >
        <div className="flex items-center justify-between gap-2.5">
          <div className="flex items-center space-x-2 min-w-0 flex-1 overflow-hidden">
            {isBg ? (
              <div className="relative shrink-0 flex items-center justify-center">
                <Layers className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                <span className="absolute -bottom-0.5 -right-0.5 h-1.5 w-1.5 rounded-full bg-purple-500 ring-1 ring-white dark:ring-[#161b22]" />
              </div>
            ) : (
              <div className="relative shrink-0 flex items-center justify-center">
                <Zap className="h-4 w-4 text-sky-600 dark:text-sky-400 fill-sky-500/20" />
                <span className="absolute -bottom-0.5 -right-0.5 h-1.5 w-1.5 rounded-full bg-sky-500 ring-1 ring-white dark:ring-[#161b22]" />
              </div>
            )}

            <span
              style={roleBadgeStyle}
              className="px-2 py-0.5 rounded text-[11px] font-mono font-bold tracking-wider border shrink-0 select-none uppercase"
            >
              {formatSubagentRoleDisplay(rawRole, subagentNumber || 1)}
            </span>

            <span className="font-sans font-medium text-xs text-[#0d0d0d] dark:text-[#f0f6fc] group-hover:text-[#171717] dark:group-hover:text-white transition-colors truncate">
              {displayTitle}
            </span>
          </div>

          <div className="flex items-center space-x-2 shrink-0">
            {isRunning ? (
              <div className="flex items-center space-x-1.5">
                <span className="relative flex h-2 w-2 shrink-0">
                  <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75" />
                  <span className="relative inline-flex rounded-full h-2 w-2 bg-emerald-500" />
                </span>
                <span className="text-[11px] font-mono text-[#15803d] dark:text-[#7ee787]">Выполняется</span>
              </div>
            ) : isSuccess ? (
              <div className="flex items-center space-x-1.5">
                <Check className="h-3.5 w-3.5 text-[#15803d] dark:text-[#7ee787] stroke-[2.5]" />
                <span className="text-[11px] font-mono font-medium text-[#15803d] dark:text-[#7ee787]">Готово</span>
              </div>
            ) : isAborted ? (
              <div className="flex items-center space-x-1.5">
                <X className="h-3.5 w-3.5 text-[#d97706] dark:text-[#f59e0b] stroke-[2.5]" />
                <span className="text-[11px] font-mono font-medium text-[#d97706] dark:text-[#f59e0b]">Прервано</span>
              </div>
            ) : (
              <div className="flex items-center space-x-1.5">
                <X className="h-3.5 w-3.5 text-[#dc2626] dark:text-[#ff7b72] stroke-[2.5]" />
                <span className="text-[11px] font-mono font-medium text-[#dc2626] dark:text-[#ff7b72]">Ошибка</span>
              </div>
            )}

            {subDurationText && (
              <span className="font-mono text-[#71717a] dark:text-[#8b949e] shrink-0 tabular-nums text-[11px]">
                {subDurationText}
              </span>
            )}

            <ChevronRight className="h-4 w-4 text-[#8f8f8f] group-hover:text-[#171717] dark:group-hover:text-white group-hover:translate-x-0.5 transition-all shrink-0" />
          </div>
        </div>

        {/* Subagent Prompt Snippet if available */}
        {prompt && (
          <div className="mt-2 pt-2 border-t border-[#f4f4f5] dark:border-[#21262d] text-[11px] font-mono text-[#52525b] dark:text-[#8b949e] line-clamp-2 leading-relaxed">
            {prompt}
          </div>
        )}
      </div>
    );
  }

  // Collect subagent interactive card
  if (toolName === 'collect_subagent') {
    const rawIds: string[] = [];
    const extractIds = (val: unknown) => {
      if (!val) return;
      if (Array.isArray(val)) {
        for (const item of val) {
          if (typeof item === 'string' && item.trim()) rawIds.push(item.trim());
        }
      } else if (typeof val === 'string') {
        const trimmed = val.trim();
        if (!trimmed) return;
        if (trimmed.startsWith('[') && trimmed.endsWith(']')) {
          try {
            const parsed = JSON.parse(trimmed);
            if (Array.isArray(parsed)) {
              for (const item of parsed) {
                if (typeof item === 'string' && item.trim()) rawIds.push(item.trim());
              }
              return;
            }
          } catch {}
        }
        trimmed.split(',').forEach((s) => {
          const part = s.trim();
          if (part) rawIds.push(part);
        });
      }
    };

    extractIds(params?.subagent_ids);
    extractIds(params?.subagent_id);
    extractIds(params?.ids);

    const uniqueIds = Array.from(new Set(rawIds));

    const orderMap = activeTaskId
      ? getSubagentOrderMap(activeTaskId, storeSubagents, toolExecutions, toolIdToSubagentId, boardTasks)
      : new Map<string, number>();

    const formatIdLabel = (id: string, index: number): string => {
      let num: number | undefined = orderMap.get(id);
      if (num === undefined && toolIdToSubagentId[id]) {
        num = orderMap.get(toolIdToSubagentId[id]);
      }
      if (num === undefined && activeTaskId) {
        const list = storeSubagents[activeTaskId] || [];
        const found = list.find((s) => s.id === id || s.toolCallId === id);
        if (found) {
          num = orderMap.get(found.id) ?? (found.toolCallId ? orderMap.get(found.toolCallId) : undefined);
        }
      }
      if (num === undefined) {
        const match = id.match(/(?:call_|subagent_|sub-|_|#)(\d+)$/i);
        if (match) {
          num = parseInt(match[1], 10);
        }
      }
      if (num === undefined || isNaN(num)) {
        num = index + 1;
      }
      return `субагент #${num}`;
    };

    const displayBadgeText =
      uniqueIds.length > 0
        ? uniqueIds.map((id, idx) => formatIdLabel(id, idx)).join(', ')
        : 'все субагенты';

    const isRunning = tool.status === 'running';
    const isSuccess = tool.status === 'success';
    const firstSubagentId = uniqueIds[0];

    return (
      <div
        onClick={() => {
          if (uniqueIds.length === 1 && firstSubagentId) {
            openSubagent(firstSubagentId, formatIdLabel(firstSubagentId, 0), 'general', '');
          }
        }}
        className={`group rounded-lg border border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#161b22] hover:bg-[#f9f9f9] dark:hover:bg-[#1c2128] p-3 shadow-xs transition-all duration-150 select-none my-2 ${
          uniqueIds.length === 1
            ? 'cursor-pointer hover:border-[#171717]/40 dark:hover:border-[#30363d]'
            : 'cursor-default'
        }`}
      >
        <div className="flex items-center justify-between gap-2.5">
          <div className="flex items-center space-x-2 min-w-0 flex-1 overflow-hidden">
            <FolderDown className="h-4 w-4 text-sky-600 dark:text-sky-400 shrink-0 select-none" />

            <span className="px-1.5 py-0.5 rounded text-[10px] font-mono font-bold tracking-wider uppercase border border-sky-200 dark:border-sky-800/60 bg-sky-50 dark:bg-sky-950/40 text-sky-700 dark:text-sky-300 shrink-0">
              СБОР
            </span>

            <span className="font-mono text-xs font-semibold text-[#171717] dark:text-[#f0f6fc] shrink-0">
              collect_subagent
            </span>

            <span className="font-mono text-[11px] text-[#71717a] dark:text-[#8b949e] px-1.5 py-0.5 rounded bg-[#f4f4f5] dark:bg-[#0d1117] border border-[#ebebeb] dark:border-[#30363d] truncate">
              {displayBadgeText}
            </span>
          </div>

          <div className="flex items-center space-x-2 shrink-0">
            {isRunning ? (
              <div className="flex items-center space-x-1.5">
                <span className="relative flex h-2 w-2 shrink-0">
                  <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75" />
                  <span className="relative inline-flex rounded-full h-2 w-2 bg-emerald-500" />
                </span>
                <span className="text-[11px] font-mono text-[#15803d] dark:text-[#7ee787]">Ожидание отчета</span>
              </div>
            ) : isSuccess ? (
              <div className="flex items-center space-x-1.5">
                <Check className="h-3.5 w-3.5 text-[#15803d] dark:text-[#7ee787] stroke-[2.5]" />
                <span className="text-[11px] font-mono font-medium text-[#15803d] dark:text-[#7ee787]">Отчет получен</span>
              </div>
            ) : (
              <div className="flex items-center space-x-1.5">
                <X className="h-3.5 w-3.5 text-[#dc2626] dark:text-[#ff7b72] stroke-[2.5]" />
                <span className="text-[11px] font-mono font-medium text-[#dc2626] dark:text-[#ff7b72]">Ошибка сбора</span>
              </div>
            )}

            {uniqueIds.length === 1 && (
              <ChevronRight className="h-4 w-4 text-[#8f8f8f] group-hover:text-[#171717] dark:group-hover:text-white group-hover:translate-x-0.5 transition-all shrink-0" />
            )}
          </div>
        </div>

        {/* Report excerpt if available */}
        {tool.stdout && (
          <div className="mt-2 pt-2 border-t border-[#f4f4f5] dark:border-[#21262d] text-[11px] font-mono text-[#52525b] dark:text-[#8b949e] line-clamp-2 leading-relaxed">
            {tool.stdout}
          </div>
        )}
      </div>
    );
  }

  // Subagent list interactive card (subagent_list or list_subagents)
  if (toolName === 'subagent_list' || toolName === 'list_subagents') {
    const isRunning = tool.status === 'running';
    const isSuccess = tool.status === 'success';

    const orderMap = activeTaskId
      ? getSubagentOrderMap(activeTaskId, storeSubagents, toolExecutions, toolIdToSubagentId, boardTasks)
      : new Map<string, number>();

    // Parse discovered subagents from stdout or fallback to storeSubagents
    const discoveredList: Array<{
      id: string;
      number: number;
      title: string;
      role: string;
      status: string;
      durationMs?: number;
      isBg?: boolean;
    }> = [];

    if (tool.stdout) {
      try {
        const parsed = JSON.parse(tool.stdout);
        const items = Array.isArray(parsed)
          ? parsed
          : Array.isArray(parsed?.subagents)
          ? parsed.subagents
          : Array.isArray(parsed?.items)
          ? parsed.items
          : null;

        if (items) {
          items.forEach((item: any, idx: number) => {
            const sid = (item.id || item.subagent_id || `sub-${idx + 1}`) as string;
            const mappedNum = orderMap.get(sid) ?? orderMap.get(toolIdToSubagentId[sid]) ?? (idx + 1);
            discoveredList.push({
              id: sid,
              number: mappedNum,
              title: (item.title || item.name || item.prompt?.slice(0, 50) || `Субагент #${mappedNum}`) as string,
              role: (item.role || 'general') as string,
              status: (item.status || 'finished') as string,
              durationMs: typeof item.duration_ms === 'number' ? item.duration_ms : typeof item.durationMs === 'number' ? item.durationMs : undefined,
              isBg: Boolean(item.is_bg || item.isBg),
            });
          });
        }
      } catch {}
    }

    // If stdout did not yield items, fallback to current storeSubagents for active task
    if (discoveredList.length === 0 && activeTaskId) {
      const known = storeSubagents[activeTaskId] || [];
      known.forEach((sub, idx) => {
        const mappedNum = orderMap.get(sub.id) ?? orderMap.get(sub.toolCallId || '') ?? (idx + 1);
        discoveredList.push({
          id: sub.id,
          number: mappedNum,
          title: sub.title || `Субагент #${mappedNum}`,
          role: sub.role || 'general',
          status: sub.status,
          durationMs: sub.durationMs,
          isBg: sub.isBg,
        });
      });
    }

    const durationText = formatDurationCompact(tool.durationMs);

    return (
      <div className="rounded-lg border border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#161b22] p-3 shadow-xs my-2 select-none">
        <div className="flex items-center justify-between gap-2.5">
          <div className="flex items-center space-x-2 min-w-0 flex-1 overflow-hidden">
            <div className="relative shrink-0 flex items-center justify-center">
              <Users className="h-4 w-4 text-purple-600 dark:text-purple-400" />
            </div>

            <span className="px-1.5 py-0.5 rounded text-[10px] font-mono font-bold tracking-wider uppercase border border-purple-200 dark:border-purple-800/60 bg-purple-50 dark:bg-purple-950/40 text-purple-700 dark:text-purple-300 shrink-0">
              СПИСОК
            </span>

            <span className="font-mono text-xs font-semibold text-[#171717] dark:text-[#f0f6fc] shrink-0">
              subagent_list
            </span>

            <span className="font-mono text-[11px] text-[#71717a] dark:text-[#8b949e] px-1.5 py-0.5 rounded bg-[#f4f4f5] dark:bg-[#0d1117] border border-[#ebebeb] dark:border-[#30363d] truncate">
              всего: {discoveredList.length}
            </span>
          </div>

          <div className="flex items-center space-x-2 shrink-0">
            {isRunning ? (
              <div className="flex items-center space-x-1.5">
                <span className="relative flex h-2 w-2 shrink-0">
                  <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75" />
                  <span className="relative inline-flex rounded-full h-2 w-2 bg-emerald-500" />
                </span>
                <span className="text-[11px] font-mono text-[#15803d] dark:text-[#7ee787]">Поиск</span>
              </div>
            ) : isSuccess ? (
              <div className="flex items-center space-x-1.5">
                <Check className="h-3.5 w-3.5 text-[#15803d] dark:text-[#7ee787] stroke-[2.5]" />
                <span className="text-[11px] font-mono font-medium text-[#15803d] dark:text-[#7ee787]">
                  {discoveredList.length > 0 ? `Найдено: ${discoveredList.length}` : 'Список получен'}
                </span>
              </div>
            ) : (
              <div className="flex items-center space-x-1.5">
                <X className="h-3.5 w-3.5 text-[#dc2626] dark:text-[#ff7b72] stroke-[2.5]" />
                <span className="text-[11px] font-mono font-medium text-[#dc2626] dark:text-[#ff7b72]">Ошибка</span>
              </div>
            )}

            {durationText && (
              <span className="font-mono text-[#71717a] dark:text-[#8b949e] shrink-0 tabular-nums text-[11px]">
                {durationText}
              </span>
            )}
          </div>
        </div>

        {/* Subagent Compact Rows */}
        {discoveredList.length > 0 ? (
          <div className="mt-2.5 pt-2 border-t border-[#f4f4f5] dark:border-[#21262d] space-y-1.5">
            {discoveredList.map((sub, idx) => {
              const persona = agentPersonas.find(
                (p) => p.role.toLowerCase() === (sub.role || '').toLowerCase()
              );
              const roleBadgeStyle = getRoleBadgeStyle(sub.role, persona?.color);
              const displaySubTitle = formatSubagentDisplayTitle(sub.title || `Субагент #${sub.number}`);
              const subIsRunning = sub.status === 'running';
              const subIsSuccess = sub.status === 'finished' || sub.status === 'done' || (sub.status as string) === 'success';
              const subIsAborted = sub.status === 'stopped' || sub.status === 'aborted';

              return (
                <div
                  key={sub.id || idx}
                  onClick={() => {
                    if (sub.id) openSubagent(sub.id, sub.title, sub.role, '');
                  }}
                  className="group flex items-center justify-between gap-2 px-2.5 py-1.5 rounded-md bg-[#fafafa] dark:bg-[#0d1117] border border-[#ebebeb] dark:border-[#21262d] hover:border-[#171717]/40 dark:hover:border-[#30363d] cursor-pointer transition-colors text-xs"
                >
                  <div className="flex items-center space-x-2 min-w-0 flex-1 overflow-hidden">
                    <span className="font-mono font-bold text-[10px] text-[#71717a] dark:text-[#8b949e] shrink-0">
                      #{sub.number}
                    </span>

                    <span
                      style={roleBadgeStyle}
                      className="px-1.5 py-0.2 rounded text-[10px] font-mono font-bold uppercase tracking-wider border shrink-0 select-none"
                    >
                      {formatSubagentRoleDisplay(sub.role, sub.number)}
                    </span>

                    <span className="font-sans font-medium text-[11px] text-[#171717] dark:text-[#f0f6fc] group-hover:text-[#000000] dark:group-hover:text-white transition-colors truncate">
                      {displaySubTitle}
                    </span>
                  </div>

                  <div className="flex items-center space-x-2 shrink-0">
                    {subIsRunning ? (
                      <div className="flex items-center space-x-1">
                        <span className="relative flex h-1.5 w-1.5 shrink-0">
                          <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75" />
                          <span className="relative inline-flex rounded-full h-1.5 w-1.5 bg-emerald-500" />
                        </span>
                        <span className="text-[10px] font-mono text-[#15803d] dark:text-[#7ee787]">
                          Выполняется
                        </span>
                      </div>
                    ) : subIsSuccess ? (
                      <div className="flex items-center space-x-1">
                        <Check className="h-3 w-3 text-[#15803d] dark:text-[#7ee787] stroke-[2.5]" />
                        <span className="text-[10px] font-mono font-medium text-[#15803d] dark:text-[#7ee787]">
                          Готово
                        </span>
                      </div>
                    ) : subIsAborted ? (
                      <div className="flex items-center space-x-1">
                        <X className="h-3 w-3 text-[#d97706] dark:text-[#f59e0b] stroke-[2.5]" />
                        <span className="text-[10px] font-mono font-medium text-[#d97706] dark:text-[#f59e0b]">
                          Прервано
                        </span>
                      </div>
                    ) : (
                      <div className="flex items-center space-x-1">
                        <X className="h-3 w-3 text-[#dc2626] dark:text-[#ff7b72] stroke-[2.5]" />
                        <span className="text-[10px] font-mono font-medium text-[#dc2626] dark:text-[#ff7b72]">
                          Ошибка
                        </span>
                      </div>
                    )}

                    {sub.durationMs ? (
                      <span className="font-mono text-[10px] text-[#8f8f8f] dark:text-[#8b949e] tabular-nums">
                        {formatDurationCompact(sub.durationMs)}
                      </span>
                    ) : null}

                    <ChevronRight className="h-3.5 w-3.5 text-[#8f8f8f] group-hover:text-[#171717] dark:group-hover:text-white group-hover:translate-x-0.5 transition-all shrink-0" />
                  </div>
                </div>
              );
            })}
          </div>
        ) : !isRunning ? (
          <div className="mt-2 pt-2 border-t border-[#f4f4f5] dark:border-[#21262d] text-[11px] font-mono text-[#8f8f8f] dark:text-[#8b949e]">
            Субагенты не обнаружены
          </div>
        ) : null}
      </div>
    );
  }

  // Compact context specialized interactive card with animated stages progress bar
  if (toolName === 'compact_context') {
    return <CompactContextExecutionCard tool={tool} />;
  }

  // Check if tool is a file modification/creation operation
  const isFileOp =
    toolName === 'write_file' ||
    toolName === 'write' ||
    toolName === 'edit_file' ||
    toolName === 'edit' ||
    toolName === 'str_replace';

  // Check if tool is a file read operation
  const isReadOp =
    toolName === 'read_file' ||
    toolName === 'read' ||
    toolName === 'fetch_file';

  const isEditTool =
    toolName === 'edit_file' ||
    toolName === 'str_replace' ||
    toolName === 'edit';
  const isDiffTool = isEditTool;

  const rawFilePath =
    (params?.file_path as string) ||
    (params?.path as string) ||
    (params?.filePath as string) ||
    (params?.target_file as string) ||
    '';

  const normalizedPath = rawFilePath.replace(/\\/g, '/');
  const fileName = normalizedPath.split('/').pop() || rawFilePath;
  const dirPath = normalizedPath.substring(0, normalizedPath.length - fileName.length).replace(/\/$/, '');

  const oldString =
    (params?.old_string as string | undefined) ??
    (params?.old_str as string | undefined) ??
    (params?.before as string | undefined);

  const newString =
    (params?.new_string as string | undefined) ??
    (params?.new_str as string | undefined) ??
    (params?.content as string | undefined);

  // Compute delta additions and deletions (+X -Y)
  let additions = 0;
  let deletions = 0;

  if (oldString !== undefined && oldString !== '') {
    deletions = oldString.split('\n').length;
  }
  if (newString !== undefined && newString !== '') {
    additions = newString.split('\n').length;
  }

  // Fallback for line counts if not computed from strings
  if (additions === 0 && deletions === 0) {
    if (typeof params?.lines === 'number') {
      additions = params.lines;
    } else if (tool.stdout) {
      const match = tool.stdout.match(/\+(\d+)\s*-(\d+)/);
      if (match) {
        additions = parseInt(match[1], 10);
        deletions = parseInt(match[2], 10);
      }
    }
  }

  const isFailed =
    tool.status === 'failed' ||
    (tool.status as string) === 'error' ||
    Boolean(tool.stderr && tool.stderr.trim().length > 0) ||
    (tool.exitCode !== undefined && tool.exitCode !== null && tool.exitCode !== 0);

  // Label: "Created" (if new file) or "Edited" (if edit/overwrite)
  let opLabel: 'Created' | 'Edited' = 'Edited';
  if (isEditTool) {
    opLabel = 'Edited';
  } else {
    const op = (params?.operation as string)?.toLowerCase();
    if (op === 'create' || params?.is_new === true) {
      opLabel = 'Created';
    } else if (op === 'update' || deletions > 0 || params?.before !== undefined) {
      opLabel = 'Edited';
    } else {
      const stdoutLower = (tool.stdout ?? '').toLowerCase();
      if (stdoutLower.includes('created') || stdoutLower.includes('new file')) {
        opLabel = 'Created';
      } else if (stdoutLower.includes('update') || stdoutLower.includes('overwrit') || stdoutLower.includes('replaced')) {
        opLabel = 'Edited';
      } else {
        opLabel = deletions === 0 ? 'Created' : 'Edited';
      }
    }
  }

  // Select code icon vs generic text icon
  const ext = fileName.includes('.') ? fileName.split('.').pop()?.toLowerCase() : '';
  const isCode = ['ts', 'tsx', 'js', 'jsx', 'rs', 'py', 'json', 'toml', 'yaml', 'yml', 'xaml', 'xml', 'html', 'css', 'scss', 'sh', 'ps1', 'bat'].includes(ext || '');
  const DocIcon = isCode ? FileCode2 : FileText;
  const langBadge = getLanguageBadge(ext);

  // Line range calculations for read operations
  const offsetVal =
    params?.offset !== undefined && params?.offset !== null
      ? Number(params.offset)
      : undefined;
  const limitVal =
    params?.limit !== undefined && params?.limit !== null
      ? Number(params.limit)
      : undefined;
  const startLine =
    offsetVal !== undefined && !isNaN(offsetVal)
      ? Math.max(1, offsetVal)
      : limitVal !== undefined && !isNaN(limitVal)
      ? 1
      : undefined;
  const endLine =
    startLine !== undefined && limitVal !== undefined && !isNaN(limitVal)
      ? startLine + limitVal - 1
      : undefined;

  let lineRangeBadge: string | null = null;
  if (startLine !== undefined && endLine !== undefined) {
    lineRangeBadge = `#L${startLine}-${endLine}`;
  } else if (startLine !== undefined) {
    lineRangeBadge = `#L${startLine}+`;
  } else if (tool.stdout) {
    const stdoutLines = tool.stdout.split('\n').filter((l) => l.trim().length > 0).length;
    lineRangeBadge = `· ${stdoutLines} lines`;
  } else if (limitVal !== undefined) {
    lineRangeBadge = `· ${limitVal} lines`;
  }

  const getTarget = (): { target: string; fullTarget: string } => {
    const p = params ?? {};
    const str = (v: unknown): string | undefined =>
      typeof v === 'string' && v.trim().length > 0 ? v.trim() : undefined;

    let raw = '';
    switch (tool.toolName) {
      case 'write':
      case 'write_file':
      case 'read':
      case 'read_file':
      case 'edit':
      case 'edit_file':
      case 'str_replace':
        raw = str(p.file_path) || str(p.path) || '';
        break;
      case 'glob':
        raw = str(p.pattern) || '';
        break;
      case 'grep': {
        const pattern = str(p.pattern) || '';
        const path = str(p.path);
        raw = path && path !== '.' ? `${pattern} in ${path}` : pattern;
        break;
      }
      case 'execute_command':
      case 'bash':
      case 'pwsh':
        raw = str(p.command) || str(p.cmd) || '';
        break;
      case 'web_search':
        raw = str(p.query) || '';
        break;
      case 'web_fetch':
        raw = str(p.url) || '';
        break;
      case 'subagent':
        raw = str(p.role) || str(p.description) || str(p.prompt) || '';
        break;
      case 'compact_context':
        raw = 'Оптимизация контекста';
        break;
      default: {
        const firstStr = Object.values(p).find(
          (v) => typeof v === 'string' && (v as string).trim().length > 0
        ) as string | undefined;
        raw = firstStr || '';
        break;
      }
    }

    const fullTarget = raw;
    const target = raw.length > 55 ? `${raw.slice(0, 52)}…` : raw;
    return { target, fullTarget };
  };

  const getExtraInfo = (): string | null => {
    const p = tool.params ?? {};
    switch (tool.toolName) {
      case 'read':
      case 'read_file': {
        if (tool.stdout) {
          const lines = tool.stdout.split('\n').length;
          if (lines > 1) return `${lines} lines`;
        }
        if (typeof p.limit === 'number') {
          return `${p.limit} lines`;
        }
        return null;
      }
      case 'glob': {
        if (tool.stdout) {
          const count = tool.stdout
            .split('\n')
            .filter((l) => l.trim().length > 0).length;
          return `${count} files`;
        }
        return null;
      }
      case 'grep': {
        if (tool.stdout) {
          const count = tool.stdout
            .split('\n')
            .filter((l) => l.trim().length > 0).length;
          return `${count} matches`;
        }
        return null;
      }
      case 'compact_context': {
        if (tool.status === 'running') {
          return 'выполняется...';
        }
        if (tool.status === 'success') {
          if (typeof p.savedTokens === 'number') {
            return `-${p.savedTokens} токенов`;
          }
          return 'готово';
        }
        return null;
      }
      case 'execute_command':
      case 'bash':
      case 'pwsh': {
        if (tool.status === 'success') {
          return 'exit 0';
        }
        if (tool.status === 'failed') {
          return `exit ${tool.exitCode ?? 1}`;
        }
        return null;
      }
      default: {
        if (tool.stdout) {
          const lines = tool.stdout.split('\n').length;
          if (lines > 1) return `${lines} lines`;
        }
        return null;
      }
    }
  };

  const formatDuration = (ms: number): string | null => {
    return formatDurationCompact(ms);
  };

  const { target } = getTarget();
  const extraInfo = getExtraInfo();
  const durationText = formatDuration(tool.durationMs);

  const handleCopyLogs = (e: React.MouseEvent) => {
    e.stopPropagation();
    const formattedParams =
      tool.params && Object.keys(tool.params).length > 0
        ? Object.entries(tool.params as Record<string, unknown>)
            .map(([k, v]) => {
              if (typeof v === 'string') {
                return v.includes('\n') ? `${k}:\n${v}` : `${k}: ${v}`;
              }
              return `${k}: ${JSON.stringify(v, null, 2)}`;
            })
            .join('\n\n')
        : '';

    const content = [
      `Инструмент: ${tool.toolName}`,
      `Статус: ${tool.status} (код ${tool.exitCode ?? 0})`,
      formattedParams ? `ПАРАМЕТРЫ:\n${formattedParams}` : '',
      tool.stdout ? `STDOUT:\n${tool.stdout}` : '',
      tool.stderr ? `STDERR:\n${tool.stderr}` : '',
    ]
      .filter(Boolean)
      .join('\n\n');

    navigator.clipboard.writeText(content);
    setCopied(true);
    if (copiedTimeoutRef.current) {
      clearTimeout(copiedTimeoutRef.current);
    }
    copiedTimeoutRef.current = setTimeout(() => {
      setCopied(false);
      copiedTimeoutRef.current = null;
    }, 2000);
  };

  const renderStatusIndicator = () => {
    if (tool.status === 'running') {
      return (
        <span className="inline-flex rounded-full h-2 w-2 bg-amber-500 animate-pulse ring-2 ring-amber-500/20 shrink-0" />
      );
    }
    if (!isFailed && tool.status === 'success') {
      return (
        <span className="flex items-center shrink-0">
          <Check className="h-3.5 w-3.5 text-emerald-600 dark:text-emerald-400 stroke-[2.5]" />
        </span>
      );
    }
    return (
      <span className="flex items-center shrink-0">
        <X className="h-3.5 w-3.5 text-rose-600 dark:text-rose-400 stroke-[2.5]" />
      </span>
    );
  };

  const renderUnifiedDiff = () => {
    if (oldString !== undefined || newString !== undefined) {
      const oldLines = oldString ? oldString.split('\n') : [];
      const newLines = newString ? newString.split('\n') : [];

      return (
        <div className="rounded-md border border-[#ebebeb] dark:border-[#21262d] bg-[#fafafa] dark:bg-[#0d1117] overflow-hidden font-mono text-[11px]">
          {rawFilePath && (
            <div className="flex items-center justify-between px-2.5 py-1.5 bg-[#f4f4f5] dark:bg-[#161b22] border-b border-[#ebebeb] dark:border-[#21262d] text-[#666666] dark:text-[#8b949e] text-[10px]">
              <span className="truncate max-w-[340px] text-[#171717] dark:text-[#f0f6fc] font-semibold">
                {rawFilePath}
              </span>
              <div className="flex items-center space-x-2 font-mono tabular-nums">
                {oldLines.length > 0 && (
                  <span className="text-[#dc2626] dark:text-[#ff7b72]">-{oldLines.length}</span>
                )}
                {newLines.length > 0 && (
                  <span className="text-[#16a34a] dark:text-[#7ee787]">+{newLines.length}</span>
                )}
                <span className="text-[#8f8f8f] dark:text-[#8b949e]">Различия</span>
              </div>
            </div>
          )}

          <div className="divide-y divide-[#ebebeb] dark:divide-[#21262d] select-text overflow-x-auto max-h-72 font-mono">
            {oldLines.map((line, idx) => (
              <div
                key={`old-${idx}`}
                className="flex items-start bg-[#fef2f2] dark:bg-[#3b1219] text-[#991b1b] dark:text-[#ff7b72] px-2 py-0.5 leading-relaxed font-mono"
              >
                <span className="w-4 shrink-0 text-[#ef4444] dark:text-[#ff7b72] select-none text-center font-bold">
                  -
                </span>
                <span className="w-8 shrink-0 text-[#8f8f8f] dark:text-[#8b949e] select-none text-right pr-2 tabular-nums">
                  {idx + 1}
                </span>
                <span className="whitespace-pre-wrap break-all">
                  {line || ' '}
                </span>
              </div>
            ))}

            {newLines.map((line, idx) => (
              <div
                key={`new-${idx}`}
                className="flex items-start bg-[#f0fdf4] dark:bg-[#12281e] text-[#166534] dark:text-[#7ee787] px-2 py-0.5 leading-relaxed font-mono"
              >
                <span className="w-4 shrink-0 text-[#22c55e] dark:text-[#7ee787] select-none text-center font-bold">
                  +
                </span>
                <span className="w-8 shrink-0 text-[#8f8f8f] dark:text-[#8b949e] select-none text-right pr-2 tabular-nums">
                  {idx + 1}
                </span>
                <span className="whitespace-pre-wrap break-all">
                  {line || ' '}
                </span>
              </div>
            ))}
          </div>
        </div>
      );
    }

    return null;
  };

  const renderFileContentPreview = () => {
    if (!newString) return null;
    const lines = newString.split('\n');

    return (
      <div className="rounded-md border border-[#ebebeb] dark:border-[#21262d] bg-[#fafafa] dark:bg-[#0d1117] overflow-hidden font-mono text-[11px]">
        {rawFilePath && (
          <div className="flex items-center justify-between px-2.5 py-1.5 bg-[#f4f4f5] dark:bg-[#161b22] border-b border-[#ebebeb] dark:border-[#21262d] text-[#666666] dark:text-[#8b949e] text-[10px]">
            <span className="truncate max-w-[340px] text-[#171717] dark:text-[#f0f6fc] font-semibold">
              {rawFilePath}
            </span>
            <span className="text-[#8f8f8f] dark:text-[#8b949e] tabular-nums font-mono">
              {lines.length} {lines.length === 1 ? 'строка' : 'строк'}
            </span>
          </div>
        )}
        <div className="divide-y divide-[#f0f0f0] dark:divide-[#21262d] select-text overflow-x-auto max-h-72 font-mono bg-white dark:bg-[#0d1117]">
          {lines.map((line, idx) => (
            <div
              key={`line-${idx}`}
              className="flex items-start px-2 py-0.5 leading-relaxed hover:bg-[#fafafa] dark:hover:bg-[#161b22]"
            >
              <span className="w-8 shrink-0 text-[#a1a1aa] dark:text-[#8b949e] select-none text-right pr-2.5 tabular-nums text-[10px]">
                {idx + 1}
              </span>
              <span className="whitespace-pre-wrap break-all text-[#171717] dark:text-[#f0f6fc]">
                {line || ' '}
              </span>
            </div>
          ))}
        </div>
      </div>
    );
  };

  const renderReadContent = () => {
    if (!tool.stdout) return null;
    const rawLines = tool.stdout.split('\n');
    if (rawLines.length > 0 && rawLines[rawLines.length - 1] === '') {
      rawLines.pop();
    }
    if (rawLines.length === 0) return null;

    const baseLineNumber = startLine ?? 1;

    return (
      <div className="rounded-md border border-[#ebebeb] dark:border-[#21262d] bg-[#fafafa] dark:bg-[#0d1117] overflow-hidden font-mono text-[11px]">
        {rawFilePath && (
          <div className="flex items-center justify-between px-2.5 py-1.5 bg-[#f4f4f5] dark:bg-[#161b22] border-b border-[#ebebeb] dark:border-[#21262d] text-[#666666] dark:text-[#8b949e] text-[10px]">
            <span className="truncate max-w-[340px] text-[#171717] dark:text-[#f0f6fc] font-semibold">
              {rawFilePath}
            </span>
            <span className="text-[#8f8f8f] dark:text-[#8b949e] tabular-nums font-mono">
              {lineRangeBadge || `${rawLines.length} строк`}
            </span>
          </div>
        )}
        <div className="divide-y divide-[#f0f0f0] dark:divide-[#21262d] select-text overflow-x-auto max-h-80 font-mono bg-white dark:bg-[#0d1117]">
          {rawLines.map((line, idx) => {
            const prefixMatch = line.match(/^(\d+)[:|]\s?(.*)$/);
            let displayLineNum: number | string = baseLineNumber + idx;
            let displayContent = line;

            if (prefixMatch) {
              displayLineNum = parseInt(prefixMatch[1], 10);
              displayContent = prefixMatch[2];
            }

            return (
              <div
                key={`readline-${idx}`}
                className="flex items-start px-2 py-0.5 leading-relaxed hover:bg-[#fafafa] dark:hover:bg-[#161b22]"
              >
                <span className="w-10 shrink-0 text-[#a1a1aa] dark:text-[#8b949e] select-none text-right pr-2.5 tabular-nums text-[10px]">
                  {displayLineNum}
                </span>
                <span className="whitespace-pre-wrap break-all text-[#171717] dark:text-[#f0f6fc]">
                  {displayContent || ' '}
                </span>
              </div>
            );
          })}
        </div>
      </div>
    );
  };

  const renderToolParams = (paramsObj: Record<string, unknown>) => {
    const entries = Object.entries(paramsObj);
    if (entries.length === 0) return null;

    return (
      <div className="space-y-1.5">
        <span className="text-[10px] text-[#666666] dark:text-[#8b949e] block font-mono">
          Параметры вызова
        </span>
        <div className="space-y-2">
          {entries.map(([key, value]) => {
            if (typeof value === 'string') {
              const isMultiline = value.includes('\n');
              const isCodeLike =
                key === 'content' ||
                key === 'code' ||
                key === 'command' ||
                key === 'cmd' ||
                key === 'old_string' ||
                key === 'new_string' ||
                key === 'prompt' ||
                key === 'script' ||
                key === 'patch' ||
                isMultiline;

              if (isCodeLike || isMultiline) {
                const lineCount = value.split('\n').length;
                return (
                  <div key={key} className="space-y-1">
                    <div className="flex items-center justify-between text-[10px] text-[#666666] dark:text-[#8b949e]">
                      <span className="font-semibold text-[#171717] dark:text-[#f0f6fc] font-mono">{key}:</span>
                      <span className="text-[#8f8f8f] dark:text-[#8b949e] tabular-nums font-mono text-[10px]">
                        {lineCount > 1 ? `${lineCount} строк` : `${value.length} симв.`}
                      </span>
                    </div>
                    <pre className="p-2.5 rounded-md bg-[#fafafa] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#21262d] text-[11px] text-[#171717] dark:text-[#f0f6fc] overflow-x-auto leading-relaxed max-h-64 whitespace-pre select-text font-mono">
                      {value}
                    </pre>
                  </div>
                );
              }

              return (
                <div key={key} className="flex items-baseline space-x-2 py-0.5 text-[11px] font-mono">
                  <span className="text-[#666666] dark:text-[#8b949e] font-semibold shrink-0">{key}:</span>
                  <span className="text-[#171717] dark:text-[#f0f6fc] select-text break-all">{value}</span>
                </div>
              );
            }

            if (typeof value === 'object' && value !== null) {
              return (
                <div key={key} className="space-y-1">
                  <span className="text-[10px] font-semibold text-[#171717] dark:text-[#f0f6fc] font-mono">{key}:</span>
                  <pre className="p-2 rounded-md bg-[#fafafa] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#21262d] text-[11px] text-[#171717] dark:text-[#f0f6fc] overflow-x-auto leading-relaxed max-h-48 whitespace-pre-wrap select-text font-mono">
                    {JSON.stringify(value, null, 2)}
                  </pre>
                </div>
              );
            }

            return (
              <div key={key} className="flex items-baseline space-x-2 py-0.5 text-[11px] font-mono">
                <span className="text-[#666666] dark:text-[#8b949e] font-semibold shrink-0">{key}:</span>
                <span className="text-[#171717] dark:text-[#f0f6fc] select-text">{String(value)}</span>
              </div>
            );
          })}
        </div>
      </div>
    );
  };

  const isRunning = tool.status === 'running';

  return (
    <div
      className={`group rounded-md border text-xs shadow-2xs hover:shadow-xs overflow-hidden my-1 animate-silk-entrance transition-colors bg-white dark:bg-[#161b22] ${
        isRunning
          ? 'border-amber-500/40 dark:border-amber-500/30 bg-amber-500/[0.02]'
          : 'border-[#ebebeb] dark:border-[#21262d]'
      }`}
    >
      {/* Header Strip */}
      <div
        onClick={() => setIsExpanded(!isExpanded)}
        className={`flex items-center justify-between px-2.5 py-1.5 bg-[#f8f8f9] dark:bg-[#161b22] hover:bg-[#f2f2f3] dark:hover:bg-[#1c2128] transition-colors cursor-pointer select-none gap-2 text-xs min-h-[30px] ${
          isExpanded ? 'border-b border-[#ebebeb] dark:border-[#21262d]' : ''
        }`}
      >
        {isReadOp ? (
          /* Read operation tactile card: [Analyzed C# Transit.cs #L4270-4300] */
          <div className="flex items-center space-x-2 min-w-0 flex-1 overflow-hidden">
            <button
              type="button"
              className="p-0.5 text-[#8f8f8f] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] rounded shrink-0 transition cursor-pointer"
              onClick={(e) => {
                e.stopPropagation();
                setIsExpanded(!isExpanded);
              }}
              aria-label={isExpanded ? 'Свернуть подробности' : 'Развернуть подробности'}
            >
              <ChevronRight
                className={`h-3 w-3 transition-transform duration-150 ${
                  isExpanded ? 'rotate-90 text-[#171717] dark:text-[#f0f6fc]' : ''
                }`}
              />
            </button>

            {/* Label: Analyzed (cyan/teal tactile badge) */}
            <span className="px-1.5 py-0.5 rounded text-[10px] font-mono font-semibold uppercase tracking-wider shrink-0 select-none bg-[#ecfeff] dark:bg-[#083344] text-[#0e7490] dark:text-[#22d3ee] border border-[#a5f3fc] dark:border-[#155e75]">
              Analyzed
            </span>

            {/* Brand SVG language badge or document icon */}
            {langBadge ? (
              <FileExtensionBadge ext={ext} />
            ) : (
              <DocIcon className="h-3.5 w-3.5 text-[#52525b] dark:text-[#8b949e] shrink-0" />
            )}

            {/* Clean filename with muted path */}
            <div className="flex items-center min-w-0 font-mono text-xs overflow-hidden">
              {dirPath && (
                <span className="text-[#8f8f8f] dark:text-[#8b949e] truncate max-w-[120px] sm:max-w-[180px] shrink-0">
                  {dirPath}/
                </span>
              )}
              <span className="font-semibold text-[#171717] dark:text-[#f0f6fc] truncate">
                {fileName || rawFilePath || 'file'}
              </span>
            </div>

            {/* Line range badge (#L4270-4300 or · 61 lines) */}
            {lineRangeBadge && (
              <span className="font-mono text-[11px] text-[#71717a] dark:text-[#8b949e] tabular-nums shrink-0 ml-1">
                {lineRangeBadge}
              </span>
            )}

            {durationText && (
              <>
                <span className="text-[#a1a1aa] dark:text-[#6e7681] select-none text-[10px]">•</span>
                <span className="font-mono text-[#71717a] dark:text-[#8b949e] shrink-0 tabular-nums text-[11px]">
                  {durationText}
                </span>
              </>
            )}
          </div>
        ) : isFileOp ? (
          /* File operation card: [Edited MainWindow.xaml +4 -2] */
          <div className="flex items-center space-x-2 min-w-0 flex-1 overflow-hidden">
            <button
              type="button"
              className="p-0.5 text-[#8f8f8f] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] rounded shrink-0 transition cursor-pointer"
              onClick={(e) => {
                e.stopPropagation();
                setIsExpanded(!isExpanded);
              }}
              aria-label={isExpanded ? 'Свернуть подробности' : 'Развернуть подробности'}
            >
              <ChevronRight
                className={`h-3 w-3 transition-transform duration-150 ${
                  isExpanded ? 'rotate-90 text-[#171717] dark:text-[#f0f6fc]' : ''
                }`}
              />
            </button>

            {/* Label: Created or Edited or FAILED EDIT */}
            <span
              className={`px-1.5 py-0.5 rounded text-[10px] font-mono font-semibold uppercase tracking-wider shrink-0 select-none ${
                isFailed
                  ? 'bg-[#fef2f2] dark:bg-[#3b1219] text-[#b91c1c] dark:text-[#ff7b72] border border-[#fecaca] dark:border-[#da3633]'
                  : opLabel === 'Created'
                  ? 'bg-[#ecfdf5] dark:bg-[#12281e] text-[#047857] dark:text-[#7ee787] border border-[#a7f3d0] dark:border-[#238636]'
                  : 'bg-[#eff6ff] dark:bg-[#172554] text-[#1d4ed8] dark:text-[#60a5fa] border border-[#bfdbfe] dark:border-[#1e40af]'
              }`}
            >
              {isFailed ? 'FAILED EDIT' : opLabel}
            </span>

            {/* Brand SVG language badge or document icon */}
            {langBadge ? (
              <FileExtensionBadge ext={ext} />
            ) : (
              <DocIcon className="h-3.5 w-3.5 text-[#52525b] dark:text-[#8b949e] shrink-0" />
            )}

            {/* Clean filename with muted path */}
            <div className="flex items-center min-w-0 font-mono text-xs overflow-hidden">
              {dirPath && (
                <span className="text-[#8f8f8f] dark:text-[#8b949e] truncate max-w-[120px] sm:max-w-[180px] shrink-0">
                  {dirPath}/
                </span>
              )}
              <span className="font-semibold text-[#171717] dark:text-[#f0f6fc] truncate">
                {fileName || rawFilePath || 'file'}
              </span>
            </div>

            {/* Delta badge: +X -Y */}
            {(additions > 0 || deletions > 0) && (
              <div className="flex items-center space-x-1 font-mono text-[11px] tabular-nums font-semibold shrink-0 ml-1">
                {additions > 0 && (
                  <span className="text-[#16a34a] dark:text-[#7ee787] bg-[#f0fdf4] dark:bg-[#12281e] px-1 py-0.2 rounded border border-[#bbf7d0] dark:border-[#238636]">
                    +{additions}
                  </span>
                )}
                {deletions > 0 && (
                  <span className="text-[#dc2626] dark:text-[#ff7b72] bg-[#fef2f2] dark:bg-[#3b1219] px-1 py-0.2 rounded border border-[#fecaca] dark:border-[#da3633]">
                    -{deletions}
                  </span>
                )}
              </div>
            )}

            {durationText && (
              <>
                <span className="text-[#a1a1aa] dark:text-[#6e7681] select-none text-[10px]">•</span>
                <span className="font-mono text-[#71717a] dark:text-[#8b949e] shrink-0 tabular-nums text-[11px]">
                  {durationText}
                </span>
              </>
            )}
          </div>
        ) : (
          /* Standard tool card */
          <div className="flex items-center space-x-1.5 min-w-0 flex-1 overflow-hidden">
            <button
              type="button"
              className="p-0.5 text-[#8f8f8f] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] rounded shrink-0 transition cursor-pointer"
              onClick={(e) => {
                e.stopPropagation();
                setIsExpanded(!isExpanded);
              }}
              aria-label={isExpanded ? 'Свернуть подробности' : 'Развернуть подробности'}
            >
              <ChevronRight
                className={`h-3 w-3 transition-transform duration-150 ${
                  isExpanded ? 'rotate-90 text-[#171717] dark:text-[#f0f6fc]' : ''
                }`}
              />
            </button>

            <span className="flex items-center shrink-0 select-none text-[#8f8f8f] dark:text-[#8b949e]">
              {tool.toolName === 'compact_context' ? (
                <Minimize2 className="h-3.5 w-3.5 text-indigo-600 dark:text-indigo-400" />
              ) : (
                <Wrench className="h-3.5 w-3.5" />
              )}
            </span>

            <span className="font-mono font-medium text-[#171717] dark:text-[#f0f6fc] shrink-0">
              {tool.toolName}{target ? ':' : ''}
            </span>

            {target && (
              <span className="font-mono text-[#52525b] dark:text-[#8b949e] truncate max-w-[260px] sm:max-w-[360px] md:max-w-[460px]">
                {target}
              </span>
            )}

            {extraInfo && (
              <>
                <span className="text-[#a1a1aa] dark:text-[#6e7681] select-none text-[10px]">•</span>
                <span className="font-mono text-[#71717a] dark:text-[#8b949e] shrink-0 tabular-nums text-[11px]">
                  {extraInfo}
                </span>
              </>
            )}

            {durationText && (
              <>
                <span className="text-[#a1a1aa] dark:text-[#6e7681] select-none text-[10px]">•</span>
                <span className="font-mono text-[#71717a] dark:text-[#8b949e] shrink-0 tabular-nums text-[11px]">
                  {durationText}
                </span>
              </>
            )}
          </div>
        )}

        <div className="flex items-center space-x-2 shrink-0">
          <button
            type="button"
            onClick={handleCopyLogs}
            className={`p-1 rounded text-[#8f8f8f] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-white dark:hover:bg-[#161b22] transition cursor-pointer ${
              isExpanded ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'
            }`}
            aria-label="Скопировать вывод"
          >
            {copied ? (
              <Check className="h-3 w-3 text-emerald-600 dark:text-emerald-400" />
            ) : (
              <Copy className="h-3 w-3" />
            )}
          </button>

          {renderStatusIndicator()}
        </div>
      </div>

      {/* Expanded Details */}
      {isExpanded && (
        <div className="bg-white dark:bg-[#0d1117] p-3 border-t border-[#ebebeb] dark:border-[#21262d] space-y-2.5 font-mono text-xs">
          {isReadOp ? (
            <div className="space-y-2">
              {rawFilePath && (
                <div className="flex items-baseline space-x-2 py-0.5 text-[11px] font-mono">
                  <span className="text-[#666666] dark:text-[#8b949e] font-semibold shrink-0">Файл:</span>
                  <span className="text-[#171717] dark:text-[#f0f6fc] select-text break-all">{rawFilePath}</span>
                </div>
              )}
              {tool.stdout ? (
                <div>
                  <span className="text-[10px] text-[#666666] dark:text-[#8b949e] block mb-1">
                    Прочитанные строки {lineRangeBadge ? `(${lineRangeBadge})` : ''}
                  </span>
                  {renderReadContent()}
                </div>
              ) : (
                tool.params &&
                Object.keys(tool.params).length > 0 && renderToolParams(params)
              )}
            </div>
          ) : isFileOp ? (
            <div className="space-y-2">
              {rawFilePath && (
                <div className="flex items-baseline space-x-2 py-0.5 text-[11px] font-mono">
                  <span className="text-[#666666] dark:text-[#8b949e] font-semibold shrink-0">Файл:</span>
                  <span className="text-[#171717] dark:text-[#f0f6fc] select-text break-all">{rawFilePath}</span>
                </div>
              )}
              {oldString !== undefined && newString !== undefined ? (
                <div>
                  <span className="text-[10px] text-[#666666] dark:text-[#8b949e] block mb-1">
                    {isFailed ? 'Запланированные изменения (не применены)' : 'Дифф изменений'}
                  </span>
                  {renderUnifiedDiff()}
                </div>
              ) : newString ? (
                <div>
                  <span className="text-[10px] text-[#666666] dark:text-[#8b949e] block mb-1">
                    Содержимое файла
                  </span>
                  {renderFileContentPreview()}
                </div>
              ) : (
                tool.params &&
                Object.keys(tool.params).length > 0 && renderToolParams(params)
              )}
            </div>
          ) : isDiffTool && (oldString !== undefined || newString !== undefined) ? (
            <div className="space-y-2">
              {rawFilePath && (
                <div className="flex items-baseline space-x-2 py-0.5 text-[11px] font-mono">
                  <span className="text-[#666666] dark:text-[#8b949e] font-semibold shrink-0">Файл:</span>
                  <span className="text-[#171717] dark:text-[#f0f6fc] select-text break-all">{rawFilePath}</span>
                </div>
              )}
              <div>
                <span className="text-[10px] text-[#666666] dark:text-[#8b949e] block mb-1">
                  {isFailed ? 'Запланированные изменения (не применены)' : 'Дифф изменений'}
                </span>
                {renderUnifiedDiff()}
              </div>
            </div>
          ) : (
            tool.params &&
            Object.keys(tool.params).length > 0 && renderToolParams(params)
          )}

          {tool.stdout && (
            <div>
              <span className="text-[10px] text-[#666666] dark:text-[#8b949e] block mb-1">
                Вывод команды
              </span>
              <pre className="p-2.5 rounded-md bg-[#fafafa] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#21262d] text-[11px] text-[#171717] dark:text-[#f0f6fc] overflow-x-auto leading-relaxed max-h-48 whitespace-pre-wrap select-text">
                {tool.stdout}
              </pre>
            </div>
          )}

          {tool.stderr && (
            <div>
              <span className="text-[10px] text-[#b91c1c] dark:text-[#ff7b72] block mb-1">
                Ошибки выполнения
              </span>
              <pre className="p-2.5 rounded-md bg-[#fef2f2] dark:bg-[#3b1219] border border-[#fecaca] dark:border-[#da3633] text-[11px] text-[#991b1b] dark:text-[#ff7b72] overflow-x-auto leading-relaxed max-h-48 whitespace-pre-wrap select-text">
                {tool.stderr}
              </pre>
            </div>
          )}

          <div className="flex items-center justify-between pt-1 border-t border-[#f4f4f5] dark:border-[#21262d] text-[10px] text-[#8f8f8f] dark:text-[#8b949e]">
            <span>Статус: {isFailed ? `failed (код ${tool.exitCode ?? 1})` : tool.status}</span>
          </div>
        </div>
      )}
    </div>
  );
}
