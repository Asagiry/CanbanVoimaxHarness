import { useState, useRef, useEffect, useId } from 'react';
import { useDrawerStore } from '../../stores/drawerStore';
import { useBoardStore } from '../../stores/boardStore';
import ToolExecutionCard from './ToolExecutionCard';
import MarkdownViewer from './MarkdownViewer';
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
  Sparkles,
  UserCheck,
} from 'lucide-react';

export default function AgentConsoleDrawer() {
  const {
    isOpen,
    activeTaskId,
    activeTab,
    streamEntries,
    toolExecutions,
    taskTelemetry,
    closeDrawer,
    setActiveTab,
    sendSteerMessage,
    compactTaskContext,
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
  const telemetry = taskTelemetry[task.id];

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
  };

  const handleForceKill = () => {
    updateTask(task.id, {
      status: 'blocked',
      isTimerRunning: false,
    });
    showCue('Agent Force Terminated');
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
  };

  const handleSendPrompt = async (e: React.FormEvent) => {
    e.preventDefault();
    const trimmed = interactiveInput.trim();
    if (!trimmed) return;

    setInteractiveInput('');
    await sendSteerMessage(task.id, trimmed);
    showCue('Инструкция отправлена агенту');
  };

  return (
    <div className="fixed inset-y-0 right-0 z-50 flex w-[680px] max-w-full flex-col bg-obsidian-900 border-l border-slate-750 shadow-2xl animate-in slide-in-from-right duration-200">
      {/* Visual Feedback Cue Toast */}
      {feedbackCue && (
        <div className="absolute top-14 left-1/2 -translate-x-1/2 z-50 flex items-center space-x-1.5 px-3 py-1.5 rounded-full bg-cyber-cyan text-obsidian-950 font-bold text-xs shadow-lg shadow-cyan-500/20 animate-in fade-in zoom-in-95">
          <Zap className="h-3.5 w-3.5 fill-obsidian-950" />
          <span>{feedbackCue}</span>
        </div>
      )}

      {/* Drawer Header */}
      <div className="flex flex-col border-b border-slate-800 bg-obsidian-850 p-4">
        <div className="flex items-start justify-between">
          <div className="flex items-center space-x-2 min-w-0">
            <span className="px-2 py-0.5 rounded font-mono text-xs font-bold bg-cyan-950/70 border border-cyan-800/50 text-cyber-cyan">
              {task.id}
            </span>
            <div
              className="px-2 py-0.5 rounded font-mono text-[11px] font-semibold uppercase"
              style={{
                backgroundColor: `${column?.color || '#64748b'}20`,
                color: column?.color || '#64748b',
                border: `1px solid ${column?.color || '#64748b'}40`,
              }}
            >
              {column?.title || task.columnId}
            </div>
            <span
              className={`px-2 py-0.5 rounded text-[11px] font-mono uppercase font-bold ${
                task.status === 'running'
                  ? 'bg-emerald-500/15 text-emerald-400 border border-emerald-500/30'
                  : task.status === 'paused'
                  ? 'bg-amber-500/15 text-amber-400 border border-amber-500/30'
                  : task.status === 'blocked'
                  ? 'bg-rose-500/15 text-rose-400 border border-rose-500/30'
                  : 'bg-slate-700/30 text-slate-300'
              }`}
            >
              ● {task.status}
            </span>
          </div>

          <button
            onClick={closeDrawer}
            className="p-1 rounded text-slate-400 hover:text-white hover:bg-obsidian-750 transition"
            title="Close Drawer"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        <h3 className="mt-2 text-sm font-bold text-white line-clamp-2">
          {task.title}
        </h3>

        {/* Worktree and Git Branch Meta */}
        <div className="mt-2.5 flex flex-wrap gap-2 text-[11px] font-mono text-slate-400">
          <div className="flex items-center space-x-1 px-2 py-0.5 rounded bg-obsidian-900 border border-slate-800">
            <FolderGit2 className="h-3 w-3 text-slate-500" />
            <span className="text-slate-300">{task.worktreePath}</span>
          </div>
          <div className="flex items-center space-x-1 px-2 py-0.5 rounded bg-obsidian-900 border border-slate-800">
            <GitBranch className="h-3 w-3 text-slate-500" />
            <span className="text-slate-300">{task.branch}</span>
          </div>
          {telemetry && telemetry.cacheHitRate > 0 && (
            <div className="flex items-center space-x-1 px-2 py-0.5 rounded bg-cyan-950/40 border border-cyan-800/40 text-cyber-cyan font-bold">
              <Zap className="h-3 w-3" />
              <span>Prefix Cache: {telemetry.cacheHitRate.toFixed(1)}%</span>
            </div>
          )}
        </div>
      </div>

      {/* State-Preserving Context Compactor Gauge */}
      <ContextGauge
        taskId={task.id}
        contextWindow={column?.contextWindow || 128000}
        currentEstimatedTokens={telemetry?.inputTokens || 14200}
        onCompact={() => compactTaskContext(task.id)}
      />

      {/* Action Controls Toolbar */}
      <div className="flex items-center justify-between border-b border-slate-800 bg-obsidian-850/80 px-4 py-2">
        <div className="flex items-center space-x-1.5">
          {/* Pause / Resume */}
          <button
            onClick={handlePauseResume}
            className={`flex items-center space-x-1 px-2.5 py-1 rounded text-xs font-semibold transition active:scale-95 ${
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
            className="flex items-center space-x-1 px-2.5 py-1 rounded text-xs font-semibold bg-rose-950/40 hover:bg-rose-900/60 text-rose-300 border border-rose-800/50 transition active:scale-95"
            title="Terminate Agent Subprocess"
          >
            <Square className="h-3 w-3" />
            <span>Force Kill</span>
          </button>

          {/* Retry */}
          <button
            onClick={handleRetry}
            className="flex items-center space-x-1 px-2.5 py-1 rounded text-xs font-medium bg-obsidian-800 hover:bg-obsidian-750 text-slate-300 border border-slate-750 transition active:scale-95"
            title="Re-run Agent Sensor Loop"
          >
            <RotateCw className="h-3 w-3" />
            <span>Retry</span>
          </button>
        </div>

        {/* Promote */}
        <button
          onClick={handlePromote}
          className="flex items-center space-x-1 px-3 py-1 rounded text-xs font-bold bg-cyber-cyan hover:bg-cyan-400 text-obsidian-950 transition active:scale-95 shadow-sm shadow-cyan-900/40"
          title="Promote task to next pipeline column"
        >
          <span>Promote</span>
          <ArrowRightCircle className="h-3.5 w-3.5" />
        </button>
      </div>

      {/* Tabs Header */}
      <div className="flex border-b border-slate-800 bg-obsidian-900 px-4 text-xs font-medium">
        <button
          onClick={() => setActiveTab('thoughts')}
          className={`flex items-center space-x-2 py-2.5 px-3 border-b-2 transition select-none ${
            activeTab === 'thoughts'
              ? 'border-cyber-cyan text-cyber-cyan font-bold'
              : 'border-transparent text-slate-400 hover:text-slate-200'
          }`}
        >
          <Brain className="h-3.5 w-3.5" />
          <span>Agent Thoughts</span>
          <span className="font-mono text-[10px] tabular-nums px-1 rounded bg-obsidian-800 text-slate-400">
            {currentStream.thoughts.length}
          </span>
        </button>

        <button
          onClick={() => setActiveTab('output')}
          className={`flex items-center space-x-2 py-2.5 px-3 border-b-2 transition select-none ${
            activeTab === 'output'
              ? 'border-cyber-cyan text-cyber-cyan font-bold'
              : 'border-transparent text-slate-400 hover:text-slate-200'
          }`}
        >
          <MessageSquare className="h-3.5 w-3.5" />
          <span>Chat / Assistant Stream</span>
          <span className="font-mono text-[10px] tabular-nums px-1 rounded bg-obsidian-800 text-slate-400">
            {currentStream.outputs.length}
          </span>
        </button>

        <button
          onClick={() => setActiveTab('tools')}
          className={`flex items-center space-x-2 py-2.5 px-3 border-b-2 transition select-none ${
            activeTab === 'tools'
              ? 'border-cyber-cyan text-cyber-cyan font-bold'
              : 'border-transparent text-slate-400 hover:text-slate-200'
          }`}
        >
          <Terminal className="h-3.5 w-3.5" />
          <span>Tool Executions</span>
          <span className="font-mono text-[10px] tabular-nums px-1 rounded bg-obsidian-800 text-slate-400">
            {currentTools.length}
          </span>
        </button>
      </div>

      {/* Tab Content Stream Area */}
      <div ref={scrollRef} className="flex-1 overflow-y-auto p-4 space-y-3 bg-obsidian-950 font-mono text-xs">
        {/* Tab 1: Live Thinking Stream (<thinking> tokens with distinct monospace & glowing amber/neon accent border) */}
        {activeTab === 'thoughts' && (
          <div className="space-y-3">
            <div className="p-2 rounded bg-amber-950/30 border border-amber-500/40 text-[11px] text-amber-300 flex items-center justify-between shadow-[0_0_12px_rgba(245,158,11,0.15)]">
              <span className="flex items-center space-x-1.5">
                <Cpu className="h-3.5 w-3.5 text-cyber-amber animate-pulse" />
                <span className="font-bold">Live Thinking Stream (&lt;thinking&gt; tokens)</span>
              </span>
              <span className="text-[10px] text-cyber-amber font-mono font-bold animate-pulse">
                ● LIVE REASONING
              </span>
            </div>

            {currentStream.thoughts.map((entry) => (
              <div
                key={entry.id}
                className="rounded-lg border border-amber-500/40 bg-obsidian-900/90 p-3.5 text-slate-200 leading-relaxed shadow-[0_0_14px_rgba(245,158,11,0.15)] transition-all"
              >
                <div className="flex items-center justify-between pb-1.5 mb-2 border-b border-amber-500/30 text-[10px]">
                  <span className="font-mono font-bold text-cyber-amber flex items-center space-x-1">
                    <Sparkles className="h-3 w-3" />
                    <span>&lt;thinking&gt;</span>
                  </span>
                  <div className="flex items-center space-x-2 text-slate-400 font-mono">
                    {entry.durationMs && (
                      <span className="text-amber-400 tabular-nums">
                        +{(entry.durationMs / 1000).toFixed(2)}s
                      </span>
                    )}
                    <span className="tabular-nums">{entry.timestamp}</span>
                  </div>
                </div>
                <div className="whitespace-pre-wrap select-text font-mono text-xs text-slate-200 leading-relaxed font-normal">
                  {entry.content}
                </div>
              </div>
            ))}

            {currentStream.thoughts.length === 0 && (
              <div className="h-40 flex flex-col items-center justify-center text-slate-600 font-mono">
                <span>No thoughts recorded yet for this task</span>
              </div>
            )}
          </div>
        )}

        {/* Tab 2: Rich Markdown Assistant / Chat Stream */}
        {activeTab === 'output' && (
          <div className="space-y-3 font-sans">
            {currentStream.outputs.map((entry) => {
              const isSteer = entry.type === 'steer' || entry.type === 'user';
              return (
                <div
                  key={entry.id}
                  className={`rounded-lg border p-3.5 leading-relaxed shadow-sm transition ${
                    isSteer
                      ? 'border-cyan-500/60 bg-cyan-950/20 text-cyan-100'
                      : entry.type === 'error'
                      ? 'border-rose-900/60 bg-rose-950/20 text-rose-200'
                      : entry.type === 'system'
                      ? 'border-amber-900/50 bg-amber-950/20 text-amber-200 font-mono text-xs'
                      : 'border-slate-800 bg-obsidian-900 text-slate-200'
                  }`}
                >
                  <div className="flex items-center justify-between pb-1.5 mb-2 border-b border-slate-800/60 font-mono text-[10px] text-slate-400">
                    <span className="font-semibold uppercase flex items-center space-x-1">
                      {isSteer ? (
                        <>
                          <UserCheck className="h-3 w-3 text-cyber-cyan" />
                          <span className="text-cyber-cyan font-bold">User Steering</span>
                        </>
                      ) : (
                        <span>{entry.type}</span>
                      )}
                    </span>
                    <span className="tabular-nums">{entry.timestamp}</span>
                  </div>

                  {isSteer || entry.type === 'system' ? (
                    <p className="whitespace-pre-wrap select-text text-xs">{entry.content}</p>
                  ) : (
                    <MarkdownViewer content={entry.content} />
                  )}
                </div>
              );
            })}

            {currentStream.outputs.length === 0 && (
              <div className="h-40 flex flex-col items-center justify-center font-mono text-slate-600">
                <span>No assistant output generated yet</span>
              </div>
            )}
          </div>
        )}

        {/* Tab 3: Interactive Tool Execution Accordions */}
        {activeTab === 'tools' && (
          <div className="space-y-2.5">
            {currentTools.map((tool) => (
              <ToolExecutionCard key={tool.id} tool={tool} />
            ))}

            {currentTools.length === 0 && (
              <div className="h-40 flex flex-col items-center justify-center text-slate-600">
                <span>No tool executions recorded for this task</span>
              </div>
            )}
          </div>
        )}
      </div>

      {/* Interactive Sticky Bottom Input Bar (Steer / Chat Input) */}
      <div className="border-t border-slate-800 bg-obsidian-900 p-3">
        <form onSubmit={handleSendPrompt} className="flex items-center space-x-2">
          <input
            id={promptInputId}
            name="interactivePromptInput"
            type="text"
            value={interactiveInput}
            onChange={(e) => setInteractiveInput(e.target.value)}
            placeholder="Отправить инструкцию / дообучить агента прямо в карточке..."
            className="flex-1 rounded-lg bg-obsidian-950 border border-slate-750 px-3.5 py-2 text-xs text-slate-100 placeholder-slate-500 focus:outline-hidden focus:border-cyan-500 font-sans transition"
          />
          <button
            type="submit"
            className="p-2 rounded-lg bg-cyber-cyan hover:bg-cyan-400 text-obsidian-950 font-bold transition active:scale-95 shrink-0 cursor-pointer shadow-sm shadow-cyan-900/40"
            title="Отправить инструкцию / запустить следующий шаг"
          >
            <Send className="h-4 w-4" />
          </button>
        </form>
      </div>
    </div>
  );
}
