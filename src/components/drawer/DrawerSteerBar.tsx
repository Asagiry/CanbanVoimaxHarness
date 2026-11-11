import React, { useState, useRef, useEffect } from 'react';
import type { KanbanTask, ReasoningEffort } from '../../types';
import ContextGauge from './ContextGauge';
import EffortSelector from './EffortSelector';
import TaskTodoWidget from './TaskTodoWidget';
import { useDrawerStore } from '../../stores/drawerStore';
import { useTranslation } from '../../stores/languageStore';
import { getRoleBadgeStyle } from '../../lib/agentPalette';
import AgentIcon from '../common/AgentIcon';
import { formatSubagentRoleDisplay } from './subagentHelpers';
import {
  ArrowUp,
  Plus,
  Square,
  Play,
} from 'lucide-react';
import {
  AttachedImage,
  SteerImageBar,
  SteerModelMenu,
  useSteerModelResolution,
  useSteerImages,
} from './steer';

export * from './steer';

export interface DrawerSteerBarProps {
  task: KanbanTask;
  isRunning: boolean;
  turns: number;
  onSendPrompt: (promptText: string, attachedImages: AttachedImage[]) => Promise<boolean> | boolean | void;
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
  const [isModelMenuOpen, setIsModelMenuOpen] = useState(false);
  const { t, language } = useTranslation();
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const {
    activeSubagentId,
    activeSubagentRole,
    subagentNumber,
    isBoardSubtask,
    activePersona,
    effectiveGaugeTaskId,
    effectiveGaugePersona,
    effectiveLimit,
    matchedPersona,
    modelId,
    providerModelLabel,
    personaEffort,
    subConfig,
    groupedModels,
  } = useSteerModelResolution(task, selectableModels);

  const draftKey = activeSubagentId
    ? (activeSubagentId.includes(':sub:') ? activeSubagentId : `${task?.id}:sub:${activeSubagentId}`)
    : task?.id;

  const {
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
  } = useSteerImages({
    draftKey,
    interactiveInput,
    textareaRef,
  });

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

    const result = onSendPrompt(textToSend, currentImages);
    const clear = (ok: boolean) => {
      if (!ok) return;
      if (textareaRef.current) {
        textareaRef.current.style.height = 'auto';
      }
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
          <SteerImageBar attachedImages={attachedImages} onRemoveImage={handleRemoveImage} />

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
              <button
                type="button"
                onClick={() => chatFileInputRef.current?.click()}
                className="w-7 h-7 rounded-full bg-[#f4f4f5] dark:bg-[#17171b] hover:bg-[#171717] dark:hover:bg-[#f4f4f5] text-[#171717] dark:text-[#f4f4f5] hover:text-white dark:hover:text-[#0b0b0e] flex items-center justify-center transition cursor-pointer"
                aria-label={t('drawer.attachImage')}
              >
                <Plus className="h-4 w-4 block shrink-0" />
              </button>

              <input
                ref={chatFileInputRef}
                type="file"
                accept="image/png,image/jpeg,image/webp,image/gif"
                multiple
                className="hidden"
                onChange={handleChatFileSelect}
              />

              {(Boolean(activeSubagentId) || isToolAllowed('todo_write', task.stageId) || (task.todos && task.todos.length > 0)) && (
                <TaskTodoWidget task={task} align="left" />
              )}

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
              <SteerModelMenu
                modelId={modelId}
                providerModelLabel={providerModelLabel}
                groupedModels={groupedModels}
                isOpen={isModelMenuOpen}
                onToggle={() => setIsModelMenuOpen(!isModelMenuOpen)}
                onSelectPair={(providerId, modId) => {
                  onSelectPair(providerId, modId);
                  setIsModelMenuOpen(false);
                }}
              />
              <EffortSelector
                taskEffort={activeSubagentId ? (subConfig?.reasoningEffort ?? task.reasoningEffort) : task.reasoningEffort}
                personaEffort={personaEffort}
                onSelectEffort={onSelectEffort}
                onOpen={() => setIsModelMenuOpen(false)}
              />

              {/* Action Button */}
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

        {/* Elegant Telemetry Status Line */}
        <div className="flex items-center justify-center">
          <ContextGauge taskId={effectiveGaugeTaskId} persona={effectiveGaugePersona} turns={turns} contextWindow={effectiveLimit} />
        </div>
      </div>
    </footer>
  );
}
