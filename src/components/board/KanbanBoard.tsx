import { useEffect, useState, useRef, useCallback } from 'react';
import { DragDropContext, Droppable, DropResult } from '@hello-pangea/dnd';
import { useBoardStore } from '../../stores/boardStore';
import { useDrawerStore } from '../../stores/drawerStore';
import { useSettingsStore } from '../../stores/settingsStore';
import { useCortexStore } from '../../stores/cortexStore';
import KanbanColumn from './KanbanColumn';
import { PipelineColumn, BoardDef } from '../../types';
import { Plus, X, Layout, Pencil, Trash2, Search } from 'lucide-react';
import CreateBoardModal from './CreateBoardModal';
import { slugifyTitle, uniqueColumnId } from '../../lib/slug';

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

interface AgentPersonaOption {
  role: string;
  hasPrompt?: boolean;
  has_prompt?: boolean;
  isBuiltin?: boolean;
  is_builtin?: boolean;
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
    setSearchQuery,
    moveTask,
    addTask,
    tickTimers,
    setColumns,
    hoveredTaskId,
    setHoveredTaskId,
    boards,
    activeBoardId,
    setActiveBoard,
    saveBoard,
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
  const [selectedModel, setSelectedModel] = useState<string>(settings.model || 'gemini-3.8-flash-high');

  // New task form fields
  const [newTitle, setNewTitle] = useState('');
  const [newDescription, setNewDescription] = useState('');
  const [createWorktree, setCreateWorktree] = useState(true);

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
  const [columnsError, setColumnsError] = useState<string | null>(null);

  // Board tab context menu and inline renaming state
  interface BoardContextMenuState {
    x: number;
    y: number;
    boardId: string;
  }
  const [boardContextMenu, setBoardContextMenu] = useState<BoardContextMenuState | null>(null);
  const [renamingBoardId, setRenamingBoardId] = useState<string | null>(null);
  const [renamingBoardName, setRenamingBoardName] = useState('');

  useEffect(() => {
    if (!boardContextMenu) return;
    const handleClose = () => setBoardContextMenu(null);
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setBoardContextMenu(null);
    };
    window.addEventListener('click', handleClose);
    window.addEventListener('contextmenu', handleClose);
    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('click', handleClose);
      window.removeEventListener('contextmenu', handleClose);
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [boardContextMenu]);

  const handleCommitRename = async (board: BoardDef) => {
    const trimmed = renamingBoardName.trim();
    setRenamingBoardId(null);
    if (!trimmed || trimmed === board.name) return;
    try {
      await saveBoard({ ...board, name: trimmed });
    } catch (err) {
      console.info('saveBoard failed for rename:', err);
    }
  };

  const handleTabContextMenu = (e: React.MouseEvent, boardId: string) => {
    e.preventDefault();
    e.stopPropagation();
    setBoardContextMenu({
      x: e.clientX,
      y: e.clientY,
      boardId,
    });
  };

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
      const sourceCol = columns[source.index];
      if (!sourceCol) return;
      // Закреплены только backlog/ready/terminal по id/флагу (без привязки к индексу).
      if (
        sourceCol.id === 'backlog' ||
        sourceCol.id === 'ready' ||
        Boolean(sourceCol.terminal)
      ) {
        return;
      }
      // Бэклог строго первый.
      if (destination.index === 0) return;

      // Хвост закреплённых: первая terminal-колонка, иначе ready, иначе конец.
      // Бэкенд нормализует ready-last по id — фронт совпадает до сейва.
      const terminalIdx = columns.findIndex((c) => Boolean(c.terminal));
      const readyIdx = columns.findIndex((c) => c.id === 'ready');
      const tailStart =
        terminalIdx !== -1 ? terminalIdx : readyIdx !== -1 ? readyIdx : columns.length;
      // Источник в хвосте быть не должен (закреплён выше), назначение в хвост — зажать.
      if (source.index >= tailStart) return;
      const maxDest = Math.max(1, tailStart - 1);
      const clampedDestinationIndex = Math.max(1, Math.min(destination.index, maxDest));

      void handleReorderColumns(source.index, clampedDestinationIndex);
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
    if (sourceIndex === destinationIndex) return;
    if (columns.length <= 2) return;

    const prev = columns;
    const sourceCol = prev[sourceIndex];
    if (!sourceCol) return;
    // Закреплены только backlog/ready/terminal по id/флагу.
    if (sourceCol.id === 'backlog' || sourceCol.id === 'ready' || Boolean(sourceCol.terminal)) {
      return;
    }
    if (destinationIndex <= 0) return;

    setColumnsError(null);
    const reordered = Array.from(prev);
    const [moved] = reordered.splice(sourceIndex, 1);
    if (!moved) return;
    // Назначение в хвост закреплённых — зажать перед хвостом, нормализация ниже всё равно удержит ready последней.
    const terminalIdx = prev.findIndex((c) => Boolean(c.terminal));
    const readyIdx = prev.findIndex((c) => c.id === 'ready');
    const tailStart =
      terminalIdx !== -1 ? terminalIdx : readyIdx !== -1 ? readyIdx : prev.length;
    const maxDest = Math.max(1, tailStart - 1);
    const safeDest = Math.max(1, Math.min(destinationIndex, maxDest));
    reordered.splice(safeDest, 0, moved);

    // backlog строго первая, ready строго последняя, остальные — в drop-порядке.
    // Совпадает с бэкенд-нормализацией (backlog-first/ready-last по id) до сейва.
    const prevLast = prev[prev.length - 1];
    const backlog = reordered.find((c) => c.id === 'backlog') ?? prev[0];
    const ready = reordered.find((c) => c.id === 'ready') ?? prevLast;
    const middle = reordered.filter((c) => c.id !== backlog.id && c.id !== ready.id);
    const next = [backlog, ...middle, ready];

    if (prev.length === next.length && prev.every((c, i) => c.id === next[i]?.id)) return;

    setColumns(next);
    const saved = await savePipelineConfig(next);
    if (!saved) {
      setColumns(prev);
      setColumnsError('Не удалось сохранить порядок колонок. Изменения откачены.');
    }
  };

  // Создание задач — только через общую кнопку доски: цель backlog, иначе первая колонка.
  // Создание идёт через бэкенд create_task (on_enter-хуки + worktree на старте).
  const handleOpenAddTask = () => {
    setNewTitle('');
    setNewDescription('');
    setCreateWorktree(true);
    setSelectedModel(settings.model || (availableModels.length > 0 ? availableModels[0] : 'gemini-3.8-flash-high'));
    setIsNewTaskModalOpen(true);
  };

  const handleCreateTask = (e: React.FormEvent) => {
    e.preventDefault();
    if (!newTitle.trim()) return;

    const targetColumnId = columns.some((c) => c.id === 'backlog') ? 'backlog' : (columns[0]?.id ?? 'backlog');

    void addTask({
      title: newTitle.trim(),
      description: newDescription.trim() || undefined,
      stageId: targetColumnId,
      boardId: activeBoardId || 'development',
      skipWorktree: !createWorktree,
      model: selectedModel,
    });
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
      // Кириллица преобразуется в латиницу автоматически (бэкенд принимает только [a-z0-9-]).
      const role = slugifyTitle(newAgentName);
      if (!role || role === 'column') {
        setColumnFormError('Имя должно содержать буквы или цифры.');
        return;
      }
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('create_agent_persona', { role });
        await useCortexStore.getState().loadAgentPersonas();
        await loadAgentPersonas();
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
      // Persona обязана сохраняться (select и свободный ввод): пустая подменяется значением по умолчанию,
      // чтобы колонка гарантированно попадала в проводник.
      persona: personaName || 'developer',
      contextWindow: 128000,
      modelOverride: null,
      hooks: {},
    };

    const prev = columns;
    // Вставка ПЕРЕД первой terminal-колонкой; fallback — перед id 'ready', иначе в конец.
    // Совпадает с бэкенд-нормализацией ready-last по id до сейва.
    const terminalIdx = prev.findIndex((c) => Boolean(c.terminal));
    const readyIdx = prev.findIndex((c) => c.id === 'ready');
    const insertIdx =
      terminalIdx !== -1 ? terminalIdx : readyIdx !== -1 ? readyIdx : prev.length;
    const next = [...prev.slice(0, insertIdx), newCol, ...prev.slice(insertIdx)];

    setColumns(next);
    useCortexStore.getState().syncStagesFromColumns(next);
    const saved = await savePipelineConfig(next);
    if (!saved) {
      setColumns(prev);
      useCortexStore.getState().syncStagesFromColumns(prev);
      setColumnFormError('Не удалось сохранить колонку. Изменения откачены.');
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
      setColumnsError('Не удалось сохранить удаление колонки. Изменения откачены.');
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
        {/* Left: Search first, then board name tabs */}
        <div className="flex items-center space-x-1.5 overflow-x-auto py-0.5">
          <div className="relative w-44 sm:w-52 shrink-0">
            <Search className="absolute left-2.5 top-2 h-3.5 w-3.5 text-[#8f8f8f] pointer-events-none" />
            <input
              id="board-search"
              name="kanbanBoardSearch"
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Поиск..."
              className="w-full pl-8 pr-2.5 py-1 rounded-md bg-[#f9f9f9] border border-[#ebebeb] text-xs text-[#171717] placeholder:text-[#8f8f8f] focus:outline-hidden focus:border-[#171717] focus:bg-white font-sans transition"
            />
          </div>
          {boards.map((b) => {
            const isActive = b.id === activeBoardId;
            const isRenaming = renamingBoardId === b.id;
            const boardTasksCount = tasks.filter((t) => (t.boardId || 'development') === b.id).length;
            return (
              <div
                key={b.id}
                className="group relative flex items-center"
                onContextMenu={(e) => handleTabContextMenu(e, b.id)}
              >
                {isRenaming ? (
                  <div
                    className={`flex items-center space-x-1 px-2 py-1 rounded-md text-xs font-medium border ${
                      isActive
                        ? 'bg-[#171717] border-[#171717] text-white shadow-xs'
                        : 'bg-white border-[#171717] text-[#171717]'
                    }`}
                  >
                    <Layout className={`h-3.5 w-3.5 shrink-0 ${isActive ? 'text-white' : 'text-[#71717a]'}`} />
                    <input
                      type="text"
                      autoFocus
                      value={renamingBoardName}
                      onChange={(e) => setRenamingBoardName(e.target.value)}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter') {
                          e.preventDefault();
                          e.stopPropagation();
                          void handleCommitRename(b);
                        } else if (e.key === 'Escape') {
                          e.preventDefault();
                          e.stopPropagation();
                          setRenamingBoardId(null);
                        }
                      }}
                      onBlur={() => {
                        void handleCommitRename(b);
                      }}
                      onClick={(e) => e.stopPropagation()}
                      className={`px-1.5 py-0.5 rounded text-xs outline-hidden w-28 sm:w-36 font-sans ${
                        isActive
                          ? 'bg-[#27272a] text-white border border-[#3f3f46]'
                          : 'bg-[#fafafa] text-[#171717] border border-[#d4d4d8]'
                      }`}
                    />
                  </div>
                ) : (
                  <button
                    onClick={() => setActiveBoard(b.id)}
                    onDoubleClick={(e) => {
                      e.stopPropagation();
                      setRenamingBoardId(b.id);
                      setRenamingBoardName(b.name);
                    }}
                    className={`flex items-center space-x-1.5 px-3 py-1.5 rounded-md text-xs font-medium transition cursor-pointer select-none ${
                      isActive
                        ? 'bg-[#171717] text-white shadow-xs'
                        : 'bg-[#f4f4f5] hover:bg-[#e4e4e7] text-[#52525b] hover:text-[#171717]'
                    }`}
                    title={`Доска: ${b.name} (ПКМ — контекстное меню, Двойной клик — переименовать)`}
                  >
                    <Layout className="h-3.5 w-3.5 shrink-0" />
                    <span className="truncate max-w-[140px]">{b.name}</span>
                    <span
                      className={`text-[10px] px-1.5 py-0.2 rounded-full font-mono font-medium shrink-0 ${
                        isActive
                          ? 'bg-[#27272a] text-[#d4d4d8]'
                          : 'bg-[#e4e4e7] text-[#71717a]'
                      }`}
                    >
                      {boardTasksCount}
                    </span>
                  </button>
                )}
                {!isRenaming && boards.length > 1 && (
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
                  {agentPersonas.map((p) => {
                    const isBuiltin = p.role === 'cvh-dev' || Boolean(p.isBuiltin || p.is_builtin);
                    return (
                      <option key={p.role} value={p.role}>
                        {isBuiltin ? `${p.role} (Встроенный)` : p.role}
                      </option>
                    );
                  })}
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
                  className="w-44 px-3 py-1.5 rounded-md bg-[#fafafa] border border-[#ebebeb] text-[#171717] placeholder:text-[#8f8f8f] focus:outline-hidden focus:border-[#171717] focus:bg-white font-sans transition"
                />
                {newAgentName.trim() !== '' && (
                  <div className="mt-1 text-[10px] font-mono text-[#8f8f8f]">
                    Роль: {slugifyTitle(newAgentName) === 'column' ? '— нужны буквы или цифры —' : slugifyTitle(newAgentName)}
                  </div>
                )}
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

      {columnsError && (
        <div className="mx-3.5 mt-2 flex items-center justify-between gap-2 rounded-md border border-red-200 bg-red-50 px-3 py-1.5 text-xs font-sans text-red-700">
          <span>{columnsError}</span>
          <button
            type="button"
            onClick={() => setColumnsError(null)}
            className="p-1 rounded-md hover:bg-red-100 transition cursor-pointer"
            title="Закрыть"
          >
            <X className="h-3.5 w-3.5" />
          </button>
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
              // Закреплены только backlog/ready/terminal по id/флагу.
              // index===0 — лишь fallback, когда id бэклога неизвестен (нет колонки backlog).
              const pinnedById =
                column.id === 'backlog' ||
                column.id === 'ready' ||
                Boolean(column.terminal);
              const isBacklogFallback =
                index === 0 &&
                !pinnedById &&
                !columns.some((c) => c.id === 'backlog');
              const isPinned = pinnedById || isBacklogFallback;

              return (
                <KanbanColumn
                  key={column.id}
                  column={column}
                  tasks={columnTasks}
                  index={index}
                  totalColumns={columns.length}
                  isPinned={isPinned}
                  onDeleteColumn={columns.length > 1 && !isPinned ? handleDeleteColumn : undefined}
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

              <label className="flex items-center space-x-2 cursor-pointer select-none">
                <input
                  type="checkbox"
                  checked={createWorktree}
                  onChange={(e) => setCreateWorktree(e.target.checked)}
                  className="h-3.5 w-3.5 accent-[#171717] cursor-pointer"
                />
                <span className="font-sans text-[#4d4d4d]">
                  Создать worktree при старте
                </span>
              </label>

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

      {/* Board Tab Context Menu (ПКМ) */}
      {boardContextMenu && (() => {
        const targetBoard = boards.find((b) => b.id === boardContextMenu.boardId);
        if (!targetBoard) return null;
        return (
          <div
            className="fixed z-50 min-w-[190px] bg-white rounded-lg shadow-xl border border-[#ebebeb] py-1 text-xs font-sans animate-in fade-in zoom-in-95 duration-100 select-none"
            style={{
              top: Math.min(boardContextMenu.y, window.innerHeight - 120),
              left: Math.min(boardContextMenu.x, window.innerWidth - 200),
            }}
            onClick={(e) => e.stopPropagation()}
            onContextMenu={(e) => e.preventDefault()}
          >
            <div className="px-3 py-1 text-[10px] font-semibold text-[#a1a1aa] uppercase tracking-wider border-b border-[#f4f4f5] mb-1 truncate">
              Доска: {targetBoard.name}
            </div>
            <button
              onClick={() => {
                setBoardContextMenu(null);
                setRenamingBoardId(targetBoard.id);
                setRenamingBoardName(targetBoard.name);
              }}
              className="w-full flex items-center space-x-2 px-3 py-1.5 text-left text-[#171717] hover:bg-[#f4f4f5] transition cursor-pointer"
            >
              <Pencil className="h-3.5 w-3.5 text-[#71717a]" />
              <span>Переименовать доску</span>
            </button>
            <button
              onClick={() => {
                setBoardContextMenu(null);
                if (boards.length <= 1) return;
                if (
                  window.confirm(
                    `Удалить доску «${targetBoard.name}»? Все задачи этой доски будут перенесены на активную доску.`
                  )
                ) {
                  void deleteBoard(targetBoard.id);
                }
              }}
              disabled={boards.length <= 1}
              className={`w-full flex items-center space-x-2 px-3 py-1.5 text-left transition ${
                boards.length <= 1
                  ? 'text-[#a1a1aa] cursor-not-allowed opacity-50'
                  : 'text-red-600 hover:bg-red-50 hover:text-red-700 cursor-pointer'
              }`}
              title={boards.length <= 1 ? 'Нельзя удалить единственную доску' : undefined}
            >
              <Trash2 className="h-3.5 w-3.5" />
              <span>Удалить доску</span>
            </button>
          </div>
        );
      })()}

      {/* Create Board Modal */}
      <CreateBoardModal
        isOpen={isCreateBoardModalOpen}
        onClose={() => setIsCreateBoardModalOpen(false)}
      />
    </div>
  );
}
