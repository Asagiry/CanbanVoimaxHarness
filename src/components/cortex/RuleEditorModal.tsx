import { useState, useEffect, useMemo } from 'react';
import { useShallow } from 'zustand/react/shallow';
import { useCortexStore } from '../../stores/cortexStore';
import { useTranslation } from '../../stores/languageStore';
import AgentSettingVisualEditor from './AgentSettingVisualEditor';
import { useUiZoom, getZoomOverlayStyle } from '../../hooks/useUiZoom';
import { RuntimeRuleIcon, isRuntimeRule } from '../common/RuntimeRuleIcon';
import HookIcon from '../common/HookIcon';
import { MarkdownContent } from '../drawer/stream';
import { hookMomentOf } from './types';
import {
  X,
  Save,
  Check,
  FileText,
  Eye,
  Edit3,
  SlidersHorizontal,
  Code2,
} from 'lucide-react';

export default function RuleEditorModal() {
  const { zoom } = useUiZoom();
  const {
    activeFile,
    isEditorOpen,
    isSaving,
    saveFeedback,
    updateActiveFileContent,
    saveRules,
    closeEditor,
  } = useCortexStore(
    useShallow((s) => ({
      activeFile: s.activeFile,
      isEditorOpen: s.isEditorOpen,
      isSaving: s.isSaving,
      saveFeedback: s.saveFeedback,
      updateActiveFileContent: s.updateActiveFileContent,
      saveRules: s.saveRules,
      closeEditor: s.closeEditor,
    }))
  );
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
    if (!isEditorOpen) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === 's') {
        e.preventDefault();
        if (activeFile) {
          saveRules();
        }
      }
      if (e.key === 'Escape') {
        e.preventDefault();
        e.stopImmediatePropagation();
        closeEditor();
      }
    };

    window.addEventListener('keydown', handleKeyDown, { capture: true });
    return () => window.removeEventListener('keydown', handleKeyDown, { capture: true });
  }, [isEditorOpen, activeFile, saveRules, closeEditor]);

  if (!isEditorOpen || !activeFile) return null;

  const lines = activeFile.content.split('\n');
  const lineCount = lines.length;
  const wordCount = activeFile.content.trim().split(/\s+/).filter(Boolean).length;
  const isHook = activeFile.type === 'hook';
  const isRuntime = isRuntimeRule(activeFile.name);

  return (
    <div style={getZoomOverlayStyle(zoom)} className="fixed inset-0 z-[85] flex items-center justify-center bg-black/60 dark:bg-black/80 p-3">
      <div className="relative flex flex-col h-[85%] max-h-[calc(100%-2rem)] w-[860px] max-w-[95%] rounded-lg border border-[#ebebeb] dark:border-[#212126] bg-white dark:bg-[#111114] shadow-md overflow-hidden">
        {/* Editor Top Bar */}
        <div className="flex items-center justify-between border-b border-[#ebebeb] dark:border-[#212126] bg-white dark:bg-[#17171b] px-4 py-2.5">
          <div className="flex items-center space-x-2 min-w-0">
            {isSettings ? (
              <SlidersHorizontal className="h-4 w-4 text-sky-600 dark:text-sky-400 shrink-0" />
            ) : isHook ? (
              <HookIcon file={activeFile} className="h-4 w-4" />
            ) : isRuntime ? (
              <RuntimeRuleIcon className="h-4 w-4 text-sky-600 dark:text-sky-400 shrink-0" />
            ) : (
              <FileText className="h-4 w-4 text-[#666666] dark:text-[#71717a] shrink-0" />
            )}
            <span className="font-mono text-xs font-semibold text-[#171717] dark:text-[#f4f4f5] truncate">
              {activeFile.name}
            </span>
            {isHook && (
              <span className="px-1.5 py-0.5 rounded border border-[#ebebeb] dark:border-[#2c2c33] bg-[#f4f4f5] dark:bg-[#232328] text-[10px] font-mono text-[#666666] dark:text-[#71717a] shrink-0">
                {hookMomentOf(activeFile) ?? t('ruleEditor.hookBadge')}
              </span>
            )}
            {isRuntime && (
              <span
                title={t('cortex.runtimeRuleTooltip')}
                className="px-1.5 py-0.5 rounded border border-sky-200 dark:border-sky-800/80 bg-sky-50 dark:bg-sky-950/50 text-[10px] font-mono font-medium text-sky-700 dark:text-sky-300 shrink-0"
              >
                {t('cortex.runtimeRuleBadge')}
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
              <div className="flex items-center rounded-md border border-[#ebebeb] dark:border-[#2c2c33] bg-[#f4f4f5] dark:bg-[#232328] p-0.5 text-xs font-mono">
                <button
                  type="button"
                  onClick={() => setSettingsTab('visual')}
                  className={`flex items-center space-x-1.5 px-2.5 py-1 rounded transition cursor-pointer ${
                    settingsTab === 'visual'
                      ? 'bg-white dark:bg-[#17171b] text-[#171717] dark:text-[#f4f4f5] shadow-2xs font-semibold'
                      : 'text-[#666666] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5]'
                  }`}
                >
                  <SlidersHorizontal className="h-3.5 w-3.5 text-sky-600 dark:text-sky-400" />
                  <span>{t('ruleEditor.settingsTabVisual')}</span>
                </button>
                <button
                  type="button"
                  onClick={() => setSettingsTab('code')}
                  className={`flex items-center space-x-1.5 px-2.5 py-1 rounded transition cursor-pointer ${
                    settingsTab === 'code'
                      ? 'bg-white dark:bg-[#17171b] text-[#171717] dark:text-[#f4f4f5] shadow-2xs font-semibold'
                      : 'text-[#666666] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5]'
                  }`}
                >
                  <Code2 className="h-3.5 w-3.5 text-[#666666] dark:text-[#71717a]" />
                  <span>{t('ruleEditor.settingsTabCode')}</span>
                </button>
              </div>
            ) : (
              <button
                type="button"
                onClick={() => setPreviewMode(!previewMode)}
                className="flex items-center space-x-1 px-2.5 py-1 rounded-md text-xs font-mono text-[#171717] dark:text-[#f4f4f5] hover:bg-[#f4f4f5] dark:hover:bg-[#232328] bg-white dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] transition cursor-pointer"
                aria-label={t('ruleEditor.togglePreview')}
              >
                {previewMode ? <Edit3 className="h-3.5 w-3.5 text-[#666666] dark:text-[#71717a]" /> : <Eye className="h-3.5 w-3.5 text-[#666666] dark:text-[#71717a]" />}
                <span>{previewMode ? t('ruleEditor.edit') : t('ruleEditor.preview')}</span>
              </button>
            )}

            <button
              type="button"
              onClick={() => saveRules()}
              disabled={isSaving}
              className="flex items-center space-x-1 px-3 py-1 rounded-md text-xs font-mono bg-[#171717] dark:bg-[#238636] hover:bg-[#262626] dark:hover:bg-[#2ea043] text-white font-medium transition cursor-pointer shadow-xs disabled:opacity-50"
              aria-label={t('ruleEditor.saveHint')}
              title={t('ruleEditor.saveHint')}
            >
              <Save className="h-3.5 w-3.5" />
              <span>{isSaving ? t('common.saving') : t('common.save')}</span>
            </button>

            <button
              type="button"
              onClick={closeEditor}
              className="p-1 rounded-md text-[#666666] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] hover:bg-[#f4f4f5] dark:hover:bg-[#232328] transition cursor-pointer"
              aria-label={t('common.close')}
            >
              <X className="h-4 w-4" />
            </button>
          </div>
        </div>

        {/* Editor Body */}
        <div className="flex-1 overflow-hidden flex bg-white dark:bg-[#111114] font-mono text-xs text-[#171717] dark:text-[#f4f4f5]">
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
                <div className="w-10 py-2.5 bg-[#f9f9f9] dark:bg-[#17171b] border-r border-[#ebebeb] dark:border-[#212126] text-right pr-2 select-none text-[#8f8f8f] dark:text-[#71717a] font-mono text-xs leading-5 tabular-nums">
                  {lines.map((_, i) => (
                    <div key={`settings-ln-${i + 1}`}>{i + 1}</div>
                  ))}
                </div>

                <textarea
                  value={activeFile.content}
                  onChange={(e) => updateActiveFileContent(e.target.value)}
                  spellCheck={false}
                  className="flex-1 w-full h-full p-2.5 bg-white dark:bg-[#111114] text-[#171717] dark:text-[#f4f4f5] font-mono text-xs leading-5 resize-none focus:outline-hidden selection:bg-[#e5e5e5] dark:selection:bg-[#264f78]"
                  placeholder={t('ruleEditor.placeholder')}
                />
              </div>
            )
          ) : !previewMode ? (
            <div className="flex w-full h-full">
              <div className="w-10 py-2.5 bg-[#f9f9f9] dark:bg-[#17171b] border-r border-[#ebebeb] dark:border-[#212126] text-right pr-2 select-none text-[#8f8f8f] dark:text-[#71717a] font-mono text-xs leading-5 tabular-nums">
                {lines.map((_, i) => (
                  <div key={`md-ln-${i + 1}`}>{i + 1}</div>
                ))}
              </div>

              <textarea
                value={activeFile.content}
                onChange={(e) => updateActiveFileContent(e.target.value)}
                spellCheck={false}
                className="flex-1 w-full h-full p-2.5 bg-white dark:bg-[#111114] text-[#171717] dark:text-[#f4f4f5] font-mono text-xs leading-5 resize-none focus:outline-hidden selection:bg-[#e5e5e5] dark:selection:bg-[#264f78]"
                placeholder={t('ruleEditor.placeholder')}
              />
            </div>
          ) : (
            <div className="w-full h-full overflow-y-auto bg-white dark:bg-[#111114] text-[#171717] dark:text-[#f4f4f5] font-sans leading-relaxed">
              <div className="w-full max-w-4xl px-5 py-4 space-y-2 text-sm">
                <MarkdownContent content={activeFile.content} />
              </div>
            </div>
          )}
        </div>

        {/* Editor Footer */}
        <div className="flex items-center justify-between border-t border-[#ebebeb] dark:border-[#212126] bg-[#f9f9f9] dark:bg-[#17171b] px-4 py-1.5 text-[10px] text-[#666666] dark:text-[#71717a] font-mono">
          <div className="flex items-center space-x-3">
            <span>{t('ruleEditor.stage')} <strong className="text-[#171717] dark:text-[#f4f4f5] uppercase">{activeFile.stageId}</strong></span>
            {isSettings && settingsTab === 'visual' ? (
              <span className="text-sky-600 dark:text-sky-400 font-medium">UI Editor</span>
            ) : (
              <>
                <span>{t('ruleEditor.lines')} <strong className="text-[#171717] dark:text-[#f4f4f5] tabular-nums">{lineCount}</strong></span>
                <span>{t('ruleEditor.words')} <strong className="text-[#171717] dark:text-[#f4f4f5] tabular-nums">{wordCount}</strong></span>
              </>
            )}
          </div>
          <span>{isSettings ? 'JSON / UTF-8' : 'UTF-8'}</span>
        </div>
      </div>
    </div>
  );
}
