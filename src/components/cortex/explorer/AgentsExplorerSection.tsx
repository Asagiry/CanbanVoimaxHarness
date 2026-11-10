import { ChevronDown, ChevronRight, Plus } from 'lucide-react';
import type { CortexAgentStage } from '../../../types';
import { useTranslation } from '../../../stores/languageStore';
import { useCortexStore } from '../../../stores/cortexStore';
import { isAgentDirty } from '../../../stores/cortex/cortexGitSave';
import AgentMindInspector from '../AgentMindInspector';
import DirtyDot from '../DirtyDot';

export interface AgentsExplorerSectionProps {
  stages: CortexAgentStage[];
  totalCount: number;
  isOpen: boolean;
  onToggle: () => void;
  isSearching: boolean;
  onCreateAgent: () => void;
  onAgentContextMenu: (e: React.MouseEvent, stage: CortexAgentStage) => void;
}

/**
 * Collapsible "Agents" section inside CortexExplorer.
 */
export function AgentsExplorerSection({
  stages,
  totalCount,
  isOpen,
  onToggle,
  isSearching,
  onCreateAgent,
  onAgentContextMenu,
}: AgentsExplorerSectionProps) {
  const { t } = useTranslation();
  const allAgentStages = useCortexStore((s) => s.agentStages);
  const focusedStageId = useCortexStore((s) => s.focusedStageId);
  const cvhGitStatus = useCortexStore((s) => s.cvhGitStatus);
  const pendingDeletes = useCortexStore((s) => s.pendingDeletes);

  const sectionDirty = allAgentStages.some((s) =>
    isAgentDirty(s, cvhGitStatus, pendingDeletes),
  );

  return (
    <section aria-label={t('cortex.agentsSection')}>
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
            {t('cortex.agentsSection')}
          </span>
          <span className="text-[10px] font-mono text-[#8f8f8f] dark:text-[#71717a] tabular-nums leading-none">
            ({totalCount})
          </span>
          {sectionDirty && <DirtyDot />}
        </button>
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            onCreateAgent();
          }}
          aria-label={t('cortex.createAgent')}
          className="p-1 rounded-md text-[#8f8f8f] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] hover:bg-[#ebebeb]/60 dark:hover:bg-[#232328] transition cursor-pointer shrink-0"
        >
          <Plus className="h-3.5 w-3.5" />
        </button>
      </div>

      {isOpen && (
        <div className="space-y-1">
          {totalCount === 0 && (
            <div className="mx-1 mt-1 rounded-md border border-dashed border-[#d4d4d4] dark:border-[#2c2c33] bg-white dark:bg-[#17171b] px-3 py-4 text-center space-y-1.5">
              <div className="text-xs font-sans font-semibold text-[#171717] dark:text-[#f4f4f5]">
                {t('cortex.noAvailableAgents')}
              </div>
              <div className="text-[11px] font-sans text-[#666666] dark:text-[#71717a] leading-relaxed">
                {t('cortex.agentsFromProject')}
              </div>
            </div>
          )}

          {totalCount > 0 && stages.length === 0 && (
            <div className="mx-1 mt-1 rounded-md border border-[#ebebeb] dark:border-[#2c2c33] bg-white dark:bg-[#17171b] px-3 py-4 text-center">
              <div className="text-[11px] font-sans text-[#666666] dark:text-[#71717a]">
                {t('cortex.agentsNothingFound')}
              </div>
            </div>
          )}

          {stages.map((stage) => (
            <AgentMindInspector
              key={stage.stageId}
              stage={stage}
              isFocused={focusedStageId === stage.stageId}
              isSearching={isSearching}
              onContextMenu={(e) => onAgentContextMenu(e, stage)}
            />
          ))}
        </div>
      )}
    </section>
  );
}
