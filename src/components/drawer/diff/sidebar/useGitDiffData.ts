import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { TaskGitDiff } from '../../../../types';
import { isCvhPath } from '../utils';
import { listenToEvent } from '../../../../services/tauriEventGateway';

export function useGitDiffData(taskId: string) {
  const [diff, setDiff] = useState<TaskGitDiff | null>(null);
  const [expandedFiles, setExpandedFiles] = useState<Set<string>>(() => new Set());
  const [isLoading, setIsLoading] = useState(false);
  const [hasInitialLoaded, setHasInitialLoaded] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const hasInitialLoadedRef = useRef(false);
  const userCollapsedFilesRef = useRef<Set<string>>(new Set());
  const debounceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Reset initial tracking on task switch
  useEffect(() => {
    hasInitialLoadedRef.current = false;
    setHasInitialLoaded(false);
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
        const filteredValidPaths = new Set(
          data.files.filter((f) => f && !isCvhPath(f.path)).map((f) => f.path)
        );
        if (!hasInitialLoadedRef.current) {
          setExpandedFiles(new Set());
        } else {
          setExpandedFiles((prev) => {
            const next = new Set<string>();
            let changed = false;
            for (const path of prev) {
              if (filteredValidPaths.has(path) && !userCollapsedFilesRef.current.has(path)) {
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
    } catch (err: unknown) {
      console.info('get_task_git_diff invoke failed or web mode:', err);
      const msg =
        typeof err === 'string'
          ? err
          : err instanceof Error
          ? err.message
          : 'Ошибка загрузки Git Diff';
      setError(msg);
    } finally {
      hasInitialLoadedRef.current = true;
      setHasInitialLoaded(true);
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

  useEffect(() => {
    fetchDiff();
  }, [fetchDiff]);

  const MODIFYING_TOOLS = useMemo(
    () => new Set(['write_file', 'edit_file', 'str_replace', 'execute_command', 'bash']),
    []
  );

  useEffect(() => {
    if (!taskId) return;
    let isDisposed = false;
    let unlistenToolResult: (() => void) | undefined;
    let unlistenFinish: (() => void) | undefined;

    const setupListeners = async () => {
      try {
        if (isDisposed) return;

        const uResult = await listenToEvent('agent:tool_result', (p) => {
          const eventTaskId = p?.taskId;
          const isMatchingTask =
            eventTaskId === taskId ||
            (eventTaskId && (eventTaskId.startsWith(taskId + '.') || eventTaskId.startsWith(taskId + ':')));
          if (!isMatchingTask) return;

          const toolName = (p?.name || '').toLowerCase().trim();
          if (MODIFYING_TOOLS.has(toolName)) {
            scheduleFetch();
          }
        });
        if (isDisposed) {
          uResult();
        } else {
          unlistenToolResult = uResult;
        }

        const uFinish = await listenToEvent('agent:finished', (p) => {
          const eventTaskId = p?.taskId;
          const isMatchingTask =
            eventTaskId === taskId ||
            (eventTaskId && (eventTaskId.startsWith(taskId + '.') || eventTaskId.startsWith(taskId + ':')));
          if (isMatchingTask) {
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
      if (unlistenToolResult) unlistenToolResult();
      if (unlistenFinish) unlistenFinish();
      if (debounceTimerRef.current) {
        clearTimeout(debounceTimerRef.current);
      }
    };
  }, [taskId, scheduleFetch, MODIFYING_TOOLS]);

  const toggleFile = (path: string) => {
    setExpandedFiles((prev) => {
      const next = new Set(prev);
      if (next.has(path)) {
        next.delete(path);
        userCollapsedFilesRef.current.add(path);
      } else {
        next.add(path);
        userCollapsedFilesRef.current.delete(path);
      }
      return next;
    });
  };

  const rawFiles = diff?.files ?? [];
  const displayFiles = useMemo(
    () => rawFiles.filter((f) => f && !isCvhPath(f.path)),
    [rawFiles]
  );

  const totalAdditions = useMemo(
    () => displayFiles.reduce((acc, f) => acc + (f.additions ?? 0), 0),
    [displayFiles]
  );
  const totalDeletions = useMemo(
    () => displayFiles.reduce((acc, f) => acc + (f.deletions ?? 0), 0),
    [displayFiles]
  );
  const filesCount = displayFiles.length;

  const handleExpandAll = () => {
    if (!displayFiles.length) return;
    userCollapsedFilesRef.current.clear();
    setExpandedFiles(new Set(displayFiles.map((f) => f.path)));
  };

  const handleCollapseAll = () => {
    if (!displayFiles.length) return;
    displayFiles.forEach((f) => userCollapsedFilesRef.current.add(f.path));
    setExpandedFiles(new Set());
  };

  return {
    diff,
    displayFiles,
    filesCount,
    totalAdditions,
    totalDeletions,
    expandedFiles,
    isLoading,
    hasInitialLoaded,
    error,
    fetchDiff,
    toggleFile,
    handleExpandAll,
    handleCollapseAll,
  };
}
