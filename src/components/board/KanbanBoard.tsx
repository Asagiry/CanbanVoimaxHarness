import { useEffect, useState, useRef, useCallback } from 'react';
import { DragDropContext, Droppable, DropResult } from '@hello-pangea/dnd';
import { useBoardStore } from '../../stores/boardStore';
import { useDrawerStore } from '../../stores/drawerStore';
import { useSettingsStore } from '../../stores/settingsStore';
import KanbanColumn from './KanbanColumn';
import { KanbanTask, PipelineColumn, TaskStatus } from '../../types';
import { Plus, X, Layout } from 'lucide-react';
import CreateBoardModal from './CreateBoardModal';

const COLUMN_PALETTE = [
  '#64748b',
  '#f59e0b',
  '#a855f7',
  '#0284c7',
  '#10b981',
  '#ef4444',
  '#6366f1',
  '#14b8a6',
];

const CYRILLIC_MAP: Record<string, string> = {
  'а': 'a', 'б': 'b', 'в': 'v', 'г': 'g', 'д': 'd', 'е': 'e', 'ё': 'yo', 'ж': 'zh',
  'з': 'z', 'и': 'i', 'й': 'y', 'к': 'k', 'л': 'l', 'м': 'm', 'н': 'n', 'о': 'o',
  'п': 'p', 'р': 'r', 'с': 's', 'т': 't', 'у': 'u', 'ф': 'f', 'х': 'h', 'ц': 'ts',
  'ч': 'ch', 'ш': 'sh', 'щ': 'sch', 'ъ': '', 'ы': 'y', 'ь': '', 'э': 'e', 'ю': 'yu', 'я': 'ya',
};

function slugifyTitle(title: string): string {
  const latin = title
    .toLowerCase()
    .split('')
    .map((ch) => CYRILLIC_MAP[ch] ?? ch)
    .join('');
  const slug = latin
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 40);
  return slug || 'column';
}

function uniqueColumnId(base: string, taken: Set<string>): string {
  if (!taken.has(base)) return base;
  let n = 2;
  while (taken.has(`${base}-${n}`)) n += 1;
  return `${base}-${n}`;
}

interface AgentPersonaOption {
  role: string;
  hasPrompt?: boolean;
  has_prompt?: boolean;
}

const NEW_AGENT_OPTION = '__new__';

interface ThreadLine {
  key: string;
  parentId: string;
  subtaskId: string;
  x1: number;
  y1: number;
  x2: number;
  y2: number;
  cx1: number;
  cy1: number;
  cx2: number;
  cy2: number;
  mx: number;
  my: number;
  pathData: string;
  isHighlighted: boolean;
  parentColor: string;
  subtaskColor: string;
}

export default function KanbanBoard() {
  const {
    columns,
    tasks,
    searchQuery,
    moveTask,
    addTask,
    tickTimers,
    setColumns,
    hoveredTaskId,
    setHoveredTaskId,
    boards,
    activeBoardId,
    setActiveBoard,
    deleteBoard,
    saveCurrentBoard,
  } = useBoardStore();
  const { openDrawer } = useDrawerStore();
  const { availableModels, settings } = useSettingsStore();
  const scrollContainerRef = useRef<HTMLDivElement | null>(null);

  const [threads, setThreads] = useState<ThreadLine[]>([]);
  const [svgDimensions, setSvgDimensions] = useState({ width: 0, height: 0 });

  const computeThreads = useCallback(() => {
    const container = scrollContainerRef.current;
    if (!container) return;

    const cRect = container.getBoundingClientRect();
    const scrollLeft = container.scrollLeft;
    const scrollTop = container.scrollTop;

    setSvgDimensions({
      width: Math.max(container.scrollWidth, container.clientWidth),
      height: Math.max(container.scrollHeight, container.clientHeight),
    });

    const lines: ThreadLine[] = [];

    for (const subtask of tasks) {
      if (!subtask.parentId) continue;
      const parent = tasks.find((t) => t.id === subtask.parentId);
      if (!parent) continue;

      const parentCol = columns.find((c) => c.id === parent.columnId);
      const subtaskCol = columns.find((c) => c.id === subtask.columnId);

      const isParentDone = Boolean(
        parentCol?.terminal ||
        parent.columnId === 'ready' ||
        parent.status === 'done' ||
        parentCol?.title.toLowerCase().includes('готов') ||
        parentCol?.title.toLowerCase().includes('done')
      );
      const isSubtaskDone = Boolean(
        subtaskCol?.terminal ||
        subtask.columnId === 'ready' ||
        subtask.status === 'done' ||
        subtaskCol?.title.toLowerCase().includes('готов') ||
        subtaskCol?.title.toLowerCase().includes('done')
      );

      // In Done column, subtasks are aggregated inside root card
      if (isParentDone && isSubtaskDone) continue;

      const elP = document.getElementById(`task-card-${parent.id}`);
      const elS = document.getElementById(`task-card-${subtask.id}`);
      if (!elP || !elS) continue;

      const pRect = elP.getBoundingClientRect();
      const sRect = elS.getBoundingClientRect();

      const isForward = pRect.left <= sRect.left;
      const x1 = isForward
        ? pRect.right - cRect.left + scrollLeft
        : pRect.left - cRect.left + scrollLeft;
      const y1 = pRect.top + pRect.height / 2 - cRect.top + scrollTop;

      const x2 = isForward
        ? sRect.left - cRect.left + scrollLeft
        : sRect.right - cRect.left + scrollLeft;
      const y2 = sRect.top + sRect.height / 2 - cRect.top + scrollTop;

      const dx = Math.max(36, Math.abs(x2 - x1) * 0.45);
      const cx1 = isForward ? x1 + dx : x1 - dx;
      const cy1 = y1;
      const cx2 = isForward ? x2 - dx : x2 + dx;
      const cy2 = y2;

      const pathData = `M ${x1} ${y1} C ${cx1} ${cy1}, ${cx2} ${cy2}, ${x2} ${y2}`;

      // Midpoint on cubic Bezier at t = 0.5
      const mx = 0.125 * x1 + 0.375 * cx1 + 0.375 * cx2 + 0.125 * x2;
      const my = 0.125 * y1 + 0.375 * cy1 + 0.375 * cy2 + 0.125 * y2;

      const isHighlighted = Boolean(
        hoveredTaskId &&
          (hoveredTaskId === parent.id ||
            hoveredTaskId === subtask.id ||
            (parent.waitingForSubtaskId && hoveredTaskId === parent.waitingForSubtaskId))
      );

      lines.push({
        key: `${parent.id}->${subtask.id}`,
        parentId: parent.id,
        subtaskId: subtask.id,
        x1,
        y1,
        x2,
        y2,
        cx1,
        cy1,
        cx2,
        cy2,
        mx,
        my,
        pathData,
        isHighlighted,
        parentColor: parentCol?.color || '#f59e0b',
        subtaskColor: subtaskCol?.color || '#8b5cf6',
      });
    }

    setThreads(lines);
  }, [tasks, columns, hoveredTaskId]);

  useEffect(() => {
    let animId = requestAnimationFrame(computeThreads);
    const handleResize = () => {
      cancelAnimationFrame(animId);
      animId = requestAnimationFrame(computeThreads);
    };
    window.addEventListener('resize', handleResize);

    const container = scrollContainerRef.current;
    let resizeObserver: ResizeObserver | null = null;
    if (container && typeof ResizeObserver !== 'undefined') {
      resizeObserver = new ResizeObserver(() => {
        cancelAnimationFrame(animId);
        animId = requestAnimationFrame(computeThreads);
      });
      resizeObserver.observe(container);
    }

    return () => {
      cancelAnimationFrame(animId);
      window.removeEventListener('resize', handleResize);
      if (resizeObserver) resizeObserver.disconnect();
    };
  }, [computeThreads]);
  const [isNewTaskModalOpen, setIsNewTaskModalOpen] = useState(false);
  const [isCreateBoardModalOpen, setIsCreateBoardModalOpen] = useState(false);
  const [targetColumnForNew, setTargetColumnForNew] = useState<string>('backlog');
  const [selectedModel, setSelectedModel] = useState<string>(settings.model || 'gemini-3.8-flash-high');

  // New task form fields
  const [newTitle, setNewTitle] = useState('');
  const [newDescription, setNewDescription] = useState('');

  // New column inline form fields
  const [isAddColumnOpen, setIsAddColumnOpen] = useState(false);
  const [newColumnTitle, setNewColumnTitle] = useState('');
  const [newColumnPersona, setNewColumnPersona] = useState('developer');
  const [agentPersonas, setAgentPersonas] = useState<AgentPersonaOption[]>([]);
  const [usePersonaSelect, setUsePersonaSelect] = useState(false);
  const [personasLoading, setPersonasLoading] = useState(false);
  const [isNewAgentSelected, setIsNewAgentSelected] = useState(false);
  const [newAgentName, setNewAgentName] = useState('');
  const [columnFormError, setColumnFormError] = useState<string | null>(null);

  // 1-second timer ticker interval for live cards
  useEffect(() => {
    const timer = setInterval(() => {
      tickTimers();
    }, 1000);

    return () => clearInterval(timer);
  }, [tickTimers]);

  const handleDragEnd = (result: DropResult) => {
    const { source, destination, draggableId, type } = result;
    if (!destination) return;
    if (source.droppableId === destination.droppableId && source.index === destination.index) {
      return;
    }

    if (type === 'COLUMN') {
      void handleReorderColumns(source.index, destination.index);
      return;
    }

    moveTask(draggableId, destination.droppableId);
  };

  const savePipelineConfig = async (next: PipelineColumn[]): Promise<boolean> => {
    try {
      await saveCurrentBoard(next);
      return true;
    } catch (err) {
      console.info('saveCurrentBoard failed, rolling back:', err);
      return false;
    }
  };

  const handleReorderColumns = async (sourceIndex: number, destinationIndex: number) => {
    const prev = columns;
    const reordered = Array.from(prev);
    const [moved] = reordered.splice(sourceIndex, 1);
    if (!moved) return;
    reordered.splice(destinationIndex, 0, moved);

    // backlog строго первая, ready строго последняя, остальные — в drop-порядке.
    const backlog = reordered.find((c) => c.id === 'backlog');
    const ready = reordered.find((c) => c.id === 'ready');
    const middle = reordered.filter((c) => c.id !== 'backlog' && c.id !== 'ready');
    const next = [...(backlog ? [backlog] : []), ...middle, ...(ready ? [ready] : [])];

    if (prev.length === next.length && prev.every((c, i) => c.id === next[i]?.id)) return;

    setColumns(next);
    const saved = await savePipelineConfig(next);
    if (!saved) {
      setColumns(prev);
    }
  };

  // Создание задач — только через общую кнопку доски: цель backlog, иначе первая колонка.
  const handleOpenAddTask = () => {
    const target = columns.some((c) => c.id === 'backlog') ? 'backlog' : (columns[0]?.id ?? 'backlog');
    setTargetColumnForNew(target);
    setNewTitle('');
    setNewDescription('');
    setSelectedModel(settings.model || (availableModels.length > 0 ? availableModels[0] : 'gemini-3.8-flash-high'));
    setIsNewTaskModalOpen(true);
  };

  const handleCreateTask = (e: React.FormEvent) => {
    e.preventDefault();
    if (!newTitle.trim()) return;

    const targetCol = columns.find((c) => c.id === targetColumnForNew);
    const persona = targetCol?.persona ?? 'developer';
    const status: TaskStatus = targetCol?.terminal ? 'done' : targetCol?.persona ? 'running' : 'idle';

    const nextIdNum = tasks.length > 0
      ? Math.max(...tasks.map((t) => parseInt(t.id.replace(/\D/g, ''), 10) || 0), tasks.length) + 1
      : 1;
    const paddedId = `CVH-${String(nextIdNum).padStart(3, '0')}`;
    const taskId = paddedId;

    const newTask: KanbanTask = {
      id: taskId,
      title: newTitle.trim(),
      description: newDescription.trim() || undefined,
      priority: 'high',
      columnId: targetColumnForNew,
      boardId: activeBoardId || 'development',
      persona,
      branch: `feat/${paddedId.toLowerCase()}`,
      worktreePath: `.cvh/worktrees/${paddedId.toLowerCase()}`,
      status,
      activeStageDuration: 0,
      stageDurations: {},
      totalElapsed: 0,
      isTimerRunning: status === 'running',
      returnsCount: 0,
      verdict: 'NONE',
      model: selectedModel,
    };

    addTask(newTask);
    setIsNewTaskModalOpen(false);
  };

  const loadAgentPersonas = async () => {
    setPersonasLoading(true);
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const list = await invoke<AgentPersonaOption[]>('list_agent_personas');
      if (Array.isArray(list)) {
        setAgentPersonas(list);
        setUsePersonaSelect(true);
        const roles = new Set(list.map((p) => p.role));
        setNewColumnPersona(roles.has('developer') ? 'developer' : (list[0]?.role ?? ''));
      } else {
        setUsePersonaSelect(false);
      }
    } catch {
      // IPC недоступен — откат на свободный текстовый ввод.
      setUsePersonaSelect(false);
    } finally {
      setPersonasLoading(false);
    }
  };

  const handleOpenAddColumn = () => {
    setNewColumnTitle('');
    setNewColumnPersona('developer');
    setIsNewAgentSelected(false);
    setNewAgentName('');
    setColumnFormError(null);
    setIsAddColumnOpen(true);
    void loadAgentPersonas();
  };

  const handleCreateColumn = async (e: React.FormEvent) => {
    e.preventDefault();
    const title = newColumnTitle.trim();
    if (!title) return;
    setColumnFormError(null);

    let personaName: string;
    if (usePersonaSelect && isNewAgentSelected) {
      const role = newAgentName.trim();
      if (!role) {
        setColumnFormError('Укажите имя нового агента.');
        return;
      }
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('create_agent_persona', { role });
      } catch (err) {
        setColumnFormError(err instanceof Error ? err.message : String(err));
        return;
      }
      personaName = role;
    } else {
      personaName = newColumnPersona.trim();
    }

    const taken = new Set(columns.map((c) => c.id));
    const id = uniqueColumnId(slugifyTitle(title), taken);
    const newCol: PipelineColumn = {
      id,
      title,
      color: COLUMN_PALETTE[columns.length % COLUMN_PALETTE.length],
      icon: 'Layers',
      persona: personaName || null,
      contextWindow: 128000,
      modelOverride: null,
      hooks: {},
    };

    const prev = columns;
    const terminalIdx = prev.findIndex((c) => c.terminal);
    const next =
      terminalIdx === -1
        ? [...prev, newCol]
        : [...prev.slice(0, terminalIdx), newCol, ...prev.slice(terminalIdx)];

    setColumns(next);
    const saved = await savePipelineConfig(next);
    if (!saved) {
      setColumns(prev);
      return;
    }
    setNewColumnTitle('');
    setNewColumnPersona('developer');
    setIsNewAgentSelected(false);
    setNewAgentName('');
    setColumnFormError(null);
    setIsAddColumnOpen(false);
  };

  const handleDeleteColumn = async (columnId: string) => {
    if (columnId === 'backlog' || columnId === 'ready') return;
    if (columns.length <= 1) return;
    const idx = columns.findIndex((c) => c.id === columnId);
    if (idx === -1) return;
    const col = columns[idx];
    const neighbor = columns[idx - 1] ?? columns[idx + 1] ?? null;
    if (!neighbor) return;

    const doomedTasks = tasks.filter((t) => t.columnId === columnId);
    const confirmed = window.confirm(
      doomedTasks.length === 0
        ? `Удалить колонку «${col.title}»?`
        : `Удалить колонку «${col.title}»? Задач в ней: ${doomedTasks.length}. Они будут перенесены в колонку «${neighbor.title}».`
    );
    if (!confirmed) return;

    const prev = columns;
    const next = prev.filter((c) => c.id !== columnId);

    setColumns(next);
    const saved = await savePipelineConfig(next);
    if (!saved) {
      setColumns(prev);
      return;
    }
    for (const t of doomedTasks) {
      await moveTask(t.id, neighbor.id);
    }
  };

  // Filter tasks based on activeBoardId and searchQuery
  const currentBoardTasks = tasks.filter(
    (task) => (task.boardId || 'development') === activeBoardId
  );

  const filteredTasks = currentBoardTasks.filter((task) => {
    if (!searchQuery) return true;
    const q = searchQuery.toLowerCase();
    return (
      task.id.toLowerCase().includes(q) ||
      task.title.toLowerCase().includes(q) ||
      task.branch.toLowerCase().includes(q) ||
      (task.description && task.description.toLowerCase().includes(q))
    );
  });

  return (
    <div className="flex-1 flex flex-col h-full overflow-hidden bg-[#fafafa]">
      {/* Board Top Toolbar */}
      <div className="flex items-center justify-between px-3.5 py-2 border-b border-[#ebebeb] bg-white text-xs select-none">
        {/* Left: Board Switcher Tabs */}
        <div className="flex items-center space-x-1.5 overflow-x-auto py-0.5">
          {boards.map((b) => {
            const isActive = b.id === activeBoardId;
            const boardTasksCount = tasks.filter((t) => (t.boardId || 'development') === b.id).length;
            return (
              <div key={b.id} className="group relative flex items-center">
                <button
                  onClick={() => setActiveBoard(b.id)}
                  className={`flex items-center space-x-1.5 px-3 py-1.5 rounded-md text-xs font-medium transition cursor-pointer select-none ${
                    isActive
                      ? 'bg-[#171717] text-white shadow-xs'
                      : 'bg-[#f4f4f5] hover:bg-[#e4e4e7] text-[#52525b] hover:text-[#171717]'
                  }`}
                >
                  <Layout className="h-3.5 w-3.5" />
                  <span>{b.name}</span>
                  <span
                    className={`text-[10px] px-1.5 py-0.2 rounded-full font-mono font-medium ${
                      isActive
                        ? 'bg-[#27272a] text-[#d4d4d8]'
                        : 'bg-[#e4e4e7] text-[#71717a]'
                    }`}
                  >
                    {boardTasksCount}
                  </span>
                </button>
                {boards.length > 1 && (
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      if (
                        window.confirm(
                          `Удалить доску «${b.name}»? Все задачи этой доски будут перенесены на активную доску.`
                        )
                      ) {
                        void deleteBoard(b.id);
                      }
                    }}
                    title={`Удалить доску «${b.name}»`}
                    className="opacity-0 group-hover:opacity-100 p-1 hover:text-red-500 text-[#a1a1aa] transition cursor-pointer -ml-1 mr-0.5"
                  >
                    <X className="h-3 w-3" />
                  </button>
                )}
              </div>
            );
          })}
          <button
            onClick={() => setIsCreateBoardModalOpen(true)}
            className="flex items-center space-x-1 px-2.5 py-1.5 rounded-md border border-dashed border-[#d4d4d8] hover:border-[#171717] text-[#71717a] hover:text-[#171717] text-xs font-medium transition cursor-pointer select-none"
            title="Создать новую доску"
          >
            <Plus className="h-3 w-3" />
            <span>Новая доска</span>
          </button>
        </div>

        {/* Right: Board Stats & Action Buttons */}
        <div className="flex items-center space-x-3">
          <div className="hidden sm:flex items-center space-x-1.5 text-[#666666] text-xs">
            <span>Задач на доске:</span>
            <span className="text-[#171717] font-semibold tabular-nums font-mono">
              {currentBoardTasks.length}
            </span>
          </div>

          <div className="flex items-center space-x-2">
            <button
              onClick={handleOpenAddColumn}
              className="flex items-center space-x-1.5 px-3 py-1.5 rounded-md bg-white border border-[#ebebeb] hover:bg-[#f4f4f5] text-[#171717] font-sans text-xs font-medium transition cursor-pointer active:scale-[0.99]"
              title="Добавить колонку"
            >
              <Plus className="h-3.5 w-3.5" />
              <span>Колонка</span>
            </button>
            <button
              onClick={handleOpenAddTask}
              className="flex items-center space-x-1.5 px-3 py-1.5 rounded-md bg-[#171717] hover:bg-[#262626] text-white font-sans text-xs font-medium transition cursor-pointer shadow-xs active:scale-[0.99]"
              title="Создать новую задачу"
            >
              <Plus className="h-3.5 w-3.5 text-white" />
              <span>Создать задачу</span>
            </button>
          </div>
        </div>
      </div>

      {/* Add Column Inline Form */}
      {isAddColumnOpen && (
        <div className="px-3.5 py-2.5 border-b border-[#ebebeb] bg-white">
          <form onSubmit={handleCreateColumn} className="flex flex-wrap items-end gap-x-3 gap-y-2 text-xs">
            <div>
              <label className="block font-sans text-[#4d4d4d] font-medium mb-1">
                Название колонки
              </label>
              <input
                type="text"
                required
                autoFocus
                value={newColumnTitle}
                onChange={(e) => setNewColumnTitle(e.target.value)}
                placeholder="Например: Тестирование"
                className="w-56 px-3 py-1.5 rounded-md bg-[#fafafa] border border-[#ebebeb] text-[#171717] placeholder:text-[#8f8f8f] focus:outline-hidden focus:border-[#171717] focus:bg-white font-sans transition"
              />
            </div>

            <div>
              <label className="block font-sans text-[#4d4d4d] font-medium mb-1">
                Агент
              </label>
              {usePersonaSelect ? (
                <select
                  value={isNewAgentSelected ? NEW_AGENT_OPTION : newColumnPersona}
                  onChange={(e) => {
                    if (e.target.value === NEW_AGENT_OPTION) {
                      setIsNewAgentSelected(true);
                    } else {
                      setIsNewAgentSelected(false);
                      setNewColumnPersona(e.target.value);
                    }
                  }}
                  disabled={personasLoading}
                  className="w-44 px-2.5 py-1.5 rounded-md bg-[#fafafa] border border-[#ebebeb] text-[#171717] focus:outline-hidden focus:border-[#171717] focus:bg-white font-mono transition cursor-pointer"
                >
                  {agentPersonas.map((p) => (
                    <option key={p.role} value={p.role}>
                      {p.role}
                    </option>
                  ))}
                  <option value={NEW_AGENT_OPTION}>+ Новый агент</option>
                </select>
              ) : (
                <input
                  type="text"
                  value={newColumnPersona}
                  onChange={(e) => setNewColumnPersona(e.target.value)}
                  placeholder={personasLoading ? 'Загрузка агентов...' : 'developer'}
                  className="w-44 px-3 py-1.5 rounded-md bg-[#fafafa] border border-[#ebebeb] text-[#171717] placeholder:text-[#8f8f8f] focus:outline-hidden focus:border-[#171717] focus:bg-white font-mono transition"
                />
              )}
            </div>

            {usePersonaSelect && isNewAgentSelected && (
              <div>
                <label className="block font-sans text-[#4d4d4d] font-medium mb-1">
                  Имя нового агента
                </label>
                <input
                  type="text"
                  required
                  autoFocus
                  value={newAgentName}
                  onChange={(e) => setNewAgentName(e.target.value)}
                  placeholder="Например: tester"
                  className="w-44 px-3 py-1.5 rounded-md bg-[#fafafa] border border-[#ebebeb] text-[#171717] placeholder:text-[#8f8f8f] focus:outline-hidden focus:border-[#171717] focus:bg-white font-mono transition"
                />
              </div>
            )}

            <div
              className="flex items-center space-x-1.5 pb-2 text-[#8f8f8f]"
              title="Цвет колонки подберется автоматически"
            >
              <span
                className="h-4 w-4 rounded-full border border-[#ebebeb]"
                style={{ backgroundColor: COLUMN_PALETTE[columns.length % COLUMN_PALETTE.length] }}
              />
              <span>Цвет авто</span>
            </div>

            <div className="flex items-center space-x-2 pb-0.5">
              <button
                type="submit"
                className="px-3.5 py-1.5 rounded-md bg-[#171717] hover:bg-[#262626] text-white font-medium transition cursor-pointer font-sans shadow-xs"
              >
                Создать
              </button>
              <button
                type="button"
                onClick={() => setIsAddColumnOpen(false)}
                className="px-3 py-1.5 rounded-md bg-white border border-[#ebebeb] text-[#171717] hover:bg-[#f4f4f5] transition cursor-pointer font-sans"
              >
                Отмена
              </button>
            </div>

            {columnFormError && (
              <div className="w-full text-xs font-sans text-red-700 bg-red-50 border border-red-200 rounded-md px-3 py-1.5">
                {columnFormError}
              </div>
            )}
          </form>
        </div>
      )}

      {/* Kanban Columns Horizontal Container with Mouse Wheel Scroll */}
      <DragDropContext onDragEnd={handleDragEnd}>
        <Droppable droppableId="board-columns" direction="horizontal" type="COLUMN">
          {(provided) => (
        <div
          ref={(el) => {
            provided.innerRef(el);
            scrollContainerRef.current = el;
          }}
          {...provided.droppableProps}
          onWheel={(e) => {
            if (e.deltaY !== 0) {
              e.currentTarget.scrollLeft += e.deltaY;
            }
          }}
          onScroll={computeThreads}
          className="flex-1 overflow-x-auto overflow-y-hidden p-3.5 relative"
        >
          {/* Visual Threads SVG Overlay */}
          {threads.length > 0 && (
            <svg
              className="pointer-events-none absolute top-0 left-0 z-20 overflow-visible"
              style={{
                width: svgDimensions.width || '100%',
                height: svgDimensions.height || '100%',
              }}
            >
              <defs>
                <filter id="thread-glow" x="-20%" y="-20%" width="140%" height="140%">
                  <feDropShadow dx="0" dy="0" stdDeviation="3.5" floodColor="#8b5cf6" floodOpacity="0.75" />
                </filter>
              </defs>

              {threads.map((thread) => (
                <g key={thread.key} className="transition-all duration-150">
                  {/* Glowing halo behind active or hovered thread */}
                  {thread.isHighlighted && (
                    <path
                      d={thread.pathData}
                      stroke={thread.subtaskColor}
                      strokeWidth={6}
                      strokeOpacity={0.4}
                      fill="none"
                      filter="url(#thread-glow)"
                    />
                  )}

                  {/* Main connector Bezier line */}
                  <path
                    d={thread.pathData}
                    stroke={thread.subtaskColor}
                    strokeWidth={thread.isHighlighted ? 2.5 : 1.5}
                    strokeDasharray={thread.isHighlighted ? 'none' : '4 3'}
                    strokeOpacity={thread.isHighlighted ? 1 : 0.65}
                    fill="none"
                  />

                  {/* Invisible wide interactive stroke for hover and click */}
                  <path
                    d={thread.pathData}
                    stroke="transparent"
                    strokeWidth={16}
                    fill="none"
                    className="cursor-pointer pointer-events-auto"
                    onClick={() => openDrawer(thread.subtaskId)}
                    onMouseEnter={() => setHoveredTaskId(thread.subtaskId)}
                    onMouseLeave={() => setHoveredTaskId(null)}
                  >
                    <title>{`Нить: ${thread.parentId} ➔ ${thread.subtaskId} (нажмите для открытия консоли подзадачи)`}</title>
                  </path>

                  {/* Node at parent */}
                  <circle
                    cx={thread.x1}
                    cy={thread.y1}
                    r={thread.isHighlighted ? 4.5 : 3.5}
                    fill={thread.parentColor}
                    stroke="#ffffff"
                    strokeWidth={1.5}
                  />

                  {/* Node at subtask */}
                  <circle
                    cx={thread.x2}
                    cy={thread.y2}
                    r={thread.isHighlighted ? 4.5 : 3.5}
                    fill={thread.subtaskColor}
                    stroke="#ffffff"
                    strokeWidth={1.5}
                  />
                  {thread.isHighlighted && (
                    <circle
                      cx={thread.x2}
                      cy={thread.y2}
                      r={7}
                      fill="none"
                      stroke={thread.subtaskColor}
                      strokeWidth={1.5}
                      opacity={0.8}
                      className="animate-ping"
                    />
                  )}

                  {/* Midpoint badge displaying subtask ID */}
                  <g
                    transform={`translate(${thread.mx - 27}, ${thread.my - 10})`}
                    className="cursor-pointer pointer-events-auto select-none"
                    onClick={() => openDrawer(thread.subtaskId)}
                    onMouseEnter={() => setHoveredTaskId(thread.subtaskId)}
                    onMouseLeave={() => setHoveredTaskId(null)}
                  >
                    <rect
                      width={54}
                      height={20}
                      rx={10}
                      fill={thread.isHighlighted ? '#171717' : '#ffffff'}
                      stroke={thread.isHighlighted ? thread.subtaskColor : '#e5e7eb'}
                      strokeWidth={1.5}
                      className="shadow-xs transition-colors duration-150"
                    />
                    <text
                      x={27}
                      y={13.5}
                      textAnchor="middle"
                      fill={thread.isHighlighted ? '#ffffff' : '#374151'}
                      fontSize={9}
                      fontFamily="monospace"
                      fontWeight={600}
                    >
                      {thread.subtaskId}
                    </text>
                  </g>
                </g>
              ))}
            </svg>
          )}

          <div className="flex items-start space-x-3.5 h-full min-w-max pb-1">
            {columns.map((column, index) => {
              const columnTasks = filteredTasks.filter((t) => t.columnId === column.id);
              return (
                <KanbanColumn
                  key={column.id}
                  column={column}
                  tasks={columnTasks}
                  index={index}
                  onDeleteColumn={columns.length > 1 ? handleDeleteColumn : undefined}
                />
              );
            })}
            {provided.placeholder}
          </div>
        </div>
          )}
        </Droppable>
      </DragDropContext>

      {/* Add Task Modal */}
      {isNewTaskModalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-xs p-3 animate-in fade-in duration-100">
          <div className="relative flex flex-col w-[480px] max-w-full rounded-lg border border-[#ebebeb] bg-white shadow-xl p-5 text-[#171717]">
            <div className="flex items-center justify-between pb-3 border-b border-[#ebebeb]">
              <h3 className="text-sm font-semibold text-[#171717]">
                Новая задача
              </h3>
              <button
                onClick={() => setIsNewTaskModalOpen(false)}
                className="p-1 rounded-md text-[#666666] hover:text-[#171717] hover:bg-[#f4f4f5] transition cursor-pointer"
                title="Закрыть"
              >
                <X className="h-4 w-4" />
              </button>
            </div>

            <form onSubmit={handleCreateTask} className="mt-4 space-y-3.5 text-xs">
              <div>
                <label className="block font-sans text-[#4d4d4d] font-medium mb-1">
                  Название
                </label>
                <input
                  type="text"
                  required
                  value={newTitle}
                  onChange={(e) => setNewTitle(e.target.value)}
                  placeholder="Краткое описание задачи..."
                  className="w-full px-3 py-1.5 rounded-md bg-[#fafafa] border border-[#ebebeb] text-[#171717] placeholder:text-[#8f8f8f] focus:outline-hidden focus:border-[#171717] focus:bg-white font-sans transition"
                />
              </div>

              <div>
                <label className="block font-sans text-[#4d4d4d] font-medium mb-1">
                  Описание
                </label>
                <textarea
                  rows={2}
                  value={newDescription}
                  onChange={(e) => setNewDescription(e.target.value)}
                  placeholder="Детали и требования к задаче..."
                  className="w-full px-3 py-1.5 rounded-md bg-[#fafafa] border border-[#ebebeb] text-[#171717] placeholder:text-[#8f8f8f] focus:outline-hidden focus:border-[#171717] focus:bg-white font-sans resize-none transition"
                />
              </div>

              <div>
                <label className="block font-sans text-[#4d4d4d] font-medium mb-1">
                  Колонка размещения
                </label>
                <select
                  value={targetColumnForNew}
                  onChange={(e) => setTargetColumnForNew(e.target.value)}
                  className="w-full px-2.5 py-1.5 rounded-md bg-[#fafafa] border border-[#ebebeb] text-[#171717] focus:outline-hidden focus:border-[#171717] focus:bg-white font-mono text-xs cursor-pointer transition"
                >
                  {columns.map((c) => (
                    <option key={c.id} value={c.id}>
                      {c.title}
                    </option>
                  ))}
                </select>
              </div>

              <div>
                <label className="block font-mono text-[#4d4d4d] font-medium mb-1">
                  Модель агента
                </label>
                <select
                  value={selectedModel}
                  onChange={(e) => setSelectedModel(e.target.value)}
                  className="w-full px-2.5 py-1.5 rounded-md bg-[#fafafa] border border-[#ebebeb] text-[#171717] focus:outline-hidden focus:border-[#171717] focus:bg-white font-mono text-xs cursor-pointer transition"
                >
                  {(availableModels.length > 0 ? availableModels : [settings.model || 'gemini-3.8-flash-high']).map((m) => (
                    <option key={m} value={m}>
                      {m}
                    </option>
                  ))}
                </select>
              </div>

              <div className="flex items-center justify-end space-x-2 pt-3 border-t border-[#ebebeb]">
                <button
                  type="button"
                  onClick={() => setIsNewTaskModalOpen(false)}
                  className="px-3 py-1.5 rounded-md bg-white border border-[#ebebeb] text-[#171717] hover:bg-[#f4f4f5] transition cursor-pointer font-sans"
                >
                  Отмена
                </button>
                <button
                  type="submit"
                  className="px-3.5 py-1.5 rounded-md bg-[#171717] hover:bg-[#262626] text-white font-medium transition cursor-pointer font-sans shadow-xs"
                >
                  Создать
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* Create Board Modal */}
      <CreateBoardModal
        isOpen={isCreateBoardModalOpen}
        onClose={() => setIsCreateBoardModalOpen(false)}
      />
    </div>
  );
}
