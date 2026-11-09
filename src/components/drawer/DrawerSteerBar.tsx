import React, { useState, useRef, useEffect } from 'react';
import type { KanbanTask, ReasoningEffort } from '../../types';
import ContextGauge from './ContextGauge';
import EffortSelector from './EffortSelector';
import TaskTodoWidget from './TaskTodoWidget';
import { useDrawerStore } from '../../stores/drawerStore';
import { useBoardStore } from '../../stores/boardStore';
import { useCortexStore } from '../../stores/cortexStore';
import { useSettingsStore } from '../../stores/settingsStore';
import { useTranslation } from '../../stores/languageStore';
import { getRoleBadgeStyle } from '../../lib/agentPalette';
import AgentIcon from '../common/AgentIcon';
import { getSubagentOrderMap, formatSubagentRoleDisplay } from './subagentHelpers';
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
  onSendPrompt: (promptText: string, attachedImages: Array<{ id: string; name: string; dataUrl: string }>) => Promise<boolean> | boolean | void;
  onAbortAgent: () => Promise<void> | void;
  isToolAllowed: (toolName: string, stageId?: string) => boolean;
  selectableModels: string[];
  onSelectPair: (providerId: string, model: string) => void;
  onSelectEffort: (effort: ReasoningEffort) => void;
}

export default function DrawerSteerBar({
  task,
  isRunning,
  turns,
  onSendPrompt,
  onAbortAgent,
  isToolAllowed,
  selectableModels,
  onSelectPair,
  onSelectEffort,
}: DrawerSteerBarProps) {
  const [interactiveInput, setInteractiveInput] = useState('');
  const [attachedImages, setAttachedImages] = useState<Array<{ id: string; name: string; dataUrl: string }>>([]);
  const [isDragOverChat, setIsDragOverChat] = useState(false);
  const [isModelMenuOpen, setIsModelMenuOpen] = useState(false);
  const { t, language } = useTranslation();
  const chatFileInputRef = useRef<HTMLInputElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const activeSubagentId = useDrawerStore((s) => s.activeSubagentId);
  const draftKey = activeSubagentId
    ? (activeSubagentId.includes(':sub:') ? activeSubagentId : `${task?.id}:sub:${activeSubagentId}`)
    : task?.id;
  const activeSubagentRole = useDrawerStore((s) => s.activeSubagentRole);
  const storeSubagents = useDrawerStore((s) => s.subagents);
  const subagentConfigs = useDrawerStore((s) => s.subagentConfigs);
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

  const boards = useBoardStore((s) => s.boards) ?? [];
  const columns = useBoardStore((s) => s.columns) ?? [];
  const taskBoard = boards.find((b) => b.id === task.boardId);
  const boardCols = taskBoard?.columns && taskBoard.columns.length > 0 ? taskBoard.columns : columns;
  const taskColumn = boardCols.find((c) => c.id === task.stageId) || columns.find((c) => c.id === task.stageId);

  const workingCols = boardCols.filter(
    (c) => !c.isIntake && c.id !== 'backlog' && !c.terminal && c.id !== 'ready' && c.id !== 'done'
  );
  const firstStagePersona = workingCols[0]?.persona;
  const defaultWorkingPersona = firstStagePersona || 'general';

  const isTerminalTask = Boolean(taskColumn?.terminal || task.stageId === 'ready' || task.stageId === 'done');
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

  const effectiveLimit = matchedPersona?.contextWindow || parentMatchedPersona?.contextWindow || 128000;

  // --- Provider / Model label (screenshot format: "OpenCode Go / Crouton Alpha Free Max") ---
  // Honest display = what the backend will actually dispatch:
  // explicit pair (task.providerId + task.model) > per-persona setting >
  // hardcoded fallback. The global flat `settings.model` is display-only
  // legacy and would lie (it still says beam-free while the planner
  // dispatches crouton), so it is NOT in this chain — same rule the
  // backend resolver uses (pair > agent setting > col > project > crouton).
  const providers = useSettingsStore((s) => s.providers) ?? [];
  const defaultModel = useSettingsStore.getState().settings?.model || (useSettingsStore.getState() as any).model || 'default';
  const activeBoardSubtask = isBoardSubtask ? boardTasks.find((t) => t.id === activeSubagentId) : undefined;
  const resolvedSubId = activeSubagentId ? (toolIdToSubagentId[activeSubagentId] || activeSubagentId) : null;
  const subConfig = activeSubagentId
    ? activeBoardSubtask
      ? {
          model: activeBoardSubtask.model,
          providerId: activeBoardSubtask.providerId ?? undefined,
          reasoningEffort: activeBoardSubtask.reasoningEffort as ReasoningEffort | undefined,
        }
      : (subagentConfigs?.[activeSubagentId] || (resolvedSubId ? subagentConfigs?.[resolvedSubId] : undefined))
    : undefined;

  const modelId = activeSubagentId
    ? (subConfig?.model || matchedPersona?.model || task.model || defaultModel)
    : (task.model || matchedPersona?.model || defaultModel);

  const pairProvider = activeSubagentId
    ? ((subConfig?.providerId ? providers.find((p) => p.id === subConfig.providerId) : undefined) ||
        (matchedPersona?.providerId ? providers.find((p) => p.id === matchedPersona.providerId) : undefined) ||
        (task.providerId ? providers.find((p) => p.id === task.providerId) : undefined) ||
        providers.find((p) => p.models.some((m) => m.id === modelId)) ||
        null)
    : ((task.providerId ? providers.find((p) => p.id === task.providerId) : undefined) ||
        (matchedPersona?.providerId ? providers.find((p) => p.id === matchedPersona.providerId) : undefined) ||
        providers.find((p) => p.models.some((m) => m.id === modelId)) ||
        null);

  const modelDisplay =
    pairProvider?.models.find((m) => m.id === modelId)?.displayName ||
    modelId;
  const providerModelLabel = pairProvider
    ? `${pairProvider.name} / ${modelDisplay}`
    : modelDisplay;
  const personaEffort = matchedPersona?.reasoningEffort || null;

  const groupedModels = React.useMemo(() => {
    const flat = providers.flatMap((p) => p.models.map((m) => ({
      providerName: p.name,
      providerId: p.id,
      id: m.id,
      displayName: m.displayName || m.id,
      label: `${p.name} / ${m.displayName || m.id}`,
    })));
    if (flat.length === 0 && (selectableModels?.length ?? 0) > 0) {
      return (selectableModels ?? []).map((id) => ({ providerName: '', providerId: '', id, displayName: id, label: id }));
    }
    return flat;
  }, [providers, selectableModels]);

  // Restore draft when switching tasks or subagents
  useEffect(() => {
    if (!draftKey) return;
    const draft = useDrawerStore.getState().getTaskDraft(draftKey);
    setInteractiveInput(draft?.text || '');
    setAttachedImages(draft?.images || []);
    if (textareaRef.current) {
      textareaRef.current.style.height = 'auto';
      requestAnimationFrame(() => {
        if (textareaRef.current) adjustTextareaHeight(textareaRef.current);
      });
    }
  }, [draftKey]);

  const adjustTextareaHeight = (el: HTMLTextAreaElement) => {
    el.style.height = 'auto';
    const newHeight = Math.min(Math.max(el.scrollHeight, 40), 180);
    el.style.height = `${newHeight}px`;
  };

  const handleInputChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const newVal = e.target.value;
    setInteractiveInput(newVal);
    adjustTextareaHeight(e.target);
    if (draftKey) {
      useDrawerStore.getState().setTaskDraft(draftKey, {
        text: newVal,
        images: attachedImages,
      });
    }
  };

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

      const successful = results.filter((item): item is { id: string; name: string; dataUrl: string } => item !== null);
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

    if (textareaRef.current) {
      textareaRef.current.style.height = 'auto';
    }

    const result = onSendPrompt(textToSend, currentImages);
    const clear = (ok: boolean) => {
      if (!ok) return;
      if (draftKey) {
        useDrawerStore.getState().clearTaskDraft(draftKey);
      }
      setInteractiveInput('');
      setAttachedImages([]);
    };
    if (result && typeof (result as Promise<boolean>).then === 'function') {
      void (result as Promise<boolean>).then(clear);
    } else if (result !== false) {
      clear(true);
    }
  };

  const hasContent = Boolean(interactiveInput.trim() || attachedImages.length > 0);

  return (
    <footer className="bg-[#fafafa] dark:bg-[#0b0b0e] px-3 pb-3 pt-1 shrink-0 shadow-xs z-10 transition-colors">
      <div className="max-w-4xl mx-auto">
        {/* Floating Modern Card */}
        <div
          onDragOver={handleChatDragOver}
          onDragLeave={handleChatDragLeave}
          onDrop={handleChatDrop}
          className={`rounded-xl bg-white dark:bg-[#111114] border border-[#ebebeb] dark:border-[#2c2c33] transition-colors shadow-xs p-3 ${
            isDragOverChat ? 'ring-2 ring-blue-500/30 bg-blue-50/40 dark:bg-blue-950/40' : ''
          }`}
        >


          {/* Attached Images Thumbnail Preview Bar */}
          {attachedImages.length > 0 && (
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
                    onClick={() => handleRemoveImage(img.id)}
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
            className="w-full bg-transparent text-sm text-[#171717] dark:text-[#f4f4f5] placeholder:text-[#8f8f8f] dark:placeholder:text-[#71717a] focus:outline-none resize-none leading-relaxed min-h-[44px] max-h-44 overflow-y-auto font-sans"
          />

          {/* Bottom toolbar inside card */}
          <div className="flex items-center justify-between pt-2 mt-1 border-t border-[#ebebeb] dark:border-[#212126] select-none">
            {/* Left Toolbar Items */}
            <div className="flex items-center space-x-2">
              {/* Round "+" Button */}
              <button
                type="button"
                onClick={() => chatFileInputRef.current?.click()}
                className="w-7 h-7 rounded-full bg-[#f4f4f5] dark:bg-[#17171b] hover:bg-[#171717] dark:hover:bg-[#f4f4f5] text-[#171717] dark:text-[#f4f4f5] hover:text-white dark:hover:text-[#0b0b0e] flex items-center justify-center transition cursor-pointer"
                aria-label={t('drawer.attachImage')}
              >
                <Plus className="h-4 w-4 block shrink-0" />
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
              {(Boolean(activeSubagentId) || isToolAllowed('todo_write', task.stageId) || (task.todos && task.todos.length > 0)) && (
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
              {/* Provider / Model selector (screenshot format: "OpenCode Go / Crouton Alpha Free Max") */}
              <div className="relative flex items-center gap-2">
                <button
                  type="button"
                  onClick={() => setIsModelMenuOpen(!isModelMenuOpen)}
                  className="px-3 py-1.5 rounded-full bg-[#f4f4f5] dark:bg-[#17171b] hover:bg-[#e4e4e7] dark:hover:bg-[#232328] text-[#171717] dark:text-[#f4f4f5] text-[13px] border border-[#ebebeb] dark:border-[#2c2c33] transition cursor-pointer flex items-center gap-1.5 max-w-[280px] font-sans"
                  aria-label={t('drawer.taskModel', { model: modelId })}
                >
                  <span className="truncate">{providerModelLabel}</span>
                  <ChevronDown className="h-3 w-3 block shrink-0 text-[#8f8f8f] dark:text-[#71717a]" />
                </button>
                <EffortSelector
                  taskEffort={activeSubagentId ? (subConfig?.reasoningEffort ?? task.reasoningEffort) : task.reasoningEffort}
                  personaEffort={personaEffort}
                  onSelectEffort={onSelectEffort}
                  onOpen={() => setIsModelMenuOpen(false)}
                />
                {isModelMenuOpen && (
                  <div className="absolute bottom-full right-0 mb-2 w-64 rounded-xl bg-white dark:bg-[#17171b] border border-[#ebebeb] dark:border-[#2c2c33] shadow-lg p-1.5 z-30 text-xs max-h-60 overflow-y-auto">
                    <div className="px-2 py-1 text-[#8f8f8f] dark:text-[#71717a] text-[10px] uppercase font-sans">{t('drawer.selectModel')}</div>
                    {groupedModels.map((m) => (
                      <button
                        key={`${m.providerId}-${m.id}`}
                        type="button"
                        onClick={() => {
                          onSelectPair(m.providerId, m.id);
                          setIsModelMenuOpen(false);
                        }}
                        className={`w-full text-left px-2.5 py-2 rounded-lg flex items-center justify-between cursor-pointer font-sans ${
                          modelId === m.id ? 'bg-[#f4f4f5] dark:bg-[#232328] text-[#171717] dark:text-[#f4f4f5]' : 'text-[#4d4d4d] dark:text-[#a1a1aa] hover:bg-[#f9f9f9] dark:hover:bg-[#232328]'
                        }`}
                      >
                        <span className="truncate">{m.label}</span>
                        {modelId === m.id && <span className="text-emerald-600 dark:text-emerald-400 shrink-0 ml-1">✓</span>}
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
                  <Square className="h-3.5 w-3.5 block shrink-0 fill-current" />
                </button>
              ) : hasContent ? (
                <button
                  type="button"
                  onClick={() => handleSubmit()}
                  className="w-8 h-8 rounded-full bg-[#171717] dark:bg-[#f4f4f5] hover:bg-black dark:hover:bg-white text-white dark:text-[#0b0b0e] flex items-center justify-center shadow-xs transition-colors cursor-pointer"
                  aria-label={t('drawer.sendMessage')}
                  title={t('drawer.sendMessage')}
                >
                  <ArrowUp className="h-4 w-4 block shrink-0 stroke-[2.5]" />
                </button>
              ) : (
                <button
                  type="button"
                  onClick={() => handleSubmit()}
                  className="h-8 px-2.5 rounded-full bg-[#171717] dark:bg-[#f4f4f5] hover:bg-black dark:hover:bg-white text-white dark:text-[#0b0b0e] flex items-center gap-1.5 shadow-xs transition-colors cursor-pointer text-xs font-medium"
                  aria-label={t('drawer.runLoop')}
                  title={t('drawer.runLoop')}
                >
                  <Play className="h-3.5 w-3.5 block shrink-0 fill-current ml-0.5" />
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
