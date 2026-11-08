import { useState, type ReactNode } from 'react';
import { Trash2 } from 'lucide-react';
import { useDrawerStore } from '../../../stores/drawerStore';
import { useTranslation } from '../../../stores/languageStore';

export default function ActivityItemActions({ taskId, itemId, disabled, children }: {
  taskId: string;
  itemId: string;
  disabled: boolean;
  children: ReactNode;
}) {
  const { t } = useTranslation();
  const [deleting, setDeleting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const remove = async () => {
    setDeleting(true);
    setError(null);
    try {
      await useDrawerStore.getState().deleteDialogItem!(taskId, itemId);
    } catch (err) {
      setError(String(err));
    } finally {
      setDeleting(false);
    }
  };
  return (
    <div className="group/activity min-w-0">
      <div className="flex items-center gap-1.5 min-w-0">
        <div className="flex-1 min-w-0">{children}</div>
        <button
          type="button"
          onClick={() => void remove()}
          disabled={disabled || deleting}
          title={disabled ? t('drawer.stopBeforeDelete') : t('drawer.deleteMessage')}
          aria-label={t('drawer.deleteMessage')}
          className="self-center p-1.5 shrink-0 rounded text-[#71717a] hover:text-red-500 hover:bg-[#f4f4f5] dark:hover:bg-[#232328] disabled:cursor-not-allowed disabled:opacity-30 opacity-50 group-hover/activity:opacity-100 focus-visible:opacity-100 transition"
        >
          <Trash2 className="h-3.5 w-3.5 leading-none" />
        </button>
      </div>
      {error && <p role="alert" className="mt-1 text-xs text-red-600 dark:text-red-400 break-words">{error}</p>}
    </div>
  );
}
