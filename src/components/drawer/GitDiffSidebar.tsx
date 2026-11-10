import { AlertCircle } from 'lucide-react';
import { useTranslation } from '../../stores/languageStore';
import { DiffFileList } from './diff';
import {
  useDiffResize,
  useGitDiffData,
  DiffCollapsedRail,
  DiffHeader,
} from './diff/sidebar';

export { ImageDiffViewer, isImageFile } from './diff';
export * from './diff/sidebar';

export interface GitDiffSidebarProps {
  taskId: string;
  isCollapsed: boolean;
  onToggleCollapse: () => void;
  className?: string;
}

export default function GitDiffSidebar({
  taskId,
  isCollapsed,
  onToggleCollapse,
  className = '',
}: GitDiffSidebarProps) {
  const { t } = useTranslation();
  const { sidebarWidth, isResizing, handleMouseDown } = useDiffResize();

  const {
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
  } = useGitDiffData(taskId);

  return (
    <aside
      style={{ width: (isCollapsed ? 44 : sidebarWidth) + 'px' }}
      className={'relative shrink-0 my-1.5 mr-1.5 rounded-xl border border-[#ebebeb] dark:border-[#212126] shadow-2xs bg-[#fafafa] dark:bg-[#0b0b0e] flex flex-col min-h-0 h-[calc(100%-0.75rem)] text-[#171717] dark:text-[#f4f4f5] overflow-hidden select-none transition-colors ' +
        (isResizing ? 'select-none' : 'transition-[width] duration-200 ease-out') + ' ' + (className || '')}
      aria-label={t('gitDiff.panelAria')}
    >
      {/* Collapsed Rail View */}
      <DiffCollapsedRail
        isCollapsed={isCollapsed}
        onToggleCollapse={onToggleCollapse}
        isLoading={isLoading}
        hasInitialLoaded={hasInitialLoaded}
        filesCount={filesCount}
        totalAdditions={totalAdditions}
        totalDeletions={totalDeletions}
      />

      {/* Expanded Content View */}
      <div
        style={{ width: sidebarWidth + 'px' }}
        className={'h-full flex flex-col min-h-0 overflow-hidden transition-opacity duration-200 ' +
          (isCollapsed ? 'opacity-0 pointer-events-none' : 'opacity-100 pointer-events-auto')}
      >
        {/* Left Resize Drag Handle */}
        {!isCollapsed && (
          <div
            onMouseDown={handleMouseDown}
            className="group/handle absolute top-0 bottom-0 left-0 w-2 cursor-col-resize z-30 flex items-center justify-center select-none"
            aria-label="Изменить ширину панели Git Diff"
          >
            <div
              className={'w-0.5 h-8 rounded-full transition-colors ' +
                (isResizing ? 'bg-emerald-500' : 'bg-transparent group-hover/handle:bg-[#00000025]')}
            />
          </div>
        )}

        {/* Header */}
        <DiffHeader
          filesCount={filesCount}
          totalAdditions={totalAdditions}
          totalDeletions={totalDeletions}
          branch={diff?.branch}
          isLoading={isLoading}
          onExpandAll={handleExpandAll}
          onCollapseAll={handleCollapseAll}
          onRefresh={fetchDiff}
          onToggleCollapse={onToggleCollapse}
        />

        {/* Error state */}
        {error && (
          <div className="p-2.5 bg-[#fef2f2] dark:bg-[#450a0a]/40 border-b border-[#fecaca] dark:border-[#991b1b] text-[#991b1b] dark:text-[#f87171] text-xs font-mono flex items-center space-x-2 shrink-0">
            <AlertCircle className="h-4 w-4 text-[#dc2626] shrink-0" />
            <span className="flex-1 truncate">{error}</span>
            <button
              type="button"
              onClick={fetchDiff}
              className="underline hover:text-[#171717] dark:hover:text-[#f4f4f5] cursor-pointer ml-1 text-[11px]"
            >
              Повторить
            </button>
          </div>
        )}

        {/* Diff File List */}
        <DiffFileList
          taskId={taskId}
          files={displayFiles}
          hasInitialLoaded={hasInitialLoaded}
          expandedFiles={expandedFiles}
          onToggleFile={toggleFile}
          branch={diff?.branch}
        />
      </div>
    </aside>
  );
}
