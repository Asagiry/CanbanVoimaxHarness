import React, { useState, useMemo, useRef, memo } from 'react';
import type { DrawerActivityItem } from '../../stores/drawerStore';
import { useDrawerStore } from '../../stores/drawerStore';
import { Check, CornerDownLeft, Lock } from 'lucide-react';
import { MarkdownContent, formatInline } from './stream';

interface QuestionOptionItem {
  label: string;
  description?: string;
}

export interface QuestionPromptCardProps {
  activity: DrawerActivityItem;
  taskId: string;
}

function QuestionMarkIcon({ className = 'w-4 h-4' }: { className?: string }) {
  return (
    <svg viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg" className={className}>
      <circle cx="8" cy="8" r="7" stroke="currentColor" strokeWidth="1.5" />
      <path
        d="M6.25 6.25C6.25 5.28 7.03 4.5 8 4.5C8.97 4.5 9.75 5.28 9.75 6.25C9.75 7.15 9.07 7.7 8.5 8.1C8.16 8.34 8 8.65 8 9.1V9.25"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
      />
      <circle cx="8" cy="11.5" r="0.75" fill="currentColor" />
    </svg>
  );
}

export function QuestionPromptCardInner({ activity, taskId }: QuestionPromptCardProps) {
  const recordQuestionAnswer = useDrawerStore((s) => s.recordQuestionAnswer);

  const params = useMemo(() => {
    const rawParams = activity.tool?.params;
    if (rawParams && typeof rawParams === 'object') {
      return rawParams as Record<string, unknown>;
    }
    return {} as Record<string, unknown>;
  }, [activity.tool?.params]);

  const questionId = useMemo(() => {
    return (
      (params.questionId as string) ||
      activity.tool?.id?.replace(/^q-/, '') ||
      activity.id.replace(/^q-/, '')
    );
  }, [params.questionId, activity.tool?.id, activity.id]);

  const questionText = useMemo(() => {
    return (
      (params.text as string) ||
      activity.content ||
      'Агент запросил уточнение для продолжения работы'
    );
  }, [params.text, activity.content]);

  const isMultiple = Boolean(params.multiple);

  const rawOptions = useMemo(() => {
    const opts = params.options;
    if (Array.isArray(opts)) {
      return opts
        .map((opt) => {
          if (typeof opt === 'string') {
            return { label: opt, description: undefined } as QuestionOptionItem;
          }
          if (opt && typeof opt === 'object' && 'label' in opt) {
            return {
              label: String((opt as { label: unknown }).label || ''),
              description: (opt as { description?: unknown }).description
                ? String((opt as { description?: unknown }).description)
                : undefined,
            } as QuestionOptionItem;
          }
          return null;
        })
        .filter((opt): opt is QuestionOptionItem => Boolean(opt && opt.label));
    }
    return [] as QuestionOptionItem[];
  }, [params.options]);

  // Answered state detection
  const storedAnswer = useMemo(() => {
    if (params.selectedAnswer && typeof params.selectedAnswer === 'string') {
      return params.selectedAnswer;
    }
    if (activity.tool?.status === 'success' && activity.tool.stdout) {
      return activity.tool.stdout;
    }
    return null;
  }, [params.selectedAnswer, activity.tool?.status, activity.tool?.stdout]);

  const [localAnswer, setLocalAnswer] = useState<string | null>(storedAnswer);
  const [selectedMultiple, setSelectedMultiple] = useState<Set<string>>(new Set());
  const [customInput, setCustomInput] = useState('');
  const [isCustomFocused, setIsCustomFocused] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  const isAnswered = Boolean(localAnswer || storedAnswer);
  const activeAnswer = localAnswer || storedAnswer;

  const handleSelectOption = async (optionLabel: string) => {
    if (isAnswered || isSubmitting) return;

    if (isMultiple) {
      setSelectedMultiple((prev) => {
        const next = new Set(prev);
        if (next.has(optionLabel)) {
          next.delete(optionLabel);
        } else {
          next.add(optionLabel);
        }
        return next;
      });
      return;
    }

    // Single choice: immediately resolve
    setIsSubmitting(true);
    setLocalAnswer(optionLabel);
    try {
      const ok = await recordQuestionAnswer(taskId, questionId, optionLabel);
      if (!ok) {
        setLocalAnswer(null);
      }
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleCustomSubmit = async (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    const trimmed = customInput.trim();
    if (!trimmed || isAnswered || isSubmitting) return;

    setIsSubmitting(true);
    setLocalAnswer(trimmed);
    try {
      const ok = await recordQuestionAnswer(taskId, questionId, trimmed);
      if (!ok) {
        setLocalAnswer(null);
      }
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleMultipleSubmit = async () => {
    if (isAnswered || isSubmitting || selectedMultiple.size === 0) return;
    const combined = Array.from(selectedMultiple).join(', ');
    setIsSubmitting(true);
    setLocalAnswer(combined);
    try {
      const ok = await recordQuestionAnswer(taskId, questionId, combined);
      if (!ok) {
        setLocalAnswer(null);
      }
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="rounded-lg border border-[#ebebeb] dark:border-[#212126] bg-white dark:bg-[#111114] p-4 text-[#171717] dark:text-[#f4f4f5] shadow-xs my-3 transition-colors">
      {/* 1. Header with Question Counter & Icon */}
      <div className="flex items-center justify-between pb-2.5 mb-3 border-b border-[#ebebeb] dark:border-[#212126]">
        <div className="flex items-center space-x-2 text-[#71717a] dark:text-[#71717a]">
          <QuestionMarkIcon className="w-3.5 h-3.5 text-[#171717] dark:text-[#f4f4f5]" />
          <span className="text-xs font-mono font-medium text-[#71717a] dark:text-[#71717a] tabular-nums">
            1 из 1 вопросов
          </span>
          {isMultiple && !isAnswered && (
            <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-[#f4f4f5] dark:bg-[#17171b] text-[#71717a] dark:text-[#71717a] border border-[#ebebeb] dark:border-[#2c2c33]">
              множественный выбор
            </span>
          )}
        </div>

        <div className="flex items-center space-x-2">
          {isAnswered ? (
            <div className="flex items-center space-x-1.5 px-2 py-0.5 rounded-md bg-[#f0fdf4] dark:bg-[#12281e] border border-[#bbf7d0] dark:border-[#238636] text-[#15803d] dark:text-[#7ee787] text-[11px] font-mono">
              <Check className="w-3 h-3 stroke-[2.5]" />
              <span>Отвечено</span>
            </div>
          ) : (
            <div className="flex items-center space-x-1 text-[11px] font-mono text-amber-600 dark:text-amber-400">
              <span className="w-1.5 h-1.5 rounded-full bg-amber-500" />
              <span>Ожидает ответа</span>
            </div>
          )}
          {activity.durationMs ? (
            <span className="text-[10px] font-mono text-[#8f8f8f] dark:text-[#71717a] tabular-nums">
              {activity.durationMs < 1000 ? `${Math.round(activity.durationMs)}ms` : `${(activity.durationMs / 1000).toFixed(1)}s`}
            </span>
          ) : null}
        </div>
      </div>

      {/* 2. Question Text with Markdown Support */}
      <div className="mb-4 select-text leading-relaxed text-[#171717] dark:text-[#f4f4f5]">
        <MarkdownContent content={questionText} />
      </div>

      {/* 3. Predefined Selectable Choice Cards */}
      <div className="space-y-2">
        {rawOptions.map((option, idx) => {
          const isSelected = isAnswered
            ? activeAnswer === option.label ||
              (activeAnswer && activeAnswer.includes(option.label))
            : isMultiple
            ? selectedMultiple.has(option.label)
            : false;

          const isRecommended =
            option.label.includes('(Рекомендуется)') ||
            option.label.includes('(Recommended)');

          const cleanLabel = option.label
            .replace('(Рекомендуется)', '')
            .replace('(Recommended)', '')
            .trim();

          return (
            <button
              key={idx}
              type="button"
              disabled={isAnswered || isSubmitting}
              onClick={() => handleSelectOption(option.label)}
              className={`w-full text-left p-3 rounded-md border transition-colors flex items-start space-x-3 group ${
                isSelected
                  ? 'border-[#171717] dark:border-[#f4f4f5] bg-[#f9fafb] dark:bg-[#17171b]'
                  : 'border-[#ebebeb] dark:border-[#2c2c33] bg-white dark:bg-[#111114] hover:border-[#a1a1aa] dark:hover:border-[#52525b]'
              } ${
                isAnswered
                  ? 'cursor-default opacity-85'
                  : 'cursor-pointer'
              }`}
            >
              {/* Radio circle */}
              <div
                className={`w-4 h-4 rounded-full border flex items-center justify-center shrink-0 mt-0.5 transition-colors ${
                  isSelected
                    ? 'border-[#171717] dark:border-[#f4f4f5] bg-[#171717] dark:bg-[#f4f4f5]'
                    : 'border-[#a1a1aa] dark:border-[#52525b]'
                }`}
              >
                {isSelected && (
                  <div className="w-1.5 h-1.5 rounded-full bg-white dark:bg-[#111114]" />
                )}
              </div>

              {/* Title & Subtitle */}
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2 flex-wrap">
                  <span className="text-sm font-medium text-[#171717] dark:text-[#f4f4f5]">
                    {formatInline(cleanLabel, `opt-${idx}`)}
                  </span>
                  {isRecommended && (
                    <span className="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-mono leading-none bg-[#f0fdf4] dark:bg-[#12281e] text-[#15803d] dark:text-[#7ee787] border border-[#bbf7d0] dark:border-[#238636]">
                      Рекомендуется
                    </span>
                  )}
                  {isSelected && isAnswered && (
                    <span className="ml-auto flex items-center space-x-1 text-xs text-[#15803d] dark:text-[#7ee787] font-mono">
                      <Check className="w-3.5 h-3.5" />
                      <span>Выбрано</span>
                    </span>
                  )}
                </div>
                {option.description && (
                  <p className="text-xs text-[#71717a] dark:text-[#71717a] font-mono mt-1 leading-relaxed">
                    {formatInline(option.description, `desc-${idx}`)}
                  </p>
                )}
              </div>
            </button>
          );
        })}

        {/* 4. Bottom Custom Input Option */}
        {!isAnswered ? (
          <form
            onSubmit={handleCustomSubmit}
            className={`rounded-md border transition-colors p-2.5 px-3 flex items-center space-x-3 ${
              isCustomFocused || customInput.length > 0
                ? 'border-[#171717] dark:border-[#f4f4f5] bg-[#fafafa] dark:bg-[#17171b]'
                : 'border-[#ebebeb] dark:border-[#2c2c33] bg-white dark:bg-[#17171b] hover:border-[#a1a1aa] dark:hover:border-[#52525b]'
            }`}
          >
            {/* Radio circle */}
            <div
              className={`w-4 h-4 rounded-full border flex items-center justify-center shrink-0 transition-colors ${
                customInput.length > 0
                  ? 'border-[#171717] dark:border-[#f4f4f5] bg-[#171717] dark:bg-[#f4f4f5]'
                  : 'border-[#a1a1aa] dark:border-[#52525b]'
              }`}
            >
              {customInput.length > 0 && (
                <div className="w-1.5 h-1.5 rounded-full bg-white dark:bg-[#111114]" />
              )}
            </div>

            <input
              ref={inputRef}
              type="text"
              value={customInput}
              disabled={isSubmitting}
              onChange={(e) => setCustomInput(e.target.value)}
              onFocus={() => setIsCustomFocused(true)}
              onBlur={() => setIsCustomFocused(false)}
              placeholder="Введите свой ответ..."
              className="flex-1 bg-transparent border-none text-sm text-[#171717] dark:text-[#f4f4f5] placeholder-[#8f8f8f] dark:placeholder-[#52525b] focus:outline-none font-sans"
            />

            {customInput.trim().length > 0 && (
              <button
                type="submit"
                disabled={isSubmitting}
                className="px-2.5 py-1 rounded-md bg-[#171717] dark:bg-[#f4f4f5] hover:bg-black dark:hover:bg-white text-white dark:text-[#0b0b0e] text-xs font-medium flex items-center space-x-1 transition-colors cursor-pointer shadow-xs"
              >
                <span>Отправить</span>
                <CornerDownLeft className="w-3 h-3 ml-0.5" />
              </button>
            )}
          </form>
        ) : (
          /* Answered custom response display (if selected answer wasn't among predefined) */
          activeAnswer &&
          !rawOptions.some((o) => o.label === activeAnswer) && (
            <div className="p-3 rounded-md border border-[#171717] dark:border-[#f4f4f5] bg-[#f9fafb] dark:bg-[#17171b] flex items-start space-x-3">
              <div className="w-4 h-4 rounded-full border border-[#171717] dark:border-[#f4f4f5] bg-[#171717] dark:bg-[#f4f4f5] flex items-center justify-center shrink-0 mt-0.5">
                <div className="w-1.5 h-1.5 rounded-full bg-white dark:bg-[#111114]" />
              </div>
              <div className="flex-1 min-w-0">
                <span className="text-xs font-mono text-[#71717a] dark:text-[#71717a]">Пользовательский ответ:</span>
                <p className="text-sm font-medium text-[#171717] dark:text-[#f4f4f5] mt-0.5 break-words">
                  {formatInline(activeAnswer, 'ans-custom')}
                </p>
              </div>
              <span className="flex items-center space-x-1 text-xs text-[#15803d] dark:text-[#7ee787] font-mono">
                <Check className="w-3.5 h-3.5" />
                <span>Выбрано</span>
              </span>
            </div>
          )
        )}
      </div>

      {/* Multiple Choice Submit Action */}
      {isMultiple && !isAnswered && (
        <div className="mt-3.5 pt-3 border-t border-neutral-800/80 flex items-center justify-between">
          <span className="text-xs font-mono text-neutral-400">
            Выбрано вариантов: {selectedMultiple.size}
          </span>
          <button
            type="button"
            disabled={selectedMultiple.size === 0 || isSubmitting}
            onClick={handleMultipleSubmit}
            className={`px-3 py-1.5 rounded-lg text-xs font-medium flex items-center space-x-1.5 transition ${
              selectedMultiple.size > 0
                ? 'bg-indigo-600 hover:bg-indigo-500 text-white cursor-pointer active:scale-95'
                : 'bg-neutral-800 text-neutral-500 cursor-not-allowed'
            }`}
          >
            <span>Отправить ответ</span>
            <CornerDownLeft className="w-3 h-3" />
          </button>
        </div>
      )}

      {/* Answered Footer Note */}
      {isAnswered && (
        <div className="mt-3 pt-2.5 border-t border-neutral-800/80 flex items-center justify-between text-xs font-mono text-neutral-400">
          <div className="flex items-center space-x-1.5 truncate">
            <Lock className="w-3 h-3 text-neutral-500 shrink-0" />
            <span className="text-neutral-500">Зафиксированный ответ:</span>
            <span className="text-white font-medium truncate max-w-md">
              {activeAnswer}
            </span>
          </div>
          <span className="text-[10px] text-emerald-400 font-mono">
            Агент возобновил выполнение
          </span>
        </div>
      )}
    </div>
  );
}

const QuestionPromptCard = memo(QuestionPromptCardInner);
export default QuestionPromptCard;
