import React, { useState, useRef } from 'react';
import { useDrawerStore } from '../../../stores/drawerStore';
import { compressImageFile } from '../../../lib/imageCompression';
import { AttachedImage } from './types';

export interface UseSteerImagesOptions {
  draftKey?: string;
  interactiveInput: string;
  textareaRef: React.RefObject<HTMLTextAreaElement | null>;
}

export function useSteerImages({
  draftKey,
  interactiveInput,
  textareaRef,
}: UseSteerImagesOptions) {
  const [attachedImages, setAttachedImages] = useState<AttachedImage[]>([]);
  const [isDragOverChat, setIsDragOverChat] = useState(false);
  const chatFileInputRef = useRef<HTMLInputElement>(null);

  const handleRemoveImage = (imageId: string) => {
    setAttachedImages((prev) => {
      const next = prev.filter((i) => i.id !== imageId);
      if (draftKey) {
        const currentText = textareaRef.current ? textareaRef.current.value : interactiveInput;
        useDrawerStore.getState().setTaskDraft(draftKey, {
          text: currentText,
          images: next,
        });
      }
      return next;
    });
  };

  const handleAddChatImages = async (files: FileList | File[]) => {
    const validFiles = Array.from(files).filter(
      (f) => f.type.startsWith('image/') || /\.(png|jpe?g|webp|gif|svg)$/i.test(f.name)
    );
    if (validFiles.length === 0) return;

    try {
      const results = await Promise.all(
        validFiles.map(async (file) => {
          try {
            const dataUrl = await compressImageFile(file, 1568, 0.85);
            return {
              id: `${Date.now()}-${Math.random().toString(36).substring(2, 7)}`,
              name: file.name,
              dataUrl,
            };
          } catch (err) {
            console.error('[DrawerSteerBar] Failed to process image:', file.name, err);
            return null;
          }
        })
      );

      const successful = results.filter((item): item is AttachedImage => item !== null);
      if (successful.length > 0) {
        setAttachedImages((prev) => {
          const next = [...prev, ...successful];
          if (draftKey) {
            const currentText = textareaRef.current ? textareaRef.current.value : interactiveInput;
            useDrawerStore.getState().setTaskDraft(draftKey, {
              text: currentText,
              images: next,
            });
          }
          return next;
        });
      }
    } catch (err) {
      console.error('[DrawerSteerBar] Error handling images:', err);
    }
  };

  const handleChatFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.files) {
      handleAddChatImages(e.target.files);
    }
    e.target.value = '';
  };

  const handleChatDragOver = (e: React.DragEvent) => {
    if (e.dataTransfer.types.includes('Files')) {
      e.preventDefault();
      e.stopPropagation();
      setIsDragOverChat(true);
    }
  };

  const handleChatDragLeave = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragOverChat(false);
  };

  const handleChatDrop = (e: React.DragEvent) => {
    if (e.dataTransfer.files && e.dataTransfer.files.length > 0) {
      e.preventDefault();
      e.stopPropagation();
      setIsDragOverChat(false);
      handleAddChatImages(e.dataTransfer.files);
    }
  };

  const handleChatPaste = (e: React.ClipboardEvent<HTMLTextAreaElement>) => {
    const items = e.clipboardData?.items;
    if (!items) return;

    const pastedFiles: File[] = [];
    for (let i = 0; i < items.length; i++) {
      const item = items[i];
      if (item.type.startsWith('image/')) {
        const file = item.getAsFile();
        if (file) {
          pastedFiles.push(file);
        }
      }
    }

    if (pastedFiles.length > 0) {
      e.preventDefault();
      handleAddChatImages(pastedFiles);
    }
  };

  return {
    attachedImages,
    setAttachedImages,
    isDragOverChat,
    chatFileInputRef,
    handleRemoveImage,
    handleChatFileSelect,
    handleChatDragOver,
    handleChatDragLeave,
    handleChatDrop,
    handleChatPaste,
  };
}
