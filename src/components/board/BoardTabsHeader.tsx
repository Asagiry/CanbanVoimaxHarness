import React, { useState, useEffect, useRef } from 'react';
import { Search, Layout, Plus, Pencil, Trash2 } from 'lucide-react';
import { useBoardStore } from '../../stores/boardStore';
import { useTranslation } from '../../stores/languageStore';
import { BoardDef } from '../../types';

export interface BoardTabsHeaderProps {
  tasksCount: number;
  onOpenAddColumn: () => void;
  onOpenAddTask: () => void;
}

interface BoardContextMenuState {
  x: number;
  y: number;
  boardId: string;
}

export default function BoardTabsHeader({
  tasksCount,
  onOpenAddColumn,
  onOpenAddTask,
}: BoardTabsHeaderProps) {
  const {
    boards,
    activeBoardId,
    setActiveBoard,
    createBoard,
    saveBoard,
    deleteBoard,
    searchQuery,
    setSearchQuery,
    tasks,
  } = useBoardStore();
  const { t } = useTranslation();

  const [boardContextMenu, setBoardContextMenu] = useState<BoardContextMenuState | null>(null);
  const [renamingBoardId, setRenamingBoardId] = useState<string | null>(null);
  const [renamingBoardName, setRenamingBoardName] = useState('');
  const [isCreatingBoard, setIsCreatingBoard] = useState(false);
  const [newBoardName, setNewBoardName] = useState('');
  const isSubmittingBoardRef = useRef(false);

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

  const handleCommitCreateBoard = async () => {
    if (isSubmittingBoardRef.current) return;
    isSubmittingBoardRef.current = true;
    const trimmed = newBoardName.trim();
    setIsCreatingBoard(false);
    setNewBoardName('');
    if (!trimmed) {
      isSubmittingBoardRef.current = false;
      return;
    }
    try {
      await createBoard(trimmed);
    } catch (err) {
      console.info('createBoard failed:', err);
    } finally {
      isSubmittingBoardRef.current = false;
    }
  };

  const handleCancelCreateBoard = () => {
    if (isSubmittingBoardRef.current) return;
    setIsCreatingBoard(false);
    setNewBoardName('');
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

  return (
    <>
      <div className="h-12 shrink-0 flex items-center justify-between px-3.5 border-b border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#0d1117] text-xs select-none transition-colors">
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
          {isCreatingBoard ? (
            <div className="flex items-center space-x-1 px-2 py-1 rounded-md text-xs font-medium border bg-white dark:bg-[#0d1117] border-[#171717] dark:border-[#30363d] text-[#171717] dark:text-[#f0f6fc]">
              <Layout className="h-3.5 w-3.5 shrink-0 text-[#71717a] dark:text-[#8b949e]" />
              <input
                type="text"
                autoFocus
                placeholder={t('createBoard.namePlaceholder')}
                value={newBoardName}
                onChange={(e) => setNewBoardName(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    e.preventDefault();
                    e.stopPropagation();
                    void handleCommitCreateBoard();
                  } else if (e.key === 'Escape') {
                    e.preventDefault();
                    e.stopPropagation();
                    handleCancelCreateBoard();
                  }
                }}
                onBlur={handleCancelCreateBoard}
                onClick={(e) => e.stopPropagation()}
                className="px-1.5 py-0.5 rounded text-xs outline-hidden w-28 sm:w-36 font-sans bg-[#fafafa] dark:bg-[#161b22] text-[#171717] dark:text-[#f0f6fc] border border-[#d4d4d8] dark:border-[#30363d]"
              />
            </div>
          ) : (
            <button
              onClick={() => {
                setNewBoardName('');
                setIsCreatingBoard(true);
              }}
              className="flex items-center space-x-1 px-2.5 py-1.5 rounded-md border border-dashed border-[#d4d4d8] dark:border-[#30363d] hover:border-[#171717] dark:hover:border-[#f0f6fc] text-[#71717a] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] text-xs font-medium transition cursor-pointer select-none"
              aria-label={t('board.newBoardTitle')}
            >
              <Plus className="h-3 w-3" />
              <span>{t('board.newBoard')}</span>
            </button>
          )}
        </div>

        {/* Right: Board Stats & Action Buttons */}
        <div className="flex items-center space-x-3 shrink-0">
          <div className="hidden sm:flex items-center space-x-1.5 text-[#666666] dark:text-[#8b949e] text-xs">
            <span>{t('board.tasksOnBoard')}</span>
            <span className="text-[#171717] dark:text-[#f0f6fc] font-semibold tabular-nums font-mono">
              {tasksCount}
            </span>
          </div>

          <div className="flex items-center space-x-2">
            <button
              onClick={onOpenAddColumn}
              className="flex items-center space-x-1.5 px-3 py-1.5 rounded-md bg-white dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] text-[#171717] dark:text-[#f0f6fc] font-sans text-xs font-medium transition cursor-pointer active:scale-[0.99]"
              aria-label={t('board.addColumnTitle')}
            >
              <Plus className="h-3.5 w-3.5" />
              <span>{t('board.addColumn')}</span>
            </button>
            <button
              onClick={onOpenAddTask}
              className="flex items-center space-x-1.5 px-3 py-1.5 rounded-md bg-[#171717] dark:bg-[#21262d] hover:bg-[#262626] dark:hover:bg-[#30363d] text-white font-sans text-xs font-medium transition cursor-pointer shadow-xs active:scale-[0.99] border border-transparent dark:border-[#30363d]"
              aria-label={t('board.newTaskTitle')}
            >
              <Plus className="h-3.5 w-3.5 text-white" />
              <span>{t('board.newTask')}</span>
            </button>
          </div>
        </div>
      </div>

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
    </>
  );
}
