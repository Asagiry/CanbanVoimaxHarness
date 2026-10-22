import React, { useState, useEffect, useRef, useCallback } from 'react';
import { createPortal } from 'react-dom';
import {
  X,
  ZoomIn,
  ZoomOut,
  Copy,
  Check,
  Maximize2,
  Image as ImageIcon,
} from 'lucide-react';

export interface ImageLightboxModalProps {
  src: string;
  alt?: string;
  onClose: () => void;
}

export default function ImageLightboxModal({
  src,
  alt = 'Изображение',
  onClose,
}: ImageLightboxModalProps) {
  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [isDragging, setIsDragging] = useState(false);
  const [dragStart, setDragStart] = useState({ x: 0, y: 0 });
  const [hasMoved, setHasMoved] = useState(false);
  const [copied, setCopied] = useState(false);
  const [isLoaded, setIsLoaded] = useState(false);

  const containerRef = useRef<HTMLDivElement>(null);
  const imgRef = useRef<HTMLImageElement>(null);
  const copyTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Lock body scroll while lightbox is open (§ 2.13) & cleanup timers (§ 2.14)
  useEffect(() => {
    const origOverflow = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    return () => {
      document.body.style.overflow = origOverflow;
      if (copyTimerRef.current) {
        clearTimeout(copyTimerRef.current);
        copyTimerRef.current = null;
      }
    };
  }, []);

  const handleZoomIn = useCallback(() => {
    setZoom((prev) => Math.min(5, Number((prev + 0.25).toFixed(2))));
  }, []);

  const handleZoomOut = useCallback(() => {
    setZoom((prev) => {
      const next = Math.max(0.5, Number((prev - 0.25).toFixed(2)));
      if (next === 1) setPan({ x: 0, y: 0 });
      return next;
    });
  }, []);

  const handleReset = useCallback(() => {
    setZoom(1);
    setPan({ x: 0, y: 0 });
  }, []);

  // Keyboard navigation: Escape to close, +/-/0 to zoom
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        onClose();
      } else if (e.key === '+' || e.key === '=') {
        e.preventDefault();
        handleZoomIn();
      } else if (e.key === '-' || e.key === '_') {
        e.preventDefault();
        handleZoomOut();
      } else if (e.key === '0') {
        e.preventDefault();
        handleReset();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [onClose, handleZoomIn, handleZoomOut, handleReset]);

  // Mouse wheel zoom
  const handleWheel = (e: React.WheelEvent) => {
    e.preventDefault();
    if (e.deltaY < 0) {
      handleZoomIn();
    } else {
      handleZoomOut();
    }
  };

  // Dragging / Pan when zoomed
  const handleMouseDown = (e: React.MouseEvent) => {
    // Only drag with primary mouse button
    if (e.button !== 0) return;
    setIsDragging(true);
    setHasMoved(false);
    setDragStart({ x: e.clientX - pan.x, y: e.clientY - pan.y });
  };

  const handleMouseMove = (e: React.MouseEvent) => {
    if (!isDragging) return;
    const dx = Math.abs(e.clientX - (dragStart.x + pan.x));
    const dy = Math.abs(e.clientY - (dragStart.y + pan.y));
    if (dx > 3 || dy > 3) {
      setHasMoved(true);
    }
    setPan({
      x: e.clientX - dragStart.x,
      y: e.clientY - dragStart.y,
    });
  };

  const handleMouseUp = () => {
    setIsDragging(false);
  };

  // Click image: toggle zoom between 1x and 2x (if not dragging)
  const handleImageClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (hasMoved) return;
    if (zoom === 1) {
      setZoom(2);
    } else {
      handleReset();
    }
  };

  // Copy image URL or Data URL to clipboard
  const handleCopy = async (e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      await navigator.clipboard.writeText(src);
      setCopied(true);
      if (copyTimerRef.current) {
        clearTimeout(copyTimerRef.current);
      }
      copyTimerRef.current = setTimeout(() => {
        setCopied(false);
        copyTimerRef.current = null;
      }, 2000);
    } catch (err) {
      console.error('Failed to copy image source:', err);
    }
  };

  // Backdrop click closes lightbox (if not clicking controls)
  const handleBackdropClick = (e: React.MouseEvent) => {
    if (e.target === containerRef.current && !hasMoved) {
      onClose();
    }
  };

  const displayName = alt || 'Изображение';

  const modal = (
    <div
      ref={containerRef}
      onClick={handleBackdropClick}
      onWheel={handleWheel}
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUp}
      onMouseLeave={handleMouseUp}
      className="fixed inset-0 z-[100] bg-black/90 backdrop-blur-md flex flex-col select-none overflow-hidden transition-opacity duration-200 ease-out"
      style={{ animation: 'cvhFadeIn 0.2s ease-out' }}
    >
      <style>{'\
        @keyframes cvhFadeIn {\
          from { opacity: 0; }\
          to { opacity: 1; }\
        }\
        @keyframes cvhZoomIn {\
          from { opacity: 0; transform: scale(0.92); }\
          to { opacity: 1; transform: scale(1); }\
        }\
      '}</style>

      {/* Top Tactical Controls Bar */}
      <div
        onClick={(e) => e.stopPropagation()}
        className="flex items-center justify-between px-4 py-3 bg-black/40 border-b border-white/10 z-10 shrink-0 text-white"
      >
        {/* Left: Image Info */}
        <div className="flex items-center space-x-2.5 min-w-0 pr-4">
          <div className="p-1 rounded bg-white/10 text-zinc-300 shrink-0">
            <ImageIcon className="w-4 h-4" />
          </div>
          <span className="text-xs font-mono font-medium text-zinc-200 truncate max-w-[260px] md:max-w-md">
            {displayName}
          </span>
        </div>

        {/* Center: Zoom Controls Pill */}
        <div className="flex items-center space-x-1 px-2 py-1 rounded-lg bg-zinc-900/80 border border-white/10 text-zinc-300 shadow-lg">
          <button
            type="button"
            onClick={handleZoomOut}
            disabled={zoom <= 0.5}
            className="p-1 rounded hover:bg-white/10 hover:text-white disabled:opacity-30 disabled:hover:bg-transparent disabled:cursor-not-allowed transition-colors cursor-pointer"
            aria-label="Уменьшить (-)"
          >
            <ZoomOut className="w-3.5 h-3.5" />
          </button>

          <button
            type="button"
            onClick={handleReset}
            className="px-2 py-0.5 text-[11px] font-mono tabular-nums text-zinc-200 hover:text-white hover:bg-white/10 rounded transition-colors cursor-pointer"
            aria-label="Сбросить масштаб (0)"
          >
            {Math.round(zoom * 100)}%
          </button>

          <button
            type="button"
            onClick={handleZoomIn}
            disabled={zoom >= 5}
            className="p-1 rounded hover:bg-white/10 hover:text-white disabled:opacity-30 disabled:hover:bg-transparent disabled:cursor-not-allowed transition-colors cursor-pointer"
            aria-label="Увеличить (+)"
          >
            <ZoomIn className="w-3.5 h-3.5" />
          </button>

          <div className="w-px h-3.5 bg-white/10 mx-0.5" />

          <button
            type="button"
            onClick={handleReset}
            className="p-1 rounded hover:bg-white/10 hover:text-white transition-colors cursor-pointer"
            aria-label="По размеру экрана"
          >
            <Maximize2 className="w-3.5 h-3.5" />
          </button>
        </div>

        {/* Right: Actions & Close */}
        <div className="flex items-center space-x-2 shrink-0">
          <button
            type="button"
            onClick={handleCopy}
            className="flex items-center space-x-1.5 px-2.5 py-1 rounded-md bg-white/10 hover:bg-white/20 text-zinc-200 hover:text-white text-xs font-mono transition-colors cursor-pointer"
            aria-label="Скопировать ссылку/данные изображения"
          >
            {copied ? (
              <>
                <Check className="w-3.5 h-3.5 text-emerald-400" />
                <span className="hidden sm:inline text-emerald-400">Скопировано</span>
              </>
            ) : (
              <>
                <Copy className="w-3.5 h-3.5" />
                <span className="hidden sm:inline">Копировать</span>
              </>
            )}
          </button>

          <button
            type="button"
            onClick={onClose}
            className="flex items-center space-x-1 p-1.5 rounded-full bg-white/10 hover:bg-rose-600/80 text-zinc-200 hover:text-white transition-colors cursor-pointer"
            aria-label="Закрыть (Escape)"
          >
            <X className="w-4 h-4" />
          </button>
        </div>
      </div>

      {/* Main Viewport Container */}
      <div
        className="flex-1 flex items-center justify-center p-4 md:p-8 overflow-hidden relative cursor-pointer"
        style={{
          cursor: zoom > 1 ? (isDragging ? 'grabbing' : 'grab') : 'zoom-in',
        }}
      >
        <div
          style={{
            transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})`,
            transition: isDragging ? 'none' : 'transform 0.15s ease-out',
            animation: 'cvhZoomIn 0.25s cubic-bezier(0.16, 1, 0.3, 1)',
          }}
          className="relative max-w-full max-h-full flex items-center justify-center pointer-events-auto"
        >
          <img
            ref={imgRef}
            src={src}
            alt={displayName}
            onLoad={() => setIsLoaded(true)}
            onClick={handleImageClick}
            draggable={false}
            className={`max-w-[92vw] max-h-[85vh] object-contain rounded-md shadow-2xl transition-opacity duration-200 select-none ${
              isLoaded ? 'opacity-100' : 'opacity-0'
            }`}
          />
        </div>
      </div>

      {/* Footer Navigation / Help Hint */}
      <div className="px-4 py-2 bg-black/40 border-t border-white/5 flex items-center justify-between text-[11px] font-mono text-zinc-400 z-10 shrink-0">
        <div className="flex items-center space-x-3">
          <span>Клик: {zoom === 1 ? 'зум 2x' : 'сброс'}</span>
          <span>•</span>
          <span>Колесико: зум +/-</span>
          {zoom > 1 && (
            <>
              <span>•</span>
              <span className="text-amber-400">Перетаскивание активно</span>
            </>
          )}
        </div>
        <div className="flex items-center space-x-1 text-zinc-500">
          <kbd className="px-1.5 py-0.5 rounded bg-zinc-800 border border-zinc-700 text-[10px]">Esc</kbd>
          <span>чтобы закрыть</span>
        </div>
      </div>
    </div>
  );

  return createPortal(modal, document.body);
}
