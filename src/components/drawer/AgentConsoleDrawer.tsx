import { useState, useRef, useEffect, useMemo } from 'react';
import { useDrawerStore, DrawerActivityItem } from '../../stores/drawerStore';
import { useBoardStore, syncTasksBackend } from '../../stores/boardStore';
import { useSettingsStore } from '../../stores/settingsStore';
import ToolExecutionCard from './ToolExecutionCard';
import ContextGauge from './ContextGauge';
import GitDiffSidebar from './GitDiffSidebar';
import ErrorBoundary from '../common/ErrorBoundary';
import {
  ArrowDown,
  ArrowLeft,
  ArrowUp,
  Plus,
  X,
  GitBranch,
  FolderGit2,
  Brain,
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
  GitCompare,
} from 'lucide-react';

// Lightweight Markdown Renderer for streaming assistant output
function MarkdownContent({ content }: { content?: string }) {
  const elements = useMemo(() => {
    const lines = (content ?? '').split('\n');
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
              className="px-1.5 py-0.5 rounded bg-[#f4f4f5] border border-[#ebebeb] text-[#171717] font-mono text-[13px]"
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

    const splitTableRow = (row: string): string[] => {
      let t = row.trim();
      if (t.startsWith('|')) t = t.slice(1);
      if (t.endsWith('|')) t = t.slice(0, -1);
      return t.split('|').map((c) => c.trim());
    };

    const parseDelimiterRow = (row: string): Array<'left' | 'center' | 'right'> | null => {
      const cells = splitTableRow(row);
      if (cells.length === 0) return null;
      const aligns: Array<'left' | 'center' | 'right'> = [];
      for (const c of cells) {
        if (!/^:?-+:?$/.test(c)) return null;
        if (c.startsWith(':') && c.endsWith(':') && c.length >= 3) aligns.push('center');
        else if (c.endsWith(':')) aligns.push('right');
        else aligns.push('left');
      }
      return aligns;
    };

    const isHrLine = (row: string): boolean => {
      const nospace = row.trim().replace(/\s+/g, '');
      return /^(-{3,}|\*{3,}|_{3,})$/.test(nospace);
    };

    for (let index = 0; index < lines.length; index++) {
      const line = lines[index];
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
              className="my-2.5 rounded-md border border-[#ebebeb] bg-[#fafafa] overflow-hidden font-mono text-[13px]"
            >
              {codeLanguage && (
                <div className="px-3 py-1 bg-[#f4f4f5] border-b border-[#ebebeb] text-xs text-[#666666] font-medium flex justify-between">
                  <span>{codeLanguage}</span>
                </div>
              )}
              <pre className="p-3 text-[#171717] overflow-x-auto leading-relaxed select-text font-mono text-[13px]">
                {codeBuffer.join('\n')}
              </pre>
            </div>
          );
        }
        continue;
      }

      if (inCodeBlock) {
        codeBuffer.push(line);
        continue;
      }

      // GFM table: header with `|` + delimiter row, then data rows with `|`
      if (line.includes('|') && index + 1 < lines.length) {
        const aligns = parseDelimiterRow(lines[index + 1]);
        if (aligns) {
          const headerCells = splitTableRow(line);
          const colCount = Math.max(headerCells.length, aligns.length);
          const normAligns: Array<'left' | 'center' | 'right'> = Array.from(
            { length: colCount },
            (_, k) => aligns[k] ?? 'left'
          );
          const normHeader: string[] = Array.from(
            { length: colCount },
            (_, k) => headerCells[k] ?? ''
          );
          const bodyRows: string[][] = [];
          let j = index + 2;
          while (j < lines.length && lines[j].includes('|') && lines[j].trim() !== '' && !lines[j].startsWith('```')) {
            const cells = splitTableRow(lines[j]);
            bodyRows.push(Array.from({ length: colCount }, (_, k) => cells[k] ?? ''));
            j++;
          }
          result.push(
            <div
              key={`table-${index}`}
              className="my-2 overflow-x-auto rounded-md border border-[#ebebeb]"
            >
              <table className="w-full border-collapse text-[13px] font-mono tabular-nums">
                <thead>
                  <tr>
                    {normHeader.map((cell, k) => (
                      <th
                        key={k}
                        style={{ textAlign: normAligns[k] }}
                        className="border border-[#ebebeb] bg-[#f9f9f9] px-2 py-1.5 font-semibold text-[#171717]"
                      >
                        {formatInline(cell)}
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {bodyRows.map((row, r) => (
                    <tr key={r}>
                      {row.map((cell, k) => (
                        <td
                          key={k}
                          style={{ textAlign: normAligns[k] }}
                          className="border border-[#ebebeb] px-2 py-1.5 text-[#171717]"
                        >
                          {formatInline(cell)}
                        </td>
                      ))}
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          );
          index = j - 1;
          continue;
        }
      }

      // Thematic break outside tables
      if (isHrLine(line)) {
        result.push(<hr key={index} className="border-[#ebebeb] my-2.5" />);
        continue;
      }

      // Image line: ![alt](url)
      const imgMatch = line.trim().match(/^!\[(.*?)\]\((.*?)\)$/);
      if (imgMatch) {
        result.push(
          <div key={`img-${index}`} className="my-2 rounded-lg overflow-hidden border border-[#ebebeb] max-w-lg bg-[#f4f4f5] shadow-xs">
            <img
              src={imgMatch[2]}
              alt={imgMatch[1] || 'Изображение'}
              className="w-full max-h-80 object-contain cursor-pointer"
              onClick={() => window.open(imgMatch[2], '_blank')}
            />
          </div>
        );
        continue;
      }

      // Headings
      if (line.startsWith('#### ')) {
        result.push(
          <h5 key={index} className="text-[13px] md:text-sm font-semibold text-[#171717] mt-2 mb-1">
            {formatInline(line.slice(5))}
          </h5>
        );
      } else if (line.startsWith('### ')) {
        result.push(
          <h4 key={index} className="text-sm font-semibold text-[#171717] mt-2.5 mb-1">
            {formatInline(line.slice(4))}
          </h4>
        );
      } else if (line.startsWith('## ')) {
        result.push(
          <h3 key={index} className="text-sm md:text-base font-semibold text-[#171717] mt-3 mb-1">
            {formatInline(line.slice(3))}
          </h3>
        );
      } else if (line.startsWith('# ')) {
        result.push(
          <h2 key={index} className="text-base md:text-lg font-bold text-[#171717] mt-3.5 mb-1.5">
            {formatInline(line.slice(2))}
          </h2>
        );
      } else if (line.startsWith('- ') || line.startsWith('* ')) {
        result.push(
          <li key={index} className="ml-4 list-disc text-sm leading-relaxed text-[#171717]">
            {formatInline(line.slice(2))}
          </li>
        );
      } else if (line.trim() === '') {
        result.push(<div key={index} className="h-1.5" />);
      } else {
        result.push(
          <p key={index} className="text-sm leading-relaxed text-[#171717]">
            {formatInline(line)}
          </p>
        );
      }
    }

    if (inCodeBlock && codeBuffer.length > 0) {
      result.push(
        <div
          key="code-unfinished"
          className="my-2.5 rounded-md border border-[#ebebeb] bg-[#fafafa] overflow-hidden font-mono text-[13px]"
        >
          <pre className="p-3 text-[#171717] overflow-x-auto leading-relaxed select-text font-mono text-[13px]">
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
  taskId,
}: {
  thought: DrawerActivityItem;
  isTaskRunning: boolean;
  taskId: string;
}) {
  const [isExpanded, setIsExpanded] = useState(true);
  const thinkingStart = useDrawerStore((s) => s.thinkingStartTime[taskId] ?? null);

  // Живой таймер без setInterval: обновляется на ре-рендерах от чанков.
  // Пока задача running — от thinkingStartTime, после финиша — финал из thought.durationMs.
  let timerText: string;
  if (isTaskRunning && thinkingStart != null) {
    timerText = `${((Date.now() - thinkingStart) / 1000).toFixed(1)}с`;
  } else if (thought?.durationMs) {
    timerText = `${Math.round(thought.durationMs / 1000)}с`;
  } else {
    timerText = '0с';
  }

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
        </div>

        <div className="flex items-center space-x-2 text-[10px] font-mono text-[#666666]">
          <span className="flex items-center space-x-1 tabular-nums">
            <Clock className="h-3 w-3" />
            <span>{timerText}</span>
          </span>
          <span className="tabular-nums">{thought?.timestamp ?? ''}</span>
        </div>
      </div>

      {isExpanded && (
        <div className="p-3 border-t border-[#ebebeb] bg-[#ffffff] font-mono text-[13px] leading-relaxed text-[#333333] whitespace-pre-wrap select-text max-h-96 overflow-y-auto">
          {thought?.content ?? ''}
        </div>
      )}
    </div>
  );
}

// Clean prompt helper: strips legacy appended [USER STEER] or [USER]: blocks for pure prompt display
function getCleanPrompt(description?: string, title?: string): string {
  const raw = (description || title || '').trim();
  const steerIdx = raw.search(/\n\s*\[USER(?:\s+STEER)?\]:/i);
  if (steerIdx !== -1) {
    return raw.slice(0, steerIdx).trim();
  }
  return raw;
}

export default function AgentConsoleDrawer() {
  const {
    isOpen,
    activeTaskId,
    activityFeed,
    streamEntries,
    toolExecutions,
    openDrawer,
    closeDrawer,
    appendOutput,
  } = useDrawerStore();

  const { tasks, columns, updateTask } = useBoardStore();
  const { settings } = useSettingsStore();

  const [interactiveInput, setInteractiveInput] = useState('');
  const [runError, setRunError] = useState<string | null>(null);
  // 1. Свёрнутость блоков «Ответ агента»: по умолчанию все свёрнуты (Set раскрытых пуст).
  const [expandedOutputs, setExpandedOutputs] = useState<Set<string>>(() => new Set());
  // 2. Умный автоскролл: кнопка «К новому» видна, только если пришли события вне низа.
  const [showNewButton, setShowNewButton] = useState(false);
  // 3. Состояние панели Git Diff (по умолчанию развёрнута)
  const [isDiffCollapsed, setIsDiffCollapsed] = useState(false);
  const isFirstBatchRef = useRef(true);
  const nearBottomRef = useRef(true);
  const scrollRef = useRef<HTMLDivElement>(null);

  // Modern chat attachments, toolbar states & textarea ref
  const [attachedImages, setAttachedImages] = useState<Array<{ id: string; name: string; dataUrl: string }>>([]);
  const [isDragOverChat, setIsDragOverChat] = useState(false);
  const [isToolMenuOpen, setIsToolMenuOpen] = useState(false);
  const [isModelMenuOpen, setIsModelMenuOpen] = useState(false);
  const [toolAccessMode, setToolAccessMode] = useState<'full' | 'readonly'>('full');
  const chatFileInputRef = useRef<HTMLInputElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const toggleOutput = (id: string) => {
    setExpandedOutputs((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  };

  const scrollToBottom = () => {
    const el = scrollRef.current;
    if (el) {
      el.scrollTop = el.scrollHeight;
    }
    nearBottomRef.current = true;
    setShowNewButton(false);
  };

  const handleFeedScroll = () => {
    const el = scrollRef.current;
    if (!el) return;
    const nearBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 120;
    nearBottomRef.current = nearBottom;
    if (nearBottom) {
      setShowNewButton(false);
    }
  };

  const task = tasks.find((t) => t.id === activeTaskId);
  const column = columns.find((c) => c.id === task?.columnId);

  // Reset error when switching tasks & load dialog history from disk
  useEffect(() => {
    setRunError(null);
    // Новые задачи открываются со свёрнутыми ответами и сброшенным скроллом
    setExpandedOutputs(new Set());
    setShowNewButton(false);
    isFirstBatchRef.current = true;
    nearBottomRef.current = true;
    if (activeTaskId) {
      useDrawerStore.getState().loadTaskDialog(activeTaskId);
    }
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
            const st = event.payload.status;
            // Заморозка TPS на терминальных статусах (бэкенд может не прислать agent:finished)
            if (st === 'finished' || st === 'aborted' || st === 'error') {
              try {
                const ds = useDrawerStore.getState();
                if (ds.finalTps[task.id] == null) {
                  const tele = ds.taskTelemetry[task.id];
                  const start = ds.thinkingStartTime[task.id];
                  if (tele && start != null) {
                    const elapsedSec = (Date.now() - start) / 1000;
                    const outTokens = tele.outputTokens ?? 0;
                    if (elapsedSec > 0 && outTokens > 0) {
                      ds.setFinalTps(task.id, outTokens / elapsedSec);
                    }
                  }
                }
              } catch {
                // ignore
              }
            }
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
      ...(rawStream.thoughts ?? []).map((th) => ({
        id: th?.id ?? `th-${Date.now()}`,
        type: 'thought' as const,
        timestamp: th?.timestamp ?? '',
        content: th?.content ?? '',
        durationMs: th?.durationMs,
      })),
      ...(rawStream.outputs ?? []).map((out) => ({
        id: out?.id ?? `out-${Date.now()}`,
        type: (out?.type === 'steer' || out?.type === 'system' || out?.type === 'error' || out?.type === 'user'
          ? out.type
          : 'output') as any,
        timestamp: out?.timestamp ?? '',
        content: out?.content ?? '',
      })),
      ...(rawTools ?? []).map((t) => ({
        id: t?.id ?? `tool-${Date.now()}`,
        type: 'tool' as const,
        timestamp: t?.timestamp ?? '',
        tool: t,
      })),
    ];

    return merged;
  }, [task, activityFeed, streamEntries, toolExecutions]);

  // Умный автоскролл: вниз только если пользователь уже внизу или это первое событие.
  // Позиция до вставки нового контента отслеживается в nearBottomRef через onScroll,
  // поэтому рост scrollHeight на длинных чанках не даёт ложного «отошёл вверх».
  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;
    if (isFirstBatchRef.current) {
      isFirstBatchRef.current = false;
      el.scrollTop = el.scrollHeight;
      nearBottomRef.current = true;
      setShowNewButton(false);
      return;
    }
    if (nearBottomRef.current) {
      el.scrollTop = el.scrollHeight;
      setShowNewButton(false);
    } else {
      setShowNewButton(true);
    }
  }, [unifiedActivities]);

  if (!isOpen || !task) return null;

  const isRunning = task.status === 'running';
  const effectiveModel = task.model || settings.model || 'gemini-3.8-flash-high';

  const { availableModels } = useSettingsStore();
  const selectableModels = useMemo(() => {
    return Array.from(
      new Set([
        effectiveModel,
        ...(availableModels || []),
        'gemini-3.8-flash-high',
        'gemini-2.5-pro',
        'claude-3-7-sonnet-20250219',
        'gpt-4o',
      ])
    );
  }, [effectiveModel, availableModels]);

  const turns = useMemo(() => {
    const feed = activityFeed[task.id] || [];
    const feedCount = (feed ?? []).filter((a) => a?.type === 'tool' || a?.type === 'thought' || a?.type === 'output').length;
    const toolsCount = toolExecutions[task.id]?.length || 0;
    const outputsCount = streamEntries[task.id]?.outputs?.filter((o) => o?.type === 'output').length || 0;
    return Math.max(feedCount, toolsCount + outputsCount, 0);
  }, [task.id, activityFeed, toolExecutions, streamEntries]);

  const adjustTextareaHeight = (el: HTMLTextAreaElement) => {
    el.style.height = 'auto';
    const newHeight = Math.min(Math.max(el.scrollHeight, 40), 180);
    el.style.height = `${newHeight}px`;
  };

  const handleInputChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setInteractiveInput(e.target.value);
    adjustTextareaHeight(e.target);
  };

  const handleAddChatImages = (files: FileList | File[]) => {
    const validFiles = Array.from(files).filter(
      (f) => f.type.startsWith('image/') || /\.(png|jpe?g|webp|gif)$/i.test(f.name)
    );
    if (validFiles.length === 0) return;

    validFiles.forEach((file) => {
      const reader = new FileReader();
      reader.onload = () => {
        const dataUrl = reader.result as string;
        setAttachedImages((prev) => [
          ...prev,
          {
            id: `${Date.now()}-${Math.random().toString(36).substring(2, 7)}`,
            name: file.name,
            dataUrl,
          },
        ]);
      };
      reader.readAsDataURL(file);
    });
  };

  const handleChatFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.files) {
      handleAddChatImages(e.target.files);
    }
    e.target.value = '';
  };

  const handleChatDragOver = (e: React.DragEvent) => {
    if (e.dataTransfer.types.includes('Files')) {
      e.preventDefault();
      e.stopPropagation();
      setIsDragOverChat(true);
    }
  };

  const handleChatDragLeave = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragOverChat(false);
  };

  const handleChatDrop = (e: React.DragEvent) => {
    if (e.dataTransfer.files && e.dataTransfer.files.length > 0) {
      e.preventDefault();
      e.stopPropagation();
      setIsDragOverChat(false);
      handleAddChatImages(e.dataTransfer.files);
    }
  };

  const handleChatPaste = (e: React.ClipboardEvent<HTMLTextAreaElement>) => {
    const items = e.clipboardData?.items;
    if (!items) return;

    const pastedFiles: File[] = [];
    for (let i = 0; i < items.length; i++) {
      const item = items[i];
      if (item.type.startsWith('image/')) {
        const file = item.getAsFile();
        if (file) {
          pastedFiles.push(file);
        }
      }
    }

    if (pastedFiles.length > 0) {
      e.preventDefault();
      handleAddChatImages(pastedFiles);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSendPrompt();
    }
  };

  const handleRunAgent = async () => {
    setRunError(null);
    updateTask(task.id, { status: 'running', isTimerRunning: true });
    try {
      await syncTasksBackend(useBoardStore.getState().tasks);
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('run_agent', { taskId: task.id, prompt: task.description || null });
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

  const handleSendPrompt = async (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    const trimmed = interactiveInput.trim();
    if (!trimmed && attachedImages.length === 0) return;

    const currentImages = [...attachedImages];
    const imageUrls = currentImages.map((img) => img.dataUrl);

    setInteractiveInput('');
    setAttachedImages([]);
    if (textareaRef.current) {
      textareaRef.current.style.height = 'auto';
    }
    setRunError(null);

    const promptText = trimmed || (currentImages.length > 0 ? 'Изображение прикреплено' : '');

    // Append user steer message into feed immediately with attached images
    appendOutput(task.id, promptText, 'user', imageUrls.length > 0 ? imageUrls : undefined);
    useDrawerStore.getState().flushSaveDialog(task.id);

    const imageNotice = currentImages.length > 0
      ? `\n[Прикреплено ${currentImages.length} изображений: ${currentImages.map((i) => i.name).join(', ')}]`
      : '';
    const fullMessage = `${promptText}${imageNotice}`;

    // Do NOT pollute task description with user steer messages.
    // If task has no description yet, set initial prompt. Otherwise, keep original description untouched.
    if (!task.description) {
      updateTask(task.id, { description: fullMessage, status: 'running', isTimerRunning: true });
    } else {
      updateTask(task.id, { status: 'running', isTimerRunning: true });
    }

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await syncTasksBackend(useBoardStore.getState().tasks);
      if (isRunning) {
        await invoke('steer_agent', { taskId: task.id, message: fullMessage });
      } else {
        await invoke('run_agent', { taskId: task.id, prompt: fullMessage });
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
              {task.id ?? ''}
            </span>

            <span className="px-2 py-0.5 rounded-md font-mono text-xs uppercase bg-[#f4f4f5] border border-[#ebebeb] text-[#171717]">
              {column?.title || task.columnId || ''}
            </span>

            {/* Parent Task Badge & Navigation */}
            {task.parentId && (
              <button
                type="button"
                onClick={() => openDrawer(task.parentId!)}
                className="flex items-center space-x-1 px-2 py-0.5 rounded-md font-mono text-xs text-[#6366f1] bg-[#eef2ff] border border-[#c7d2fe] hover:bg-[#e0e7ff] transition cursor-pointer"
                title={`Перейти к родительской задаче ${task.parentId}`}
              >
                <GitBranch className="h-3 w-3 text-[#6366f1]" />
                <span>Родитель: {task.parentId}</span>
              </button>
            )}

            {/* Waiting for Subtask Badge & Navigation */}
            {task.waitingForSubtaskId && (
              <button
                type="button"
                onClick={() => openDrawer(task.waitingForSubtaskId!)}
                className="flex items-center space-x-1 px-2 py-0.5 rounded-md font-mono text-xs text-[#b45309] bg-[#fffbeb] border border-[#fde68a] hover:bg-[#fef3c7] transition cursor-pointer"
                title={`Открыть активную подзадачу ${task.waitingForSubtaskId}`}
              >
                <span className="h-1.5 w-1.5 rounded-full bg-[#f59e0b] animate-pulse" />
                <span>Ждем: {task.waitingForSubtaskId}</span>
              </button>
            )}

            {/* Model Badge */}
            <span
              className="flex items-center space-x-1 px-2 py-0.5 rounded-md font-mono text-xs text-[#4d4d4d] bg-[#fafafa] border border-[#ebebeb]"
              title={`Модель задачи: ${effectiveModel}`}
            >
              <Cpu className="h-3 w-3 text-[#666666]" />
              <span>[{effectiveModel}]</span>
            </span>

            <h2 className="text-sm font-semibold text-[#171717] truncate max-w-sm font-sans">
              {task.title ?? ''}
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

          {/* Right Controls: Status */}
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
              </div>
            )}
            {/* Git Diff Toggle Button */}
            <button
              type="button"
              onClick={() => setIsDiffCollapsed((prev) => !prev)}
              className={`flex items-center space-x-1.5 px-2.5 py-1 rounded-md text-xs font-mono border transition cursor-pointer ${
                !isDiffCollapsed
                  ? 'bg-[#171717] text-white border-[#171717] shadow-xs'
                  : 'bg-[#f4f4f5] text-[#171717] border-[#ebebeb] hover:bg-[#e4e4e7]'
              }`}
              title={isDiffCollapsed ? 'Развернуть Git Diff панель' : 'Скрыть Git Diff панель'}
            >
              <GitCompare className="h-3.5 w-3.5" />
              <span>Git Diff</span>
            </button>
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

      {/* Two-Column Layout: Left (Feed + Steer Bar) | Right (GitDiffSidebar) */}
      <div className="flex-1 flex flex-row min-h-0 min-w-0 overflow-hidden">
        {/* Left Column: Activity Feed + Steer Bar */}
        <div className="flex-1 flex flex-col min-h-0 min-w-0 bg-[#fafafa]">
        <ErrorBoundary scopeLabel="чат задачи" onClose={closeDrawer}>
          {/* Main Unified Activity Feed Area */}
          <div className="relative flex-1 flex flex-col min-h-0 bg-[#fafafa]">
        <div ref={scrollRef} onScroll={handleFeedScroll} className="flex-1 overflow-y-auto p-4 md:p-6">
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
            <div className="text-sm text-[#171717] leading-relaxed whitespace-pre-wrap select-text font-sans">
              {getCleanPrompt(task.description, task.title)}
            </div>
          </div>

          {/* Unified Chronological Activity Stream */}
          {(unifiedActivities ?? []).map((activity, idx) => {
            const key = activity?.id ?? `activity-${idx}`;
            // b) Live Streaming Thinking Block
            if (activity?.type === 'thought') {
              return (
                <ThinkingCard
                  key={key}
                  thought={activity}
                  isTaskRunning={isRunning}
                  taskId={task.id}
                />
              );
            }

            // d) Inline Tool Execution Card
            if (activity?.type === 'tool' && activity?.tool) {
              return (
                <ToolExecutionCard
                  key={key}
                  tool={activity.tool}
                />
              );
            }

            // User steer message
            if (activity?.type === 'user' || activity?.type === 'steer') {
              const images = Array.isArray(activity?.images) ? activity.images : [];
              return (
                <div
                  key={key}
                  className="rounded-lg border border-[#ebebeb] bg-[#f9f9f9] p-3.5 shadow-xs"
                >
                  <div className="flex items-center justify-between pb-1.5 mb-2 border-b border-[#ebebeb] text-[10px] font-mono text-[#666666]">
                    <div className="flex items-center space-x-1.5 text-[#171717] font-medium">
                      <User className="h-3 w-3" />
                      <span>Указание пользователя (Steer)</span>
                    </div>
                    <span className="tabular-nums">{activity?.timestamp ?? ''}</span>
                  </div>
                  {images.length > 0 && (
                    <div className="flex items-center gap-2 mb-2.5 flex-wrap">
                      {images.map((imgUrl, i) => (
                        <div
                          key={i}
                          className="relative rounded-lg overflow-hidden border border-[#ebebeb] max-w-[240px] max-h-40 bg-[#f4f4f5] shadow-xs"
                        >
                          <img
                            src={imgUrl}
                            alt={`Прикреплённое изображение ${i + 1}`}
                            className="w-full h-full max-h-40 object-cover hover:scale-105 transition-transform duration-150 cursor-pointer"
                            onClick={() => window.open(imgUrl, '_blank')}
                          />
                        </div>
                      ))}
                    </div>
                  )}
                  <p className="text-sm text-[#171717] whitespace-pre-wrap select-text leading-relaxed font-sans">
                    {activity?.content ?? ''}
                  </p>
                </div>
              );
            }

            // System notices
            if (activity?.type === 'system') {
              return (
                <div
                  key={key}
                  className="flex items-center space-x-2 rounded-md border border-[#ebebeb] bg-[#f4f4f5] px-3 py-2 text-xs text-[#4d4d4d] font-mono shadow-xs"
                >
                  <Info className="h-3.5 w-3.5 text-[#666666] shrink-0" />
                  <span className="flex-1">{activity?.content ?? ''}</span>
                  <span className="text-[10px] text-[#8f8f8f] tabular-nums">
                    {activity?.timestamp ?? ''}
                  </span>
                </div>
              );
            }

            // Error notices
            if (activity?.type === 'error') {
              return (
                <div
                  key={key}
                  className="flex items-start space-x-2 rounded-md border border-[#fecaca] bg-[#fef2f2] p-3 text-xs text-[#991b1b] font-mono shadow-xs"
                >
                  <AlertTriangle className="h-4 w-4 text-[#dc2626] shrink-0 mt-0.5" />
                  <div className="flex-1 whitespace-pre-wrap select-text">
                    {activity?.content ?? ''}
                  </div>
                  <span className="text-[10px] text-[#b91c1c] tabular-nums shrink-0">
                    {activity?.timestamp ?? ''}
                  </span>
                </div>
              );
            }

            // c) Assistant Response / Output Markdown Block (скрываемый, по умолчанию свёрнут)
            const isOutputExpanded = expandedOutputs.has(key);
            return (
              <div
                key={key}
                className="rounded-lg border border-[#ebebeb] bg-white shadow-xs overflow-hidden"
              >
                <div
                  onClick={() => toggleOutput(key)}
                  className="flex items-center justify-between px-4 py-2.5 hover:bg-[#f9f9f9] cursor-pointer transition select-none text-[10px] font-mono text-[#666666]"
                >
                  <div className="flex items-center space-x-1.5 text-[#171717] font-medium">
                    {isOutputExpanded ? (
                      <ChevronDown className="h-3.5 w-3.5 text-[#666666]" />
                    ) : (
                      <ChevronRight className="h-3.5 w-3.5 text-[#666666]" />
                    )}
                    <Sparkles className="h-3 w-3 text-[#171717]" />
                    <span>Ответ агента</span>
                  </div>
                  <span className="tabular-nums">{activity?.timestamp ?? ''}</span>
                </div>
                {isOutputExpanded && (
                  <div className="px-4 pb-4 pt-2.5 border-t border-[#ebebeb]">
                    {activity?.content ? (
                      <MarkdownContent content={activity.content} />
                    ) : (
                      <div className="text-xs text-[#8f8f8f] font-mono">Генерация ответа...</div>
                    )}
                  </div>
                )}
              </div>
            );
          })}

          {/* No placeholder: thinking/tool events render immediately on arrival.
              Agent is launched via the input bar below or the card Play button. */}
        </div>
        </div>
        {showNewButton && (
          <button
            type="button"
            onClick={scrollToBottom}
            className="absolute bottom-4 left-1/2 -translate-x-1/2 flex items-center space-x-1.5 px-3 py-1.5 rounded-md bg-[#171717] hover:bg-[#262626] text-white text-xs font-medium font-sans shadow-md transition cursor-pointer active:scale-[0.98]"
            title="Перейти к новым сообщениям"
          >
            <ArrowDown className="h-3.5 w-3.5" />
            <span>К новому</span>
          </button>
        )}
      </div>

      {/* f) Interactive Input & Steer Bar: Floating Light Card & Bottom Status Bar */}
      <footer className="border-t border-[#ebebeb] bg-[#fafafa]/80 backdrop-blur-xs p-3 shrink-0 shadow-xs">
        <div className="max-w-4xl mx-auto">
          {/* Floating Modern Light Card */}
          <div
            onDragOver={handleChatDragOver}
            onDragLeave={handleChatDragLeave}
            onDrop={handleChatDrop}
            className={`rounded-2xl bg-white border border-[#ebebeb] transition-all shadow-xs p-3 ${
              isDragOverChat ? 'ring-2 ring-blue-500/30 bg-blue-50/40' : ''
            }`}
          >
            {/* Attached Images Thumbnail Preview Bar */}
            {attachedImages.length > 0 && (
              <div className="flex items-center gap-2 mb-2.5 pb-1 overflow-x-auto select-none">
                {attachedImages.map((img) => (
                  <div
                    key={img.id}
                    className="relative group shrink-0 w-16 h-16 rounded-xl overflow-hidden border border-[#ebebeb] bg-[#f4f4f5]"
                  >
                    <img
                      src={img.dataUrl}
                      alt={img.name}
                      className="w-full h-full object-cover"
                    />
                    <button
                      type="button"
                      onClick={() => setAttachedImages((prev) => prev.filter((i) => i.id !== img.id))}
                      className="absolute top-1 right-1 w-4 h-4 rounded-full bg-white/90 hover:bg-white text-[#171717] border border-[#ebebeb] flex items-center justify-center text-xs leading-none transition cursor-pointer"
                      title="Удалить"
                    >
                      ×
                    </button>
                    <div className="absolute inset-x-0 bottom-0 bg-white/90 px-1 py-0.5 text-[8.5px] font-mono text-[#666666] truncate text-center border-t border-[#ebebeb]">
                      {img.name}
                    </div>
                  </div>
                ))}
              </div>
            )}

            {/* Multiline auto-resizing textarea */}
            <textarea
              ref={textareaRef}
              rows={1}
              value={interactiveInput}
              onChange={handleInputChange}
              onKeyDown={handleKeyDown}
              onPaste={handleChatPaste}
              placeholder="Напишите сообщение, задачу или прикрепите изображение..."
              className="w-full bg-transparent text-sm text-[#171717] placeholder:text-[#8f8f8f] focus:outline-none resize-none leading-relaxed min-h-[44px] max-h-44 overflow-y-auto font-sans"
            />

            {/* Bottom toolbar inside card */}
            <div className="flex items-center justify-between pt-2 mt-1 border-t border-[#ebebeb] select-none">
              {/* Left Toolbar Items */}
              <div className="flex items-center space-x-2">
                {/* Round "+" Button */}
                <button
                  type="button"
                  onClick={() => chatFileInputRef.current?.click()}
                  className="w-7 h-7 rounded-full bg-[#f4f4f5] hover:bg-[#171717] text-[#171717] hover:text-white flex items-center justify-center transition cursor-pointer"
                  title="Прикрепить изображение (PNG, JPG, WebP) или файл"
                >
                  <Plus className="h-4 w-4" />
                </button>

                {/* Hidden File Input for "+" Button */}
                <input
                  ref={chatFileInputRef}
                  type="file"
                  accept="image/png,image/jpeg,image/webp,image/gif"
                  multiple
                  className="hidden"
                  onChange={handleChatFileSelect}
                />

                {/* Status Pill: "🛡️ Full access ⌵" */}
                <div className="relative">
                  <button
                    type="button"
                    onClick={() => setIsToolMenuOpen(!isToolMenuOpen)}
                    className="px-2.5 py-1 rounded-full bg-[#f4f4f5] hover:bg-[#e4e4e7] text-[#171717] font-mono text-xs border border-[#ebebeb] transition cursor-pointer flex items-center gap-1.5"
                    title="Режим доступа к инструментам"
                  >
                    <span>{toolAccessMode === 'full' ? '🛡️ Full access' : '🔒 Read only'}</span>
                    <ChevronDown className="h-3 w-3 text-[#8f8f8f]" />
                  </button>
                  {isToolMenuOpen && (
                    <div className="absolute bottom-full left-0 mb-2 w-48 rounded-lg bg-white border border-[#ebebeb] shadow-md p-1 z-30 font-mono text-xs">
                      <div className="px-2 py-1 text-[#8f8f8f] text-[10px] uppercase">Режим инструментов</div>
                      <button
                        type="button"
                        onClick={() => {
                          setToolAccessMode('full');
                          setIsToolMenuOpen(false);
                        }}
                        className={`w-full text-left px-2 py-1.5 rounded flex items-center justify-between cursor-pointer ${
                          toolAccessMode === 'full' ? 'bg-[#f4f4f5] text-[#171717]' : 'text-[#4d4d4d] hover:bg-[#f9f9f9]'
                        }`}
                      >
                        <span>🛡️ Full access</span>
                        {toolAccessMode === 'full' && <span className="text-emerald-600">✓</span>}
                      </button>
                      <button
                        type="button"
                        onClick={() => {
                          setToolAccessMode('readonly');
                          setIsToolMenuOpen(false);
                        }}
                        className={`w-full text-left px-2 py-1.5 rounded flex items-center justify-between cursor-pointer ${
                          toolAccessMode === 'readonly' ? 'bg-[#f4f4f5] text-[#171717]' : 'text-[#4d4d4d] hover:bg-[#f9f9f9]'
                        }`}
                      >
                        <span>🔒 Read only</span>
                        {toolAccessMode === 'readonly' && <span className="text-emerald-600">✓</span>}
                      </button>
                    </div>
                  )}
                </div>
              </div>

              {/* Right Toolbar Items */}
              <div className="flex items-center space-x-2">
                {/* Minimalist Model Selector */}
                <div className="relative">
                  <button
                    type="button"
                    onClick={() => setIsModelMenuOpen(!isModelMenuOpen)}
                    className="px-2.5 py-1 rounded-full bg-[#f4f4f5] hover:bg-[#e4e4e7] text-[#171717] font-mono text-xs border border-[#ebebeb] transition cursor-pointer flex items-center gap-1.5 max-w-[200px]"
                    title={`Модель задачи: ${effectiveModel}`}
                  >
                    <span className="truncate">{effectiveModel}</span>
                    <ChevronDown className="h-3 w-3 text-[#8f8f8f] shrink-0" />
                  </button>
                  {isModelMenuOpen && (
                    <div className="absolute bottom-full right-0 mb-2 w-56 rounded-lg bg-white border border-[#ebebeb] shadow-md p-1 z-30 font-mono text-xs max-h-60 overflow-y-auto">
                      <div className="px-2 py-1 text-[#8f8f8f] text-[10px] uppercase">Выбор модели</div>
                      {selectableModels.map((m) => (
                        <button
                          key={m}
                          type="button"
                          onClick={() => {
                            updateTask(task.id, { model: m });
                            setIsModelMenuOpen(false);
                          }}
                          className={`w-full text-left px-2 py-1.5 rounded flex items-center justify-between cursor-pointer ${
                            effectiveModel === m ? 'bg-[#f4f4f5] text-[#171717]' : 'text-[#4d4d4d] hover:bg-[#f9f9f9]'
                          }`}
                        >
                          <span className="truncate">{m}</span>
                          {effectiveModel === m && <span className="text-emerald-600 shrink-0 ml-1">✓</span>}
                        </button>
                      ))}
                    </div>
                  )}
                </div>

                {/* Round Action Button: Abort when running, Send when idle */}
                {isRunning ? (
                  <button
                    type="button"
                    onClick={handleAbortAgent}
                    className="w-8 h-8 rounded-full bg-[#fef2f2] hover:bg-[#fee2e2] text-[#b91c1c] border border-[#fecaca] flex items-center justify-center shadow-xs transition cursor-pointer active:scale-95"
                    title="Остановить выполнение (abort_agent)"
                  >
                    <Square className="h-3.5 w-3.5 fill-current" />
                  </button>
                ) : (
                  <button
                    type="button"
                    onClick={() => handleSendPrompt()}
                    disabled={!interactiveInput.trim() && attachedImages.length === 0}
                    className="w-8 h-8 rounded-full bg-[#171717] hover:bg-black disabled:opacity-40 disabled:hover:bg-[#171717] disabled:cursor-not-allowed text-white flex items-center justify-center shadow-xs transition cursor-pointer active:scale-95"
                    title="Отправить сообщение агенту (Enter)"
                  >
                    <ArrowUp className="h-4 w-4 stroke-[2.5]" />
                  </button>
                )}
              </div>
            </div>
          </div>

          {/* Elegant Telemetry Status Line (Кэш, TPS, Контекст) under chat card */}
          <ContextGauge taskId={task.id} turns={turns} />
        </div>
      </footer>
        </ErrorBoundary>
    </div>

    {/* Right Column: Git Diff Interactive Sidebar */}
    <ErrorBoundary scopeLabel="Git Diff" onClose={() => setIsDiffCollapsed(true)}>
      <GitDiffSidebar
        taskId={task.id}
        isCollapsed={isDiffCollapsed}
        onToggleCollapse={() => setIsDiffCollapsed((prev) => !prev)}
      />
    </ErrorBoundary>
  </div>
</div>
  );
}
