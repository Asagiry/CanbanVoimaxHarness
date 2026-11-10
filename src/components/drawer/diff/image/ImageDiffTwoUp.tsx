import { GitImageDiff } from '../../../../types';
import { useTranslation } from '../../../../stores/languageStore';
import { formatBytes } from '../utils';
import { CHECKERBOARD_CLASSES, ImageDimensions, LightboxState } from './types';

export interface ImageDiffTwoUpProps {
  data: GitImageDiff;
  filePath: string;
  prevDims: ImageDimensions | null;
  currentDims: ImageDimensions | null;
  setPrevDims: (dims: ImageDimensions) => void;
  setCurrentDims: (dims: ImageDimensions) => void;
  setLightbox: (state: LightboxState) => void;
}

export function ImageDiffTwoUp({
  data,
  filePath,
  prevDims,
  currentDims,
  setPrevDims,
  setCurrentDims,
  setLightbox,
}: ImageDiffTwoUpProps) {
  const { t } = useTranslation();

  return (
    <div className="grid grid-cols-2 gap-2 p-2.5">
      {/* Old / Previous */}
      <div className="flex flex-col space-y-1">
        <div className="flex items-center justify-between text-[10px] px-0.5">
          <span className="text-[#b91c1c] dark:text-[#ff7b72] font-semibold">
            {t('gitDiff.imageOld')}
          </span>
          <div className="flex items-center space-x-1.5 text-[#8f8f8f] dark:text-[#71717a] tabular-nums text-[9px]">
            {data.previousBytes && <span>{formatBytes(data.previousBytes)}</span>}
            {prevDims && <span>{prevDims.width}&times;{prevDims.height}</span>}
          </div>
        </div>
        <div
          className={CHECKERBOARD_CLASSES + ' cursor-zoom-in group min-h-[140px]'}
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
          <div className="flex items-center space-x-1.5 text-[#8f8f8f] dark:text-[#71717a] tabular-nums text-[9px]">
            {data.currentBytes && <span>{formatBytes(data.currentBytes)}</span>}
            {currentDims && <span>{currentDims.width}&times;{currentDims.height}</span>}
          </div>
        </div>
        <div
          className={CHECKERBOARD_CLASSES + ' cursor-zoom-in group min-h-[140px]'}
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
  );
}
