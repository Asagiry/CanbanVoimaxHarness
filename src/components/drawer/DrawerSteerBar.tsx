import React, { useState, useRef, useEffect } from 'react';
import type { KanbanTask } from '../../types';
import ContextGauge from './ContextGauge';
import TaskTodoWidget from './TaskTodoWidget';
import { useDrawerStore } from '../../stores/drawerStore';
import {
  ArrowUp,
  Plus,
  ChevronDown,
  Square,
  Minimize2,
} from 'lucide-react';

export interface DrawerSteerBarProps {
  task: KanbanTask;
  isRunning: boolean;
  turns: number;
  onSendPrompt: (promptText: string, attachedImages: Array<{ id: string; name: string; dataUrl: string }>) => Promise<void> | void;
  onAbortAgent: () => Promise<void> | void;
  isToolAllowed: (toolName: string, columnId?: string) => boolean;
  effectiveModel: string;
  selectableModels: string[];
  onSelectModel: (model: string) => void;
}

export default function DrawerSteerBar({
  task,
  isRunning,
  turns,
  onSendPrompt,
  onAbortAgent,
  isToolAllowed,
  effectiveModel,
  selectableModels,
  onSelectModel,
}: DrawerSteerBarProps) {
  const [interactiveInput, setInteractiveInput] = useState('');
  const [attachedImages, setAttachedImages] = useState<Array<{ id: string; name: string; dataUrl: string }>>([]);
  const [isDragOverChat, setIsDragOverChat] = useState(false);
  const [isModelMenuOpen, setIsModelMenuOpen] = useState(false);
  const [isCompacting, setIsCompacting] = useState(false);
  const chatFileInputRef = useRef<HTMLInputElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const compactTaskContext = useDrawerStore((s) => s.compactTaskContext);

  const handleCompact = async () => {
    if (isCompacting || !task?.id) return;
    setIsCompacting(true);
    try {
      await compactTaskContext(task.id);
    } catch (err) {
      console.warn('Failed to compact context:', err);
    } finally {
      setIsCompacting(false);
    }
  };

  // Reset input and attachments on task switch
  useEffect(() => {
    setInteractiveInput('');
    setAttachedImages([]);
    if (textareaRef.current) {
      textareaRef.current.style.height = 'auto';
    }
  }, [task?.id]);

  const adjustTextareaHeight = (el: HTMLTextAreaElement) => {
    el.style.height = 'auto';
    const newHeight = Math.min(Math.max(el.scrollHeight, 40), 180);
    el.style.height = `${newHeight}px`;
  };

  const handleInputChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setInteractiveInput(e.target.value);
    adjustTextareaHeight(e.target);
  };

  const handleAddChatImages = (files: FileList | File[]) => {
    const validFiles = Array.from(files).filter(
      (f) => f.type.startsWith('image/') || /\.(png|jpe?g|webp|gif)$/i.test(f.name)
    );
    if (validFiles.length === 0) return;

    validFiles.forEach((file) => {
      const reader = new FileReader();
      reader.onload = () => {
        const dataUrl = reader.result as string;
        setAttachedImages((prev) => [
          ...prev,
          {
            id: `${Date.now()}-${Math.random().toString(36).substring(2, 7)}`,
            name: file.name,
            dataUrl,
          },
        ]);
      };
      reader.readAsDataURL(file);
    });
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

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  };

  const handleSubmit = (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    const trimmed = interactiveInput.trim();
    if (!trimmed && attachedImages.length === 0) return;

    const currentImages = [...attachedImages];
    const textToSend = interactiveInput;

    setInteractiveInput('');
    setAttachedImages([]);
    if (textareaRef.current) {
      textareaRef.current.style.height = 'auto';
    }

    void onSendPrompt(textToSend, currentImages);
  };

  return (
    <footer className="border-t border-[#ebebeb] bg-[#fafafa]/80 backdrop-blur-xs px-3 pb-3 pt-1 shrink-0 shadow-xs z-10">
      <div className="max-w-4xl mx-auto">
        {/* Floating Modern Light Card */}
        <div
          onDragOver={handleChatDragOver}
          onDragLeave={handleChatDragLeave}
          onDrop={handleChatDrop}
          className={`rounded-2xl bg-white border border-[#ebebeb] transition-all shadow-xs p-3 ${
            isDragOverChat ? 'ring-2 ring-blue-500/30 bg-blue-50/40' : ''
          }`}
        >
          {/* Attached Images Thumbnail Preview Bar */}
          {attachedImages.length > 0 && (
            <div className="flex items-center gap-2 mb-2.5 pb-1 overflow-x-auto select-none">
              {attachedImages.map((img) => (
                <div
                  key={img.id}
                  className="relative group shrink-0 w-16 h-16 rounded-xl overflow-hidden border border-[#ebebeb] bg-[#f4f4f5]"
                >
                  <img
                    src={img.dataUrl}
                    alt={img.name}
                    className="w-full h-full object-cover"
                  />
                  <button
                    type="button"
                    onClick={() => setAttachedImages((prev) => prev.filter((i) => i.id !== img.id))}
                    className="absolute top-1 right-1 w-4 h-4 rounded-full bg-white/90 hover:bg-white text-[#171717] border border-[#ebebeb] flex items-center justify-center text-xs leading-none transition cursor-pointer"
                    title="Удалить"
                  >
                    ×
                  </button>
                  <div className="absolute inset-x-0 bottom-0 bg-white/90 px-1 py-0.5 text-[8.5px] font-mono text-[#666666] truncate text-center border-t border-[#ebebeb]">
                    {img.name}
                  </div>
                </div>
              ))}
            </div>
          )}

          {/* Multiline auto-resizing textarea */}
          <textarea
            ref={textareaRef}
            rows={1}
            value={interactiveInput}
            onChange={handleInputChange}
            onKeyDown={handleKeyDown}
            onPaste={handleChatPaste}
            placeholder="Напишите сообщение, задачу или прикрепите изображение..."
            className="w-full bg-transparent text-sm text-[#171717] placeholder:text-[#8f8f8f] focus:outline-none resize-none leading-relaxed min-h-[44px] max-h-44 overflow-y-auto font-sans"
          />

          {/* Bottom toolbar inside card */}
          <div className="flex items-center justify-between pt-2 mt-1 border-t border-[#ebebeb] select-none">
            {/* Left Toolbar Items */}
            <div className="flex items-center space-x-2">
              {/* Round "+" Button */}
              <button
                type="button"
                onClick={() => chatFileInputRef.current?.click()}
                className="w-7 h-7 rounded-full bg-[#f4f4f5] hover:bg-[#171717] text-[#171717] hover:text-white flex items-center justify-center transition cursor-pointer"
                title="Прикрепить изображение (PNG, JPG, WebP) или файл"
              >
                <Plus className="h-4 w-4" />
              </button>

              {/* Hidden File Input for "+" Button */}
              <input
                ref={chatFileInputRef}
                type="file"
                accept="image/png,image/jpeg,image/webp,image/gif"
                multiple
                className="hidden"
                onChange={handleChatFileSelect}
              />

              {/* Compact TaskTodoWidget pill & overlay popover */}
              {(isToolAllowed('todo_write', task.columnId) || (task.todos && task.todos.length > 0)) && (
                <TaskTodoWidget task={task} align="left" />
              )}
            </div>

            {/* Right Toolbar Items */}
            <div className="flex items-center space-x-2">
              {/* Minimalist Model Selector */}
              <div className="relative">
                <button
                  type="button"
                  onClick={() => setIsModelMenuOpen(!isModelMenuOpen)}
                  className="px-2.5 py-1 rounded-full bg-[#f4f4f5] hover:bg-[#e4e4e7] text-[#171717] font-mono text-xs border border-[#ebebeb] transition cursor-pointer flex items-center gap-1.5 max-w-[200px]"
                  title={`Модель задачи: ${effectiveModel}`}
                >
                  <span className="truncate">{effectiveModel}</span>
                  <ChevronDown className="h-3 w-3 text-[#8f8f8f] shrink-0" />
                </button>
                {isModelMenuOpen && (
                  <div className="absolute bottom-full right-0 mb-2 w-56 rounded-lg bg-white border border-[#ebebeb] shadow-md p-1 z-30 font-mono text-xs max-h-60 overflow-y-auto">
                    <div className="px-2 py-1 text-[#8f8f8f] text-[10px] uppercase">Выбор модели</div>
                    {selectableModels.map((m) => (
                      <button
                        key={m}
                        type="button"
                        onClick={() => {
                          onSelectModel(m);
                          setIsModelMenuOpen(false);
                        }}
                        className={`w-full text-left px-2 py-1.5 rounded flex items-center justify-between cursor-pointer ${
                          effectiveModel === m ? 'bg-[#f4f4f5] text-[#171717]' : 'text-[#4d4d4d] hover:bg-[#f9f9f9]'
                        }`}
                      >
                        <span className="truncate">{m}</span>
                        {effectiveModel === m && <span className="text-emerald-600 shrink-0 ml-1">✓</span>}
                      </button>
                    ))}
                  </div>
                )}
              </div>

              {/* Round Action Button: Abort when running, Send when idle */}
              {isRunning ? (
                <button
                  type="button"
                  onClick={onAbortAgent}
                  className="w-8 h-8 rounded-full bg-[#fef2f2] hover:bg-[#fee2e2] text-[#b91c1c] border border-[#fecaca] flex items-center justify-center shadow-xs transition cursor-pointer active:scale-95"
                  title="Остановить выполнение (abort_agent)"
                >
                  <Square className="h-3.5 w-3.5 fill-current" />
                </button>
              ) : (
                <button
                  type="button"
                  onClick={() => handleSubmit()}
                  disabled={!interactiveInput.trim() && attachedImages.length === 0}
                  className="w-8 h-8 rounded-full bg-[#171717] hover:bg-black disabled:opacity-40 disabled:hover:bg-[#171717] disabled:cursor-not-allowed text-white flex items-center justify-center shadow-xs transition cursor-pointer active:scale-95"
                  title="Отправить сообщение агенту (Enter)"
                >
                  <ArrowUp className="h-4 w-4 stroke-[2.5]" />
                </button>
              )}
            </div>
          </div>
        </div>

        {/* Elegant Telemetry Status Line (Кэш, TPS, Контекст) & Compactor Trigger */}
        <div className="flex items-center justify-center gap-3">
          <ContextGauge taskId={task.id} turns={turns} />
          <div className="h-3 w-px bg-[#ebebeb]" />
          <button
            type="button"
            onClick={handleCompact}
            disabled={isCompacting}
            className="flex items-center space-x-1.5 px-2 py-0.5 rounded-md bg-white border border-[#ebebeb] hover:bg-[#f4f4f5] hover:border-[#d4d4d8] text-[#0d0d0d] hover:text-[#000000] text-[11px] font-sans font-medium transition cursor-pointer shadow-2xs disabled:opacity-50 active:scale-[0.98]"
            title="Сжать контекст (DSH-Style State-Preserving Compaction)"
          >
            <Minimize2 className={`h-3 w-3 text-[#4d4d4d] ${isCompacting ? 'animate-spin' : ''}`} />
            <span>{isCompacting ? 'Сжатие...' : 'Сжать контекст'}</span>
          </button>
        </div>
      </div>
    </footer>
  );
}
