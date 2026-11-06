import { GripVertical, Trash2 } from 'lucide-react';
import type { ProviderModel } from '../../../stores/settings/providers';

interface ProviderModelRowProps {
  model: ProviderModel;
  index: number;
  onChange: (index: number, model: ProviderModel) => void;
  onDelete: (index: number) => void;
}

export default function ProviderModelRow({ model, index, onChange, onDelete }: ProviderModelRowProps) {
  return (
    <div className="flex items-center gap-2">
      <GripVertical className="h-4 w-4 shrink-0 text-[#8f8f8f] dark:text-[#71717a]" aria-hidden="true" />
      <input
        type="text"
        value={model.id}
        onChange={(e) => onChange(index, { ...model, id: e.target.value })}
        aria-label={`Model id ${index + 1}`}
        className="flex-1 min-w-0 rounded-md bg-white dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] px-2.5 py-1.5 font-mono text-xs text-[#171717] dark:text-[#f4f4f5] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f4f4f5]"
      />
      <input
        type="text"
        value={model.displayName ?? ''}
        onChange={(e) => onChange(index, { ...model, displayName: e.target.value || undefined })}
        aria-label={`Model display name ${index + 1}`}
        placeholder="Display name"
        className="flex-1 min-w-0 rounded-md bg-white dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] px-2.5 py-1.5 font-mono text-xs text-[#171717] dark:text-[#f4f4f5] placeholder:text-[#8f8f8f] dark:placeholder:text-[#71717a] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f4f4f5]"
      />
      <button
        type="button"
        onClick={() => onDelete(index)}
        aria-label={`Delete model ${model.id || index + 1}`}
        className="p-1.5 shrink-0 text-[#8f8f8f] hover:text-red-600 dark:hover:text-red-400 cursor-pointer rounded-md hover:bg-[#f4f4f5] dark:hover:bg-[#1e1e23]"
      >
        <Trash2 className="h-3.5 w-3.5" />
      </button>
    </div>
  );
}
