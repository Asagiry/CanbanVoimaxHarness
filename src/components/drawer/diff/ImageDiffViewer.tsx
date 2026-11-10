import { useState, useEffect, useRef } from 'react';
import {
  AlertCircle,
  Loader2,
  Sliders,
  Columns,
  Layers,
} from 'lucide-react';
import { GitImageDiff } from '../../../types';
import { useTranslation } from '../../../stores/languageStore';
import { formatBytes } from './utils';
import {
  ImageViewMode,
  ImageDimensions,
  LightboxState,
  ImageDiffTwoUp,
  ImageDiffSplit,
  ImageDiffOnion,
  ImageDiffSingle,
  ImageDiffLightbox,
} from './image';

export * from './image';

export interface ImageDiffViewerProps {
  taskId: string;
  filePath: string;
}

export function ImageDiffViewer({ taskId, filePath }: ImageDiffViewerProps) {
  const { t } = useTranslation();
  const [data, setData] = useState<GitImageDiff | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [viewMode, setViewMode] = useState<ImageViewMode>('2up');
  const [splitPercent, setSplitPercent] = useState(50);
  const [isDraggingSplit, setIsDraggingSplit] = useState(false);
  const [onionOpacity, setOnionOpacity] = useState(50);
  const [currentDims, setCurrentDims] = useState<ImageDimensions | null>(null);
  const [prevDims, setPrevDims] = useState<ImageDimensions | null>(null);
  const [lightbox, setLightbox] = useState<LightboxState | null>(null);
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
      <div className="border-t border-[#ebebeb] dark:border-[#212126] bg-white dark:bg-[#111114] p-5 flex flex-col items-center justify-center space-y-2 text-[#8f8f8f] dark:text-[#71717a]">
        <Loader2 className="h-4 w-4 animate-spin text-amber-500" />
        <span className="text-xs font-mono">{t('gitDiff.imageLoading')}</span>
      </div>
    );
  }

  if (error || !data) {
    return (
      <div className="border-t border-[#ebebeb] dark:border-[#212126] bg-white dark:bg-[#111114] p-4 flex flex-col items-center justify-center space-y-1.5 text-rose-500">
        <AlertCircle className="h-4 w-4" />
        <span className="text-xs font-mono text-center">{error || t('gitDiff.imageFailed')}</span>
      </div>
    );
  }

  const hasBoth = Boolean(data.currentDataUrl && data.previousDataUrl);
  const isDeleted = !data.currentDataUrl && Boolean(data.previousDataUrl);
  const isAdded = Boolean(data.currentDataUrl) && !data.previousDataUrl;
  const bytesDelta =
    data.currentBytes && data.previousBytes ? data.currentBytes - data.previousBytes : null;

  return (
    <div className="border-t border-[#ebebeb] dark:border-[#212126] bg-white dark:bg-[#111114] font-mono text-xs select-text overflow-hidden">
      {/* Subheader Toolbar: Mode Toggles & Size Badges */}
      <div className="flex items-center justify-between px-3 py-1.5 border-b border-[#ebebeb] dark:border-[#212126] bg-[#fcfcfc] dark:bg-[#17171b] text-[11px] gap-2 flex-wrap">
        {hasBoth ? (
          <div className="flex items-center border border-[#e5e7eb] dark:border-[#2c2c33] rounded p-0.5 bg-[#f4f4f5] dark:bg-[#111114] gap-0.5">
            <button
              type="button"
              onClick={() => setViewMode('2up')}
              className={'flex items-center gap-1 px-2 py-0.5 rounded text-[10px] font-semibold transition cursor-pointer ' +
                (viewMode === '2up'
                  ? 'bg-white dark:bg-[#232328] text-[#171717] dark:text-[#f4f4f5] shadow-xs'
                  : 'text-[#666666] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5]')}
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
                  ? 'bg-white dark:bg-[#232328] text-[#171717] dark:text-[#f4f4f5] shadow-xs'
                  : 'text-[#666666] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5]')}
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
                  ? 'bg-white dark:bg-[#232328] text-[#171717] dark:text-[#f4f4f5] shadow-xs'
                  : 'text-[#666666] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5]')}
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
                  ? 'bg-white dark:bg-[#232328] text-[#171717] dark:text-[#f4f4f5] shadow-xs'
                  : 'text-[#666666] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5]')}
            >
              {t('gitDiff.imageNew')}
            </button>
            <button
              type="button"
              onClick={() => setViewMode('old')}
              className={'px-2 py-0.5 rounded text-[10px] font-semibold transition cursor-pointer ' +
                (viewMode === 'old'
                  ? 'bg-white dark:bg-[#232328] text-[#171717] dark:text-[#f4f4f5] shadow-xs'
                  : 'text-[#666666] dark:text-[#71717a] hover:text-[#171717] dark:hover:text-[#f4f4f5]')}
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
          <span className="text-[#8f8f8f] dark:text-[#71717a] text-[10px]">
            {data.mimeType}
          </span>
        )}

        {/* Size Metrics & Delta */}
        <div className="flex items-center space-x-2 tabular-nums text-[10px] shrink-0">
          {data.previousBytes && !hasBoth && (
            <span className="text-[#8f8f8f] dark:text-[#71717a]">
              {formatBytes(data.previousBytes)}
            </span>
          )}
          {data.currentBytes && !hasBoth && (
            <span className="text-[#8f8f8f] dark:text-[#71717a]">
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
                  : 'text-[#8f8f8f] dark:text-[#71717a]')}
            >
              {bytesDelta > 0 ? ('+' + formatBytes(bytesDelta)) : bytesDelta < 0 ? ('-' + formatBytes(Math.abs(bytesDelta))) : '±0 B'}
            </span>
          )}
        </div>
      </div>

      {/* Main Diff Content Container */}
      {hasBoth && viewMode === '2up' ? (
        <ImageDiffTwoUp
          data={data}
          filePath={filePath}
          prevDims={prevDims}
          currentDims={currentDims}
          setPrevDims={setPrevDims}
          setCurrentDims={setCurrentDims}
          setLightbox={setLightbox}
        />
      ) : hasBoth && viewMode === 'split' ? (
        <ImageDiffSplit
          data={data}
          splitPercent={splitPercent}
          setSplitPercent={setSplitPercent}
          setIsDraggingSplit={setIsDraggingSplit}
          splitContainerRef={splitContainerRef}
          setPrevDims={setPrevDims}
          setCurrentDims={setCurrentDims}
        />
      ) : hasBoth && viewMode === 'onion' ? (
        <ImageDiffOnion
          data={data}
          onionOpacity={onionOpacity}
          setOnionOpacity={setOnionOpacity}
          setPrevDims={setPrevDims}
          setCurrentDims={setCurrentDims}
        />
      ) : (
        <ImageDiffSingle
          data={data}
          filePath={filePath}
          viewMode={viewMode}
          isAdded={isAdded}
          currentDims={currentDims}
          prevDims={prevDims}
          setCurrentDims={setCurrentDims}
          setPrevDims={setPrevDims}
          setLightbox={setLightbox}
        />
      )}

      {/* Lightbox Modal */}
      <ImageDiffLightbox lightbox={lightbox} onClose={() => setLightbox(null)} />
    </div>
  );
}

export default ImageDiffViewer;
