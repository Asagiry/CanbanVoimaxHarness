import { useTranslation } from '../../../stores/languageStore';
import { AttachedImage } from './types';

export interface SteerImageBarProps {
  attachedImages: AttachedImage[];
  onRemoveImage: (imageId: string) => void;
}

export function SteerImageBar({ attachedImages, onRemoveImage }: SteerImageBarProps) {
  const { t } = useTranslation();

  if (attachedImages.length === 0) return null;

  return (
    <div className="flex items-center gap-2 mb-2.5 pb-1 overflow-x-auto select-none">
      {attachedImages.map((img) => (
        <div
          key={img.id}
          className="relative group shrink-0 w-16 h-16 rounded-xl overflow-hidden border border-[#ebebeb] dark:border-[#2c2c33] bg-[#f4f4f5] dark:bg-[#17171b]"
        >
          <img
            src={img.dataUrl}
            alt={img.name}
            className="w-full h-full object-cover"
          />
          <button
            type="button"
            onClick={() => onRemoveImage(img.id)}
            className="absolute top-1 right-1 w-4 h-4 rounded-full bg-white/90 dark:bg-[#17171b]/90 hover:bg-white dark:hover:bg-[#232328] text-[#171717] dark:text-[#f4f4f5] border border-[#ebebeb] dark:border-[#2c2c33] flex items-center justify-center text-xs leading-none transition cursor-pointer"
            aria-label={t('drawer.removeImage')}
          >
            ×
          </button>
          <div className="absolute inset-x-0 bottom-0 bg-white/90 dark:bg-[#17171b]/90 px-1 py-0.5 text-[8.5px] font-mono text-[#666666] dark:text-[#71717a] truncate text-center border-t border-[#ebebeb] dark:border-[#2c2c33]">
            {img.name}
          </div>
        </div>
      ))}
    </div>
  );
}
