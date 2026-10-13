import { useState, useRef, useEffect, useId } from 'react';
import { useDrawerStore } from '../../stores/drawerStore';
import { useBoardStore } from '../../stores/boardStore';
import ToolExecutionCard from './ToolExecutionCard';
import ContextGauge from './ContextGauge';
import {
  ArrowLeft,
  X,
  GitBranch,
  FolderGit2,
  Terminal,
  Brain,
  MessageSquare,
  Send,
} from 'lucide-react';

export default function AgentConsoleDrawer() {
  const {
    isOpen,
    activeTaskId,
    activeTab,
    streamEntries,
    toolExecutions,
    closeDrawer,
    setActiveTab,
    appendThought,
    appendOutput,
  } = useDrawerStore();

  const { tasks, columns } = useBoardStore();
  const [interactiveInput, setInteractiveInput] = useState('');
  const scrollRef = useRef<HTMLDivElement>(null);
  const promptInputId = useId();

  const task = tasks.find((t) => t.id === activeTaskId);
  const column = columns.find((c) => c.id === task?.columnId);

  // Esc key listener to close fullscreen view
  useEffect(() => {
    if (!isOpen) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        closeDrawer();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, closeDrawer]);

  // Auto-scroll on new entries
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [streamEntries, toolExecutions, activeTab]);

  if (!isOpen || !task) return null;

  const currentStream = streamEntries[task.id] || { thoughts: [], outputs: [] };
  const currentTools = toolExecutions[task.id] || [];

  const handleSendPrompt = (e: React.FormEvent) => {
    e.preventDefault();
    if (!interactiveInput.trim()) return;

    if (activeTab === 'thoughts') {
      appendThought(task.id, interactiveInput.trim());
    } else {
      appendOutput(task.id, interactiveInput.trim(), 'output');
    }
    setInteractiveInput('');
  };

  return (
    <div className="fixed inset-0 z-50 flex flex-col bg-obsidian-950 text-slate-100 select-none animate-in fade-in duration-150">
      {/* Fullscreen Header: Clean & De-slopped */}
      <header className="border-b border-obsidian-800 bg-obsidian-900 px-4 py-2.5">
        <div className="flex items-center justify-between">
          {/* Left: "Назад к доске", Task ID & Title, Status badge */}
          <div className="flex items-center space-x-3 min-w-0">
            <button
              onClick={closeDrawer}
              className="flex items-center space-x-2 px-3.5 py-1.5 rounded-md bg-obsidian-850 hover:bg-obsidian-800 text-slate-100 hover:text-white border border-obsidian-750 hover:border-slate-600 transition font-sans text-xs font-semibold cursor-pointer shadow-xs active:scale-[0.98]"
              title="Назад к доске (Esc)"
            >
              <ArrowLeft className="h-4 w-4 text-cyan-400" />
              <span>Назад к доске</span>
            </button>

            <span className="text-obsidian-750 font-mono text-sm">/</span>

            <span className="px-2 py-0.5 rounded font-mono text-xs font-bold bg-cyan-950/70 border border-cyan-800/50 text-cyan-400 tabular-nums">
              {task.id}
            </span>

            <span
              className="px-2 py-0.5 rounded font-mono text-xs uppercase"
              style={{
                backgroundColor: `${column?.color || '#64748b'}20`,
                color: column?.color || '#64748b',
                border: `1px solid ${column?.color || '#64748b'}40`,
              }}
            >
              {column?.title || task.columnId}
            </span>

            <h2 className="text-sm font-semibold text-white truncate max-w-md font-sans">
              {task.title}
            </h2>
          </div>

          {/* Right: Only Close button */}
          <div className="flex items-center space-x-2">
            <button
              onClick={closeDrawer}
              className="p-1.5 rounded text-slate-400 hover:text-white hover:bg-obsidian-800 transition cursor-pointer"
              title="Закрыть (Esc)"
            >
              <X className="h-5 w-5" />
            </button>
          </div>
        </div>

        {/* Sub-header: Path, Branch, and Context gauge */}
        <div className="mt-2 flex items-center justify-between text-xs font-mono text-slate-400 pt-2 border-t border-obsidian-850">
          <div className="flex items-center space-x-3">
            {task.worktreePath && (
              <div className="flex items-center space-x-1 px-2 py-0.5 rounded bg-obsidian-950 border border-obsidian-800">
                <FolderGit2 className="h-3.5 w-3.5 text-slate-500" />
                <span className="text-slate-300">{task.worktreePath}</span>
              </div>
            )}
            {task.branch && (
              <div className="flex items-center space-x-1 px-2 py-0.5 rounded bg-obsidian-950 border border-obsidian-800">
                <GitBranch className="h-3.5 w-3.5 text-slate-500" />
                <span className="text-slate-300">{task.branch}</span>
              </div>
            )}
          </div>

          <div className="w-72">
            <ContextGauge taskId={task.id} />
          </div>
        </div>
      </header>

      {/* Stream Tabs Bar */}
      <div className="flex border-b border-obsidian-800 bg-obsidian-900 px-4 text-xs font-mono select-none">
        <button
          onClick={() => setActiveTab('thoughts')}
          className={`flex items-center space-x-2 py-2.5 px-3 border-b-2 transition cursor-pointer ${
            activeTab === 'thoughts'
              ? 'border-cyan-400 text-cyan-400 font-semibold'
              : 'border-transparent text-slate-400 hover:text-slate-200'
          }`}
        >
          <Brain className="h-4 w-4" />
          <span>&lt;thinking&gt;</span>
          <span className="text-[10px] tabular-nums px-1.5 py-0.2 rounded bg-obsidian-800 text-slate-400 border border-obsidian-750">
            {currentStream.thoughts.length}
          </span>
        </button>

        <button
          onClick={() => setActiveTab('output')}
          className={`flex items-center space-x-2 py-2.5 px-3 border-b-2 transition cursor-pointer ${
            activeTab === 'output'
              ? 'border-cyan-400 text-cyan-400 font-semibold'
              : 'border-transparent text-slate-400 hover:text-slate-200'
          }`}
        >
          <MessageSquare className="h-4 w-4" />
          <span>Вывод / Output</span>
          <span className="text-[10px] tabular-nums px-1.5 py-0.2 rounded bg-obsidian-800 text-slate-400 border border-obsidian-750">
            {currentStream.outputs.length}
          </span>
        </button>

        <button
          onClick={() => setActiveTab('tools')}
          className={`flex items-center space-x-2 py-2.5 px-3 border-b-2 transition cursor-pointer ${
            activeTab === 'tools'
              ? 'border-cyan-400 text-cyan-400 font-semibold'
              : 'border-transparent text-slate-400 hover:text-slate-200'
          }`}
        >
          <Terminal className="h-4 w-4" />
          <span>Инструменты</span>
          <span className="text-[10px] tabular-nums px-1.5 py-0.2 rounded bg-obsidian-800 text-slate-400 border border-obsidian-750">
            {currentTools.length}
          </span>
        </button>
      </div>

      {/* Main Fullscreen Stream Area */}
      <div ref={scrollRef} className="flex-1 overflow-y-auto p-4 md:p-6 bg-obsidian-950 font-mono text-xs">
        <div className="max-w-5xl mx-auto space-y-3">
          {/* Tab 1: Clean Thoughts Section */}
          {activeTab === 'thoughts' && (
            <div className="space-y-3">
              {currentStream.thoughts.map((entry) => (
                <div
                  key={entry.id}
                  className="rounded border border-obsidian-800 bg-obsidian-900/60 border-l-2 border-l-amber-400/80 p-3 text-slate-300 leading-relaxed shadow-xs"
                >
                  <div className="flex items-center justify-between pb-1.5 mb-2 border-b border-obsidian-800/80 text-[10px] font-mono">
                    <span className="font-medium text-amber-400/90">
                      &lt;thinking&gt;
                    </span>
                    <span className="tabular-nums text-slate-500 font-mono">
                      {entry.timestamp}
                    </span>
                  </div>
                  <p className="whitespace-pre-wrap select-text text-slate-300 text-xs leading-relaxed font-mono">
                    {entry.content}
                  </p>
                </div>
              ))}

              {currentStream.thoughts.length === 0 && (
                <div className="h-48 flex flex-col items-center justify-center text-slate-600 font-mono text-xs">
                  <span>Ожидание мыслей агента...</span>
                </div>
              )}
            </div>
          )}

          {/* Tab 2: Output */}
          {activeTab === 'output' && (
            <div className="space-y-3">
              {currentStream.outputs.map((entry) => (
                <div
                  key={entry.id}
                  className={`rounded border p-3 leading-relaxed shadow-xs ${
                    entry.type === 'error'
                      ? 'border-rose-900/50 bg-rose-950/20 text-rose-200'
                      : entry.type === 'system'
                      ? 'border-cyan-900/40 bg-cyan-950/20 text-cyan-200'
                      : 'border-obsidian-800 bg-obsidian-900 text-slate-200'
                  }`}
                >
                  <div className="flex items-center justify-between pb-1.5 mb-2 border-b border-obsidian-800 font-mono text-[10px] text-slate-400">
                    <span className="font-semibold uppercase tracking-wider">{entry.type}</span>
                    <span className="tabular-nums">{entry.timestamp}</span>
                  </div>
                  <p className="whitespace-pre-wrap select-text text-xs leading-relaxed">
                    {entry.content}
                  </p>
                </div>
              ))}

              {currentStream.outputs.length === 0 && (
                <div className="h-48 flex flex-col items-center justify-center font-mono text-slate-600 text-xs">
                  <span>Нет записей вывода</span>
                </div>
              )}
            </div>
          )}

          {/* Tab 3: Tools */}
          {activeTab === 'tools' && (
            <div className="space-y-3">
              {currentTools.map((tool) => (
                <ToolExecutionCard key={tool.id} tool={tool} />
              ))}

              {currentTools.length === 0 && (
                <div className="h-48 flex flex-col items-center justify-center text-slate-600 font-mono text-xs">
                  <span>Инструменты еще не вызывались</span>
                </div>
              )}
            </div>
          )}
        </div>
      </div>

      {/* Clean Interactive Input Bar */}
      <footer className="border-t border-obsidian-800 bg-obsidian-900 p-3">
        <div className="max-w-5xl mx-auto">
          <form onSubmit={handleSendPrompt} className="flex items-center space-x-2">
            <input
              id={promptInputId}
              name="interactivePromptInput"
              type="text"
              value={interactiveInput}
              onChange={(e) => setInteractiveInput(e.target.value)}
              placeholder={
                activeTab === 'thoughts'
                  ? 'Ввести мысль в сознание агента...'
                  : 'Отправить указание агенту...'
              }
              className="flex-1 rounded bg-obsidian-950 border border-obsidian-750 px-3 py-2 text-xs text-slate-100 placeholder-slate-500 focus:outline-hidden focus:border-cyan-500 font-mono"
            />
            <button
              type="submit"
              className="px-4 py-2 rounded bg-cyan-400 hover:bg-cyan-300 text-obsidian-950 font-semibold text-xs transition cursor-pointer flex items-center space-x-1.5"
              title="Отправить"
            >
              <span>Отправить</span>
              <Send className="h-3.5 w-3.5" />
            </button>
          </form>
        </div>
      </footer>
    </div>
  );
}
