import React, { useState, useRef, useEffect, useMemo, useCallback } from 'react';
import type { KanbanTask } from '../../types';
import type { DrawerActivityItem } from '../../stores/drawerStore';
import ToolExecutionCard from './ToolExecutionCard';
import ThinkingCard, { formatDurationCompact } from './ThinkingCard';
import QuestionPromptCard from './QuestionPromptCard';
import CompactedContextCard from './CompactedContextCard';
import ImageLightboxModal from './ImageLightboxModal';
import { useDrawerStore } from '../../stores/drawerStore';
import { useCortexStore } from '../../stores/cortexStore';
import { useTranslation } from '../../stores/languageStore';
import AgentIcon from '../common/AgentIcon';
import { getRoleBadgeStyle } from '../../lib/agentPalette';
import {
  ArrowDown,
  ChevronDown,
  ChevronRight,
  CornerDownRight,
  CheckCheck,
  User,
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
} from 'lucide-react';
import { formatSubagentDisplayTitle } from './SubagentsSidebar';

export const LightboxContext = React.createContext<((src: string, alt?: string) => void) | null>(null);

// Inline Markdown Parser Types & Helpers
interface InlineTextNode {
  type: 'text';
  text: string;
}
interface InlineCodeNode {
  type: 'code';
  text: string;
}
interface InlineBoldNode {
  type: 'bold';
  children: InlineNode[];
}
interface InlineItalicNode {
  type: 'italic';
  children: InlineNode[];
}
type InlineNode = InlineTextNode | InlineCodeNode | InlineBoldNode | InlineItalicNode;

function isValidInlineCode(code: string): boolean {
  if (!code || code.length > 120) return false;
  if (/[\r\n]/.test(code)) return false;
  // Suspicious if it spans sentence boundaries:
  if (/[.!?]\s+/.test(code)) return false;
  // Disallow inner backticks:
  if (code.includes('`')) return false;
  // Conversational Cyrillic prose (multiple Cyrillic words without quotes):
  if (/[а-яА-ЯёЁ]{2,}\s+[а-яА-ЯёЁ]{2,}/.test(code) && !/^["'].*["']$/.test(code.trim())) {
    return false;
  }
  return true;
}

function parseInlineTokens(text: string, depth = 0): InlineNode[] {
  const nodes: InlineNode[] = [];
  let i = 0;
  let textBuf = '';

  const flushText = () => {
    if (textBuf) {
      nodes.push({ type: 'text', text: textBuf });
      textBuf = '';
    }
  };

  while (i < text.length) {
    // 1. Inline code `code`
    if (text[i] === '`') {
      const closeIdx = text.indexOf('`', i + 1);
      if (closeIdx !== -1) {
        const candidate = text.slice(i + 1, closeIdx);
        if (isValidInlineCode(candidate)) {
          flushText();
          nodes.push({ type: 'code', text: candidate });
          i = closeIdx + 1;
          continue;
        }
      }
      // Stray or invalid backtick: treat as plain text
      textBuf += text[i];
      i++;
      continue;
    }

    // 2. Bold: **text** or __text__
    if (depth < 2 && text.startsWith('**', i)) {
      const closeIdx = text.indexOf('**', i + 2);
      if (closeIdx !== -1) {
        const candidate = text.slice(i + 2, closeIdx);
        if (candidate.length > 0 && candidate.length <= 300 && !candidate.startsWith(' ') && !candidate.endsWith(' ')) {
          flushText();
          nodes.push({ type: 'bold', children: parseInlineTokens(candidate, depth + 1) });
          i = closeIdx + 2;
          continue;
        }
      }
    }

    if (depth < 2 && text.startsWith('__', i)) {
      const prevChar = i > 0 ? text[i - 1] : ' ';
      if (!/\w/.test(prevChar)) {
        const closeIdx = text.indexOf('__', i + 2);
        if (closeIdx !== -1) {
          const nextChar = closeIdx + 2 < text.length ? text[closeIdx + 2] : ' ';
          if (!/\w/.test(nextChar)) {
            const candidate = text.slice(i + 2, closeIdx);
            if (candidate.length > 0 && candidate.length <= 300 && !candidate.startsWith(' ') && !candidate.endsWith(' ')) {
              flushText();
              nodes.push({ type: 'bold', children: parseInlineTokens(candidate, depth + 1) });
              i = closeIdx + 2;
              continue;
            }
          }
        }
      }
    }

    // 3. Italic: *text* or _text_
    if (depth < 2 && text[i] === '*' && !text.startsWith('**', i)) {
      const closeIdx = text.indexOf('*', i + 1);
      if (closeIdx !== -1 && !text.startsWith('**', closeIdx)) {
        const candidate = text.slice(i + 1, closeIdx);
        if (candidate.length > 0 && candidate.length <= 200 && !candidate.startsWith(' ') && !candidate.endsWith(' ')) {
          flushText();
          nodes.push({ type: 'italic', children: parseInlineTokens(candidate, depth + 1) });
          i = closeIdx + 1;
          continue;
        }
      }
    }

    if (depth < 2 && text[i] === '_' && !text.startsWith('__', i)) {
      const prevChar = i > 0 ? text[i - 1] : ' ';
      if (!/\w/.test(prevChar)) {
        const closeIdx = text.indexOf('_', i + 1);
        if (closeIdx !== -1 && !text.startsWith('__', closeIdx)) {
          const nextChar = closeIdx + 1 < text.length ? text[closeIdx + 1] : ' ';
          if (!/\w/.test(nextChar)) {
            const candidate = text.slice(i + 1, closeIdx);
            if (candidate.length > 0 && candidate.length <= 200 && !candidate.startsWith(' ') && !candidate.endsWith(' ')) {
              flushText();
              nodes.push({ type: 'italic', children: parseInlineTokens(candidate, depth + 1) });
              i = closeIdx + 1;
              continue;
            }
          }
        }
      }
    }

    textBuf += text[i];
    i++;
  }

  flushText();
  return nodes;
}

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

  const elements = useMemo(() => {
    const cleanContent = (content ?? '').replace(/\r\n/g, '\n').replace(/\r/g, '\n');
    const lines = cleanContent.split('\n');
    const result: React.ReactNode[] = [];
    let inCodeBlock = false;
    let codeLanguage = '';
    let codeBuffer: string[] = [];

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

    const bulletRegex = /^(\s*)([-*+•]|\u2022)\s+(.*)$/;
    const numberedRegex = /^(\s*)(\d+(\.\d+)*[\.\)])\s+(.*)$/;

    for (let index = 0; index < lines.length; index++) {
      const line = lines[index];
      const trimmedLine = line.trim();

      // Code block start / end (supports indented fences)
      if (trimmedLine.startsWith('```')) {
        if (!inCodeBlock) {
          inCodeBlock = true;
          codeLanguage = trimmedLine.slice(3).trim();
          codeBuffer = [];
        } else {
          inCodeBlock = false;
          result.push(
            <div
              key={`code-${index}`}
              className="my-2.5 rounded-md border border-[#ebebeb] dark:border-[#30363d] bg-[#fafafa] dark:bg-[#161b22] overflow-hidden font-mono text-[13px]"
            >
              {codeLanguage && (
                <div className="px-3 py-1 bg-[#f4f4f5] dark:bg-[#0d1117] border-b border-[#ebebeb] dark:border-[#30363d] text-xs text-[#666666] dark:text-[#8b949e] font-medium flex justify-between items-center">
                  <span>{codeLanguage}</span>
                </div>
              )}
              <pre className="p-3 text-[#171717] dark:text-[#f0f6fc] overflow-x-auto leading-relaxed select-text font-mono text-[13px]">
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
          while (j < lines.length && lines[j].includes('|') && lines[j].trim() !== '' && !lines[j].trim().startsWith('```')) {
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
                        className="border border-[#ebebeb] dark:border-[#21262d] bg-[#f9f9f9] dark:bg-[#161b22] px-2 py-1.5 font-semibold text-[#171717] dark:text-[#f0f6fc]"
                      >
                        {formatInline(cell, `th-${index}-${k}`)}
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
          index = j - 1;
          continue;
        }
      }

      // Thematic break outside tables
      if (isHrLine(line)) {
        result.push(<hr key={index} className="border-[#ebebeb] dark:border-[#21262d] my-2.5" />);
        continue;
      }

      // Image line: ![alt](url)
      const imgMatch = line.trim().match(/^!\[(.*?)\]\((.*?)\)$/);
      if (imgMatch) {
        const imgSrc = imgMatch[2];
        const imgAlt = imgMatch[1] || 'Image';
        result.push(
          <div
            key={`img-${index}`}
            onClick={() => handleImageClick?.(imgSrc, imgAlt)}
            className="my-2 rounded-lg overflow-hidden border border-[#ebebeb] dark:border-[#21262d] max-w-lg bg-[#f4f4f5] dark:bg-[#161b22] shadow-xs group cursor-zoom-in"
          >
            <img
              src={imgSrc}
              alt={imgAlt}
              className="w-full max-h-80 object-contain cursor-zoom-in group-hover:scale-[1.01] transition-transform duration-200"
            />
          </div>
        );
        continue;
      }

      // Headings
      if (trimmedLine.startsWith('#### ')) {
        result.push(
          <h5 key={index} className="text-[13px] md:text-sm font-semibold text-[#171717] dark:text-[#f0f6fc] mt-2 mb-1">
            {formatInline(trimmedLine.slice(5), `h5-${index}`)}
          </h5>
        );
      } else if (trimmedLine.startsWith('### ')) {
        result.push(
          <h4 key={index} className="text-sm font-semibold text-[#171717] dark:text-[#f0f6fc] mt-2.5 mb-1">
            {formatInline(trimmedLine.slice(4), `h4-${index}`)}
          </h4>
        );
      } else if (trimmedLine.startsWith('## ')) {
        result.push(
          <h3 key={index} className="text-sm md:text-base font-semibold text-[#171717] dark:text-[#f0f6fc] mt-3 mb-1">
            {formatInline(trimmedLine.slice(3), `h3-${index}`)}
          </h3>
        );
      } else if (trimmedLine.startsWith('# ')) {
        result.push(
          <h2 key={index} className="text-base md:text-lg font-bold text-[#171717] dark:text-[#f0f6fc] mt-3.5 mb-1.5">
            {formatInline(trimmedLine.slice(2), `h2-${index}`)}
          </h2>
        );
      } else if (numberedRegex.test(line)) {
        const numMatch = line.match(numberedRegex)!;
        const indent = numMatch[1].length;
        const mlClass = indent >= 4 ? 'ml-8' : indent >= 2 ? 'ml-6' : 'ml-4';
        result.push(
          <div key={`num-${index}`} className={`${mlClass} text-sm leading-relaxed text-[#171717] dark:text-[#f0f6fc] flex items-start gap-1.5`}>
            <span className="font-semibold text-[#171717] dark:text-[#f0f6fc] select-none shrink-0 font-mono text-[13px]">
              {numMatch[2]}
            </span>
            <span className="flex-1 min-w-0">{formatInline(numMatch[4], `num-${index}`)}</span>
          </div>
        );
      } else if (bulletRegex.test(line)) {
        const bulletMatch = line.match(bulletRegex)!;
        const indent = bulletMatch[1].length;
        const mlClass = indent >= 4 ? 'ml-8' : indent >= 2 ? 'ml-6' : 'ml-4';
        result.push(
          <div key={`bullet-${index}`} className={`${mlClass} text-sm leading-relaxed text-[#171717] dark:text-[#f0f6fc] flex items-start gap-2`}>
            <span className="text-[#666666] dark:text-[#8b949e] select-none shrink-0 leading-relaxed">•</span>
            <span className="flex-1 min-w-0">{formatInline(bulletMatch[3], `bullet-${index}`)}</span>
          </div>
        );
      } else if (line.trim() === '') {
        result.push(<div key={index} className="h-1.5" />);
      } else {
        result.push(
          <p key={index} className="text-sm leading-relaxed text-[#171717] dark:text-[#f0f6fc]">
            {formatInline(line, `p-${index}`)}
          </p>
        );
      }
    }

    // Flush unfinished streaming code block at EOF
    if (inCodeBlock) {
      result.push(
        <div
          key="code-unfinished"
          className="my-2.5 rounded-md border border-[#ebebeb] dark:border-[#21262d] bg-[#fafafa] dark:bg-[#0d1117] overflow-hidden font-mono text-[13px]"
        >
          {codeLanguage && (
            <div className="px-3 py-1 bg-[#f4f4f5] dark:bg-[#161b22] border-b border-[#ebebeb] dark:border-[#21262d] text-xs text-[#666666] dark:text-[#8b949e] font-medium flex justify-between items-center">
              <span>{codeLanguage}</span>
            </div>
          )}
          <pre className="p-3 text-[#171717] dark:text-[#f0f6fc] overflow-x-auto leading-relaxed select-text font-mono text-[13px]">
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

interface ParsedTodoItem {
  id: string;
  content: string;
  status: 'pending' | 'in_progress' | 'completed';
}

function tryExtractTodos(text: string): { leadingText?: string; todos: ParsedTodoItem[] } | null {
  if (!text) return null;
  const trimmed = text.trim();

  let candidate = trimmed;
  const fenceMatch = trimmed.match(/^```(?:json)?\s*([\s\S]*?)\s*```$/);
  if (fenceMatch) {
    candidate = fenceMatch[1].trim();
  }

  const toTodos = (arr: unknown[]): ParsedTodoItem[] | null => {
    if (!Array.isArray(arr) || arr.length === 0) return null;
    const items: ParsedTodoItem[] = [];
    for (let i = 0; i < arr.length; i++) {
      const item = arr[i];
      if (!item || typeof item !== 'object') return null;
      const rec = item as Record<string, unknown>;
      const content = rec.content ?? rec.title ?? rec.text ?? rec.task;
      if (typeof content !== 'string') return null;
      const statusRaw = String(rec.status || 'pending').toLowerCase();
      const status: 'pending' | 'in_progress' | 'completed' =
        statusRaw === 'completed' || statusRaw === 'done'
          ? 'completed'
          : statusRaw === 'in_progress' || statusRaw === 'active'
          ? 'in_progress'
          : 'pending';
      items.push({
        id: String(rec.id || `todo-${i + 1}`),
        content,
        status,
      });
    }
    return items;
  };

  if (candidate.startsWith('[') || candidate.startsWith('{')) {
    try {
      const parsed = JSON.parse(candidate);
      if (Array.isArray(parsed)) {
        const todos = toTodos(parsed);
        if (todos) return { todos };
      } else if (parsed && typeof parsed === 'object') {
        const todos = toTodos((parsed as any).todos || (parsed as any).tasks);
        if (todos) return { todos };
      }
    } catch {
      // not direct JSON
    }
  }

  const arrayStart = trimmed.indexOf('[');
  const arrayEnd = trimmed.lastIndexOf(']');
  if (arrayStart !== -1 && arrayEnd > arrayStart) {
    const jsonSub = trimmed.slice(arrayStart, arrayEnd + 1);
    try {
      const parsed = JSON.parse(jsonSub);
      const todos = toTodos(parsed);
      if (todos) {
        const leading = trimmed.slice(0, arrayStart).trim();
        return { leadingText: leading || undefined, todos };
      }
    } catch {
      // not JSON array
    }
  }

  return null;
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
}: DrawerActivityStreamProps) {
  const {
    activeSubagentId,
    activeSubagentTitle,
    activeSubagentRole,
    activeSubagentPrompt,
  } = useDrawerStore();
  const agentPersonas = useCortexStore((s) => s.agentPersonas);
  const matchedPersona = agentPersonas.find(
    (p) => p.role.toLowerCase() === (activeSubagentRole || '').toLowerCase()
  );
  const { t } = useTranslation();

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
    const res: (number | undefined)[] = new Array(list.length);
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
              title={t('drawer.closeNotification')}
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
            <div className="rounded-lg border border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#0d1117] p-4 shadow-xs">
              <div className="flex items-center justify-between pb-2 mb-2.5 border-b border-[#ebebeb] dark:border-[#21262d]">
                <div className="flex items-center space-x-2 min-w-0">
                  <AgentIcon name={matchedPersona?.icon} className="h-4 w-4 text-[#171717] dark:text-[#f0f6fc] shrink-0" />
                  <span className="text-xs font-semibold text-[#171717] dark:text-[#f0f6fc] font-sans truncate">
                    {t('drawer.subagentObjective', { title: formatSubagentDisplayTitle(activeSubagentTitle) })}
                  </span>
                </div>
                <div className="flex items-center space-x-2 shrink-0">
                  <span
                    style={getRoleBadgeStyle(activeSubagentRole, matchedPersona?.color)}
                    className="px-1.5 py-0.5 rounded text-[9px] font-mono font-bold uppercase tracking-wider border"
                  >
                    {(activeSubagentRole || 'general').toUpperCase()}
                  </span>
                </div>
              </div>
              <div className="text-sm text-[#171717] dark:text-[#f0f6fc] leading-relaxed whitespace-pre-wrap select-text font-sans bg-[#fafafa] dark:bg-[#161b22] p-3 rounded-md border border-[#ebebeb] dark:border-[#30363d]">
                {activeSubagentPrompt || t('drawer.subagentInstruction')}
              </div>
            </div>
          ) : (
            <>
              {/* a) Task Goal / User Objective Card */}
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
                <div className="text-sm text-[#171717] dark:text-[#f0f6fc] leading-relaxed whitespace-pre-wrap select-text font-sans">
                  {getCleanPrompt(task.description, task.title)}
                </div>
              </div>

          {/* a2) Relay Handoff Card */}
          {task.handoverSummary && (
            <div className="rounded-lg border border-[#ebebeb] dark:border-[#21262d] bg-[#fcfcfc] dark:bg-[#0d1117] p-4 shadow-xs">
              <div className="flex items-center justify-between pb-2 mb-2.5 border-b border-[#ebebeb] dark:border-[#21262d]">
                <div className="flex items-center space-x-2">
                  <CornerDownRight className="h-4 w-4 text-[#171717] dark:text-[#f0f6fc]" />
                  <span className="text-xs font-semibold text-[#171717] dark:text-[#f0f6fc] font-sans">
                    {t('drawer.relayContext')}
                  </span>
                </div>
                <span className="text-[10px] font-mono text-[#71717a] dark:text-[#8b949e] bg-[#f4f4f5] dark:bg-[#161b22] px-2 py-0.5 rounded border border-[#ebebeb] dark:border-[#30363d]">
                  {t('drawer.relayHandoff')}
                </span>
              </div>
              <div className="text-sm text-[#171717] dark:text-[#f0f6fc] leading-relaxed whitespace-pre-wrap select-text font-sans">
                {task.handoverSummary}
              </div>
            </div>
          )}


          </>
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
              const isLastActivity = idx === ((displayActivities?.length ?? 1) - 1);
              return (
                <ThinkingCard
                  key={key}
                  thought={activity}
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
                <ToolExecutionCard
                  key={key}
                  tool={toolWithDuration}
                />
              );
            }

            // User steer message
            if (activity?.type === 'user' || activity?.type === 'steer') {
              const images = Array.isArray(activity?.images) ? activity.images : [];
              return (
                <div
                  key={key}
                  className="rounded-lg border border-[#ebebeb] dark:border-[#21262d] bg-[#f9f9f9] dark:bg-[#0d1117] p-3.5 shadow-xs"
                >
                  <div className="flex items-center justify-between pb-1.5 mb-2 border-b border-[#ebebeb] dark:border-[#21262d] text-[10px] font-mono text-[#666666] dark:text-[#8b949e]">
                    <div className="flex items-center space-x-1.5 text-[#171717] dark:text-[#f0f6fc] font-medium">
                      <User className="h-3 w-3" />
                      <span>{t('drawer.userSteer')}</span>
                    </div>
                  </div>
                  {images.length > 0 && (
                    <div className="flex items-center gap-2 mb-2.5 flex-wrap">
                      {images.map((imgUrl, i) => (
                        <div
                          key={i}
                          onClick={() => handleOpenLightbox(imgUrl, `${t('drawer.attachImage')} ${i + 1}`)}
                          className="relative rounded-lg overflow-hidden border border-[#ebebeb] dark:border-[#21262d] max-w-[240px] max-h-40 bg-[#f4f4f5] dark:bg-[#161b22] shadow-xs group cursor-zoom-in"
                        >
                          <img
                            src={imgUrl}
                            alt={`${t('drawer.attachImage')} ${i + 1}`}
                            className="w-full h-full max-h-40 object-cover group-hover:scale-105 transition-transform duration-200 cursor-zoom-in"
                          />
                          <div className="absolute inset-0 bg-black/0 group-hover:bg-black/20 transition-colors flex items-center justify-center opacity-0 group-hover:opacity-100 pointer-events-none">
                            <span className="p-1 rounded-full bg-black/60 text-white shadow-xs">
                              <ZoomIn className="w-3.5 h-3.5" />
                            </span>
                          </div>
                        </div>
                      ))}
                    </div>
                  )}
                  <p className="text-sm text-[#171717] dark:text-[#f0f6fc] whitespace-pre-wrap select-text leading-relaxed font-sans">
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
                  className="flex items-center space-x-2 rounded-md border border-[#ebebeb] dark:border-[#21262d] bg-[#f4f4f5] dark:bg-[#161b22] px-3 py-2 text-xs text-[#4d4d4d] dark:text-[#c9d1d9] font-mono shadow-xs"
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
                  className="flex items-start space-x-2 rounded-md border border-[#fecaca] dark:border-[#da3633] bg-[#fef2f2] dark:bg-[#3b1219] p-3 text-xs text-[#991b1b] dark:text-[#ff7b72] font-mono shadow-xs"
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
              // Intermediate assistant commentary / step explanation rendered as collapsible chat card
              return (
                <AgentExplanationCard
                  key={key}
                  activity={activity}
                  content={activity?.content}
                  durationMs={activityDurations[idx] || activity?.durationMs}
                  hasNearbyTodoWrite={hasNearbyTodoWrite}
                />
              );
            }

            // True Final Response / Output Markdown Block (collapsible card)
            const isOutputExpanded = !collapsedOutputs.has(key);
            const outputDurationText = formatDurationCompact(activityDurations[idx] || activity?.durationMs);
            return (
              <div
                key={key}
                className="rounded-lg border border-[#e4e4e7] dark:border-[#21262d] bg-white dark:bg-[#0d1117] shadow-xs overflow-hidden my-2"
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
          title={t('drawer.scrollToNewTitle')}
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
