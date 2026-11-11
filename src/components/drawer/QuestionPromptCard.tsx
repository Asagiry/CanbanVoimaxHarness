import React, { useState, useMemo, useRef, memo } from 'react';
import type { DrawerActivityItem } from '../../stores/drawerStore';
import { useDrawerStore } from '../../stores/drawerStore';
import { useTranslation } from '../../stores/languageStore';
import { Check, CornerDownLeft, HelpCircle, Lock } from 'lucide-react';
import { MarkdownContent, formatInline } from './stream';
import { QuestionOptionRow, type QuestionOptionItem } from './QuestionOptionRow';

export interface QuestionPromptCardProps {
  activity: DrawerActivityItem;
  taskId: string;
}

function isOptionMatched(
  optionLabel: string,
  activeAnswer: string | null,
  isMultiple: boolean
): boolean {
  if (!activeAnswer) return false;
  if (activeAnswer === optionLabel) return true;
  if (isMultiple) {
    return activeAnswer.split(', ').includes(optionLabel);
  }
  return false;
}

export function QuestionPromptCardInner({ activity, taskId }: QuestionPromptCardProps) {
  const recordQuestionAnswer = useDrawerStore((s) => s.recordQuestionAnswer);
  const { t } = useTranslation();

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
      t('question.defaultPrompt')
    );
  }, [params.text, activity.content, t]);

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

  const isCustomAnswered = useMemo(() => {
    if (!activeAnswer) return false;
    if (isMultiple) {
      return !activeAnswer
        .split(', ')
        .every((part) => rawOptions.some((o) => o.label === part));
    }
    return !rawOptions.some((o) => o.label === activeAnswer);
  }, [activeAnswer, isMultiple, rawOptions]);

  const handleSelectOption = async (optionLabel: string) => {
    if (isAnswered || isSubmitting) return;

    if (isMultiple) {
      setSelectedMultiple((prev) => {
        const next = new Set(prev);
        if (next.has(optionLabel)) next.delete(optionLabel);
        else next.add(optionLabel);
        return next;
      });
      return;
    }

    setIsSubmitting(true);
    setLocalAnswer(optionLabel);
    try {
      const ok = await recordQuestionAnswer(taskId, questionId, optionLabel);
      if (!ok) setLocalAnswer(null);
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
      if (!ok) setLocalAnswer(null);
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
      if (!ok) setLocalAnswer(null);
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="rounded-lg border border-[#ebebeb] dark:border-[#212126] bg-white dark:bg-[#111114] p-4 text-[#171717] dark:text-[#f4f4f5] shadow-xs my-3 transition-colors">
      {/* 1. Header with Question Counter & Icon */}
      <div className="flex items-center justify-between pb-2.5 mb-3 border-b border-[#ebebeb] dark:border-[#212126]">
        <div className="flex items-center gap-2 leading-none text-[#71717a] dark:text-[#71717a]">
          <HelpCircle className="w-3.5 h-3.5 block shrink-0 text-[#171717] dark:text-[#f4f4f5]" />
          <span className="text-xs font-mono font-medium text-[#71717a] dark:text-[#71717a] tabular-nums">
            {t('question.counter')}
          </span>
          {isMultiple && !isAnswered && (
            <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-[#f4f4f5] dark:bg-[#17171b] text-[#71717a] dark:text-[#71717a] border border-[#ebebeb] dark:border-[#2c2c33]">
              {t('question.multipleChoice')}
            </span>
          )}
        </div>

        <div className="flex items-center space-x-2">
          {isAnswered ? (
            <div className="flex items-center gap-1.5 leading-none px-2 py-0.5 rounded-md bg-[#f0fdf4] dark:bg-[#12281e] border border-[#bbf7d0] dark:border-[#238636] text-[#15803d] dark:text-[#7ee787] text-[11px] font-mono">
              <Check className="w-3 h-3 block shrink-0 stroke-[2.5]" />
              <span>{t('question.answered')}</span>
            </div>
          ) : (
            <div className="flex items-center gap-1 leading-none text-[11px] font-mono text-amber-600 dark:text-amber-400">
              <span className="w-1.5 h-1.5 rounded-full bg-amber-500" />
              <span>{t('question.awaitingAnswer')}</span>
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
        <MarkdownContent content={questionText} normalizeEscapedNewlines />
      </div>

      {/* 3. Predefined Selectable Choice Cards */}
      <div className="space-y-2">
        {rawOptions.map((option, idx) => {
          const isSelected = isAnswered
            ? isOptionMatched(option.label, activeAnswer, isMultiple)
            : isMultiple
            ? selectedMultiple.has(option.label)
            : false;

          return (
            <QuestionOptionRow
              key={`${option.label}-${idx}`}
              option={option}
              index={idx}
              isSelected={isSelected}
              isAnswered={isAnswered}
              isSubmitting={isSubmitting}
              recommendedLabel={t('question.recommended')}
              selectedLabel={t('question.selected')}
              onSelect={handleSelectOption}
            />
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
              placeholder={t('question.customPlaceholder')}
              className="flex-1 bg-transparent border-none text-sm text-[#171717] dark:text-[#f4f4f5] placeholder-[#8f8f8f] dark:placeholder-[#52525b] focus:outline-none font-sans"
            />

            {customInput.trim().length > 0 && (
              <button
                type="submit"
                disabled={isSubmitting}
                className="px-2.5 py-1 rounded-md bg-[#171717] dark:bg-[#f4f4f5] hover:bg-black dark:hover:bg-white text-white dark:text-[#0b0b0e] text-xs font-medium flex items-center space-x-1 transition-colors cursor-pointer shadow-xs"
              >
                <span>{t('question.submit')}</span>
                <CornerDownLeft className="w-3 h-3 block shrink-0 ml-0.5" />
              </button>
            )}
          </form>
        ) : (
          isCustomAnswered && (
            <div className="p-3 rounded-md border border-[#171717] dark:border-[#f4f4f5] bg-[#f9fafb] dark:bg-[#17171b] flex items-start space-x-3">
              <div className="w-4 h-4 rounded-full border border-[#171717] dark:border-[#f4f4f5] bg-[#171717] dark:bg-[#f4f4f5] flex items-center justify-center shrink-0 mt-0.5">
                <div className="w-1.5 h-1.5 rounded-full bg-white dark:bg-[#111114]" />
              </div>
              <div className="flex-1 min-w-0">
                <span className="text-xs font-mono text-[#71717a] dark:text-[#71717a]">
                  {t('question.customAnswerLabel')}
                </span>
                <p className="text-sm font-medium text-[#171717] dark:text-[#f4f4f5] mt-0.5 break-words">
                  {formatInline(activeAnswer || '', 'ans-custom')}
                </p>
              </div>
              <span className="flex items-center space-x-1 text-xs text-[#15803d] dark:text-[#7ee787] font-mono">
                <Check className="w-3.5 h-3.5" />
                <span>{t('question.selected')}</span>
              </span>
            </div>
          )
        )}
      </div>

      {/* Multiple Choice Submit Action */}
      {isMultiple && !isAnswered && (
        <div className="mt-3.5 pt-3 border-t border-[#ebebeb] dark:border-[#212126] flex items-center justify-between">
          <span className="text-xs font-mono text-[#71717a] dark:text-[#a1a1aa] tabular-nums">
            {t('question.selectedCount', { count: selectedMultiple.size })}
          </span>
          <button
            type="button"
            disabled={selectedMultiple.size === 0 || isSubmitting}
            onClick={handleMultipleSubmit}
            className={`px-3 py-1.5 rounded-lg text-xs font-medium flex items-center space-x-1.5 transition ${
              selectedMultiple.size > 0
                ? 'bg-[#171717] dark:bg-[#f4f4f5] hover:bg-black dark:hover:bg-white text-white dark:text-[#0b0b0e] cursor-pointer active:scale-95'
                : 'bg-[#f4f4f5] dark:bg-[#232328] text-[#a1a1aa] dark:text-[#52525b] cursor-not-allowed'
            }`}
          >
            <span>{t('question.submitAnswer')}</span>
            <CornerDownLeft className="w-3 h-3 block shrink-0" />
          </button>
        </div>
      )}

      {/* Answered Footer Note */}
      {isAnswered && (
        <div className="mt-3 pt-2.5 border-t border-[#ebebeb] dark:border-[#212126] flex items-center justify-between text-xs font-mono text-[#71717a] dark:text-[#a1a1aa]">
          <div className="flex items-center space-x-1.5 truncate">
            <Lock className="w-3 h-3 block shrink-0 text-[#71717a]" />
            <span className="text-[#71717a]">{t('question.recordedAnswer')}</span>
            <span className="text-[#171717] dark:text-[#f4f4f5] font-medium truncate max-w-md">
              {activeAnswer}
            </span>
          </div>
          <span className="text-[10px] text-[#15803d] dark:text-emerald-400 font-mono">
            {t('question.agentResumed')}
          </span>
        </div>
      )}
    </div>
  );
}

const QuestionPromptCard = memo(QuestionPromptCardInner);
export default QuestionPromptCard;
