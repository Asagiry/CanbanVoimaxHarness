// Markdown AST Token Types

export interface InlineTextNode {
  type: 'text';
  text: string;
}

export interface InlineCodeNode {
  type: 'code';
  text: string;
}

export interface InlineBoldNode {
  type: 'bold';
  children: InlineNode[];
}

export interface InlineItalicNode {
  type: 'italic';
  children: InlineNode[];
}

export interface InlineLinkNode {
  type: 'link';
  text: string;
  url: string;
  children: InlineNode[];
}

export type InlineNode =
  | InlineTextNode
  | InlineCodeNode
  | InlineBoldNode
  | InlineItalicNode
  | InlineLinkNode;

export type TableAlignment = 'left' | 'center' | 'right';

export interface MarkdownCodeBlockToken {
  type: 'code';
  language: string;
  code: string;
  unfinished?: boolean;
}

export interface MarkdownTableToken {
  type: 'table';
  header: string[];
  aligns: TableAlignment[];
  rows: string[][];
}

export interface MarkdownHrToken {
  type: 'hr';
}

export interface MarkdownBlockquoteToken {
  type: 'blockquote';
  lines: string[];
}

export interface MarkdownImageToken {
  type: 'image';
  src: string;
  alt: string;
}

export interface MarkdownHeadingToken {
  type: 'heading';
  level: 2 | 3 | 4 | 5;
  text: string;
}

export interface MarkdownNumberedItemToken {
  type: 'numbered';
  indent: number;
  number: string;
  text: string;
}

export interface MarkdownTaskItemToken {
  type: 'task';
  indent: number;
  checked: boolean;
  text: string;
}

export interface MarkdownBulletItemToken {
  type: 'bullet';
  indent: number;
  text: string;
}

export interface MarkdownBlankToken {
  type: 'blank';
}

export interface MarkdownParagraphToken {
  type: 'paragraph';
  text: string;
}

export type MarkdownToken =
  | MarkdownCodeBlockToken
  | MarkdownTableToken
  | MarkdownHrToken
  | MarkdownBlockquoteToken
  | MarkdownImageToken
  | MarkdownHeadingToken
  | MarkdownNumberedItemToken
  | MarkdownTaskItemToken
  | MarkdownBulletItemToken
  | MarkdownBlankToken
  | MarkdownParagraphToken;

export interface ParsedTodoItem {
  id: string;
  content: string;
  status: 'pending' | 'in_progress' | 'completed';
}

export interface ExtractedTodosResult {
  leadingText?: string;
  todos: ParsedTodoItem[];
}
