import React, { useMemo } from 'react';
import { Check } from 'lucide-react';

export interface MarkdownContentProps {
  content?: string;
  variant?: 'default' | 'user';
}

export function MarkdownContent({ content, variant = 'default' }: MarkdownContentProps) {
  const isUser = variant === 'user';

  const elements = useMemo(() => {
    if (!content) return null;
    const cleanContent = content.replace(/\r\n/g, '\n').replace(/\r/g, '\n');
    if (!cleanContent.trim()) return null;
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
    const numberedRegex = /^(\s*)(\d+([.)]|\.\d+)*)\s+(.*)$/;

    for (let index = 0; index < lines.length; index++) {
      const line = lines[index];
      const trimmedLine = line.trim();

      // Code blocks
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
              className={
                isUser
                  ? 'my-2 rounded-md border border-white/20 bg-black/40 overflow-hidden font-mono text-[12px]'
                  : 'my-2 rounded-md border border-[#ebebeb] dark:border-[#21262d] bg-[#fafafa] dark:bg-[#0d1117] overflow-hidden font-mono text-[12px]'
              }
            >
              {codeLanguage && (
                <div
                  className={
                    isUser
                      ? 'px-3 py-1 bg-white/10 border-b border-white/15 text-[11px] text-white/70 font-medium flex justify-between items-center'
                      : 'px-3 py-1 bg-[#f4f4f5] dark:bg-[#161b22] border-b border-[#ebebeb] dark:border-[#21262d] text-[11px] text-[#666666] dark:text-[#8b949e] font-medium flex justify-between items-center'
                  }
                >
                  <span>{codeLanguage}</span>
                </div>
              )}
              <pre
                className={
                  isUser
                    ? 'p-2.5 text-white overflow-x-auto leading-relaxed select-text font-mono text-[12px]'
                    : 'p-2.5 text-[#171717] dark:text-[#f0f6fc] overflow-x-auto leading-relaxed select-text font-mono text-[12px]'
                }
              >
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

      // Empty line
      if (!trimmedLine) {
        result.push(<div key={`empty-${index}`} className="h-1.5" />);
        continue;
      }

      // GFM Table
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
          result.push(
            <div
              key={`table-${index}`}
              className={
                isUser
                  ? 'my-2 overflow-x-auto rounded-md border border-white/20'
                  : 'my-2 overflow-x-auto rounded-md border border-[#ebebeb] dark:border-[#21262d]'
              }
            >
              <table className="w-full border-collapse text-[12px] font-mono tabular-nums">
                <thead>
                  <tr>
                    {normHeader.map((cell, k) => (
                      <th
                        key={k}
                        style={{ textAlign: normAligns[k] }}
                        className={
                          isUser
                            ? 'border border-white/20 bg-white/10 px-2 py-1 font-semibold text-white'
                            : 'border border-[#ebebeb] dark:border-[#21262d] bg-[#f9f9f9] dark:bg-[#161b22] px-2 py-1 font-semibold text-[#171717] dark:text-[#f0f6fc]'
                        }
                      >
                        {renderInline(cell, isUser)}
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
                          className={
                            isUser
                              ? 'border border-white/20 px-2 py-1 text-white'
                              : 'border border-[#ebebeb] dark:border-[#21262d] px-2 py-1 text-[#171717] dark:text-[#f0f6fc]'
                          }
                        >
                          {renderInline(cell, isUser)}
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

      // HR
      if (isHrLine(line)) {
        result.push(
          <hr
            key={`hr-${index}`}
            className={isUser ? 'border-white/20 my-2' : 'border-[#ebebeb] dark:border-[#21262d] my-2'}
          />
        );
        continue;
      }

      // Blockquotes
      if (trimmedLine.startsWith('>')) {
        const quoteText = trimmedLine.replace(/^>+\s*/, '');
        result.push(
          <blockquote
            key={`quote-${index}`}
            className={
              isUser
                ? 'my-1.5 pl-3 border-l-2 border-violet-400/60 text-white/90 italic text-xs leading-relaxed bg-white/10 py-1 pr-2 rounded-r'
                : 'my-1.5 pl-3 border-l-2 border-[#7c3aed]/40 text-[#4d4d4d] dark:text-[#c9d1d9] italic text-xs leading-relaxed bg-[#fbfaff] dark:bg-[#1e1b4b]/30 py-1 pr-2 rounded-r'
            }
          >
            {renderInline(quoteText, isUser)}
          </blockquote>
        );
        continue;
      }

      // Headings
      if (trimmedLine.startsWith('#### ')) {
        result.push(
          <h5
            key={`h5-${index}`}
            className={
              isUser
                ? 'text-xs font-semibold text-white mt-2 mb-0.5'
                : 'text-xs font-semibold text-[#171717] dark:text-[#f0f6fc] mt-2 mb-0.5'
            }
          >
            {renderInline(trimmedLine.slice(5), isUser)}
          </h5>
        );
        continue;
      } else if (trimmedLine.startsWith('### ')) {
        result.push(
          <h4
            key={`h4-${index}`}
            className={
              isUser
                ? 'text-xs font-bold text-white mt-2.5 mb-1 flex items-center gap-1.5'
                : 'text-xs font-bold text-[#171717] dark:text-[#f0f6fc] mt-2.5 mb-1 flex items-center gap-1.5'
            }
          >
            {renderInline(trimmedLine.slice(4), isUser)}
          </h4>
        );
        continue;
      } else if (trimmedLine.startsWith('## ')) {
        result.push(
          <h3
            key={`h3-${index}`}
            className={
              isUser
                ? 'text-sm font-bold text-white mt-3 mb-1'
                : 'text-sm font-bold text-[#171717] dark:text-[#f0f6fc] mt-3 mb-1'
            }
          >
            {renderInline(trimmedLine.slice(3), isUser)}
          </h3>
        );
        continue;
      } else if (trimmedLine.startsWith('# ')) {
        result.push(
          <h2
            key={`h2-${index}`}
            className={
              isUser
                ? 'text-sm md:text-base font-bold text-white mt-3 mb-1.5'
                : 'text-sm md:text-base font-bold text-[#171717] dark:text-[#f0f6fc] mt-3 mb-1.5'
            }
          >
            {renderInline(trimmedLine.slice(2), isUser)}
          </h2>
        );
        continue;
      }

      // Bullet lists & Task checkboxes
      if (bulletRegex.test(line)) {
        const match = line.match(bulletRegex)!;
        const indent = match[1].length;
        const mlClass = indent >= 4 ? 'ml-6' : indent >= 2 ? 'ml-4' : 'ml-2';
        const taskCheckboxMatch = match[3].match(/^\[([ xX])\]\s*(.*)$/);

        if (taskCheckboxMatch) {
          const isChecked = taskCheckboxMatch[1].toLowerCase() === 'x';
          const taskText = taskCheckboxMatch[2];
          result.push(
            <div
              key={`task-${index}`}
              className={`${mlClass} text-xs leading-relaxed flex items-start gap-1.5 my-0.5`}
            >
              <span className="shrink-0 mt-0.5 select-none">
                {isChecked ? (
                  <span
                    className={
                      isUser
                        ? 'w-3 h-3 rounded bg-white text-[#18181b] flex items-center justify-center'
                        : 'w-3 h-3 rounded bg-emerald-500 text-white flex items-center justify-center'
                    }
                  >
                    <Check className="w-2 h-2 stroke-[3]" />
                  </span>
                ) : (
                  <span
                    className={
                      isUser
                        ? 'w-3 h-3 rounded border border-white/60 block bg-transparent'
                        : 'w-3 h-3 rounded border border-[#a1a1aa] dark:border-[#52525b] block bg-transparent'
                    }
                  />
                )}
              </span>
              <div
                className={`flex-1 ${
                  isChecked
                    ? isUser
                      ? 'line-through text-white/60'
                      : 'line-through text-[#8f8f8f] dark:text-[#8b949e]'
                    : isUser
                    ? 'text-white'
                    : 'text-[#171717] dark:text-[#f0f6fc]'
                }`}
              >
                {renderInline(taskText, isUser)}
              </div>
            </div>
          );
        } else {
          result.push(
            <div
              key={`bullet-${index}`}
              className={`${mlClass} text-xs leading-relaxed ${
                isUser ? 'text-white' : 'text-[#171717] dark:text-[#f0f6fc]'
              } flex items-start gap-1.5 my-0.5`}
            >
              <span className={isUser ? 'text-violet-300 font-bold select-none' : 'text-[#7c3aed] font-bold select-none'}>•</span>
              <div className="flex-1">{renderInline(match[3], isUser)}</div>
            </div>
          );
        }
        continue;
      }

      // Numbered lists
      if (numberedRegex.test(line)) {
        const match = line.match(numberedRegex)!;
        const indent = match[1].length;
        const mlClass = indent >= 4 ? 'ml-6' : indent >= 2 ? 'ml-4' : 'ml-2';
        result.push(
          <div
            key={`num-${index}`}
            className={`${mlClass} text-xs leading-relaxed ${
              isUser ? 'text-white' : 'text-[#171717] dark:text-[#f0f6fc]'
            } flex items-start gap-1.5 my-0.5`}
          >
            <span
              className={
                isUser
                  ? 'font-mono text-[11px] text-white/70 shrink-0 select-none'
                  : 'font-mono text-[11px] text-[#666666] dark:text-[#8b949e] shrink-0 select-none'
              }
            >
              {match[2]}
            </span>
            <div className="flex-1">{renderInline(match[4], isUser)}</div>
          </div>
        );
        continue;
      }

      // Paragraph
      result.push(
        <p
          key={`p-${index}`}
          className={`text-xs leading-relaxed ${
            isUser ? 'text-white' : 'text-[#171717] dark:text-[#f0f6fc]'
          } my-0.5 select-text`}
        >
          {renderInline(line, isUser)}
        </p>
      );
    }

    if (inCodeBlock && codeBuffer.length > 0) {
      result.push(
        <div
          key="code-unfinished"
          className={
            isUser
              ? 'my-2 rounded-md border border-white/20 bg-black/40 overflow-hidden font-mono text-[12px]'
              : 'my-2 rounded-md border border-[#ebebeb] dark:border-[#21262d] bg-[#fafafa] dark:bg-[#0d1117] overflow-hidden font-mono text-[12px]'
          }
        >
          <pre
            className={
              isUser
                ? 'p-2.5 text-white overflow-x-auto leading-relaxed font-mono text-[12px]'
                : 'p-2.5 text-[#171717] dark:text-[#f0f6fc] overflow-x-auto leading-relaxed font-mono text-[12px]'
            }
          >
            {codeBuffer.join('\n')}
          </pre>
        </div>
      );
    }

    return result;
  }, [content, isUser]);

  if (!elements) return null;

  return (
    <div
      className={`space-y-0.5 text-xs ${
        isUser ? 'text-white' : 'text-[#171717] dark:text-[#f0f6fc]'
      } leading-relaxed break-words`}
    >
      {elements}
    </div>
  );
}

function renderInline(text: string, isUserVariant = false): React.ReactNode {
  if (!text) return null;

  // Split on inline code, bold, italics, links
  const parts: React.ReactNode[] = [];
  let remaining = text;
  let keyIdx = 0;

  while (remaining.length > 0) {
    // 1. Inline Code `code`
    const codeMatch = remaining.match(/^(.*?)`([^`\n]+)`(.*)$/s);
    // 2. Bold **text**
    const boldMatch = remaining.match(/^(.*?)\*\*([^\*\n]+)\*\*(.*)$/s);
    // 3. Link [label](url)
    const linkMatch = remaining.match(/^(.*?)\[([^\]\n]+)\]\(([^)\n]+)\)(.*)$/s);

    // Pick closest match
    let closestType: 'none' | 'code' | 'bold' | 'link' = 'none';
    let minPos = remaining.length;

    if (codeMatch && codeMatch[1].length < minPos) {
      minPos = codeMatch[1].length;
      closestType = 'code';
    }
    if (boldMatch && boldMatch[1].length < minPos) {
      minPos = boldMatch[1].length;
      closestType = 'bold';
    }
    if (linkMatch && linkMatch[1].length < minPos) {
      minPos = linkMatch[1].length;
      closestType = 'link';
    }

    if (closestType === 'none') {
      parts.push(<span key={keyIdx++}>{remaining}</span>);
      break;
    }

    if (closestType === 'code' && codeMatch) {
      if (codeMatch[1]) parts.push(<span key={keyIdx++}>{codeMatch[1]}</span>);
      parts.push(
        <code
          key={keyIdx++}
          className={
            isUserVariant
              ? 'px-1.5 py-0.5 mx-0.5 bg-white/15 border border-white/25 rounded text-[11px] font-mono text-white font-medium select-text'
              : 'px-1.5 py-0.5 mx-0.5 bg-[#f4f4f5] dark:bg-[#161b22] border border-[#e4e4e7] dark:border-[#30363d] rounded text-[11px] font-mono text-[#09090b] dark:text-[#f0f6fc] font-medium select-text'
          }
        >
          {codeMatch[2]}
        </code>
      );
      remaining = codeMatch[3];
    } else if (closestType === 'bold' && boldMatch) {
      if (boldMatch[1]) parts.push(<span key={keyIdx++}>{boldMatch[1]}</span>);
      parts.push(
        <strong
          key={keyIdx++}
          className={isUserVariant ? 'font-semibold text-white' : 'font-semibold text-[#171717] dark:text-[#f0f6fc]'}
        >
          {boldMatch[2]}
        </strong>
      );
      remaining = boldMatch[3];
    } else if (closestType === 'link' && linkMatch) {
      if (linkMatch[1]) parts.push(<span key={keyIdx++}>{linkMatch[1]}</span>);
      const isDangerous = /^\s*javascript:/i.test(linkMatch[3]);
      parts.push(
        <a
          key={keyIdx++}
          href={isDangerous ? '#' : linkMatch[3]}
          target="_blank"
          rel="noreferrer"
          className={
            isUserVariant
              ? 'text-violet-300 underline hover:text-violet-200 cursor-pointer'
              : 'text-[#7c3aed] underline hover:text-[#6d28d9] cursor-pointer'
          }
        >
          {linkMatch[2]}
        </a>
      );
      remaining = linkMatch[4];
    }
  }

  return parts;
}

export default MarkdownContent;
