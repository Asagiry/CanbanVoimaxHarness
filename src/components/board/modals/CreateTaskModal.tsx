import React, { useState, useEffect, useRef, useCallback } from 'react';
import { createPortal } from 'react-dom';
import { X, ImagePlus, Paperclip, ZoomIn } from 'lucide-react';
import { useBoardStore } from '../../../stores/boardStore';
import { useSettingsStore } from '../../../stores/settingsStore';
import { useTranslation } from '../../../stores/languageStore';
import { compressImageFile } from '../../../lib/imageCompression';
import { PipelineStage, PipelineColumn } from '../../../types';
import { useUiZoom, getZoomOverlayStyle } from '../../../hooks/useUiZoom';
import ImageLightboxModal from './ImageLightboxModal';

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
  const { addTask, activeBoardId } = useBoardStore();
  const providers = useSettingsStore((s) => s.providers);
  const settings = useSettingsStore((s) => s.settings);
  const { t } = useTranslation();
  const { zoom } = useUiZoom();
  const effectiveStages = stages ?? columns;
  const initialStage = defaultStageId ?? defaultColumnId;

  const [newTitle, setNewTitle] = useState('');
  const [newDescription, setNewDescription] = useState('');
  const [newAttachments, setNewAttachments] = useState<{ id: string; name: string; dataUrl: string }[]>([]);
  const [selectedModel, setSelectedModel] = useState<string>(settings.model || 'gemini-3.8-flash-high');
  const [selectedColumnId, setSelectedColumnId] = useState<string>('');
  const [previewLightbox, setPreviewLightbox] = useState<{ src: string; alt?: string } | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  const fileInputRef = useRef<HTMLInputElement>(null);

  // Initialize and reset fields on open
  useEffect(() => {
    if (isOpen) {
      setNewTitle('');
      setNewDescription('');
      setNewAttachments([]);
      const firstAvailable = providers.flatMap((p) => p.models.map((m) => m.id))[0] ?? '';
      setSelectedModel(settings.model || firstAvailable || 'gemini-3.8-flash-high');
      const intakeCol = effectiveStages?.find((c) => c.isIntake);
      const fallbackId =
        intakeCol?.id ||
        (effectiveStages?.some((c) => c.id === 'backlog') ? 'backlog' : (effectiveStages?.[0]?.id ?? 'backlog'));
      setSelectedColumnId(initialStage || fallbackId);
      setPreviewLightbox(null);
      setIsSubmitting(false);
    }
  }, [isOpen, initialStage, effectiveStages, settings.model, providers]);

  // Global Escape key listener (only when lightbox is not active)
  useEffect(() => {
    if (!isOpen) return;
    const handleGlobalKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && !previewLightbox) {
        onClose();
      }
    };
    window.addEventListener('keydown', handleGlobalKeyDown);
    return () => window.removeEventListener('keydown', handleGlobalKeyDown);
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

      const successful = compressedList.filter(
        (item): item is { id: string; name: string; dataUrl: string } => item !== null
      );
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
          if (file) {
            pastedImageFiles.push(file);
          }
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
      const isV =
        e.key === 'v' ||
        e.key === 'V' ||
        e.key === 'м' ||
        e.key === 'М' ||
        e.code === 'KeyV';

      if ((e.ctrlKey || e.metaKey) && isV) {
        const target = e.target as HTMLElement | null;
        const isTextInput = target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA');

        if (!isTextInput && typeof navigator !== 'undefined' && navigator.clipboard?.read) {
          try {
            const clipboardItems = await navigator.clipboard.read();
            const files: File[] = [];
            for (const item of clipboardItems) {
              const imageType = item.types.find((t) => t.startsWith('image/'));
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

  const handleFileInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.files && e.target.files.length > 0) {
      void processAndAddImageFiles(e.target.files);
    }
    e.target.value = '';
  };

  const handleRemoveAttachment = (id: string) => {
    setNewAttachments((prev) => prev.filter((a) => a.id !== id));
  };

  const handleCreateTask = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newTitle.trim() || isSubmitting) return;

    setIsSubmitting(true);
    try {
      const intakeCol = columns?.find((c) => c.isIntake);
      const fallbackId =
        intakeCol?.id ||
        (columns?.some((c) => c.id === 'backlog') ? 'backlog' : (columns?.[0]?.id ?? 'backlog'));
      const targetColumnId = selectedColumnId || fallbackId;

      const created = await addTask({
        title: newTitle.trim(),
        description: newDescription.trim() || undefined,
        stageId: targetColumnId,
        boardId: activeBoardId || 'development',
        model: selectedModel,
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

            {/* Target Column Selection (if columns provided) */}
            {columns && columns.length > 0 && (
              <div>
                <label className="block font-sans text-[#4d4d4d] dark:text-[#a1a1aa] font-medium mb-1">
                  {t('board.columnTitleLabel')}
                </label>
                <select
                  value={selectedColumnId}
                  onChange={(e) => setSelectedColumnId(e.target.value)}
                  className="w-full px-2.5 py-1.5 rounded-md bg-[#fafafa] dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] text-[#171717] dark:text-[#f4f4f5] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f4f4f5] focus:bg-white dark:focus:bg-[#111114] font-sans text-xs cursor-pointer transition"
                >
                  {columns.map((col) => (
                    <option key={col.id} value={col.id} className="dark:bg-[#17171b] dark:text-[#f4f4f5]">
                      {col.title}
                    </option>
                  ))}
                </select>
              </div>
            )}

            {/* Attachments Section */}
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

              {/* Thumbnail gallery */}
              {newAttachments.length > 0 ? (
                <div className="flex flex-wrap gap-2.5 p-2 rounded-md border border-[#ebebeb] dark:border-[#2c2c33] bg-[#fafafa] dark:bg-[#17171b]/50 max-h-40 overflow-y-auto">
                  {newAttachments.map((att, idx) => (
                    <div
                      key={att.id}
                      className="group relative flex flex-col items-center w-24 rounded-md overflow-hidden border border-[#ebebeb] dark:border-[#2c2c33] bg-white dark:bg-[#111114] shadow-2xs hover:border-[#171717] dark:hover:border-[#f4f4f5] transition"
                    >
                      <div
                        className="relative w-full h-16 cursor-pointer overflow-hidden bg-black/5 dark:bg-white/5"
                        onClick={() => setPreviewLightbox({ src: att.dataUrl, alt: att.name })}
                        title={att.name}
                      >
                        <img
                          src={att.dataUrl}
                          alt={att.name}
                          className="w-full h-full object-cover transition-transform group-hover:scale-105"
                        />
                        <div className="absolute inset-0 bg-black/0 group-hover:bg-black/25 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity">
                          <ZoomIn className="h-4 w-4 text-white drop-shadow" />
                        </div>
                      </div>
                      <div className="w-full px-1.5 py-0.5 flex items-center justify-between text-[10px] font-mono text-[#8f8f8f] dark:text-[#71717a] border-t border-[#ebebeb] dark:border-[#2c2c33] bg-[#fafafa] dark:bg-[#17171b]">
                        <span className="truncate max-w-[60px]" title={att.name}>
                          #{idx + 1}
                        </span>
                        <button
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation();
                            handleRemoveAttachment(att.id);
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
                <div
                  onClick={() => fileInputRef.current?.click()}
                  className="flex flex-col items-center justify-center p-3 rounded-md border border-dashed border-[#d4d4d8] dark:border-[#2c2c33] bg-[#fafafa]/50 dark:bg-[#17171b]/30 text-center cursor-pointer hover:bg-[#fafafa] dark:hover:bg-[#17171b] transition"
                >
                  <Paperclip className="h-4 w-4 text-[#8f8f8f] dark:text-[#71717a] mb-1" />
                  <span className="text-[11px] font-sans text-[#666666] dark:text-[#71717a]">
                    {t('newTask.pasteHint')}
                  </span>
                </div>
              )}
            </div>

            <div>
              <label className="block font-mono text-[#4d4d4d] dark:text-[#a1a1aa] font-medium mb-1">
                {t('newTask.modelLabel')}
              </label>
              <select
                value={selectedModel}
                onChange={(e) => setSelectedModel(e.target.value)}
                className="w-full px-2.5 py-1.5 rounded-md bg-[#fafafa] dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] text-[#171717] dark:text-[#f4f4f5] focus:outline-hidden focus:border-[#171717] dark:focus:border-[#f4f4f5] focus:bg-white dark:focus:bg-[#111114] font-mono text-xs cursor-pointer transition"
              >
                {Array.from(new Set([selectedModel, ...providers.flatMap((p) => p.models.map((m) => m.id))])).map((m) => {
                  const label = providers.flatMap((p) => p.models).find((model) => model.id === m)?.displayName ?? m;
                  return (
                    <option key={m} value={m} className="dark:bg-[#17171b] dark:text-[#f4f4f5]">
                      {label}
                    </option>
                  );
                })}
              </select>
            </div>

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

      {/* Image Preview Lightbox */}
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
