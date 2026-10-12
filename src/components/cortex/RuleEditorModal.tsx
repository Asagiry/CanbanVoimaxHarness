import { useState, useEffect } from 'react';
import { useCortexStore } from '../../stores/cortexStore';
import {
  X,
  Save,
  Check,
  FileCode,
  FileText,
  Settings,
  Eye,
  Edit3,
  Sparkles,
  ShieldAlert,
} from 'lucide-react';

export default function RuleEditorModal() {
  const { activeFile, isEditorOpen, isSaving, saveFeedback, updateActiveFileContent, saveRules, closeEditor } =
    useCortexStore();

  const [previewMode, setPreviewMode] = useState(false);

  // Keyboard shortcut Ctrl+S or Cmd+S to save
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

  const getFileIcon = () => {
    if (activeFile.name.endsWith('.json')) {
      return <Settings className="h-3.5 w-3.5 text-cyber-amber shrink-0" />;
    }
    if (activeFile.type === 'persona') {
      return <Sparkles className="h-3.5 w-3.5 text-cyber-purple shrink-0" />;
    }
    return <FileText className="h-3.5 w-3.5 text-cyber-cyan shrink-0" />;
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 p-3 animate-in fade-in duration-100">
      <div className="relative flex flex-col h-[82vh] w-[880px] max-w-full rounded-lg border border-obsidian-750 bg-obsidian-900 shadow-2xl overflow-hidden">
        {/* Editor Top Bar - Developer Ergonomics */}
        <div className="flex items-center justify-between border-b border-obsidian-800 bg-obsidian-850 px-3.5 py-2">
          <div className="flex items-center space-x-2.5 min-w-0">
            <div className="flex items-center space-x-1.5 px-2 py-0.5 rounded-sm bg-obsidian-900 border border-obsidian-750">
              {getFileIcon()}
              <span className="font-mono text-xs font-semibold text-slate-200 truncate">
                {activeFile.name}
              </span>
              {activeFile.isDirty && (
                <span className="h-1.5 w-1.5 rounded-full bg-cyber-amber animate-pulse" title="Unsaved changes" />
              )}
            </div>
            <span className="text-[11px] text-slate-500 font-mono hidden sm:inline truncate max-w-xs">
              {activeFile.path}
            </span>
          </div>

          <div className="flex items-center space-x-2">
            {/* Save Status / Feedback */}
            {saveFeedback && (
              <div className="flex items-center space-x-1 px-2 py-0.5 rounded-sm bg-emerald-950/30 border border-emerald-500/40 text-cyber-emerald text-[11px] font-mono font-medium animate-in fade-in">
                <Check className="h-3 w-3" />
                <span>{saveFeedback}</span>
              </div>
            )}

            {/* Toggle Preview Button */}
            <button
              onClick={() => setPreviewMode(!previewMode)}
              className="flex items-center space-x-1 px-2 py-1 rounded-sm text-xs font-mono text-slate-300 hover:text-white bg-obsidian-800 hover:bg-obsidian-750 border border-obsidian-750 transition cursor-pointer"
              title="Toggle Markdown / Code Preview"
            >
              {previewMode ? <Edit3 className="h-3.5 w-3.5 text-cyber-cyan" /> : <Eye className="h-3.5 w-3.5 text-slate-400" />}
              <span>{previewMode ? 'Edit' : 'Preview'}</span>
            </button>

            {/* Quick Save Rules Button */}
            <button
              onClick={() => saveRules()}
              disabled={isSaving}
              className={`flex items-center space-x-1.5 px-2.5 py-1 rounded-sm text-xs font-semibold font-mono transition active:scale-[0.98] cursor-pointer ${
                activeFile.isDirty
                  ? 'bg-cyber-emerald hover:bg-emerald-400 text-obsidian-950 shadow-xs'
                  : 'bg-obsidian-800 hover:bg-obsidian-750 text-slate-200 border border-obsidian-750'
              }`}
              title="Save Rules (Ctrl+S)"
            >
              <Save className="h-3.5 w-3.5" />
              <span>{isSaving ? 'Saving...' : 'Save Rules'}</span>
            </button>

            {/* Close Button */}
            <button
              onClick={closeEditor}
              className="p-1 rounded text-slate-400 hover:text-white hover:bg-obsidian-800 transition cursor-pointer"
              title="Close (Esc)"
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
              <div className="w-11 py-2.5 bg-obsidian-900/70 border-r border-obsidian-800 text-right pr-2.5 select-none text-slate-600 font-mono text-xs leading-5 tabular-nums">
                {lines.map((_, i) => (
                  <div key={i}>{i + 1}</div>
                ))}
              </div>

              {/* Textarea */}
              <textarea
                value={activeFile.content}
                onChange={(e) => updateActiveFileContent(e.target.value)}
                spellCheck={false}
                className="flex-1 w-full h-full p-2.5 bg-transparent text-slate-200 font-mono text-xs leading-5 resize-none focus:outline-hidden selection:bg-cyan-500/25 selection:text-cyan-200"
                placeholder="Agent persona or rules content..."
              />
            </div>
          ) : (
            /* Styled Preview Pane */
            <div className="w-full h-full p-5 overflow-y-auto bg-obsidian-950 text-slate-200 font-sans leading-relaxed">
              <div className="max-w-2xl mx-auto space-y-3">
                <div className="pb-2.5 border-b border-obsidian-800 flex items-center justify-between">
                  <h3 className="text-sm font-semibold text-white flex items-center space-x-2">
                    <FileCode className="h-3.5 w-3.5 text-cyber-cyan" />
                    <span>Parsed Agent Mind Document</span>
                  </h3>
                  <span className="text-[11px] font-mono text-slate-400">{activeFile.name}</span>
                </div>

                <div className="space-y-2 text-xs">
                  {lines.map((line, idx) => {
                    if (line.startsWith('# ')) {
                      return <h1 key={idx} className="text-base font-bold text-cyber-cyan mt-3 mb-1.5">{line.replace('# ', '')}</h1>;
                    }
                    if (line.startsWith('## ')) {
                      return <h2 key={idx} className="text-sm font-semibold text-slate-100 mt-2.5 mb-1">{line.replace('## ', '')}</h2>;
                    }
                    if (line.startsWith('### ')) {
                      return <h3 key={idx} className="text-xs font-semibold text-slate-200 mt-2 mb-0.5">{line.replace('### ', '')}</h3>;
                    }
                    if (line.startsWith('- ')) {
                      return (
                        <div key={idx} className="flex items-start space-x-1.5 py-0.5 text-slate-300">
                          <span className="text-cyber-cyan font-bold">•</span>
                          <span>{line.replace('- ', '')}</span>
                        </div>
                      );
                    }
                    if (line.trim().length === 0) {
                      return <div key={idx} className="h-1.5" />;
                    }
                    return <p key={idx} className="text-xs text-slate-300 py-0.5 font-sans leading-relaxed">{line}</p>;
                  })}
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Editor Footer Telemetry */}
        <div className="flex items-center justify-between border-t border-obsidian-800 bg-obsidian-900 px-3.5 py-1 text-[10px] text-slate-400 font-mono">
          <div className="flex items-center space-x-3">
            <span className="flex items-center space-x-1">
              <span className="text-slate-500">Stage:</span>
              <span className="text-slate-200 uppercase font-semibold">{activeFile.stageId}</span>
            </span>
            <span>Lines: <strong className="text-slate-200 tabular-nums">{lineCount}</strong></span>
            <span>Words: <strong className="text-slate-200 tabular-nums">{wordCount}</strong></span>
          </div>

          <div className="flex items-center space-x-2.5">
            <span className="flex items-center space-x-1 text-slate-400">
              <ShieldAlert className="h-3 w-3 text-cyber-amber" />
              <span>Zero-Turn Primed Rule</span>
            </span>
            <span className="text-slate-500">UTF-8</span>
          </div>
        </div>
      </div>
    </div>
  );
}
