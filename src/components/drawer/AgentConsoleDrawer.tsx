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
    <div className="fixed inset-0 z-50 flex flex-col bg-white text-[#171717] select-none animate-in fade-in duration-150">
      {/* Fullscreen Header: Clean Vercel / ChatGPT Aesthetic */}
      <header className="border-b border-[#ebebeb] bg-white px-4 py-2.5">
        <div className="flex items-center justify-between">
          {/* Left: "Назад к доске", Task ID & Title, Status badge */}
          <div className="flex items-center space-x-3 min-w-0">
            <button
              onClick={closeDrawer}
              className="flex items-center space-x-1.5 px-3 py-1.5 rounded-md bg-white hover:bg-[#f9f9f9] text-[#171717] border border-[#ebebeb] transition font-sans text-xs font-medium cursor-pointer shadow-xs active:scale-[0.98]"
              title="Назад к доске (Esc)"
            >
              <ArrowLeft className="h-4 w-4 text-[#171717]" />
              <span>Назад к доске</span>
            </button>

            <span className="text-[#ebebeb] font-mono text-sm">/</span>

            <span className="px-2 py-0.5 rounded-md font-mono text-xs font-semibold bg-[#f4f4f5] border border-[#ebebeb] text-[#171717] tabular-nums">
              {task.id}
            </span>

            <span
              className="px-2 py-0.5 rounded-md font-mono text-xs uppercase bg-[#f4f4f5] border border-[#ebebeb] text-[#171717]"
            >
              {column?.title || task.columnId}
            </span>

            <h2 className="text-sm font-semibold text-[#171717] truncate max-w-md font-sans">
              {task.title}
            </h2>
          </div>

          {/* Right: Only Close button */}
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

        {/* Sub-header: Path, Branch, and Context gauge */}
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

          <div className="w-72">
            <ContextGauge taskId={task.id} />
          </div>
        </div>
      </header>

      {/* Stream Tabs Bar */}
      <div className="flex border-b border-[#ebebeb] bg-white px-4 text-xs font-mono select-none">
        <button
          onClick={() => setActiveTab('thoughts')}
          className={`flex items-center space-x-2 py-2.5 px-3 border-b-2 transition cursor-pointer ${
            activeTab === 'thoughts'
              ? 'border-[#171717] text-[#171717] font-semibold'
              : 'border-transparent text-[#666666] hover:text-[#171717]'
          }`}
        >
          <Brain className="h-4 w-4" />
          <span>&lt;thinking&gt;</span>
          <span className="text-[10px] tabular-nums px-1.5 py-0.2 rounded-md bg-[#f4f4f5] text-[#171717] border border-[#ebebeb]">
            {currentStream.thoughts.length}
          </span>
        </button>

        <button
          onClick={() => setActiveTab('output')}
          className={`flex items-center space-x-2 py-2.5 px-3 border-b-2 transition cursor-pointer ${
            activeTab === 'output'
              ? 'border-[#171717] text-[#171717] font-semibold'
              : 'border-transparent text-[#666666] hover:text-[#171717]'
          }`}
        >
          <MessageSquare className="h-4 w-4" />
          <span>Вывод / Output</span>
          <span className="text-[10px] tabular-nums px-1.5 py-0.2 rounded-md bg-[#f4f4f5] text-[#171717] border border-[#ebebeb]">
            {currentStream.outputs.length}
          </span>
        </button>

        <button
          onClick={() => setActiveTab('tools')}
          className={`flex items-center space-x-2 py-2.5 px-3 border-b-2 transition cursor-pointer ${
            activeTab === 'tools'
              ? 'border-[#171717] text-[#171717] font-semibold'
              : 'border-transparent text-[#666666] hover:text-[#171717]'
          }`}
        >
          <Terminal className="h-4 w-4" />
          <span>Инструменты</span>
          <span className="text-[10px] tabular-nums px-1.5 py-0.2 rounded-md bg-[#f4f4f5] text-[#171717] border border-[#ebebeb]">
            {currentTools.length}
          </span>
        </button>
      </div>

      {/* Main Fullscreen Stream Area */}
      <div ref={scrollRef} className="flex-1 overflow-y-auto p-4 md:p-6 bg-[#fafafa] font-mono text-xs">
        <div className="max-w-5xl mx-auto space-y-3">
          {/* Tab 1: Clean Thoughts Section */}
          {activeTab === 'thoughts' && (
            <div className="space-y-3">
              {currentStream.thoughts.map((entry) => (
                <div
                  key={entry.id}
                  className="rounded-md border border-[#ebebeb] bg-[#f9f9f9] border-l-2 border-l-[#171717] p-3.5 text-[#171717] leading-relaxed shadow-xs"
                >
                  <div className="flex items-center justify-between pb-1.5 mb-2 border-b border-[#ebebeb] text-[10px] font-mono">
                    <span className="font-semibold text-[#171717]">
                      &lt;thinking&gt;
                    </span>
                    <span className="tabular-nums text-[#666666] font-mono">
                      {entry.timestamp}
                    </span>
                  </div>
                  <p className="whitespace-pre-wrap select-text text-[#171717] text-xs leading-relaxed font-mono">
                    {entry.content}
                  </p>
                </div>
              ))}

              {currentStream.thoughts.length === 0 && (
                <div className="h-48 flex flex-col items-center justify-center text-[#8f8f8f] font-mono text-xs">
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
                  className={`rounded-md border p-3.5 leading-relaxed shadow-xs ${
                    entry.type === 'error'
                      ? 'border-[#fecaca] bg-[#fef2f2] text-[#991b1b]'
                      : entry.type === 'system'
                      ? 'border-[#ebebeb] bg-[#f9f9f9] text-[#171717]'
                      : 'border-[#ebebeb] bg-white text-[#171717]'
                  }`}
                >
                  <div className="flex items-center justify-between pb-1.5 mb-2 border-b border-[#ebebeb] font-mono text-[10px] text-[#666666]">
                    <span className="font-semibold uppercase tracking-wider">{entry.type}</span>
                    <span className="tabular-nums">{entry.timestamp}</span>
                  </div>
                  <p className="whitespace-pre-wrap select-text text-xs leading-relaxed text-[#171717]">
                    {entry.content}
                  </p>
                </div>
              ))}

              {currentStream.outputs.length === 0 && (
                <div className="h-48 flex flex-col items-center justify-center font-mono text-[#8f8f8f] text-xs">
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
                <div className="h-48 flex flex-col items-center justify-center text-[#8f8f8f] font-mono text-xs">
                  <span>Инструменты еще не вызывались</span>
                </div>
              )}
            </div>
          )}
        </div>
      </div>

      {/* Clean Interactive Input Bar */}
      <footer className="border-t border-[#ebebeb] bg-white p-3">
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
              className="flex-1 rounded-md bg-[#f9f9f9] border border-[#ebebeb] px-3 py-2 text-xs text-[#171717] placeholder:text-[#8f8f8f] focus:outline-hidden focus:border-[#171717] focus:bg-white font-mono transition"
            />
            <button
              type="submit"
              className="px-4 py-2 rounded-md bg-[#171717] hover:bg-[#262626] text-white font-medium text-xs transition cursor-pointer flex items-center space-x-1.5 shadow-xs"
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
