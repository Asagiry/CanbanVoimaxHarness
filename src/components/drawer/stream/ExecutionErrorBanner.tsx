import { useTranslation } from '../../../stores/languageStore';
import { AlertTriangle, Play, X } from 'lucide-react';

export interface ExecutionErrorBannerProps {
  error: string;
  onRetry: () => void;
  onDismiss: () => void;
}

export function ExecutionErrorBanner({
  error,
  onRetry,
  onDismiss,
}: ExecutionErrorBannerProps) {
  const { t } = useTranslation();

  return (
    <div className="mx-4 md:mx-6 mt-3 px-3.5 py-2.5 rounded-lg bg-[#fef2f2] dark:bg-[#3b1219] border border-[#fecaca] dark:border-[#da3633] text-[#991b1b] dark:text-[#ff7b72] flex items-center justify-between shadow-xs shrink-0 animate-in fade-in duration-150">
      <div className="flex items-center space-x-2.5 min-w-0 mr-3">
        <AlertTriangle className="h-4 w-4 text-[#dc2626] dark:text-[#ff7b72] shrink-0" />
        <div className="text-xs font-medium font-sans truncate">
          <span className="font-semibold">{t('drawer.executionError')}</span> {error}
        </div>
      </div>
      <div className="flex items-center space-x-2 shrink-0">
        <button
          type="button"
          onClick={onRetry}
          className="flex items-center space-x-1.5 px-3 py-1 rounded-md bg-[#dc2626] dark:bg-[#da3633] hover:bg-[#b91c1c] text-white text-xs font-medium transition cursor-pointer shadow-xs active:scale-[0.98]"
        >
          <Play className="h-3 w-3 fill-current" />
          <span>{t('drawer.retryRun')}</span>
        </button>
        <button
          type="button"
          onClick={onDismiss}
          className="p-1 text-[#dc2626] dark:text-[#ff7b72] hover:text-[#991b1b] dark:hover:text-[#ffa198] rounded transition cursor-pointer"
          aria-label={t('drawer.closeNotification')}
        >
          <X className="h-3.5 w-3.5" />
        </button>
      </div>
    </div>
  );
}
