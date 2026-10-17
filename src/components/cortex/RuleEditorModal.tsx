import React, { useState, useEffect } from 'react';
import { useCortexStore } from '../../stores/cortexStore';
import {
  X,
  Save,
  Check,
  FileText,
  Eye,
  Edit3,
  Zap,
} from 'lucide-react';

function isValidInlineCode(code: string): boolean {
  if (!code || code.length > 120) return false;
  if (/[\r\n]/.test(code)) return false;
  if (/[.!?]\s+/.test(code)) return false;
  if (code.includes('`')) return false;
  if (/[а-яА-ЯёЁ]{2,}\s+[а-яА-ЯёЁ]{2,}/.test(code) && !/^["'].*["']$/.test(code.trim())) {
    return false;
  }
  return true;
}

function renderInlineMarkdown(text: string): React.ReactNode[] {
  const nodes: React.ReactNode[] = [];
  let i = 0;
  let textBuf = '';

  const flushText = () => {
    if (textBuf) {
      nodes.push(textBuf);
      textBuf = '';
    }
  };

  while (i < text.length) {
    if (text[i] === '`') {
      const closeIdx = text.indexOf('`', i + 1);
      if (closeIdx !== -1) {
        const candidate = text.slice(i + 1, closeIdx);
        if (isValidInlineCode(candidate)) {
          flushText();
          nodes.push(
            <code
              key={`code-${i}`}
              className="px-1.5 py-0.5 bg-[#f4f4f5] border border-[#ebebeb] rounded text-xs font-mono text-[#09090b]"
            >
              {candidate}
            </code>
          );
          i = closeIdx + 1;
          continue;
        }
      }
      textBuf += text[i];
      i++;
      continue;
    }

    if (text.startsWith('**', i)) {
      const closeIdx = text.indexOf('**', i + 2);
      if (closeIdx !== -1) {
        const candidate = text.slice(i + 2, closeIdx);
        if (candidate.length > 0 && candidate.length <= 300) {
          flushText();
          nodes.push(
            <strong key={`bold-${i}`} className="font-semibold text-[#171717]">
              {candidate}
            </strong>
          );
          i = closeIdx + 2;
          continue;
        }
      }
    }

    textBuf += text[i];
    i++;
  }

  flushText();
  return nodes;
}

function renderMarkdownBlocks(content: string) {
  const cleanContent = (content ?? '').replace(/\r\n/g, '\n').replace(/\r/g, '\n');
  const lines = cleanContent.split('\n');
  const elements: React.ReactNode[] = [];
  let inCodeBlock = false;
  let codeBuffer: string[] = [];
  let codeLang = '';

  const bulletRegex = /^(\s*)([-*+•]|\u2022)\s+(.*)$/;
  const numberedRegex = /^(\s*)(\d+(\.\d+)*[\.\)])\s+(.*)$/;

  for (let idx = 0; idx < lines.length; idx++) {
    const line = lines[idx];
    const trimmedLine = line.trim();

    if (trimmedLine.startsWith('```')) {
      if (inCodeBlock) {
        const codeText = codeBuffer.join('\n');
        elements.push(
          <div
            key={`code-${idx}`}
            className="my-2 rounded-md border border-[#e4e4e7] bg-[#f8fafc] overflow-hidden"
          >
            {codeLang && (
              <div className="px-3 py-1 bg-[#f1f5f9] border-b border-[#e4e4e7] text-xs font-mono text-[#64748b]">
                {codeLang}
              </div>
            )}
            <pre className="p-3 font-mono text-xs text-[#0f172a] overflow-x-auto leading-relaxed">
              <code>{codeText}</code>
            </pre>
          </div>
        );
        codeBuffer = [];
        inCodeBlock = false;
        codeLang = '';
      } else {
        inCodeBlock = true;
        codeLang = trimmedLine.slice(3).trim();
      }
      continue;
    }

    if (inCodeBlock) {
      codeBuffer.push(line);
      continue;
    }

    if (trimmedLine.startsWith('# ')) {
      elements.push(
        <h1 key={idx} className="text-base font-bold text-[#171717] mt-3.5 mb-1.5">
          {renderInlineMarkdown(trimmedLine.slice(2))}
        </h1>
      );
      continue;
    }
    if (trimmedLine.startsWith('## ')) {
      elements.push(
        <h2
          key={idx}
          className="text-sm font-semibold text-[#171717] mt-2.5 mb-1 pb-1 border-b border-[#f4f4f5]"
        >
          {renderInlineMarkdown(trimmedLine.slice(3))}
        </h2>
      );
      continue;
    }
    if (trimmedLine.startsWith('### ')) {
      elements.push(
        <h3 key={idx} className="text-sm font-medium text-[#4d4d4d] mt-2 mb-0.5">
          {renderInlineMarkdown(trimmedLine.slice(4))}
        </h3>
      );
      continue;
    }
    if (numberedRegex.test(line)) {
      const numMatch = line.match(numberedRegex)!;
      const indent = numMatch[1].length;
      const mlClass = indent >= 4 ? 'ml-6' : indent >= 2 ? 'ml-4' : 'ml-2';
      elements.push(
        <div key={idx} className={`flex items-start space-x-2 py-0.5 text-xs text-[#171717] ${mlClass}`}>
          <span className="text-[#8f8f8f] font-mono text-xs shrink-0 font-medium">
            {numMatch[2]}
          </span>
          <span className="flex-1 min-w-0">{renderInlineMarkdown(numMatch[4])}</span>
        </div>
      );
      continue;
    }
    if (bulletRegex.test(line)) {
      const bulletMatch = line.match(bulletRegex)!;
      const indent = bulletMatch[1].length;
      const mlClass = indent >= 4 ? 'ml-6' : indent >= 2 ? 'ml-4' : 'ml-2';
      elements.push(
        <div key={idx} className={`flex items-start space-x-1.5 py-0.5 text-xs text-[#171717] ${mlClass}`}>
          <span className="text-[#8f8f8f] shrink-0 leading-relaxed">•</span>
          <span className="flex-1 min-w-0">{renderInlineMarkdown(bulletMatch[3])}</span>
        </div>
      );
      continue;
    }
    if (line.trim().length === 0) {
      elements.push(<div key={idx} className="h-1.5" />);
      continue;
    }
    elements.push(
      <p key={idx} className="text-sm text-[#171717] py-0.5 leading-relaxed">
        {renderInlineMarkdown(line)}
      </p>
    );
  }

  if (inCodeBlock) {
    elements.push(
      <div
        key="code-tail"
        className="my-2 rounded-md border border-[#e4e4e7] bg-[#f8fafc] overflow-hidden"
      >
        {codeLang && (
          <div className="px-3 py-1 bg-[#f1f5f9] border-b border-[#e4e4e7] text-xs font-mono text-[#64748b]">
            {codeLang}
          </div>
        )}
        <pre className="p-3 font-mono text-xs text-[#0f172a] overflow-x-auto leading-relaxed">
          <code>{codeBuffer.join('\n')}</code>
        </pre>
      </div>
    );
  }

  return elements;
}

export default function RuleEditorModal() {
  const { activeFile, isEditorOpen, isSaving, saveFeedback, updateActiveFileContent, saveRules, closeEditor } =
    useCortexStore();

  const [previewMode, setPreviewMode] = useState(true);

  // По умолчанию открываем файл в режиме предпросмотра
  useEffect(() => {
    if (activeFile) {
      setPreviewMode(true);
    }
  }, [activeFile?.id]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === 's') {
        e.preventDefault();
        if (activeFile) {
          saveRules();
        }
      }
      if (e.key === 'Escape') {
        closeEditor();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [activeFile, saveRules, closeEditor]);

  if (!isEditorOpen || !activeFile) return null;

  const lines = activeFile.content.split('\n');
  const lineCount = lines.length;
  const wordCount = activeFile.content.trim().split(/\s+/).filter(Boolean).length;
  const isHook = activeFile.type === 'hook';

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-xs p-3">
      <div className="relative flex flex-col h-[80vh] w-[840px] max-w-full rounded-lg border border-[#ebebeb] bg-white shadow-xl overflow-hidden">
        {/* Editor Top Bar */}
        <div className="flex items-center justify-between border-b border-[#ebebeb] bg-white px-4 py-2.5">
          <div className="flex items-center space-x-2 min-w-0">
            {isHook ? (
              <Zap className="h-4 w-4 text-[#666666] shrink-0" />
            ) : (
              <FileText className="h-4 w-4 text-[#666666] shrink-0" />
            )}
            <span className="font-mono text-xs font-semibold text-[#171717] truncate">
              {activeFile.name}
            </span>
            {isHook && (
              <span className="px-1.5 py-0.5 rounded border border-[#ebebeb] bg-[#f4f4f5] text-[10px] font-mono text-[#666666] shrink-0">
                хук
              </span>
            )}
            {activeFile.isDirty && (
              <span className="h-1.5 w-1.5 rounded-full bg-amber-500" title="Несохраненные изменения" />
            )}
            <span className="text-[11px] text-[#8f8f8f] font-mono hidden sm:inline truncate max-w-xs">
              {activeFile.path}
            </span>
          </div>

          <div className="flex items-center space-x-2">
            {saveFeedback && (
              <div className="flex items-center space-x-1 text-[#15803d] text-xs font-mono">
                <Check className="h-3.5 w-3.5" />
                <span>{saveFeedback}</span>
              </div>
            )}

            <button
              onClick={() => setPreviewMode(!previewMode)}
              className="flex items-center space-x-1 px-2.5 py-1 rounded-md text-xs font-mono text-[#171717] hover:bg-[#f4f4f5] bg-white border border-[#ebebeb] transition cursor-pointer"
              title="Переключить режим предпросмотра"
            >
              {previewMode ? <Edit3 className="h-3.5 w-3.5 text-[#666666]" /> : <Eye className="h-3.5 w-3.5 text-[#666666]" />}
              <span>{previewMode ? 'Правка' : 'Просмотр'}</span>
            </button>

            <button
              onClick={() => saveRules()}
              disabled={isSaving}
              className="flex items-center space-x-1 px-3 py-1 rounded-md text-xs font-mono bg-[#171717] hover:bg-[#262626] text-white font-medium transition cursor-pointer shadow-xs disabled:opacity-50"
              title="Сохранить (Ctrl+S)"
            >
              <Save className="h-3.5 w-3.5" />
              <span>{isSaving ? 'Сохранение...' : 'Сохранить'}</span>
            </button>

            <button
              onClick={closeEditor}
              className="p-1 rounded-md text-[#666666] hover:text-[#171717] hover:bg-[#f4f4f5] transition cursor-pointer"
              title="Закрыть (Esc)"
            >
              <X className="h-4 w-4" />
            </button>
          </div>
        </div>

        {/* Editor Body */}
        <div className="flex-1 overflow-hidden flex bg-white font-mono text-xs text-[#171717]">
          {!previewMode ? (
            <div className="flex w-full h-full">
              {/* Line Numbers */}
              <div className="w-10 py-2.5 bg-[#f9f9f9] border-r border-[#ebebeb] text-right pr-2 select-none text-[#8f8f8f] font-mono text-xs leading-5 tabular-nums">
                {lines.map((_, i) => (
                  <div key={i}>{i + 1}</div>
                ))}
              </div>

              {/* Textarea */}
              <textarea
                value={activeFile.content}
                onChange={(e) => updateActiveFileContent(e.target.value)}
                spellCheck={false}
                className="flex-1 w-full h-full p-2.5 bg-white text-[#171717] font-mono text-xs leading-5 resize-none focus:outline-hidden selection:bg-[#e5e5e5]"
                placeholder="Код хука, инструкции или правила роли..."
              />
            </div>
          ) : (
            <div className="w-full h-full overflow-y-auto bg-white text-[#171717] font-sans leading-relaxed">
              <div className="w-full max-w-4xl px-5 py-4 space-y-2 text-sm">
                {renderMarkdownBlocks(activeFile.content)}
              </div>
            </div>
          )}
        </div>

        {/* Editor Footer */}
        <div className="flex items-center justify-between border-t border-[#ebebeb] bg-[#f9f9f9] px-4 py-1.5 text-[10px] text-[#666666] font-mono">
          <div className="flex items-center space-x-3">
            <span>Этап: <strong className="text-[#171717] uppercase">{activeFile.stageId}</strong></span>
            <span>Строк: <strong className="text-[#171717] tabular-nums">{lineCount}</strong></span>
            <span>Слов: <strong className="text-[#171717] tabular-nums">{wordCount}</strong></span>
          </div>
          <span>UTF-8</span>
        </div>
      </div>
    </div>
  );
}
