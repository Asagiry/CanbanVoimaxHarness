import { memo, useContext, useMemo } from 'react';
import type { DrawerActivityItem } from '../../../stores/drawerStore';
import { useTranslation } from '../../../stores/languageStore';
import { ShieldAlert, User, ZoomIn, Clock } from 'lucide-react';
import { MarkdownContent, LightboxContext } from '../DrawerActivityStream';

export interface SteerDirectiveCardProps {
  activity: DrawerActivityItem;
  onImageClick?: (src: string, alt?: string) => void;
}

/**
 * SteerDirectiveCard - Displays user steering instructions and gatekeeper sensor rejections
 * with timestamps, status badges, image attachments, and markdown rendering.
 */
export const SteerDirectiveCard = memo(function SteerDirectiveCard({
  activity,
  onImageClick,
}: SteerDirectiveCardProps) {
  const { t } = useTranslation();
  const contextLightbox = useContext(LightboxContext);
  const handleImageClick = onImageClick || contextLightbox || undefined;

  const images = Array.isArray(activity?.images) ? activity.images : [];
  const isSensorRejection = Boolean(
    activity?.content?.includes('[ОТКЛОНЕНО СЕНСОРОМ') ||
    activity?.content?.includes('[SENSOR REJECTED') ||
    activity?.content?.includes('SENSOR REJECTED:')
  );

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

  if (isSensorRejection) {
    return (
      <div className="animate-silk-entrance motion-reduce:animate-none rounded-lg border border-amber-500/20 bg-amber-500/10 p-3.5 shadow-xs">
        <div className="flex items-center justify-between pb-1.5 mb-2 border-b border-amber-500/20 text-[10px] font-mono">
          <div className="flex items-center space-x-1.5 text-amber-500 dark:text-amber-400 font-medium">
            <ShieldAlert className="h-3.5 w-3.5 shrink-0" />
            <span>{t('drawer.sensorRejection')}</span>
          </div>
          {formattedTime && (
            <span className="flex items-center space-x-1 text-amber-600 dark:text-amber-400 tabular-nums">
              <Clock className="h-3 w-3 shrink-0" />
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
                className="relative rounded-lg overflow-hidden border border-amber-500/20 max-w-[240px] max-h-40 bg-[#f4f4f5] dark:bg-[#161b22] shadow-xs group cursor-zoom-in"
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
        <div className="text-sm text-[#171717] dark:text-[#f0f6fc] leading-relaxed select-text font-sans">
          <MarkdownContent content={activity?.content ?? ''} onImageClick={handleImageClick} />
        </div>
      </div>
    );
  }

  return (
    <div className="animate-silk-entrance motion-reduce:animate-none rounded-lg border border-[#ebebeb] dark:border-[#21262d] bg-[#f9f9f9] dark:bg-[#0d1117] p-3.5 shadow-xs">
      <div className="flex items-center justify-between pb-1.5 mb-2 border-b border-[#ebebeb] dark:border-[#21262d] text-[10px] font-mono text-[#666666] dark:text-[#8b949e]">
        <div className="flex items-center space-x-1.5 text-[#171717] dark:text-[#f0f6fc] font-medium">
          <User className="h-3 w-3" />
          <span>{t('drawer.userSteer')}</span>
          <span className="px-1.5 py-0.2 rounded text-[9px] font-mono uppercase bg-[#ebebeb] dark:bg-[#21262d] text-[#666666] dark:text-[#8b949e]">
            STEER
          </span>
        </div>
        {formattedTime && (
          <span className="flex items-center space-x-1 text-[#71717a] dark:text-[#8b949e] tabular-nums">
            <Clock className="h-3 w-3 shrink-0 text-[#a1a1aa] dark:text-[#6e7681]" />
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
              className="relative rounded-lg overflow-hidden border border-[#ebebeb] dark:border-[#21262d] max-w-[240px] max-h-40 bg-[#f4f4f5] dark:bg-[#161b22] shadow-xs group cursor-zoom-in"
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
      <div className="text-sm text-[#171717] dark:text-[#f0f6fc] leading-relaxed select-text font-sans">
        <MarkdownContent content={activity?.content ?? ''} onImageClick={handleImageClick} />
      </div>
    </div>
  );
});

export default SteerDirectiveCard;
