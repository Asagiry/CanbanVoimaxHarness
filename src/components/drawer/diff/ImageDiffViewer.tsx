import { useState, useEffect, useRef } from 'react';
import { createPortal } from 'react-dom';
import {
  AlertCircle,
  Image as ImageIcon,
  ZoomIn,
  ZoomOut,
  X,
  Loader2,
  Sliders,
  Columns,
  Layers,
} from 'lucide-react';
import { GitImageDiff } from '../../../types';
import { useTranslation } from '../../../stores/languageStore';
import { formatBytes } from './utils';

export interface ImageDiffViewerProps {
  taskId: string;
  filePath: string;
}

export type ImageViewMode = '2up' | 'split' | 'onion' | 'new' | 'old';

export function ImageDiffViewer({ taskId, filePath }: ImageDiffViewerProps) {
  const { t } = useTranslation();
  const [data, setData] = useState<GitImageDiff | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [viewMode, setViewMode] = useState<ImageViewMode>('2up');
  const [splitPercent, setSplitPercent] = useState(50);
  const [isDraggingSplit, setIsDraggingSplit] = useState(false);
  const [onionOpacity, setOnionOpacity] = useState(50);
  const [currentDims, setCurrentDims] = useState<{ width: number; height: number } | null>(null);
  const [prevDims, setPrevDims] = useState<{ width: number; height: number } | null>(null);
  const [lightbox, setLightbox] = useState<{ url: string; title: string } | null>(null);
  const [lightboxZoom, setLightboxZoom] = useState(false);
  const splitContainerRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    let isMounted = true;
    setIsLoading(true);
    setError(null);

    const loadImageDiff = async () => {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const res = await invoke<GitImageDiff>('get_task_image_diff', {
          taskId,
          path: filePath,
        });
        if (isMounted) {
          setData(res);
          if (res.currentDataUrl && res.previousDataUrl) {
            setViewMode('2up');
          } else if (res.currentDataUrl) {
            setViewMode('new');
          } else if (res.previousDataUrl) {
            setViewMode('old');
          }
        }
      } catch (err: unknown) {
        if (isMounted) {
          console.error('get_task_image_diff failed:', err);
          const msg =
            typeof err === 'string'
              ? err
              : err instanceof Error
              ? err.message
              : 'Failed to load image preview';
          setError(msg);
        }
      } finally {
        if (isMounted) {
          setIsLoading(false);
        }
      }
    };

    loadImageDiff();

    return () => {
      isMounted = false;
    };
  }, [taskId, filePath]);

  // Keyboard shortcut (Escape) to close lightbox
  useEffect(() => {
    if (!lightbox) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setLightbox(null);
        setLightboxZoom(false);
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [lightbox]);

  // Drag interaction for Split / Swipe comparison
  useEffect(() => {
    if (!isDraggingSplit) return;

    const handleMouseMove = (e: MouseEvent) => {
      if (!splitContainerRef.current) return;
      const rect = splitContainerRef.current.getBoundingClientRect();
      if (rect.width <= 0) return;
      const clientX = Math.min(Math.max(rect.left, e.clientX), rect.right);
      const ratio = ((clientX - rect.left) / rect.width) * 100;
      setSplitPercent(Math.round(ratio));
    };

    const handleMouseUp = () => {
      setIsDraggingSplit(false);
    };

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  }, [isDraggingSplit]);

  if (isLoading) {
    return (
      <div className="border-t border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#0d1117] p-5 flex flex-col items-center justify-center space-y-2 text-[#8f8f8f] dark:text-[#8b949e]">
        <Loader2 className="h-4 w-4 animate-spin text-amber-500" />
        <span className="text-xs font-mono">{t('gitDiff.imageLoading')}</span>
      </div>
    );
  }

  if (error || !data) {
    return (
      <div className="border-t border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#0d1117] p-4 flex flex-col items-center justify-center space-y-1.5 text-rose-500">
        <AlertCircle className="h-4 w-4" />
        <span className="text-xs font-mono text-center">{error || t('gitDiff.imageFailed')}</span>
      </div>
    );
  }

  const hasBoth = Boolean(data.currentDataUrl && data.previousDataUrl);
  const isDeleted = !data.currentDataUrl && Boolean(data.previousDataUrl);
  const isAdded = Boolean(data.currentDataUrl) && !data.previousDataUrl;

  const checkerboardClasses =
    'relative flex items-center justify-center p-2 rounded overflow-hidden select-none ' +
    'bg-[#f8fafc] dark:bg-[#090d16] ' +
    '[background-image:linear-gradient(45deg,#e2e8f0_25%,transparent_25%),linear-gradient(-45deg,#e2e8f0_25%,transparent_25%),linear-gradient(45deg,transparent_75%,#e2e8f0_75%),linear-gradient(-45deg,transparent_75%,#e2e8f0_75%)] ' +
    'dark:[background-image:linear-gradient(45deg,#1c2128_25%,transparent_25%),linear-gradient(-45deg,#1c2128_25%,transparent_25%),linear-gradient(45deg,transparent_75%,#1c2128_75%),linear-gradient(-45deg,transparent_75%,#1c2128_75%)] ' +
    '[background-size:16px_16px] [background-position:0_0,0_8px,8px_-8px,-8px_0px] border border-[#e5e7eb] dark:border-[#21262d]';

  const bytesDelta =
    data.currentBytes && data.previousBytes ? data.currentBytes - data.previousBytes : null;

  return (
    <div className="border-t border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#0d1117] font-mono text-xs select-text overflow-hidden">
      {/* Subheader Toolbar: Mode Toggles & Size Badges */}
      <div className="flex items-center justify-between px-3 py-1.5 border-b border-[#ebebeb] dark:border-[#21262d] bg-[#fcfcfc] dark:bg-[#161b22] text-[11px] gap-2 flex-wrap">
        {hasBoth ? (
          <div className="flex items-center border border-[#e5e7eb] dark:border-[#30363d] rounded p-0.5 bg-[#f4f4f5] dark:bg-[#0d1117] gap-0.5">
            <button
              type="button"
              onClick={() => setViewMode('2up')}
              className={'flex items-center gap-1 px-2 py-0.5 rounded text-[10px] font-semibold transition cursor-pointer ' +
                (viewMode === '2up'
                  ? 'bg-white dark:bg-[#21262d] text-[#171717] dark:text-[#f0f6fc] shadow-xs'
                  : 'text-[#666666] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc]')}
              title="2-up Side-by-Side comparison"
            >
              <Columns className="h-3 w-3" />
              <span>{t('gitDiff.image2Up')}</span>
            </button>
            <button
              type="button"
              onClick={() => setViewMode('split')}
              className={'flex items-center gap-1 px-2 py-0.5 rounded text-[10px] font-semibold transition cursor-pointer ' +
                (viewMode === 'split'
                  ? 'bg-white dark:bg-[#21262d] text-[#171717] dark:text-[#f0f6fc] shadow-xs'
                  : 'text-[#666666] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc]')}
              title="Split swipe slider comparison"
            >
              <Sliders className="h-3 w-3" />
              <span>{t('gitDiff.imageSplit')}</span>
            </button>
            <button
              type="button"
              onClick={() => setViewMode('onion')}
              className={'flex items-center gap-1 px-2 py-0.5 rounded text-[10px] font-semibold transition cursor-pointer ' +
                (viewMode === 'onion'
                  ? 'bg-white dark:bg-[#21262d] text-[#171717] dark:text-[#f0f6fc] shadow-xs'
                  : 'text-[#666666] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc]')}
              title="Onion skin opacity comparison"
            >
              <Layers className="h-3 w-3" />
              <span>{t('gitDiff.imageOnion')}</span>
            </button>
            <button
              type="button"
              onClick={() => setViewMode('new')}
              className={'px-2 py-0.5 rounded text-[10px] font-semibold transition cursor-pointer ' +
                (viewMode === 'new'
                  ? 'bg-white dark:bg-[#21262d] text-[#171717] dark:text-[#f0f6fc] shadow-xs'
                  : 'text-[#666666] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc]')}
            >
              {t('gitDiff.imageNew')}
            </button>
            <button
              type="button"
              onClick={() => setViewMode('old')}
              className={'px-2 py-0.5 rounded text-[10px] font-semibold transition cursor-pointer ' +
                (viewMode === 'old'
                  ? 'bg-white dark:bg-[#21262d] text-[#171717] dark:text-[#f0f6fc] shadow-xs'
                  : 'text-[#666666] dark:text-[#8b949e] hover:text-[#171717] dark:hover:text-[#f0f6fc]')}
            >
              {t('gitDiff.imageOld')}
            </button>
          </div>
        ) : isAdded ? (
          <span className="px-1.5 py-0.5 rounded text-[10px] font-bold bg-[#f0fdf4] dark:bg-[#12281e] text-[#15803d] dark:text-[#7ee787] border border-[#bbf7d0] dark:border-[#238636]">
            {t('gitDiff.imageAdded')}
          </span>
        ) : isDeleted ? (
          <span className="px-1.5 py-0.5 rounded text-[10px] font-bold bg-[#fef2f2] dark:bg-[#3b1219] text-[#b91c1c] dark:text-[#ff7b72] border border-[#fecaca] dark:border-[#da3633]">
            {t('gitDiff.imageDeleted')}
          </span>
        ) : (
          <span className="text-[#8f8f8f] dark:text-[#8b949e] text-[10px]">
            {data.mimeType}
          </span>
        )}

        {/* Size Metrics & Delta */}
        <div className="flex items-center space-x-2 tabular-nums text-[10px] shrink-0">
          {data.previousBytes && !hasBoth && (
            <span className="text-[#8f8f8f] dark:text-[#8b949e]">
              {formatBytes(data.previousBytes)}
            </span>
          )}
          {data.currentBytes && !hasBoth && (
            <span className="text-[#8f8f8f] dark:text-[#8b949e]">
              {formatBytes(data.currentBytes)}
            </span>
          )}
          {hasBoth && bytesDelta !== null && (
            <span
              className={'font-semibold ' +
                (bytesDelta > 0
                  ? 'text-[#b91c1c] dark:text-[#ff7b72]'
                  : bytesDelta < 0
                  ? 'text-[#15803d] dark:text-[#7ee787]'
                  : 'text-[#8f8f8f] dark:text-[#8b949e]')}
            >
              {bytesDelta > 0 ? ('+' + formatBytes(bytesDelta)) : bytesDelta < 0 ? ('-' + formatBytes(Math.abs(bytesDelta))) : '±0 B'}
            </span>
          )}
        </div>
      </div>

      {/* Main Diff Content Container */}
      {hasBoth && viewMode === '2up' ? (
        <div className="grid grid-cols-2 gap-2 p-2.5">
          {/* Old / Previous */}
          <div className="flex flex-col space-y-1">
            <div className="flex items-center justify-between text-[10px] px-0.5">
              <span className="text-[#b91c1c] dark:text-[#ff7b72] font-semibold">
                {t('gitDiff.imageOld')}
              </span>
              <div className="flex items-center space-x-1.5 text-[#8f8f8f] dark:text-[#8b949e] tabular-nums text-[9px]">
                {data.previousBytes && <span>{formatBytes(data.previousBytes)}</span>}
                {prevDims && <span>{prevDims.width}&times;{prevDims.height}</span>}
              </div>
            </div>
            <div
              className={checkerboardClasses + ' cursor-zoom-in group min-h-[140px]'}
              onClick={() => setLightbox({ url: data.previousDataUrl!, title: filePath + ' (' + t('gitDiff.imageOld') + ')' })}
              title={t('gitDiff.imageClickToZoom')}
            >
              <img
                src={data.previousDataUrl!}
                alt="Previous version"
                className="max-h-[220px] max-w-full object-contain group-hover:scale-[1.02] transition-transform"
                onLoad={(e) => setPrevDims({ width: e.currentTarget.naturalWidth, height: e.currentTarget.naturalHeight })}
              />
            </div>
          </div>

          {/* New / Current */}
          <div className="flex flex-col space-y-1">
            <div className="flex items-center justify-between text-[10px] px-0.5">
              <span className="text-[#15803d] dark:text-[#7ee787] font-semibold">
                {t('gitDiff.imageNew')}
              </span>
              <div className="flex items-center space-x-1.5 text-[#8f8f8f] dark:text-[#8b949e] tabular-nums text-[9px]">
                {data.currentBytes && <span>{formatBytes(data.currentBytes)}</span>}
                {currentDims && <span>{currentDims.width}&times;{currentDims.height}</span>}
              </div>
            </div>
            <div
              className={checkerboardClasses + ' cursor-zoom-in group min-h-[140px]'}
              onClick={() => setLightbox({ url: data.currentDataUrl!, title: filePath + ' (' + t('gitDiff.imageNew') + ')' })}
              title={t('gitDiff.imageClickToZoom')}
            >
              <img
                src={data.currentDataUrl!}
                alt="Current version"
                className="max-h-[220px] max-w-full object-contain group-hover:scale-[1.02] transition-transform"
                onLoad={(e) => setCurrentDims({ width: e.currentTarget.naturalWidth, height: e.currentTarget.naturalHeight })}
              />
            </div>
          </div>
        </div>
      ) : hasBoth && viewMode === 'split' ? (
        /* Split / Swipe Mode with Slider */
        <div className="p-2.5 flex flex-col space-y-2">
          <div className="flex items-center justify-between text-[10px] px-0.5">
            <span className="text-[#b91c1c] dark:text-[#ff7b72] font-semibold">
              {t('gitDiff.imageOld')} ({splitPercent}%)
            </span>
            <span className="text-[#15803d] dark:text-[#7ee787] font-semibold">
              {t('gitDiff.imageNew')} ({100 - splitPercent}%)
            </span>
          </div>

          {/* Interactive Split Viewer Stage */}
          <div
            ref={splitContainerRef}
            className={checkerboardClasses + ' min-h-[180px] max-h-[340px] relative overflow-hidden select-none cursor-ew-resize group'}
            onMouseDown={() => setIsDraggingSplit(true)}
            onClick={(e) => {
              const rect = e.currentTarget.getBoundingClientRect();
              if (rect.width > 0) {
                const ratio = ((e.clientX - rect.left) / rect.width) * 100;
                setSplitPercent(Math.round(ratio));
              }
            }}
          >
            {/* Base layer: Old Image */}
            <img
              src={data.previousDataUrl!}
              alt="Previous version"
              className="max-h-[300px] max-w-full object-contain pointer-events-none"
              onLoad={(e) => setPrevDims({ width: e.currentTarget.naturalWidth, height: e.currentTarget.naturalHeight })}
            />

            {/* Top clipped layer: New Image */}
            <div
              className="absolute inset-0 flex items-center justify-center overflow-hidden pointer-events-none"
              style={{ clipPath: 'inset(0 0 0 ' + splitPercent + '%)' }}
            >
              <img
                src={data.currentDataUrl!}
                alt="Current version"
                className="max-h-[300px] max-w-full object-contain"
                onLoad={(e) => setCurrentDims({ width: e.currentTarget.naturalWidth, height: e.currentTarget.naturalHeight })}
              />
            </div>

            {/* Split divider vertical line with handle */}
            <div
              className="absolute top-0 bottom-0 w-0.5 bg-amber-500 shadow-md flex items-center justify-center pointer-events-none transition-none"
              style={{ left: splitPercent + '%' }}
            >
              <div className="w-5 h-5 rounded-full bg-amber-500 text-black flex items-center justify-center shadow-lg transform -translate-x-1/2 select-none text-[9px] font-bold">
                &harr;
              </div>
            </div>
          </div>

          {/* Precision Swipe Slider Control */}
          <div className="flex items-center space-x-2 px-1">
            <span className="text-[10px] text-[#b91c1c] dark:text-[#ff7b72] font-semibold">{t('gitDiff.imageOld')}</span>
            <input
              type="range"
              min="0"
              max="100"
              value={splitPercent}
              onChange={(e) => setSplitPercent(parseInt(e.target.value, 10))}
              className="flex-1 h-1.5 bg-[#e5e7eb] dark:bg-[#21262d] rounded-lg appearance-none cursor-pointer accent-amber-500"
            />
            <span className="text-[10px] text-[#15803d] dark:text-[#7ee787] font-semibold">{t('gitDiff.imageNew')}</span>
          </div>
        </div>
      ) : hasBoth && viewMode === 'onion' ? (
        /* Onion Skin Mode with Opacity Slider */
        <div className="p-2.5 flex flex-col space-y-2">
          <div className="flex items-center justify-between text-[10px] px-0.5">
            <span className="text-[#8f8f8f] dark:text-[#8b949e]">
              {t('gitDiff.imageOld')}: {100 - onionOpacity}%
            </span>
            <span className="text-amber-600 dark:text-amber-400 font-semibold tabular-nums">
              {onionOpacity}% {t('gitDiff.imageNew')}
            </span>
          </div>

          {/* Onion Stage: Stacked Images */}
          <div className={checkerboardClasses + ' min-h-[180px] max-h-[340px] relative overflow-hidden select-none'}>
            {/* Bottom Layer: Old */}
            <img
              src={data.previousDataUrl!}
              alt="Previous version"
              className="max-h-[300px] max-w-full object-contain"
              onLoad={(e) => setPrevDims({ width: e.currentTarget.naturalWidth, height: e.currentTarget.naturalHeight })}
            />

            {/* Top Layer: New with adjustable opacity */}
            <div
              className="absolute inset-0 flex items-center justify-center pointer-events-none transition-opacity duration-75"
              style={{ opacity: onionOpacity / 100 }}
            >
              <img
                src={data.currentDataUrl!}
                alt="Current version"
                className="max-h-[300px] max-w-full object-contain"
                onLoad={(e) => setCurrentDims({ width: e.currentTarget.naturalWidth, height: e.currentTarget.naturalHeight })}
              />
            </div>
          </div>

          {/* Precision Opacity Slider Control */}
          <div className="flex items-center space-x-2 px-1">
            <span className="text-[10px] text-[#b91c1c] dark:text-[#ff7b72] font-semibold">{t('gitDiff.imageOld')}</span>
            <input
              type="range"
              min="0"
              max="100"
              value={onionOpacity}
              onChange={(e) => setOnionOpacity(parseInt(e.target.value, 10))}
              className="flex-1 h-1.5 bg-[#e5e7eb] dark:bg-[#21262d] rounded-lg appearance-none cursor-pointer accent-amber-500"
            />
            <span className="text-[10px] text-[#15803d] dark:text-[#7ee787] font-semibold">{t('gitDiff.imageNew')}</span>
          </div>
        </div>
      ) : (
        /* Single View Mode (New or Old) */
        <div className="p-2.5 flex flex-col space-y-1">
          {(() => {
            const isShowingNew = viewMode === 'new' || isAdded;
            const activeUrl = isShowingNew ? data.currentDataUrl : data.previousDataUrl;
            const activeBytes = isShowingNew ? data.currentBytes : data.previousBytes;
            const activeDims = isShowingNew ? currentDims : prevDims;

            if (!activeUrl) {
              return (
                <div className="p-4 text-center text-[#8f8f8f] dark:text-[#8b949e] text-xs">
                  {t('gitDiff.imageNoPreview')}
                </div>
              );
            }

            return (
              <>
                <div className="flex items-center justify-between text-[10px] px-0.5">
                  <span
                    className={'font-semibold ' +
                      (isShowingNew
                        ? 'text-[#15803d] dark:text-[#7ee787]'
                        : 'text-[#b91c1c] dark:text-[#ff7b72]')}
                  >
                    {isShowingNew ? t('gitDiff.imageNew') : t('gitDiff.imageOld')}
                  </span>
                  <div className="flex items-center space-x-1.5 text-[#8f8f8f] dark:text-[#8b949e] tabular-nums text-[9px]">
                    {activeBytes && <span>{formatBytes(activeBytes)}</span>}
                    {activeDims && <span>{activeDims.width}&times;{activeDims.height} px</span>}
                  </div>
                </div>
                <div
                  className={checkerboardClasses + ' cursor-zoom-in group min-h-[160px]'}
                  onClick={() =>
                    setLightbox({
                      url: activeUrl,
                      title: filePath + ' (' + (isShowingNew ? t('gitDiff.imageNew') : t('gitDiff.imageOld')) + ')',
                    })
                  }
                  title={t('gitDiff.imageClickToZoom')}
                >
                  <img
                    src={activeUrl}
                    alt={filePath}
                    className="max-h-[300px] max-w-full object-contain group-hover:scale-[1.02] transition-transform"
                    onLoad={(e) => {
                      const dims = {
                        width: e.currentTarget.naturalWidth,
                        height: e.currentTarget.naturalHeight,
                      };
                      if (isShowingNew) setCurrentDims(dims);
                      else setPrevDims(dims);
                    }}
                  />
                </div>
              </>
            );
          })()}
        </div>
      )}

      {/* Lightbox Modal (Hoisted via createPortal per AGENTS.md § 2.13) */}
      {lightbox &&
        createPortal(
          <div
            role="dialog"
            aria-modal="true"
            className="fixed inset-0 z-[100] bg-black/85 backdrop-blur-sm flex flex-col animate-in fade-in duration-150"
            onClick={() => {
              setLightbox(null);
              setLightboxZoom(false);
            }}
          >
            {/* Lightbox Toolbar */}
            <div
              className="flex items-center justify-between px-4 py-3 bg-[#0d1117]/95 border-b border-[#30363d] select-none text-white z-10 shrink-0"
              onClick={(e) => e.stopPropagation()}
            >
              <div className="flex items-center space-x-2.5 min-w-0 mr-4">
                <ImageIcon className="h-4 w-4 text-[#8b949e] shrink-0" />
                <span className="font-mono text-xs font-semibold truncate text-[#f0f6fc]">
                  {lightbox.title}
                </span>
              </div>

              <div className="flex items-center space-x-2 shrink-0">
                <button
                  type="button"
                  onClick={() => setLightboxZoom((prev) => !prev)}
                  className="flex items-center space-x-1 px-2.5 py-1 rounded bg-[#21262d] hover:bg-[#30363d] text-xs font-mono text-[#f0f6fc] transition cursor-pointer"
                >
                  {lightboxZoom ? (
                    <ZoomOut className="h-3.5 w-3.5" />
                  ) : (
                    <ZoomIn className="h-3.5 w-3.5" />
                  )}
                  <span>{lightboxZoom ? '100%' : 'Fit'}</span>
                </button>

                <button
                  type="button"
                  onClick={() => {
                    setLightbox(null);
                    setLightboxZoom(false);
                  }}
                  className="p-1 rounded bg-[#21262d] hover:bg-[#30363d] text-[#8b949e] hover:text-[#f0f6fc] transition cursor-pointer"
                  aria-label="Close"
                >
                  <X className="h-4 w-4" />
                </button>
              </div>
            </div>

            {/* Lightbox Image Stage with Checkerboard */}
            <div
              className="flex-1 overflow-auto p-4 flex items-center justify-center min-h-0"
              onClick={() => {
                setLightbox(null);
                setLightboxZoom(false);
              }}
            >
              <div
                className={checkerboardClasses + ' shadow-2xl max-w-full max-h-full cursor-default'}
                onClick={(e) => e.stopPropagation()}
              >
                <img
                  src={lightbox.url}
                  alt={lightbox.title}
                  className={'transition-all duration-150 ' +
                    (lightboxZoom
                      ? 'max-w-none'
                      : 'max-h-[82vh] max-w-[85vw] object-contain')}
                />
              </div>
            </div>
          </div>,
          document.body
        )}
    </div>
  );
}

export default ImageDiffViewer;
