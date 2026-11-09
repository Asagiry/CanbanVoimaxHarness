import { memo, useContext, useMemo } from 'react';
import type { DrawerActivityItem } from '../../../stores/drawerStore';
import { useTranslation } from '../../../stores/languageStore';
import { ShieldAlert, User, ZoomIn, Clock, Bot, Trash2 } from 'lucide-react';
import { MarkdownContent, LightboxContext } from './MarkdownContent';

export interface SteerDirectiveCardProps {
  activity: DrawerActivityItem;
  onImageClick?: (src: string, alt?: string) => void;
  onDelete?: (activityId: string) => void;
}

export function isSensorRejectionActivity(activity?: DrawerActivityItem): boolean {
  return Boolean(
    activity?.content?.includes('[ОТКЛОНЕНО СЕНСОРОМ') ||
    activity?.content?.includes('[SENSOR REJECTED') ||
    activity?.content?.includes('SENSOR REJECTED:')
  );
}

export function isHookRejectionActivity(activity?: DrawerActivityItem): boolean {
  return Boolean(
    activity?.content?.includes('[ОТКЛОНЕНО ХУКОМ') ||
    activity?.content?.includes('[HOOK REJECTED') ||
    activity?.content?.includes('HOOK REJECTED:')
  );
}

export function isAgentMessageActivity(activity?: DrawerActivityItem): boolean {
  if (isSensorRejectionActivity(activity) || isHookRejectionActivity(activity)) {
    return false;
  }
  return Boolean(
    activity?.type === 'agent' ||
    (activity as { source?: string } | undefined)?.source === 'agent' ||
    activity?.content?.includes('[ОТЧЕТ ОБ ИСПРАВЛЕНИЯХ') ||
    activity?.content?.includes('HANDOVER SUMMARY') ||
    activity?.content?.includes('Замечания исправлены. Отчет исполнителя:') ||
    activity?.content?.includes('### Саммари этапа') ||
    activity?.content?.includes('Задача возвращена на этап:') ||
    activity?.content?.includes('Вердикт ревьюера:') ||
    activity?.content?.includes('ОТКЛОНЕНО:') ||
    activity?.content?.includes('ОТКЛОНЕНО РЕВЬЮ')
  );
}

/**
 * SteerDirectiveCard - Displays user steering instructions, agent handover/rejection messages,
 * and gatekeeper sensor / transition hook rejections with timestamps, status badges,
 * image attachments, and markdown rendering.
 */
export const SteerDirectiveCard = memo(function SteerDirectiveCard({
  activity,
  onImageClick,
  onDelete,
}: SteerDirectiveCardProps) {
  const { t } = useTranslation();
  const contextLightbox = useContext(LightboxContext);
  const handleImageClick = onImageClick || contextLightbox || undefined;

  const images = Array.isArray(activity?.images) ? activity.images : [];
  const isSensorRejection = isSensorRejectionActivity(activity);
  const isHookRejection = isHookRejectionActivity(activity);
  const isAgentMessage = isAgentMessageActivity(activity);

  const formattedTime = useMemo(() => {
    if (activity.timestamp) {
      if (/^\d{1,2}:\d{2}(:\d{2})?$/.test(activity.timestamp.trim())) {
        return activity.timestamp.trim();
      }
      const d = new Date(activity.timestamp);
      if (!isNaN(d.getTime())) {
        return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false });
      }
    }
    if (activity.createdAt) {
      const d = new Date(activity.createdAt);
      if (!isNaN(d.getTime())) {
        return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false });
      }
    }
    return null;
  }, [activity.timestamp, activity.createdAt]);

  if (isSensorRejection || isHookRejection) {
    const titleKey = isSensorRejection ? 'drawer.sensorRejection' : 'drawer.hookRejection';
    return (
      <div className="animate-silk-entrance motion-reduce:animate-none rounded-lg border border-amber-500/20 bg-amber-500/10 p-3.5 shadow-xs min-w-0 overflow-hidden">
        <div className="flex items-center justify-between pb-1.5 mb-2 border-b border-amber-500/20 text-[10px] font-mono">
          <div className="flex items-center gap-1.5 leading-none text-amber-500 dark:text-amber-400 font-medium">
            <ShieldAlert className="h-3.5 w-3.5 block shrink-0" />
            <span>{t(titleKey)}</span>
          </div>
          {formattedTime && (
            <span className="flex items-center gap-1 leading-none text-amber-600 dark:text-amber-400 tabular-nums">
              <Clock className="h-3 w-3 block shrink-0" />
              <span>{formattedTime}</span>
            </span>
          )}
        </div>
        {images.length > 0 && (
          <div className="flex items-center gap-2 mb-2.5 flex-wrap">
            {images.map((imgUrl, i) => (
              <div
                key={i}
                onClick={() => handleImageClick?.(imgUrl, `${t('drawer.attachImage')} ${i + 1}`)}
                className="relative rounded-lg overflow-hidden border border-amber-500/20 max-w-[240px] max-h-40 bg-[#f4f4f5] dark:bg-[#17171b] shadow-xs group cursor-zoom-in"
              >
                <img
                  src={imgUrl}
                  alt={`${t('drawer.attachImage')} ${i + 1}`}
                  className="w-full h-full max-h-40 object-cover group-hover:scale-105 transition-transform duration-200 cursor-zoom-in"
                />
                <div className="absolute inset-0 bg-black/0 group-hover:bg-black/20 transition-colors flex items-center justify-center opacity-0 group-hover:opacity-100 pointer-events-none">
                  <span className="p-1 rounded-full bg-black/60 text-white shadow-xs">
                    <ZoomIn className="w-3.5 h-3.5" />
                  </span>
                </div>
              </div>
            ))}
          </div>
        )}
        <div className="text-sm text-[#171717] dark:text-[#f4f4f5] leading-relaxed select-text font-sans min-w-0 break-words">
          <MarkdownContent content={activity?.content ?? ''} onImageClick={handleImageClick} />
        </div>
      </div>
    );
  }

  return (
    <div className="animate-silk-entrance motion-reduce:animate-none rounded-lg border border-[#ebebeb] dark:border-[#212126] bg-[#f9f9f9] dark:bg-[#111114] p-3.5 shadow-xs min-w-0 overflow-hidden">
      <div className="flex items-center justify-between pb-1.5 mb-2 border-b border-[#ebebeb] dark:border-[#212126] text-[10px] font-mono text-[#666666] dark:text-[#71717a]">
        <div className="flex items-center gap-1.5 leading-none text-[#171717] dark:text-[#f4f4f5] font-medium">
          {isAgentMessage ? (
            <>
              <Bot className="h-3.5 w-3.5 block shrink-0 text-blue-500 dark:text-blue-400" />
              <span>{t('drawer.agentMessage')}</span>
            </>
          ) : (
            <>
              <User className="h-3 w-3 block shrink-0" />
              <span>{t('drawer.userSteer')}</span>
            </>
          )}
        </div>
        <div className="flex items-center gap-2 leading-none">
          {onDelete && !isAgentMessage && (
            <button
              type="button"
              onClick={() => onDelete(activity.id)}
              className="p-0.5 rounded text-[#71717a] hover:text-red-500 hover:bg-[#e4e4e7] dark:hover:bg-[#232328] transition cursor-pointer"
              title={t('drawer.deleteMessage') || 'Удалить сообщение'}
            >
              <Trash2 className="h-3 w-3 block shrink-0" />
            </button>
          )}
          {formattedTime && (
            <span className="flex items-center gap-1 leading-none text-[#71717a] dark:text-[#71717a] tabular-nums">
              <Clock className="h-3 w-3 block shrink-0 text-[#a1a1aa] dark:text-[#52525b]" />
              <span>{formattedTime}</span>
            </span>
          )}
        </div>
      </div>
      {images.length > 0 && (
        <div className="flex items-center gap-2 mb-2.5 flex-wrap">
          {images.map((imgUrl, i) => (
            <div
              key={i}
              onClick={() => handleImageClick?.(imgUrl, `${t('drawer.attachImage')} ${i + 1}`)}
              className="relative rounded-lg overflow-hidden border border-[#ebebeb] dark:border-[#212126] max-w-[240px] max-h-40 bg-[#f4f4f5] dark:bg-[#17171b] shadow-xs group cursor-zoom-in"
            >
              <img
                src={imgUrl}
                alt={`${t('drawer.attachImage')} ${i + 1}`}
                className="w-full h-full max-h-40 object-cover group-hover:scale-105 transition-transform duration-200 cursor-zoom-in"
              />
              <div className="absolute inset-0 bg-black/0 group-hover:bg-black/20 transition-colors flex items-center justify-center opacity-0 group-hover:opacity-100 pointer-events-none">
                <span className="p-1 rounded-full bg-black/60 text-white shadow-xs">
                  <ZoomIn className="w-3.5 h-3.5" />
                </span>
              </div>
            </div>
          ))}
        </div>
      )}
      <div className="text-sm text-[#171717] dark:text-[#f4f4f5] leading-relaxed select-text font-sans min-w-0 break-words">
        <MarkdownContent content={activity?.content ?? ''} onImageClick={handleImageClick} />
      </div>
    </div>
  );
});

export default SteerDirectiveCard;
