import React, { useState, useRef, useEffect } from 'react';
import { Check, Copy } from 'lucide-react';
import {
  parseMarkdownTokens,
  type MarkdownToken,
  type ParseMarkdownOptions,
} from '../../../lib/markdown';

export const LightboxContext = React.createContext<((src: string, alt?: string) => void) | null>(null);

interface CachedMarkdownEntry {
  source: string;
  normalizeEscapedNewlines: boolean;
  tokens: MarkdownToken[];
}

const MARKDOWN_CACHE_MAX = 100;
const markdownTokenCache = new Map<string, CachedMarkdownEntry>();

export function hashContent(content: string): string {
  let h1 = 0x811c9dc5;
  let h2 = 0x01000193;
  for (let i = 0; i < content.length; i++) {
    const c = content.charCodeAt(i);
    h1 = Math.imul(h1 ^ c, 0x01000193);
    h2 = Math.imul(h2 ^ c, 0x85ebca6b);
  }
  return `${content.length}:${(h1 >>> 0).toString(36)}${(h2 >>> 0).toString(36)}`;
}

export function getCachedMarkdownTokens(
  content: string,
  options: ParseMarkdownOptions = {}
): MarkdownToken[] {
  const normalizeEscapedNewlines = Boolean(options.normalizeEscapedNewlines);
  const key = hashContent(content);
  const cached = markdownTokenCache.get(key);
  if (
    cached &&
    cached.source === content &&
    cached.normalizeEscapedNewlines === normalizeEscapedNewlines
  ) {
    markdownTokenCache.delete(key);
    markdownTokenCache.set(key, cached);
    return cached.tokens;
  }
  const tokens = parseMarkdownTokens(content, { normalizeEscapedNewlines });
  markdownTokenCache.set(key, { source: content, normalizeEscapedNewlines, tokens });
  if (markdownTokenCache.size > MARKDOWN_CACHE_MAX) {
    const oldest = markdownTokenCache.keys().next().value;
    if (oldest !== undefined) markdownTokenCache.delete(oldest);
  }
  return tokens;
}

export function clearMarkdownTokenCache(): void {
  markdownTokenCache.clear();
}

export function MarkdownCodeBlock({
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
    void navigator.clipboard.writeText(code).catch((err) => {
      console.warn('[MarkdownCodeBlock] clipboard copy failed:', err);
    });
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
          ? 'my-2.5 w-full max-w-full rounded-md border border-[#ebebeb] dark:border-[#212126] bg-[#fafafa] dark:bg-[#111114] overflow-hidden font-mono text-[13px]'
          : 'my-2.5 w-full max-w-full rounded-md border border-[#ebebeb] dark:border-[#2c2c33] bg-[#fafafa] dark:bg-[#17171b] overflow-hidden font-mono text-[13px]'
      }
    >
      {(language || unfinished) ? (
        <div
          className={
            unfinished
              ? 'px-3 py-1 bg-[#f4f4f5] dark:bg-[#17171b] border-b border-[#ebebeb] dark:border-[#212126] text-xs text-[#666666] dark:text-[#71717a] font-medium flex justify-between items-center'
              : 'px-3 py-1 bg-[#f4f4f5] dark:bg-[#111114] border-b border-[#ebebeb] dark:border-[#2c2c33] text-xs text-[#666666] dark:text-[#71717a] font-medium flex justify-between items-center'
          }
        >
          <span>{language || (unfinished ? 'streaming...' : '')}</span>
          <button
            type="button"
            onClick={handleCopy}
            aria-label="Copy code block"
            className="flex items-center space-x-1 p-0.5 text-[11px] text-[#666666] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] transition cursor-pointer"
          >
            {copied ? (
              <Check className="h-3 w-3 text-emerald-600 dark:text-emerald-400" />
            ) : (
              <Copy className="h-3 w-3" />
            )}
          </button>
        </div>
      ) : (
        <div className="px-3 py-1 bg-[#f4f4f5] dark:bg-[#111114] border-b border-[#ebebeb] dark:border-[#2c2c33] text-xs text-[#666666] dark:text-[#71717a] font-medium flex justify-end items-center">
          <button
            type="button"
            onClick={handleCopy}
            aria-label="Copy code block"
            className="flex items-center space-x-1 p-0.5 text-[11px] text-[#666666] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] transition cursor-pointer"
          >
            {copied ? (
              <Check className="h-3 w-3 text-emerald-600 dark:text-emerald-400" />
            ) : (
              <Copy className="h-3 w-3" />
            )}
          </button>
        </div>
      )}
      <pre className="p-3 text-[#171717] dark:text-[#f4f4f5] overflow-x-auto leading-relaxed select-text font-mono text-[13px]">
        {code}
      </pre>
    </div>
  );
}
