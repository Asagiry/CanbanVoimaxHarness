import React, { useMemo } from 'react';

interface MarkdownContentProps {
  content?: string;
}

export function MarkdownContent({ content }: MarkdownContentProps) {
  const elements = useMemo(() => {
    if (!content) return null;
    const cleanContent = content.replace(/\r\n/g, '\n').replace(/\r/g, '\n');
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
              className="my-2 rounded-md border border-[#ebebeb] bg-[#fafafa] overflow-hidden font-mono text-[12px]"
            >
              {codeLanguage && (
                <div className="px-3 py-1 bg-[#f4f4f5] border-b border-[#ebebeb] text-[11px] text-[#666666] font-medium flex justify-between items-center">
                  <span>{codeLanguage}</span>
                </div>
              )}
              <pre className="p-2.5 text-[#171717] overflow-x-auto leading-relaxed select-text font-mono text-[12px]">
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
            <div key={`table-${index}`} className="my-2 overflow-x-auto rounded-md border border-[#ebebeb]">
              <table className="w-full border-collapse text-[12px] font-mono tabular-nums">
                <thead>
                  <tr>
                    {normHeader.map((cell, k) => (
                      <th
                        key={k}
                        style={{ textAlign: normAligns[k] }}
                        className="border border-[#ebebeb] bg-[#f9f9f9] px-2 py-1 font-semibold text-[#171717]"
                      >
                        {renderInline(cell)}
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
                          className="border border-[#ebebeb] px-2 py-1 text-[#171717]"
                        >
                          {renderInline(cell)}
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
        result.push(<hr key={`hr-${index}`} className="border-[#ebebeb] my-2" />);
        continue;
      }

      // Blockquotes
      if (trimmedLine.startsWith('>')) {
        const quoteText = trimmedLine.replace(/^>+\s*/, '');
        result.push(
          <blockquote
            key={`quote-${index}`}
            className="my-1.5 pl-3 border-l-2 border-[#7c3aed]/40 text-[#4d4d4d] italic text-xs leading-relaxed bg-[#fbfaff] py-1 pr-2 rounded-r"
          >
            {renderInline(quoteText)}
          </blockquote>
        );
        continue;
      }

      // Headings
      if (trimmedLine.startsWith('#### ')) {
        result.push(
          <h5 key={`h5-${index}`} className="text-xs font-semibold text-[#171717] mt-2 mb-0.5">
            {renderInline(trimmedLine.slice(5))}
          </h5>
        );
        continue;
      } else if (trimmedLine.startsWith('### ')) {
        result.push(
          <h4 key={`h4-${index}`} className="text-xs font-bold text-[#171717] mt-2.5 mb-1 flex items-center gap-1.5">
            {renderInline(trimmedLine.slice(4))}
          </h4>
        );
        continue;
      } else if (trimmedLine.startsWith('## ')) {
        result.push(
          <h3 key={`h3-${index}`} className="text-sm font-bold text-[#171717] mt-3 mb-1">
            {renderInline(trimmedLine.slice(3))}
          </h3>
        );
        continue;
      } else if (trimmedLine.startsWith('# ')) {
        result.push(
          <h2 key={`h2-${index}`} className="text-sm md:text-base font-bold text-[#171717] mt-3 mb-1.5">
            {renderInline(trimmedLine.slice(2))}
          </h2>
        );
        continue;
      }

      // Bullet lists
      if (bulletRegex.test(line)) {
        const match = line.match(bulletRegex)!;
        const indent = match[1].length;
        const mlClass = indent >= 4 ? 'ml-6' : indent >= 2 ? 'ml-4' : 'ml-2';
        result.push(
          <div key={`bullet-${index}`} className={`${mlClass} text-xs leading-relaxed text-[#171717] flex items-start gap-1.5 my-0.5`}>
            <span className="text-[#7c3aed] font-bold select-none">•</span>
            <div className="flex-1">{renderInline(match[3])}</div>
          </div>
        );
        continue;
      }

      // Numbered lists
      if (numberedRegex.test(line)) {
        const match = line.match(numberedRegex)!;
        const indent = match[1].length;
        const mlClass = indent >= 4 ? 'ml-6' : indent >= 2 ? 'ml-4' : 'ml-2';
        result.push(
          <div key={`num-${index}`} className={`${mlClass} text-xs leading-relaxed text-[#171717] flex items-start gap-1.5 my-0.5`}>
            <span className="font-mono text-[11px] text-[#666666] shrink-0 select-none">{match[2]}</span>
            <div className="flex-1">{renderInline(match[4])}</div>
          </div>
        );
        continue;
      }

      // Paragraph
      result.push(
        <p key={`p-${index}`} className="text-xs leading-relaxed text-[#171717] my-0.5 select-text">
          {renderInline(line)}
        </p>
      );
    }

    if (inCodeBlock && codeBuffer.length > 0) {
      result.push(
        <div
          key="code-unfinished"
          className="my-2 rounded-md border border-[#ebebeb] bg-[#fafafa] overflow-hidden font-mono text-[12px]"
        >
          <pre className="p-2.5 text-[#171717] overflow-x-auto leading-relaxed font-mono text-[12px]">
            {codeBuffer.join('\n')}
          </pre>
        </div>
      );
    }

    return result;
  }, [content]);

  return <div className="space-y-0.5 text-xs text-[#171717] leading-relaxed break-words">{elements}</div>;
}

function renderInline(text: string): React.ReactNode {
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
          className="px-1.5 py-0.5 mx-0.5 bg-[#f4f4f5] border border-[#e4e4e7] rounded text-[11px] font-mono text-[#09090b] font-medium select-text"
        >
          {codeMatch[2]}
        </code>
      );
      remaining = codeMatch[3];
    } else if (closestType === 'bold' && boldMatch) {
      if (boldMatch[1]) parts.push(<span key={keyIdx++}>{boldMatch[1]}</span>);
      parts.push(
        <strong key={keyIdx++} className="font-semibold text-[#171717]">
          {boldMatch[2]}
        </strong>
      );
      remaining = boldMatch[3];
    } else if (closestType === 'link' && linkMatch) {
      if (linkMatch[1]) parts.push(<span key={keyIdx++}>{linkMatch[1]}</span>);
      parts.push(
        <a
          key={keyIdx++}
          href={linkMatch[3]}
          target="_blank"
          rel="noreferrer"
          className="text-[#7c3aed] underline hover:text-[#6d28d9] cursor-pointer"
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
