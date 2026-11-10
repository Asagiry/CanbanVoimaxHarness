import { useState, useEffect } from 'react';
import { createPortal } from 'react-dom';
import { Image as ImageIcon, ZoomIn, ZoomOut, X } from 'lucide-react';
import { useUiZoom, getZoomOverlayStyle } from '../../../../hooks/useUiZoom';
import { CHECKERBOARD_CLASSES, LightboxState } from './types';

export interface ImageDiffLightboxProps {
  lightbox: LightboxState | null;
  onClose: () => void;
}

export function ImageDiffLightbox({ lightbox, onClose }: ImageDiffLightboxProps) {
  const { zoom } = useUiZoom();
  const [lightboxZoom, setLightboxZoom] = useState(false);

  useEffect(() => {
    if (!lightbox) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
        setLightboxZoom(false);
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [lightbox, onClose]);

  if (!lightbox) return null;

  return createPortal(
    <div
      role="dialog"
      aria-modal="true"
      style={getZoomOverlayStyle(zoom)}
      className="fixed inset-0 z-[100] bg-black/85 backdrop-blur-sm flex flex-col animate-in fade-in duration-150"
      onClick={() => {
        onClose();
        setLightboxZoom(false);
      }}
    >
      {/* Lightbox Toolbar */}
      <div
        className="flex items-center justify-between px-4 py-3 bg-[#111114]/95 border-b border-[#2c2c33] select-none text-white z-10 shrink-0"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center space-x-2.5 min-w-0 mr-4">
          <ImageIcon className="h-4 w-4 text-[#71717a] shrink-0" />
          <span className="font-mono text-xs font-semibold truncate text-[#f4f4f5]">
            {lightbox.title}
          </span>
        </div>

        <div className="flex items-center space-x-2 shrink-0">
          <button
            type="button"
            onClick={() => setLightboxZoom((prev) => !prev)}
            className="flex items-center space-x-1 px-2.5 py-1 rounded bg-[#232328] hover:bg-[#2c2c33] text-xs font-mono text-[#f4f4f5] transition cursor-pointer"
          >
            {lightboxZoom ? <ZoomOut className="h-3.5 w-3.5" /> : <ZoomIn className="h-3.5 w-3.5" />}
            <span>{lightboxZoom ? '100%' : 'Fit'}</span>
          </button>

          <button
            type="button"
            onClick={() => {
              onClose();
              setLightboxZoom(false);
            }}
            className="p-1 rounded bg-[#232328] hover:bg-[#2c2c33] text-[#71717a] hover:text-[#f4f4f5] transition cursor-pointer"
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
          onClose();
          setLightboxZoom(false);
        }}
      >
        <div
          className={CHECKERBOARD_CLASSES + ' shadow-2xl max-w-full max-h-full cursor-default'}
          onClick={(e) => e.stopPropagation()}
        >
          <img
            src={lightbox.url}
            alt={lightbox.title}
            className={'transition-all duration-150 ' +
              (lightboxZoom ? 'max-w-none' : 'max-h-[82vh] max-w-[85vw] object-contain')}
          />
        </div>
      </div>
    </div>,
    document.body
  );
}
