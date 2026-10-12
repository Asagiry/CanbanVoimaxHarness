import { useState } from 'react';
import { Copy, Check, Terminal } from 'lucide-react';

interface MarkdownViewerProps {
  content: string;
}

export default function MarkdownViewer({ content }: MarkdownViewerProps) {
  // Simple, robust Markdown parser that renders headings, code blocks, lists, quotes, and inline styles
  const renderMarkdown = (text: string) => {
    const lines = text.split('\n');
    const elements: React.ReactNode[] = [];
    let inCodeBlock = false;
    let codeLanguage = '';
    let codeBuffer: string[] = [];
    let blockIndex = 0;

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];

      // Code block start/end
      if (line.trim().startsWith('```')) {
        if (inCodeBlock) {
          // Finish current code block
          const codeText = codeBuffer.join('\n');
          elements.push(
            <CodeSnippetCard
              key={`code-${blockIndex++}`}
              language={codeLanguage}
              code={codeText}
            />
          );
          codeBuffer = [];
          inCodeBlock = false;
          codeLanguage = '';
        } else {
          inCodeBlock = true;
          codeLanguage = line.trim().replace(/^```/, '').trim() || 'text';
        }
        continue;
      }

      if (inCodeBlock) {
        codeBuffer.push(line);
        continue;
      }

      // Headings
      if (line.startsWith('# ')) {
        elements.push(
          <h1 key={`h1-${i}`} className="text-base font-bold text-slate-100 mt-3 mb-1 border-b border-slate-800 pb-1">
            {formatInline(line.replace(/^#\s+/, ''))}
          </h1>
        );
      } else if (line.startsWith('## ')) {
        elements.push(
          <h2 key={`h2-${i}`} className="text-sm font-bold text-cyber-cyan mt-2.5 mb-1">
            {formatInline(line.replace(/^##\s+/, ''))}
          </h2>
        );
      } else if (line.startsWith('### ')) {
        elements.push(
          <h3 key={`h3-${i}`} className="text-xs font-bold text-slate-200 mt-2 mb-0.5">
            {formatInline(line.replace(/^###\s+/, ''))}
          </h3>
        );
      } else if (line.startsWith('> ')) {
        // Blockquote
        elements.push(
          <blockquote
            key={`quote-${i}`}
            className="border-l-2 border-cyan-500/60 bg-cyan-950/20 px-3 py-1 text-slate-300 italic text-xs my-1 rounded-r"
          >
            {formatInline(line.replace(/^>\s+/, ''))}
          </blockquote>
        );
      } else if (line.trim().startsWith('- ') || line.trim().startsWith('* ')) {
        // Unordered list
        elements.push(
          <li key={`li-${i}`} className="ml-4 list-disc text-slate-300 text-xs my-0.5">
            {formatInline(line.trim().replace(/^[-*]\s+/, ''))}
          </li>
        );
      } else if (/^\d+\.\s+/.test(line.trim())) {
        // Ordered list
        elements.push(
          <li key={`oli-${i}`} className="ml-4 list-decimal text-slate-300 text-xs my-0.5">
            {formatInline(line.trim().replace(/^\d+\.\s+/, ''))}
          </li>
        );
      } else if (line.trim() === '') {
        elements.push(<div key={`sp-${i}`} className="h-1.5" />);
      } else {
        // Normal paragraph
        elements.push(
          <p key={`p-${i}`} className="text-xs text-slate-200 leading-relaxed my-0.5">
            {formatInline(line)}
          </p>
        );
      }
    }

    // Flush unclosed code block if stream cut off
    if (inCodeBlock && codeBuffer.length > 0) {
      elements.push(
        <CodeSnippetCard
          key={`code-${blockIndex++}`}
          language={codeLanguage}
          code={codeBuffer.join('\n')}
        />
      );
    }

    return elements;
  };

  const formatInline = (str: string): React.ReactNode => {
    // Replace inline code `...`
    const parts = str.split(/(`[^`]+`)/);
    return parts.map((part, idx) => {
      if (part.startsWith('`') && part.endsWith('`') && part.length > 1) {
        return (
          <code
            key={idx}
            className="px-1 py-0.5 mx-0.5 rounded bg-obsidian-800 border border-slate-750 font-mono text-[11px] text-cyber-cyan font-medium"
          >
            {part.slice(1, -1)}
          </code>
        );
      }

      // Bold text **...**
      const boldParts = part.split(/(\*\*[^*]+\*\*)/);
      return boldParts.map((bPart, bIdx) => {
        if (bPart.startsWith('**') && bPart.endsWith('**') && bPart.length > 3) {
          return (
            <strong key={`${idx}-${bIdx}`} className="font-bold text-white">
              {bPart.slice(2, -2)}
            </strong>
          );
        }
        return bPart;
      });
    });
  };

  return <div className="space-y-0.5">{renderMarkdown(content)}</div>;
}

interface CodeSnippetCardProps {
  language: string;
  code: string;
}

function CodeSnippetCard({ language, code }: CodeSnippetCardProps) {
  const [copied, setCopied] = useState(false);

  const handleCopy = () => {
    navigator.clipboard.writeText(code);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="my-2 rounded-lg border border-slate-800 bg-obsidian-950 overflow-hidden shadow-sm">
      <div className="flex items-center justify-between px-3 py-1.5 bg-obsidian-900 border-b border-slate-800 text-[11px] font-mono text-slate-400">
        <div className="flex items-center space-x-1.5">
          <Terminal className="h-3 w-3 text-cyber-cyan" />
          <span className="font-semibold text-slate-300">{language || 'code'}</span>
        </div>
        <button
          onClick={handleCopy}
          className="flex items-center space-x-1 text-[10px] text-slate-400 hover:text-white transition cursor-pointer px-1.5 py-0.5 rounded hover:bg-obsidian-800"
          title="Copy code snippet"
        >
          {copied ? (
            <>
              <Check className="h-3 w-3 text-cyber-emerald" />
              <span className="text-cyber-emerald">Copied</span>
            </>
          ) : (
            <>
              <Copy className="h-3 w-3" />
              <span>Copy</span>
            </>
          )}
        </button>
      </div>
      <pre className="p-3 overflow-x-auto font-mono text-xs text-slate-200 leading-relaxed selection:bg-cyan-900/40">
        <code>{code}</code>
      </pre>
    </div>
  );
}
