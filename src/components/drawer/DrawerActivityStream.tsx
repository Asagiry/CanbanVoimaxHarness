import React, { useState, useRef, useEffect, useMemo } from 'react';
import type { KanbanTask } from '../../types';
import type { DrawerActivityItem } from '../../stores/drawerStore';
import ToolExecutionCard, { getRoleBadgeClass } from './ToolExecutionCard';
import ThinkingCard from './ThinkingCard';
import QuestionPromptCard from './QuestionPromptCard';
import CompactedContextCard from './CompactedContextCard';
import { useDrawerStore } from '../../stores/drawerStore';
import {
  ArrowDown,
  ArrowLeft,
  ChevronDown,
  ChevronRight,
  Sparkles,
  User,
  AlertTriangle,
  Info,
  Target,
  Play,
  X,
  Bot,
} from 'lucide-react';

// Lightweight Markdown Renderer for streaming assistant output
export function MarkdownContent({ content }: { content?: string }) {
  const elements = useMemo(() => {
    const lines = (content ?? '').split('\n');
    const result: React.ReactNode[] = [];
    let inCodeBlock = false;
    let codeLanguage = '';
    let codeBuffer: string[] = [];

    const formatInline = (text: string): React.ReactNode[] => {
      // Inline code `code`
      const parts = text.split(/(`[^`]+`|\*\*[^*]+\*\*)/g);
      return parts.map((part, i) => {
        if (part.startsWith('`') && part.endsWith('`') && part.length >= 2) {
          return (
            <code
              key={i}
              className="px-1.5 py-0.5 rounded bg-[#f4f4f5] border border-[#ebebeb] text-[#171717] font-mono text-[13px]"
            >
              {part.slice(1, -1)}
            </code>
          );
        }
        if (part.startsWith('**') && part.endsWith('**') && part.length >= 4) {
          return (
            <strong key={i} className="font-semibold text-[#171717]">
              {part.slice(2, -2)}
            </strong>
          );
        }
        return part;
      });
    };

    const splitTableRow = (row: string): string[] => {
      let t = row.trim();
      if (t.startsWith('|')) t = t.slice(1);
      if (t.endsWith('|')) t = t.slice(0, -1);
      return t.split('|').map((c) => c.trim());
    };

    const parseDelimiterRow = (row: string): Array<'left' | 'center' | 'right'> | null => {
      const cells = splitTableRow(row);
      if (cells.length === 0) return null;
      const aligns: Array<'left' | 'center' | 'right'> = [];
      for (const c of cells) {
        if (!/^:?-+:?$/.test(c)) return null;
        if (c.startsWith(':') && c.endsWith(':') && c.length >= 3) aligns.push('center');
        else if (c.endsWith(':')) aligns.push('right');
        else aligns.push('left');
      }
      return aligns;
    };

    const isHrLine = (row: string): boolean => {
      const nospace = row.trim().replace(/\s+/g, '');
      return /^(-{3,}|\*{3,}|_{3,})$/.test(nospace);
    };

    for (let index = 0; index < lines.length; index++) {
      const line = lines[index];
      if (line.startsWith('```')) {
        if (!inCodeBlock) {
          inCodeBlock = true;
          codeLanguage = line.slice(3).trim();
          codeBuffer = [];
        } else {
          inCodeBlock = false;
          result.push(
            <div
              key={`code-${index}`}
              className="my-2.5 rounded-md border border-[#ebebeb] bg-[#fafafa] overflow-hidden font-mono text-[13px]"
            >
              {codeLanguage && (
                <div className="px-3 py-1 bg-[#f4f4f5] border-b border-[#ebebeb] text-xs text-[#666666] font-medium flex justify-between">
                  <span>{codeLanguage}</span>
                </div>
              )}
              <pre className="p-3 text-[#171717] overflow-x-auto leading-relaxed select-text font-mono text-[13px]">
                {codeBuffer.join('\n')}
              </pre>
            </div>
          );
        }
        continue;
      }

      if (inCodeBlock) {
        codeBuffer.push(line);
        continue;
      }

      // GFM table: header with `|` + delimiter row, then data rows with `|`
      if (line.includes('|') && index + 1 < lines.length) {
        const aligns = parseDelimiterRow(lines[index + 1]);
        if (aligns) {
          const headerCells = splitTableRow(line);
          const colCount = Math.max(headerCells.length, aligns.length);
          const normAligns: Array<'left' | 'center' | 'right'> = Array.from(
            { length: colCount },
            (_, k) => aligns[k] ?? 'left'
          );
          const normHeader: string[] = Array.from(
            { length: colCount },
            (_, k) => headerCells[k] ?? ''
          );
          const bodyRows: string[][] = [];
          let j = index + 2;
          while (j < lines.length && lines[j].includes('|') && lines[j].trim() !== '' && !lines[j].startsWith('```')) {
            const cells = splitTableRow(lines[j]);
            bodyRows.push(Array.from({ length: colCount }, (_, k) => cells[k] ?? ''));
            j++;
          }
          result.push(
            <div
              key={`table-${index}`}
              className="my-2 overflow-x-auto rounded-md border border-[#ebebeb]"
            >
              <table className="w-full border-collapse text-[13px] font-mono tabular-nums">
                <thead>
                  <tr>
                    {normHeader.map((cell, k) => (
                      <th
                        key={k}
                        style={{ textAlign: normAligns[k] }}
                        className="border border-[#ebebeb] bg-[#f9f9f9] px-2 py-1.5 font-semibold text-[#171717]"
                      >
                        {formatInline(cell)}
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {bodyRows.map((row, r) => (
                    <tr key={r}>
                      {row.map((cell, k) => (
                        <td
                          key={k}
                          style={{ textAlign: normAligns[k] }}
                          className="border border-[#ebebeb] px-2 py-1.5 text-[#171717]"
                        >
                          {formatInline(cell)}
                        </td>
                      ))}
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          );
          index = j - 1;
          continue;
        }
      }

      // Thematic break outside tables
      if (isHrLine(line)) {
        result.push(<hr key={index} className="border-[#ebebeb] my-2.5" />);
        continue;
      }

      // Image line: ![alt](url)
      const imgMatch = line.trim().match(/^!\[(.*?)\]\((.*?)\)$/);
      if (imgMatch) {
        result.push(
          <div key={`img-${index}`} className="my-2 rounded-lg overflow-hidden border border-[#ebebeb] max-w-lg bg-[#f4f4f5] shadow-xs">
            <img
              src={imgMatch[2]}
              alt={imgMatch[1] || 'Изображение'}
              className="w-full max-h-80 object-contain cursor-pointer"
              onClick={() => window.open(imgMatch[2], '_blank')}
            />
          </div>
        );
        continue;
      }

      // Headings
      if (line.startsWith('#### ')) {
        result.push(
          <h5 key={index} className="text-[13px] md:text-sm font-semibold text-[#171717] mt-2 mb-1">
            {formatInline(line.slice(5))}
          </h5>
        );
      } else if (line.startsWith('### ')) {
        result.push(
          <h4 key={index} className="text-sm font-semibold text-[#171717] mt-2.5 mb-1">
            {formatInline(line.slice(4))}
          </h4>
        );
      } else if (line.startsWith('## ')) {
        result.push(
          <h3 key={index} className="text-sm md:text-base font-semibold text-[#171717] mt-3 mb-1">
            {formatInline(line.slice(3))}
          </h3>
        );
      } else if (line.startsWith('# ')) {
        result.push(
          <h2 key={index} className="text-base md:text-lg font-bold text-[#171717] mt-3.5 mb-1.5">
            {formatInline(line.slice(2))}
          </h2>
        );
      } else if (line.startsWith('- ') || line.startsWith('* ')) {
        result.push(
          <li key={index} className="ml-4 list-disc text-sm leading-relaxed text-[#171717]">
            {formatInline(line.slice(2))}
          </li>
        );
      } else if (line.trim() === '') {
        result.push(<div key={index} className="h-1.5" />);
      } else {
        result.push(
          <p key={index} className="text-sm leading-relaxed text-[#171717]">
            {formatInline(line)}
          </p>
        );
      }
    }

    if (inCodeBlock && codeBuffer.length > 0) {
      result.push(
        <div
          key="code-unfinished"
          className="my-2.5 rounded-md border border-[#ebebeb] bg-[#fafafa] overflow-hidden font-mono text-[13px]"
        >
          <pre className="p-3 text-[#171717] overflow-x-auto leading-relaxed select-text font-mono text-[13px]">
            {codeBuffer.join('\n')}
          </pre>
        </div>
      );
    }

    return result;
  }, [content]);

  return <div className="space-y-1 select-text font-sans">{elements}</div>;
}

// Clean prompt helper: strips legacy appended [USER STEER] or [USER]: blocks for pure prompt display
export function getCleanPrompt(description?: string, title?: string): string {
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

export default function DrawerActivityStream({
  task,
  unifiedActivities,
  isRunning,
  runError,
  onRunAgent,
  onClearRunError,
  subtasks,
  onOpenDrawer,
}: DrawerActivityStreamProps) {
  const {
    activeSubagentId,
    activeSubagentTitle,
    activeSubagentRole,
    activeSubagentPrompt,
    closeSubagent,
  } = useDrawerStore();

  const [expandedOutputs, setExpandedOutputs] = useState<Set<string>>(() => new Set());
  const [showNewButton, setShowNewButton] = useState(false);
  const isFirstBatchRef = useRef(true);
  const nearBottomRef = useRef(true);
  const scrollRef = useRef<HTMLDivElement>(null);

  const toggleOutput = (id: string) => {
    setExpandedOutputs((prev) => {
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

  // Reset scroll and expanded state on task switch
  useEffect(() => {
    setExpandedOutputs(new Set());
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
  }, [unifiedActivities]);

  return (
    <div className="relative flex-1 flex flex-col min-h-0 min-w-0 bg-[#fafafa] overflow-hidden">
      {/* Warning Banner on agent run error */}
      {runError && (
        <div className="mx-4 md:mx-6 mt-3 px-3.5 py-2.5 rounded-lg bg-[#fef2f2] border border-[#fecaca] text-[#991b1b] flex items-center justify-between shadow-xs shrink-0 animate-in fade-in duration-150">
          <div className="flex items-center space-x-2.5 min-w-0 mr-3">
            <AlertTriangle className="h-4 w-4 text-[#dc2626] shrink-0" />
            <div className="text-xs font-medium font-sans truncate">
              <span className="font-semibold">Ошибка выполнения:</span> {runError}
            </div>
          </div>
          <div className="flex items-center space-x-2 shrink-0">
            <button
              type="button"
              onClick={onRunAgent}
              className="flex items-center space-x-1.5 px-3 py-1 rounded-md bg-[#dc2626] hover:bg-[#b91c1c] text-white text-xs font-medium transition cursor-pointer shadow-xs active:scale-[0.98]"
            >
              <Play className="h-3 w-3 fill-current" />
              <span>Повторить запуск</span>
            </button>
            <button
              type="button"
              onClick={onClearRunError}
              className="p-1 text-[#dc2626] hover:text-[#991b1b] rounded transition cursor-pointer"
              title="Закрыть уведомление"
            >
              <X className="h-3.5 w-3.5" />
            </button>
          </div>
        </div>
      )}

      {/* Main Unified Activity Feed Scrollable Area */}
      <div ref={scrollRef} onScroll={handleFeedScroll} className="flex-1 min-h-0 overflow-y-auto p-4 md:p-6">
        <div className="max-w-4xl mx-auto space-y-3.5">
          {activeSubagentId ? (
            /* Subagent Atomic Objective Card */
            <div className="rounded-lg border border-[#ebebeb] bg-white p-4 shadow-xs">
              <div className="flex items-center justify-between pb-2 mb-2.5 border-b border-[#ebebeb]">
                <div className="flex items-center space-x-2 min-w-0">
                  <span className="text-[#171717] text-xs font-mono shrink-0">■</span>
                  <span className="text-xs font-semibold text-[#171717] font-sans truncate">
                    Атомарная задача субагента: {activeSubagentTitle || 'Субагент'}
                  </span>
                </div>
                <div className="flex items-center space-x-2 shrink-0">
                  <span className={`px-1.5 py-0.5 rounded text-[9px] font-mono font-bold uppercase tracking-wider border ${getRoleBadgeClass(activeSubagentRole)}`}>
                    {(activeSubagentRole || 'general').toUpperCase()}
                  </span>
                  <button
                    type="button"
                    onClick={closeSubagent}
                    className="flex items-center space-x-1.5 px-2.5 py-1 rounded-md text-xs font-sans font-medium text-[#71717a] hover:text-[#171717] hover:bg-[#f4f4f5] border border-transparent hover:border-[#ebebeb] transition cursor-pointer"
                    title="Вернуться к родительской задаче"
                  >
                    <ArrowLeft className="h-3.5 w-3.5" />
                    <span>← К родительской задаче</span>
                  </button>
                </div>
              </div>
              <div className="text-sm text-[#171717] leading-relaxed whitespace-pre-wrap select-text font-sans bg-[#fafafa] p-3 rounded-md border border-[#ebebeb]">
                {activeSubagentPrompt || 'Инструкция для субагента'}
              </div>
            </div>
          ) : (
            <>
              {/* a) Task Goal / User Objective Card */}
              <div className="rounded-lg border border-[#ebebeb] bg-white p-4 shadow-xs">
                <div className="flex items-center justify-between pb-2 mb-2.5 border-b border-[#ebebeb]">
                  <div className="flex items-center space-x-2">
                    <Target className="h-4 w-4 text-[#171717]" />
                    <span className="text-xs font-semibold text-[#171717] font-sans">
                      Цель задачи / Prompt
                    </span>
                  </div>
                  <span className="text-[10px] font-mono text-[#8f8f8f]">
                    Постановка задачи
                  </span>
                </div>
                <div className="text-sm text-[#171717] leading-relaxed whitespace-pre-wrap select-text font-sans">
                  {getCleanPrompt(task.description, task.title)}
                </div>
              </div>

          {/* a2) Relay Handoff / Zero-Turn Memory Priming Card */}
          {task.handoverSummary && (
            <div className="rounded-lg border border-[#c7d2fe] bg-[#f5f7ff] p-4 shadow-xs">
              <div className="flex items-center justify-between pb-2 mb-2.5 border-b border-[#e0e7ff]">
                <div className="flex items-center space-x-2">
                  <Sparkles className="h-4 w-4 text-[#6366f1]" />
                  <span className="text-xs font-semibold text-[#4338ca] font-sans">
                    Эстафета пайплайна / Relay Handoff (Zero-Turn Memory Priming)
                  </span>
                </div>
                <span className="text-[10px] font-mono text-[#6366f1] bg-[#eef2ff] px-2 py-0.5 rounded border border-[#c7d2fe]">
                  Передано предыдущим этапом
                </span>
              </div>
              <div className="text-sm text-[#1e1b4b] leading-relaxed whitespace-pre-wrap select-text font-sans">
                {task.handoverSummary}
              </div>
            </div>
          )}

          {/* Subagent Child Tasks Cards (if any) */}
          {subtasks && subtasks.length > 0 && onOpenDrawer && (
            <div className="rounded-lg border border-[#e4e4e7] bg-white p-3 shadow-xs">
              <div className="flex items-center space-x-2 pb-2 mb-2 border-b border-[#ebebeb] text-xs font-semibold text-[#171717]">
                <Bot className="h-3.5 w-3.5 text-[#6366f1]" />
                <span>Дочерние подагенты ({subtasks.length})</span>
              </div>
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
                {subtasks.map((st) => (
                  <button
                    key={st.id}
                    type="button"
                    onClick={() => onOpenDrawer(st.id)}
                    className="flex items-center justify-between p-2 rounded-md border border-[#ebebeb] hover:border-[#d4d4d8] bg-[#f9fafb] hover:bg-[#f4f4f5] transition cursor-pointer text-left"
                  >
                    <div className="flex items-center space-x-2 min-w-0">
                      <span className="font-mono text-xs font-semibold text-[#171717]">{st.id}</span>
                      <span className="text-xs text-[#52525b] truncate">{st.title}</span>
                    </div>
                    <span className={`text-[10px] font-mono px-1.5 py-0.5 rounded shrink-0 ${
                      st.status === 'running'
                        ? 'bg-emerald-50 text-emerald-700 border border-emerald-200'
                        : st.status === 'done'
                        ? 'bg-blue-50 text-blue-700 border border-blue-200'
                        : 'bg-zinc-100 text-zinc-600'
                    }`}>
                      {st.status}
                    </span>
                  </button>
                ))}
              </div>
            </div>
          )}
          </>
        )}

          {/* Unified Chronological Activity Stream */}
          {(unifiedActivities ?? []).map((activity, idx) => {
            const key = activity?.id ?? `activity-${idx}`;

            // e) Interactive Question Prompt Card (Human-in-the-Loop)
            if (
              activity?.type === 'question' ||
              (activity?.type === 'tool' &&
                (activity.tool?.toolName === 'question' || activity.tool?.toolName === 'ask_user'))
            ) {
              return (
                <QuestionPromptCard
                  key={key}
                  activity={activity}
                  taskId={task.id}
                />
              );
            }

            // Compacted context accordion card
            if (activity?.type === 'compacted') {
              return (
                <CompactedContextCard
                  key={key}
                  activity={activity}
                />
              );
            }

            // b) Live Streaming Thinking Block
            if (activity?.type === 'thought') {
              const isLastActivity = idx === ((unifiedActivities?.length ?? 1) - 1);
              return (
                <ThinkingCard
                  key={key}
                  thought={activity}
                  isTaskRunning={isRunning}
                  isLive={isRunning && isLastActivity}
                  taskId={task.id}
                />
              );
            }

            // d) Inline Tool Execution Card (including subagent tool executions)
            if (activity?.type === 'tool' && activity?.tool) {
              return (
                <ToolExecutionCard
                  key={key}
                  tool={activity.tool}
                />
              );
            }

            // User steer message
            if (activity?.type === 'user' || activity?.type === 'steer') {
              const images = Array.isArray(activity?.images) ? activity.images : [];
              return (
                <div
                  key={key}
                  className="rounded-lg border border-[#ebebeb] bg-[#f9f9f9] p-3.5 shadow-xs"
                >
                  <div className="flex items-center justify-between pb-1.5 mb-2 border-b border-[#ebebeb] text-[10px] font-mono text-[#666666]">
                    <div className="flex items-center space-x-1.5 text-[#171717] font-medium">
                      <User className="h-3 w-3" />
                      <span>Указание пользователя (Steer)</span>
                    </div>
                    <span className="tabular-nums">{activity?.timestamp ?? ''}</span>
                  </div>
                  {images.length > 0 && (
                    <div className="flex items-center gap-2 mb-2.5 flex-wrap">
                      {images.map((imgUrl, i) => (
                        <div
                          key={i}
                          className="relative rounded-lg overflow-hidden border border-[#ebebeb] max-w-[240px] max-h-40 bg-[#f4f4f5] shadow-xs"
                        >
                          <img
                            src={imgUrl}
                            alt={`Прикреплённое изображение ${i + 1}`}
                            className="w-full h-full max-h-40 object-cover hover:scale-105 transition-transform duration-150 cursor-pointer"
                            onClick={() => window.open(imgUrl, '_blank')}
                          />
                        </div>
                      ))}
                    </div>
                  )}
                  <p className="text-sm text-[#171717] whitespace-pre-wrap select-text leading-relaxed font-sans">
                    {activity?.content ?? ''}
                  </p>
                </div>
              );
            }

            // System notices
            if (activity?.type === 'system') {
              return (
                <div
                  key={key}
                  className="flex items-center space-x-2 rounded-md border border-[#ebebeb] bg-[#f4f4f5] px-3 py-2 text-xs text-[#4d4d4d] font-mono shadow-xs"
                >
                  <Info className="h-3.5 w-3.5 text-[#666666] shrink-0" />
                  <span className="flex-1">{activity?.content ?? ''}</span>
                  <span className="text-[10px] text-[#8f8f8f] tabular-nums">
                    {activity?.timestamp ?? ''}
                  </span>
                </div>
              );
            }

            // Error notices
            if (activity?.type === 'error') {
              return (
                <div
                  key={key}
                  className="flex items-start space-x-2 rounded-md border border-[#fecaca] bg-[#fef2f2] p-3 text-xs text-[#991b1b] font-mono shadow-xs"
                >
                  <AlertTriangle className="h-4 w-4 text-[#dc2626] shrink-0 mt-0.5" />
                  <div className="flex-1 whitespace-pre-wrap select-text">
                    {activity?.content ?? ''}
                  </div>
                  <span className="text-[10px] text-[#b91c1c] tabular-nums shrink-0">
                    {activity?.timestamp ?? ''}
                  </span>
                </div>
              );
            }

            // c) Assistant Response / Output Markdown Block (collapsible, collapsed by default)
            const isOutputExpanded = expandedOutputs.has(key);
            return (
              <div
                key={key}
                className="rounded-lg border border-[#e4e4e7] bg-white shadow-xs overflow-hidden my-2"
              >
                <div
                  onClick={() => toggleOutput(key)}
                  className="flex items-center justify-between px-4 py-3 hover:bg-[#fafafa] cursor-pointer transition select-none"
                >
                  <div className="flex items-center space-x-2 text-[#171717]">
                    {isOutputExpanded ? (
                      <ChevronDown className="h-4 w-4 text-[#71717a] shrink-0" />
                    ) : (
                      <ChevronRight className="h-4 w-4 text-[#71717a] shrink-0" />
                    )}
                    <Sparkles className="h-4 w-4 text-[#171717] shrink-0" />
                    <span className="text-sm font-semibold text-[#171717] font-sans">Ответ агента</span>
                  </div>
                  <span className="text-xs font-mono text-[#71717a] tabular-nums">{activity?.timestamp ?? ''}</span>
                </div>
                {isOutputExpanded && (
                  <div className="px-4 pb-4 pt-3 border-t border-[#f4f4f5]">
                    {activity?.content ? (
                      <MarkdownContent content={activity.content} />
                    ) : (
                      <div className="text-xs text-[#8f8f8f] font-mono">Генерация ответа...</div>
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
          title="Перейти к новым сообщениям"
        >
          <ArrowDown className="h-3.5 w-3.5" />
          <span>К новому</span>
        </button>
      )}
    </div>
  );
}
