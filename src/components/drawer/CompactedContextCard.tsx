import { useState, useRef, useEffect, memo } from 'react';
import type { DrawerActivityItem } from '../../stores/drawerStore';
import { MarkdownContent } from './stream';
import { useTranslation } from '../../stores/languageStore';
import {
  ChevronRight,
  ChevronDown,
  Copy,
  Check,
  Minimize2,
} from 'lucide-react';

interface CompactedContextCardProps {
  activity: DrawerActivityItem;
}

function CompactedContextCardInner({ activity }: CompactedContextCardProps) {
  const { t, language } = useTranslation();
  const [isExpanded, setIsExpanded] = useState(false);
  const [copied, setCopied] = useState(false);
  const copiedTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (copiedTimeoutRef.current) {
        clearTimeout(copiedTimeoutRef.current);
      }
    };
  }, []);

  // Extract metadata from activity.tool if present
  const toolData =
    activity.tool && typeof activity.tool === 'object'
      ? (activity.tool as unknown as Record<string, unknown>)
      : undefined;
  const count = typeof toolData?.compactedCount === 'number' ? toolData.compactedCount : 8;
  const savedTokens =
    typeof toolData?.savedTokens === 'number' || typeof toolData?.savedTokens === 'string'
      ? toolData.savedTokens
      : 2450;
  const compactorModel = typeof toolData?.compactorModel === 'string' ? toolData.compactorModel : undefined;
  const locale = language === 'ru' ? 'ru-RU' : 'en-US';
  const formattedTokens = typeof savedTokens === 'number'
    ? savedTokens.toLocaleString(locale)
    : String(savedTokens);

  const pluralMsg =
    language === 'ru'
      ? (() => {
          const mod10 = Math.abs(count) % 10;
          const mod100 = Math.abs(count) % 100;
          if (mod100 >= 11 && mod100 <= 19) return 'сообщений';
          if (mod10 === 1) return 'сообщение';
          if (mod10 >= 2 && mod10 <= 4) return 'сообщения';
          return 'сообщений';
        })()
      : count === 1
      ? 'message'
      : 'messages';

  const handleCopy = () => {
    if (activity.content) {
      navigator.clipboard?.writeText(activity.content).catch((err) => {
        console.debug('[CompactedContextCard] clipboard write failed:', err);
      });
      setCopied(true);
      if (copiedTimeoutRef.current) {
        clearTimeout(copiedTimeoutRef.current);
      }
      copiedTimeoutRef.current = setTimeout(() => {
        setCopied(false);
        copiedTimeoutRef.current = null;
      }, 2000);
    }
  };

  return (
    <div className="my-2 rounded-md border border-[#ebebeb] dark:border-[#212126] bg-white dark:bg-[#111114] overflow-hidden shadow-xs transition-all">
      {/* Collapsed State: Clean hairline paper strip */}
      <div
        className={`flex items-center justify-between px-3.5 py-2.5 select-none transition-colors ${
          isExpanded ? 'bg-[#fafafa] dark:bg-[#17171b] border-b border-[#ebebeb] dark:border-[#212126]' : 'hover:bg-[#f4f4f5] dark:hover:bg-[#17171b]'
        }`}
      >
        <button
          type="button"
          onClick={() => setIsExpanded(!isExpanded)}
          aria-expanded={isExpanded}
          className="flex items-center space-x-2 text-[#0d0d0d] dark:text-[#f4f4f5] min-w-0 flex-1 text-left cursor-pointer"
        >
          {isExpanded ? (
            <ChevronDown className="h-3.5 w-3.5 text-[#4d4d4d] dark:text-[#71717a] shrink-0 transition-transform duration-150" />
          ) : (
            <ChevronRight className="h-3.5 w-3.5 text-[#4d4d4d] dark:text-[#71717a] shrink-0 transition-transform duration-150" />
          )}

          <div className="flex items-center space-x-2 text-xs font-medium font-sans truncate">
            <span className="text-[#0d0d0d] dark:text-[#f4f4f5] font-semibold">
              {t('drawer.contextCompacted')}
            </span>
            <span className="text-[#8f8f8f] dark:text-[#52525b]">·</span>
            <span className="text-[#4d4d4d] dark:text-[#71717a]">
              {language === 'ru' ? 'Сжато' : 'Compacted'}{' '}
              <span className="font-mono tabular-nums text-[#0d0d0d] dark:text-[#f4f4f5]">{count}</span>{' '}
              {pluralMsg}
            </span>
            <span className="text-[#8f8f8f] dark:text-[#52525b]">·</span>
            <span className="font-mono tabular-nums text-[#15803d] dark:text-[#7ee787] text-[11px] bg-[#ecfdf5] dark:bg-[#12281e] px-1.5 py-0.5 rounded border border-[#a7f3d0] dark:border-[#238636] font-medium">
              {t('drawer.freedTokens', { tokens: formattedTokens })}
            </span>
          </div>
        </button>

        <div className="flex items-center space-x-2 shrink-0 ml-2">
          {isExpanded && (
            <button
              type="button"
              onClick={handleCopy}
              className="flex items-center space-x-1 px-2 py-0.5 text-[10px] font-mono text-[#666666] dark:text-[#71717a] hover:text-[#0d0d0d] dark:hover:text-[#f4f4f5] bg-white dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] hover:bg-[#f4f4f5] dark:hover:bg-[#232328] rounded transition cursor-pointer"
              aria-label="Скопировать отчет сжатия"
            >
              {copied ? (
                <>
                  <Check className="h-3 w-3 text-emerald-600 dark:text-emerald-400" />
                  <span className="text-emerald-600 dark:text-emerald-400">Скопировано</span>
                </>
              ) : (
                <>
                  <Copy className="h-3 w-3 text-[#666666] dark:text-[#71717a]" />
                  <span>Копия</span>
                </>
              )}
            </button>
          )}

          {activity.durationMs ? (
            <span className="text-[10px] font-mono text-[#8f8f8f] dark:text-[#71717a] tabular-nums">
              {activity.durationMs < 1000 ? `${Math.round(activity.durationMs)}ms` : `${(activity.durationMs / 1000).toFixed(1)}s`}
            </span>
          ) : null}
        </div>
      </div>

      {/* Expanded State: Smooth slide-down showing structured Markdown report */}
      {isExpanded && (
        <div className="p-4 bg-white dark:bg-[#111114] text-[#0d0d0d] dark:text-[#f4f4f5] font-sans text-xs leading-relaxed animate-in fade-in slide-in-from-top-1 duration-150">
          <div className="mb-3 pb-2 border-b border-[#ebebeb] dark:border-[#212126] flex items-center justify-between">
            <div className="flex items-center space-x-1.5 text-[11px] font-mono text-[#666666] dark:text-[#71717a]">
              <Minimize2 className="h-3.5 w-3.5 text-[#0d0d0d] dark:text-[#f4f4f5]" />
              <span className="font-semibold text-[#0d0d0d] dark:text-[#f4f4f5]">Сводка сжатия контекста</span>
            </div>
            {compactorModel && (
              <span className="text-[10px] font-mono text-[#8f8f8f] dark:text-[#71717a] bg-[#f4f4f5] dark:bg-[#17171b] px-1.5 py-0.5 rounded border border-[#ebebeb] dark:border-[#2c2c33]">
                model: {compactorModel}
              </span>
            )}
          </div>

          {activity.content ? (
            <div className="prose-xs space-y-2 select-text">
              <MarkdownContent content={activity.content} />
            </div>
          ) : (
            <div className="text-xs font-mono text-[#8f8f8f] italic">
              Отчет контекста отсутствует или пуст.
            </div>
          )}

          <div className="mt-4 pt-2.5 border-t border-[#ebebeb] dark:border-[#212126] flex items-center justify-between text-[11px] font-mono text-[#8f8f8f] dark:text-[#71717a]">
            <span>Контекст оптимизирован</span>
            <button
              type="button"
              onClick={() => setIsExpanded(false)}
              className="text-[#4d4d4d] hover:text-[#0d0d0d] hover:underline cursor-pointer"
            >
              Свернуть
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

const CompactedContextCard = memo(CompactedContextCardInner);
export default CompactedContextCard;
