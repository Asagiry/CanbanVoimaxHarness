import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import {
  GitCompare,
  GitBranch,
  RefreshCw,
  PanelRightClose,
  PanelRightOpen,
  CheckCircle2,
  FileCode,
  Copy,
  Check,
  AlertCircle,
} from 'lucide-react';
import { DiffFile, TaskGitDiff } from '../../types';

interface GitDiffSidebarProps {
  taskId: string;
  isCollapsed: boolean;
  onToggleCollapse: () => void;
  className?: string;
}

interface ParsedDiffLine {
  type: 'meta' | 'hunk' | 'add' | 'del' | 'context';
  text: string;
  oldNum: number | null;
  newNum: number | null;
}

function parseUnifiedPatch(patch: string): ParsedDiffLine[] {
  if (!patch) return [];
  const lines = patch.split('\n');
  const result: ParsedDiffLine[] = [];
  let oldLine = 0;
  let newLine = 0;

  for (const line of lines) {
    if (line.startsWith('@@')) {
      const match = line.match(/@@\s+-(\d+)(?:,\d+)?\s+\+(\d+)(?:,\d+)?\s+@@/);
      if (match) {
        oldLine = parseInt(match[1], 10);
        newLine = parseInt(match[2], 10);
      }
      result.push({ type: 'hunk', text: line, oldNum: null, newNum: null });
    } else if (line.startsWith('+') && !line.startsWith('+++')) {
      result.push({ type: 'add', text: line.slice(1), oldNum: null, newNum: newLine });
      newLine++;
    } else if (line.startsWith('-') && !line.startsWith('---')) {
      result.push({ type: 'del', text: line.slice(1), oldNum: oldLine, newNum: null });
      oldLine++;
    } else if (
      line.startsWith('diff --git') ||
      line.startsWith('index ') ||
      line.startsWith('--- ') ||
      line.startsWith('+++ ') ||
      line.startsWith('new file ') ||
      line.startsWith('deleted file ')
    ) {
      result.push({ type: 'meta', text: line, oldNum: null, newNum: null });
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

export default function GitDiffSidebar({
  taskId,
  isCollapsed,
  onToggleCollapse,
  className = '',
}: GitDiffSidebarProps) {
  const [diff, setDiff] = useState<TaskGitDiff | null>(null);
  const [selectedFilePath, setSelectedFilePath] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  const debounceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const fetchDiff = useCallback(async () => {
    if (!taskId) return;
    setIsLoading(true);
    setError(null);
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const data = await invoke<TaskGitDiff>('get_task_git_diff', { taskId });
      setDiff(data);
      setSelectedFilePath((prev) => {
        if (!data.files || data.files.length === 0) return null;
        if (prev && data.files.some((f) => f.path === prev)) return prev;
        return data.files[0]?.path || null;
      });
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

  // Listen to live agent tool execution & finish events
  useEffect(() => {
    if (!taskId) return;
    let unlistenTool: (() => void) | undefined;
    let unlistenFinish: (() => void) | undefined;

    const setupListeners = async () => {
      try {
        const { listen } = await import('@tauri-apps/api/event');
        unlistenTool = await listen<{ taskId?: string }>('agent:tool_result', (event) => {
          if (event.payload?.taskId === taskId) {
            scheduleFetch();
          }
        });

        unlistenFinish = await listen<{ taskId?: string }>('agent:finished', (event) => {
          if (event.payload?.taskId === taskId) {
            scheduleFetch();
          }
        });
      } catch {
        // web fallback
      }
    };

    setupListeners();

    return () => {
      if (unlistenTool) unlistenTool();
      if (unlistenFinish) unlistenFinish();
      if (debounceTimerRef.current) {
        clearTimeout(debounceTimerRef.current);
      }
    };
  }, [taskId, scheduleFetch]);

  const activeFile = useMemo<DiffFile | null>(() => {
    if (!diff?.files || diff.files.length === 0) return null;
    return diff.files.find((f) => f.path === selectedFilePath) || diff.files[0] || null;
  }, [diff, selectedFilePath]);

  const parsedLines = useMemo(() => {
    return activeFile ? parseUnifiedPatch(activeFile.patch) : [];
  }, [activeFile]);

  const handleCopyPatch = async () => {
    if (!activeFile?.patch) return;
    try {
      await navigator.clipboard.writeText(activeFile.patch);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // ignore
    }
  };

  const totalAdditions = diff?.totalAdditions ?? (diff as any)?.total_additions ?? 0;
  const totalDeletions = diff?.totalDeletions ?? (diff as any)?.total_deletions ?? 0;
  const filesCount = diff?.files?.length ?? 0;

  // Collapsed Rail View
  if (isCollapsed) {
    return (
      <aside
        className={`w-11 shrink-0 border-l border-[#27272a] bg-[#121214] flex flex-col items-center py-3 select-none transition-all duration-150 ${className}`}
      >
        <button
          type="button"
          onClick={onToggleCollapse}
          className="p-1.5 rounded-md text-[#a1a1aa] hover:text-white hover:bg-[#27272a] transition cursor-pointer"
          title="Развернуть Git Diff"
        >
          <PanelRightOpen className="h-4 w-4" />
        </button>

        <div className="mt-4 flex flex-col items-center space-y-2">
          <GitCompare className="h-4 w-4 text-[#71717a]" />
          {filesCount > 0 && (
            <span className="px-1.5 py-0.5 rounded text-[10px] font-mono font-bold bg-[#27272a] text-[#a1a1aa] border border-[#3f3f46]/50">
              {filesCount}
            </span>
          )}
          {totalAdditions > 0 && (
            <span className="text-[10px] font-mono font-medium text-emerald-400">
              +{totalAdditions}
            </span>
          )}
          {totalDeletions > 0 && (
            <span className="text-[10px] font-mono font-medium text-rose-400">
              -{totalDeletions}
            </span>
          )}
        </div>

        <div
          onClick={onToggleCollapse}
          className="mt-6 flex-1 flex items-center justify-center cursor-pointer group"
          title="Нажмите, чтобы развернуть Git Diff"
        >
          <span className="text-[11px] font-mono text-[#71717a] group-hover:text-white transition tracking-wider [writing-mode:vertical-rl] rotate-180">
            Git Diff
          </span>
        </div>
      </aside>
    );
  }

  // Expanded Sidebar View (440px wide)
  return (
    <aside
      className={`w-[440px] shrink-0 border-l border-[#27272a] bg-[#121214] flex flex-col h-full text-[#e4e4e7] overflow-hidden select-none ${className}`}
    >
      {/* 1. Header: Title, Badges, Refresh, Collapse */}
      <header className="px-3.5 py-2.5 border-b border-[#27272a] bg-[#18181b]/90 flex items-center justify-between shrink-0">
        <div className="flex items-center space-x-2 min-w-0">
          <GitCompare className="h-4 w-4 text-[#a1a1aa] shrink-0" />
          <span className="font-semibold text-xs text-[#f4f4f5] tracking-wide font-sans">
            Git Diff
          </span>

          {/* Changed files badge */}
          <span className="px-2 py-0.5 rounded-md text-[11px] font-mono bg-[#27272a] text-[#a1a1aa] border border-[#3f3f46]/50 whitespace-nowrap">
            {formatFilesCount(filesCount)}
          </span>

          {/* Additions / deletions badges */}
          {(totalAdditions > 0 || totalDeletions > 0) && (
            <div className="flex items-center space-x-1.5 text-[11px] font-mono tabular-nums">
              {totalAdditions > 0 && (
                <span className="text-emerald-400 font-medium">+{totalAdditions}</span>
              )}
              {totalDeletions > 0 && (
                <span className="text-rose-400 font-medium">-{totalDeletions}</span>
              )}
            </div>
          )}
        </div>

        <div className="flex items-center space-x-1 shrink-0">
          {/* Branch badge */}
          {diff?.branch && (
            <div
              className="flex items-center space-x-1 px-2 py-0.5 rounded-md bg-[#222226] border border-[#2e2e33] text-[10px] font-mono text-[#a1a1aa] max-w-[130px] truncate"
              title={`Текущая ветка: ${diff.branch}`}
            >
              <GitBranch className="h-3 w-3 text-[#71717a] shrink-0" />
              <span className="truncate">{diff.branch}</span>
            </div>
          )}

          {/* Refresh button */}
          <button
            type="button"
            onClick={fetchDiff}
            disabled={isLoading}
            className="p-1.5 rounded-md text-[#a1a1aa] hover:text-white hover:bg-[#27272a] disabled:opacity-50 transition cursor-pointer"
            title="Обновить дифф"
          >
            <RefreshCw
              className={`h-3.5 w-3.5 ${isLoading ? 'animate-spin text-emerald-400' : ''}`}
            />
          </button>

          {/* Collapse button */}
          <button
            type="button"
            onClick={onToggleCollapse}
            className="p-1.5 rounded-md text-[#a1a1aa] hover:text-white hover:bg-[#27272a] transition cursor-pointer"
            title="Свернуть панель (Git Diff)"
          >
            <PanelRightClose className="h-4 w-4" />
          </button>
        </div>
      </header>

      {/* Error state */}
      {error && (
        <div className="p-2.5 bg-rose-950/40 border-b border-rose-900/50 text-rose-300 text-xs font-mono flex items-center space-x-2 shrink-0">
          <AlertCircle className="h-4 w-4 text-rose-400 shrink-0" />
          <span className="flex-1 truncate">{error}</span>
          <button
            type="button"
            onClick={fetchDiff}
            className="underline hover:text-white cursor-pointer ml-1 text-[11px]"
          >
            Повторить
          </button>
        </div>
      )}

      {/* 2. Main Content Area */}
      {filesCount === 0 ? (
        // Clean empty state placeholder
        <div className="flex-1 flex flex-col items-center justify-center p-6 text-center">
          <div className="h-12 w-12 rounded-full bg-emerald-950/30 border border-emerald-800/40 flex items-center justify-center mb-3">
            <CheckCircle2 className="h-6 w-6 text-emerald-400" />
          </div>
          <h3 className="text-sm font-semibold text-[#f4f4f5] font-sans">Ворктри чистое</h3>
          <p className="text-xs text-[#71717a] mt-1 max-w-[260px] leading-relaxed">
            Нет изменений / рабочее дерево чистое
          </p>
          {diff?.branch && (
            <div className="mt-4 px-2.5 py-1 rounded-md bg-[#18181b] border border-[#27272a] text-[11px] font-mono text-[#a1a1aa] flex items-center space-x-1.5">
              <GitBranch className="h-3 w-3 text-[#71717a]" />
              <span>Ветка: {diff.branch}</span>
            </div>
          )}
        </div>
      ) : (
        <div className="flex-1 flex flex-col min-h-0">
          {/* File list cards */}
          <div className="max-h-[220px] overflow-y-auto p-2 space-y-1.5 border-b border-[#27272a] bg-[#141417]/80 shrink-0">
            <div className="px-1 text-[10px] font-mono text-[#71717a] uppercase tracking-wider font-semibold">
              Изменённые файлы ({filesCount})
            </div>
            {diff?.files.map((file) => {
              const isActive = file.path === activeFile?.path;
              const lastSlashIndex = file.path.lastIndexOf('/');
              const dirPart =
                lastSlashIndex !== -1 ? file.path.slice(0, lastSlashIndex + 1) : '';
              const filePart =
                lastSlashIndex !== -1 ? file.path.slice(lastSlashIndex + 1) : file.path;

              const statusColor =
                file.status === 'A'
                  ? 'bg-emerald-500/15 text-emerald-400 border-emerald-500/30'
                  : file.status === 'D'
                  ? 'bg-rose-500/15 text-rose-400 border-rose-500/30'
                  : 'bg-amber-500/15 text-amber-400 border-amber-500/30';

              return (
                <button
                  key={file.path}
                  type="button"
                  onClick={() => setSelectedFilePath(file.path)}
                  className={`w-full flex items-center justify-between px-2.5 py-1.5 rounded-md text-left text-xs font-mono transition cursor-pointer ${
                    isActive
                      ? 'bg-[#27272a] text-white ring-1 ring-white/10 shadow-xs'
                      : 'bg-[#18181b]/70 hover:bg-[#202024] text-[#d4d4d8]'
                  }`}
                >
                  <div className="flex items-center space-x-2 min-w-0 mr-2">
                    <span
                      className={`px-1.5 py-0.2 rounded text-[10px] font-bold border shrink-0 ${statusColor}`}
                    >
                      {file.status}
                    </span>
                    <span className="truncate text-xs">
                      {dirPart && <span className="text-[#71717a]">{dirPart}</span>}
                      <span className="font-medium text-[#f4f4f5]">{filePart}</span>
                    </span>
                  </div>

                  <div className="flex items-center space-x-1 text-[11px] tabular-nums shrink-0 font-mono">
                    {file.additions > 0 && (
                      <span className="text-emerald-400">+{file.additions}</span>
                    )}
                    {file.deletions > 0 && (
                      <span className="text-rose-400">-{file.deletions}</span>
                    )}
                  </div>
                </button>
              );
            })}
          </div>

          {/* 3. Diff Viewer of Selected File */}
          <div className="flex-1 flex flex-col min-h-0 bg-[#0d0d0f]">
            {activeFile ? (
              <>
                {/* Active file toolbar */}
                <div className="px-3 py-1.5 border-b border-[#27272a] bg-[#141416] flex items-center justify-between text-xs font-mono shrink-0">
                  <div className="flex items-center space-x-1.5 min-w-0 mr-2">
                    <FileCode className="h-3.5 w-3.5 text-[#a1a1aa] shrink-0" />
                    <span className="truncate text-[#f4f4f5] font-medium" title={activeFile.path}>
                      {activeFile.path}
                    </span>
                  </div>

                  <div className="flex items-center space-x-2 shrink-0">
                    <div className="flex items-center space-x-1 text-[11px] tabular-nums">
                      {activeFile.additions > 0 && (
                        <span className="text-emerald-400">+{activeFile.additions}</span>
                      )}
                      {activeFile.deletions > 0 && (
                        <span className="text-rose-400">-{activeFile.deletions}</span>
                      )}
                    </div>

                    <button
                      type="button"
                      onClick={handleCopyPatch}
                      className="p-1 rounded hover:bg-[#27272a] text-[#a1a1aa] hover:text-white transition cursor-pointer"
                      title="Скопировать patch"
                    >
                      {copied ? (
                        <Check className="h-3.5 w-3.5 text-emerald-400" />
                      ) : (
                        <Copy className="h-3.5 w-3.5" />
                      )}
                    </button>
                  </div>
                </div>

                {/* Diff lines view */}
                <div className="flex-1 overflow-auto p-2 font-mono text-[11px] leading-relaxed select-text bg-[#0b0b0d]">
                  {parsedLines.length === 0 ? (
                    <div className="p-4 text-center text-[#71717a] text-xs">
                      Нет изменений в данном файле
                    </div>
                  ) : (
                    parsedLines.map((line, idx) => {
                      if (line.type === 'meta') {
                        return (
                          <div
                            key={idx}
                            className="text-[#52525b] text-[10px] py-0.5 px-1 truncate select-text"
                          >
                            {line.text}
                          </div>
                        );
                      }

                      if (line.type === 'hunk') {
                        return (
                          <div
                            key={idx}
                            className="bg-cyan-950/30 text-cyan-400 border-y border-cyan-900/40 my-1 py-0.5 px-2 text-[10px] font-semibold select-text"
                          >
                            {line.text}
                          </div>
                        );
                      }

                      if (line.type === 'add') {
                        return (
                          <div
                            key={idx}
                            className="flex items-start bg-[#10301a]/70 text-emerald-300 hover:bg-[#133d21] border-l-2 border-emerald-500 py-0.5"
                          >
                            <span className="w-8 shrink-0 text-right pr-2 text-[#52525b] select-none text-[10px]">
                              {/* no old line */}
                            </span>
                            <span className="w-8 shrink-0 text-right pr-2 text-emerald-500/80 select-none text-[10px] tabular-nums">
                              {line.newNum}
                            </span>
                            <span className="w-4 shrink-0 text-center text-emerald-400 font-bold select-none">
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
                            className="flex items-start bg-[#3b1219]/70 text-rose-300 hover:bg-[#48161f] border-l-2 border-rose-500 py-0.5"
                          >
                            <span className="w-8 shrink-0 text-right pr-2 text-rose-500/80 select-none text-[10px] tabular-nums">
                              {line.oldNum}
                            </span>
                            <span className="w-8 shrink-0 text-right pr-2 text-[#52525b] select-none text-[10px]">
                              {/* no new line */}
                            </span>
                            <span className="w-4 shrink-0 text-center text-rose-400 font-bold select-none">
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
                          className="flex items-start hover:bg-[#161619] py-0.5 text-[#d4d4d8]"
                        >
                          <span className="w-8 shrink-0 text-right pr-2 text-[#52525b] select-none text-[10px] tabular-nums">
                            {line.oldNum}
                          </span>
                          <span className="w-8 shrink-0 text-right pr-2 text-[#52525b] select-none text-[10px] tabular-nums">
                            {line.newNum}
                          </span>
                          <span className="w-4 shrink-0 text-center text-[#52525b] select-none">
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
              </>
            ) : (
              <div className="flex-1 flex items-center justify-center text-xs text-[#71717a]">
                Выберите файл для просмотра изменений
              </div>
            )}
          </div>
        </div>
      )}
    </aside>
  );
}
