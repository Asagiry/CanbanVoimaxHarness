import React, { useMemo, useContext } from 'react';
import { Check } from 'lucide-react';
import {
  parseInlineTokens,
  sanitizeMarkdownUrl,
  type InlineNode,
} from '../../../lib/markdown';
import {
  LightboxContext,
  MarkdownCodeBlock,
  hashContent,
  getCachedMarkdownTokens,
  clearMarkdownTokenCache,
} from './MarkdownCodeBlock';

export {
  LightboxContext,
  hashContent,
  getCachedMarkdownTokens,
  clearMarkdownTokenCache,
};

export function renderInlineNodes(nodes: InlineNode[], prefix: string): React.ReactNode[] {
  return nodes.map((node, i) => {
    const key = `${prefix}-${node.type}-${i}`;
    switch (node.type) {
      case 'code':
        return (
          <code
            key={key}
            className="px-1.5 py-0.5 rounded bg-[#f4f4f5] dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] text-[#171717] dark:text-[#f4f4f5] font-mono text-[13px] break-all"
          >
            {node.text}
          </code>
        );
      case 'bold':
        return (
          <strong key={key} className="font-semibold text-[#171717] dark:text-[#f4f4f5]">
            {renderInlineNodes(node.children, `${key}-b`)}
          </strong>
        );
      case 'italic':
        return (
          <em key={key} className="italic text-[#171717] dark:text-[#f4f4f5]">
            {renderInlineNodes(node.children, `${key}-i`)}
          </em>
        );
      case 'link': {
        const safeHref = sanitizeMarkdownUrl(node.url);
        return (
          <a
            key={key}
            href={safeHref}
            target={safeHref === '#' ? undefined : '_blank'}
            rel={safeHref === '#' ? undefined : 'noopener noreferrer'}
            className="text-sky-600 dark:text-sky-400 hover:underline"
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

export interface MarkdownContentProps {
  content?: string;
  onImageClick?: (src: string, alt?: string) => void;
  deferred?: boolean;
  normalizeEscapedNewlines?: boolean;
}

// Lightweight Markdown Renderer for streaming assistant output
export function MarkdownContent({
  content,
  onImageClick,
  deferred,
  normalizeEscapedNewlines,
}: MarkdownContentProps) {
  const contextOnImageClick = useContext(LightboxContext);
  const handleImageClick = onImageClick || contextOnImageClick;

  const tokens = useMemo(
    () => (deferred ? [] : getCachedMarkdownTokens(content ?? '', { normalizeEscapedNewlines })),
    [content, deferred, normalizeEscapedNewlines]
  );

  if (deferred) return null;

  return (
    <div className="space-y-1 select-text font-sans break-words [overflow-wrap:anywhere] min-w-0">
      {tokens.map((token, index) => {
        switch (token.type) {
          case 'code':
            return (
              <MarkdownCodeBlock
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
                className="my-2 w-full max-w-full overflow-x-auto rounded-md border border-[#ebebeb] dark:border-[#212126]"
              >
                <table className="w-full border-collapse text-[13px] font-mono tabular-nums">
                  <thead>
                    <tr>
                      {token.header.map((cell, k) => (
                        <th
                          key={`th-${index}-${k}-${cell}`}
                          style={{ textAlign: token.aligns[k] }}
                          className="border border-[#ebebeb] dark:border-[#212126] bg-[#f9f9f9] dark:bg-[#17171b] px-2 py-1.5 font-semibold text-[#171717] dark:text-[#f4f4f5]"
                        >
                          {formatInline(cell, `th-${index}-${k}`)}
                        </th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {token.rows.map((row, r) => (
                      <tr key={`tr-${index}-${r}`}>
                        {row.map((cell, k) => (
                          <td
                            key={`td-${index}-${r}-${k}`}
                            style={{ textAlign: token.aligns[k] }}
                            className="border border-[#ebebeb] dark:border-[#212126] px-2 py-1.5 text-[#171717] dark:text-[#f4f4f5]"
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
            return <hr key={`hr-${index}`} className="border-[#ebebeb] dark:border-[#212126] my-2.5" />;

          case 'blockquote':
            return (
              <blockquote
                key={`quote-${index}`}
                className="my-2 border-l-2 border-[#a1a1aa] dark:border-[#52525b] bg-[#f9f9fb] dark:bg-[#17171b]/40 rounded-r-md px-3.5 py-2 text-sm italic text-[#666666] dark:text-[#71717a] space-y-1"
              >
                {token.lines.map((ql, qIdx) => (
                  <div key={`ql-${index}-${qIdx}`} className="leading-relaxed">
                    {ql.trim() === '' ? <div className="h-1" /> : formatInline(ql, `quote-${index}-${qIdx}`)}
                  </div>
                ))}
              </blockquote>
            );

          case 'image': {
            const safeImgSrc = sanitizeMarkdownUrl(token.src, { allowDataImage: true });
            if (!safeImgSrc) {
              return null;
            }
            const imgElement = (
              <img
                src={safeImgSrc}
                alt={token.alt}
                className="w-full max-h-80 object-contain cursor-zoom-in group-hover:scale-[1.01] transition-transform duration-200"
              />
            );
            if (handleImageClick) {
              return (
                <button
                  key={`img-${index}`}
                  type="button"
                  onClick={() => handleImageClick(safeImgSrc, token.alt)}
                  aria-label={token.alt}
                  className="block my-2 rounded-lg overflow-hidden border border-[#ebebeb] dark:border-[#212126] max-w-full max-w-lg bg-[#f4f4f5] dark:bg-[#17171b] shadow-xs group cursor-zoom-in text-left"
                >
                  {imgElement}
                </button>
              );
            }
            return (
              <div
                key={`img-${index}`}
                className="my-2 rounded-lg overflow-hidden border border-[#ebebeb] dark:border-[#212126] max-w-full max-w-lg bg-[#f4f4f5] dark:bg-[#17171b] shadow-xs"
              >
                {imgElement}
              </div>
            );
          }

          case 'heading': {
            if (token.level === 6) {
              return (
                <h6 key={`h6-${index}`} className="text-xs font-semibold text-[#171717] dark:text-[#f4f4f5] mt-2 mb-1 first:mt-0">
                  {formatInline(token.text, `h6-${index}`)}
                </h6>
              );
            }
            if (token.level === 5) {
              return (
                <h5 key={`h5-${index}`} className="text-[13px] md:text-sm font-semibold text-[#171717] dark:text-[#f4f4f5] mt-2 mb-1 first:mt-0">
                  {formatInline(token.text, `h5-${index}`)}
                </h5>
              );
            }
            if (token.level === 4) {
              return (
                <h4 key={`h4-${index}`} className="text-sm font-semibold text-[#171717] dark:text-[#f4f4f5] mt-2.5 mb-1 first:mt-0">
                  {formatInline(token.text, `h4-${index}`)}
                </h4>
              );
            }
            if (token.level === 3) {
              return (
                <h3 key={`h3-${index}`} className="text-sm md:text-base font-semibold text-[#171717] dark:text-[#f4f4f5] mt-3 mb-1 first:mt-0">
                  {formatInline(token.text, `h3-${index}`)}
                </h3>
              );
            }
            return (
              <h2 key={`h2-${index}`} className="text-base md:text-lg font-bold text-[#171717] dark:text-[#f4f4f5] mt-3.5 mb-1.5 first:mt-0">
                {formatInline(token.text, `h2-${index}`)}
              </h2>
            );
          }

          case 'numbered': {
            const mlClass = token.indent >= 4 ? 'ml-8' : token.indent >= 2 ? 'ml-6' : 'ml-4';
            return (
              <div key={`num-${index}`} className={`${mlClass} text-sm leading-relaxed text-[#171717] dark:text-[#f4f4f5] flex items-start gap-1.5`}>
                <span className="font-semibold text-[#171717] dark:text-[#f4f4f5] select-none shrink-0 font-mono text-[13px]">
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
                  <span className="w-4 h-4 rounded bg-[#f0fdf4] dark:bg-[#12281e] border border-[#bbf7d0] dark:border-[#238636] text-[#15803d] dark:text-[#7ee787] flex items-center justify-center shrink-0 mt-0.5 select-none">
                    <Check className="w-2.5 h-2.5 stroke-[3]" />
                  </span>
                ) : (
                  <span className="w-4 h-4 rounded border border-[#d4d4d8] dark:border-[#2c2c33] bg-transparent shrink-0 mt-0.5 select-none" />
                )}
                <span
                  className={`flex-1 min-w-0 ${
                    token.checked ? 'line-through text-[#71717a] dark:text-[#71717a]' : 'text-[#171717] dark:text-[#f4f4f5]'
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
              <div key={`bullet-${index}`} className={`${mlClass} text-sm leading-relaxed text-[#171717] dark:text-[#f4f4f5] flex items-start gap-2`}>
                <span className="text-[#666666] dark:text-[#71717a] select-none shrink-0 leading-relaxed">•</span>
                <span className="flex-1 min-w-0">{formatInline(token.text, `bullet-${index}`)}</span>
              </div>
            );
          }

          case 'blank':
            return <div key={`blank-${index}`} className="h-1.5" />;

          case 'paragraph':
          default:
            return (
              <p key={`p-${index}`} className="text-sm leading-relaxed text-[#171717] dark:text-[#f4f4f5] break-words">
                {formatInline(token.text, `p-${index}`)}
              </p>
            );
        }
      })}
    </div>
  );
}
