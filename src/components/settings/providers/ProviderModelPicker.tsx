import { useEffect, useMemo, useState } from 'react';
import { createPortal } from 'react-dom';
import { CheckSquare, Cpu, Search, Square, X } from 'lucide-react';
import { useTranslation } from '../../../stores/languageStore';
import { useUiZoom, getZoomOverlayStyle } from '../../../hooks/useUiZoom';
import type { ProviderModel } from '../../../stores/settings/providers';

interface ProviderModelPickerProps {
  open: boolean;
  candidates: string[];
  existing: ProviderModel[];
  onClose: () => void;
  onAdd: (selected: string[]) => void;
}

export default function ProviderModelPicker({ open, candidates, existing, onClose, onAdd }: ProviderModelPickerProps) {
  const { t } = useTranslation();
  const { zoom } = useUiZoom();
  const [searchFilter, setSearchFilter] = useState('');
  const [selected, setSelected] = useState<Set<string>>(new Set(candidates));

  useEffect(() => {
    if (open) {
      setSearchFilter('');
      setSelected(new Set(candidates));
    }
  }, [open, candidates]);

  useEffect(() => {
    if (!open) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        e.stopImmediatePropagation();
        onClose();
      }
    };
    window.addEventListener('keydown', handleKeyDown, { capture: true });
    return () => window.removeEventListener('keydown', handleKeyDown, { capture: true });
  }, [open, onClose]);

  const filtered = useMemo(() => {
    if (!searchFilter.trim()) return candidates;
    const q = searchFilter.toLowerCase();
    return candidates.filter((m) => m.toLowerCase().includes(q));
  }, [candidates, searchFilter]);

  if (!open) return null;

  const existingIds = new Set(existing.map((m) => m.id));
  const toggle = (modelName: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(modelName)) next.delete(modelName);
      else next.add(modelName);
      return next;
    });
  };

  const modal = (
    <div style={getZoomOverlayStyle(zoom)} className="fixed inset-0 z-[90] flex items-center justify-center bg-black/60 dark:bg-black/80 p-4">
      <div className="relative flex flex-col w-[520px] max-w-full max-h-[90%] overflow-y-auto rounded-lg border border-[#ebebeb] dark:border-[#212126] bg-white dark:bg-[#111114] shadow-md p-5 text-[#171717] dark:text-[#f4f4f5]">
        <div className="flex items-center justify-between pb-3 border-b border-[#ebebeb] dark:border-[#212126]">
          <div className="flex items-center space-x-2">
            <Cpu className="h-4 w-4" />
            <h3 className="text-sm font-semibold tracking-tight">{t('settings.chooseModelsTitle')}</h3>
          </div>
          <button type="button" onClick={onClose} aria-label={t('common.close')} className="p-1 rounded-md text-[#666666] hover:text-[#171717] cursor-pointer">
            <X className="h-4 w-4" />
          </button>
        </div>

        <div className="mt-4">
          <div className="relative flex items-center">
            <Search className="absolute left-2.5 h-3.5 w-3.5 text-[#8f8f8f] pointer-events-none" />
            <input
              type="text"
              value={searchFilter}
              onChange={(e) => setSearchFilter(e.target.value)}
              placeholder={t('settings.searchModels')}
              className="w-full rounded-md bg-[#fafafa] dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] pl-8 pr-2.5 py-1.5 text-xs font-sans focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f4f4f5]"
              autoFocus
            />
          </div>
        </div>

        <div className="mt-2.5 flex items-center justify-between text-xs text-[#666666] px-0.5 font-sans">
          <button
            type="button"
            onClick={() => setSelected(selected.size === filtered.length ? new Set() : new Set(filtered))}
            className="text-[#171717] dark:text-[#f4f4f5] hover:underline font-medium cursor-pointer"
          >
            {selected.size === filtered.length && filtered.length > 0 ? t('settings.deselectAll') : t('settings.selectAll')}
          </button>
          <span className="text-[11px] tabular-nums">{t('settings.selectedCount', { selected: selected.size, total: filtered.length })}</span>
        </div>

        <div className="mt-2 border border-[#ebebeb] dark:border-[#2c2c33] rounded-md bg-white dark:bg-[#17171b] max-h-64 overflow-y-auto divide-y divide-[#ebebeb] dark:divide-[#212126]">
          {filtered.length > 0 ? (
            filtered.map((modelName) => {
              const isChecked = selected.has(modelName);
              const isAdded = existingIds.has(modelName);
              return (
                <button
                  key={modelName}
                  type="button"
                  onClick={() => toggle(modelName)}
                  className={`w-full text-left flex items-center space-x-2.5 px-3 py-2 cursor-pointer select-none ${isChecked ? 'bg-[#f4f4f5] dark:bg-[#232328]' : 'hover:bg-[#fafafa] dark:hover:bg-[#232328]/50'}`}
                >
                  {isChecked ? <CheckSquare className="h-4 w-4 shrink-0" /> : <Square className="h-4 w-4 shrink-0 text-[#8f8f8f]" />}
                  <span className="font-mono text-xs truncate flex-1">{modelName}</span>
                  {isAdded && (
                    <span className="text-[10px] text-[#666666] bg-[#f4f4f5] dark:bg-[#111114] border border-[#ebebeb] dark:border-[#2c2c33] px-1.5 py-0.5 rounded-md font-sans">
                      {t('settings.alreadyAdded')}
                    </span>
                  )}
                </button>
              );
            })
          ) : (
            <div className="p-6 text-center text-xs text-[#8f8f8f] font-sans">{t('settings.noModelsFound')}</div>
          )}
        </div>

        <div className="flex items-center justify-end space-x-2 pt-4 mt-3 border-t border-[#ebebeb] dark:border-[#212126]">
          <button type="button" onClick={onClose} className="px-3 py-1.5 rounded-md bg-white dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] text-xs font-sans cursor-pointer">
            {t('common.cancel')}
          </button>
          <button
            type="button"
            onClick={() => onAdd(Array.from(selected))}
            disabled={selected.size === 0}
            className="px-3.5 py-1.5 rounded-md bg-[#171717] dark:bg-[#238636] text-white text-xs font-medium font-sans cursor-pointer disabled:opacity-50"
          >
            {t('settings.addSelected', { count: selected.size })}
          </button>
        </div>
      </div>
    </div>
  );

  return typeof document !== 'undefined' ? createPortal(modal, document.body) : modal;
}
