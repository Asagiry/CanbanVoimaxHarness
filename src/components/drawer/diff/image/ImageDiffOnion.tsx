import { GitImageDiff } from '../../../../types';
import { useTranslation } from '../../../../stores/languageStore';
import { CHECKERBOARD_CLASSES, ImageDimensions } from './types';

export interface ImageDiffOnionProps {
  data: GitImageDiff;
  onionOpacity: number;
  setOnionOpacity: (val: number) => void;
  setPrevDims: (dims: ImageDimensions) => void;
  setCurrentDims: (dims: ImageDimensions) => void;
}

export function ImageDiffOnion({
  data,
  onionOpacity,
  setOnionOpacity,
  setPrevDims,
  setCurrentDims,
}: ImageDiffOnionProps) {
  const { t } = useTranslation();

  return (
    <div className="p-2.5 flex flex-col space-y-2">
      <div className="flex items-center justify-between text-[10px] px-0.5">
        <span className="text-[#8f8f8f] dark:text-[#71717a]">
          {t('gitDiff.imageOld')}: {100 - onionOpacity}%
        </span>
        <span className="text-amber-600 dark:text-amber-400 font-semibold tabular-nums">
          {onionOpacity}% {t('gitDiff.imageNew')}
        </span>
      </div>

      {/* Onion Stage: Stacked Images */}
      <div className={CHECKERBOARD_CLASSES + ' min-h-[180px] max-h-[340px] relative overflow-hidden select-none'}>
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
          className="flex-1 h-1.5 bg-[#e5e7eb] dark:bg-[#232328] rounded-lg appearance-none cursor-pointer accent-amber-500"
        />
        <span className="text-[10px] text-[#15803d] dark:text-[#7ee787] font-semibold">{t('gitDiff.imageNew')}</span>
      </div>
    </div>
  );
}
