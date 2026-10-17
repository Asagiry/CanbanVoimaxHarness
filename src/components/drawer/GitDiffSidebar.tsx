import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import {
  GitCompare,
  GitBranch,
  RefreshCw,
  PanelRightClose,
  PanelRightOpen,
  CheckCircle2,
  Copy,
  Check,
  AlertCircle,
  ChevronDown,
  ChevronRight,
  ChevronsDownUp,
  ChevronsUpDown,
} from 'lucide-react';
import { DiffFile, TaskGitDiff } from '../../types';

interface GitDiffSidebarProps {
  taskId: string;
  isCollapsed: boolean;
  onToggleCollapse: () => void;
  className?: string;
}

interface ParsedDiffLine {
  type: 'hunk' | 'add' | 'del' | 'context';
  text: string;
  oldNum: number | null;
  newNum: number | null;
}

/**
 * Parses unified diff into clean hunks, stripping all git metadata noise
 * (diff --git, index, --- /dev/null, +++ b/..., new file mode, etc.)
 */
function parseCleanDiff(patch?: string | null): ParsedDiffLine[] {
  if (!patch || typeof patch !== 'string') return [];
  const lines = patch.split('\n');
  const result: ParsedDiffLine[] = [];
  let oldLine = 0;
  let newLine = 0;
  let seenFirstHunk = false;

  for (const line of lines) {
    // 1. Hunk header: starts with @@
    if (line.startsWith('@@')) {
      seenFirstHunk = true;
      const match = line.match(/@@\s+-(\d+)(?:,\d+)?\s+\+(\d+)(?:,\d+)?\s+@@/);
      if (match) {
        oldLine = parseInt(match[1], 10);
        newLine = parseInt(match[2], 10);
      }
      result.push({ type: 'hunk', text: line, oldNum: null, newNum: null });
      continue;
    }

    // 2. Ignore any git metadata noise before or between hunks
    if (!seenFirstHunk) {
      continue;
    }

    if (
      line.startsWith('diff --git') ||
      line.startsWith('index ') ||
      line.startsWith('--- ') ||
      line.startsWith('+++ ') ||
      line.startsWith('new file ') ||
      line.startsWith('deleted file ') ||
      line.startsWith('similarity index') ||
      line.startsWith('rename from') ||
      line.startsWith('rename to') ||
      line.startsWith('\\ No newline')
    ) {
      continue;
    }

    // 3. Diff content lines
    if (line.startsWith('+')) {
      result.push({ type: 'add', text: line.slice(1), oldNum: null, newNum: newLine });
      newLine++;
    } else if (line.startsWith('-')) {
      result.push({ type: 'del', text: line.slice(1), oldNum: oldLine, newNum: null });
      oldLine++;
    } else {
      const text = line.startsWith(' ') ? line.slice(1) : line;
      result.push({ type: 'context', text, oldNum: oldLine, newNum: newLine });
      oldLine++;
      newLine++;
    }
  }

  return result;
}

function formatFilesCount(count: number): string {
  const mod10 = count % 10;
  const mod100 = count % 100;
  if (mod100 >= 11 && mod100 <= 19) return `${count} файлов`;
  if (mod10 === 1) return `${count} файл`;
  if (mod10 >= 2 && mod10 <= 4) return `${count} файла`;
  return `${count} файлов`;
}

interface AccordionFileItemProps {
  file: DiffFile;
  isExpanded: boolean;
  onToggle: () => void;
}

function AccordionFileItem({ file, isExpanded, onToggle }: AccordionFileItemProps) {
  const [copied, setCopied] = useState(false);
  const copiedTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (copiedTimeoutRef.current) {
        clearTimeout(copiedTimeoutRef.current);
      }
    };
  }, []);

  const parsedLines = useMemo(() => {
    return parseCleanDiff(file.patch);
  }, [file.patch]);

  const lastSlashIndex = file.path.lastIndexOf('/');
  const dirPart = lastSlashIndex !== -1 ? file.path.slice(0, lastSlashIndex + 1) : '';
  const filePart = lastSlashIndex !== -1 ? file.path.slice(lastSlashIndex + 1) : file.path;

  const statusColor =
    file.status === 'A'
      ? 'bg-[#f0fdf4] text-[#15803d] border-[#bbf7d0]'
      : file.status === 'D'
      ? 'bg-[#fef2f2] text-[#b91c1c] border-[#fecaca]'
      : 'bg-[#fffbeb] text-[#b45309] border-[#fde68a]';

  const handleCopyPatch = async (e: React.MouseEvent) => {
    e.stopPropagation();
    if (!file.patch) return;
    try {
      await navigator.clipboard.writeText(file.patch);
      setCopied(true);
      if (copiedTimeoutRef.current) {
        clearTimeout(copiedTimeoutRef.current);
      }
      copiedTimeoutRef.current = setTimeout(() => {
        setCopied(false);
        copiedTimeoutRef.current = null;
      }, 1500);
    } catch {
      // ignore
    }
  };

  return (
    <div className="rounded-md border border-[#ebebeb] bg-white overflow-hidden shadow-xs transition duration-150">
      {/* File Header Row (Clickable Accordion Trigger) */}
      <div
        onClick={onToggle}
        className="flex items-center justify-between px-3 py-2 bg-[#fcfcfc] hover:bg-[#f4f4f5] cursor-pointer transition select-none"
      >
        <div className="flex items-center space-x-2 min-w-0 mr-2">
          {isExpanded ? (
            <ChevronDown className="h-3.5 w-3.5 text-[#666666] shrink-0" />
          ) : (
            <ChevronRight className="h-3.5 w-3.5 text-[#666666] shrink-0" />
          )}

          <span
            className={`px-1.5 py-0.2 rounded text-[10px] font-bold border shrink-0 font-mono ${statusColor}`}
          >
            {file.status}
          </span>

          <span className="truncate text-xs font-mono" title={file.path}>
            {dirPart && <span className="text-[#8f8f8f]">{dirPart}</span>}
            <span className="font-semibold text-[#171717]">{filePart}</span>
          </span>
        </div>

        <div className="flex items-center space-x-2 shrink-0 font-mono text-[11px] tabular-nums">
          {(file.additions ?? 0) > 0 && (
            <span className="text-[#15803d] font-medium">+{file.additions}</span>
          )}
          {(file.deletions ?? 0) > 0 && (
            <span className="text-[#b91c1c] font-medium">-{file.deletions}</span>
          )}

          <button
            type="button"
            onClick={handleCopyPatch}
            className="p-1 rounded text-[#8f8f8f] hover:text-[#171717] hover:bg-[#ebebeb] transition cursor-pointer"
            title="Скопировать patch этого файла"
          >
            {copied ? (
              <Check className="h-3 w-3 text-emerald-600" />
            ) : (
              <Copy className="h-3 w-3" />
            )}
          </button>
        </div>
      </div>

      {/* Expanded Clean Diff View */}
      {isExpanded && (
        <div className="border-t border-[#ebebeb] bg-white font-mono text-[11px] leading-relaxed select-text overflow-x-auto max-h-[460px] overflow-y-auto">
          {parsedLines.length === 0 ? (
            <div className="p-3 text-center text-[#8f8f8f] text-xs">
              Нет текстовых изменений (бинарный файл или пустой diff)
            </div>
          ) : (
            parsedLines.map((line, idx) => {
              if (line.type === 'hunk') {
                return (
                  <div
                    key={idx}
                    className="bg-[#f4f4f5] text-[#666666] border-y border-[#ebebeb] my-0.5 py-0.5 px-2 text-[10px] font-semibold select-text"
                  >
                    {line.text}
                  </div>
                );
              }

              if (line.type === 'add') {
                return (
                  <div
                    key={idx}
                    className="flex items-start bg-[#f0fdf4] text-[#166534] hover:bg-[#dcfce7] border-l-2 border-[#22c55e] py-0.5"
                  >
                    <span className="w-8 shrink-0 text-right pr-2 text-[#8f8f8f] select-none text-[10px]">
                      {/* empty old line */}
                    </span>
                    <span className="w-8 shrink-0 text-right pr-2 text-[#16a34a] select-none text-[10px] tabular-nums font-mono">
                      {line.newNum}
                    </span>
                    <span className="w-4 shrink-0 text-center text-[#16a34a] font-bold select-none">
                      +
                    </span>
                    <span className="flex-1 whitespace-pre-wrap break-all pr-2">
                      {line.text}
                    </span>
                  </div>
                );
              }

              if (line.type === 'del') {
                return (
                  <div
                    key={idx}
                    className="flex items-start bg-[#fef2f2] text-[#991b1b] hover:bg-[#fee2e2] border-l-2 border-[#ef4444] py-0.5"
                  >
                    <span className="w-8 shrink-0 text-right pr-2 text-[#ef4444] select-none text-[10px] tabular-nums font-mono">
                      {line.oldNum}
                    </span>
                    <span className="w-8 shrink-0 text-right pr-2 text-[#8f8f8f] select-none text-[10px]">
                      {/* empty new line */}
                    </span>
                    <span className="w-4 shrink-0 text-center text-[#ef4444] font-bold select-none">
                      -
                    </span>
                    <span className="flex-1 whitespace-pre-wrap break-all pr-2">
                      {line.text}
                    </span>
                  </div>
                );
              }

              // Context line
              return (
                <div
                  key={idx}
                  className="flex items-start hover:bg-[#f9f9f9] py-0.5 text-[#171717]"
                >
                  <span className="w-8 shrink-0 text-right pr-2 text-[#8f8f8f] select-none text-[10px] tabular-nums font-mono">
                    {line.oldNum}
                  </span>
                  <span className="w-8 shrink-0 text-right pr-2 text-[#8f8f8f] select-none text-[10px] tabular-nums font-mono">
                    {line.newNum}
                  </span>
                  <span className="w-4 shrink-0 text-center text-[#8f8f8f] select-none">
                    {' '}
                  </span>
                  <span className="flex-1 whitespace-pre-wrap break-all pr-2">
                    {line.text}
                  </span>
                </div>
              );
            })
          )}
        </div>
      )}
    </div>
  );
}

const DEFAULT_SIDEBAR_WIDTH = 480;
const MIN_SIDEBAR_WIDTH = 360;
const MAX_SIDEBAR_WIDTH = 840;

export default function GitDiffSidebar({
  taskId,
  isCollapsed,
  onToggleCollapse,
  className = '',
}: GitDiffSidebarProps) {
  const [diff, setDiff] = useState<TaskGitDiff | null>(null);
  const [expandedFiles, setExpandedFiles] = useState<Set<string>>(() => new Set());
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Width & Resizability (default 480px, stored in localStorage)
  const [sidebarWidth, setSidebarWidth] = useState<number>(() => {
    try {
      const saved = localStorage.getItem('cvh_git_diff_width');
      if (saved) {
        const parsed = parseInt(saved, 10);
        if (!isNaN(parsed) && parsed >= MIN_SIDEBAR_WIDTH && parsed <= MAX_SIDEBAR_WIDTH) {
          return parsed;
        }
      }
    } catch {}
    return DEFAULT_SIDEBAR_WIDTH;
  });
  const [isResizing, setIsResizing] = useState(false);
  const isDraggingRef = useRef(false);
  const startXRef = useRef(0);
  const startWidthRef = useRef(sidebarWidth);

  // Bug Fix: Track initial load and explicit user-collapsed state
  // Background syncs on tool events MUST NOT re-open files intentionally collapsed by user!
  const hasInitialLoadedRef = useRef(false);
  const userCollapsedFilesRef = useRef<Set<string>>(new Set());

  const debounceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Reset initial tracking on task switch
  useEffect(() => {
    hasInitialLoadedRef.current = false;
    userCollapsedFilesRef.current.clear();
    setExpandedFiles(new Set());
  }, [taskId]);

  const fetchDiff = useCallback(async () => {
    if (!taskId) return;
    setIsLoading(true);
    setError(null);
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const data = await invoke<TaskGitDiff | null>('get_task_git_diff', { taskId });
      setDiff(data ?? null);

      if (data?.files && data.files.length > 0) {
        if (!hasInitialLoadedRef.current) {
          // Initial load for this task: auto-expand the first file (or 2 if total <= 2)
          hasInitialLoadedRef.current = true;
          setExpandedFiles(() => {
            const next = new Set<string>();
            if (data.files.length <= 2) {
              data.files.forEach((f) => {
                if (!userCollapsedFilesRef.current.has(f.path)) {
                  next.add(f.path);
                }
              });
            } else if (data.files[0]?.path && !userCollapsedFilesRef.current.has(data.files[0].path)) {
              next.add(data.files[0].path);
            }
            return next;
          });
        } else {
          // Background sync (subsequent fetch on live tool events, etc.):
          // NEVER forcibly re-expand files that the user intentionally collapsed!
          // Maintain currently expanded files, purging any paths no longer in diff
          setExpandedFiles((prev) => {
            const currentValidPaths = new Set(data.files.map((f) => f.path));
            const next = new Set<string>();
            let changed = false;
            for (const path of prev) {
              if (currentValidPaths.has(path) && !userCollapsedFilesRef.current.has(path)) {
                next.add(path);
              } else {
                changed = true;
              }
            }
            return changed ? next : prev;
          });
        }
      } else {
        setExpandedFiles(new Set());
      }
    } catch (err: any) {
      console.info('get_task_git_diff invoke failed or web mode:', err);
      const msg = err?.message || (typeof err === 'string' ? err : 'Ошибка загрузки Git Diff');
      setError(msg);
    } finally {
      setIsLoading(false);
    }
  }, [taskId]);

  const scheduleFetch = useCallback(() => {
    if (debounceTimerRef.current) {
      clearTimeout(debounceTimerRef.current);
    }
    debounceTimerRef.current = setTimeout(() => {
      debounceTimerRef.current = null;
      fetchDiff();
    }, 300);
  }, [fetchDiff]);

  // Initial load on mount or task switch
  useEffect(() => {
    fetchDiff();
  }, [fetchDiff]);

  // Listen to live agent tool execution & finish events with anti-race disposal
  useEffect(() => {
    if (!taskId) return;
    let isDisposed = false;
    let unlistenToolCall: (() => void) | undefined;
    let unlistenToolResult: (() => void) | undefined;
    let unlistenFinish: (() => void) | undefined;

    const setupListeners = async () => {
      try {
        const { listen } = await import('@tauri-apps/api/event');
        if (isDisposed) return;

        const uCall = await listen<{ taskId?: string }>('agent:tool_call', (event) => {
          if (event.payload?.taskId === taskId) {
            scheduleFetch();
          }
        });
        if (isDisposed) {
          uCall();
        } else {
          unlistenToolCall = uCall;
        }

        const uResult = await listen<{ taskId?: string }>('agent:tool_result', (event) => {
          if (event.payload?.taskId === taskId) {
            scheduleFetch();
          }
        });
        if (isDisposed) {
          uResult();
        } else {
          unlistenToolResult = uResult;
        }

        const uFinish = await listen<{ taskId?: string }>('agent:finished', (event) => {
          if (event.payload?.taskId === taskId) {
            scheduleFetch();
          }
        });
        if (isDisposed) {
          uFinish();
        } else {
          unlistenFinish = uFinish;
        }
      } catch {
        // web fallback
      }
    };

    setupListeners();

    return () => {
      isDisposed = true;
      if (unlistenToolCall) unlistenToolCall();
      if (unlistenToolResult) unlistenToolResult();
      if (unlistenFinish) unlistenFinish();
      if (debounceTimerRef.current) {
        clearTimeout(debounceTimerRef.current);
      }
    };
  }, [taskId, scheduleFetch]);

  // Drag handle resize logic
  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    isDraggingRef.current = true;
    startXRef.current = e.clientX;
    startWidthRef.current = sidebarWidth;
    setIsResizing(true);
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';

    const handleMouseMove = (moveEvent: MouseEvent) => {
      if (!isDraggingRef.current) return;
      const delta = startXRef.current - moveEvent.clientX;
      const maxAllowed = Math.min(MAX_SIDEBAR_WIDTH, window.innerWidth * 0.7);
      const newWidth = Math.min(Math.max(MIN_SIDEBAR_WIDTH, startWidthRef.current + delta), maxAllowed);
      setSidebarWidth(newWidth);
    };

    const handleMouseUp = () => {
      if (isDraggingRef.current) {
        isDraggingRef.current = false;
        setIsResizing(false);
        document.body.style.cursor = '';
        document.body.style.userSelect = '';
        setSidebarWidth((finalWidth) => {
          try {
            localStorage.setItem('cvh_git_diff_width', String(Math.round(finalWidth)));
          } catch {}
          return finalWidth;
        });
      }
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
  }, [sidebarWidth]);

  useEffect(() => {
    return () => {
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    };
  }, []);

  const toggleFile = (path: string) => {
    setExpandedFiles((prev) => {
      const next = new Set(prev);
      if (next.has(path)) {
        // User intentionally collapses this file
        next.delete(path);
        userCollapsedFilesRef.current.add(path);
      } else {
        // User intentionally expands this file
        next.add(path);
        userCollapsedFilesRef.current.delete(path);
      }
      return next;
    });
  };

  const handleExpandAll = () => {
    if (!diff?.files) return;
    userCollapsedFilesRef.current.clear();
    setExpandedFiles(new Set(diff.files.map((f) => f.path)));
  };

  const handleCollapseAll = () => {
    if (!diff?.files) return;
    diff.files.forEach((f) => userCollapsedFilesRef.current.add(f.path));
    setExpandedFiles(new Set());
  };

  const totalAdditions = diff?.totalAdditions ?? (diff as any)?.total_additions ?? 0;
  const totalDeletions = diff?.totalDeletions ?? (diff as any)?.total_deletions ?? 0;
  const filesCount = diff?.files?.length ?? 0;
  const files = diff?.files ?? [];

  // Collapsed Rail View
  if (isCollapsed) {
    return (
      <aside
        className={`w-11 shrink-0 border-l border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#0d1117] flex flex-col items-center py-3 select-none transition-all duration-150 ${className}`}
      >
        <button
          type="button"
          onClick={onToggleCollapse}
          className="p-1.5 rounded-md text-[#8f8f8f] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#f4f4f5] dark:hover:bg-[#161b22] transition cursor-pointer"
          title="Развернуть Git Diff"
        >
          <PanelRightOpen className="h-4 w-4" />
        </button>

        <div className="mt-4 flex flex-col items-center space-y-2">
          <GitCompare className="h-4 w-4 text-[#8f8f8f] dark:text-[#8b949e]" />
          {filesCount > 0 && (
            <span className="px-1.5 py-0.5 rounded text-[10px] font-mono font-bold bg-[#f4f4f5] dark:bg-[#161b22] text-[#666666] dark:text-[#8b949e] border border-[#ebebeb] dark:border-[#30363d]">
              {filesCount}
            </span>
          )}
          {totalAdditions > 0 && (
            <span className="text-[10px] font-mono font-medium text-[#15803d]">
              +{totalAdditions}
            </span>
          )}
          {totalDeletions > 0 && (
            <span className="text-[10px] font-mono font-medium text-[#b91c1c]">
              -{totalDeletions}
            </span>
          )}
        </div>

        <div
          onClick={onToggleCollapse}
          className="mt-6 flex-1 flex items-center justify-center cursor-pointer group"
          title="Нажмите, чтобы развернуть Git Diff"
        >
          <span className="text-[11px] font-mono text-[#8f8f8f] dark:text-[#8b949e] group-hover:text-[#171717] dark:group-hover:text-[#f0f6fc] transition tracking-wider [writing-mode:vertical-rl] rotate-180">
            Git Diff
          </span>
        </div>
      </aside>
    );
  }

  // Expanded Sidebar View (Default 480px, Resizable) with File Accordion List
  return (
    <aside
      style={{ width: `${sidebarWidth}px` }}
      className={`relative shrink-0 border-l border-[#ebebeb] dark:border-[#21262d] bg-[#fafafa] dark:bg-[#090a0f] flex flex-col h-full text-[#171717] dark:text-[#f0f6fc] overflow-hidden select-none transition-colors ${isResizing ? 'select-none' : ''} ${className}`}
    >
      {/* Left Resize Drag Handle */}
      <div
        onMouseDown={handleMouseDown}
        className="group/handle absolute top-0 bottom-0 left-0 w-2 -ml-1 cursor-col-resize z-30 flex items-center justify-center select-none"
        title="Потяните для изменения ширины панели Git Diff"
      >
        <div
          className={`w-0.5 h-8 rounded-full transition-colors ${
            isResizing ? 'bg-emerald-500' : 'bg-transparent group-hover/handle:bg-[#00000025]'
          }`}
        />
      </div>

      {/* 1. Header: Title, Badges, Expand/Collapse All, Refresh, Panel Collapse */}
      <header className="px-3 py-2 border-b border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#0d1117] flex flex-wrap items-center justify-between gap-1.5 shrink-0 shadow-xs min-w-0 transition-colors">
        {/* Left Section: Title, File count, Diff stats */}
        <div className="flex items-center gap-1.5 min-w-0 flex-wrap">
          <div className="flex items-center gap-1.5 shrink-0">
            <GitCompare className="h-3.5 w-3.5 text-[#8f8f8f] dark:text-[#8b949e] shrink-0" />
            <span className="font-semibold text-xs text-[#171717] dark:text-[#f0f6fc] tracking-tight font-sans">
              Git Diff
            </span>
          </div>

          {/* Changed files count badge */}
          <span
            className="px-1.5 py-0.5 rounded text-[10px] font-mono font-medium bg-[#f4f4f5] dark:bg-[#161b22] text-[#666666] dark:text-[#8b949e] border border-[#ebebeb] dark:border-[#30363d] shrink-0 whitespace-nowrap"
            title={`${filesCount} измененных файлов`}
          >
            {formatFilesCount(filesCount)}
          </span>

          {/* Additions / deletions badges */}
          {(totalAdditions > 0 || totalDeletions > 0) && (
            <div className="flex items-center gap-1 text-[10px] font-mono tabular-nums font-semibold shrink-0">
              {totalAdditions > 0 && (
                <span className="text-[#15803d] dark:text-[#4ade80]" title={`+${totalAdditions} добавленных строк`}>
                  +{totalAdditions}
                </span>
              )}
              {totalDeletions > 0 && (
                <span className="text-[#b91c1c] dark:text-[#f87171]" title={`-${totalDeletions} удаленных строк`}>
                  -{totalDeletions}
                </span>
              )}
            </div>
          )}
        </div>

        {/* Right Section: Branch badge & Action buttons */}
        <div className="flex items-center gap-1 shrink-0 ml-auto">
          {/* Branch badge with clean truncation and tooltip */}
          {diff?.branch && (
            <div
              className="flex items-center gap-1 px-1.5 py-0.5 rounded bg-[#f4f4f5] dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] text-[10px] font-mono text-[#666666] dark:text-[#8b949e] max-w-[130px] min-w-0 cursor-default"
              title={`Текущая ветка: ${diff.branch}`}
            >
              <GitBranch className="h-3 w-3 text-[#8f8f8f] dark:text-[#8b949e] shrink-0" />
              <span className="truncate">{diff.branch}</span>
            </div>
          )}

          {/* Expand / Collapse All Quick Actions */}
          {filesCount > 1 && (
            <div className="flex items-center border border-[#ebebeb] dark:border-[#30363d] rounded bg-white dark:bg-[#161b22]">
              <button
                type="button"
                onClick={handleExpandAll}
                className="p-1 text-[#8f8f8f] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] transition cursor-pointer"
                title="Развернуть все файлы"
              >
                <ChevronsUpDown className="h-3 w-3" />
              </button>
              <div className="w-px h-3 bg-[#ebebeb] dark:bg-[#30363d]" />
              <button
                type="button"
                onClick={handleCollapseAll}
                className="p-1 text-[#8f8f8f] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] transition cursor-pointer"
                title="Свернуть все файлы"
              >
                <ChevronsDownUp className="h-3 w-3" />
              </button>
            </div>
          )}

          {/* Refresh button */}
          <button
            type="button"
            onClick={fetchDiff}
            disabled={isLoading}
            className="p-1 rounded text-[#8f8f8f] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] disabled:opacity-50 transition cursor-pointer"
            title="Обновить дифф"
          >
            <RefreshCw
              className={`h-3.5 w-3.5 ${isLoading ? 'animate-spin text-emerald-600' : ''}`}
            />
          </button>

          {/* Collapse sidebar button */}
          <button
            type="button"
            onClick={onToggleCollapse}
            className="p-1 rounded text-[#8f8f8f] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc] hover:bg-[#f4f4f5] dark:hover:bg-[#21262d] transition cursor-pointer"
            title="Свернуть панель (Git Diff)"
          >
            <PanelRightClose className="h-3.5 w-3.5" />
          </button>
        </div>
      </header>

      {/* Error state */}
      {error && (
        <div className="p-2.5 bg-[#fef2f2] dark:bg-[#450a0a]/40 border-b border-[#fecaca] dark:border-[#991b1b] text-[#991b1b] dark:text-[#f87171] text-xs font-mono flex items-center space-x-2 shrink-0">
          <AlertCircle className="h-4 w-4 text-[#dc2626] shrink-0" />
          <span className="flex-1 truncate">{error}</span>
          <button
            type="button"
            onClick={fetchDiff}
            className="underline hover:text-[#171717] dark:hover:text-[#f0f6fc] cursor-pointer ml-1 text-[11px]"
          >
            Повторить
          </button>
        </div>
      )}

      {/* 2. Main Content Area: Accordion File List */}
      {filesCount === 0 ? (
        // Clean empty state placeholder
        <div className="flex-1 flex flex-col items-center justify-center p-6 text-center bg-[#fafafa] dark:bg-[#090a0f]">
          <div className="h-12 w-12 rounded-full bg-[#f0fdf4] dark:bg-[#052e16]/50 border border-[#bbf7d0] dark:border-[#166534] flex items-center justify-center mb-3">
            <CheckCircle2 className="h-6 w-6 text-[#15803d] dark:text-[#4ade80]" />
          </div>
          <h3 className="text-sm font-semibold text-[#171717] dark:text-[#f0f6fc] font-sans">Ворктри чистое</h3>
          <p className="text-xs text-[#666666] dark:text-[#8b949e] mt-1 max-w-[260px] leading-relaxed">
            Нет изменений / рабочее дерево чистое
          </p>
          {diff?.branch && (
            <div className="mt-4 px-2.5 py-1 rounded-md bg-white dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] text-[11px] font-mono text-[#666666] dark:text-[#8b949e] flex items-center space-x-1.5">
              <GitBranch className="h-3 w-3 text-[#8f8f8f] dark:text-[#8b949e]" />
              <span>Ветка: {diff.branch}</span>
            </div>
          )}
        </div>
      ) : (
        <div className="flex-1 overflow-y-auto p-3 space-y-2 min-h-0 bg-[#fafafa] dark:bg-[#090a0f]">
          {files.map((file) => {
            if (!file?.path) return null;
            const isExpanded = expandedFiles.has(file.path);
            return (
              <AccordionFileItem
                key={file.path}
                file={file}
                isExpanded={isExpanded}
                onToggle={() => toggleFile(file.path)}
              />
            );
          })}
        </div>
      )}
    </aside>
  );
}
