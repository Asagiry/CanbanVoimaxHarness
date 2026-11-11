import React, { useRef } from 'react';
import { X, ImagePlus, Paperclip, ZoomIn } from 'lucide-react';
import { useTranslation } from '../../../stores/languageStore';

export interface TaskAttachmentItem {
  id: string;
  name: string;
  dataUrl: string;
}

interface TaskAttachmentPickerProps {
  attachments: TaskAttachmentItem[];
  onFilesSelected: (files: FileList) => void;
  onRemoveAttachment: (id: string) => void;
  onPreview: (item: { src: string; alt?: string }) => void;
}

export default function TaskAttachmentPicker({
  attachments,
  onFilesSelected,
  onRemoveAttachment,
  onPreview,
}: TaskAttachmentPickerProps) {
  const { t } = useTranslation();
  const fileInputRef = useRef<HTMLInputElement>(null);

  const handleFileInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.files && e.target.files.length > 0) {
      onFilesSelected(e.target.files);
    }
    e.target.value = '';
  };

  return (
    <div>
      <div className="flex items-center justify-between mb-1.5">
        <label className="block font-sans text-[#4d4d4d] dark:text-[#a1a1aa] font-medium">
          {t('newTask.attachmentsLabel')}
        </label>
        <button
          type="button"
          onClick={() => fileInputRef.current?.click()}
          className="inline-flex items-center gap-1.5 px-2 py-0.5 rounded text-[11px] font-sans text-[#4d4d4d] dark:text-[#a1a1aa] bg-[#fafafa] dark:bg-[#17171b] hover:bg-[#f4f4f5] dark:hover:bg-[#232328] border border-[#ebebeb] dark:border-[#2c2c33] transition cursor-pointer"
        >
          <ImagePlus className="h-3 w-3" />
          <span>{t('newTask.pasteHint')}</span>
        </button>
        <input
          ref={fileInputRef}
          type="file"
          accept="image/*"
          multiple
          className="hidden"
          onChange={handleFileInputChange}
        />
      </div>

      {attachments.length > 0 ? (
        <div className="flex flex-wrap gap-2.5 p-2 rounded-md border border-[#ebebeb] dark:border-[#2c2c33] bg-[#fafafa] dark:bg-[#17171b]/50 max-h-40 overflow-y-auto">
          {attachments.map((att, idx) => (
            <div
              key={att.id}
              className="group relative flex flex-col items-center w-24 rounded-md overflow-hidden border border-[#ebebeb] dark:border-[#2c2c33] bg-white dark:bg-[#111114] shadow-2xs hover:border-[#171717] dark:hover:border-[#f4f4f5] transition"
            >
              <button
                type="button"
                className="relative w-full h-16 cursor-pointer overflow-hidden bg-black/5 dark:bg-white/5"
                onClick={() => onPreview({ src: att.dataUrl, alt: att.name })}
                title={att.name}
                aria-label={att.name}
              >
                <img
                  src={att.dataUrl}
                  alt={att.name}
                  className="w-full h-full object-cover transition-transform group-hover:scale-105"
                />
                <div className="absolute inset-0 bg-black/0 group-hover:bg-black/25 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity">
                  <ZoomIn className="h-4 w-4 text-white drop-shadow" />
                </div>
              </button>
              <div className="w-full px-1.5 py-0.5 flex items-center justify-between text-[10px] font-mono text-[#8f8f8f] dark:text-[#71717a] border-t border-[#ebebeb] dark:border-[#2c2c33] bg-[#fafafa] dark:bg-[#17171b]">
                <span className="truncate max-w-[60px] tabular-nums" title={att.name}>
                  #{idx + 1}
                </span>
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    onRemoveAttachment(att.id);
                  }}
                  className="p-0.5 rounded text-[#8f8f8f] hover:text-[#ef4444] dark:hover:text-[#f87171] transition cursor-pointer"
                  aria-label={t('newTask.removeAttachment')}
                >
                  <X className="h-3 w-3" />
                </button>
              </div>
            </div>
          ))}
        </div>
      ) : (
        <button
          type="button"
          onClick={() => fileInputRef.current?.click()}
          className="w-full flex flex-col items-center justify-center p-3 rounded-md border border-dashed border-[#d4d4d8] dark:border-[#2c2c33] bg-[#fafafa]/50 dark:bg-[#17171b]/30 text-center cursor-pointer hover:bg-[#fafafa] dark:hover:bg-[#17171b] transition"
        >
          <Paperclip className="h-4 w-4 text-[#8f8f8f] dark:text-[#71717a] mb-1" />
          <span className="text-[11px] font-sans text-[#666666] dark:text-[#71717a]">
            {t('newTask.pasteHint')}
          </span>
        </button>
      )}
    </div>
  );
}
