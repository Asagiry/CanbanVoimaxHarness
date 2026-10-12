import { useState, useRef, useEffect, useId } from 'react';
import { useDrawerStore } from '../../stores/drawerStore';
import { useBoardStore } from '../../stores/boardStore';
import { useToolPolicyStore } from '../../stores/toolPolicyStore';
import ToolExecutionCard from './ToolExecutionCard';
import ContextGauge from './ContextGauge';
import {
  X,
  Play,
  Square,
  ArrowRightCircle,
  Terminal,
  Brain,
  FileCode2,
  Send,
  Trash2,
  Settings,
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
    clearTaskLogs,
  } = useDrawerStore();

  const { tasks, columns, updateTask, promoteTaskToNextStage } = useBoardStore();
  const { openModal: openSettingsModal } = useToolPolicyStore();
  const [interactiveInput, setInteractiveInput] = useState('');
  const [feedbackCue, setFeedbackCue] = useState<string | null>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const promptInputId = useId();

  const task = tasks.find((t) => t.id === activeTaskId);
  const column = columns.find((c) => c.id === task?.columnId);

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
    setTimeout(() => setFeedbackCue(null), 2000);
  };

  const handleStartStop = () => {
    const isRunning = task.status === 'running';
    const nextStatus = isRunning ? 'paused' : 'running';
    updateTask(task.id, {
      status: nextStatus,
      isTimerRunning: nextStatus === 'running',
    });
    showCue(isRunning ? 'Остановлен' : 'Запущен');
  };

  const handleClear = () => {
    clearTaskLogs(task.id);
    showCue('Очищено');
  };

  const handlePromote = () => {
    promoteTaskToNextStage(task.id);
    showCue('Продвинуто');
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

  const getStatusText = () => {
    switch (task.status) {
      case 'running':
        return <span className="text-emerald-400 font-mono text-xs">Выполняется</span>;
      case 'paused':
        return <span className="text-amber-400 font-mono text-xs">Приостановлен</span>;
      case 'waiting_review':
        return <span className="text-cyan-400 font-mono text-xs">На проверке</span>;
      case 'done':
        return <span className="text-slate-400 font-mono text-xs">Готово</span>;
      case 'blocked':
        return <span className="text-rose-400 font-mono text-xs">Заблокирован</span>;
      default:
        return <span className="text-slate-500 font-mono text-xs">В ожидании</span>;
    }
  };

  return (
    <div className="fixed inset-y-0 right-0 z-50 flex w-[640px] max-w-full flex-col bg-obsidian-900 border-l border-obsidian-800 shadow-xl">
      {/* Drawer Header */}
      <div className="flex flex-col border-b border-obsidian-800 bg-obsidian-850 px-4 py-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-2">
            <span className="font-mono text-xs font-semibold text-slate-300">
              {task.id}
            </span>
            <span className="text-slate-600 font-mono text-xs">/</span>
            <span className="text-xs text-slate-400 font-sans">
              {column?.title || task.columnId}
            </span>
            <span className="text-slate-600 font-mono text-xs">/</span>
            {getStatusText()}
          </div>

          <div className="flex items-center space-x-1">
            {feedbackCue && (
              <span className="text-xs font-mono text-slate-400 mr-2">
                {feedbackCue}
              </span>
            )}
            <button
              onClick={closeDrawer}
              className="p-1 rounded text-slate-400 hover:text-white hover:bg-obsidian-800 transition cursor-pointer"
              title="Закрыть"
            >
              <X className="h-4 w-4" />
            </button>
          </div>
        </div>

        <h3 className="mt-1.5 text-sm font-medium text-slate-100 line-clamp-2">
          {task.title}
        </h3>

        <div className="mt-1 text-[11px] font-mono text-slate-500">
          <span>{task.branch}</span>
        </div>

        <div className="mt-2 pt-2 border-t border-obsidian-800/80">
          <ContextGauge taskId={task.id} />
        </div>
      </div>

      {/* Action Controls Toolbar */}
      <div className="flex items-center justify-between border-b border-obsidian-800 bg-obsidian-850/60 px-4 py-1.5 text-xs font-mono">
        <div className="flex items-center space-x-2">
          <button
            onClick={handleStartStop}
            className={`flex items-center space-x-1 px-2.5 py-1 rounded text-xs font-medium transition cursor-pointer ${
              task.status === 'running'
                ? 'bg-obsidian-800 hover:bg-obsidian-750 text-amber-300 border border-obsidian-750'
                : 'bg-slate-200 hover:bg-white text-obsidian-950 font-semibold'
            }`}
            title={task.status === 'running' ? 'Остановить' : 'Запустить агента'}
          >
            {task.status === 'running' ? (
              <>
                <Square className="h-3 w-3" />
                <span>Остановить</span>
              </>
            ) : (
              <>
                <Play className="h-3 w-3" />
                <span>Запустить агента</span>
              </>
            )}
          </button>

          <button
            onClick={handleClear}
            className="flex items-center space-x-1 px-2 py-1 rounded bg-obsidian-950 hover:bg-obsidian-800 border border-obsidian-800 text-slate-400 hover:text-white transition cursor-pointer"
            title="Очистить"
          >
            <Trash2 className="h-3 w-3" />
            <span>Очистить</span>
          </button>

          <button
            onClick={openSettingsModal}
            className="flex items-center space-x-1 px-2 py-1 rounded bg-obsidian-950 hover:bg-obsidian-800 border border-obsidian-800 text-slate-400 hover:text-white transition cursor-pointer"
            title="Настройки"
          >
            <Settings className="h-3 w-3" />
            <span>Настройки</span>
          </button>
        </div>

        <button
          onClick={handlePromote}
          className="flex items-center space-x-1 px-2.5 py-1 rounded bg-obsidian-800 hover:bg-obsidian-750 border border-obsidian-700 text-slate-200 transition cursor-pointer"
          title="Продвинуть этап"
        >
          <span>Продвинуть этап</span>
          <ArrowRightCircle className="h-3 w-3" />
        </button>
      </div>

      {/* Tabs */}
      <div className="flex border-b border-obsidian-800 bg-obsidian-900 px-4 text-xs font-mono select-none">
        <button
          onClick={() => setActiveTab('thoughts')}
          className={`flex items-center space-x-1.5 py-2 px-3 border-b-2 transition cursor-pointer ${
            activeTab === 'thoughts'
              ? 'border-slate-300 text-slate-100 font-medium'
              : 'border-transparent text-slate-500 hover:text-slate-300'
          }`}
        >
          <Brain className="h-3.5 w-3.5" />
          <span>Поток рассуждений</span>
          <span className="text-[10px] tabular-nums text-slate-500">
            {currentStream.thoughts.length}
          </span>
        </button>

        <button
          onClick={() => setActiveTab('output')}
          className={`flex items-center space-x-1.5 py-2 px-3 border-b-2 transition cursor-pointer ${
            activeTab === 'output'
              ? 'border-slate-300 text-slate-100 font-medium'
              : 'border-transparent text-slate-500 hover:text-slate-300'
          }`}
        >
          <Terminal className="h-3.5 w-3.5" />
          <span>Терминал</span>
          <span className="text-[10px] tabular-nums text-slate-500">
            {currentStream.outputs.length}
          </span>
        </button>

        <button
          onClick={() => setActiveTab('tools')}
          className={`flex items-center space-x-1.5 py-2 px-3 border-b-2 transition cursor-pointer ${
            activeTab === 'tools'
              ? 'border-slate-300 text-slate-100 font-medium'
              : 'border-transparent text-slate-500 hover:text-slate-300'
          }`}
        >
          <FileCode2 className="h-3.5 w-3.5" />
          <span>Измененные файлы</span>
          <span className="text-[10px] tabular-nums text-slate-500">
            {currentTools.length}
          </span>
        </button>
      </div>

      {/* Tab Stream Area */}
      <div ref={scrollRef} className="flex-1 overflow-y-auto p-4 space-y-2 bg-obsidian-950 font-mono text-xs">
        {/* Tab 1: Поток рассуждений */}
        {activeTab === 'thoughts' && (
          <div className="space-y-2">
            {currentStream.thoughts.map((entry) => (
              <div
                key={entry.id}
                className="p-2.5 rounded border border-obsidian-800 bg-obsidian-900/60 text-slate-300 leading-relaxed"
              >
                <div className="flex items-center justify-between pb-1 mb-1 border-b border-obsidian-850 text-[10px] text-slate-500">
                  <span>Рассуждения</span>
                  <span className="tabular-nums">{entry.timestamp}</span>
                </div>
                <p className="whitespace-pre-wrap select-text text-slate-300 text-xs leading-relaxed font-mono">
                  {entry.content}
                </p>
              </div>
            ))}

            {currentStream.thoughts.length === 0 && (
              <div className="h-32 flex items-center justify-center text-slate-600 text-xs">
                Записи рассуждений отсутствуют
              </div>
            )}
          </div>
        )}

        {/* Tab 2: Терминал */}
        {activeTab === 'output' && (
          <div className="space-y-2">
            {currentStream.outputs.map((entry) => {
              const typeLabel =
                entry.type === 'error'
                  ? 'Ошибка'
                  : entry.type === 'system'
                  ? 'Система'
                  : entry.type === 'steer'
                  ? 'Команда'
                  : 'Вывод';

              return (
                <div
                  key={entry.id}
                  className={`p-2.5 rounded border leading-relaxed ${
                    entry.type === 'error'
                      ? 'border-rose-900/50 bg-rose-950/20 text-rose-300'
                      : entry.type === 'system'
                      ? 'border-obsidian-800 bg-obsidian-900/40 text-slate-400'
                      : 'border-obsidian-800 bg-obsidian-900 text-slate-200'
                  }`}
                >
                  <div className="flex items-center justify-between pb-1 mb-1 border-b border-obsidian-850 text-[10px] text-slate-500 font-mono">
                    <span>{typeLabel}</span>
                    <span className="tabular-nums">{entry.timestamp}</span>
                  </div>
                  <p className="whitespace-pre-wrap select-text text-xs leading-relaxed">
                    {entry.content}
                  </p>
                </div>
              );
            })}

            {currentStream.outputs.length === 0 && (
              <div className="h-32 flex items-center justify-center text-slate-600 text-xs">
                Журнал вывода пуст
              </div>
            )}
          </div>
        )}

        {/* Tab 3: Измененные файлы */}
        {activeTab === 'tools' && (
          <div className="space-y-2">
            {currentTools.map((tool) => (
              <ToolExecutionCard key={tool.id} tool={tool} />
            ))}

            {currentTools.length === 0 && (
              <div className="h-32 flex items-center justify-center text-slate-600 text-xs">
                Нет зарегистрированных изменений файлов
              </div>
            )}
          </div>
        )}
      </div>

      {/* Input */}
      <div className="border-t border-obsidian-800 bg-obsidian-900 p-3">
        <form onSubmit={handleSendPrompt} className="flex items-center space-x-2">
          <input
            id={promptInputId}
            name="interactivePromptInput"
            type="text"
            value={interactiveInput}
            onChange={(e) => setInteractiveInput(e.target.value)}
            placeholder="Ввод команд агенту..."
            title="Ввод команд агенту"
            className="flex-1 rounded bg-obsidian-950 border border-obsidian-800 px-3 py-1.5 text-xs text-slate-100 placeholder-slate-600 focus:outline-hidden focus:border-slate-500 font-mono"
          />
          <button
            type="submit"
            className="px-3 py-1.5 rounded bg-slate-200 hover:bg-white text-obsidian-950 font-medium transition cursor-pointer text-xs"
            title="Отправить"
          >
            <Send className="h-3.5 w-3.5" />
          </button>
        </form>
      </div>
    </div>
  );
}
