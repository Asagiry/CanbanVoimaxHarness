import { useState, useRef, useEffect, useId } from 'react';
import { useDrawerStore } from '../../stores/drawerStore';
import { useBoardStore } from '../../stores/boardStore';
import ToolExecutionCard from './ToolExecutionCard';
import ContextGauge from './ContextGauge';
import {
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
    showCue(isPaused ? 'Agent Resumed' : 'Agent Paused');
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
    showCue('Agent Force Terminated');
    appendOutput(
      task.id,
      '[CRITICAL] Agent process terminated via Force Kill signal (SIGKILL). Worktree preserved.',
      'error'
    );
  };

  const handlePromote = () => {
    promoteTaskToNextStage(task.id);
    showCue('Promoted to Next Stage');
  };

  const handleRetry = () => {
    updateTask(task.id, {
      status: 'running',
      isTimerRunning: true,
      verdict: 'NONE',
    });
    showCue('Re-running Agent Sensors');
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
    <div className="fixed inset-y-0 right-0 z-50 flex w-[640px] max-w-full flex-col bg-obsidian-900 border-l border-obsidian-800 shadow-2xl animate-in slide-in-from-right duration-150">
      {/* Visual Feedback Cue Toast */}
      {feedbackCue && (
        <div className="absolute top-12 left-1/2 -translate-x-1/2 z-50 flex items-center space-x-1.5 px-2.5 py-1 rounded-full bg-cyber-cyan text-obsidian-950 font-bold text-xs shadow-md shadow-cyan-500/20 animate-in fade-in zoom-in-95 font-mono">
          <Zap className="h-3 w-3 fill-obsidian-950" />
          <span>{feedbackCue}</span>
        </div>
      )}

      {/* Drawer Header - Compact Ergonomics */}
      <div className="flex flex-col border-b border-obsidian-800 bg-obsidian-850 px-3.5 py-2.5">
        <div className="flex items-start justify-between">
          <div className="flex items-center space-x-2 min-w-0">
            <span className="px-1.5 py-0.2 rounded font-mono text-[11px] font-bold bg-cyan-950/70 border border-cyan-800/50 text-cyber-cyan tabular-nums">
              {task.id}
            </span>
            <div
              className="px-1.5 py-0.2 rounded font-mono text-[10px] font-semibold uppercase"
              style={{
                backgroundColor: `${column?.color || '#64748b'}20`,
                color: column?.color || '#64748b',
                border: `1px solid ${column?.color || '#64748b'}40`,
              }}
            >
              {column?.title || task.columnId}
            </div>

            {/* Status badge */}
            <span
              className={`px-1.5 py-0.2 rounded text-[10px] font-mono uppercase font-bold flex items-center space-x-1 tabular-nums ${
                task.status === 'running'
                  ? 'bg-emerald-950/40 text-cyber-emerald border border-emerald-500/30'
                  : task.status === 'paused'
                  ? 'bg-amber-950/40 text-cyber-amber border border-amber-500/30'
                  : task.status === 'waiting_review'
                  ? 'bg-cyan-950/40 text-cyber-cyan border border-cyan-500/30'
                  : task.status === 'done'
                  ? 'bg-emerald-950/40 text-cyber-emerald border border-emerald-500/30'
                  : task.status === 'blocked'
                  ? 'bg-rose-950/40 text-cyber-crimson border border-rose-500/30'
                  : 'bg-obsidian-800 text-slate-400 border border-obsidian-750'
              }`}
            >
              {task.status === 'running' && (
                <>
                  <span className="h-1.5 w-1.5 rounded-full bg-cyber-emerald animate-pulse" />
                  <span>Running</span>
                </>
              )}
              {task.status === 'paused' && (
                <>
                  <span className="h-1.5 w-1.5 rounded-full bg-cyber-amber" />
                  <span>Paused</span>
                </>
              )}
              {task.status === 'waiting_review' && (
                <>
                  <span className="h-1.5 w-1.5 rounded-full bg-cyber-cyan animate-pulse" />
                  <span>Awaiting Review</span>
                </>
              )}
              {task.status === 'done' && (
                <>
                  <span className="h-1.5 w-1.5 rounded-full bg-cyber-emerald" />
                  <span>Done</span>
                </>
              )}
              {task.status === 'blocked' && (
                <>
                  <span className="h-1.5 w-1.5 rounded-full bg-cyber-crimson" />
                  <span>Blocked</span>
                </>
              )}
              {task.status === 'idle' && (
                <>
                  <span className="h-1.5 w-1.5 rounded-full bg-slate-400" />
                  <span>Idle</span>
                </>
              )}
            </span>
          </div>

          <button
            onClick={closeDrawer}
            className="p-1 rounded text-slate-400 hover:text-white hover:bg-obsidian-750 transition cursor-pointer"
            title="Close Drawer"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        <h3 className="mt-1.5 text-sm font-semibold text-white line-clamp-2">
          {task.title}
        </h3>

        {/* Worktree and Git Branch Meta */}
        <div className="mt-2 flex flex-wrap gap-2 text-[10px] font-mono text-slate-400">
          <div className="flex items-center space-x-1 px-1.5 py-0.2 rounded bg-obsidian-950 border border-obsidian-800">
            <FolderGit2 className="h-3 w-3 text-slate-500" />
            <span className="text-slate-300">{task.worktreePath}</span>
          </div>
          <div className="flex items-center space-x-1 px-1.5 py-0.2 rounded bg-obsidian-950 border border-obsidian-800">
            <GitBranch className="h-3 w-3 text-slate-500" />
            <span className="text-slate-300">{task.branch}</span>
          </div>
        </div>

        {/* Embedded Context Window Gauge */}
        <div className="mt-2.5">
          <ContextGauge taskId={task.id} />
        </div>
      </div>

      {/* Action Controls Toolbar - Developer Ergonomics */}
      <div className="flex items-center justify-between border-b border-obsidian-800 bg-obsidian-850/80 px-3.5 py-1.5">
        <div className="flex items-center space-x-1.5">
          {/* Pause / Resume */}
          <button
            onClick={handlePauseResume}
            className={`flex items-center space-x-1 px-2.5 py-1 rounded-sm text-xs font-mono font-semibold transition active:scale-[0.98] cursor-pointer ${
              task.status === 'paused'
                ? 'bg-emerald-600 hover:bg-emerald-500 text-white'
                : 'bg-obsidian-800 hover:bg-obsidian-750 text-amber-300 border border-amber-500/30'
            }`}
          >
            {task.status === 'paused' ? (
              <>
                <Play className="h-3 w-3" />
                <span>Resume</span>
              </>
            ) : (
              <>
                <Pause className="h-3 w-3" />
                <span>Pause</span>
              </>
            )}
          </button>

          {/* Force Kill */}
          <button
            onClick={handleForceKill}
            className="flex items-center space-x-1 px-2.5 py-1 rounded-sm text-xs font-mono font-semibold bg-rose-950/40 hover:bg-rose-900/60 text-cyber-crimson border border-rose-800/50 transition active:scale-[0.98] cursor-pointer"
            title="Terminate Agent Subprocess (SIGKILL)"
          >
            <Square className="h-3 w-3" />
            <span>Force Kill</span>
          </button>

          {/* Retry */}
          <button
            onClick={handleRetry}
            className="flex items-center space-x-1 px-2.5 py-1 rounded-sm text-xs font-mono font-medium bg-obsidian-800 hover:bg-obsidian-750 text-slate-300 border border-obsidian-750 transition active:scale-[0.98] cursor-pointer"
            title="Re-run Agent Sensor Loop"
          >
            <RotateCw className="h-3 w-3" />
            <span>Retry</span>
          </button>
        </div>

        {/* Promote */}
        <button
          onClick={handlePromote}
          className="flex items-center space-x-1 px-3 py-1 rounded-sm text-xs font-mono font-bold bg-cyber-cyan hover:bg-cyan-400 text-obsidian-950 transition active:scale-[0.98] cursor-pointer"
          title="Promote task to next pipeline column"
        >
          <span>Promote</span>
          <ArrowRightCircle className="h-3.5 w-3.5" />
        </button>
      </div>

      {/* Tabs Header */}
      <div className="flex border-b border-obsidian-800 bg-obsidian-900 px-3.5 text-xs font-medium font-mono select-none">
        <button
          onClick={() => setActiveTab('thoughts')}
          className={`flex items-center space-x-1.5 py-2 px-2.5 border-b-2 transition cursor-pointer ${
            activeTab === 'thoughts'
              ? 'border-cyber-cyan text-cyber-cyan font-bold'
              : 'border-transparent text-slate-400 hover:text-slate-200'
          }`}
        >
          <Brain className="h-3.5 w-3.5" />
          <span>&lt;thinking&gt;</span>
          <span className="font-mono text-[10px] tabular-nums px-1 py-0.2 rounded bg-obsidian-800 text-slate-400 border border-obsidian-750">
            {currentStream.thoughts.length}
          </span>
        </button>

        <button
          onClick={() => setActiveTab('output')}
          className={`flex items-center space-x-1.5 py-2 px-2.5 border-b-2 transition cursor-pointer ${
            activeTab === 'output'
              ? 'border-cyber-cyan text-cyber-cyan font-bold'
              : 'border-transparent text-slate-400 hover:text-slate-200'
          }`}
        >
          <MessageSquare className="h-3.5 w-3.5" />
          <span>Output Stream</span>
          <span className="font-mono text-[10px] tabular-nums px-1 py-0.2 rounded bg-obsidian-800 text-slate-400 border border-obsidian-750">
            {currentStream.outputs.length}
          </span>
        </button>

        <button
          onClick={() => setActiveTab('tools')}
          className={`flex items-center space-x-1.5 py-2 px-2.5 border-b-2 transition cursor-pointer ${
            activeTab === 'tools'
              ? 'border-cyber-cyan text-cyber-cyan font-bold'
              : 'border-transparent text-slate-400 hover:text-slate-200'
          }`}
        >
          <Terminal className="h-3.5 w-3.5" />
          <span>Tool Executions</span>
          <span className="font-mono text-[10px] tabular-nums px-1 py-0.2 rounded bg-obsidian-800 text-slate-400 border border-obsidian-750">
            {currentTools.length}
          </span>
        </button>
      </div>

      {/* Tab Content Stream Area */}
      <div ref={scrollRef} className="flex-1 overflow-y-auto p-3.5 space-y-2.5 bg-obsidian-950 font-mono text-xs">
        {/* Tab 1: Thoughts Stream - Dedicated <thinking> block with dim slate typography and glowing amber accent */}
        {activeTab === 'thoughts' && (
          <div className="space-y-2.5">
            <div className="p-2 rounded-md bg-obsidian-900 border border-obsidian-800 text-[11px] text-slate-400 flex items-center justify-between">
              <span className="flex items-center space-x-1.5">
                <Cpu className="h-3.5 w-3.5 text-cyber-amber shrink-0" />
                <span>Demuxed &lt;thinking&gt; tokens from {task.persona}</span>
              </span>
              <span className="text-[10px] text-cyber-amber font-semibold font-mono flex items-center space-x-1">
                <span className="h-1.5 w-1.5 rounded-full bg-cyber-amber animate-pulse" />
                <span>INTROSPECTION</span>
              </span>
            </div>

            {currentStream.thoughts.map((entry) => (
              <div
                key={entry.id}
                className="rounded-md border border-amber-900/40 bg-amber-950/10 border-l-2 border-l-cyber-amber p-2.5 text-slate-400 leading-relaxed shadow-xs"
              >
                <div className="flex items-center justify-between pb-1 mb-1.5 border-b border-amber-900/30 text-[10px]">
                  <span className="font-semibold text-cyber-amber font-mono flex items-center space-x-1">
                    <span className="h-1.5 w-1.5 rounded-full bg-cyber-amber" />
                    <span>&lt;thinking&gt;</span>
                  </span>
                  <span className="tabular-nums text-slate-500 font-mono">{entry.timestamp}</span>
                </div>
                <p className="whitespace-pre-wrap select-text text-slate-400 text-xs leading-relaxed font-mono">
                  {entry.content}
                </p>
              </div>
            ))}

            {currentStream.thoughts.length === 0 && (
              <div className="h-36 flex flex-col items-center justify-center text-slate-600 font-mono text-xs">
                <span>No &lt;thinking&gt; tokens emitted yet</span>
              </div>
            )}
          </div>
        )}

        {/* Tab 2: Assistant Output Stream */}
        {activeTab === 'output' && (
          <div className="space-y-2.5 font-sans">
            {currentStream.outputs.map((entry) => (
              <div
                key={entry.id}
                className={`rounded-md border p-2.5 leading-relaxed shadow-xs ${
                  entry.type === 'error'
                    ? 'border-rose-900/60 bg-rose-950/20 text-rose-200'
                    : entry.type === 'system'
                    ? 'border-cyan-900/50 bg-cyan-950/20 text-cyan-200'
                    : 'border-obsidian-800 bg-obsidian-900 text-slate-200'
                }`}
              >
                <div className="flex items-center justify-between pb-1 mb-1 border-b border-obsidian-800 font-mono text-[10px] text-slate-400">
                  <span className="font-semibold uppercase tracking-wider">{entry.type}</span>
                  <span className="tabular-nums font-mono">{entry.timestamp}</span>
                </div>
                <p className="whitespace-pre-wrap select-text text-xs leading-relaxed">
                  {entry.content}
                </p>
              </div>
            ))}

            {currentStream.outputs.length === 0 && (
              <div className="h-36 flex flex-col items-center justify-center font-mono text-slate-600 text-xs">
                <span>No assistant output recorded</span>
              </div>
            )}
          </div>
        )}

        {/* Tab 3: Tool Execution Cards */}
        {activeTab === 'tools' && (
          <div className="space-y-2">
            {currentTools.map((tool) => (
              <ToolExecutionCard key={tool.id} tool={tool} />
            ))}

            {currentTools.length === 0 && (
              <div className="h-36 flex flex-col items-center justify-center text-slate-600 font-mono text-xs">
                <span>No tool executions recorded</span>
              </div>
            )}
          </div>
        )}
      </div>

      {/* Interactive Input Prompt Bar - Compact & Sticky */}
      <div className="border-t border-obsidian-800 bg-obsidian-900 p-2.5 space-y-2">
        {/* Quick Prompt Tactile Suggestion Tags */}
        <div className="flex items-center space-x-1.5 overflow-x-auto pb-0.5 text-[10px] font-mono select-none">
          <span className="text-slate-500 uppercase font-semibold shrink-0">Tags:</span>
          <button
            type="button"
            onClick={() => setInteractiveInput('▶ Продолжай')}
            className="px-2 py-0.5 rounded-sm bg-obsidian-850 hover:bg-obsidian-800 border border-obsidian-750 hover:border-cyan-500/50 text-slate-300 hover:text-white transition active:scale-[0.98] shrink-0 cursor-pointer"
          >
            [▶ Продолжай]
          </button>
          <button
            type="button"
            onClick={() => setInteractiveInput('🔍 Тесты')}
            className="px-2 py-0.5 rounded-sm bg-obsidian-850 hover:bg-obsidian-800 border border-obsidian-750 hover:border-cyan-500/50 text-slate-300 hover:text-white transition active:scale-[0.98] shrink-0 cursor-pointer"
          >
            [🔍 Тесты]
          </button>
          <button
            type="button"
            onClick={() => setInteractiveInput('🛑 Откати')}
            className="px-2 py-0.5 rounded-sm bg-obsidian-850 hover:bg-obsidian-800 border border-obsidian-750 hover:border-rose-500/50 text-slate-300 hover:text-rose-200 transition active:scale-[0.98] shrink-0 cursor-pointer"
          >
            [🛑 Откати]
          </button>
          <button
            type="button"
            onClick={async () => {
              const { savedTokens } = await useDrawerStore.getState().compactTaskContext(task.id);
              showCue(`🔥 ~${savedTokens.toLocaleString()} tokens saved!`);
            }}
            className="px-2 py-0.5 rounded-sm bg-obsidian-850 hover:bg-obsidian-800 border border-amber-500/30 hover:border-amber-400 text-cyber-amber hover:text-amber-300 transition active:scale-[0.98] shrink-0 font-semibold cursor-pointer"
          >
            [⚡ Сжать]
          </button>
        </div>

        <form onSubmit={handleSendPrompt} className="flex items-center space-x-1.5">
          <input
            id={promptInputId}
            name="interactivePromptInput"
            type="text"
            value={interactiveInput}
            onChange={(e) => setInteractiveInput(e.target.value)}
            placeholder={
              activeTab === 'thoughts'
                ? 'Inject human observation into agent mind...'
                : 'Send instruction to assistant stream...'
            }
            className="flex-1 rounded-sm bg-obsidian-950 border border-obsidian-750 px-2.5 py-1 text-xs text-slate-100 placeholder-slate-500 focus:outline-hidden focus:border-cyan-500 font-mono"
          />
          <button
            type="submit"
            className="p-1.5 rounded-sm bg-cyber-cyan hover:bg-cyan-400 text-obsidian-950 transition active:scale-[0.98] shrink-0 cursor-pointer"
            title="Inject message"
          >
            <Send className="h-3.5 w-3.5" />
          </button>
        </form>
      </div>
    </div>
  );
}
