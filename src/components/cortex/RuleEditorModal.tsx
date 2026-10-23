import React, { useState, useEffect, useMemo } from 'react';
import { useCortexStore } from '../../stores/cortexStore';
import { useTranslation } from '../../stores/languageStore';
import AgentSettingVisualEditor from './AgentSettingVisualEditor';
import {
  X,
  Save,
  Check,
  FileText,
  Eye,
  Edit3,
  Zap,
  SlidersHorizontal,
  Code2,
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
              className="px-1.5 py-0.5 bg-[#f4f4f5] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] rounded text-xs font-mono text-[#09090b] dark:text-[#f0f6fc]"
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
            <strong key={`bold-${i}`} className="font-semibold text-[#171717] dark:text-[#f0f6fc]">
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
            className="my-2 rounded-md border border-[#e4e4e7] dark:border-[#21262d] bg-[#f8fafc] dark:bg-[#161b22] overflow-hidden"
          >
            {codeLang && (
              <div className="px-3 py-1 bg-[#f1f5f9] dark:bg-[#21262d] border-b border-[#e4e4e7] dark:border-[#30363d] text-xs font-mono text-[#64748b] dark:text-[#8b949e]">
                {codeLang}
              </div>
            )}
            <pre className="p-3 font-mono text-xs text-[#0f172a] dark:text-[#f0f6fc] overflow-x-auto leading-relaxed">
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
        <h1 key={idx} className="text-base font-bold text-[#171717] dark:text-[#f0f6fc] mt-3.5 mb-1.5">
          {renderInlineMarkdown(trimmedLine.slice(2))}
        </h1>
      );
      continue;
    }
    if (trimmedLine.startsWith('## ')) {
      elements.push(
        <h2
          key={idx}
          className="text-sm font-semibold text-[#171717] dark:text-[#f0f6fc] mt-2.5 mb-1 pb-1 border-b border-[#f4f4f5] dark:border-[#21262d]"
        >
          {renderInlineMarkdown(trimmedLine.slice(3))}
        </h2>
      );
      continue;
    }
    if (trimmedLine.startsWith('### ')) {
      elements.push(
        <h3 key={idx} className="text-sm font-medium text-[#4d4d4d] dark:text-[#c9d1d9] mt-2 mb-0.5">
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
        <div key={idx} className={`flex items-start space-x-2 py-0.5 text-xs text-[#171717] dark:text-[#f0f6fc] ${mlClass}`}>
          <span className="text-[#8f8f8f] dark:text-[#8b949e] font-mono text-xs shrink-0 font-medium">
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
        <div key={idx} className={`flex items-start space-x-1.5 py-0.5 text-xs text-[#171717] dark:text-[#f0f6fc] ${mlClass}`}>
          <span className="text-[#8f8f8f] dark:text-[#8b949e] shrink-0 leading-relaxed">•</span>
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
      <p key={idx} className="text-sm text-[#171717] dark:text-[#f0f6fc] py-0.5 leading-relaxed">
        {renderInlineMarkdown(line)}
      </p>
    );
  }

  if (inCodeBlock) {
    elements.push(
      <div
        key="code-tail"
        className="my-2 rounded-md border border-[#e4e4e7] dark:border-[#21262d] bg-[#f8fafc] dark:bg-[#161b22] overflow-hidden"
      >
        {codeLang && (
          <div className="px-3 py-1 bg-[#f1f5f9] dark:bg-[#21262d] border-b border-[#e4e4e7] dark:border-[#30363d] text-xs font-mono text-[#64748b] dark:text-[#8b949e]">
            {codeLang}
          </div>
        )}
        <pre className="p-3 font-mono text-xs text-[#0f172a] dark:text-[#f0f6fc] overflow-x-auto leading-relaxed">
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
  const { t } = useTranslation();

  const [previewMode, setPreviewMode] = useState(true);
  const [settingsTab, setSettingsTab] = useState<'visual' | 'code'>('visual');

  const isSettings = activeFile?.type === 'settings';

  const activeRole = useMemo(() => {
    if (!activeFile) return undefined;
    if (activeFile.stageId?.startsWith('agent:')) {
      return activeFile.stageId.slice(6);
    }
    const match = activeFile.path?.match(/\.cvh\/agents\/([^/\\]+)/);
    if (match) return match[1];
    return activeFile.stageId;
  }, [activeFile]);

  // По умолчанию открываем файл: если settings -> visual, иначе previewMode -> true
  useEffect(() => {
    if (activeFile) {
      if (activeFile.type === 'settings') {
        setSettingsTab('visual');
      } else {
        setPreviewMode(true);
      }
    }
  }, [activeFile?.id, activeFile?.type]);

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
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 dark:bg-black/80 p-3">
      <div className="relative flex flex-col h-[82vh] w-[860px] max-w-full rounded-lg border border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#0d1117] shadow-md overflow-hidden">
        {/* Editor Top Bar */}
        <div className="flex items-center justify-between border-b border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#161b22] px-4 py-2.5">
          <div className="flex items-center space-x-2 min-w-0">
            {isSettings ? (
              <SlidersHorizontal className="h-4 w-4 text-[#6366f1] dark:text-[#818cf8] shrink-0" />
            ) : isHook ? (
              <Zap className="h-4 w-4 text-[#666666] dark:text-[#8b949e] shrink-0" />
            ) : (
              <FileText className="h-4 w-4 text-[#666666] dark:text-[#8b949e] shrink-0" />
            )}
            <span className="font-mono text-xs font-semibold text-[#171717] dark:text-[#f0f6fc] truncate">
              {activeFile.name}
            </span>
            {isHook && (
              <span className="px-1.5 py-0.5 rounded border border-[#ebebeb] dark:border-[#30363d] bg-[#f4f4f5] dark:bg-[#21262d] text-[10px] font-mono text-[#666666] dark:text-[#8b949e] shrink-0">
                {t('ruleEditor.hookBadge')}
              </span>
            )}
          </div>

          <div className="flex items-center space-x-2">
            {saveFeedback && (
              <div className="flex items-center space-x-1 text-[#15803d] dark:text-[#4ade80] text-xs font-mono">
                <Check className="h-3.5 w-3.5" />
                <span>{saveFeedback}</span>
              </div>
            )}

            {isSettings ? (
              <div className="flex items-center rounded-md border border-[#ebebeb] dark:border-[#30363d] bg-[#f4f4f5] dark:bg-[#21262d] p-0.5 text-xs font-mono">
                <button
                  type="button"
                  onClick={() => setSettingsTab('visual')}
                  className={`flex items-center space-x-1.5 px-2.5 py-1 rounded transition cursor-pointer ${
                    settingsTab === 'visual'
                      ? 'bg-white dark:bg-[#161b22] text-[#171717] dark:text-[#f0f6fc] shadow-2xs font-semibold'
                      : 'text-[#666666] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc]'
                  }`}
                >
                  <SlidersHorizontal className="h-3.5 w-3.5 text-[#6366f1] dark:text-[#818cf8]" />
                  <span>{t('ruleEditor.settingsTabVisual')}</span>
                </button>
                <button
                  type="button"
                  onClick={() => setSettingsTab('code')}
                  className={`flex items-center space-x-1.5 px-2.5 py-1 rounded transition cursor-pointer ${
                    settingsTab === 'code'
                      ? 'bg-white dark:bg-[#161b22] text-[#171717] dark:text-[#f0f6fc] shadow-2xs font-semibold'
                      : 'text-[#666666] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc]'
                  }`}
                >
                  <Code2 className="h-3.5 w-3.5 text-[#666666] dark:text-[#8b949e]" />
                  <span>{t('ruleEditor.settingsTabCode')}</span>
                </button>
              </div>
            ) : (
              <button
                onClick={() => setPreviewMode(!previewMode)}
                className="flex items-center space-x-1 px-2.5 py-1 rounded-md text-xs font-mono text-[#171717] dark:text-[#f0f6fc] hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] bg-white dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] transition cursor-pointer"
                title={t('ruleEditor.togglePreview')}
              >
                {previewMode ? <Edit3 className="h-3.5 w-3.5 text-[#666666] dark:text-[#8b949e]" /> : <Eye className="h-3.5 w-3.5 text-[#666666] dark:text-[#8b949e]" />}
                <span>{previewMode ? t('ruleEditor.edit') : t('ruleEditor.preview')}</span>
              </button>
            )}

            <button
              onClick={() => saveRules()}
              disabled={isSaving}
              className="flex items-center space-x-1 px-3 py-1 rounded-md text-xs font-mono bg-[#171717] dark:bg-[#238636] hover:bg-[#262626] dark:hover:bg-[#2ea043] text-white font-medium transition cursor-pointer shadow-xs disabled:opacity-50"
              title={t('ruleEditor.saveHint')}
            >
              <Save className="h-3.5 w-3.5" />
              <span>{isSaving ? t('common.saving') : t('common.save')}</span>
            </button>

            <button
              onClick={closeEditor}
              className="p-1 rounded-md text-[#666666] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] transition cursor-pointer"
              aria-label={t('common.close')}
            >
              <X className="h-4 w-4" />
            </button>
          </div>
        </div>

        {/* Editor Body */}
        <div className="flex-1 overflow-hidden flex bg-white dark:bg-[#0d1117] font-mono text-xs text-[#171717] dark:text-[#f0f6fc]">
          {isSettings ? (
            settingsTab === 'visual' ? (
              <AgentSettingVisualEditor
                content={activeFile.content}
                role={activeRole}
                onChange={updateActiveFileContent}
                onSwitchToCode={() => setSettingsTab('code')}
              />
            ) : (
              <div className="flex w-full h-full">
                {/* Line Numbers */}
                <div className="w-10 py-2.5 bg-[#f9f9f9] dark:bg-[#161b22] border-r border-[#ebebeb] dark:border-[#21262d] text-right pr-2 select-none text-[#8f8f8f] dark:text-[#8b949e] font-mono text-xs leading-5 tabular-nums">
                  {lines.map((_, i) => (
                    <div key={i}>{i + 1}</div>
                  ))}
                </div>

                {/* Textarea */}
                <textarea
                  value={activeFile.content}
                  onChange={(e) => updateActiveFileContent(e.target.value)}
                  spellCheck={false}
                  className="flex-1 w-full h-full p-2.5 bg-white dark:bg-[#0d1117] text-[#171717] dark:text-[#f0f6fc] font-mono text-xs leading-5 resize-none focus:outline-hidden selection:bg-[#e5e5e5] dark:selection:bg-[#264f78]"
                  placeholder={t('ruleEditor.placeholder')}
                />
              </div>
            )
          ) : !previewMode ? (
            <div className="flex w-full h-full">
              {/* Line Numbers */}
              <div className="w-10 py-2.5 bg-[#f9f9f9] dark:bg-[#161b22] border-r border-[#ebebeb] dark:border-[#21262d] text-right pr-2 select-none text-[#8f8f8f] dark:text-[#8b949e] font-mono text-xs leading-5 tabular-nums">
                {lines.map((_, i) => (
                  <div key={i}>{i + 1}</div>
                ))}
              </div>

              {/* Textarea */}
              <textarea
                value={activeFile.content}
                onChange={(e) => updateActiveFileContent(e.target.value)}
                spellCheck={false}
                className="flex-1 w-full h-full p-2.5 bg-white dark:bg-[#0d1117] text-[#171717] dark:text-[#f0f6fc] font-mono text-xs leading-5 resize-none focus:outline-hidden selection:bg-[#e5e5e5] dark:selection:bg-[#264f78]"
                placeholder={t('ruleEditor.placeholder')}
              />
            </div>
          ) : (
            <div className="w-full h-full overflow-y-auto bg-white dark:bg-[#0d1117] text-[#171717] dark:text-[#f0f6fc] font-sans leading-relaxed">
              <div className="w-full max-w-4xl px-5 py-4 space-y-2 text-sm">
                {renderMarkdownBlocks(activeFile.content)}
              </div>
            </div>
          )}
        </div>

        {/* Editor Footer */}
        <div className="flex items-center justify-between border-t border-[#ebebeb] dark:border-[#21262d] bg-[#f9f9f9] dark:bg-[#161b22] px-4 py-1.5 text-[10px] text-[#666666] dark:text-[#8b949e] font-mono">
          <div className="flex items-center space-x-3">
            <span>{t('ruleEditor.stage')} <strong className="text-[#171717] dark:text-[#f0f6fc] uppercase">{activeFile.stageId}</strong></span>
            {isSettings && settingsTab === 'visual' ? (
              <span className="text-[#6366f1] dark:text-[#818cf8] font-medium">UI Editor</span>
            ) : (
              <>
                <span>{t('ruleEditor.lines')} <strong className="text-[#171717] dark:text-[#f0f6fc] tabular-nums">{lineCount}</strong></span>
                <span>{t('ruleEditor.words')} <strong className="text-[#171717] dark:text-[#f0f6fc] tabular-nums">{wordCount}</strong></span>
              </>
            )}
          </div>
          <span>{isSettings ? 'JSON / UTF-8' : 'UTF-8'}</span>
        </div>
      </div>
    </div>
  );
}
