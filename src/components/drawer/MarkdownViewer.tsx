import { useState } from 'react';
import { Copy, Check, Terminal } from 'lucide-react';

interface MarkdownViewerProps {
  content: string;
}

export default function MarkdownViewer({ content }: MarkdownViewerProps) {
  const [copiedIndex, setCopiedIndex] = useState<number | null>(null);

  const handleCopyCode = (code: string, idx: number) => {
    navigator.clipboard.writeText(code);
    setCopiedIndex(idx);
    setTimeout(() => setCopiedIndex(null), 2000);
  };

  // Parse markdown code blocks vs regular paragraphs
  const parts = content.split(/(```[\s\S]*?```)/g);

  return (
    <div className="space-y-2.5 text-xs text-slate-200 leading-relaxed font-sans select-text">
      {parts.map((part, index) => {
        if (part.startsWith('```') && part.endsWith('```')) {
          const firstLineEnd = part.indexOf('\n');
          const lang = part
            .substring(3, firstLineEnd > 0 ? firstLineEnd : 3)
            .trim() || 'text';
          const code = firstLineEnd > 0
            ? part.substring(firstLineEnd + 1, part.length - 3)
            : part.substring(3, part.length - 3);

          return (
            <div
              key={index}
              className="rounded-lg border border-slate-800 bg-obsidian-950 overflow-hidden font-mono text-[11px] my-2"
            >
              <div className="flex items-center justify-between px-3 py-1 bg-obsidian-900 border-b border-slate-800 text-[10px] text-slate-400">
                <div className="flex items-center space-x-1.5 font-bold uppercase text-cyber-cyan">
                  <Terminal className="h-3 w-3" />
                  <span>{lang}</span>
                </div>
                <button
                  onClick={() => handleCopyCode(code.trim(), index)}
                  className="flex items-center space-x-1 hover:text-white transition"
                  title="Copy code"
                >
                  {copiedIndex === index ? (
                    <>
                      <Check className="h-2.5 w-2.5 text-emerald-400" />
                      <span className="text-emerald-400">Copied!</span>
                    </>
                  ) : (
                    <>
                      <Copy className="h-2.5 w-2.5" />
                      <span>Copy</span>
                    </>
                  )}
                </button>
              </div>
              <pre className="p-3 overflow-x-auto text-slate-100 leading-relaxed whitespace-pre font-mono">
                <code>{code.trim()}</code>
              </pre>
            </div>
          );
        }

        // Render regular markdown text
        return (
          <div key={index} className="space-y-1.5">
            {part.split('\n\n').map((paragraph, pIdx) => {
              const trimmed = paragraph.trim();
              if (!trimmed) return null;

              if (trimmed.startsWith('### ')) {
                return (
                  <h4 key={pIdx} className="text-xs font-bold text-cyber-cyan mt-2 mb-1">
                    {trimmed.substring(4)}
                  </h4>
                );
              }
              if (trimmed.startsWith('## ')) {
                return (
                  <h3 key={pIdx} className="text-sm font-bold text-white mt-3 mb-1.5">
                    {trimmed.substring(3)}
                  </h3>
                );
              }
              if (trimmed.startsWith('# ')) {
                return (
                  <h2 key={pIdx} className="text-sm font-extrabold text-cyber-amber mt-3 mb-2">
                    {trimmed.substring(2)}
                  </h2>
                );
              }

              // Bullet points
              if (trimmed.startsWith('- ') || trimmed.startsWith('* ') || trimmed.startsWith('• ')) {
                const items = trimmed.split('\n');
                return (
                  <ul key={pIdx} className="list-disc list-inside space-y-1 pl-1 text-slate-300">
                    {items.map((it, itIdx) => (
                      <li key={itIdx}>
                        {renderInlineFormatting(it.replace(/^[-*•]\s+/, ''))}
                      </li>
                    ))}
                  </ul>
                );
              }

              return (
                <p key={pIdx} className="text-slate-300 leading-relaxed">
                  {renderInlineFormatting(trimmed)}
                </p>
              );
            })}
          </div>
        );
      })}
    </div>
  );
}

function renderInlineFormatting(text: string) {
  // Simple inline code & bold parser
  const tokens = text.split(/(`[^`]+`|\*\*[^*]+\*\*)/g);
  return tokens.map((token, i) => {
    if (token.startsWith('`') && token.endsWith('`')) {
      return (
        <code
          key={i}
          className="px-1.5 py-0.5 rounded bg-obsidian-850 border border-slate-750 font-mono text-cyber-cyan text-[11px]"
        >
          {token.slice(1, -1)}
        </code>
      );
    }
    if (token.startsWith('**') && token.endsWith('**')) {
      return (
        <strong key={i} className="font-bold text-white">
          {token.slice(2, -2)}
        </strong>
      );
    }
    return token;
  });
}
