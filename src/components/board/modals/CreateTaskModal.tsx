import React, { useState, useEffect, useCallback, useRef } from 'react';
import { createPortal } from 'react-dom';
import { useShallow } from 'zustand/react/shallow';
import { X } from 'lucide-react';
import { useBoardStore } from '../../../stores/boardStore';
import { useTranslation } from '../../../stores/languageStore';
import { compressImageFile } from '../../../lib/imageCompression';
import { PipelineStage, PipelineColumn } from '../../../types';
import { useUiZoom, getZoomOverlayStyle } from '../../../hooks/useUiZoom';
import ImageLightboxModal from './ImageLightboxModal';
import TaskAttachmentPicker, { type TaskAttachmentItem } from './TaskAttachmentPicker';

export interface CreateTaskModalProps {
  isOpen: boolean;
  onClose: () => void;
  stages?: PipelineStage[];
  /** @deprecated Use stages instead */
  columns?: PipelineColumn[];
  defaultStageId?: string;
  /** @deprecated Use defaultStageId instead */
  defaultColumnId?: string;
}

export default function CreateTaskModal({
  isOpen,
  onClose,
  stages,
  columns,
  defaultStageId,
  defaultColumnId,
}: CreateTaskModalProps) {
  const { addTask, activeBoardId } = useBoardStore(
    useShallow((s) => ({ addTask: s.addTask, activeBoardId: s.activeBoardId }))
  );
  const { t } = useTranslation();
  const { zoom } = useUiZoom();
  const effectiveStages = stages ?? columns;
  const initialStage = defaultStageId ?? defaultColumnId;

  const [newTitle, setNewTitle] = useState('');
  const [newDescription, setNewDescription] = useState('');
  const [newAttachments, setNewAttachments] = useState<TaskAttachmentItem[]>([]);
  const [selectedColumnId, setSelectedColumnId] = useState<string>('');
  const [previewLightbox, setPreviewLightbox] = useState<{ src: string; alt?: string } | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  const wasOpenRef = useRef(false);

  // Initialize and reset fields ONLY on open transition (never wipe user input if effectiveStages updates while open)
  useEffect(() => {
    if (isOpen && !wasOpenRef.current) {
      setNewTitle('');
      setNewDescription('');
      setNewAttachments([]);
      const intakeCol = effectiveStages?.find((c) => c.isIntake);
      const fallbackId =
        intakeCol?.id ||
        (effectiveStages?.some((c) => c.id === 'backlog') ? 'backlog' : (effectiveStages?.[0]?.id ?? 'backlog'));
      setSelectedColumnId(initialStage || fallbackId);
      setPreviewLightbox(null);
      setIsSubmitting(false);
    }
    wasOpenRef.current = isOpen;
  }, [isOpen, initialStage, effectiveStages]);

  useEffect(() => {
    if (!isOpen) return;
    const handleGlobalKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && !previewLightbox) {
        e.preventDefault();
        e.stopImmediatePropagation();
        onClose();
      }
    };
    window.addEventListener('keydown', handleGlobalKeyDown, { capture: true });
    return () => window.removeEventListener('keydown', handleGlobalKeyDown, { capture: true });
  }, [isOpen, onClose, previewLightbox]);

  const processAndAddImageFiles = useCallback(async (files: File[] | FileList) => {
    const list = Array.from(files).filter(
      (f) => f.type.startsWith('image/') || /\.(png|jpe?g|webp|gif|svg)$/i.test(f.name)
    );
    if (list.length === 0) return;

    try {
      const compressedList = await Promise.all(
        list.map(async (file, idx) => {
          try {
            const dataUrl = await compressImageFile(file, 1568, 0.85);
            return {
              id: `${Date.now()}-${Math.random().toString(36).substring(2, 7)}`,
              name: file.name || `image-${idx + 1}.png`,
              dataUrl,
            };
          } catch (err) {
            console.error('[CreateTaskModal] Failed to compress image:', file.name, err);
            return null;
          }
        })
      );

      const successful = compressedList.filter((item): item is TaskAttachmentItem => item !== null);
      if (successful.length > 0) {
        setNewAttachments((prev) => [...prev, ...successful]);
      }
    } catch (err) {
      console.error('[CreateTaskModal] Error adding images:', err);
    }
  }, []);

  const handleModalPaste = useCallback(
    (e: React.ClipboardEvent) => {
      const items = e.clipboardData?.items;
      if (!items) return;

      const pastedImageFiles: File[] = [];
      for (let i = 0; i < items.length; i++) {
        const item = items[i];
        if (item.type.startsWith('image/')) {
          const file = item.getAsFile();
          if (file) pastedImageFiles.push(file);
        }
      }

      if (pastedImageFiles.length > 0) {
        e.preventDefault();
        void processAndAddImageFiles(pastedImageFiles);
      }
    },
    [processAndAddImageFiles]
  );

  const handleModalKeyDown = useCallback(
    async (e: React.KeyboardEvent) => {
      const isV = e.key === 'v' || e.key === 'V' || e.key === 'м' || e.key === 'М' || e.code === 'KeyV';

      if ((e.ctrlKey || e.metaKey) && isV) {
        const target = e.target as HTMLElement | null;
        const isTextInput = target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA');

        if (!isTextInput && typeof navigator !== 'undefined' && navigator.clipboard?.read) {
          try {
            const clipboardItems = await navigator.clipboard.read();
            const files: File[] = [];
            for (const item of clipboardItems) {
              const imageType = item.types.find((tp) => tp.startsWith('image/'));
              if (imageType) {
                const blob = await item.getType(imageType);
                files.push(
                  new File([blob], `screenshot-${Date.now()}.${imageType.split('/')[1] || 'png'}`, {
                    type: imageType,
                  })
                );
              }
            }
            if (files.length > 0) {
              e.preventDefault();
              void processAndAddImageFiles(files);
            }
          } catch (err) {
            console.debug('[CreateTaskModal] Clipboard read fallback note:', err);
          }
        }
      }
    },
    [processAndAddImageFiles]
  );

  const handleRemoveAttachment = (id: string) => {
    setNewAttachments((prev) => prev.filter((a) => a.id !== id));
  };

  const handleCreateTask = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newTitle.trim() || isSubmitting) return;

    setIsSubmitting(true);
    try {
      const intakeCol = effectiveStages?.find((c) => c.isIntake);
      const fallbackId =
        intakeCol?.id ||
        (effectiveStages?.some((c) => c.id === 'backlog') ? 'backlog' : (effectiveStages?.[0]?.id ?? 'backlog'));
      const targetColumnId = selectedColumnId || fallbackId;

      const created = await addTask({
        title: newTitle.trim(),
        description: newDescription.trim() || undefined,
        stageId: targetColumnId,
        boardId: activeBoardId || 'development',
        attachments: newAttachments.map((a) => a.dataUrl),
      });
      if (created) {
        onClose();
      }
    } finally {
      setIsSubmitting(false);
    }
  };

  if (!isOpen) return null;

  const modalContent = (
    <>
      <div
        style={getZoomOverlayStyle(zoom)}
        className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 dark:bg-black/80 p-3 animate-in fade-in duration-100"
        onKeyDown={handleModalKeyDown}
        onClick={(e) => {
          if (e.target === e.currentTarget && !previewLightbox) onClose();
        }}
      >
        <div
          className="relative flex flex-col w-[620px] max-w-[95%] max-h-[90%] rounded-lg border border-[#ebebeb] dark:border-[#212126] bg-white dark:bg-[#111114] shadow-md p-5 text-[#171717] dark:text-[#f4f4f5] transition-colors overflow-hidden"
          onPaste={handleModalPaste}
        >
          <div className="flex items-center justify-between pb-3 border-b border-[#ebebeb] dark:border-[#212126] shrink-0">
            <h3 className="text-sm font-semibold text-[#171717] dark:text-[#f4f4f5]">
              {t('newTask.modalTitle')}
            </h3>
            <button
              type="button"
              onClick={onClose}
              className="p-1 rounded-md text-[#666666] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5] hover:bg-[#f4f4f5] dark:hover:bg-[#17171b] transition cursor-pointer"
              aria-label={t('common.close')}
            >
              <X className="h-4 w-4" />
            </button>
          </div>

          <form
            onSubmit={handleCreateTask}
            className="mt-4 flex flex-col flex-1 min-h-0 overflow-y-auto space-y-4 text-xs pr-1"
          >
            <div>
              <label className="block font-sans text-[#4d4d4d] dark:text-[#a1a1aa] font-medium mb-1">
                {t('newTask.titleLabel')}
              </label>
              <input
                type="text"
                required
                autoFocus
                value={newTitle}
                onChange={(e) => setNewTitle(e.target.value)}
                placeholder={t('newTask.titlePlaceholder')}
                className="w-full px-3 py-1.5 rounded-md bg-[#fafafa] dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] text-[#171717] dark:text-[#f4f4f5] placeholder:text-[#8f8f8f] dark:placeholder:text-[#71717a] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f4f4f5] focus:bg-white dark:focus:bg-[#111114] font-sans transition"
              />
            </div>

            <div>
              <label className="block font-sans text-[#4d4d4d] dark:text-[#a1a1aa] font-medium mb-1">
                {t('newTask.descLabel')}
              </label>
              <textarea
                rows={6}
                value={newDescription}
                onChange={(e) => setNewDescription(e.target.value)}
                placeholder={t('newTask.descPlaceholder')}
                className="w-full min-h-[130px] px-3 py-2 rounded-md bg-[#fafafa] dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] text-[#171717] dark:text-[#f4f4f5] placeholder:text-[#8f8f8f] dark:placeholder:text-[#71717a] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f4f4f5] focus:bg-white dark:focus:bg-[#111114] font-sans leading-relaxed resize-y transition"
              />
            </div>

            {effectiveStages && effectiveStages.length > 0 && (
              <div>
                <label className="block font-sans text-[#4d4d4d] dark:text-[#a1a1aa] font-medium mb-1">
                  {t('board.columnTitleLabel')}
                </label>
                <select
                  value={selectedColumnId}
                  onChange={(e) => setSelectedColumnId(e.target.value)}
                  className="w-full px-2.5 py-1.5 rounded-md bg-[#fafafa] dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] text-[#171717] dark:text-[#f4f4f5] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f4f4f5] focus:bg-white dark:focus:bg-[#111114] font-sans text-xs cursor-pointer transition"
                >
                  {effectiveStages.map((col) => (
                    <option key={col.id} value={col.id} className="dark:bg-[#17171b] dark:text-[#f4f4f5]">
                      {col.title}
                    </option>
                  ))}
                </select>
              </div>
            )}

            <TaskAttachmentPicker
              attachments={newAttachments}
              onFilesSelected={(files) => void processAndAddImageFiles(files)}
              onRemoveAttachment={handleRemoveAttachment}
              onPreview={setPreviewLightbox}
            />

            <div className="flex items-center justify-end space-x-2 pt-3 border-t border-[#ebebeb] dark:border-[#212126] shrink-0">
              <button
                type="button"
                onClick={onClose}
                disabled={isSubmitting}
                className="px-3 py-1.5 rounded-md bg-white dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] text-[#171717] dark:text-[#f4f4f5] hover:bg-[#f4f4f5] dark:hover:bg-[#232328] transition cursor-pointer font-sans disabled:opacity-50"
              >
                {t('common.cancel')}
              </button>
              <button
                type="submit"
                disabled={isSubmitting || !newTitle.trim()}
                className="px-3.5 py-1.5 rounded-md bg-[#171717] dark:bg-[#232328] hover:bg-[#262626] dark:hover:bg-[#2c2c33] text-white font-medium transition cursor-pointer font-sans shadow-xs border border-transparent dark:border-[#2c2c33] disabled:opacity-50"
              >
                {t('common.create')}
              </button>
            </div>
          </form>
        </div>
      </div>

      {previewLightbox && (
        <ImageLightboxModal
          src={previewLightbox.src}
          alt={previewLightbox.alt}
          onClose={() => setPreviewLightbox(null)}
        />
      )}
    </>
  );

  return typeof document !== 'undefined' ? createPortal(modalContent, document.body) : modalContent;
}
