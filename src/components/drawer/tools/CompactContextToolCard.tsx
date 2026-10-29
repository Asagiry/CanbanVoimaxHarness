import { useState, useRef, useEffect } from 'react';
import { useTranslation } from '../../../stores/languageStore';
import { formatDurationCompact } from '../ThinkingCard';
import { ChevronRight, Copy, Check, X, Minimize2 } from 'lucide-react';
import { ToolCardProps, isToolFailed } from './shared';

export default function CompactContextToolCard({ tool }: ToolCardProps) {
  const [isExpanded, setIsExpanded] = useState(false);
  const [copied, setCopied] = useState(false);
  const copiedTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [activeStep, setActiveStep] = useState(1);
  const [progressPct, setProgressPct] = useState(25);
  const { t } = useTranslation();

  const isRunning = tool.status === 'running';
  const isSuccess = tool.status === 'success';
  const isFailed = isToolFailed(tool);

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
          <Minimize2 className="h-3.5 w-3.5 text-indigo-600 dark:text-indigo-400 shrink-0" />

          {/* Badge */}
          <span className="px-1.5 py-0.5 rounded text-[10px] font-mono font-medium shrink-0 select-none bg-indigo-50 dark:bg-indigo-950/40 text-indigo-700 dark:text-indigo-300 border border-indigo-200 dark:border-indigo-800/60">
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
              <span className="text-[11px] font-mono text-indigo-600 dark:text-indigo-400">{t('tools.status.running')}</span>
            </div>
          ) : isSuccess ? (
            <div className="flex items-center space-x-1.5">
              <Check className="h-3.5 w-3.5 text-emerald-600 dark:text-emerald-400 stroke-[2.5]" />
              <span className="text-[11px] font-mono font-medium text-emerald-600 dark:text-emerald-400">{t('tools.status.done')}</span>
            </div>
          ) : (
            <div className="flex items-center space-x-1.5">
              <X className="h-3.5 w-3.5 text-rose-600 dark:text-rose-400 stroke-[2.5]" />
              <span className="text-[11px] font-mono font-medium text-rose-600 dark:text-rose-400">{t('tools.status.error')}</span>
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
            <div className="h-1.5 w-full bg-[#f4f4f5] dark:bg-[#161b22] border border-transparent dark:border-[#30363d] rounded-full overflow-hidden">
              <div
                className="h-full bg-indigo-600 dark:bg-indigo-500 transition-all duration-300 ease-out rounded-full"
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
