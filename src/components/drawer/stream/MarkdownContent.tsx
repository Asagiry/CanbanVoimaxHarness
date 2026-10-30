import React, { useState, useRef, useEffect, useMemo, useContext } from 'react';
import { Check, Copy } from 'lucide-react';
import {
  parseMarkdownTokens,
  parseInlineTokens,
  type InlineNode,
} from '../../../lib/markdown';

export const LightboxContext = React.createContext<((src: string, alt?: string) => void) | null>(null);

export function renderInlineNodes(nodes: InlineNode[], prefix: string): React.ReactNode[] {
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
      case 'link': {
        const isDangerous = /^\s*javascript\s*:/i.test(node.url);
        return (
          <a
            key={key}
            href={isDangerous ? '#' : node.url}
            target="_blank"
            rel="noreferrer"
            className="text-[#7c3aed] dark:text-[#a78bfa] hover:underline"
          >
            {node.children.length > 0 ? renderInlineNodes(node.children, `${key}-l`) : node.text}
          </a>
        );
      }
      case 'text':
      default:
        return node.text;
    }
  });
}

export function formatInline(text: string, prefix = 'inl'): React.ReactNode[] {
  if (!text) return [];
  const nodes = parseInlineTokens(text);
  return renderInlineNodes(nodes, prefix);
}

function CodeBlock({
  code,
  language,
  unfinished,
}: {
  code: string;
  language?: string;
  unfinished?: boolean;
}) {
  const [copied, setCopied] = useState(false);
  const copyTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (copyTimeoutRef.current) {
        clearTimeout(copyTimeoutRef.current);
      }
    };
  }, []);

  const handleCopy = (e: React.MouseEvent) => {
    e.stopPropagation();
    navigator.clipboard.writeText(code);
    setCopied(true);
    if (copyTimeoutRef.current) {
      clearTimeout(copyTimeoutRef.current);
    }
    copyTimeoutRef.current = setTimeout(() => {
      setCopied(false);
      copyTimeoutRef.current = null;
    }, 1500);
  };

  return (
    <div
      className={
        unfinished
          ? 'my-2.5 rounded-md border border-[#ebebeb] dark:border-[#21262d] bg-[#fafafa] dark:bg-[#0d1117] overflow-hidden font-mono text-[13px]'
          : 'my-2.5 rounded-md border border-[#ebebeb] dark:border-[#30363d] bg-[#fafafa] dark:bg-[#161b22] overflow-hidden font-mono text-[13px]'
      }
    >
      {(language || unfinished) ? (
        <div
          className={
            unfinished
              ? 'px-3 py-1 bg-[#f4f4f5] dark:bg-[#161b22] border-b border-[#ebebeb] dark:border-[#21262d] text-xs text-[#666666] dark:text-[#8b949e] font-medium flex justify-between items-center'
              : 'px-3 py-1 bg-[#f4f4f5] dark:bg-[#0d1117] border-b border-[#ebebeb] dark:border-[#30363d] text-xs text-[#666666] dark:text-[#8b949e] font-medium flex justify-between items-center'
          }
        >
          <span>{language || (unfinished ? 'streaming...' : '')}</span>
          <button
            type="button"
            onClick={handleCopy}
            aria-label="Copy code block"
            className="flex items-center space-x-1 p-0.5 text-[11px] text-[#666666] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] transition cursor-pointer"
          >
            {copied ? (
              <Check className="h-3 w-3 text-emerald-600 dark:text-emerald-400" />
            ) : (
              <Copy className="h-3 w-3" />
            )}
          </button>
        </div>
      ) : (
        <div className="px-3 py-1 bg-[#f4f4f5] dark:bg-[#0d1117] border-b border-[#ebebeb] dark:border-[#30363d] text-xs text-[#666666] dark:text-[#8b949e] font-medium flex justify-end items-center">
          <button
            type="button"
            onClick={handleCopy}
            aria-label="Copy code block"
            className="flex items-center space-x-1 p-0.5 text-[11px] text-[#666666] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] transition cursor-pointer"
          >
            {copied ? (
              <Check className="h-3 w-3 text-emerald-600 dark:text-emerald-400" />
            ) : (
              <Copy className="h-3 w-3" />
            )}
          </button>
        </div>
      )}
      <pre className="p-3 text-[#171717] dark:text-[#f0f6fc] overflow-x-auto leading-relaxed select-text font-mono text-[13px]">
        {code}
      </pre>
    </div>
  );
}

export interface MarkdownContentProps {
  content?: string;
  onImageClick?: (src: string, alt?: string) => void;
}

// Lightweight Markdown Renderer for streaming assistant output
export function MarkdownContent({
  content,
  onImageClick,
}: MarkdownContentProps) {
  const contextOnImageClick = useContext(LightboxContext);
  const handleImageClick = onImageClick || contextOnImageClick;

  const tokens = useMemo(() => parseMarkdownTokens(content ?? ''), [content]);

  return (
    <div className="space-y-1 select-text font-sans">
      {tokens.map((token, index) => {
        switch (token.type) {
          case 'code':
            return (
              <CodeBlock
                key={`code-${index}`}
                code={token.code}
                language={token.language}
                unfinished={token.unfinished}
              />
            );

          case 'table':
            return (
              <div
                key={`table-${index}`}
                className="my-2 overflow-x-auto rounded-md border border-[#ebebeb] dark:border-[#21262d]"
              >
                <table className="w-full border-collapse text-[13px] font-mono tabular-nums">
                  <thead>
                    <tr>
                      {token.header.map((cell, k) => (
                        <th
                          key={k}
                          style={{ textAlign: token.aligns[k] }}
                          className="border border-[#ebebeb] dark:border-[#21262d] bg-[#f9f9f9] dark:bg-[#161b22] px-2 py-1.5 font-semibold text-[#171717] dark:text-[#f0f6fc]"
                        >
                          {formatInline(cell, `th-${index}-${k}`)}
                        </th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {token.rows.map((row, r) => (
                      <tr key={r}>
                        {row.map((cell, k) => (
                          <td
                            key={k}
                            style={{ textAlign: token.aligns[k] }}
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

          case 'hr':
            return <hr key={`hr-${index}`} className="border-[#ebebeb] dark:border-[#21262d] my-2.5" />;

          case 'blockquote':
            return (
              <blockquote
                key={`quote-${index}`}
                className="my-2 border-l-2 border-[#7c3aed]/50 bg-[#f9f9fb] dark:bg-[#161b22]/40 rounded-r-md px-3.5 py-2 text-sm italic text-[#666666] dark:text-[#8b949e] space-y-1"
              >
                {token.lines.map((ql, qIdx) => (
                  <div key={qIdx} className="leading-relaxed">
                    {ql.trim() === '' ? <div className="h-1" /> : formatInline(ql, `quote-${index}-${qIdx}`)}
                  </div>
                ))}
              </blockquote>
            );

          case 'image':
            return (
              <div
                key={`img-${index}`}
                onClick={() => handleImageClick?.(token.src, token.alt)}
                className="my-2 rounded-lg overflow-hidden border border-[#ebebeb] dark:border-[#21262d] max-w-lg bg-[#f4f4f5] dark:bg-[#161b22] shadow-xs group cursor-zoom-in"
              >
                <img
                  src={token.src}
                  alt={token.alt}
                  className="w-full max-h-80 object-contain cursor-zoom-in group-hover:scale-[1.01] transition-transform duration-200"
                />
              </div>
            );

          case 'heading': {
            if (token.level === 5) {
              return (
                <h5 key={`h5-${index}`} className="text-[13px] md:text-sm font-semibold text-[#171717] dark:text-[#f0f6fc] mt-2 mb-1">
                  {formatInline(token.text, `h5-${index}`)}
                </h5>
              );
            }
            if (token.level === 4) {
              return (
                <h4 key={`h4-${index}`} className="text-sm font-semibold text-[#171717] dark:text-[#f0f6fc] mt-2.5 mb-1">
                  {formatInline(token.text, `h4-${index}`)}
                </h4>
              );
            }
            if (token.level === 3) {
              return (
                <h3 key={`h3-${index}`} className="text-sm md:text-base font-semibold text-[#171717] dark:text-[#f0f6fc] mt-3 mb-1">
                  {formatInline(token.text, `h3-${index}`)}
                </h3>
              );
            }
            return (
              <h2 key={`h2-${index}`} className="text-base md:text-lg font-bold text-[#171717] dark:text-[#f0f6fc] mt-3.5 mb-1.5">
                {formatInline(token.text, `h2-${index}`)}
              </h2>
            );
          }

          case 'numbered': {
            const mlClass = token.indent >= 4 ? 'ml-8' : token.indent >= 2 ? 'ml-6' : 'ml-4';
            return (
              <div key={`num-${index}`} className={`${mlClass} text-sm leading-relaxed text-[#171717] dark:text-[#f0f6fc] flex items-start gap-1.5`}>
                <span className="font-semibold text-[#171717] dark:text-[#f0f6fc] select-none shrink-0 font-mono text-[13px]">
                  {token.number}
                </span>
                <span className="flex-1 min-w-0">{formatInline(token.text, `num-${index}`)}</span>
              </div>
            );
          }

          case 'task': {
            const mlClass = token.indent >= 4 ? 'ml-8' : token.indent >= 2 ? 'ml-6' : 'ml-4';
            return (
              <div
                key={`task-${index}`}
                className={`${mlClass} text-sm leading-relaxed flex items-start gap-2 my-0.5`}
              >
                {token.checked ? (
                  <span className="w-4 h-4 rounded bg-[#7c3aed]/15 dark:bg-[#7c3aed]/25 border border-[#7c3aed]/50 text-[#7c3aed] dark:text-[#a78bfa] flex items-center justify-center shrink-0 mt-0.5 select-none">
                    <Check className="w-2.5 h-2.5 stroke-[3]" />
                  </span>
                ) : (
                  <span className="w-4 h-4 rounded border border-[#d4d4d8] dark:border-[#30363d] bg-transparent shrink-0 mt-0.5 select-none" />
                )}
                <span
                  className={`flex-1 min-w-0 ${
                    token.checked ? 'line-through text-[#71717a] dark:text-[#8b949e]' : 'text-[#171717] dark:text-[#f0f6fc]'
                  }`}
                >
                  {formatInline(token.text, `task-${index}`)}
                </span>
              </div>
            );
          }

          case 'bullet': {
            const mlClass = token.indent >= 4 ? 'ml-8' : token.indent >= 2 ? 'ml-6' : 'ml-4';
            return (
              <div key={`bullet-${index}`} className={`${mlClass} text-sm leading-relaxed text-[#171717] dark:text-[#f0f6fc] flex items-start gap-2`}>
                <span className="text-[#666666] dark:text-[#8b949e] select-none shrink-0 leading-relaxed">•</span>
                <span className="flex-1 min-w-0">{formatInline(token.text, `bullet-${index}`)}</span>
              </div>
            );
          }

          case 'blank':
            return <div key={`blank-${index}`} className="h-1.5" />;

          case 'paragraph':
          default:
            return (
              <p key={`p-${index}`} className="text-sm leading-relaxed text-[#171717] dark:text-[#f0f6fc]">
                {formatInline(token.text, `p-${index}`)}
              </p>
            );
        }
      })}
    </div>
  );
}
