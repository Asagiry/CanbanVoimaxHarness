import { useTranslation } from '../../stores/languageStore';

export default function DirtyDot() {
  const { t } = useTranslation();
  return (
    <span
      className="h-1.5 w-1.5 rounded-full bg-amber-500 shrink-0"
      title={t('cortex.unsavedChanges')}
      aria-label={t('cortex.unsavedChanges')}
    />
  );
}
