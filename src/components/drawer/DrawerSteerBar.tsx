import React, { useState, useRef, useEffect } from 'react';
import type { KanbanTask } from '../../types';
import ContextGauge from './ContextGauge';
import TaskTodoWidget from './TaskTodoWidget';
import { useDrawerStore } from '../../stores/drawerStore';
import { useBoardStore } from '../../stores/boardStore';
import { useCortexStore } from '../../stores/cortexStore';
import { useTranslation } from '../../stores/languageStore';
import { getRoleBadgeStyle } from '../../lib/agentPalette';
import AgentIcon from '../common/AgentIcon';
import { getSubagentOrderMap, formatSubagentRoleDisplay } from './SubagentsSidebar';
import { compressImageFile } from '../../lib/imageCompression';
import {
  ArrowUp,
  Plus,
  ChevronDown,
  Square,
  Play,
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
  const { t, language } = useTranslation();
  const chatFileInputRef = useRef<HTMLInputElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const activeSubagentId = useDrawerStore((s) => s.activeSubagentId);
  const activeSubagentRole = useDrawerStore((s) => s.activeSubagentRole);
  const storeSubagents = useDrawerStore((s) => s.subagents);
  const toolExecutions = useDrawerStore((s) => s.toolExecutions);
  const toolIdToSubagentId = useDrawerStore((s) => s.toolIdToSubagentId);

  const boardTasks = useBoardStore((s) => s.tasks);
  const subagentNumber = React.useMemo(() => {
    if (!activeSubagentId || !task?.id) return null;
    const map = getSubagentOrderMap(task.id, storeSubagents, toolExecutions, toolIdToSubagentId, boardTasks);
    return map.get(activeSubagentId) ?? (toolIdToSubagentId[activeSubagentId] ? map.get(toolIdToSubagentId[activeSubagentId]) : null) ?? 1;
  }, [activeSubagentId, task?.id, storeSubagents, toolExecutions, toolIdToSubagentId, boardTasks]);

  const isBoardSubtask = React.useMemo(() => {
    if (!activeSubagentId) return false;
    return boardTasks.some((t) => t.id === activeSubagentId);
  }, [activeSubagentId, boardTasks]);

  const boards = useBoardStore((s) => s.boards);
  const columns = useBoardStore((s) => s.columns);
  const taskBoard = boards.find((b) => b.id === task.boardId);
  const boardCols = taskBoard?.columns && taskBoard.columns.length > 0 ? taskBoard.columns : columns;
  const taskColumn = boardCols.find((c) => c.id === task.columnId) || columns.find((c) => c.id === task.columnId);

  const workingCols = boardCols.filter(
    (c) => !c.isIntake && c.id !== 'backlog' && !c.terminal && c.id !== 'ready' && c.id !== 'done'
  );
  const firstStagePersona = workingCols[0]?.persona;
  const defaultWorkingPersona = firstStagePersona || 'general';

  const isTerminalTask = Boolean(taskColumn?.terminal || task.columnId === 'ready' || task.columnId === 'done');
  const effectivePersonaCandidate = (!task.parentId && (isTerminalTask || !taskColumn?.persona))
    ? (firstStagePersona || task.persona)
    : (task.persona || taskColumn?.persona);
  const taskPersona = effectivePersonaCandidate || defaultWorkingPersona;
  const activePersona = activeSubagentRole || taskPersona;
  const effectiveGaugeTaskId = activeSubagentId || task.id;
  const effectiveGaugePersona = activeSubagentId ? (activeSubagentRole || 'general') : activePersona;

  const agentPersonas = useCortexStore((s) => s.agentPersonas);
  const matchedPersona = agentPersonas.find(
    (p) => p.role.toLowerCase() === activePersona.toLowerCase()
  );

  const parentMatchedPersona = agentPersonas.find(
    (p) => p.role.toLowerCase() === taskPersona.toLowerCase()
  );
  const parentLimit = parentMatchedPersona?.contextWindow || (taskPersona.toLowerCase() === 'cvh-dev' ? 200000 : 128000);
  const effectiveLimit = matchedPersona?.contextWindow || parentLimit;

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

      const successful = results.filter((item): item is { id: string; name: string; dataUrl: string } => item !== null);
      if (successful.length > 0) {
        setAttachedImages((prev) => [...prev, ...successful]);
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

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  };

  const handleSubmit = (e?: React.FormEvent) => {
    if (e) e.preventDefault();

    const currentImages = [...attachedImages];
    const textToSend = interactiveInput;

    setInteractiveInput('');
    setAttachedImages([]);
    if (textareaRef.current) {
      textareaRef.current.style.height = 'auto';
    }

    void onSendPrompt(textToSend, currentImages);
  };

  const hasContent = Boolean(interactiveInput.trim() || attachedImages.length > 0);

  return (
    <footer className="bg-[#fafafa] dark:bg-[#090a0f] px-3 pb-3 pt-1 shrink-0 shadow-xs z-10 transition-colors">
      <div className="max-w-4xl mx-auto">
        {/* Floating Modern Card */}
        <div
          onDragOver={handleChatDragOver}
          onDragLeave={handleChatDragLeave}
          onDrop={handleChatDrop}
          className={`rounded-xl bg-white dark:bg-[#0d1117] border border-[#ebebeb] dark:border-[#30363d] transition-colors shadow-xs p-3 ${
            isDragOverChat ? 'ring-2 ring-blue-500/30 bg-blue-50/40 dark:bg-blue-950/40' : ''
          }`}
        >


          {/* Attached Images Thumbnail Preview Bar */}
          {attachedImages.length > 0 && (
            <div className="flex items-center gap-2 mb-2.5 pb-1 overflow-x-auto select-none">
              {attachedImages.map((img) => (
                <div
                  key={img.id}
                  className="relative group shrink-0 w-16 h-16 rounded-xl overflow-hidden border border-[#ebebeb] dark:border-[#30363d] bg-[#f4f4f5] dark:bg-[#161b22]"
                >
                  <img
                    src={img.dataUrl}
                    alt={img.name}
                    className="w-full h-full object-cover"
                  />
                  <button
                    type="button"
                    onClick={() => setAttachedImages((prev) => prev.filter((i) => i.id !== img.id))}
                    className="absolute top-1 right-1 w-4 h-4 rounded-full bg-white/90 dark:bg-[#161b22]/90 hover:bg-white dark:hover:bg-[#21262d] text-[#171717] dark:text-[#f0f6fc] border border-[#ebebeb] dark:border-[#30363d] flex items-center justify-center text-xs leading-none transition cursor-pointer"
                    aria-label={t('drawer.removeImage')}
                  >
                    ×
                  </button>
                  <div className="absolute inset-x-0 bottom-0 bg-white/90 dark:bg-[#161b22]/90 px-1 py-0.5 text-[8.5px] font-mono text-[#666666] dark:text-[#8b949e] truncate text-center border-t border-[#ebebeb] dark:border-[#30363d]">
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
            placeholder={
              activeSubagentId
                ? t('drawer.steerSubagentPlaceholder')
                : t('drawer.steerTaskPlaceholder')
            }
            className="w-full bg-transparent text-sm text-[#171717] dark:text-[#f0f6fc] placeholder:text-[#8f8f8f] dark:placeholder:text-[#8b949e] focus:outline-none resize-none leading-relaxed min-h-[44px] max-h-44 overflow-y-auto font-sans"
          />

          {/* Bottom toolbar inside card */}
          <div className="flex items-center justify-between pt-2 mt-1 border-t border-[#ebebeb] dark:border-[#21262d] select-none">
            {/* Left Toolbar Items */}
            <div className="flex items-center space-x-2">
              {/* Round "+" Button */}
              <button
                type="button"
                onClick={() => chatFileInputRef.current?.click()}
                className="w-7 h-7 rounded-full bg-[#f4f4f5] dark:bg-[#161b22] hover:bg-[#171717] dark:hover:bg-[#f0f6fc] text-[#171717] dark:text-[#f0f6fc] hover:text-white dark:hover:text-[#090a0f] flex items-center justify-center transition cursor-pointer"
                aria-label={t('drawer.attachImage')}
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
              {(Boolean(activeSubagentId) || isToolAllowed('todo_write', task.columnId) || (task.todos && task.todos.length > 0)) && (
                <TaskTodoWidget task={task} align="left" />
              )}

              {/* Sleek Subagent Badge or Active Agent Badge */}
              {activeSubagentId ? (
                <div className="flex items-center space-x-1 shrink-0">
                  <span
                    style={getRoleBadgeStyle(activeSubagentRole)}
                    className="px-2 py-0.5 rounded text-[10px] font-mono font-bold tracking-wider border uppercase shrink-0 select-none"
                  >
                    {formatSubagentRoleDisplay(activeSubagentRole, subagentNumber || 1, isBoardSubtask, language)}
                  </span>
                </div>
              ) : (
                <span
                  style={getRoleBadgeStyle(activePersona, matchedPersona?.color)}
                  className="px-2 py-0.5 rounded text-[10px] font-mono font-bold tracking-wider border uppercase shrink-0 select-none flex items-center gap-1.5"
                >
                  <AgentIcon name={matchedPersona?.icon} className="h-3 w-3" />
                  <span>{activePersona.toUpperCase()}</span>
                </span>
              )}
            </div>

            {/* Right Toolbar Items */}
            <div className="flex items-center space-x-2">
              {/* Minimalist Model Selector */}
              <div className="relative">
                <button
                  type="button"
                  onClick={() => setIsModelMenuOpen(!isModelMenuOpen)}
                  className="px-2.5 py-1 rounded-full bg-[#f4f4f5] dark:bg-[#161b22] hover:bg-[#e4e4e7] dark:hover:bg-[#21262d] text-[#171717] dark:text-[#f0f6fc] font-mono text-xs border border-[#ebebeb] dark:border-[#30363d] transition cursor-pointer flex items-center gap-1.5 max-w-[200px]"
                  aria-label={t('drawer.taskModel', { model: effectiveModel })}
                >
                  <span className="truncate">{effectiveModel}</span>
                  <ChevronDown className="h-3 w-3 text-[#8f8f8f] dark:text-[#8b949e] shrink-0" />
                </button>
                {isModelMenuOpen && (
                  <div className="absolute bottom-full right-0 mb-2 w-56 rounded-lg bg-white dark:bg-[#161b22] border border-[#ebebeb] dark:border-[#30363d] shadow-md p-1 z-30 font-mono text-xs max-h-60 overflow-y-auto">
                    <div className="px-2 py-1 text-[#8f8f8f] dark:text-[#8b949e] text-[10px] uppercase">{t('drawer.selectModel')}</div>
                    {selectableModels.map((m) => (
                      <button
                        key={m}
                        type="button"
                        onClick={() => {
                          onSelectModel(m);
                          setIsModelMenuOpen(false);
                        }}
                        className={`w-full text-left px-2 py-1.5 rounded flex items-center justify-between cursor-pointer ${
                          effectiveModel === m ? 'bg-[#f4f4f5] dark:bg-[#21262d] text-[#171717] dark:text-[#f0f6fc]' : 'text-[#4d4d4d] dark:text-[#8b949e] hover:bg-[#f9f9f9] dark:hover:bg-[#21262d]'
                        }`}
                      >
                        <span className="truncate">{m}</span>
                        {effectiveModel === m && <span className="text-emerald-600 dark:text-emerald-400 shrink-0 ml-1">✓</span>}
                      </button>
                    ))}
                  </div>
                )}
              </div>

              {/* Action Button: Abort when running, Send when idle and has content, Play/Continue when idle and empty */}
              {isRunning ? (
                <button
                  type="button"
                  onClick={onAbortAgent}
                  className="w-8 h-8 rounded-full bg-[#fef2f2] dark:bg-[#3b1219] hover:bg-[#fee2e2] dark:hover:bg-[#4d1922] text-[#b91c1c] dark:text-[#ff7b72] border border-[#fecaca] dark:border-[#da3633] flex items-center justify-center shadow-xs transition-colors cursor-pointer"
                  aria-label={t('drawer.stopExecution')}
                >
                  <Square className="h-3.5 w-3.5 fill-current" />
                </button>
              ) : hasContent ? (
                <button
                  type="button"
                  onClick={() => handleSubmit()}
                  className="w-8 h-8 rounded-full bg-[#171717] dark:bg-[#f0f6fc] hover:bg-black dark:hover:bg-white text-white dark:text-[#090a0f] flex items-center justify-center shadow-xs transition-colors cursor-pointer"
                  aria-label={t('drawer.sendMessage')}
                  title={t('drawer.sendMessage')}
                >
                  <ArrowUp className="h-4 w-4 stroke-[2.5]" />
                </button>
              ) : (
                <button
                  type="button"
                  onClick={() => handleSubmit()}
                  className="h-8 px-2.5 rounded-full bg-[#171717] dark:bg-[#f0f6fc] hover:bg-black dark:hover:bg-white text-white dark:text-[#090a0f] flex items-center gap-1.5 shadow-xs transition-colors cursor-pointer text-xs font-medium"
                  aria-label={t('drawer.runLoop')}
                  title={t('drawer.runLoop')}
                >
                  <Play className="h-3.5 w-3.5 fill-current ml-0.5" />
                  <span className="hidden sm:inline font-sans">{t('drawer.continueLoop')}</span>
                </button>
              )}
            </div>
          </div>
        </div>

        {/* Elegant Telemetry Status Line (Кэш, TPS, Контекст) */}
        <div className="flex items-center justify-center">
          <ContextGauge taskId={effectiveGaugeTaskId} persona={effectiveGaugePersona} turns={turns} contextWindow={effectiveLimit} />
        </div>
      </div>
    </footer>
  );
}
