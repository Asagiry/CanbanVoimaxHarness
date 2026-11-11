import type {
  TableAlignment,
  MarkdownToken,
  ParsedTodoItem,
  ExtractedTodosResult,
} from './types';
import {
  isValidInlineCode,
  parseInlineTokens,
  splitTableRow,
  parseDelimiterRow,
  isHrLine,
} from './parseInlineTokens';

export {
  isValidInlineCode,
  parseInlineTokens,
  splitTableRow,
  parseDelimiterRow,
  isHrLine,
};

const BULLET_REGEX = /^(\s*)([-*+•]|\u2022)\s+(.*)$/;
const NUMBERED_REGEX = /^(\s*)(\d+(\.\d+)*[.)])\s+(.*)$/;
const TASK_CHECKBOX_REGEX = /^\[([ xX])\]\s*(.*)$/;
const IMAGE_LINE_REGEX = /^!\[(.*?)\]\((.*?)\)$/;

export interface ParseMarkdownOptions {
  /** Opt-in normalization for JSON-escaped literal `\\n` strings in single-line inputs (e.g. tool question prompts). */
  normalizeEscapedNewlines?: boolean;
}

function parseHeadingToken(trimmedLine: string): MarkdownToken | null {
  if (trimmedLine.startsWith('###### ')) return { type: 'heading', level: 6, text: trimmedLine.slice(7) };
  if (trimmedLine.startsWith('##### ')) return { type: 'heading', level: 6, text: trimmedLine.slice(6) };
  if (trimmedLine.startsWith('#### ')) return { type: 'heading', level: 5, text: trimmedLine.slice(5) };
  if (trimmedLine.startsWith('### ')) return { type: 'heading', level: 4, text: trimmedLine.slice(4) };
  if (trimmedLine.startsWith('## ')) return { type: 'heading', level: 3, text: trimmedLine.slice(3) };
  if (trimmedLine.startsWith('# ')) return { type: 'heading', level: 2, text: trimmedLine.slice(2) };
  return null;
}

/**
 * Tokenizes raw Markdown content into a high-level block AST tokens array.
 */
export function parseMarkdownTokens(
  content: string,
  options: ParseMarkdownOptions = {}
): MarkdownToken[] {
  let cleanContent = (content ?? '').replace(/\r\n/g, '\n').replace(/\r/g, '\n');
  if (
    options.normalizeEscapedNewlines &&
    !cleanContent.includes('\n') &&
    cleanContent.includes('\\n')
  ) {
    cleanContent = cleanContent.replace(/\\n/g, '\n');
  }
  const lines = cleanContent.split('\n');
  const tokens: MarkdownToken[] = [];
  let inCodeBlock = false;
  let codeLanguage = '';
  let codeBuffer: string[] = [];

  for (let index = 0; index < lines.length; index++) {
    const line = lines[index];
    const trimmedLine = line.trim();

    if (trimmedLine.startsWith('```')) {
      if (!inCodeBlock) {
        inCodeBlock = true;
        codeLanguage = trimmedLine.slice(3).trim();
        codeBuffer = [];
      } else {
        inCodeBlock = false;
        tokens.push({
          type: 'code',
          language: codeLanguage,
          code: codeBuffer.join('\n'),
        });
      }
      continue;
    }

    if (inCodeBlock) {
      codeBuffer.push(line);
      continue;
    }

    if (line.includes('|') && index + 1 < lines.length) {
      const aligns = parseDelimiterRow(lines[index + 1]);
      if (aligns) {
        const headerCells = splitTableRow(line);
        const colCount = Math.max(headerCells.length, aligns.length);
        const normAligns: TableAlignment[] = Array.from(
          { length: colCount },
          (_, k) => aligns[k] ?? 'left'
        );
        const normHeader: string[] = Array.from(
          { length: colCount },
          (_, k) => headerCells[k] ?? ''
        );
        const bodyRows: string[][] = [];
        let j = index + 2;
        while (
          j < lines.length &&
          lines[j].includes('|') &&
          lines[j].trim() !== '' &&
          !lines[j].trim().startsWith('```')
        ) {
          const cells = splitTableRow(lines[j]);
          bodyRows.push(Array.from({ length: colCount }, (_, k) => cells[k] ?? ''));
          j++;
        }
        tokens.push({
          type: 'table',
          header: normHeader,
          aligns: normAligns,
          rows: bodyRows,
        });
        index = j - 1;
        continue;
      }
    }

    if (isHrLine(line)) {
      tokens.push({ type: 'hr' });
      continue;
    }

    if (trimmedLine.startsWith('>')) {
      const quoteLines: string[] = [];
      let j = index;
      while (j < lines.length && lines[j].trim().startsWith('>')) {
        quoteLines.push(lines[j].trim().replace(/^>\s?/, ''));
        j++;
      }
      tokens.push({ type: 'blockquote', lines: quoteLines });
      index = j - 1;
      continue;
    }

    const imgMatch = trimmedLine.match(IMAGE_LINE_REGEX);
    if (imgMatch) {
      tokens.push({
        type: 'image',
        src: imgMatch[2],
        alt: imgMatch[1] || 'Image',
      });
      continue;
    }

    const headingToken = parseHeadingToken(trimmedLine);
    if (headingToken) {
      tokens.push(headingToken);
      continue;
    }

    const numMatch = line.match(NUMBERED_REGEX);
    if (numMatch) {
      tokens.push({
        type: 'numbered',
        indent: numMatch[1].length,
        number: numMatch[2],
        text: numMatch[4],
      });
      continue;
    }

    const bulletMatch = line.match(BULLET_REGEX);
    if (bulletMatch) {
      const indent = bulletMatch[1].length;
      const taskMatch = bulletMatch[3].match(TASK_CHECKBOX_REGEX);
      if (taskMatch) {
        tokens.push({
          type: 'task',
          indent,
          checked: taskMatch[1].toLowerCase() === 'x',
          text: taskMatch[2],
        });
      } else {
        tokens.push({
          type: 'bullet',
          indent,
          text: bulletMatch[3],
        });
      }
      continue;
    }

    if (trimmedLine === '') {
      tokens.push({ type: 'blank' });
    } else {
      tokens.push({ type: 'paragraph', text: line });
    }
  }

  if (inCodeBlock) {
    tokens.push({
      type: 'code',
      language: codeLanguage,
      code: codeBuffer.join('\n'),
      unfinished: true,
    });
  }

  return tokens;
}

function normalizeTodoItems(arr: unknown[]): ParsedTodoItem[] | null {
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
}

/**
 * Extracts action plan todos from JSON payloads or code blocks.
 */
export function tryExtractTodos(text: string): ExtractedTodosResult | null {
  if (!text) return null;
  const trimmed = text.trim();
  const fenceMatch = trimmed.match(/^```(?:json)?\s*([\s\S]*?)\s*```$/);
  const candidate = fenceMatch ? fenceMatch[1].trim() : trimmed;

  if (candidate.startsWith('[') || candidate.startsWith('{')) {
    try {
      const parsed: unknown = JSON.parse(candidate);
      if (Array.isArray(parsed)) {
        const todos = normalizeTodoItems(parsed);
        if (todos) return { todos };
      } else if (parsed && typeof parsed === 'object') {
        const obj = parsed as Record<string, unknown>;
        const rawTodos = obj.todos ?? obj.tasks;
        if (Array.isArray(rawTodos)) {
          const todos = normalizeTodoItems(rawTodos);
          if (todos) return { todos };
        }
      }
    } catch (err) {
      console.debug('[parseMarkdownTokens] direct todo JSON parse skipped:', err);
    }
  }

  const arrayStart = trimmed.indexOf('[');
  const arrayEnd = trimmed.lastIndexOf(']');
  if (arrayStart !== -1 && arrayEnd > arrayStart) {
    try {
      const parsed: unknown = JSON.parse(trimmed.slice(arrayStart, arrayEnd + 1));
      if (Array.isArray(parsed)) {
        const todos = normalizeTodoItems(parsed);
        if (todos) {
          const leading = trimmed.slice(0, arrayStart).trim();
          return { leadingText: leading || undefined, todos };
        }
      }
    } catch (err) {
      console.debug('[parseMarkdownTokens] embedded todo JSON parse skipped:', err);
    }
  }

  return null;
}
