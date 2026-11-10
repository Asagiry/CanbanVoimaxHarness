import { ChevronDown, ChevronRight } from 'lucide-react';
import type { CortexAgentStage } from '../../../types';
import { useTranslation } from '../../../stores/languageStore';
import { useCortexStore } from '../../../stores/cortexStore';
import { useBoardStore } from '../../../stores/boardStore';
import { isColumnDirty } from '../../../stores/cortex/cortexGitSave';
import { isBacklogSystemStage } from '../types';
import ColumnStageCard from '../ColumnStageCard';
import DirtyDot from '../DirtyDot';

export interface ColumnsExplorerSectionProps {
  stages: CortexAgentStage[];
  totalCount: number;
  isOpen: boolean;
  onToggle: () => void;
  isSearching: boolean;
}

/**
 * Collapsible "Columns" section inside CortexExplorer.
 */
export function ColumnsExplorerSection({
  stages,
  totalCount,
  isOpen,
  onToggle,
  isSearching,
}: ColumnsExplorerSectionProps) {
  const { t } = useTranslation();
  const allStages = useCortexStore((s) => s.stages);
  const focusedStageId = useCortexStore((s) => s.focusedStageId);
  const cvhGitStatus = useCortexStore((s) => s.cvhGitStatus);
  const pendingColumnChanges = useCortexStore((s) => s.pendingColumnChanges);
  const columns = useBoardStore((s) => s.columns);

  const sectionDirty = allStages.some((s) =>
    isColumnDirty(
      s,
      columns.find((c) => c.id === s.stageId),
      cvhGitStatus,
      pendingColumnChanges,
    ),
  );

  return (
    <section aria-label={t('cortex.columnsSection')}>
      <div className="w-full flex items-center justify-between px-1.5 py-1 rounded-md text-[#171717] dark:text-[#f4f4f5] hover:bg-[#f4f4f5] dark:hover:bg-[#1e1e23] transition group">
        <button
          type="button"
          onClick={onToggle}
          aria-expanded={isOpen}
          className="flex-1 flex items-center gap-1.5 py-1 text-left cursor-pointer"
        >
          {isOpen ? (
            <ChevronDown className="h-3.5 w-3.5 text-[#666666] dark:text-[#71717a] shrink-0" />
          ) : (
            <ChevronRight className="h-3.5 w-3.5 text-[#666666] dark:text-[#71717a] shrink-0" />
          )}
          <span className="font-sans text-xs font-semibold leading-none">
            {t('cortex.columnsSection')}
          </span>
          <span className="text-[10px] font-mono text-[#8f8f8f] dark:text-[#71717a] tabular-nums leading-none">
            ({totalCount})
          </span>
          {sectionDirty && <DirtyDot />}
        </button>
      </div>

      {isOpen && (
        <div className="space-y-1">
          {totalCount === 0 && (
            <div className="mx-1 mt-1 rounded-md border border-dashed border-[#d4d4d4] dark:border-[#2c2c33] bg-white dark:bg-[#17171b] px-3 py-4 text-center space-y-1.5">
              <div className="text-xs font-sans font-semibold text-[#171717] dark:text-[#f4f4f5]">
                {t('cortex.noAgentsInColumns')}
              </div>
              <div className="text-[11px] font-sans text-[#666666] dark:text-[#71717a] leading-relaxed">
                {t('cortex.createFirstColumnAgent')}
              </div>
              <div className="text-[10px] font-mono text-[#8f8f8f] dark:text-[#71717a]">
                {t('cortex.freshProjectNotice')}
              </div>
            </div>
          )}

          {totalCount > 0 && stages.length === 0 && (
            <div className="mx-1 mt-1 rounded-md border border-[#ebebeb] dark:border-[#2c2c33] bg-white dark:bg-[#17171b] px-3 py-4 text-center">
              <div className="text-[11px] font-sans text-[#666666] dark:text-[#71717a]">
                {t('cortex.columnsNothingFound')}
              </div>
            </div>
          )}

          {stages.map((stage) => {
            const backlog = isBacklogSystemStage(stage);
            const column = columns.find((c) => c.id === stage.stageId);
            return (
              <ColumnStageCard
                key={stage.stageId}
                stage={stage}
                column={column}
                isBacklogSystem={backlog}
                isFocused={focusedStageId === stage.stageId}
                isSearching={isSearching}
              />
            );
          })}
        </div>
      )}
    </section>
  );
}
