import { useState, useEffect } from 'react';
import { useCortexStore } from '../../stores/cortexStore';
import {
  X,
  Save,
  Check,
  FileText,
  Eye,
  Edit3,
} from 'lucide-react';

export default function RuleEditorModal() {
  const { activeFile, isEditorOpen, isSaving, saveFeedback, updateActiveFileContent, saveRules, closeEditor } =
    useCortexStore();

  const [previewMode, setPreviewMode] = useState(false);

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

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-3">
      <div className="relative flex flex-col h-[80vh] w-[840px] max-w-full rounded border border-obsidian-750 bg-obsidian-900 shadow-xl overflow-hidden">
        {/* Editor Top Bar */}
        <div className="flex items-center justify-between border-b border-obsidian-800 bg-obsidian-850 px-3.5 py-2">
          <div className="flex items-center space-x-2 min-w-0">
            <FileText className="h-3.5 w-3.5 text-slate-400 shrink-0" />
            <span className="font-mono text-xs font-medium text-slate-200 truncate">
              {activeFile.name}
            </span>
            {activeFile.isDirty && (
              <span className="h-1.5 w-1.5 rounded-full bg-amber-400" title="Несохраненные изменения" />
            )}
            <span className="text-[11px] text-slate-500 font-mono hidden sm:inline truncate max-w-xs">
              {activeFile.path}
            </span>
          </div>

          <div className="flex items-center space-x-2">
            {saveFeedback && (
              <div className="flex items-center space-x-1 text-emerald-400 text-xs font-mono">
                <Check className="h-3 w-3" />
                <span>Сохранено</span>
              </div>
            )}

            <button
              onClick={() => setPreviewMode(!previewMode)}
              className="flex items-center space-x-1 px-2 py-1 rounded text-xs font-mono text-slate-300 hover:text-white bg-obsidian-800 hover:bg-obsidian-750 border border-obsidian-750 transition cursor-pointer"
              title="Переключить режим предпросмотра"
            >
              {previewMode ? <Edit3 className="h-3.5 w-3.5 text-slate-400" /> : <Eye className="h-3.5 w-3.5 text-slate-400" />}
              <span>{previewMode ? 'Правка' : 'Просмотр'}</span>
            </button>

            <button
              onClick={() => saveRules()}
              disabled={isSaving}
              className="flex items-center space-x-1 px-2.5 py-1 rounded text-xs font-mono bg-slate-200 hover:bg-white text-obsidian-950 font-medium transition cursor-pointer"
              title="Сохранить (Ctrl+S)"
            >
              <Save className="h-3 w-3" />
              <span>{isSaving ? 'Сохранение...' : 'Сохранить'}</span>
            </button>

            <button
              onClick={closeEditor}
              className="p-1 rounded text-slate-400 hover:text-white transition cursor-pointer"
              title="Закрыть (Esc)"
            >
              <X className="h-3.5 w-3.5" />
            </button>
          </div>
        </div>

        {/* Editor Body */}
        <div className="flex-1 overflow-hidden flex bg-obsidian-950 font-mono text-xs">
          {!previewMode ? (
            <div className="flex w-full h-full">
              {/* Line Numbers */}
              <div className="w-10 py-2.5 bg-obsidian-900/60 border-r border-obsidian-800/80 text-right pr-2 select-none text-slate-600 font-mono text-xs leading-5 tabular-nums">
                {lines.map((_, i) => (
                  <div key={i}>{i + 1}</div>
                ))}
              </div>

              {/* Textarea */}
              <textarea
                value={activeFile.content}
                onChange={(e) => updateActiveFileContent(e.target.value)}
                spellCheck={false}
                className="flex-1 w-full h-full p-2.5 bg-transparent text-slate-200 font-mono text-xs leading-5 resize-none focus:outline-hidden selection:bg-slate-700"
                placeholder="Инструкции или правила роли..."
              />
            </div>
          ) : (
            <div className="w-full h-full p-4 overflow-y-auto bg-obsidian-950 text-slate-200 font-sans leading-relaxed">
              <div className="max-w-2xl mx-auto space-y-2 text-xs">
                {lines.map((line, idx) => {
                  if (line.startsWith('# ')) {
                    return <h1 key={idx} className="text-sm font-bold text-slate-100 mt-2 mb-1">{line.replace('# ', '')}</h1>;
                  }
                  if (line.startsWith('## ')) {
                    return <h2 key={idx} className="text-xs font-semibold text-slate-200 mt-2 mb-1">{line.replace('## ', '')}</h2>;
                  }
                  if (line.startsWith('### ')) {
                    return <h3 key={idx} className="text-xs font-medium text-slate-300 mt-1 mb-0.5">{line.replace('### ', '')}</h3>;
                  }
                  if (line.startsWith('- ')) {
                    return (
                      <div key={idx} className="flex items-start space-x-1.5 py-0.5 text-slate-300">
                        <span className="text-slate-500">•</span>
                        <span>{line.replace('- ', '')}</span>
                      </div>
                    );
                  }
                  if (line.trim().length === 0) {
                    return <div key={idx} className="h-1" />;
                  }
                  return <p key={idx} className="text-xs text-slate-300 py-0.5 leading-relaxed">{line}</p>;
                })}
              </div>
            </div>
          )}
        </div>

        {/* Editor Footer */}
        <div className="flex items-center justify-between border-t border-obsidian-800 bg-obsidian-900 px-3.5 py-1 text-[10px] text-slate-500 font-mono">
          <div className="flex items-center space-x-3">
            <span>Этап: <strong className="text-slate-400 uppercase">{activeFile.stageId}</strong></span>
            <span>Строк: <strong className="text-slate-400 tabular-nums">{lineCount}</strong></span>
            <span>Слов: <strong className="text-slate-400 tabular-nums">{wordCount}</strong></span>
          </div>
          <span>UTF-8</span>
        </div>
      </div>
    </div>
  );
}
