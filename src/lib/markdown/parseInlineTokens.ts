import type { InlineNode, TableAlignment } from './types';

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
