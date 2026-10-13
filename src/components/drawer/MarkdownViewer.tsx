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
          <h1 key={`h1-${i}`} className="text-base font-bold text-[#171717] mt-3 mb-1 border-b border-[#ebebeb] pb-1">
            {formatInline(line.replace(/^#\s+/, ''))}
          </h1>
        );
      } else if (line.startsWith('## ')) {
        elements.push(
          <h2 key={`h2-${i}`} className="text-sm font-bold text-[#171717] mt-2.5 mb-1">
            {formatInline(line.replace(/^##\s+/, ''))}
          </h2>
        );
      } else if (line.startsWith('### ')) {
        elements.push(
          <h3 key={`h3-${i}`} className="text-xs font-bold text-[#171717] mt-2 mb-0.5">
            {formatInline(line.replace(/^###\s+/, ''))}
          </h3>
        );
      } else if (line.startsWith('> ')) {
        // Blockquote
        elements.push(
          <blockquote
            key={`quote-${i}`}
            className="border-l-2 border-[#171717] bg-[#f9f9f9] px-3 py-1 text-[#4d4d4d] italic text-xs my-1 rounded-r"
          >
            {formatInline(line.replace(/^>\s+/, ''))}
          </blockquote>
        );
      } else if (line.trim().startsWith('- ') || line.trim().startsWith('* ')) {
        // Unordered list
        elements.push(
          <li key={`li-${i}`} className="ml-4 list-disc text-[#171717] text-xs my-0.5">
            {formatInline(line.trim().replace(/^[-*]\s+/, ''))}
          </li>
        );
      } else if (/^\d+\.\s+/.test(line.trim())) {
        // Ordered list
        elements.push(
          <li key={`oli-${i}`} className="ml-4 list-decimal text-[#171717] text-xs my-0.5">
            {formatInline(line.trim().replace(/^\d+\.\s+/, ''))}
          </li>
        );
      } else if (line.trim() === '') {
        elements.push(<div key={`sp-${i}`} className="h-1.5" />);
      } else {
        // Normal paragraph
        elements.push(
          <p key={`p-${i}`} className="text-xs text-[#171717] leading-relaxed my-0.5">
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
            className="px-1.5 py-0.5 mx-0.5 rounded-md bg-[#f4f4f5] border border-[#ebebeb] font-mono text-[11px] text-[#171717] font-medium"
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
            <strong key={`${idx}-${bIdx}`} className="font-semibold text-[#171717]">
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
    <div className="my-2.5 rounded-lg border border-[#ebebeb] bg-[#fafafa] overflow-hidden shadow-xs">
      <div className="flex items-center justify-between px-3 py-1.5 bg-[#f4f4f5] border-b border-[#ebebeb] text-[11px] font-mono text-[#666666]">
        <div className="flex items-center space-x-1.5">
          <Terminal className="h-3.5 w-3.5 text-[#171717]" />
          <span className="font-medium text-[#171717]">{language || 'code'}</span>
        </div>
        <button
          onClick={handleCopy}
          className="flex items-center space-x-1 text-[10px] text-[#666666] hover:text-[#171717] transition cursor-pointer px-1.5 py-0.5 rounded-md hover:bg-white"
          title="Скопировать фрагмент"
        >
          {copied ? (
            <>
              <Check className="h-3 w-3 text-[#15803d]" />
              <span className="text-[#15803d]">Скопировано</span>
            </>
          ) : (
            <>
              <Copy className="h-3 w-3" />
              <span>Копировать</span>
            </>
          )}
        </button>
      </div>
      <pre className="p-3 overflow-x-auto font-mono text-xs text-[#171717] leading-relaxed selection:bg-[#e5e5e5]">
        <code>{code}</code>
      </pre>
    </div>
  );
}
