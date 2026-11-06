import { Plus } from 'lucide-react';
import { useSettingsStore } from '../../../stores/settingsStore';
import { useTranslation } from '../../../stores/languageStore';
import type { Provider } from '../../../stores/settings/providers';
import type { ProviderId } from '../../../types/domain';

interface ProviderListProps {
  onEdit: (provider: Provider) => void;
  onAdd: () => void;
}

export default function ProviderList({ onEdit, onAdd }: ProviderListProps) {
  const providers = useSettingsStore((s) => s.providers);
  const activeProviderId = useSettingsStore((s) => s.activeProviderId);
  const deleteProvider = useSettingsStore((s) => s.deleteProvider);
  const setActiveProvider = useSettingsStore((s) => s.setActiveProvider);
  const { t } = useTranslation();

  const handleDelete = async (provider: Provider) => {
    if (!window.confirm(t('settings.providers.deleteConfirm', { name: provider.name }))) return;
    await deleteProvider(provider.id);
  };

  const handleSelect = (providerId: ProviderId) => {
    setActiveProvider(providerId);
  };

  return (
    <div className="space-y-3">
      <div className="space-y-1">
        <h3 className="text-sm font-semibold text-[#171717] dark:text-[#f4f4f5] font-sans">{t('settings.providers.title')}</h3>
        <p className="text-[11px] text-[#666666] dark:text-[#71717a] font-sans">{t('settings.providers.subtitle')}</p>
      </div>

      <div className="space-y-2">
        {providers.map((provider) => {
          const signedIn = Boolean(provider.apiKey.trim()) || provider.models.length > 0;
          const isActive = provider.id === activeProviderId;
          return (
            <div
              key={provider.id}
              className="flex items-center justify-between gap-3 rounded-lg border border-[#ebebeb] dark:border-[#2c2c33] bg-white dark:bg-[#17171b] px-3 py-2.5"
            >
              <button
                type="button"
                onClick={() => handleSelect(provider.id)}
                className="flex items-center gap-2 min-w-0 text-left cursor-pointer"
                aria-label={provider.name}
              >
                <span className="text-xs font-medium text-[#171717] dark:text-[#f4f4f5] font-sans truncate">{provider.name}</span>
                {provider.isCustom && (
                  <span className="shrink-0 rounded border border-[#ebebeb] dark:border-[#2c2c33] bg-[#fafafa] dark:bg-[#111114] px-1.5 py-0.5 text-[10px] text-[#666666] dark:text-[#71717a] font-sans">
                    {t('settings.providers.custom')}
                  </span>
                )}
                <span
                  className={`h-2 w-2 shrink-0 rounded-full ${signedIn ? 'bg-[#22c55e]' : 'bg-[#ef4444]'}`}
                  aria-hidden="true"
                />
                <span className={`text-[11px] font-sans ${signedIn ? 'text-[#22c55e]' : 'text-[#666666] dark:text-[#71717a]'}`}>
                  {signedIn ? t('settings.providers.signedIn') : t('settings.providers.notSignedIn')}
                </span>
                {isActive && <span className="sr-only">active</span>}
              </button>
              <div className="flex items-center gap-2 shrink-0">
                <button
                  type="button"
                  onClick={() => onEdit(provider)}
                  className="rounded-md border border-[#ebebeb] dark:border-[#2c2c33] bg-white dark:bg-[#17171b] px-2.5 py-1 text-[11px] text-[#171717] dark:text-[#f4f4f5] font-sans cursor-pointer hover:bg-[#f4f4f5] dark:hover:bg-[#1e1e23]"
                >
                  {t('settings.providers.edit')}
                </button>
                {provider.isCustom && (
                  <button
                    type="button"
                    onClick={() => void handleDelete(provider)}
                    className="px-1.5 py-1 text-[11px] text-red-600 dark:text-red-400 font-sans cursor-pointer hover:underline"
                  >
                    {t('settings.providers.delete')}
                  </button>
                )}
              </div>
            </div>
          );
        })}
      </div>

      <button
        type="button"
        onClick={onAdd}
        className="flex w-full items-center justify-center gap-1.5 rounded-lg border border-dashed border-[#ebebeb] dark:border-[#2c2c33] px-3 py-2.5 text-xs text-[#171717] dark:text-[#f4f4f5] font-sans cursor-pointer hover:bg-[#fafafa] dark:hover:bg-[#17171b]"
      >
        <Plus className="h-3.5 w-3.5" />
        {t('settings.providers.add')}
      </button>
    </div>
  );
}
