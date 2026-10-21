import { useEffect, useState, useRef, useCallback } from 'react';
import { DragDropContext, Droppable, DropResult } from '@hello-pangea/dnd';
import { useBoardStore } from '../../stores/boardStore';
import { useDrawerStore } from '../../stores/drawerStore';
import { useSettingsStore } from '../../stores/settingsStore';
import { useCortexStore } from '../../stores/cortexStore';
import KanbanColumn from './KanbanColumn';
import { PipelineColumn, BoardDef, KanbanTask } from '../../types';
import { Plus, X, Layout, Pencil, Trash2, Search } from 'lucide-react';
import CreateBoardModal from './CreateBoardModal';
import PersonaSelect from '../common/PersonaSelect';
import { slugifyTitle, uniqueColumnId } from '../../lib/slug';
import { AGENT_PALETTE, getDeterministicAgentColor, isAllowedAgentColor } from '../../lib/agentPalette';
import { useTranslation } from '../../stores/languageStore';

const COLUMN_PALETTE = AGENT_PALETTE;

interface AgentPersonaOption {
  role: string;
  hasPrompt?: boolean;
  has_prompt?: boolean;
  isBuiltin?: boolean;
  is_builtin?: boolean;
  icon?: string;
  color?: string;
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
    setTasks,
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
  const { t } = useTranslation();
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

  const handleDragEnd = async (result: DropResult) => {
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

    const task = tasks.find((t) => t.id === draggableId);
    if (!task) return;

    const sourceColId = source.droppableId;
    const destColId = destination.droppableId;
    const sourceIndex = source.index;
    const destIndex = destination.index;

    const isDoneCol = (colId: string) => {
      const col = columns.find((c) => c.id === colId);
      return Boolean(
        col?.terminal ||
        colId === 'ready' ||
        col?.title.toLowerCase().includes('готов') ||
        col?.title.toLowerCase().includes('done')
      );
    };

    const firstColId = columns[0]?.id ?? 'backlog';
    const getColumnDisplayTasks = (targetColId: string) => {
      const isDone = isDoneCol(targetColId);
      const isFirst = targetColId === firstColId;
      const baseTasks = searchQuery ? filteredTasks : currentBoardTasks;
      const colTasks = baseTasks.filter((t) => {
        if (t.columnId === targetColId) return true;
        if (isFirst) {
          const colExists = columns.some((c) => c.id === t.columnId);
          return !colExists;
        }
        return false;
      });
      return isDone ? colTasks.filter((t) => !t.parentId) : colTasks;
    };

    if (sourceColId === destColId) {
      if (sourceIndex === destIndex) return;

      const colDisplayTasks = getColumnDisplayTasks(destColId);
      const otherTasks = colDisplayTasks.filter((t) => t.id !== draggableId);
      const nextTasks = [...tasks];
      const taskPos = nextTasks.findIndex((t) => t.id === draggableId);
      if (taskPos === -1) return;
      const [movedTask] = nextTasks.splice(taskPos, 1);
      const effectiveBoardId = activeBoardId || 'development';
      const updatedMovedTask: KanbanTask = {
        ...movedTask,
        boardId: effectiveBoardId,
      };

      if (destIndex < otherTasks.length) {
        const refTask = otherTasks[destIndex];
        const refPos = nextTasks.findIndex((t) => t.id === refTask.id);
        if (refPos !== -1) {
          nextTasks.splice(refPos, 0, updatedMovedTask);
        } else {
          nextTasks.push(updatedMovedTask);
        }
      } else {
        if (otherTasks.length > 0) {
          const lastTask = otherTasks[otherTasks.length - 1];
          const lastPos = nextTasks.findIndex((t) => t.id === lastTask.id);
          if (lastPos !== -1) {
            nextTasks.splice(lastPos + 1, 0, updatedMovedTask);
          } else {
            nextTasks.push(updatedMovedTask);
          }
        } else {
          nextTasks.push(updatedMovedTask);
        }
      }

      setTasks(nextTasks);
      return;
    }

    // Moving between columns with target placement:
    // @hello-pangea/dnd invariant: onDragEnd must synchronously place the card into the destination
    // column state to prevent drop animation glitches and disappearing DOM elements.
    const destDisplayTasks = getColumnDisplayTasks(destColId);
    const originalTasks = [...tasks];
    const taskPos = originalTasks.findIndex((t) => t.id === draggableId);
    if (taskPos === -1) return;

    const [movedTask] = originalTasks.splice(taskPos, 1);
    const destColDef = columns.find((c) => c.id === destColId);
    const effectiveBoardId = activeBoardId || 'development';
    const isDone = Boolean(
      destColDef?.terminal ||
      destColId === 'ready' ||
      destColDef?.title.toLowerCase().includes('готов') ||
      destColDef?.title.toLowerCase().includes('done')
    );
    const targetStatus = isDone
      ? 'done'
      : destColDef?.persona
      ? 'running'
      : 'idle';

    const updatedMovedTask: KanbanTask = {
      ...movedTask,
      columnId: destColId,
      boardId: effectiveBoardId,
      persona: destColDef?.persona || movedTask.persona,
      status: targetStatus,
      updatedAt: Date.now(),
      stageEnteredAt: Date.now(),
    };

    const nextTasks = [...originalTasks];
    if (destIndex < destDisplayTasks.length) {
      const refTask = destDisplayTasks[destIndex];
      const refPos = nextTasks.findIndex((t) => t.id === refTask.id);
      if (refPos !== -1) {
        nextTasks.splice(refPos, 0, updatedMovedTask);
      } else {
        nextTasks.push(updatedMovedTask);
      }
    } else {
      if (destDisplayTasks.length > 0) {
        const lastTask = destDisplayTasks[destDisplayTasks.length - 1];
        const lastPos = nextTasks.findIndex((t) => t.id === lastTask.id);
        if (lastPos !== -1) {
          nextTasks.splice(lastPos + 1, 0, updatedMovedTask);
        } else {
          nextTasks.push(updatedMovedTask);
        }
      } else {
        nextTasks.push(updatedMovedTask);
      }
    }

    // Synchronous state update guarantees smooth DnD transition
    setTasks(nextTasks);

    // Backend-authoritative confirmation: if backend rejects (e.g. sensor or network error),
    // safely revert local tasks state and notify the user.
    try {
      await moveTask(draggableId, destColId);
    } catch (err: unknown) {
      console.warn(`[handleDragEnd] moveTask rejected for ${draggableId}, reverting state:`, err);
      setTasks(tasks);
      const msg = err instanceof Error ? err.message : String(err);
      alert(`Не удалось переместить задачу:\n${msg}`);
    }
  };

  const savePipelineConfig = useCallback(async (next: PipelineColumn[]): Promise<boolean> => {
    try {
      await saveCurrentBoard(next);
      return true;
    } catch (err) {
      console.info('saveCurrentBoard failed, rolling back:', err);
      return false;
    }
  }, [saveCurrentBoard]);

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

  const handleCreateTask = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newTitle.trim()) return;

    const targetColumnId = columns.some((c) => c.id === 'backlog') ? 'backlog' : (columns[0]?.id ?? 'backlog');

    const created = await addTask({
      title: newTitle.trim(),
      description: newDescription.trim() || undefined,
      stageId: targetColumnId,
      boardId: activeBoardId || 'development',
      skipWorktree: !createWorktree,
      model: selectedModel,
    });
    if (!created) return;
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
    const targetPersona = personaName || 'developer';
    const matchedPersona = agentPersonas.find((p) => p.role === targetPersona);
    let columnColor = COLUMN_PALETTE[columns.length % COLUMN_PALETTE.length];
    if (id !== 'backlog' && id !== 'ready') {
      const personaColor = matchedPersona?.color || getDeterministicAgentColor(targetPersona);
      if (personaColor && isAllowedAgentColor(personaColor)) {
        columnColor = personaColor;
      }
    }
    const newCol: PipelineColumn = {
      id,
      title,
      color: columnColor,
      icon: matchedPersona?.icon || 'Layers',
      // Persona обязана сохраняться (select и свободный ввод): пустая подменяется значением по умолчанию,
      // чтобы колонка гарантированно попадала в проводник.
      persona: targetPersona,
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

  const handleDeleteColumn = useCallback(async (columnId: string) => {
    if (columnId === 'backlog' || columnId === 'ready') return;
    const { columns: curCols, tasks: curTasks, setColumns: curSetCols, moveTask: curMoveTask } = useBoardStore.getState();
    if (curCols.length <= 1) return;
    const idx = curCols.findIndex((c) => c.id === columnId);
    if (idx === -1) return;
    const col = curCols[idx];
    const neighbor = curCols[idx - 1] ?? curCols[idx + 1] ?? null;
    if (!neighbor) return;

    const doomedTasks = curTasks.filter((t) => t.columnId === columnId);
    const confirmed = window.confirm(
      doomedTasks.length === 0
        ? `Удалить колонку «${col.title}»?`
        : `Удалить колонку «${col.title}»? Задач в ней: ${doomedTasks.length}. Они будут перенесены в колонку «${neighbor.title}».`
    );
    if (!confirmed) return;

    const prev = curCols;
    const next = prev.filter((c) => c.id !== columnId);

    curSetCols(next);
    const saved = await savePipelineConfig(next);
    if (!saved) {
      curSetCols(prev);
      setColumnsError('Не удалось сохранить удаление колонки. Изменения откачены.');
      return;
    }
    for (const t of doomedTasks) {
      await curMoveTask(t.id, neighbor.id);
    }
  }, [savePipelineConfig]);

  // Filter tasks based on activeBoardId and searchQuery:
  // ZERO INVISIBLE TASKS (§ 2.18 & § 2.3): Every task must be visible on the board it belongs to,
  // or on the active board if orphaned, if its declared board is absent, or if its column belongs to the active board.
  const currentBoardTasks = tasks.filter((task) => {
    // 1. Direct match: task belongs to activeBoardId or has no boardId assigned
    if (!task.boardId || task.boardId === activeBoardId) {
      return true;
    }
    const matchingBoard = boards.find((b) => b.id === task.boardId);
    // 2. Fallback: if declared board no longer exists, show on active board
    if (!matchingBoard) {
      return true;
    }
    // 3. Self-heal: If declared board does NOT have task's column, but active board DOES,
    // show on active board (prevents vanishing tasks when moved across boards or boardId drifted).
    const boardHasCol = matchingBoard.columns.some(
      (c) => c.id === task.columnId || c.id.toLowerCase() === task.columnId?.toLowerCase()
    );
    const activeHasCol = columns.some(
      (c) => c.id === task.columnId || c.id.toLowerCase() === task.columnId?.toLowerCase()
    );
    if (!boardHasCol && activeHasCol) {
      return true;
    }
    return false;
  });

  const filteredTasks = currentBoardTasks.filter((task) => {
    if (!searchQuery) return true;
    const q = searchQuery.toLowerCase();
    return (
      task.id.toLowerCase().includes(q) ||
      task.title.toLowerCase().includes(q) ||
      (task.branch ? task.branch.toLowerCase().includes(q) : false) ||
      (task.description && task.description.toLowerCase().includes(q))
    );
  });

  return (
    <div className="flex-1 flex flex-col h-full overflow-hidden bg-[#fafafa] dark:bg-[#090a0f] transition-colors">
      {/* Board Top Toolbar */}
      <div className="flex items-center justify-between px-3.5 py-2 border-b border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#0d1117] text-xs select-none transition-colors">
        {/* Left: Search first, then board name tabs */}
        <div className="flex items-center space-x-1.5 overflow-x-auto py-0.5">
          <div className="relative w-44 sm:w-52 shrink-0">
            <Search className="absolute left-2.5 top-2 h-3.5 w-3.5 text-[#8f8f8f] dark:text-[#8b949e] pointer-events-none" />
            <input
              id="board-search"
              name="kanbanBoardSearch"
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder={t('board.searchPlaceholder')}
              className="w-full pl-8 pr-2.5 py-1 rounded-md bg-[#f9f9f9] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] text-xs text-[#171717] dark:text-[#f0f6fc] placeholder:text-[#8f8f8f] dark:placeholder:text-[#8b949e] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc] focus:bg-white dark:focus:bg-[#0d1117] font-sans transition"
            />
          </div>
          {boards.map((b) => {
            const isActive = b.id === activeBoardId;
            const isRenaming = renamingBoardId === b.id;
            const boardTasksCount = tasks.filter((t) => {
              if (t.parentId) return false;
              const hasMatchingBoard = boards.some((board) => board.id === t.boardId);
              const effectiveBoardId = hasMatchingBoard ? t.boardId : activeBoardId;
              return effectiveBoardId === b.id;
            }).length;
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
                        ? 'bg-[#171717] dark:bg-[#21262d] border-[#171717] dark:border-[#30363d] text-white shadow-xs'
                        : 'bg-white dark:bg-[#0d1117] border-[#171717] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc]'
                    }`}
                  >
                    <Layout className={`h-3.5 w-3.5 shrink-0 ${isActive ? 'text-white' : 'text-[#71717a] dark:text-[#8b949e]'}`} />
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
                          ? 'bg-[#27272a] dark:bg-[#161b22] text-white border border-[#3f3f46] dark:border-[#30363d]'
                          : 'bg-[#fafafa] dark:bg-[#161b22] text-[#171717] dark:text-[#f0f6fc] border border-[#d4d4d8] dark:border-[#30363d]'
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
                        ? 'bg-[#171717] dark:bg-[#21262d] text-white shadow-xs'
                        : 'bg-[#f4f4f5] dark:bg-[#161b22] hover:bg-[#e4e4e7] dark:hover:bg-[#21262d] text-[#52525b] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc]'
                    }`}
                    title={t('board.boardTabTitle', { name: b.name })}
                  >
                    <Layout className="h-3.5 w-3.5 shrink-0" />
                    <span className="truncate max-w-[140px]">{b.name}</span>
                    <span
                      className={`text-[10px] px-1.5 py-0.2 rounded-full font-mono font-medium shrink-0 ${
                        isActive
                          ? 'bg-[#27272a] dark:bg-[#30363d] text-[#d4d4d8] dark:text-[#f0f6fc]'
                          : 'bg-[#e4e4e7] dark:bg-[#21262d] text-[#71717a] dark:text-[#8b949e]'
                      }`}
                    >
                      {boardTasksCount}
                    </span>
                  </button>
                )}

              </div>
            );
          })}
          <button
            onClick={() => setIsCreateBoardModalOpen(true)}
            className="flex items-center space-x-1 px-2.5 py-1.5 rounded-md border border-dashed border-[#d4d4d8] dark:border-[#30363d] hover:border-[#171717] dark:hover:border-[#f0f6fc] text-[#71717a] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] text-xs font-medium transition cursor-pointer select-none"
            title={t('board.newBoardTitle')}
          >
            <Plus className="h-3 w-3" />
            <span>{t('board.newBoard')}</span>
          </button>
        </div>

        {/* Right: Board Stats & Action Buttons */}
        <div className="flex items-center space-x-3">
          <div className="hidden sm:flex items-center space-x-1.5 text-[#666666] dark:text-[#8b949e] text-xs">
            <span>{t('board.tasksOnBoard')}</span>
            <span className="text-[#171717] dark:text-[#f0f6fc] font-semibold tabular-nums font-mono">
              {currentBoardTasks.filter((t) => !t.parentId).length}
            </span>
          </div>

          <div className="flex items-center space-x-2">
            <button
              onClick={handleOpenAddColumn}
              className="flex items-center space-x-1.5 px-3 py-1.5 rounded-md bg-white dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] text-[#171717] dark:text-[#f0f6fc] font-sans text-xs font-medium transition cursor-pointer active:scale-[0.99]"
              title={t('board.addColumnTitle')}
            >
              <Plus className="h-3.5 w-3.5" />
              <span>{t('board.addColumn')}</span>
            </button>
            <button
              onClick={handleOpenAddTask}
              className="flex items-center space-x-1.5 px-3 py-1.5 rounded-md bg-[#171717] dark:bg-[#21262d] hover:bg-[#262626] dark:hover:bg-[#30363d] text-white font-sans text-xs font-medium transition cursor-pointer shadow-xs active:scale-[0.99] border border-transparent dark:border-[#30363d]"
              title={t('board.newTaskTitle')}
            >
              <Plus className="h-3.5 w-3.5 text-white" />
              <span>{t('board.newTask')}</span>
            </button>
          </div>
        </div>
      </div>

      {/* Add Column Inline Form */}
      {isAddColumnOpen && (
        <div className="px-3.5 py-2.5 border-b border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#0d1117] transition-colors">
          <form onSubmit={handleCreateColumn} className="flex flex-wrap items-end gap-x-3 gap-y-2 text-xs">
            <div>
              <label className="block font-sans text-[#4d4d4d] dark:text-[#c9d1d9] font-medium mb-1">
                {t('board.columnTitleLabel')}
              </label>
              <input
                type="text"
                required
                autoFocus
                value={newColumnTitle}
                onChange={(e) => setNewColumnTitle(e.target.value)}
                placeholder={t('board.columnTitlePlaceholder')}
                className="w-56 px-3 py-1.5 rounded-md bg-[#fafafa] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] placeholder:text-[#8f8f8f] dark:placeholder:text-[#8b949e] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc] focus:bg-white dark:focus:bg-[#0d1117] font-sans transition"
              />
            </div>

            <div>
              <label className="block font-sans text-[#4d4d4d] dark:text-[#c9d1d9] font-medium mb-1">
                {t('board.agentLabel')}
              </label>
              {usePersonaSelect ? (
                <PersonaSelect
                  value={isNewAgentSelected ? NEW_AGENT_OPTION : newColumnPersona}
                  onChange={(val) => {
                    if (val === NEW_AGENT_OPTION) {
                      setIsNewAgentSelected(true);
                    } else {
                      setIsNewAgentSelected(false);
                      setNewColumnPersona(val);
                    }
                  }}
                  disabled={personasLoading}
                  personas={agentPersonas}
                  allowNewAgent={true}
                  newAgentOptionValue={NEW_AGENT_OPTION}
                  className="w-48"
                />
              ) : (
                <input
                  type="text"
                  value={newColumnPersona}
                  onChange={(e) => setNewColumnPersona(e.target.value)}
                  placeholder={personasLoading ? t('board.loadingAgents') : 'developer'}
                  className="w-44 px-3 py-1.5 rounded-md bg-[#fafafa] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] placeholder:text-[#8f8f8f] dark:placeholder:text-[#8b949e] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc] focus:bg-white dark:focus:bg-[#0d1117] font-mono transition"
                />
              )}
            </div>

            {usePersonaSelect && isNewAgentSelected && (
              <div>
                <label className="block font-sans text-[#4d4d4d] dark:text-[#c9d1d9] font-medium mb-1">
                  {t('board.newAgentNameLabel')}
                </label>
                <input
                  type="text"
                  required
                  autoFocus
                  value={newAgentName}
                  onChange={(e) => setNewAgentName(e.target.value)}
                  placeholder={t('board.newAgentNamePlaceholder')}
                  className="w-44 px-3 py-1.5 rounded-md bg-[#fafafa] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] placeholder:text-[#8f8f8f] dark:placeholder:text-[#8b949e] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc] focus:bg-white dark:focus:bg-[#0d1117] font-sans transition"
                />
                {newAgentName.trim() !== '' && (
                  <div className="mt-1 text-[10px] font-mono text-[#8f8f8f] dark:text-[#8b949e]">
                    {t('board.newAgentRoleHint', { role: slugifyTitle(newAgentName) === 'column' ? '—' : slugifyTitle(newAgentName) })}
                  </div>
                )}
              </div>
            )}

            <div
              className="flex items-center space-x-1.5 pb-2 text-[#8f8f8f] dark:text-[#8b949e]"
              title={t('board.autoColorTitle')}
            >
              <span
                className="h-4 w-4 rounded-full border border-[#ebebeb] dark:border-[#30363d]"
                style={{ backgroundColor: COLUMN_PALETTE[columns.length % COLUMN_PALETTE.length] }}
              />
              <span>{t('board.autoColor')}</span>
            </div>

            <div className="flex items-center space-x-2 pb-0.5">
              <button
                type="submit"
                className="px-3.5 py-1.5 rounded-md bg-[#171717] dark:bg-[#21262d] hover:bg-[#262626] dark:hover:bg-[#30363d] text-white font-medium transition cursor-pointer font-sans shadow-xs border border-transparent dark:border-[#30363d]"
              >
                {t('common.create')}
              </button>
              <button
                type="button"
                onClick={() => setIsAddColumnOpen(false)}
                className="px-3 py-1.5 rounded-md bg-white dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] transition cursor-pointer font-sans"
              >
                {t('common.cancel')}
              </button>
            </div>

            {columnFormError && (
              <div className="w-full text-xs font-sans text-[#ef4444] bg-[#fef2f2] dark:bg-[#161b22] border border-[#fecaca] dark:border-[#3b1219] rounded-md px-3 py-1.5">
                {columnFormError}
              </div>
            )}
          </form>
        </div>
      )}

      {columnsError && (
        <div className="mx-3.5 mt-2 flex items-center justify-between gap-2 rounded-md border border-[#fecaca] dark:border-[#3b1219] bg-[#fef2f2] dark:bg-[#161b22] px-3 py-1.5 text-xs font-sans text-[#ef4444]">
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
                    <title>{`Нить: ${thread.parentId} -> ${thread.subtaskId} (нажмите для открытия консоли подзадачи)`}</title>
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
                      r={6}
                      fill="none"
                      stroke={thread.subtaskColor}
                      strokeWidth={1.5}
                      opacity={0.5}
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
              const firstColId = columns[0]?.id ?? 'backlog';
              const isFirstColumn = column.id === firstColId;
              const columnTasks = filteredTasks.filter((t) => {
                if (t.columnId === column.id) return true;
                if (isFirstColumn) {
                  // If a task's columnId doesn't match any column in the active board, fallback to the first column (backlog)
                  const colExists = columns.some((c) => c.id === t.columnId);
                  return !colExists;
                }
                return false;
              });
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
                  key={`${activeBoardId}-${column.id}`}
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
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 dark:bg-black/80 p-3 animate-in fade-in duration-100">
          <div className="relative flex flex-col w-[480px] max-w-full rounded-lg border border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#0d1117] shadow-md p-5 text-[#171717] dark:text-[#f0f6fc] transition-colors">
            <div className="flex items-center justify-between pb-3 border-b border-[#ebebeb] dark:border-[#21262d]">
              <h3 className="text-sm font-semibold text-[#171717] dark:text-[#f0f6fc]">
                {t('newTask.modalTitle')}
              </h3>
              <button
                onClick={() => setIsNewTaskModalOpen(false)}
                className="p-1 rounded-md text-[#666666] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#f4f4f5] dark:hover:bg-[#161b22] transition cursor-pointer"
                title={t('common.close')}
              >
                <X className="h-4 w-4" />
              </button>
            </div>

            <form onSubmit={handleCreateTask} className="mt-4 space-y-3.5 text-xs">
              <div>
                <label className="block font-sans text-[#4d4d4d] dark:text-[#c9d1d9] font-medium mb-1">
                  {t('newTask.titleLabel')}
                </label>
                <input
                  type="text"
                  required
                  value={newTitle}
                  onChange={(e) => setNewTitle(e.target.value)}
                  placeholder={t('newTask.titlePlaceholder')}
                  className="w-full px-3 py-1.5 rounded-md bg-[#fafafa] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] placeholder:text-[#8f8f8f] dark:placeholder:text-[#8b949e] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc] focus:bg-white dark:focus:bg-[#0d1117] font-sans transition"
                />
              </div>

              <div>
                <label className="block font-sans text-[#4d4d4d] dark:text-[#c9d1d9] font-medium mb-1">
                  {t('newTask.descLabel')}
                </label>
                <textarea
                  rows={2}
                  value={newDescription}
                  onChange={(e) => setNewDescription(e.target.value)}
                  placeholder={t('newTask.descPlaceholder')}
                  className="w-full px-3 py-1.5 rounded-md bg-[#fafafa] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] placeholder:text-[#8f8f8f] dark:placeholder:text-[#8b949e] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc] focus:bg-white dark:focus:bg-[#0d1117] font-sans resize-none transition"
                />
              </div>

              <div>
                <label className="block font-mono text-[#4d4d4d] dark:text-[#c9d1d9] font-medium mb-1">
                  {t('newTask.modelLabel')}
                </label>
                <select
                  value={selectedModel}
                  onChange={(e) => setSelectedModel(e.target.value)}
                  className="w-full px-2.5 py-1.5 rounded-md bg-[#fafafa] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc] focus:bg-white dark:focus:bg-[#0d1117] font-mono text-xs cursor-pointer transition"
                >
                  {Array.from(new Set([selectedModel, ...(availableModels.length > 0 ? availableModels : [settings.model || 'gemini-3.8-flash-high'])])).map((m) => (
                    <option key={m} value={m} className="dark:bg-[#161b22] dark:text-[#f0f6fc]">
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
                  className="h-3.5 w-3.5 accent-[#171717] dark:accent-[#8b949e] cursor-pointer"
                />
                <span className="font-sans text-[#4d4d4d] dark:text-[#c9d1d9]">
                  {t('newTask.createWorktree')}
                </span>
              </label>

              <div className="flex items-center justify-end space-x-2 pt-3 border-t border-[#ebebeb] dark:border-[#21262d]">
                <button
                  type="button"
                  onClick={() => setIsNewTaskModalOpen(false)}
                  className="px-3 py-1.5 rounded-md bg-white dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] transition cursor-pointer font-sans"
                >
                  {t('common.cancel')}
                </button>
                <button
                  type="submit"
                  className="px-3.5 py-1.5 rounded-md bg-[#171717] dark:bg-[#21262d] hover:bg-[#262626] dark:hover:bg-[#30363d] text-white font-medium transition cursor-pointer font-sans shadow-xs border border-transparent dark:border-[#30363d]"
                >
                  {t('common.create')}
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
            className="fixed z-50 min-w-[190px] bg-white dark:bg-[#161b22] rounded-lg shadow-md border border-[#ebebeb] dark:border-[#30363d] py-1 text-xs font-sans animate-in fade-in zoom-in-95 duration-100 select-none"
            style={{
              top: Math.min(boardContextMenu.y, window.innerHeight - 120),
              left: Math.min(boardContextMenu.x, window.innerWidth - 200),
            }}
            onClick={(e) => e.stopPropagation()}
            onContextMenu={(e) => e.preventDefault()}
          >
            <div className="px-3 py-1 text-[10px] font-semibold text-[#a1a1aa] dark:text-[#8b949e] uppercase tracking-wider border-b border-[#f4f4f5] dark:border-[#21262d] mb-1 truncate">
              {t('board.boardContextMenu', { name: targetBoard.name })}
            </div>
            <button
              onClick={() => {
                setBoardContextMenu(null);
                setRenamingBoardId(targetBoard.id);
                setRenamingBoardName(targetBoard.name);
              }}
              className="w-full flex items-center space-x-2 px-3 py-1.5 text-left text-[#171717] dark:text-[#f0f6fc] hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] transition cursor-pointer"
            >
              <Pencil className="h-3.5 w-3.5 text-[#71717a] dark:text-[#8b949e]" />
              <span>{t('board.renameBoard')}</span>
            </button>
            <button
              onClick={() => {
                setBoardContextMenu(null);
                if (boards.length <= 1) return;
                if (
                  window.confirm(
                    t('board.confirmDeleteBoard', { name: targetBoard.name })
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
            >
              <Trash2 className="h-3.5 w-3.5" />
              <span>{t('board.deleteBoard')}</span>
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
