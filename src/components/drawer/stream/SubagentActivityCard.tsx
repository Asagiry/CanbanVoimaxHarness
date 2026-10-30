import { memo, useContext } from 'react';
import type { DrawerActivityItem } from '../../../stores/drawerStore';
import { useCortexStore } from '../../../stores/cortexStore';
import { useTranslation } from '../../../stores/languageStore';
import { getRoleBadgeStyle } from '../../../lib/agentPalette';
import AgentIcon from '../../common/AgentIcon';
import { formatSubagentDisplayTitle } from '../SubagentsSidebar';
import { Paperclip, ZoomIn, Clock } from 'lucide-react';
import { MarkdownContent, LightboxContext } from './MarkdownContent';
import { formatDurationCompact } from './ThoughtStreamCard';

export interface SubagentActivityCardProps {
  // Mode A: Active subagent objective view (at top of drawer feed)
  activeSubagentId?: string | null;
  activeSubagentTitle?: string | null;
  activeSubagentRole?: string | null;
  activeSubagentPrompt?: string | null;
  effectiveAttachments?: string[];
  onImageClick?: (src: string, alt?: string) => void;

  // Mode B: Subagent activity / report item in stream
  activity?: DrawerActivityItem;
  durationMs?: number;
}

/**
 * SubagentActivityCard - Renders atomic objective cards and activity reports for nested subagents,
 * with role badges, persona icons, Markdown instructions/findings, and image attachments.
 */
export const SubagentActivityCard = memo(function SubagentActivityCard({
  activeSubagentId: _activeSubagentId,
  activeSubagentTitle,
  activeSubagentRole,
  activeSubagentPrompt,
  effectiveAttachments,
  onImageClick,
  activity,
  durationMs,
}: SubagentActivityCardProps) {
  const { t } = useTranslation();
  const contextLightbox = useContext(LightboxContext);
  const handleImageClick = onImageClick || contextLightbox || undefined;

  const agentPersonas = useCortexStore((s) => s.agentPersonas);

  // Mode B: Activity item in feed (report or step from subagent)
  if (activity) {
    const params = (activity.tool?.params as Record<string, unknown> | undefined) || {};
    const role = (params.role as string) || 'general';
    const rawTitle = (params.title as string) || (params.prompt as string) || '';
    const displayTitle = rawTitle ? formatSubagentDisplayTitle(rawTitle) : (activity.content ? t('drawer.agentExplanation') : 'Subagent Report');
    const matchedPersona = agentPersonas.find(
      (p) => p.role.toLowerCase() === role.toLowerCase()
    );
    const effectiveDuration = durationMs || activity.durationMs || activity.tool?.durationMs;
    const durationText = formatDurationCompact(effectiveDuration);

    return (
      <div className="animate-silk-entrance motion-reduce:animate-none rounded-lg border border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#0d1117] p-3.5 shadow-xs my-2">
        <div className="flex items-center justify-between pb-2 mb-2 border-b border-[#ebebeb] dark:border-[#21262d]">
          <div className="flex items-center space-x-2 min-w-0">
            <AgentIcon name={matchedPersona?.icon} className="h-3.5 w-3.5 text-[#171717] dark:text-[#f0f6fc] shrink-0" />
            <span className="text-xs font-semibold text-[#171717] dark:text-[#f0f6fc] font-sans truncate">
              {displayTitle}
            </span>
            <span
              style={getRoleBadgeStyle(role, matchedPersona?.color)}
              className="px-1.5 py-0.5 rounded text-[9px] font-mono font-bold uppercase tracking-wider border shrink-0"
            >
              {role.toUpperCase()}
            </span>
          </div>
          {durationText && (
            <span className="flex items-center space-x-1 text-[10px] font-mono text-[#71717a] dark:text-[#8b949e] tabular-nums shrink-0">
              <Clock className="h-3 w-3 text-[#a1a1aa] dark:text-[#6e7681]" />
              <span>{durationText}</span>
            </span>
          )}
        </div>
        {activity.content && (
          <div className="text-sm text-[#171717] dark:text-[#f0f6fc] leading-relaxed select-text font-sans">
            <MarkdownContent content={activity.content} onImageClick={handleImageClick} />
          </div>
        )}
      </div>
    );
  }

  // Mode A: Subagent Atomic Objective Card (displayed when viewing active subagent)
  const matchedPersona = agentPersonas.find(
    (p) => p.role.toLowerCase() === (activeSubagentRole || '').toLowerCase()
  );

  return (
    <div className="rounded-lg border border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#0d1117] p-4 shadow-xs">
      <div className="flex items-center justify-between pb-2 mb-2.5 border-b border-[#ebebeb] dark:border-[#21262d]">
        <div className="flex items-center space-x-2 min-w-0">
          <AgentIcon name={matchedPersona?.icon} className="h-4 w-4 text-[#171717] dark:text-[#f0f6fc] shrink-0" />
          <span className="text-xs font-semibold text-[#171717] dark:text-[#f0f6fc] font-sans truncate">
            {t('drawer.subagentObjective', { title: formatSubagentDisplayTitle(activeSubagentTitle) })}
          </span>
        </div>
        <div className="flex items-center space-x-2 shrink-0">
          <span
            style={getRoleBadgeStyle(activeSubagentRole, matchedPersona?.color)}
            className="px-1.5 py-0.5 rounded text-[9px] font-mono font-bold uppercase tracking-wider border"
          >
            {(activeSubagentRole || 'general').toUpperCase()}
          </span>
        </div>
      </div>
      <div className="text-sm text-[#171717] dark:text-[#f0f6fc] leading-relaxed select-text font-sans bg-[#fafafa] dark:bg-[#161b22] p-3 rounded-md border border-[#ebebeb] dark:border-[#30363d]">
        <MarkdownContent
          content={activeSubagentPrompt || t('drawer.subagentInstruction')}
          onImageClick={handleImageClick}
        />
      </div>

      {/* Subagent Attachments Gallery */}
      {effectiveAttachments && effectiveAttachments.length > 0 && (
        <div className="mt-3 pt-3 border-t border-[#ebebeb] dark:border-[#21262d]">
          <div className="text-[11px] font-mono font-medium text-[#666666] dark:text-[#8b949e] mb-2 flex items-center gap-1.5">
            <Paperclip className="h-3 w-3" />
            <span>{t('newTask.attachmentsLabel')} ({effectiveAttachments.length})</span>
          </div>
          <div className="flex flex-wrap gap-2">
            {effectiveAttachments.map((att, idx) => (
              <div
                key={idx}
                onClick={() => handleImageClick?.(att, `Attachment ${idx + 1}`)}
                className="relative group rounded-md overflow-hidden border border-[#ebebeb] dark:border-[#30363d] bg-[#fafafa] dark:bg-[#161b22] cursor-pointer hover:border-[#171717] dark:hover:border-[#f0f6fc] transition shadow-2xs"
              >
                <img
                  src={att}
                  alt={`Attachment ${idx + 1}`}
                  className="h-20 w-28 object-cover transition-transform group-hover:scale-105"
                />
                <div className="absolute inset-0 bg-black/0 group-hover:bg-black/20 flex items-center justify-center transition-opacity opacity-0 group-hover:opacity-100">
                  <ZoomIn className="h-4 w-4 text-white drop-shadow-md" />
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
});

export default SubagentActivityCard;
