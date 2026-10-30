import { useState, useMemo } from 'react';
import type { DrawerActivityItem } from '../../../stores/drawerStore';
import { useTranslation } from '../../../stores/languageStore';
import {
  ChevronDown,
  ChevronRight,
  Bot,
  ListTodo,
  CheckCircle2,
  Circle,
  Clock,
} from 'lucide-react';
import { formatDurationCompact } from './ThoughtStreamCard';
import { tryExtractTodos } from '../../../lib/markdown';
import { MarkdownContent } from './MarkdownContent';

export interface AgentExplanationCardProps {
  activity: DrawerActivityItem;
  content?: string;
  durationMs?: number;
  hasNearbyTodoWrite?: boolean;
  onImageClick?: (src: string, alt?: string) => void;
}

export function AgentExplanationCard({
  activity,
  content,
  durationMs,
  hasNearbyTodoWrite,
  onImageClick,
}: AgentExplanationCardProps) {
  const [isExpanded, setIsExpanded] = useState(true);
  const { t } = useTranslation();

  const todoData = useMemo(() => {
    return content ? tryExtractTodos(content) : null;
  }, [content]);

  // If this explanation is purely raw JSON of todos and duplicates nearby todo_write tool, hide completely
  if (todoData && hasNearbyTodoWrite && !todoData.leadingText) {
    return null;
  }

  const effectiveLeadingText = todoData?.leadingText;
  const effectiveTodos = (!hasNearbyTodoWrite && todoData?.todos) ? todoData.todos : null;
  const effectiveDurationMs = durationMs ?? activity?.durationMs;
  const durationText = formatDurationCompact(effectiveDurationMs);

  return (
    <div className="rounded-lg border border-[#e4e4e7] dark:border-[#21262d] bg-white dark:bg-[#0d1117] shadow-xs overflow-hidden my-2.5 transition duration-150">
      <div
        onClick={() => setIsExpanded(!isExpanded)}
        className="flex items-center justify-between px-3.5 py-2.5 bg-[#f8f8f9] dark:bg-[#161b22] hover:bg-[#f4f4f5] dark:hover:bg-[#1c2128] cursor-pointer transition select-none border-l-2 border-l-[#171717] dark:border-l-[#58a6ff]"
      >
        <div className="flex items-center space-x-2 text-[#171717] dark:text-[#f0f6fc]">
          {isExpanded ? (
            <ChevronDown className="h-3.5 w-3.5 text-[#71717a] dark:text-[#8b949e] shrink-0" />
          ) : (
            <ChevronRight className="h-3.5 w-3.5 text-[#71717a] dark:text-[#8b949e] shrink-0" />
          )}
          {effectiveTodos ? (
            <ListTodo className="h-3.5 w-3.5 text-[#171717] dark:text-[#58a6ff] shrink-0" />
          ) : (
            <Bot className="h-3.5 w-3.5 text-[#171717] dark:text-[#58a6ff] shrink-0" />
          )}
          <span className="text-xs font-semibold text-[#171717] dark:text-[#f0f6fc] font-sans">
            {effectiveTodos ? t('drawer.actionPlan') : t('drawer.agentExplanation')}
          </span>
        </div>
        {durationText && (
          <span className="flex items-center space-x-1 text-[10px] font-mono text-[#71717a] dark:text-[#8b949e] tabular-nums">
            <Clock className="h-3 w-3 shrink-0 text-[#a1a1aa] dark:text-[#6e7681]" />
            <span>{durationText}</span>
          </span>
        )}
      </div>
      {isExpanded && (
        <div className="px-3.5 py-3 border-t border-[#f4f4f5] dark:border-[#21262d] text-sm leading-relaxed text-[#27272a] dark:text-[#d4d4d8] select-text space-y-2.5">
          {effectiveLeadingText && (
            <MarkdownContent content={effectiveLeadingText} onImageClick={onImageClick} />
          )}
          {effectiveTodos ? (
            <div className="space-y-1.5 pt-1">
              {effectiveTodos.map((item, i) => (
                <div
                  key={item.id || i}
                  className="flex items-start space-x-2 text-xs font-sans p-2 rounded-md bg-[#fafafa] dark:bg-[#161b22] border border-[#f0f0f0] dark:border-[#21262d]"
                >
                  <span className="mt-0.5 shrink-0">
                    {item.status === 'completed' ? (
                      <CheckCircle2 className="h-3.5 w-3.5 text-emerald-600 dark:text-emerald-400" />
                    ) : item.status === 'in_progress' ? (
                      <Clock className="h-3.5 w-3.5 text-amber-500 animate-pulse" />
                    ) : (
                      <Circle className="h-3.5 w-3.5 text-[#a1a1aa] dark:text-[#6e7681]" />
                    )}
                  </span>
                  <span
                    className={`flex-1 leading-normal ${
                      item.status === 'completed'
                        ? 'line-through text-[#71717a] dark:text-[#8b949e]'
                        : 'text-[#171717] dark:text-[#f0f6fc]'
                    }`}
                  >
                    {item.content}
                  </span>
                  <span className="text-[9px] font-mono uppercase px-1.5 py-0.2 rounded border shrink-0 text-[#71717a] dark:text-[#8b949e] border-[#e4e4e7] dark:border-[#30363d]">
                    {item.status === 'completed'
                      ? t('status.done')
                      : item.status === 'in_progress'
                      ? t('status.running')
                      : t('status.idle')}
                  </span>
                </div>
              ))}
            </div>
          ) : !effectiveLeadingText && content ? (
            <MarkdownContent content={content} onImageClick={onImageClick} />
          ) : !content && !effectiveLeadingText ? (
            <div className="text-xs text-[#8f8f8f] font-mono animate-pulse">
              {t('drawer.formingStep')}
            </div>
          ) : null}
        </div>
      )}
    </div>
  );
}
