import type {
  InlineNode,
  TableAlignment,
  MarkdownToken,
  ParsedTodoItem,
  ExtractedTodosResult,
} from './types';

const BULLET_REGEX = /^(\s*)([-*+•]|\u2022)\s+(.*)$/;
const NUMBERED_REGEX = /^(\s*)(\d+(\.\d+)*[.)])\s+(.*)$/;
const TASK_CHECKBOX_REGEX = /^\[([ xX])\]\s*(.*)$/;
const IMAGE_LINE_REGEX = /^!\[(.*?)\]\((.*?)\)$/;

/**
 * Validates whether candidate text inside backticks is a legitimate short inline code snippet
 * rather than conversational Russian prose or malformed text.
 */
export function isValidInlineCode(code: string): boolean {
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

/**
 * Parses inline formatting (code, links, bold, italic, text) recursively.
 */
export function parseInlineTokens(text: string, depth = 0): InlineNode[] {
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

    // 2. Inline links: [label](url)
    if (depth < 2 && text[i] === '[' && (i === 0 || text[i - 1] !== '!')) {
      const closeBracketIdx = text.indexOf('](', i + 1);
      if (closeBracketIdx !== -1) {
        const candidateLabel = text.slice(i + 1, closeBracketIdx);
        if (
          !candidateLabel.includes('\n') &&
          !candidateLabel.includes('[') &&
          candidateLabel.length > 0 &&
          candidateLabel.length <= 300
        ) {
          let parenDepth = 1;
          let closeParenIdx = -1;
          for (let p = closeBracketIdx + 2; p < text.length; p++) {
            if (text[p] === '(') parenDepth++;
            else if (text[p] === ')') {
              parenDepth--;
              if (parenDepth === 0) {
                closeParenIdx = p;
                break;
              }
            } else if (text[p] === ' ' || text[p] === '\n') {
              break;
            }
          }
          if (closeParenIdx !== -1) {
            const candidateUrl = text.slice(closeBracketIdx + 2, closeParenIdx).trim();
            if (candidateUrl.length > 0) {
              flushText();
              nodes.push({
                type: 'link',
                text: candidateLabel,
                url: candidateUrl,
                children: parseInlineTokens(candidateLabel, depth + 1),
              });
              i = closeParenIdx + 1;
              continue;
            }
          }
        }
      }
    }

    // 3. Bold: **text** or __text__
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

    // 4. Italic: *text* or _text_
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

/**
 * Splits a Markdown table row into trimmed cell values.
 */
export function splitTableRow(row: string): string[] {
  let t = row.trim();
  if (t.startsWith('|')) t = t.slice(1);
  if (t.endsWith('|')) t = t.slice(0, -1);
  return t.split('|').map((c) => c.trim());
}

/**
 * Parses a table delimiter row (e.g. |:---|:---:|---:|) into alignment values.
 */
export function parseDelimiterRow(row: string): TableAlignment[] | null {
  const cells = splitTableRow(row);
  if (cells.length === 0) return null;
  const aligns: TableAlignment[] = [];
  for (const c of cells) {
    if (!/^:?-+:?$/.test(c)) return null;
    if (c.startsWith(':') && c.endsWith(':') && c.length >= 3) aligns.push('center');
    else if (c.endsWith(':')) aligns.push('right');
    else aligns.push('left');
  }
  return aligns;
}

/**
 * Determines whether a line is a horizontal thematic break (---, ***, ___).
 */
export function isHrLine(row: string): boolean {
  const nospace = row.trim().replace(/\s+/g, '');
  return /^(-{3,}|\*{3,}|_{3,})$/.test(nospace);
}

/**
 * Tokenizes raw Markdown content into a high-level block AST tokens array.
 */
export function parseMarkdownTokens(content: string): MarkdownToken[] {
  const cleanContent = (content ?? '').replace(/\r\n/g, '\n').replace(/\r/g, '\n');
  const lines = cleanContent.split('\n');
  const tokens: MarkdownToken[] = [];
  let inCodeBlock = false;
  let codeLanguage = '';
  let codeBuffer: string[] = [];

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

    // GFM table: header with `|` + delimiter row, then data rows with `|`
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

    // Thematic break outside tables
    if (isHrLine(line)) {
      tokens.push({ type: 'hr' });
      continue;
    }

    // Blockquotes: lines starting with `> quote`
    if (trimmedLine.startsWith('>')) {
      const quoteLines: string[] = [];
      let j = index;
      while (j < lines.length && lines[j].trim().startsWith('>')) {
        quoteLines.push(lines[j].trim().replace(/^>\s?/, ''));
        j++;
      }
      tokens.push({
        type: 'blockquote',
        lines: quoteLines,
      });
      index = j - 1;
      continue;
    }

    // Image line: ![alt](url)
    const imgMatch = line.trim().match(IMAGE_LINE_REGEX);
    if (imgMatch) {
      const imgSrc = imgMatch[2];
      const imgAlt = imgMatch[1] || 'Image';
      tokens.push({
        type: 'image',
        src: imgSrc,
        alt: imgAlt,
      });
      continue;
    }

    // Headings
    if (trimmedLine.startsWith('#### ')) {
      tokens.push({
        type: 'heading',
        level: 5,
        text: trimmedLine.slice(5),
      });
    } else if (trimmedLine.startsWith('### ')) {
      tokens.push({
        type: 'heading',
        level: 4,
        text: trimmedLine.slice(4),
      });
    } else if (trimmedLine.startsWith('## ')) {
      tokens.push({
        type: 'heading',
        level: 3,
        text: trimmedLine.slice(3),
      });
    } else if (trimmedLine.startsWith('# ')) {
      tokens.push({
        type: 'heading',
        level: 2,
        text: trimmedLine.slice(2),
      });
    } else if (NUMBERED_REGEX.test(line)) {
      const numMatch = line.match(NUMBERED_REGEX)!;
      const indent = numMatch[1].length;
      tokens.push({
        type: 'numbered',
        indent,
        number: numMatch[2],
        text: numMatch[4],
      });
    } else if (BULLET_REGEX.test(line)) {
      const bulletMatch = line.match(BULLET_REGEX)!;
      const indent = bulletMatch[1].length;
      const taskMatch = bulletMatch[3].match(TASK_CHECKBOX_REGEX);
      if (taskMatch) {
        const isChecked = taskMatch[1].toLowerCase() === 'x';
        const taskText = taskMatch[2];
        tokens.push({
          type: 'task',
          indent,
          checked: isChecked,
          text: taskText,
        });
      } else {
        tokens.push({
          type: 'bullet',
          indent,
          text: bulletMatch[3],
        });
      }
    } else if (line.trim() === '') {
      tokens.push({ type: 'blank' });
    } else {
      tokens.push({
        type: 'paragraph',
        text: line,
      });
    }
  }

  // Flush unfinished streaming code block at EOF
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

/**
 * Extracts action plan todos from JSON payloads or code blocks.
 */
export function tryExtractTodos(text: string): ExtractedTodosResult | null {
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
        const obj = parsed as Record<string, unknown>;
        const rawTodos = (obj.todos || obj.tasks) as unknown[];
        if (Array.isArray(rawTodos)) {
          const todos = toTodos(rawTodos);
          if (todos) return { todos };
        }
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
