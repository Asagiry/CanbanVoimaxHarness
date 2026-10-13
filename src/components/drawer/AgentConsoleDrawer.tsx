import { useState, useRef, useEffect, useId, useMemo } from 'react';
import { useDrawerStore, DrawerActivityItem } from '../../stores/drawerStore';
import { useBoardStore, syncTasksBackend } from '../../stores/boardStore';
import { useSettingsStore } from '../../stores/settingsStore';
import ToolExecutionCard from './ToolExecutionCard';
import ContextGauge from './ContextGauge';
import {
  ArrowLeft,
  X,
  GitBranch,
  FolderGit2,
  Brain,
  Send,
  Play,
  Square,
  Target,
  ChevronDown,
  ChevronRight,
  Sparkles,
  Clock,
  Cpu,
  User,
  AlertTriangle,
  Info,
} from 'lucide-react';

// Lightweight Markdown Renderer for streaming assistant output
function MarkdownContent({ content }: { content: string }) {
  const elements = useMemo(() => {
    const lines = content.split('\n');
    const result: React.ReactNode[] = [];
    let inCodeBlock = false;
    let codeLanguage = '';
    let codeBuffer: string[] = [];

    const formatInline = (text: string): React.ReactNode[] => {
      // Inline code `code`
      const parts = text.split(/(`[^`]+`|\*\*[^*]+\*\*)/g);
      return parts.map((part, i) => {
        if (part.startsWith('`') && part.endsWith('`') && part.length >= 2) {
          return (
            <code
              key={i}
              className="px-1.5 py-0.5 rounded bg-[#f4f4f5] border border-[#ebebeb] text-[#171717] font-mono text-[11px]"
            >
              {part.slice(1, -1)}
            </code>
          );
        }
        if (part.startsWith('**') && part.endsWith('**') && part.length >= 4) {
          return (
            <strong key={i} className="font-semibold text-[#171717]">
              {part.slice(2, -2)}
            </strong>
          );
        }
        return part;
      });
    };

    lines.forEach((line, index) => {
      if (line.startsWith('```')) {
        if (!inCodeBlock) {
          inCodeBlock = true;
          codeLanguage = line.slice(3).trim();
          codeBuffer = [];
        } else {
          inCodeBlock = false;
          result.push(
            <div
              key={`code-${index}`}
              className="my-2.5 rounded-md border border-[#ebebeb] bg-[#fafafa] overflow-hidden font-mono text-[11px]"
            >
              {codeLanguage && (
                <div className="px-3 py-1 bg-[#f4f4f5] border-b border-[#ebebeb] text-[10px] text-[#666666] font-medium flex justify-between">
                  <span>{codeLanguage}</span>
                </div>
              )}
              <pre className="p-3 text-[#171717] overflow-x-auto leading-relaxed select-text">
                {codeBuffer.join('\n')}
              </pre>
            </div>
          );
        }
        return;
      }

      if (inCodeBlock) {
        codeBuffer.push(line);
        return;
      }

      // Headings
      if (line.startsWith('### ')) {
        result.push(
          <h4 key={index} className="text-xs font-semibold text-[#171717] mt-3 mb-1">
            {formatInline(line.slice(4))}
          </h4>
        );
      } else if (line.startsWith('## ')) {
        result.push(
          <h3 key={index} className="text-sm font-semibold text-[#171717] mt-3 mb-1">
            {formatInline(line.slice(3))}
          </h3>
        );
      } else if (line.startsWith('# ')) {
        result.push(
          <h2 key={index} className="text-sm font-bold text-[#171717] mt-3 mb-1.5">
            {formatInline(line.slice(2))}
          </h2>
        );
      } else if (line.startsWith('- ') || line.startsWith('* ')) {
        result.push(
          <li key={index} className="ml-4 list-disc text-xs leading-relaxed text-[#171717]">
            {formatInline(line.slice(2))}
          </li>
        );
      } else if (line.trim() === '') {
        result.push(<div key={index} className="h-1.5" />);
      } else {
        result.push(
          <p key={index} className="text-xs leading-relaxed text-[#171717]">
            {formatInline(line)}
          </p>
        );
      }
    });

    if (inCodeBlock && codeBuffer.length > 0) {
      result.push(
        <div
          key="code-unfinished"
          className="my-2.5 rounded-md border border-[#ebebeb] bg-[#fafafa] overflow-hidden font-mono text-[11px]"
        >
          <pre className="p-3 text-[#171717] overflow-x-auto leading-relaxed select-text">
            {codeBuffer.join('\n')}
          </pre>
        </div>
      );
    }

    return result;
  }, [content]);

  return <div className="space-y-1 select-text font-sans">{elements}</div>;
}

// Collapsible Live Thinking Card
function ThinkingCard({
  thought,
  isTaskRunning,
}: {
  thought: DrawerActivityItem;
  isTaskRunning: boolean;
}) {
  const [isExpanded, setIsExpanded] = useState(true);

  const durationSec = thought.durationMs ? Math.round(thought.durationMs / 1000) : 0;

  return (
    <div className="rounded-md border border-[#ebebeb] bg-[#fcfcfc] overflow-hidden shadow-xs transition duration-150">
      <div
        onClick={() => setIsExpanded(!isExpanded)}
        className="flex items-center justify-between px-3 py-2 bg-[#f8f8f9] hover:bg-[#f4f4f5] cursor-pointer transition select-none border-l-2 border-l-[#171717]"
      >
        <div className="flex items-center space-x-2">
          {isExpanded ? (
            <ChevronDown className="h-3.5 w-3.5 text-[#666666]" />
          ) : (
            <ChevronRight className="h-3.5 w-3.5 text-[#666666]" />
          )}
          <Brain className="h-3.5 w-3.5 text-[#171717]" />
          <span className="font-mono text-xs font-semibold text-[#171717]">
            &lt;thinking&gt;
          </span>
          {isTaskRunning && (
            <span className="flex items-center space-x-1 px-1.5 py-0.2 rounded text-[10px] font-mono text-[#15803d] bg-[#f0fdf4] border border-[#bbf7d0]">
              <span className="h-1.5 w-1.5 rounded-full bg-[#15803d] animate-pulse" />
              <span>размышление...</span>
            </span>
          )}
        </div>

        <div className="flex items-center space-x-2 text-[10px] font-mono text-[#666666]">
          {durationSec > 0 && (
            <span className="flex items-center space-x-1 tabular-nums">
              <Clock className="h-3 w-3" />
              <span>{durationSec}с</span>
            </span>
          )}
          <span className="tabular-nums">{thought.timestamp}</span>
        </div>
      </div>

      {isExpanded && (
        <div className="p-3 border-t border-[#ebebeb] bg-[#ffffff] font-mono text-xs leading-relaxed text-[#333333] whitespace-pre-wrap select-text max-h-96 overflow-y-auto">
          {thought.content}
        </div>
      )}
    </div>
  );
}

export default function AgentConsoleDrawer() {
  const {
    isOpen,
    activeTaskId,
    activityFeed,
    streamEntries,
    toolExecutions,
    closeDrawer,
    appendOutput,
  } = useDrawerStore();

  const { tasks, columns, updateTask } = useBoardStore();
  const { settings } = useSettingsStore();

  const [interactiveInput, setInteractiveInput] = useState('');
  const [runError, setRunError] = useState<string | null>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const promptInputId = useId();

  const task = tasks.find((t) => t.id === activeTaskId);
  const column = columns.find((c) => c.id === task?.columnId);

  // Reset error when switching tasks
  useEffect(() => {
    setRunError(null);
  }, [activeTaskId]);

  // Listen to agent error & status events from Tauri backend
  useEffect(() => {
    if (!task?.id) return;
    let unlistenStatus: (() => void) | undefined;
    let unlistenError: (() => void) | undefined;

    const setupErrorListeners = async () => {
      try {
        const { listen } = await import('@tauri-apps/api/event');
        unlistenStatus = await listen<{ taskId?: string; status?: string }>('agent:status', (event) => {
          if (event.payload?.taskId === task.id) {
            if (event.payload.status === 'error') {
              setRunError((prev) => prev || 'Агент сообщил об ошибке выполнения');
              updateTask(task.id, { status: 'idle', isTimerRunning: false });
            } else if (event.payload.status === 'running') {
              setRunError(null);
            }
          }
        });

        unlistenError = await listen<{ taskId?: string; error?: string }>('agent:error', (event) => {
          if (event.payload && (!event.payload.taskId || event.payload.taskId === task.id)) {
            setRunError(event.payload.error || 'Произошла непредвиденная ошибка агента');
            updateTask(task.id, { status: 'idle', isTimerRunning: false });
          }
        });
      } catch {
        // web mode fallback
      }
    };

    setupErrorListeners();
    return () => {
      if (unlistenStatus) unlistenStatus();
      if (unlistenError) unlistenError();
    };
  }, [task?.id, updateTask]);

  // Esc key listener to close drawer
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

  // Unified Chronological Activity List
  const unifiedActivities = useMemo(() => {
    if (!task) return [];

    const feed = activityFeed[task.id];
    if (feed && feed.length > 0) {
      return feed;
    }

    // Fallback: merge old streamEntries and toolExecutions if feed is not populated yet
    const rawStream = streamEntries[task.id] || { thoughts: [], outputs: [] };
    const rawTools = toolExecutions[task.id] || [];

    const merged: DrawerActivityItem[] = [
      ...rawStream.thoughts.map((th) => ({
        id: th.id,
        type: 'thought' as const,
        timestamp: th.timestamp,
        content: th.content,
        durationMs: th.durationMs,
      })),
      ...rawStream.outputs.map((out) => ({
        id: out.id,
        type: (out.type === 'steer' || out.type === 'system' || out.type === 'error' || out.type === 'user'
          ? out.type
          : 'output') as any,
        timestamp: out.timestamp,
        content: out.content,
      })),
      ...rawTools.map((t) => ({
        id: t.id,
        type: 'tool' as const,
        timestamp: t.timestamp,
        tool: t,
      })),
    ];

    return merged;
  }, [task, activityFeed, streamEntries, toolExecutions]);

  // Auto-scroll on new entries
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [unifiedActivities]);

  if (!isOpen || !task) return null;

  const isRunning = task.status === 'running';
  const effectiveModel = task.model || settings.model || 'gemini-3.8-flash-high';

  const handleRunAgent = async () => {
    setRunError(null);
    updateTask(task.id, { status: 'running', isTimerRunning: true });
    try {
      await syncTasksBackend(tasks);
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('run_agent', { taskId: task.id });
    } catch (err: any) {
      console.info('run_agent invoke failed or web mode:', err);
      const errMsg = err?.message || (typeof err === 'string' ? err : 'Не удалось запустить агента');
      setRunError(errMsg);
      updateTask(task.id, { status: 'idle', isTimerRunning: false });
    }
  };

  const handleAbortAgent = async () => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('abort_agent', { taskId: task.id });
      updateTask(task.id, { status: 'paused', isTimerRunning: false });
    } catch (err) {
      console.info('abort_agent invoke failed or web mode:', err);
    }
  };

  const handleSendPrompt = async (e: React.FormEvent) => {
    e.preventDefault();
    const trimmed = interactiveInput.trim();
    if (!trimmed) return;

    setInteractiveInput('');
    setRunError(null);

    // Append user steer message into feed immediately
    appendOutput(task.id, trimmed, 'user');

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await syncTasksBackend(tasks);
      if (isRunning) {
        await invoke('steer_agent', { taskId: task.id, message: trimmed });
      } else {
        updateTask(task.id, { status: 'running', isTimerRunning: true });
        await invoke('run_agent', { taskId: task.id });
      }
    } catch (err: any) {
      console.info('steer_agent / run_agent invoke failed or web mode:', err);
      const errMsg = err?.message || (typeof err === 'string' ? err : 'Ошибка отправки команды агенту');
      setRunError(errMsg);
      if (!isRunning) {
        updateTask(task.id, { status: 'idle', isTimerRunning: false });
      }
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex flex-col bg-white text-[#171717] select-none animate-in fade-in duration-150">
      {/* Fullscreen Header: Clean Vercel / Claude Code Aesthetic */}
      <header className="border-b border-[#ebebeb] bg-white px-4 py-2.5 shrink-0 shadow-xs">
        <div className="flex items-center justify-between">
          {/* Left: "Назад к доске", Task ID, Stage badge, Model badge & Title */}
          <div className="flex items-center space-x-2.5 min-w-0">
            <button
              onClick={closeDrawer}
              className="flex items-center space-x-1.5 px-3 py-1.5 rounded-md bg-white hover:bg-[#f9f9f9] text-[#171717] border border-[#ebebeb] transition font-sans text-xs font-medium cursor-pointer shadow-xs active:scale-[0.98]"
              title="Назад к доске (Esc)"
            >
              <ArrowLeft className="h-4 w-4 text-[#171717]" />
              <span>Назад</span>
            </button>

            <span className="text-[#ebebeb] font-mono text-sm">/</span>

            <span className="px-2 py-0.5 rounded-md font-mono text-xs font-semibold bg-[#f4f4f5] border border-[#ebebeb] text-[#171717] tabular-nums">
              {task.id}
            </span>

            <span className="px-2 py-0.5 rounded-md font-mono text-xs uppercase bg-[#f4f4f5] border border-[#ebebeb] text-[#171717]">
              {column?.title || task.columnId}
            </span>

            {/* Model Badge */}
            <span
              className="flex items-center space-x-1 px-2 py-0.5 rounded-md font-mono text-xs text-[#4d4d4d] bg-[#fafafa] border border-[#ebebeb]"
              title={`Модель задачи: ${effectiveModel}`}
            >
              <Cpu className="h-3 w-3 text-[#666666]" />
              <span>[{effectiveModel}]</span>
            </span>

            <h2 className="text-sm font-semibold text-[#171717] truncate max-w-sm font-sans">
              {task.title}
            </h2>
          </div>

          {/* Right: Close button */}
          <div className="flex items-center space-x-2">
            <button
              onClick={closeDrawer}
              className="p-1.5 rounded-md text-[#666666] hover:text-[#171717] hover:bg-[#f4f4f5] transition cursor-pointer"
              title="Закрыть (Esc)"
            >
              <X className="h-5 w-5" />
            </button>
          </div>
        </div>

        {/* Sub-header: Path, Branch, Context gauge & Agent Controls */}
        <div className="mt-2 flex items-center justify-between text-xs font-mono text-[#666666] pt-2 border-t border-[#ebebeb]">
          <div className="flex items-center space-x-2.5">
            {task.worktreePath && (
              <div className="flex items-center space-x-1.5 px-2 py-0.5 rounded-md bg-[#f9f9f9] border border-[#ebebeb]">
                <FolderGit2 className="h-3.5 w-3.5 text-[#8f8f8f]" />
                <span className="text-[#4d4d4d]">{task.worktreePath}</span>
              </div>
            )}
            {task.branch && (
              <div className="flex items-center space-x-1.5 px-2 py-0.5 rounded-md bg-[#f9f9f9] border border-[#ebebeb]">
                <GitBranch className="h-3.5 w-3.5 text-[#8f8f8f]" />
                <span className="text-[#4d4d4d]">{task.branch}</span>
              </div>
            )}
          </div>

          {/* Right Controls: Status & Compact Gauge */}
          <div className="flex items-center space-x-3">
            {isRunning ? (
              <div className="flex items-center space-x-2">
                <span className="flex items-center space-x-1.5 px-2.5 py-1 rounded-md text-xs font-mono text-[#15803d] bg-[#f0fdf4] border border-[#bbf7d0]">
                  <span className="h-2 w-2 rounded-full bg-[#15803d] animate-pulse" />
                  <span>Агент выполняет задачу...</span>
                </span>
                <button
                  type="button"
                  onClick={handleAbortAgent}
                  className="flex items-center space-x-1 px-2.5 py-1 rounded-md text-xs font-mono text-[#b91c1c] bg-[#fef2f2] hover:bg-[#fee2e2] border border-[#fecaca] transition cursor-pointer"
                  title="Остановить выполнение (abort_agent)"
                >
                  <Square className="h-3 w-3 fill-current" />
                  <span>Остановить</span>
                </button>
              </div>
            ) : (
              <div className="flex items-center space-x-2">
                <span className="flex items-center space-x-1.5 px-2.5 py-1 rounded-md text-xs font-mono text-[#666666] bg-[#f4f4f5] border border-[#ebebeb]">
                  <span>Ожидание указаний</span>
                </span>
                <button
                  type="button"
                  onClick={handleRunAgent}
                  className="flex items-center space-x-1 px-3 py-1 rounded-md text-xs font-mono font-medium text-white bg-[#171717] hover:bg-[#262626] border border-[#171717] transition cursor-pointer shadow-xs"
                  title="Запустить агента (run_agent)"
                >
                  <Play className="h-3 w-3 fill-current" />
                  <span>Запустить агента</span>
                </button>
              </div>
            )}

            <ContextGauge taskId={task.id} />
          </div>
        </div>
      </header>

      {/* Warning Banner on agent run error */}
      {runError && (
        <div className="mx-4 md:mx-6 mt-3 px-3.5 py-2.5 rounded-lg bg-[#fef2f2] border border-[#fecaca] text-[#991b1b] flex items-center justify-between shadow-xs shrink-0 animate-in fade-in duration-150">
          <div className="flex items-center space-x-2.5 min-w-0 mr-3">
            <AlertTriangle className="h-4 w-4 text-[#dc2626] shrink-0" />
            <div className="text-xs font-medium font-sans truncate">
              <span className="font-semibold">Ошибка выполнения:</span> {runError}
            </div>
          </div>
          <div className="flex items-center space-x-2 shrink-0">
            <button
              type="button"
              onClick={handleRunAgent}
              className="flex items-center space-x-1.5 px-3 py-1 rounded-md bg-[#dc2626] hover:bg-[#b91c1c] text-white text-xs font-medium transition cursor-pointer shadow-xs active:scale-[0.98]"
            >
              <Play className="h-3 w-3 fill-current" />
              <span>Повторить запуск</span>
            </button>
            <button
              type="button"
              onClick={() => setRunError(null)}
              className="p-1 text-[#dc2626] hover:text-[#991b1b] rounded transition cursor-pointer"
              title="Закрыть уведомление"
            >
              <X className="h-3.5 w-3.5" />
            </button>
          </div>
        </div>
      )}

      {/* Main Unified Activity Feed Area */}
      <div ref={scrollRef} className="flex-1 overflow-y-auto p-4 md:p-6 bg-[#fafafa]">
        <div className="max-w-4xl mx-auto space-y-3.5">
          {/* a) Task Goal / User Objective Card */}
          <div className="rounded-lg border border-[#ebebeb] bg-white p-4 shadow-xs">
            <div className="flex items-center justify-between pb-2 mb-2.5 border-b border-[#ebebeb]">
              <div className="flex items-center space-x-2">
                <Target className="h-4 w-4 text-[#171717]" />
                <span className="text-xs font-semibold text-[#171717] font-sans">
                  Цель задачи / Prompt
                </span>
              </div>
              <span className="text-[10px] font-mono text-[#8f8f8f]">
                Постановка задачи
              </span>
            </div>
            <div className="text-xs text-[#171717] leading-relaxed whitespace-pre-wrap select-text font-sans">
              {task.description || task.title}
            </div>
          </div>

          {/* Unified Chronological Activity Stream */}
          {unifiedActivities.map((activity) => {
            // b) Live Streaming Thinking Block
            if (activity.type === 'thought') {
              return (
                <ThinkingCard
                  key={activity.id}
                  thought={activity}
                  isTaskRunning={isRunning}
                />
              );
            }

            // d) Inline Tool Execution Card
            if (activity.type === 'tool' && activity.tool) {
              return (
                <ToolExecutionCard
                  key={activity.id}
                  tool={activity.tool}
                />
              );
            }

            // User steer message
            if (activity.type === 'user' || activity.type === 'steer') {
              return (
                <div
                  key={activity.id}
                  className="rounded-lg border border-[#ebebeb] bg-[#f9f9f9] p-3.5 shadow-xs"
                >
                  <div className="flex items-center justify-between pb-1.5 mb-2 border-b border-[#ebebeb] text-[10px] font-mono text-[#666666]">
                    <div className="flex items-center space-x-1.5 text-[#171717] font-medium">
                      <User className="h-3 w-3" />
                      <span>Указание пользователя (Steer)</span>
                    </div>
                    <span className="tabular-nums">{activity.timestamp}</span>
                  </div>
                  <p className="text-xs text-[#171717] whitespace-pre-wrap select-text leading-relaxed font-sans">
                    {activity.content}
                  </p>
                </div>
              );
            }

            // System notices
            if (activity.type === 'system') {
              return (
                <div
                  key={activity.id}
                  className="flex items-center space-x-2 rounded-md border border-[#ebebeb] bg-[#f4f4f5] px-3 py-2 text-xs text-[#4d4d4d] font-mono shadow-xs"
                >
                  <Info className="h-3.5 w-3.5 text-[#666666] shrink-0" />
                  <span className="flex-1">{activity.content}</span>
                  <span className="text-[10px] text-[#8f8f8f] tabular-nums">
                    {activity.timestamp}
                  </span>
                </div>
              );
            }

            // Error notices
            if (activity.type === 'error') {
              return (
                <div
                  key={activity.id}
                  className="flex items-start space-x-2 rounded-md border border-[#fecaca] bg-[#fef2f2] p-3 text-xs text-[#991b1b] font-mono shadow-xs"
                >
                  <AlertTriangle className="h-4 w-4 text-[#dc2626] shrink-0 mt-0.5" />
                  <div className="flex-1 whitespace-pre-wrap select-text">
                    {activity.content}
                  </div>
                  <span className="text-[10px] text-[#b91c1c] tabular-nums shrink-0">
                    {activity.timestamp}
                  </span>
                </div>
              );
            }

            // c) Assistant Response / Output Markdown Block
            return (
              <div
                key={activity.id}
                className="rounded-lg border border-[#ebebeb] bg-white p-4 shadow-xs"
              >
                <div className="flex items-center justify-between pb-2 mb-2.5 border-b border-[#ebebeb] text-[10px] font-mono text-[#666666]">
                  <div className="flex items-center space-x-1.5 text-[#171717] font-medium">
                    <Sparkles className="h-3 w-3 text-[#171717]" />
                    <span>Ответ агента</span>
                  </div>
                  <span className="tabular-nums">{activity.timestamp}</span>
                </div>
                {activity.content ? (
                  <MarkdownContent content={activity.content} />
                ) : (
                  <div className="text-xs text-[#8f8f8f] font-mono">Генерация ответа...</div>
                )}
              </div>
            );
          })}

          {/* Empty state when no activity yet */}
          {unifiedActivities.length === 0 && (
            <div className="h-48 flex flex-col items-center justify-center rounded-lg border border-dashed border-[#ebebeb] bg-white text-center p-6">
              <Sparkles className="h-6 w-6 text-[#8f8f8f] mb-2" />
              <p className="text-xs font-medium text-[#171717] mb-1">
                {isRunning ? 'Агент приступил к работе...' : 'Ожидание действий агента'}
              </p>
              <p className="text-[11px] text-[#666666] max-w-sm mb-3">
                {isRunning
                  ? 'События, мысли и вызовы инструментов появятся в ленте в режиме реального времени.'
                  : 'Нажмите «Запустить агента» или отправьте команду ниже для начала решения задачи.'}
              </p>
              {!isRunning && (
                <button
                  type="button"
                  onClick={handleRunAgent}
                  className="flex items-center space-x-1.5 px-3 py-1.5 rounded-md bg-[#171717] hover:bg-[#262626] text-white text-xs font-medium transition cursor-pointer shadow-xs"
                >
                  <Play className="h-3.5 w-3.5 fill-current" />
                  <span>Запустить агента</span>
                </button>
              )}
            </div>
          )}
        </div>
      </div>

      {/* f) Interactive Input & Steer Bar */}
      <footer className="border-t border-[#ebebeb] bg-white p-3 shrink-0 shadow-sm">
        <div className="max-w-4xl mx-auto">
          <form onSubmit={handleSendPrompt} className="flex items-center space-x-2">
            <input
              id={promptInputId}
              name="interactivePromptInput"
              type="text"
              value={interactiveInput}
              onChange={(e) => setInteractiveInput(e.target.value)}
              placeholder={
                isRunning
                  ? 'Направить агента (steer_agent): скорректировать ход решения...'
                  : 'Запустить агента с указанием или задать уточняющий вопрос...'
              }
              className="flex-1 rounded-md bg-[#f9f9f9] border border-[#ebebeb] px-3.5 py-2 text-xs text-[#171717] placeholder:text-[#8f8f8f] focus:outline-hidden focus:border-[#171717] focus:bg-white font-sans transition"
            />
            <button
              type="submit"
              disabled={!interactiveInput.trim()}
              className="px-4 py-2 rounded-md bg-[#171717] hover:bg-[#262626] disabled:opacity-50 disabled:cursor-not-allowed text-white font-medium text-xs transition cursor-pointer flex items-center space-x-1.5 shadow-xs"
              title="Отправить команду агенту"
            >
              <span>{isRunning ? 'Направить' : 'Отправить'}</span>
              <Send className="h-3.5 w-3.5" />
            </button>
          </form>
        </div>
      </footer>
    </div>
  );
}
