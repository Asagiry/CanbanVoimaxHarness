import { useState, useRef, useEffect, useId } from 'react';
import { useDrawerStore } from '../../stores/drawerStore';
import { useBoardStore } from '../../stores/boardStore';
import ToolExecutionCard from './ToolExecutionCard';
import ContextGauge from './ContextGauge';
import {
  ArrowLeft,
  X,
  Play,
  Pause,
  Square,
  ArrowRightCircle,
  RotateCw,
  GitBranch,
  FolderGit2,
  Terminal,
  Brain,
  MessageSquare,
  Cpu,
  Send,
  Zap,
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
    addToolExecution,
  } = useDrawerStore();

  const { tasks, columns, updateTask, promoteTaskToNextStage } = useBoardStore();
  const [interactiveInput, setInteractiveInput] = useState('');
  const [feedbackCue, setFeedbackCue] = useState<string | null>(null);
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

  const showCue = (message: string) => {
    setFeedbackCue(message);
    setTimeout(() => setFeedbackCue(null), 2500);
  };

  const handlePauseResume = () => {
    const isPaused = task.status === 'paused';
    const nextStatus = isPaused ? 'running' : 'paused';
    updateTask(task.id, {
      status: nextStatus,
      isTimerRunning: nextStatus === 'running',
    });
    showCue(isPaused ? 'Агент возобновлен' : 'Агент приостановлен');
    appendOutput(
      task.id,
      `[SYSTEM] Agent state changed: ${nextStatus.toUpperCase()}`,
      'system'
    );
  };

  const handleForceKill = () => {
    updateTask(task.id, {
      status: 'blocked',
      isTimerRunning: false,
    });
    showCue('Процесс принудительно остановлен');
    appendOutput(
      task.id,
      '[CRITICAL] Agent process terminated via Force Kill signal (SIGKILL). Worktree preserved.',
      'error'
    );
  };

  const handlePromote = () => {
    promoteTaskToNextStage(task.id);
    showCue('Переведено на следующий этап');
  };

  const handleRetry = () => {
    updateTask(task.id, {
      status: 'running',
      isTimerRunning: true,
      verdict: 'NONE',
    });
    showCue('Повторный запуск');
    appendThought(task.id, 'Initiating clean retry cycle in isolated worktree.');
    addToolExecution(task.id, {
      id: `tool-retry-${Date.now()}`,
      toolName: 'execute_command',
      timestamp: new Date().toLocaleTimeString('en-US', { hour12: false }),
      durationMs: 420,
      status: 'success',
      exitCode: 0,
      params: { command: 'git status --porcelain', workdir: task.worktreePath },
      stdout: 'Working tree clean. Re-triggering sensors.',
    });
  };

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
      {/* Visual Feedback Cue Toast */}
      {feedbackCue && (
        <div className="absolute top-14 left-1/2 -translate-x-1/2 z-50 flex items-center space-x-1.5 px-3 py-1 rounded-full bg-cyan-400 text-obsidian-950 font-bold text-xs shadow-lg animate-in fade-in zoom-in-95 font-mono">
          <Zap className="h-3.5 w-3.5 fill-obsidian-950" />
          <span>{feedbackCue}</span>
        </div>
      )}

      {/* Fullscreen Header */}
      <header className="border-b border-obsidian-800 bg-obsidian-900 px-4 py-2.5">
        <div className="flex items-center justify-between">
          {/* Left: Prominent "Назад к доске" button and Task Details */}
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

          {/* Right: Actions and Close button */}
          <div className="flex items-center space-x-2">
            <button
              onClick={handlePauseResume}
              className={`flex items-center space-x-1.5 px-3 py-1.5 rounded text-xs font-mono font-medium transition cursor-pointer ${
                task.status === 'paused'
                  ? 'bg-emerald-600 hover:bg-emerald-500 text-white'
                  : 'bg-obsidian-850 hover:bg-obsidian-800 text-amber-300 border border-obsidian-750'
              }`}
            >
              {task.status === 'paused' ? (
                <>
                  <Play className="h-3.5 w-3.5" />
                  <span>Возобновить</span>
                </>
              ) : (
                <>
                  <Pause className="h-3.5 w-3.5" />
                  <span>Пауза</span>
                </>
              )}
            </button>

            <button
              onClick={handleForceKill}
              className="flex items-center space-x-1.5 px-3 py-1.5 rounded text-xs font-mono font-medium bg-rose-950/30 hover:bg-rose-900/50 text-rose-300 border border-rose-900/40 transition cursor-pointer"
              title="Принудительно остановить процесс"
            >
              <Square className="h-3.5 w-3.5" />
              <span>Стоп</span>
            </button>

            <button
              onClick={handleRetry}
              className="flex items-center space-x-1.5 px-3 py-1.5 rounded text-xs font-mono font-medium bg-obsidian-850 hover:bg-obsidian-800 text-slate-300 border border-obsidian-750 transition cursor-pointer"
              title="Повторить выполнение"
            >
              <RotateCw className="h-3.5 w-3.5" />
              <span>Повтор</span>
            </button>

            <button
              onClick={handlePromote}
              className="flex items-center space-x-1.5 px-3.5 py-1.5 rounded text-xs font-mono font-bold bg-cyan-400 hover:bg-cyan-300 text-obsidian-950 transition cursor-pointer"
              title="Перевести на следующий этап"
            >
              <span>Дальше</span>
              <ArrowRightCircle className="h-4 w-4" />
            </button>

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
          {/* Tab 1: Thoughts */}
          {activeTab === 'thoughts' && (
            <div className="space-y-3">
              <div className="p-2.5 rounded bg-obsidian-900 border border-obsidian-800 text-slate-400 flex items-center justify-between">
                <span className="flex items-center space-x-2">
                  <Cpu className="h-4 w-4 text-amber-400 shrink-0" />
                  <span>Поток мыслей &lt;thinking&gt; исполнителя {task.persona}</span>
                </span>
                <span className="text-[10px] text-amber-400 font-semibold flex items-center space-x-1">
                  <span className="h-1.5 w-1.5 rounded-full bg-amber-400 animate-pulse" />
                  <span>INTROSPECTION</span>
                </span>
              </div>

              {currentStream.thoughts.map((entry) => (
                <div
                  key={entry.id}
                  className="rounded border border-amber-900/30 bg-amber-950/10 border-l-2 border-l-amber-400 p-3 text-slate-300 leading-relaxed shadow-xs"
                >
                  <div className="flex items-center justify-between pb-1.5 mb-2 border-b border-amber-900/20 text-[10px]">
                    <span className="font-semibold text-amber-400 flex items-center space-x-1">
                      <span className="h-1.5 w-1.5 rounded-full bg-amber-400" />
                      <span>&lt;thinking&gt;</span>
                    </span>
                    <span className="tabular-nums text-slate-500">{entry.timestamp}</span>
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

      {/* Sticky Bottom Interactive Input Bar */}
      <footer className="border-t border-obsidian-800 bg-obsidian-900 p-3">
        <div className="max-w-5xl mx-auto space-y-2">
          {/* Quick Suggestion Tags */}
          <div className="flex items-center space-x-2 overflow-x-auto text-xs font-mono select-none">
            <button
              type="button"
              onClick={() => setInteractiveInput('▶ Продолжай')}
              className="px-2.5 py-1 rounded bg-obsidian-850 hover:bg-obsidian-800 border border-obsidian-750 hover:border-cyan-500/50 text-slate-300 hover:text-white transition cursor-pointer"
            >
              [▶ Продолжай]
            </button>
            <button
              type="button"
              onClick={() => setInteractiveInput('🔍 Тесты')}
              className="px-2.5 py-1 rounded bg-obsidian-850 hover:bg-obsidian-800 border border-obsidian-750 hover:border-cyan-500/50 text-slate-300 hover:text-white transition cursor-pointer"
            >
              [🔍 Тесты]
            </button>
            <button
              type="button"
              onClick={() => setInteractiveInput('🛑 Откати')}
              className="px-2.5 py-1 rounded bg-obsidian-850 hover:bg-obsidian-800 border border-obsidian-750 hover:border-rose-500/50 text-slate-300 hover:text-rose-200 transition cursor-pointer"
            >
              [🛑 Откати]
            </button>
            <button
              type="button"
              onClick={async () => {
                const { savedTokens } = await useDrawerStore.getState().compactTaskContext(task.id);
                showCue(`Сжато ~ ${savedTokens.toLocaleString()} токенов`);
              }}
              className="px-2.5 py-1 rounded bg-obsidian-850 hover:bg-obsidian-800 border border-amber-500/30 hover:border-amber-400 text-amber-400 hover:text-amber-300 transition cursor-pointer"
            >
              [⚡ Сжать контекст]
            </button>
          </div>

          {/* Input Form */}
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
