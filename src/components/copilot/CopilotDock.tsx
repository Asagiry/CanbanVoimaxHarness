import { useState, useRef, useEffect, useCallback } from 'react';
import {
  Bot,
  PanelRightClose,
  Trash2,
  ArrowUp,
  Square,
  ChevronDown,
  ArrowDown,
  Sparkles,
  Check,
  Cpu,
} from 'lucide-react';
import { useCopilotStore, CopilotMessage } from '../../stores/copilotStore';
import MarkdownContent from './MarkdownContent';
import ThinkingCard from './ThinkingCard';
import ToolExecutionCard from './ToolExecutionCard';

/**
 * CVH Copilot (Sidecar Architect Dock)
 * Strict White Paper / Vercel design system:
 * bg-white, left border 1px #ebebeb, graphite ink typography #171717, zero dark slop.
 */
export default function CopilotDock() {
  const {
    isOpen,
    width,
    messages,
    isStreaming,
    input,
    selectedModel,
    availableModels,
    statusText,
    setIsOpen,
    setWidth,
    setInput,
    setSelectedModel,
    sendMessage,
    abortStreaming,
    loadHistory,
    clearHistory,
  } = useCopilotStore();

  const [confirmClear, setConfirmClear] = useState(false);
  const [modelDropdownOpen, setModelDropdownOpen] = useState(false);
  const [isResizing, setIsResizing] = useState(false);
  const [userScrolledUp, setUserScrolledUp] = useState(false);

  const messagesEndRef = useRef<HTMLDivElement>(null);
  const scrollContainerRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const modelDropdownRef = useRef<HTMLDivElement>(null);

  // Load history on mount
  useEffect(() => {
    void loadHistory();
  }, [loadHistory]);

  // Close model dropdown on click outside
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (
        modelDropdownRef.current &&
        !modelDropdownRef.current.contains(e.target as Node)
      ) {
        setModelDropdownOpen(false);
      }
    };
    if (modelDropdownOpen) {
      document.addEventListener('mousedown', handleClickOutside);
    }
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [modelDropdownOpen]);

  // Auto-scroll to bottom unless user scrolled up
  const scrollToBottom = useCallback((smooth = true) => {
    if (messagesEndRef.current) {
      messagesEndRef.current.scrollIntoView({
        behavior: smooth ? 'smooth' : 'auto',
      });
    }
  }, []);

  useEffect(() => {
    if (!userScrolledUp) {
      scrollToBottom(false);
    }
  }, [messages, isStreaming, userScrolledUp, scrollToBottom]);

  // Track scroll position to allow user to read older messages without snapping
  const handleScroll = () => {
    const container = scrollContainerRef.current;
    if (!container) return;
    const { scrollTop, scrollHeight, clientHeight } = container;
    const distanceToBottom = scrollHeight - scrollTop - clientHeight;
    setUserScrolledUp(distanceToBottom > 80);
  };

  // Auto-resize textarea
  const adjustTextareaHeight = () => {
    const textarea = textareaRef.current;
    if (!textarea) return;
    textarea.style.height = 'auto';
    const nextHeight = Math.min(Math.max(textarea.scrollHeight, 40), 160);
    textarea.style.height = `${nextHeight}px`;
  };

  const handleInputChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setInput(e.target.value);
    adjustTextareaHeight();
  };

  const handleSubmit = (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    if (!input.trim() || isStreaming) return;
    const textToSend = input;
    void sendMessage(textToSend);
    setUserScrolledUp(false);
    if (textareaRef.current) {
      textareaRef.current.style.height = '40px';
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  };

  // Drag resize handler
  const handleMouseDownResize = (e: React.MouseEvent) => {
    e.preventDefault();
    setIsResizing(true);
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';

    const startX = e.clientX;
    const startWidth = width;

    const handleMouseMove = (moveEvent: MouseEvent) => {
      // Dock is on the right, dragging left increases width
      const deltaX = startX - moveEvent.clientX;
      const nextWidth = Math.min(650, Math.max(320, startWidth + deltaX));
      setWidth(nextWidth);
    };

    const handleMouseUp = () => {
      setIsResizing(false);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
  };

  // Clear confirmation timer
  const handleClearClick = () => {
    if (!confirmClear) {
      setConfirmClear(true);
      setTimeout(() => setConfirmClear(false), 3000);
    } else {
      void clearHistory();
      setConfirmClear(false);
    }
  };

  if (!isOpen) {
    return null;
  }

  return (
    <aside
      style={{ width: `${width}px` }}
      className="h-full flex flex-col min-h-0 min-w-0 bg-white border-l border-[#ebebeb] relative shrink-0 z-20 select-none shadow-xs transition-[width] duration-75"
    >
      {/* Resizable drag-handle on left edge */}
      <div
        onMouseDown={handleMouseDownResize}
        className={`absolute left-0 top-0 bottom-0 w-1.5 -ml-1 cursor-col-resize z-30 transition-colors ${
          isResizing ? 'bg-[#7c3aed]' : 'hover:bg-[#7c3aed]/40'
        }`}
        title="Перетащите для изменения ширины панели"
      />

      {/* Header Bar */}
      <header className="h-11 px-3 border-b border-[#ebebeb] bg-white flex items-center justify-between shrink-0 select-none">
        {/* Left: Icon Bot + purple/indigo soft badge [АРХИТЕКТОР] / [COPILOT] */}
        <div className="flex items-center space-x-2 min-w-0">
          <div className="w-6 h-6 rounded-md bg-[#f5f3ff] border border-[#ddd6fe] flex items-center justify-center text-[#7c3aed] shrink-0">
            <Bot className="w-3.5 h-3.5" />
          </div>

          <div className="flex items-center space-x-1.5 min-w-0">
            <span className="font-semibold text-xs text-[#171717] tracking-tight truncate">
              CVH Copilot
            </span>
            <span className="px-1.5 py-0.5 rounded text-[10px] font-mono font-medium bg-[#f5f3ff] text-[#7c3aed] border border-[#ddd6fe] shrink-0">
              АРХИТЕКТОР
            </span>
          </div>

          {isStreaming && (
            <span className="flex h-2 w-2 relative shrink-0" title="Генерация ответа...">
              <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75" />
              <span className="relative inline-flex rounded-full h-2 w-2 bg-emerald-500" />
            </span>
          )}
        </div>

        {/* Right Quick Actions: Model Pill, Clear, Close */}
        <div className="flex items-center space-x-1 shrink-0">
          {/* Model Selector Pill */}
          <div className="relative" ref={modelDropdownRef}>
            <button
              type="button"
              onClick={() => setModelDropdownOpen(!modelDropdownOpen)}
              className="px-2 py-0.5 rounded-full bg-[#f4f4f5] hover:bg-[#ebebeb] border border-[#e4e4e7] text-[11px] font-mono text-[#52525b] flex items-center space-x-1 cursor-pointer transition"
              title="Выбор активной модели Архитектора"
            >
              <Cpu className="w-3 h-3 text-[#71717a]" />
              <span className="truncate max-w-[110px]">
                {selectedModel.replace(/-d{8}$/, '')}
              </span>
              <ChevronDown className="w-2.5 h-2.5 opacity-60" />
            </button>

            {modelDropdownOpen && (
              <div className="absolute right-0 top-full mt-1 w-52 bg-white rounded-lg border border-[#e4e4e7] shadow-lg py-1 z-50 text-xs">
                <div className="px-3 py-1 font-mono text-[10px] uppercase text-[#a1a1aa] border-b border-[#f4f4f5]">
                  Модель Архитектора
                </div>
                {availableModels.map((m) => {
                  const isCurrent = m === selectedModel;
                  return (
                    <button
                      key={m}
                      type="button"
                      onClick={() => {
                        setSelectedModel(m);
                        setModelDropdownOpen(false);
                      }}
                      className={`w-full text-left px-3 py-1.5 flex items-center justify-between font-mono text-[11px] hover:bg-[#f4f4f5] transition cursor-pointer ${
                        isCurrent ? 'text-[#7c3aed] font-medium bg-[#f5f3ff]/50' : 'text-[#27272a]'
                      }`}
                    >
                      <span className="truncate">{m}</span>
                      {isCurrent && <Check className="w-3 h-3 text-[#7c3aed] shrink-0 ml-1" />}
                    </button>
                  );
                })}
              </div>
            )}
          </div>

          {/* Quick Action: [Очистить чат] */}
          <button
            type="button"
            onClick={handleClearClick}
            className={`px-2 py-1 rounded-md text-[11px] font-sans flex items-center space-x-1 border transition cursor-pointer ${
              confirmClear
                ? 'bg-rose-50 border-rose-200 text-rose-600 font-medium'
                : 'bg-white hover:bg-[#f9f9f9] border-[#ebebeb] text-[#71717a] hover:text-[#171717]'
            }`}
            title="Очистить историю диалога"
          >
            <Trash2 className="w-3 h-3" />
            <span>{confirmClear ? 'Точно?' : 'Очистить'}</span>
          </button>

          {/* Close Dock Button */}
          <button
            type="button"
            onClick={() => setIsOpen(false)}
            className="p-1.5 rounded-md bg-white hover:bg-[#f9f9f9] border border-[#ebebeb] text-[#71717a] hover:text-[#171717] transition cursor-pointer"
            title="Свернуть док (Ctrl+B)"
          >
            <PanelRightClose className="w-3.5 h-3.5" />
          </button>
        </div>
      </header>

      {/* Body Stream: Auto-scrolling feed */}
      <div
        ref={scrollContainerRef}
        onScroll={handleScroll}
        className="flex-1 min-h-0 overflow-y-auto px-3.5 py-3 space-y-3.5 select-text relative"
      >
        {messages.length === 0 ? (
          <div className="h-full flex flex-col items-center justify-center text-center px-4 py-8 text-[#a1a1aa] select-none">
            <div className="w-10 h-10 rounded-full bg-[#f5f3ff] border border-[#ddd6fe] flex items-center justify-center text-[#7c3aed] mb-3">
              <Sparkles className="w-5 h-5" />
            </div>
            <h4 className="text-xs font-semibold text-[#18181b] mb-1">
              Архитектор готов к работе
            </h4>
            <p className="text-[11px] text-[#71717a] max-w-[260px] leading-relaxed">
              Задайте вопрос по структуре проекта, попросите декомпозировать фичу или спланировать шаги для карточек доски.
            </p>
          </div>
        ) : (
          messages.map((msg) => (
            <MessageItem key={msg.id} message={msg} />
          ))
        )}

        {/* Live streaming status banner */}
        {isStreaming && (
          <div className="p-2 rounded-lg bg-[#f5f3ff] border border-[#ddd6fe] flex items-center space-x-2 text-xs text-[#7c3aed] animate-pulse">
            <Sparkles className="w-3.5 h-3.5 shrink-0" />
            <span className="text-[11px] font-medium truncate">
              {statusText || 'Архитектор формирует архитектурное решение...'}
            </span>
          </div>
        )}

        <div ref={messagesEndRef} className="h-1" />
      </div>

      {/* Scroll to bottom button if user scrolled up */}
      {userScrolledUp && (
        <button
          type="button"
          onClick={() => {
            setUserScrolledUp(false);
            scrollToBottom(true);
          }}
          className="absolute bottom-24 right-4 z-30 px-2.5 py-1 rounded-full bg-[#18181b]/90 text-white font-mono text-[11px] shadow-md flex items-center space-x-1 cursor-pointer hover:bg-black transition backdrop-blur-xs"
        >
          <ArrowDown className="w-3 h-3" />
          <span>Вниз</span>
        </button>
      )}

      {/* Steer Bar (Footer) */}
      <footer className="p-3 border-t border-[#ebebeb] bg-white shrink-0 select-none">
        <form onSubmit={handleSubmit} className="relative">
          <div className="rounded-xl border border-[#ebebeb] bg-[#fafafa] focus-within:bg-white focus-within:border-[#7c3aed]/50 focus-within:ring-2 focus-within:ring-[#7c3aed]/10 transition-all p-1.5 flex flex-col">
            <textarea
              ref={textareaRef}
              rows={1}
              value={input}
              onChange={handleInputChange}
              onKeyDown={handleKeyDown}
              placeholder="Попросить Архитектора спланировать фичу, настроить доски или создать задачи..."
              className="w-full bg-transparent border-0 resize-none text-xs text-[#171717] placeholder-[#a1a1aa] px-2 py-1 focus:outline-none max-h-40 leading-relaxed font-sans"
            />

            {/* Bottom row: Hint + Submit / Stop Button */}
            <div className="flex items-center justify-between pt-1 px-1.5 text-[10px] text-[#a1a1aa]">
              <span className="font-mono">
                Enter — отправить, Shift+Enter — перенос
              </span>

              {isStreaming ? (
                <button
                  type="button"
                  onClick={() => void abortStreaming()}
                  className="w-7 h-7 rounded-full bg-rose-500 hover:bg-rose-600 text-white flex items-center justify-center cursor-pointer transition shadow-xs"
                  title="Остановить генерацию"
                >
                  <Square className="w-3 h-3 fill-current" />
                </button>
              ) : (
                <button
                  type="submit"
                  disabled={!input.trim()}
                  className={`w-7 h-7 rounded-full flex items-center justify-center transition shadow-xs cursor-pointer ${
                    input.trim()
                      ? 'bg-[#18181b] hover:bg-black text-white'
                      : 'bg-[#e4e4e7] text-[#a1a1aa] cursor-not-allowed'
                  }`}
                  title="Отправить запрос Архитектору"
                >
                  <ArrowUp className="w-3.5 h-3.5 stroke-[2.5]" />
                </button>
              )}
            </div>
          </div>
        </form>
      </footer>
    </aside>
  );
}

/**
 * Message Item with Support for:
 * - Markdown Content
 * - <thinking> blocks extraction
 * - Thought cards
 * - Tool execution badges
 */
function MessageItem({ message }: { message: CopilotMessage }) {
  const isUser = message.role === 'user';

  // Extract <thinking> tags from message content if present
  const { thinkingPart, markdownPart } = extractThinkingBlocks(message.content);
  const combinedThought = (message.thought ? message.thought + '\n' : '') + thinkingPart;

  if (isUser) {
    return (
      <div className="flex justify-end">
        <div className="max-w-[85%] rounded-2xl rounded-tr-sm bg-[#18181b] text-white px-3.5 py-2 text-xs leading-relaxed shadow-2xs">
          <div className="whitespace-pre-wrap select-text font-sans">
            {message.content}
          </div>
          {message.timestamp && (
            <div className="text-[9px] text-[#a1a1aa] font-mono text-right mt-1 tabular-nums">
              {message.timestamp}
            </div>
          )}
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col space-y-1.5 w-full">
      {/* Role Badge & Timestamp */}
      <div className="flex items-center justify-between text-[10px] text-[#71717a] px-0.5 select-none font-mono">
        <div className="flex items-center space-x-1.5">
          <div className="w-4 h-4 rounded-full bg-[#f5f3ff] border border-[#ddd6fe] flex items-center justify-center text-[#7c3aed]">
            <Bot className="w-2.5 h-2.5" />
          </div>
          <span className="font-semibold text-[#18181b]">CVH Архитектор</span>
        </div>
        {message.timestamp && (
          <span className="text-[#a1a1aa] tabular-nums">{message.timestamp}</span>
        )}
      </div>

      {/* Assistant Card Body */}
      <div className="rounded-xl border border-[#ebebeb] bg-[#ffffff] p-3 text-xs leading-relaxed shadow-2xs space-y-2">
        {/* Thinking Card if thought is present */}
        {combinedThought.trim() && (
          <ThinkingCard thought={combinedThought} isLive={message.isStreaming} />
        )}

        {/* Tool Execution Badges */}
        {message.tools && message.tools.length > 0 && (
          <div className="space-y-1">
            {message.tools.map((t) => (
              <ToolExecutionCard key={t.id} tool={t} />
            ))}
          </div>
        )}

        {/* Markdown Output */}
        {markdownPart.trim() ? (
          <MarkdownContent content={markdownPart} />
        ) : (
          message.isStreaming && !combinedThought.trim() && (
            <div className="flex items-center space-x-1.5 text-xs text-[#a1a1aa] font-mono italic">
              <span className="w-1.5 h-1.5 rounded-full bg-[#7c3aed] animate-ping" />
              <span>Формирование ответа...</span>
            </div>
          )
        )}
      </div>
    </div>
  );
}

/**
 * Extracts <thinking>...</thinking> or unclosed <thinking> from stream content
 */
function extractThinkingBlocks(rawContent: string): {
  thinkingPart: string;
  markdownPart: string;
} {
  if (!rawContent) return { thinkingPart: '', markdownPart: '' };

  let thinking = '';
  let markdown = rawContent;

  // Closed <thinking>...</thinking>
  const closedMatch = markdown.match(/<thinking>([\s\S]*?)<\/thinking>/i);
  if (closedMatch) {
    thinking += closedMatch[1] + '\n';
    markdown = markdown.replace(/<thinking>[\s\S]*?<\/thinking>/i, '').trim();
  }

  // Unclosed <thinking>... during streaming
  const openMatch = markdown.match(/<thinking>([\s\S]*)$/i);
  if (openMatch) {
    thinking += openMatch[1];
    markdown = markdown.replace(/<thinking>[\s\S]*$/i, '').trim();
  }

  return { thinkingPart: thinking.trim(), markdownPart: markdown };
}
