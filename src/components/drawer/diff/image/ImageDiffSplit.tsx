import React from 'react';
import { GitImageDiff } from '../../../../types';
import { useTranslation } from '../../../../stores/languageStore';
import { CHECKERBOARD_CLASSES, ImageDimensions } from './types';

export interface ImageDiffSplitProps {
  data: GitImageDiff;
  splitPercent: number;
  setSplitPercent: (val: number) => void;
  setIsDraggingSplit: (dragging: boolean) => void;
  splitContainerRef: React.RefObject<HTMLDivElement | null>;
  setPrevDims: (dims: ImageDimensions) => void;
  setCurrentDims: (dims: ImageDimensions) => void;
}

export function ImageDiffSplit({
  data,
  splitPercent,
  setSplitPercent,
  setIsDraggingSplit,
  splitContainerRef,
  setPrevDims,
  setCurrentDims,
}: ImageDiffSplitProps) {
  const { t } = useTranslation();

  return (
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
        className={CHECKERBOARD_CLASSES + ' min-h-[180px] max-h-[340px] relative overflow-hidden select-none cursor-ew-resize group'}
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
          className="flex-1 h-1.5 bg-[#e5e7eb] dark:bg-[#232328] rounded-lg appearance-none cursor-pointer accent-amber-500"
        />
        <span className="text-[10px] text-[#15803d] dark:text-[#7ee787] font-semibold">{t('gitDiff.imageNew')}</span>
      </div>
    </div>
  );
}
