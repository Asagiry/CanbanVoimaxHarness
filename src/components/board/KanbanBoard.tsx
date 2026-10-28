import { useEffect, useState, useRef, useCallback, useMemo } from 'react';
import { DragDropContext, Droppable, DropResult } from '@hello-pangea/dnd';
import { useBoardStore, isDoneColumnDef, sortDoneTasksByCompletionDateDesc } from '../../stores/boardStore';
import { useSettingsStore } from '../../stores/settingsStore';
import { useCortexStore } from '../../stores/cortexStore';
import KanbanColumn from './KanbanColumn';
import { PipelineColumn, BoardDef, KanbanTask } from '../../types';
import { Plus, X, Layout, Pencil, Trash2, Search, Paperclip, ImagePlus, ZoomIn } from 'lucide-react';
import CreateBoardModal from './CreateBoardModal';
import PersonaSelect from '../common/PersonaSelect';
import ImageLightboxModal from '../drawer/ImageLightboxModal';
import { compressImageFile } from '../../lib/imageCompression';
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

export default function KanbanBoard() {
  const {
    columns,
    tasks,
    searchQuery,
    setSearchQuery,
    setTasks,
    reorderTask,
    addTask,
    tickTimers,
    setColumns,
    boards,
    activeBoardId,
    setActiveBoard,
    saveBoard,
    deleteBoard,
    saveCurrentBoard,
  } = useBoardStore();
  const { availableModels, settings } = useSettingsStore();
  const { t } = useTranslation();

  const [isNewTaskModalOpen, setIsNewTaskModalOpen] = useState(false);
  const [isCreateBoardModalOpen, setIsCreateBoardModalOpen] = useState(false);
  const [selectedModel, setSelectedModel] = useState<string>(settings.model || 'gemini-3.8-flash-high');

  // New task form fields
  const [newTitle, setNewTitle] = useState('');
  const [newDescription, setNewDescription] = useState('');
  const [newAttachments, setNewAttachments] = useState<{ id: string; name: string; dataUrl: string }[]>([]);
  const [previewLightbox, setPreviewLightbox] = useState<{ src: string; alt?: string } | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

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
      // Закреплены только intake/terminal по id/флагу/позиции.
      if (
        sourceCol.isIntake ||
        Boolean(sourceCol.terminal) ||
        sourceCol.id === 'backlog' ||
        sourceCol.id === 'ready' ||
        source.index === 0 ||
        source.index === columns.length - 1
      ) {
        return;
      }
      // Intake строго первый.
      if (destination.index === 0) return;

      // Хвост закреплённых: первая terminal-колонка, иначе ready, иначе конец.
      const terminalIdx = columns.findIndex((c) => Boolean(c.terminal));
      const readyIdx = columns.findIndex((c) => c.id === 'ready');
      const tailStart =
        terminalIdx !== -1 ? terminalIdx : readyIdx !== -1 ? readyIdx : columns.length - 1;
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
      return isDoneColumnDef(col);
    };

    const firstColId = columns.find((c) => c.isIntake)?.id ?? columns[0]?.id ?? 'backlog';
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
      return isDone
        ? colTasks
            .filter((t) => !t.parentId)
            .slice()
            .sort(sortDoneTasksByCompletionDateDesc)
        : colTasks;
    };

    if (sourceColId === destColId) {
      if (sourceIndex === destIndex) return;
      if (isDoneCol(destColId)) {
        // Done column tasks are strictly sorted by completion date (newest first).
        // Manual reordering within Done column is a no-op to preserve chronological ordering.
        return;
      }

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

      // Backend-authoritative reordering: persist new index to SQLite position
      try {
        await reorderTask(draggableId, destColId, destIndex);
      } catch (err: unknown) {
        console.warn(`[handleDragEnd] reorderTask rejected for ${draggableId}, reverting state:`, err);
        setTasks(tasks);
        const msg = err instanceof Error ? err.message : String(err);
        alert(t('board.errMoveTask', { msg }));
      }
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
    const isDone = isDoneCol(destColId);
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
      completedAt: isDone ? (movedTask.completedAt || Date.now()) : undefined,
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

    // Backend-authoritative confirmation: reorder_task moves stage and updates SQLite position
    try {
      await reorderTask(draggableId, destColId, destIndex);
    } catch (err: unknown) {
      console.warn(`[handleDragEnd] reorderTask rejected for ${draggableId}, reverting state:`, err);
      setTasks(tasks);
      const msg = err instanceof Error ? err.message : String(err);
      alert(t('board.errMoveTask', { msg }));
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
    // Закреплены только intake/terminal по id/флагу/индексу.
    if (
      sourceCol.isIntake ||
      Boolean(sourceCol.terminal) ||
      sourceCol.id === 'backlog' ||
      sourceCol.id === 'ready' ||
      sourceIndex === 0 ||
      sourceIndex === prev.length - 1
    ) {
      return;
    }
    if (destinationIndex <= 0) return;

    setColumnsError(null);
    const reordered = Array.from(prev);
    const [moved] = reordered.splice(sourceIndex, 1);
    if (!moved) return;
    // Назначение в хвост закреплённых — зажать перед хвостом.
    const terminalIdx = prev.findIndex((c) => Boolean(c.terminal));
    const readyIdx = prev.findIndex((c) => c.id === 'ready');
    const tailStart =
      terminalIdx !== -1 ? terminalIdx : readyIdx !== -1 ? readyIdx : prev.length - 1;
    const maxDest = Math.max(1, tailStart - 1);
    const safeDest = Math.max(1, Math.min(destinationIndex, maxDest));
    reordered.splice(safeDest, 0, moved);

    // Intake строго первая, terminal строго последняя, остальные — в drop-порядке.
    const intake = reordered.find((c) => c.isIntake || c.id === 'backlog') ?? prev[0];
    const terminal = reordered.slice().reverse().find((c) => c.terminal || c.id === 'ready') ?? prev[prev.length - 1];
    const middle = reordered.filter((c) => c.id !== intake.id && c.id !== terminal.id);
    const next = [intake, ...middle, terminal];

    if (prev.length === next.length && prev.every((c, i) => c.id === next[i]?.id)) return;

    setColumns(next);
    const saved = await savePipelineConfig(next);
    if (!saved) {
      setColumns(prev);
      setColumnsError(t('board.errReorderRollback'));
    }
  };

  // Создание задач — только через общую кнопку доски: цель backlog, иначе первая колонка.
  // Создание идёт через бэкенд create_task (on_enter-хуки + worktree на старте).
  const handleOpenAddTask = () => {
    setNewTitle('');
    setNewDescription('');
    setNewAttachments([]);
    setSelectedModel(settings.model || (availableModels.length > 0 ? availableModels[0] : 'gemini-3.8-flash-high'));
    setIsNewTaskModalOpen(true);
  };

  const handleCloseAddTask = () => {
    setIsNewTaskModalOpen(false);
    setNewAttachments([]);
    setPreviewLightbox(null);
  };

  const processAndAddImageFiles = useCallback(async (files: File[] | FileList) => {
    const list = Array.from(files).filter(
      (f) => f.type.startsWith('image/') || /\.(png|jpe?g|webp|gif|svg)$/i.test(f.name)
    );
    if (list.length === 0) return;

    try {
      const compressedList = await Promise.all(
        list.map(async (file, idx) => {
          try {
            const dataUrl = await compressImageFile(file, 1568, 0.85);
            return {
              id: `${Date.now()}-${Math.random().toString(36).substring(2, 7)}`,
              name: file.name || `image-${idx + 1}.png`,
              dataUrl,
            };
          } catch (err) {
            console.error('[KanbanBoard] Failed to compress image:', file.name, err);
            return null;
          }
        })
      );

      const successful = compressedList.filter(
        (item): item is { id: string; name: string; dataUrl: string } => item !== null
      );
      if (successful.length > 0) {
        setNewAttachments((prev) => [...prev, ...successful]);
      }
    } catch (err) {
      console.error('[KanbanBoard] Error adding images:', err);
    }
  }, []);

  const handleModalPaste = useCallback(
    (e: React.ClipboardEvent) => {
      const items = e.clipboardData?.items;
      if (!items) return;

      const pastedImageFiles: File[] = [];
      for (let i = 0; i < items.length; i++) {
        const item = items[i];
        if (item.type.startsWith('image/')) {
          const file = item.getAsFile();
          if (file) {
            pastedImageFiles.push(file);
          }
        }
      }

      if (pastedImageFiles.length > 0) {
        e.preventDefault();
        void processAndAddImageFiles(pastedImageFiles);
      }
    },
    [processAndAddImageFiles]
  );

  const handleModalKeyDown = useCallback(
    async (e: React.KeyboardEvent) => {
      const isV =
        e.key === 'v' ||
        e.key === 'V' ||
        e.key === 'м' ||
        e.key === 'М' ||
        e.code === 'KeyV';

      if ((e.ctrlKey || e.metaKey) && isV) {
        const target = e.target as HTMLElement | null;
        const isTextInput = target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA');

        if (!isTextInput && typeof navigator !== 'undefined' && navigator.clipboard?.read) {
          try {
            const clipboardItems = await navigator.clipboard.read();
            const files: File[] = [];
            for (const item of clipboardItems) {
              const imageType = item.types.find((t) => t.startsWith('image/'));
              if (imageType) {
                const blob = await item.getType(imageType);
                files.push(
                  new File([blob], `screenshot-${Date.now()}.${imageType.split('/')[1] || 'png'}`, {
                    type: imageType,
                  })
                );
              }
            }
            if (files.length > 0) {
              e.preventDefault();
              void processAndAddImageFiles(files);
            }
          } catch (err) {
            console.debug('[KanbanBoard] Clipboard read fallback note:', err);
          }
        }
      }
    },
    [processAndAddImageFiles]
  );

  const handleFileInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.files && e.target.files.length > 0) {
      void processAndAddImageFiles(e.target.files);
    }
    e.target.value = '';
  };

  const handleRemoveAttachment = (id: string) => {
    setNewAttachments((prev) => prev.filter((a) => a.id !== id));
  };

  const handleCreateTask = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newTitle.trim()) return;

    const intakeCol = columns.find((c) => c.isIntake);
    const targetColumnId =
      intakeCol?.id ||
      (columns.some((c) => c.id === 'backlog') ? 'backlog' : (columns[0]?.id ?? 'backlog'));

    const created = await addTask({
      title: newTitle.trim(),
      description: newDescription.trim() || undefined,
      stageId: targetColumnId,
      boardId: activeBoardId || 'development',
      model: selectedModel,
      attachments: newAttachments.map((a) => a.dataUrl),
    });
    if (!created) return;
    handleCloseAddTask();
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
        setColumnFormError(t('board.errNameLettersOrDigits'));
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
      setColumnFormError(t('board.errSaveColumnRollback'));
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
    const { columns: curCols, tasks: curTasks, setColumns: curSetCols, moveTask: curMoveTask } = useBoardStore.getState();
    const idx = curCols.findIndex((c) => c.id === columnId);
    if (idx === -1) return;
    const col = curCols[idx];
    if (col.isIntake || col.terminal || col.id === 'backlog' || col.id === 'ready' || idx === 0 || idx === curCols.length - 1) return;
    if (curCols.length <= 1) return;
    const neighbor = curCols[idx - 1] ?? curCols[idx + 1] ?? null;
    if (!neighbor) return;

    const doomedTasks = curTasks.filter((t) => t.columnId === columnId);
    const confirmed = window.confirm(
      doomedTasks.length === 0
        ? t('board.confirmDeleteColumnEmpty', { title: col.title })
        : t('board.confirmDeleteColumnWithTasks', {
            title: col.title,
            count: doomedTasks.length,
            neighbor: neighbor.title,
          })
    );
    if (!confirmed) return;

    const prev = curCols;
    const next = prev.filter((c) => c.id !== columnId);

    curSetCols(next);
    const saved = await savePipelineConfig(next);
    if (!saved) {
      curSetCols(prev);
      setColumnsError(t('board.errDeleteColumnRollback'));
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

  const doneParentIds = useMemo(() => {
    const set = new Set<string>();
    for (const t of tasks) {
      if (t.status === 'done' || t.columnId === 'ready') {
        set.add(t.id);
      }
    }
    return set;
  }, [tasks]);

  const handleBoardWheel = useCallback((e: React.WheelEvent<HTMLDivElement>) => {
    // Zooming via Ctrl/Meta + wheel is handled globally by useUiZoom
    if (e.ctrlKey || e.metaKey) {
      return;
    }

    // Horizontal gestures (trackpad, tilt-wheel, or browser-remapped Shift+wheel)
    // are handled natively by the container's overflow-x-auto
    if (Math.abs(e.deltaX) > 0) {
      return;
    }

    // Explicit horizontal scroll via Shift + Wheel
    if (e.shiftKey && e.deltaY !== 0) {
      e.currentTarget.scrollLeft += e.deltaY;
      return;
    }

    // Ignore events originating inside any Kanban column (defense-in-depth)
    const target = e.target as HTMLElement | null;
    if (target?.closest('[data-kanban-column]')) {
      return;
    }

    // Scroll board horizontally only when scrolling over free canvas / board background
    if (e.deltaY !== 0) {
      e.currentTarget.scrollLeft += e.deltaY;
    }
  }, []);

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
                    aria-label={t('board.boardTabTitle', { name: b.name })}
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
            aria-label={t('board.newBoardTitle')}
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
              aria-label={t('board.addColumnTitle')}
            >
              <Plus className="h-3.5 w-3.5" />
              <span>{t('board.addColumn')}</span>
            </button>
            <button
              onClick={handleOpenAddTask}
              className="flex items-center space-x-1.5 px-3 py-1.5 rounded-md bg-[#171717] dark:bg-[#21262d] hover:bg-[#262626] dark:hover:bg-[#30363d] text-white font-sans text-xs font-medium transition cursor-pointer shadow-xs active:scale-[0.99] border border-transparent dark:border-[#30363d]"
              aria-label={t('board.newTaskTitle')}
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
            aria-label={t('common.close')}
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
          ref={provided.innerRef}
          {...provided.droppableProps}
          onWheel={handleBoardWheel}
          className="flex-1 overflow-x-auto overflow-y-hidden p-3.5 relative"
        >
          <div className="flex items-start space-x-3.5 h-full min-w-max pb-1">
            {columns.map((column, index) => {
              const firstColId = columns.find((c) => c.isIntake)?.id ?? columns[0]?.id ?? 'backlog';
              const isFirstColumn = column.id === firstColId;
              const columnTasks = filteredTasks.filter((t) => {
                if (t.columnId === column.id) return true;
                if (isFirstColumn) {
                  // If a task's columnId doesn't match any column in the active board, fallback to the first column (intake)
                  const colExists = columns.some((c) => c.id === t.columnId);
                  return !colExists;
                }
                return false;
              });
              // Закреплены intake и terminal колонки (колонка 0 и последняя, либо по флагам)
              const isPinned =
                column.isIntake ||
                Boolean(column.terminal) ||
                column.id === 'backlog' ||
                column.id === 'ready' ||
                index === 0 ||
                index === columns.length - 1;

              return (
                <KanbanColumn
                  key={`${activeBoardId}-${column.id}`}
                  column={column}
                  tasks={columnTasks}
                  index={index}
                  totalColumns={columns.length}
                  isPinned={isPinned}
                  doneParentIds={doneParentIds}
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
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 dark:bg-black/80 p-3 animate-in fade-in duration-100"
          onKeyDown={handleModalKeyDown}
        >
          <div
            className="relative flex flex-col w-[620px] max-w-[95vw] max-h-[90vh] rounded-lg border border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#0d1117] shadow-md p-5 text-[#171717] dark:text-[#f0f6fc] transition-colors overflow-hidden"
            onPaste={handleModalPaste}
          >
            <div className="flex items-center justify-between pb-3 border-b border-[#ebebeb] dark:border-[#21262d] shrink-0">
              <h3 className="text-sm font-semibold text-[#171717] dark:text-[#f0f6fc]">
                {t('newTask.modalTitle')}
              </h3>
              <button
                type="button"
                onClick={handleCloseAddTask}
                className="p-1 rounded-md text-[#666666] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#f4f4f5] dark:hover:bg-[#161b22] transition cursor-pointer"
                aria-label={t('common.close')}
              >
                <X className="h-4 w-4" />
              </button>
            </div>

            <form
              onSubmit={handleCreateTask}
              className="mt-4 flex flex-col flex-1 min-h-0 overflow-y-auto space-y-4 text-xs pr-1"
            >
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
                  rows={6}
                  value={newDescription}
                  onChange={(e) => setNewDescription(e.target.value)}
                  placeholder={t('newTask.descPlaceholder')}
                  className="w-full min-h-[130px] px-3 py-2 rounded-md bg-[#fafafa] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc] placeholder:text-[#8f8f8f] dark:placeholder:text-[#8b949e] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f0f6fc] focus:bg-white dark:focus:bg-[#0d1117] font-sans leading-relaxed resize-y transition"
                />
              </div>

              {/* Attachments Section */}
              <div>
                <div className="flex items-center justify-between mb-1.5">
                  <label className="block font-sans text-[#4d4d4d] dark:text-[#c9d1d9] font-medium">
                    {t('newTask.attachmentsLabel')}
                  </label>
                  <button
                    type="button"
                    onClick={() => fileInputRef.current?.click()}
                    className="inline-flex items-center gap-1.5 px-2 py-0.5 rounded text-[11px] font-sans text-[#4d4d4d] dark:text-[#c9d1d9] bg-[#fafafa] dark:bg-[#161b22] hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] border border-[#ebebeb] dark:border-[#30363d] transition cursor-pointer"
                  >
                    <ImagePlus className="h-3 w-3" />
                    <span>{t('newTask.pasteHint')}</span>
                  </button>
                  <input
                    ref={fileInputRef}
                    type="file"
                    accept="image/*"
                    multiple
                    className="hidden"
                    onChange={handleFileInputChange}
                  />
                </div>

                {/* Thumbnail gallery */}
                {newAttachments.length > 0 ? (
                  <div className="flex flex-wrap gap-2.5 p-2 rounded-md border border-[#ebebeb] dark:border-[#30363d] bg-[#fafafa] dark:bg-[#161b22]/50 max-h-40 overflow-y-auto">
                    {newAttachments.map((att, idx) => (
                      <div
                        key={att.id}
                        className="group relative flex flex-col items-center w-24 rounded-md overflow-hidden border border-[#ebebeb] dark:border-[#30363d] bg-white dark:bg-[#0d1117] shadow-2xs hover:border-[#171717] dark:hover:border-[#f0f6fc] transition"
                      >
                        <div
                          className="relative w-full h-16 cursor-pointer overflow-hidden bg-black/5 dark:bg-white/5"
                          onClick={() => setPreviewLightbox({ src: att.dataUrl, alt: att.name })}
                          title={att.name}
                        >
                          <img
                            src={att.dataUrl}
                            alt={att.name}
                            className="w-full h-full object-cover transition-transform group-hover:scale-105"
                          />
                          <div className="absolute inset-0 bg-black/0 group-hover:bg-black/25 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity">
                            <ZoomIn className="h-4 w-4 text-white drop-shadow" />
                          </div>
                        </div>
                        <div className="w-full px-1.5 py-0.5 flex items-center justify-between text-[10px] font-mono text-[#8f8f8f] dark:text-[#8b949e] border-t border-[#ebebeb] dark:border-[#30363d] bg-[#fafafa] dark:bg-[#161b22]">
                          <span className="truncate max-w-[60px]" title={att.name}>
                            #{idx + 1}
                          </span>
                          <button
                            type="button"
                            onClick={(e) => {
                              e.stopPropagation();
                              handleRemoveAttachment(att.id);
                            }}
                            className="p-0.5 rounded text-[#8f8f8f] hover:text-[#ef4444] dark:hover:text-[#f87171] transition cursor-pointer"
                            aria-label={t('newTask.removeAttachment')}
                          >
                            <X className="h-3 w-3" />
                          </button>
                        </div>
                      </div>
                    ))}
                  </div>
                ) : (
                  <div
                    onClick={() => fileInputRef.current?.click()}
                    className="flex flex-col items-center justify-center p-3 rounded-md border border-dashed border-[#d4d4d8] dark:border-[#30363d] bg-[#fafafa]/50 dark:bg-[#161b22]/30 text-center cursor-pointer hover:bg-[#fafafa] dark:hover:bg-[#161b22] transition"
                  >
                    <Paperclip className="h-4 w-4 text-[#8f8f8f] dark:text-[#8b949e] mb-1" />
                    <span className="text-[11px] font-sans text-[#666666] dark:text-[#8b949e]">
                      {t('newTask.pasteHint')}
                    </span>
                  </div>
                )}
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

              <div className="flex items-center justify-end space-x-2 pt-3 border-t border-[#ebebeb] dark:border-[#21262d] shrink-0">
                <button
                  type="button"
                  onClick={handleCloseAddTask}
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

      {/* Image Preview Lightbox */}
      {previewLightbox && (
        <ImageLightboxModal
          src={previewLightbox.src}
          alt={previewLightbox.alt}
          onClose={() => setPreviewLightbox(null)}
        />
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
