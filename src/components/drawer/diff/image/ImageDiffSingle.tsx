import { GitImageDiff } from '../../../../types';
import { useTranslation } from '../../../../stores/languageStore';
import { formatBytes } from '../utils';
import { CHECKERBOARD_CLASSES, ImageDimensions, LightboxState, ImageViewMode } from './types';

export interface ImageDiffSingleProps {
  data: GitImageDiff;
  filePath: string;
  viewMode: ImageViewMode;
  isAdded: boolean;
  currentDims: ImageDimensions | null;
  prevDims: ImageDimensions | null;
  setCurrentDims: (dims: ImageDimensions) => void;
  setPrevDims: (dims: ImageDimensions) => void;
  setLightbox: (state: LightboxState) => void;
}

export function ImageDiffSingle({
  data,
  filePath,
  viewMode,
  isAdded,
  currentDims,
  prevDims,
  setCurrentDims,
  setPrevDims,
  setLightbox,
}: ImageDiffSingleProps) {
  const { t } = useTranslation();

  const isShowingNew = viewMode === 'new' || isAdded;
  const activeUrl = isShowingNew ? data.currentDataUrl : data.previousDataUrl;
  const activeBytes = isShowingNew ? data.currentBytes : data.previousBytes;
  const activeDims = isShowingNew ? currentDims : prevDims;

  if (!activeUrl) {
    return (
      <div className="p-4 text-center text-[#8f8f8f] dark:text-[#71717a] text-xs">
        {t('gitDiff.imageNoPreview')}
      </div>
    );
  }

  return (
    <div className="p-2.5 flex flex-col space-y-1">
      <div className="flex items-center justify-between text-[10px] px-0.5">
        <span
          className={'font-semibold ' +
            (isShowingNew
              ? 'text-[#15803d] dark:text-[#7ee787]'
              : 'text-[#b91c1c] dark:text-[#ff7b72]')}
        >
          {isShowingNew ? t('gitDiff.imageNew') : t('gitDiff.imageOld')}
        </span>
        <div className="flex items-center space-x-1.5 text-[#8f8f8f] dark:text-[#71717a] tabular-nums text-[9px]">
          {activeBytes && <span>{formatBytes(activeBytes)}</span>}
          {activeDims && <span>{activeDims.width}&times;{activeDims.height} px</span>}
        </div>
      </div>
      <div
        className={CHECKERBOARD_CLASSES + ' cursor-zoom-in group min-h-[160px]'}
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
    </div>
  );
}
