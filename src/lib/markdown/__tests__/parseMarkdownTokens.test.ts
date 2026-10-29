import { describe, it, expect } from 'vitest';
import {
  splitTableRow,
  parseDelimiterRow,
  isHrLine,
  isValidInlineCode,
  parseInlineTokens,
  parseMarkdownTokens,
  tryExtractTodos,
} from '../parseMarkdownTokens';
import type {
  MarkdownTableToken,
  MarkdownTaskItemToken,
  MarkdownBulletItemToken,
  InlineBoldNode,
} from '../types';

describe('Markdown Table Parsing & Alignment', () => {
  describe('splitTableRow', () => {
    it('splits pipe-delimited row and trims cells', () => {
      const result = splitTableRow('| Name | Age | City |');
      expect(result).toEqual(['Name', 'Age', 'City']);
    });

    it('handles rows without leading or trailing pipes', () => {
      const result = splitTableRow('Name | Age | City');
      expect(result).toEqual(['Name', 'Age', 'City']);
    });

    it('preserves empty cells inside rows', () => {
      const result = splitTableRow('| a |  | c |');
      expect(result).toEqual(['a', '', 'c']);
    });
  });

  describe('parseDelimiterRow', () => {
    it('detects left alignment', () => {
      expect(parseDelimiterRow('|:---|---|:--|')).toEqual(['left', 'left', 'left']);
    });

    it('detects center alignment', () => {
      expect(parseDelimiterRow('|:---:|:-:|')).toEqual(['center', 'center']);
    });

    it('detects right alignment', () => {
      expect(parseDelimiterRow('|---:|--:|')).toEqual(['right', 'right']);
    });

    it('detects mixed alignments correctly', () => {
      const result = parseDelimiterRow('| :--- | :---: | ---: |');
      expect(result).toEqual(['left', 'center', 'right']);
    });

    it('returns null for non-delimiter rows', () => {
      expect(parseDelimiterRow('| Name | Age |')).toBeNull();
      expect(parseDelimiterRow('| --- | invalid |')).toBeNull();
      expect(parseDelimiterRow('')).toBeNull();
    });
  });

  describe('parseMarkdownTokens - tables', () => {
    it('parses a complete Markdown table with alignments and body rows', () => {
      const md = [
        '| Header 1 | Header 2 | Header 3 |',
        '| :---     | :---:    | ---:     |',
        '| Row 1A   | Row 1B   | Row 1C   |',
        '| Row 2A   | Row 2B   | Row 2C   |',
      ].join('\n');

      const tokens = parseMarkdownTokens(md);
      expect(tokens).toHaveLength(1);

      const tableToken = tokens[0] as MarkdownTableToken;
      expect(tableToken.type).toBe('table');
      expect(tableToken.header).toEqual(['Header 1', 'Header 2', 'Header 3']);
      expect(tableToken.aligns).toEqual(['left', 'center', 'right']);
      expect(tableToken.rows).toEqual([
        ['Row 1A', 'Row 1B', 'Row 1C'],
        ['Row 2A', 'Row 2B', 'Row 2C'],
      ]);
    });

    it('normalizes columns when body row has fewer cells than header', () => {
      const md = [
        '| H1 | H2 | H3 |',
        '|---|---|---|',
        '| A | B |',
      ].join('\n');

      const tokens = parseMarkdownTokens(md);
      const tableToken = tokens[0] as MarkdownTableToken;
      expect(tableToken.rows[0]).toEqual(['A', 'B', '']);
    });
  });
});

describe('Task Checkboxes & Bullets', () => {
  it('parses unchecked task checkbox [- ]', () => {
    const tokens = parseMarkdownTokens('- [ ] Implement Vitest suite');
    expect(tokens).toHaveLength(1);
    const task = tokens[0] as MarkdownTaskItemToken;
    expect(task.type).toBe('task');
    expect(task.checked).toBe(false);
    expect(task.text).toBe('Implement Vitest suite');
    expect(task.indent).toBe(0);
  });

  it('parses checked task checkbox [x] and [X]', () => {
    const tokensLower = parseMarkdownTokens('* [x] Checked lowercase');
    const taskLower = tokensLower[0] as MarkdownTaskItemToken;
    expect(taskLower.type).toBe('task');
    expect(taskLower.checked).toBe(true);
    expect(taskLower.text).toBe('Checked lowercase');

    const tokensUpper = parseMarkdownTokens('+ [X] Checked uppercase');
    const taskUpper = tokensUpper[0] as MarkdownTaskItemToken;
    expect(taskUpper.type).toBe('task');
    expect(taskUpper.checked).toBe(true);
    expect(taskUpper.text).toBe('Checked uppercase');
  });

  it('preserves indentation for nested tasks', () => {
    const md = '  - [ ] Nested subtask';
    const tokens = parseMarkdownTokens(md);
    const task = tokens[0] as MarkdownTaskItemToken;
    expect(task.indent).toBe(2);
    expect(task.checked).toBe(false);
    expect(task.text).toBe('Nested subtask');
  });

  it('distinguishes normal bullet items from tasks', () => {
    const tokens = parseMarkdownTokens('- Regular bullet item\n* Another bullet');
    expect(tokens).toHaveLength(2);

    const b1 = tokens[0] as MarkdownBulletItemToken;
    expect(b1.type).toBe('bullet');
    expect(b1.text).toBe('Regular bullet item');
    expect(b1.indent).toBe(0);

    const b2 = tokens[1] as MarkdownBulletItemToken;
    expect(b2.type).toBe('bullet');
    expect(b2.text).toBe('Another bullet');
  });
});

describe('Inline Tokens (Code, Bold, Italic, Links)', () => {
  describe('isValidInlineCode', () => {
    it('validates genuine code snippets', () => {
      expect(isValidInlineCode('const a = 1;')).toBe(true);
      expect(isValidInlineCode('npm run build')).toBe(true);
      expect(isValidInlineCode('AppError::NotFound')).toBe(true);
      expect(isValidInlineCode('"Привет мир"')).toBe(true);
    });

    it('rejects invalid or prose code blocks', () => {
      expect(isValidInlineCode('')).toBe(false);
      expect(isValidInlineCode('a'.repeat(121))).toBe(false);
      expect(isValidInlineCode('has\nnewline')).toBe(false);
      expect(isValidInlineCode('sentence one. sentence two')).toBe(false);
      expect(isValidInlineCode('inner`backtick')).toBe(false);
      expect(isValidInlineCode('обычный русский текст без кавычек')).toBe(false);
    });
  });

  describe('parseInlineTokens', () => {
    it('parses plain text without inline formatting', () => {
      const nodes = parseInlineTokens('Hello world');
      expect(nodes).toEqual([{ type: 'text', text: 'Hello world' }]);
    });

    it('parses inline code snippets', () => {
      const nodes = parseInlineTokens('Run `pnpm test` to check');
      expect(nodes).toEqual([
        { type: 'text', text: 'Run ' },
        { type: 'code', text: 'pnpm test' },
        { type: 'text', text: ' to check' },
      ]);
    });

    it('parses bold text (** and __)', () => {
      const nodesAsterisk = parseInlineTokens('This is **bold** text');
      expect(nodesAsterisk).toEqual([
        { type: 'text', text: 'This is ' },
        {
          type: 'bold',
          children: [{ type: 'text', text: 'bold' }],
        },
        { type: 'text', text: ' text' },
      ]);

      const nodesUnderscore = parseInlineTokens('This is __bold__ text');
      expect(nodesUnderscore).toEqual([
        { type: 'text', text: 'This is ' },
        {
          type: 'bold',
          children: [{ type: 'text', text: 'bold' }],
        },
        { type: 'text', text: ' text' },
      ]);
    });

    it('parses italic text (* and _)', () => {
      const nodesAsterisk = parseInlineTokens('This is *italic* text');
      expect(nodesAsterisk).toEqual([
        { type: 'text', text: 'This is ' },
        {
          type: 'italic',
          children: [{ type: 'text', text: 'italic' }],
        },
        { type: 'text', text: ' text' },
      ]);

      const nodesUnderscore = parseInlineTokens('This is _italic_ text');
      expect(nodesUnderscore).toEqual([
        { type: 'text', text: 'This is ' },
        {
          type: 'italic',
          children: [{ type: 'text', text: 'italic' }],
        },
        { type: 'text', text: ' text' },
      ]);
    });

    it('parses links with nested labels', () => {
      const nodes = parseInlineTokens('Go to [Documentation](https://example.com/docs)');
      expect(nodes).toEqual([
        { type: 'text', text: 'Go to ' },
        {
          type: 'link',
          text: 'Documentation',
          url: 'https://example.com/docs',
          children: [{ type: 'text', text: 'Documentation' }],
        },
      ]);
    });

    it('parses mixed formatting together', () => {
      const nodes = parseInlineTokens('Use `foo` with **strong [link](https://test.io)** now');
      expect(nodes).toHaveLength(5);
      expect(nodes[0]).toEqual({ type: 'text', text: 'Use ' });
      expect(nodes[1]).toEqual({ type: 'code', text: 'foo' });
      expect(nodes[2]).toEqual({ type: 'text', text: ' with ' });

      const boldNode = nodes[3] as InlineBoldNode;
      expect(boldNode.type).toBe('bold');
      expect(boldNode.children).toEqual([
        { type: 'text', text: 'strong ' },
        {
          type: 'link',
          text: 'link',
          url: 'https://test.io',
          children: [{ type: 'text', text: 'link' }],
        },
      ]);
      expect(nodes[4]).toEqual({ type: 'text', text: ' now' });
    });
  });
});

describe('tryExtractTodos (JSON Todo Extraction)', () => {
  it('extracts todos from a raw JSON array', () => {
    const raw = JSON.stringify([
      { id: '1', content: 'Step 1', status: 'completed' },
      { id: '2', title: 'Step 2', status: 'in_progress' },
      { id: '3', text: 'Step 3', status: 'pending' },
    ]);

    const result = tryExtractTodos(raw);
    expect(result).not.toBeNull();
    expect(result?.todos).toEqual([
      { id: '1', content: 'Step 1', status: 'completed' },
      { id: '2', content: 'Step 2', status: 'in_progress' },
      { id: '3', content: 'Step 3', status: 'pending' },
    ]);
  });

  it('extracts todos from fenced markdown json blocks', () => {
    const markdown = '```json\n[\n  { "id": "t1", "content": "Install Vitest", "status": "done" },\n  { "id": "t2", "content": "Run suite", "status": "active" }\n]\n```';

    const result = tryExtractTodos(markdown);
    expect(result).not.toBeNull();
    expect(result?.todos).toEqual([
      { id: 't1', content: 'Install Vitest', status: 'completed' },
      { id: 't2', content: 'Run suite', status: 'in_progress' },
    ]);
  });

  it('extracts todos from an object containing a todos or tasks key', () => {
    const raw = JSON.stringify({
      todos: [
        { content: 'First item', status: 'pending' },
        { content: 'Second item', status: 'completed' },
      ],
    });

    const result = tryExtractTodos(raw);
    expect(result).not.toBeNull();
    expect(result?.todos).toEqual([
      { id: 'todo-1', content: 'First item', status: 'pending' },
      { id: 'todo-2', content: 'Second item', status: 'completed' },
    ]);
  });

  it('extracts embedded JSON array and captures leadingText', () => {
    const text = 'Here is the execution plan:\n[{"content": "Deploy changes", "status": "pending"}]';
    const result = tryExtractTodos(text);
    expect(result).not.toBeNull();
    expect(result?.leadingText).toBe('Here is the execution plan:');
    expect(result?.todos).toEqual([
      { id: 'todo-1', content: 'Deploy changes', status: 'pending' },
    ]);
  });

  it('returns null for empty or invalid payloads', () => {
    expect(tryExtractTodos('')).toBeNull();
    expect(tryExtractTodos('Plain text without json')).toBeNull();
    expect(tryExtractTodos('{"randomKey": 123}')).toBeNull();
  });
});

describe('Horizontal Separators (HR) & Headings', () => {
  describe('isHrLine', () => {
    it('recognizes standard markdown thematic breaks', () => {
      expect(isHrLine('---')).toBe(true);
      expect(isHrLine('***')).toBe(true);
      expect(isHrLine('___')).toBe(true);
      expect(isHrLine('-----')).toBe(true);
      expect(isHrLine(' * * * ')).toBe(true);
      expect(isHrLine(' - - - ')).toBe(true);
    });

    it('rejects non-hr lines', () => {
      expect(isHrLine('--')).toBe(false);
      expect(isHrLine('--- heading')).toBe(false);
      expect(isHrLine('regular text')).toBe(false);
      expect(isHrLine('')).toBe(false);
    });
  });

  describe('parseMarkdownTokens - Headings & HR', () => {
    it('parses headings with mapped semantic levels', () => {
      const md = [
        '# Level 2 Heading',
        '## Level 3 Heading',
        '### Level 4 Heading',
        '#### Level 5 Heading',
      ].join('\n');

      const tokens = parseMarkdownTokens(md);
      expect(tokens).toHaveLength(4);

      expect(tokens[0]).toEqual({
        type: 'heading',
        level: 2,
        text: 'Level 2 Heading',
      });
      expect(tokens[1]).toEqual({
        type: 'heading',
        level: 3,
        text: 'Level 3 Heading',
      });
      expect(tokens[2]).toEqual({
        type: 'heading',
        level: 4,
        text: 'Level 4 Heading',
      });
      expect(tokens[3]).toEqual({
        type: 'heading',
        level: 5,
        text: 'Level 5 Heading',
      });
    });

    it('parses standalone horizontal rules', () => {
      const md = 'Paragraph 1\n\n---\n\nParagraph 2';
      const tokens = parseMarkdownTokens(md);

      expect(tokens.some((t) => t.type === 'hr')).toBe(true);
      const hrIndex = tokens.findIndex((t) => t.type === 'hr');
      expect(tokens[hrIndex]).toEqual({ type: 'hr' });
    });
  });
});
