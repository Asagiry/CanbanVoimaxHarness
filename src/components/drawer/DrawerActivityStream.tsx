import React, { useState, useRef, useEffect, useMemo, useCallback } from 'react';
import type { KanbanTask } from '../../types';
import type { DrawerActivityItem } from '../../stores/drawerStore';
import ToolExecutionCard from './ToolExecutionCard';
import QuestionPromptCard from './QuestionPromptCard';
import CompactedContextCard from './CompactedContextCard';
import ImageLightboxModal from './ImageLightboxModal';
import { useDrawerStore } from '../../stores/drawerStore';
import { useBoardStore } from '../../stores/boardStore';
import { useTranslation } from '../../stores/languageStore';
import {
  ArrowDown,
  ChevronDown,
  ChevronRight,
  CheckCheck,
  Check,
  AlertTriangle,
  Info,
  Target,
  Play,
  X,
  Bot,
  ListTodo,
  CheckCircle2,
  Circle,
  Clock,
  ZoomIn,
  Paperclip,
} from 'lucide-react';
import {
  ThoughtStreamCard,
  SteerDirectiveCard,
  SubagentActivityCard,
  formatDurationCompact,
} from './stream';
import {
  parseMarkdownTokens,
  parseInlineTokens,
  tryExtractTodos,
  type InlineNode,
} from '../../lib/markdown';

export const LightboxContext = React.createContext<((src: string, alt?: string) => void) | null>(null);

function renderInlineNodes(nodes: InlineNode[], prefix: string): React.ReactNode[] {
  return nodes.map((node, i) => {
    const key = `${prefix}-${i}`;
    switch (node.type) {
      case 'code':
        return (
          <code
            key={key}
            className="px-1.5 py-0.5 rounded bg-[#f4f4f5] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] font-mono text-[13px]"
          >
            {node.text}
          </code>
        );
      case 'bold':
        return (
          <strong key={key} className="font-semibold text-[#171717] dark:text-[#f0f6fc]">
            {renderInlineNodes(node.children, `${key}-b`)}
          </strong>
        );
      case 'italic':
        return (
          <em key={key} className="italic text-[#171717] dark:text-[#f0f6fc]">
            {renderInlineNodes(node.children, `${key}-i`)}
          </em>
        );
      case 'link': {
        const isDangerous = /^\s*javascript\s*:/i.test(node.url);
        return (
          <a
            key={key}
            href={isDangerous ? '#' : node.url}
            target="_blank"
            rel="noreferrer"
            className="text-[#7c3aed] dark:text-[#a78bfa] hover:underline"
          >
            {node.children.length > 0 ? renderInlineNodes(node.children, `${key}-l`) : node.text}
          </a>
        );
      }
      case 'text':
      default:
        return node.text;
    }
  });
}

function formatInline(text: string, prefix = 'inl'): React.ReactNode[] {
  if (!text) return [];
  const nodes = parseInlineTokens(text);
  return renderInlineNodes(nodes, prefix);
}

// Lightweight Markdown Renderer for streaming assistant output
export function MarkdownContent({
  content,
  onImageClick,
}: {
  content?: string;
  onImageClick?: (src: string, alt?: string) => void;
}) {
  const contextOnImageClick = React.useContext(LightboxContext);
  const handleImageClick = onImageClick || contextOnImageClick;

  const tokens = useMemo(() => parseMarkdownTokens(content ?? ''), [content]);

  return (
    <div className="space-y-1 select-text font-sans">
      {tokens.map((token, index) => {
        switch (token.type) {
          case 'code':
            return (
              <div
                key={`code-${index}`}
                className={
                  token.unfinished
                    ? 'my-2.5 rounded-md border border-[#ebebeb] dark:border-[#21262d] bg-[#fafafa] dark:bg-[#0d1117] overflow-hidden font-mono text-[13px]'
                    : 'my-2.5 rounded-md border border-[#ebebeb] dark:border-[#30363d] bg-[#fafafa] dark:bg-[#161b22] overflow-hidden font-mono text-[13px]'
                }
              >
                {token.language && (
                  <div
                    className={
                      token.unfinished
                        ? 'px-3 py-1 bg-[#f4f4f5] dark:bg-[#161b22] border-b border-[#ebebeb] dark:border-[#21262d] text-xs text-[#666666] dark:text-[#8b949e] font-medium flex justify-between items-center'
                        : 'px-3 py-1 bg-[#f4f4f5] dark:bg-[#0d1117] border-b border-[#ebebeb] dark:border-[#30363d] text-xs text-[#666666] dark:text-[#8b949e] font-medium flex justify-between items-center'
                    }
                  >
                    <span>{token.language}</span>
                  </div>
                )}
                <pre className="p-3 text-[#171717] dark:text-[#f0f6fc] overflow-x-auto leading-relaxed select-text font-mono text-[13px]">
                  {token.code}
                </pre>
              </div>
            );

          case 'table':
            return (
              <div
                key={`table-${index}`}
                className="my-2 overflow-x-auto rounded-md border border-[#ebebeb]"
              >
                <table className="w-full border-collapse text-[13px] font-mono tabular-nums">
                  <thead>
                    <tr>
                      {token.header.map((cell, k) => (
                        <th
                          key={k}
                          style={{ textAlign: token.aligns[k] }}
                          className="border border-[#ebebeb] dark:border-[#21262d] bg-[#f9f9f9] dark:bg-[#161b22] px-2 py-1.5 font-semibold text-[#171717] dark:text-[#f0f6fc]"
                        >
                          {formatInline(cell, `th-${index}-${k}`)}
                        </th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {token.rows.map((row, r) => (
                      <tr key={r}>
                        {row.map((cell, k) => (
                          <td
                            key={k}
                            style={{ textAlign: token.aligns[k] }}
                            className="border border-[#ebebeb] dark:border-[#21262d] px-2 py-1.5 text-[#171717] dark:text-[#f0f6fc]"
                          >
                            {formatInline(cell, `td-${index}-${r}-${k}`)}
                          </td>
                        ))}
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            );

          case 'hr':
            return <hr key={`hr-${index}`} className="border-[#ebebeb] dark:border-[#21262d] my-2.5" />;

          case 'blockquote':
            return (
              <blockquote
                key={`quote-${index}`}
                className="my-2 border-l-2 border-[#7c3aed]/50 bg-[#f9f9fb] dark:bg-[#161b22]/40 rounded-r-md px-3.5 py-2 text-sm italic text-[#666666] dark:text-[#8b949e] space-y-1"
              >
                {token.lines.map((ql, qIdx) => (
                  <div key={qIdx} className="leading-relaxed">
                    {ql.trim() === '' ? <div className="h-1" /> : formatInline(ql, `quote-${index}-${qIdx}`)}
                  </div>
                ))}
              </blockquote>
            );

          case 'image':
            return (
              <div
                key={`img-${index}`}
                onClick={() => handleImageClick?.(token.src, token.alt)}
                className="my-2 rounded-lg overflow-hidden border border-[#ebebeb] dark:border-[#21262d] max-w-lg bg-[#f4f4f5] dark:bg-[#161b22] shadow-xs group cursor-zoom-in"
              >
                <img
                  src={token.src}
                  alt={token.alt}
                  className="w-full max-h-80 object-contain cursor-zoom-in group-hover:scale-[1.01] transition-transform duration-200"
                />
              </div>
            );

          case 'heading': {
            if (token.level === 5) {
              return (
                <h5 key={`h5-${index}`} className="text-[13px] md:text-sm font-semibold text-[#171717] dark:text-[#f0f6fc] mt-2 mb-1">
                  {formatInline(token.text, `h5-${index}`)}
                </h5>
              );
            }
            if (token.level === 4) {
              return (
                <h4 key={`h4-${index}`} className="text-sm font-semibold text-[#171717] dark:text-[#f0f6fc] mt-2.5 mb-1">
                  {formatInline(token.text, `h4-${index}`)}
                </h4>
              );
            }
            if (token.level === 3) {
              return (
                <h3 key={`h3-${index}`} className="text-sm md:text-base font-semibold text-[#171717] dark:text-[#f0f6fc] mt-3 mb-1">
                  {formatInline(token.text, `h3-${index}`)}
                </h3>
              );
            }
            return (
              <h2 key={`h2-${index}`} className="text-base md:text-lg font-bold text-[#171717] dark:text-[#f0f6fc] mt-3.5 mb-1.5">
                {formatInline(token.text, `h2-${index}`)}
              </h2>
            );
          }

          case 'numbered': {
            const mlClass = token.indent >= 4 ? 'ml-8' : token.indent >= 2 ? 'ml-6' : 'ml-4';
            return (
              <div key={`num-${index}`} className={`${mlClass} text-sm leading-relaxed text-[#171717] dark:text-[#f0f6fc] flex items-start gap-1.5`}>
                <span className="font-semibold text-[#171717] dark:text-[#f0f6fc] select-none shrink-0 font-mono text-[13px]">
                  {token.number}
                </span>
                <span className="flex-1 min-w-0">{formatInline(token.text, `num-${index}`)}</span>
              </div>
            );
          }

          case 'task': {
            const mlClass = token.indent >= 4 ? 'ml-8' : token.indent >= 2 ? 'ml-6' : 'ml-4';
            return (
              <div
                key={`task-${index}`}
                className={`${mlClass} text-sm leading-relaxed flex items-start gap-2 my-0.5`}
              >
                {token.checked ? (
                  <span className="w-4 h-4 rounded bg-[#7c3aed]/15 dark:bg-[#7c3aed]/25 border border-[#7c3aed]/50 text-[#7c3aed] dark:text-[#a78bfa] flex items-center justify-center shrink-0 mt-0.5 select-none">
                    <Check className="w-2.5 h-2.5 stroke-[3]" />
                  </span>
                ) : (
                  <span className="w-4 h-4 rounded border border-[#d4d4d8] dark:border-[#30363d] bg-transparent shrink-0 mt-0.5 select-none" />
                )}
                <span
                  className={`flex-1 min-w-0 ${
                    token.checked ? 'line-through text-[#71717a] dark:text-[#8b949e]' : 'text-[#171717] dark:text-[#f0f6fc]'
                  }`}
                >
                  {formatInline(token.text, `task-${index}`)}
                </span>
              </div>
            );
          }

          case 'bullet': {
            const mlClass = token.indent >= 4 ? 'ml-8' : token.indent >= 2 ? 'ml-6' : 'ml-4';
            return (
              <div key={`bullet-${index}`} className={`${mlClass} text-sm leading-relaxed text-[#171717] dark:text-[#f0f6fc] flex items-start gap-2`}>
                <span className="text-[#666666] dark:text-[#8b949e] select-none shrink-0 leading-relaxed">•</span>
                <span className="flex-1 min-w-0">{formatInline(token.text, `bullet-${index}`)}</span>
              </div>
            );
          }

          case 'blank':
            return <div key={`blank-${index}`} className="h-1.5" />;

          case 'paragraph':
          default:
            return (
              <p key={`p-${index}`} className="text-sm leading-relaxed text-[#171717] dark:text-[#f0f6fc]">
                {formatInline(token.text, `p-${index}`)}
              </p>
            );
        }
      })}
    </div>
  );
}

// Clean prompt helper: strips legacy appended [USER STEER] or [USER]: blocks for pure prompt display
function getCleanPrompt(description?: string, title?: string): string {
  const raw = (description || title || '').trim();
  const steerIdx = raw.search(/\n\s*\[USER(?:\s+STEER)?\]:/i);
  if (steerIdx !== -1) {
    return raw.slice(0, steerIdx).trim();
  }
  return raw;
}

export interface DrawerActivityStreamProps {
  task: KanbanTask;
  unifiedActivities: DrawerActivityItem[];
  isRunning: boolean;
  runError: string | null;
  onRunAgent: () => void;
  onClearRunError: () => void;
  subtasks?: KanbanTask[];
  onOpenDrawer?: (taskId: string) => void;
}

function isFinalResponseItem(
  activity: DrawerActivityItem,
  idx: number,
  activities: DrawerActivityItem[],
  isRunning: boolean
): boolean {
  if (activity.type !== 'output') return false;

  // 1. Inspect subsequent items in the turn:
  let turnHasEnded = false;
  let hasInterveningToolsOrOutputs = false;

  for (let i = idx + 1; i < activities.length; i++) {
    const next = activities[i];
    // If next user message starts, the current turn ended
    if (next.type === 'user' || next.type === 'steer' || next.type === 'prompt') {
      turnHasEnded = true;
      break;
    }
    // If any tool call or another output occurs after this in the same turn,
    // this output was intermediate commentary!
    if (next.type === 'tool' || (next as any).tool || next.type === 'output') {
      hasInterveningToolsOrOutputs = true;
      break;
    }
  }

  // If followed by tools or outputs in the same turn, it is definitely intermediate
  if (hasInterveningToolsOrOutputs) {
    return false;
  }

  // If this turn is currently in-flight and the agent is still working,
  // it MUST NOT render as final response card until completion!
  if (!turnHasEnded && isRunning) {
    return false;
  }

  // If turn completed or agent finished without intervening tools,
  // this is the turn's true final response
  if (turnHasEnded || !isRunning) {
    return true;
  }

  return false;
}
interface AgentExplanationCardProps {
  activity: DrawerActivityItem;
  content?: string;
  durationMs?: number;
  hasNearbyTodoWrite?: boolean;
}

function AgentExplanationCard({
  activity,
  content,
  durationMs,
  hasNearbyTodoWrite,
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
            <MarkdownContent content={effectiveLeadingText} />
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
            <MarkdownContent content={content} />
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

export default function DrawerActivityStream({
  task,
  unifiedActivities,
  isRunning,
  runError,
  onRunAgent,
  onClearRunError,
  subtasks,
}: DrawerActivityStreamProps) {
  const {
    activeSubagentId,
    activeSubagentTitle,
    activeSubagentRole,
    activeSubagentPrompt,
  } = useDrawerStore();
  const { t } = useTranslation();

  const effectiveAttachments =
    task.attachments && task.attachments.length > 0
      ? task.attachments
      : task.parentId
        ? subtasks?.find((t) => t.id === task.parentId)?.attachments ||
          useBoardStore.getState().tasks.find((t) => t.id === task.parentId)?.attachments
        : undefined;

  const [collapsedOutputs, setCollapsedOutputs] = useState<Set<string>>(() => new Set());
  const [showNewButton, setShowNewButton] = useState(false);
  const isFirstBatchRef = useRef(true);
  const nearBottomRef = useRef(true);
  const scrollRef = useRef<HTMLDivElement>(null);

  // Image Lightbox state & handlers (§ 2.13 Viewport Overlay & Image Lightbox Invariant)
  const [lightboxImage, setLightboxImage] = useState<{ src: string; alt?: string } | null>(null);

  const handleOpenLightbox = useCallback((src: string, alt?: string) => {
    setLightboxImage({ src, alt });
  }, []);

  const handleContainerClick = useCallback((e: React.MouseEvent) => {
    const target = e.target as HTMLElement;
    if (target.tagName === 'IMG') {
      const img = target as HTMLImageElement;
      if (img.src && !img.closest('button') && !img.classList.contains('no-lightbox')) {
        e.preventDefault();
        e.stopPropagation();
        handleOpenLightbox(img.src, img.alt || '');
      }
    }
  }, [handleOpenLightbox]);

  // Obeying § 2.20 & Question Tool Dedup:
  // If unifiedActivities contains both a 'question' item and a duplicate 'tool' call for that question,
  // guarantee that EXACTLY ONE interactive QuestionPromptCard is rendered per question!
  const displayActivities = useMemo(() => {
    const list = unifiedActivities || [];
    const questionItems = list.filter((a) => a?.type === 'question');
    if (questionItems.length === 0) return list;

    const questionSignatures = new Set<string>();
    for (const q of questionItems) {
      if (q.id) questionSignatures.add(q.id);
      if (q.id?.startsWith('q-')) questionSignatures.add(q.id.replace(/^q-/, ''));
      const qParams = q.tool?.params as Record<string, unknown> | undefined;
      const qId = qParams?.questionId as string | undefined;
      if (qId) questionSignatures.add(qId);
      const callId = qParams?.callId as string | undefined;
      if (callId) questionSignatures.add(callId);
      if (q.tool?.id) questionSignatures.add(q.tool.id);
      if (q.content?.trim()) questionSignatures.add(`txt:${q.content.trim()}`);
      if (typeof qParams?.text === 'string' && qParams.text.trim()) {
        questionSignatures.add(`txt:${qParams.text.trim()}`);
      }
    }

    const seenQuestionIds = new Set<string>();

    return list.filter((activity) => {
      if (activity?.type === 'question') {
        const qParams = activity.tool?.params as Record<string, unknown> | undefined;
        const qId =
          (qParams?.questionId as string | undefined) ||
          activity.tool?.id ||
          activity.id;
        if (qId) {
          if (seenQuestionIds.has(qId)) return false;
          seenQuestionIds.add(qId);
        }
        return true;
      }

      if (
        activity?.type === 'tool' &&
        (activity.tool?.toolName === 'question' || activity.tool?.toolName === 'ask_user')
      ) {
        const tParams = activity.tool?.params as Record<string, unknown> | undefined;
        const tId = activity.tool?.id || activity.id;
        const qId = (tParams?.questionId as string | undefined) || '';
        const callId = (tParams?.callId as string | undefined) || '';
        const text = (
          (typeof tParams?.text === 'string' ? tParams.text : '') ||
          activity.content ||
          ''
        ).trim();

        if (
          (tId && (questionSignatures.has(tId) || questionSignatures.has(`q-${tId}`))) ||
          (qId && questionSignatures.has(qId)) ||
          (callId && questionSignatures.has(callId)) ||
          (text && questionSignatures.has(`txt:${text}`))
        ) {
          return false;
        }
      }
      return true;
    });
  }, [unifiedActivities]);

  // Pre-compute elapsed duration for each activity item
  const activityDurations = useMemo(() => {
    const list = displayActivities || [];
    const res: (number | undefined)[] = Array.from({ length: list.length });
    for (let i = 0; i < list.length; i++) {
      const act = list[i];
      if (act?.durationMs && act.durationMs > 0) {
        res[i] = act.durationMs;
      } else if (act?.tool?.durationMs && act.tool.durationMs > 0) {
        res[i] = act.tool.durationMs;
      } else {
        // Compute from timestamps between act and next activity
        const curTime = act?.timestamp ? new Date(act.timestamp).getTime() : NaN;
        if (!isNaN(curTime) && curTime > 0) {
          let nextTime = NaN;
          for (let j = i + 1; j < list.length; j++) {
            const t = list[j]?.timestamp ? new Date(list[j].timestamp).getTime() : NaN;
            if (!isNaN(t) && t > curTime) {
              nextTime = t;
              break;
            }
          }
          if (!isNaN(nextTime) && nextTime >= curTime) {
            const diff = nextTime - curTime;
            if (diff > 50 && diff < 3600000) {
              res[i] = diff;
            }
          } else if (i === list.length - 1 && isRunning) {
            const diff = Date.now() - curTime;
            if (diff > 50 && diff < 3600000) {
              res[i] = diff;
            }
          }
        }
      }
    }
    return res;
  }, [displayActivities, isRunning]);

  const toggleOutput = (id: string) => {
    setCollapsedOutputs((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  };

  const scrollToBottom = () => {
    const el = scrollRef.current;
    if (el) {
      el.scrollTop = el.scrollHeight;
    }
    nearBottomRef.current = true;
    setShowNewButton(false);
  };

  const handleFeedScroll = () => {
    const el = scrollRef.current;
    if (!el) return;
    const nearBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 120;
    nearBottomRef.current = nearBottom;
    if (nearBottom) {
      setShowNewButton(false);
    }
  };

  // Reset scroll and collapsed state on task switch
  useEffect(() => {
    setCollapsedOutputs(new Set());
    setShowNewButton(false);
    isFirstBatchRef.current = true;
    nearBottomRef.current = true;
  }, [task?.id]);

  // Smart auto-scroll: down only if user is already near bottom or initial batch
  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;
    if (isFirstBatchRef.current) {
      isFirstBatchRef.current = false;
      el.scrollTop = el.scrollHeight;
      nearBottomRef.current = true;
      setShowNewButton(false);
      return;
    }
    if (nearBottomRef.current) {
      el.scrollTop = el.scrollHeight;
      setShowNewButton(false);
    } else {
      setShowNewButton(true);
    }
  }, [displayActivities]);

  // Auto-scroll down during thinking/token streaming (with user scroll-up protection)
  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;
    const contentEl = el.firstElementChild as HTMLElement;
    if (!contentEl) return;
    const ro = new ResizeObserver(() => {
      if (nearBottomRef.current) {
        el.scrollTop = el.scrollHeight;
      }
    });
    ro.observe(contentEl);
    return () => ro.disconnect();
  }, []);

  return (
    <LightboxContext.Provider value={handleOpenLightbox}>
    <div className="relative flex-1 flex flex-col min-h-0 min-w-0 bg-[#fafafa] dark:bg-[#090a0f] overflow-hidden transition-colors">
      {/* Warning Banner on agent run error */}
      {runError && (
        <div className="mx-4 md:mx-6 mt-3 px-3.5 py-2.5 rounded-lg bg-[#fef2f2] dark:bg-[#3b1219] border border-[#fecaca] dark:border-[#da3633] text-[#991b1b] dark:text-[#ff7b72] flex items-center justify-between shadow-xs shrink-0 animate-in fade-in duration-150">
          <div className="flex items-center space-x-2.5 min-w-0 mr-3">
            <AlertTriangle className="h-4 w-4 text-[#dc2626] dark:text-[#ff7b72] shrink-0" />
            <div className="text-xs font-medium font-sans truncate">
              <span className="font-semibold">{t('drawer.executionError')}</span> {runError}
            </div>
          </div>
          <div className="flex items-center space-x-2 shrink-0">
            <button
              type="button"
              onClick={onRunAgent}
              className="flex items-center space-x-1.5 px-3 py-1 rounded-md bg-[#dc2626] dark:bg-[#da3633] hover:bg-[#b91c1c] text-white text-xs font-medium transition cursor-pointer shadow-xs active:scale-[0.98]"
            >
              <Play className="h-3 w-3 fill-current" />
              <span>{t('drawer.retryRun')}</span>
            </button>
            <button
              type="button"
              onClick={onClearRunError}
              className="p-1 text-[#dc2626] dark:text-[#ff7b72] hover:text-[#991b1b] dark:hover:text-[#ffa198] rounded transition cursor-pointer"
              aria-label={t('drawer.closeNotification')}
            >
              <X className="h-3.5 w-3.5" />
            </button>
          </div>
        </div>
      )}

      {/* Main Unified Activity Feed Scrollable Area */}
      <div
        ref={scrollRef}
        onScroll={handleFeedScroll}
        onClick={handleContainerClick}
        className="flex-1 min-h-0 overflow-y-auto p-4 md:p-6"
      >
        <div className="max-w-4xl mx-auto space-y-3.5">
          {activeSubagentId ? (
            /* Subagent Atomic Objective Card */
            <SubagentActivityCard
              activeSubagentId={activeSubagentId}
              activeSubagentTitle={activeSubagentTitle}
              activeSubagentRole={activeSubagentRole}
              activeSubagentPrompt={activeSubagentPrompt}
              effectiveAttachments={effectiveAttachments}
              onImageClick={handleOpenLightbox}
            />
          ) : (
            /* a) Task Goal / User Objective Card */
            <div className="rounded-lg border border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#0d1117] p-4 shadow-xs">
              <div className="flex items-center justify-between pb-2 mb-2.5 border-b border-[#ebebeb] dark:border-[#21262d]">
                <div className="flex items-center space-x-2">
                  <Target className="h-4 w-4 text-[#171717] dark:text-[#f0f6fc]" />
                  <span className="text-xs font-semibold text-[#171717] dark:text-[#f0f6fc] font-sans">
                    {t('drawer.taskObjective')}
                  </span>
                </div>
                <span className="text-[10px] font-mono text-[#8f8f8f] dark:text-[#8b949e]">
                  {t('drawer.taskObjective')}
                </span>
              </div>
              <div className="text-sm text-[#171717] dark:text-[#f0f6fc] leading-relaxed select-text font-sans">
                <MarkdownContent
                  content={getCleanPrompt(task.description, task.title)}
                  onImageClick={handleOpenLightbox}
                />
              </div>

              {/* Task Attachments Gallery */}
              {effectiveAttachments && effectiveAttachments.length > 0 && (
                <div className="mt-3 pt-3 border-t border-[#ebebeb] dark:border-[#21262d]">
                  <div className="text-[11px] font-mono font-medium text-[#666666] dark:text-[#8b949e] mb-2 flex items-center gap-1.5">
                    <Paperclip className="h-3 w-3" />
                    <span>{t('newTask.attachmentsLabel')} ({effectiveAttachments.length})</span>
                  </div>
                  <div className="flex flex-wrap gap-2">
                    {effectiveAttachments.map((att, idx) => (
                      <div
                        key={idx}
                        onClick={() => handleOpenLightbox(att, `Attachment ${idx + 1}`)}
                        className="relative group rounded-md overflow-hidden border border-[#ebebeb] dark:border-[#30363d] bg-[#fafafa] dark:bg-[#161b22] cursor-pointer hover:border-[#171717] dark:hover:border-[#f0f6fc] transition shadow-2xs"
                      >
                        <img
                          src={att}
                          alt={`Attachment ${idx + 1}`}
                          className="h-20 w-28 object-cover transition-transform group-hover:scale-105"
                        />
                        <div className="absolute inset-0 bg-black/0 group-hover:bg-black/20 flex items-center justify-center transition-opacity opacity-0 group-hover:opacity-100">
                          <ZoomIn className="h-4 w-4 text-white drop-shadow-md" />
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </div>
          )}

          {/* Unified Chronological Activity Stream */}
          {(displayActivities ?? []).map((activity, idx) => {
            const key = activity?.id ?? `activity-${idx}`;

            // e) Interactive Question Prompt Card (Human-in-the-Loop)
            if (
              activity?.type === 'question' ||
              (activity?.type === 'tool' &&
                (activity.tool?.toolName === 'question' || activity.tool?.toolName === 'ask_user'))
            ) {
              return (
                <div
                  key={key}
                  className="animate-silk-entrance motion-reduce:animate-none"
                >
                  <QuestionPromptCard
                    activity={activity}
                    taskId={task.id}
                  />
                </div>
              );
            }

            // Compacted context accordion card
            if (activity?.type === 'compacted') {
              return (
                <div
                  key={key}
                  className="animate-silk-entrance motion-reduce:animate-none"
                >
                  <CompactedContextCard
                    activity={activity}
                  />
                </div>
              );
            }

            // b) Live Streaming Thinking Block
            if (activity?.type === 'thought') {
              const isLastActivity = idx === ((displayActivities?.length ?? 1) - 1);
              return (
                <ThoughtStreamCard
                  key={key}
                  activity={activity}
                  isTaskRunning={isRunning}
                  isLive={isRunning && isLastActivity}
                  taskId={task.id}
                  durationMs={activityDurations[idx] || activity.durationMs}
                />
              );
            }

            // d) Inline Tool Execution Card (including subagent tool executions)
            if (activity?.type === 'tool' && activity?.tool) {
              const toolWithDuration = {
                ...activity.tool,
                durationMs: activity.tool.durationMs || activityDurations[idx] || 0,
              };
              return (
                <div
                  key={key}
                  className="animate-silk-entrance motion-reduce:animate-none"
                >
                  <ToolExecutionCard
                    tool={toolWithDuration}
                  />
                </div>
              );
            }

            // User steer message or Gatekeeper Sensor rejection
            if (activity?.type === 'user' || activity?.type === 'steer') {
              return (
                <SteerDirectiveCard
                  key={key}
                  activity={activity}
                  onImageClick={handleOpenLightbox}
                />
              );
            }

            // Subagent Activity / Report Item
            if ((activity?.type as string) === 'subagent') {
              return (
                <SubagentActivityCard
                  key={key}
                  activity={activity}
                  durationMs={activityDurations[idx] || activity.durationMs}
                  onImageClick={handleOpenLightbox}
                />
              );
            }

            // System notices
            if (activity?.type === 'system') {
              return (
                <div
                  key={key}
                  className="animate-silk-entrance motion-reduce:animate-none flex items-center space-x-2 rounded-md border border-[#ebebeb] dark:border-[#21262d] bg-[#f4f4f5] dark:bg-[#161b22] px-3 py-2 text-xs text-[#4d4d4d] dark:text-[#c9d1d9] font-mono shadow-xs"
                >
                  <Info className="h-3.5 w-3.5 text-[#666666] dark:text-[#8b949e] shrink-0" />
                  <span className="flex-1">{activity?.content ?? ''}</span>
                </div>
              );
            }

            // Error notices
            if (activity?.type === 'error') {
              return (
                <div
                  key={key}
                  className="animate-silk-entrance motion-reduce:animate-none flex items-start space-x-2 rounded-md border border-[#fecaca] dark:border-[#da3633] bg-[#fef2f2] dark:bg-[#3b1219] p-3 text-xs text-[#991b1b] dark:text-[#ff7b72] font-mono shadow-xs"
                >
                  <AlertTriangle className="h-4 w-4 text-[#dc2626] dark:text-[#ff7b72] shrink-0 mt-0.5" />
                  <div className="flex-1 whitespace-pre-wrap select-text">
                    {activity?.content ?? ''}
                  </div>
                </div>
              );
            }

            // c) Assistant Response: True Final Response vs Intermediate Step Commentary
            const isFinal = isFinalResponseItem(activity, idx, displayActivities ?? [], isRunning);

            const hasNearbyTodoWrite = (() => {
              const windowSize = 3;
              const start = Math.max(0, idx - windowSize);
              const end = Math.min((displayActivities?.length ?? 0) - 1, idx + windowSize);
              for (let i = start; i <= end; i++) {
                if (i === idx) continue;
                const act = displayActivities[i];
                if (act?.type === 'tool') {
                  const tName = act.tool?.toolName || (act.tool as any)?.name || (act.tool as any)?.tool;
                  if (tName === 'todo_write') return true;
                }
              }
              return false;
            })();

            // Suppress duplicate Agent Explanation cards when advance_task tool is nearby or duplicates summary (§ 2)
            const shouldSuppressExplanation = (() => {
              const explanationText = (activity?.content || '').trim();
              if (!explanationText) return true;
              const expLower = explanationText.toLowerCase();

              // Suppress repeated duplicate Agent Explanation cards if content is identical to previous agent message in stream
              for (let i = idx - 1; i >= 0; i--) {
                const act = displayActivities[i];
                if (act?.type === 'output' || !act?.type) {
                  const prevText = (act.content || '').trim();
                  if (prevText) {
                    if (prevText === explanationText || prevText.toLowerCase() === expLower) {
                      return true;
                    }
                    break;
                  }
                }
              }
              for (let i = idx - 1; i >= Math.max(0, idx - 5); i--) {
                const act = displayActivities[i];
                if (act?.type === 'output' || !act?.type) {
                  const prevText = (act.content || '').trim();
                  if (prevText && (prevText === explanationText || prevText.toLowerCase() === expLower)) {
                    return true;
                  }
                }
              }

              const windowSize = 2;
              const start = Math.max(0, idx - windowSize);
              const end = Math.min((displayActivities?.length ?? 0) - 1, idx + windowSize);

              for (let i = start; i <= end; i++) {
                if (i === idx) continue;
                const act = displayActivities[i];
                if (act?.type === 'tool') {
                  const tName = act.tool?.toolName || (act.tool as any)?.name || (act.tool as any)?.tool;
                  if (tName === 'advance_task' || tName === 'move_task' || tName === 'reject_task') {
                    const params = (act.tool as any)?.params || (act.tool as any)?.arguments || {};
                    const parsedParams = typeof params === 'string'
                      ? (() => { try { return JSON.parse(params); } catch { return {}; } })()
                      : params;
                    const summary = (parsedParams?.summary || parsedParams?.reason || '').trim().toLowerCase();

                    if (summary) {
                      if (
                        expLower === summary ||
                        expLower.includes(summary) ||
                        summary.includes(expLower)
                      ) {
                        return true;
                      }
                      const cleanExp = expLower.replace(/[\s\r\n\t.,;:!?-]+/g, ' ').trim();
                      const cleanSum = summary.replace(/[\s\r\n\t.,;:!?-]+/g, ' ').trim();
                      if (cleanExp === cleanSum || cleanExp.includes(cleanSum) || cleanSum.includes(cleanExp)) {
                        return true;
                      }
                    }
                    return true;
                  }
                }
              }

              for (const act of displayActivities) {
                if (act?.type === 'tool') {
                  const tName = act.tool?.toolName || (act.tool as any)?.name || (act.tool as any)?.tool;
                  if (tName === 'advance_task' || tName === 'move_task' || tName === 'reject_task') {
                    const params = (act.tool as any)?.params || (act.tool as any)?.arguments || {};
                    const parsedParams = typeof params === 'string'
                      ? (() => { try { return JSON.parse(params); } catch { return {}; } })()
                      : params;
                    const summary = (parsedParams?.summary || parsedParams?.reason || '').trim().toLowerCase();
                    if (summary && (expLower === summary || (summary.length > 20 && expLower.includes(summary)))) {
                      return true;
                    }
                  }
                }
              }

              return false;
            })();

            if (!isFinal) {
              if (shouldSuppressExplanation) {
                return null;
              }
              const explanationText = activity?.content;
              const todoData = explanationText ? tryExtractTodos(explanationText) : null;
              if (todoData && hasNearbyTodoWrite && !todoData.leadingText) {
                return null;
              }
              // Intermediate assistant commentary / step explanation rendered as collapsible chat card
              return (
                <div
                  key={key}
                  className="animate-silk-entrance motion-reduce:animate-none"
                >
                  <AgentExplanationCard
                    activity={activity}
                    content={activity?.content}
                    durationMs={activityDurations[idx] || activity?.durationMs}
                    hasNearbyTodoWrite={hasNearbyTodoWrite}
                  />
                </div>
              );
            }

            // True Final Response / Output Markdown Block (collapsible card)
            const isOutputExpanded = !collapsedOutputs.has(key);
            const outputDurationText = formatDurationCompact(activityDurations[idx] || activity?.durationMs);
            return (
              <div
                key={key}
                className="animate-silk-entrance motion-reduce:animate-none rounded-lg border border-[#e4e4e7] dark:border-[#21262d] bg-white dark:bg-[#0d1117] shadow-xs overflow-hidden my-2"
              >
                <div
                  onClick={() => toggleOutput(key)}
                  className="flex items-center justify-between px-4 py-3 hover:bg-[#fafafa] dark:hover:bg-[#161b22] cursor-pointer transition select-none"
                >
                  <div className="flex items-center space-x-2 text-[#171717] dark:text-[#f0f6fc]">
                    {isOutputExpanded ? (
                      <ChevronDown className="h-4 w-4 text-[#71717a] dark:text-[#8b949e] shrink-0" />
                    ) : (
                      <ChevronRight className="h-4 w-4 text-[#71717a] dark:text-[#8b949e] shrink-0" />
                    )}
                    <CheckCheck className="h-4 w-4 text-[#171717] dark:text-[#f0f6fc] shrink-0" />
                    <span className="text-sm font-semibold text-[#171717] dark:text-[#f0f6fc] font-sans">{t('drawer.agentResponse')}</span>
                  </div>
                  {outputDurationText && (
                    <span className="flex items-center space-x-1 text-xs font-mono text-[#71717a] dark:text-[#8b949e] tabular-nums">
                      <Clock className="h-3.5 w-3.5 shrink-0 text-[#a1a1aa] dark:text-[#6e7681]" />
                      <span>{outputDurationText}</span>
                    </span>
                  )}
                </div>
                {isOutputExpanded && (
                  <div className="px-4 pb-4 pt-3 border-t border-[#f4f4f5] dark:border-[#21262d]">
                    {activity?.content ? (
                      <MarkdownContent content={activity.content} />
                    ) : (
                      <div className="text-xs text-[#8f8f8f] font-mono">{t('drawer.generatingResponse')}</div>
                    )}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      </div>

      {/* Floating Scroll to Bottom Button */}
      {showNewButton && (
        <button
          type="button"
          onClick={scrollToBottom}
          className="absolute bottom-4 left-1/2 -translate-x-1/2 flex items-center space-x-1.5 px-3 py-1.5 rounded-md bg-[#171717] hover:bg-[#262626] text-white text-xs font-medium font-sans shadow-md transition cursor-pointer active:scale-[0.98]"
          aria-label={t('drawer.scrollToNewTitle')}
        >
          <ArrowDown className="h-3.5 w-3.5" />
          <span>{t('drawer.scrollToNew')}</span>
        </button>
      )}

      {/* Obeying § 2.13 Viewport Overlay & Image Lightbox Invariants */}
      {lightboxImage && (
        <ImageLightboxModal
          src={lightboxImage.src}
          alt={lightboxImage.alt}
          onClose={() => setLightboxImage(null)}
        />
      )}
    </div>
    </LightboxContext.Provider>
  );
}
