/**
 * Notification Formatter Service
 * Normalizes Markdown content into clean, legible plain-text typography for desktop notifications
 * and generates clean titles without emoji noise.
 */

export type NotificationType = 'advance_task' | 'question' | 'reject_task' | 'test';

export interface TitleMeta {
  stageTitle?: string;
  stageId?: string;
  isTerminal?: boolean;
}

/**
 * Normalizes markdown syntax into clean plain text suitable for desktop Toast notifications.
 * Removes raw markdown formatting, converts bullets, blockquotes and code snippets,
 * and performs word-boundary truncation.
 */
export function formatMarkdownForToast(rawText?: string | null, maxLength = 320): string {
  if (!rawText) return '';

  let text = rawText;

  // Normalize Windows CRLF to LF
  text = text.replace(/\r\n/g, '\n');

  // Strip code blocks with optional language specifier, prefixing each line with '│ '
  text = text.replace(/```[a-zA-Z0-9_-]*\n([\s\S]*?)```/g, (_match, codeContent: string) => {
    return codeContent
      .trim()
      .split('\n')
      .map((line) => `│ ${line}`)
      .join('\n');
  });

  // Strip Markdown headings (e.g. ### Title -> Title)
  text = text.replace(/^#{1,6}\s+(.+)$/gm, '$1');

  // Strip Markdown images: ![alt](url) -> [alt]
  text = text.replace(/!\[([^\]]*)\]\([^)]+\)/g, '[$1]');

  // Convert links [text](url) -> text
  text = text.replace(/\[([^\]]+)\]\([^)]+\)/g, '$1');

  // Convert inline code `code` -> 'code'
  text = text.replace(/`([^`]+)`/g, "'$1'");

  // Strip bold/italic formatting (**text**, __text__, *text*, _text_, ~~text~~)
  text = text.replace(/\*\*([^*]+)\*\*/g, '$1');
  text = text.replace(/__([^_]+)__/g, '$1');
  text = text.replace(/~~([^~]+)~~/g, '$1');
  text = text.replace(/(^|[^\w])\*([^*]+)\*([^\w]|$)/g, '$1$2$3');
  text = text.replace(/(^|[^\w])_([^_]+)_([^\w]|$)/g, '$1$2$3');

  // Convert unordered list markers (-, *, +) into clean Unicode bullets
  text = text.replace(/^[\t ]*[-*+][\t ]+(.+)$/gm, '• $1');

  // Convert blockquotes (> quote) into vertical bar prefix
  text = text.replace(/^>[\t ]*(.*)$/gm, '│ $1');

  // Strip horizontal rules (---, ***, ___ on own line)
  text = text.replace(/^[-*_]{3,}$/gm, '');

  // Strip residual HTML tags if present
  text = text.replace(/<\/?[^>]+(>|$)/g, '');

  // Collapse 3+ newlines into double newlines
  text = text.replace(/\n{3,}/g, '\n\n');

  // Trim extraneous whitespace
  text = text.trim();

  // Smart truncation at word boundary if exceeding maxLength
  if (text.length > maxLength) {
    const slicePoint = maxLength - 1;
    const truncated = text.slice(0, slicePoint);
    const lastSpace = truncated.lastIndexOf(' ');
    const boundary = lastSpace > Math.floor(maxLength * 0.7) ? lastSpace : slicePoint;
    text = text.slice(0, boundary).trimEnd() + '…';
  }

  return text;
}

/**
 * Crafts clean notification titles adhering to project standards:
 * No emoji noise, consistent Russian localization, and clear task context.
 */
export function formatNotificationTitle(
  type: NotificationType,
  taskId?: string,
  meta?: TitleMeta
): string {
  const id = taskId || '';

  switch (type) {
    case 'advance_task': {
      if (meta?.isTerminal) {
        return `Задача ${id} завершена.`;
      }
      const stageName = meta?.stageTitle || meta?.stageId || 'следующую';
      return `Задача ${id} перешла в колонку ${stageName}`;
    }
    case 'question':
      return `В задаче ${id} вопрос от агента.`;
    case 'reject_task':
      return `Задача ${id} возвращена на доработку.`;
    case 'test':
      return 'CanbanVoimaxHarness';
    default:
      return id ? `Задача ${id}` : 'CanbanVoimaxHarness';
  }
}

/**
 * Resolves the canonical column / stage SVG icon identifier for notifications.
 */
export function getStageIconName(stageId?: string, isTerminal?: boolean): string {
  if (isTerminal) return 'ready';
  if (!stageId) return 'app';

  const normalized = stageId.toLowerCase();
  switch (normalized) {
    case 'backlog':
    case 'inbox':
      return 'backlog';
    case 'plan':
    case 'planning':
      return 'plan';
    case 'dev':
    case 'developer':
      return 'dev';
    case 'review':
    case 'reviewer':
      return 'review';
    case 'retro':
    case 'retrospective':
      return 'retro';
    case 'ready':
    case 'done':
      return 'ready';
    default:
      return 'app';
  }
}
