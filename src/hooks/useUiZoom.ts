import { useState, useEffect, useCallback, useMemo } from 'react';

const STORAGE_KEY = 'cvh_ui_zoom';
const MIN_ZOOM = 0.5; // 50%
const MAX_ZOOM = 2.0; // 200%
const STEP = 0.1;     // 10%

function clampZoom(val: number): number {
  const rounded = Math.round(val * 10) / 10;
  return Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, rounded));
}

function getInitialZoom(): number {
  if (typeof window === 'undefined') return 1.0;
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved) {
      const parsed = parseFloat(saved);
      if (!isNaN(parsed) && parsed >= MIN_ZOOM && parsed <= MAX_ZOOM) {
        return clampZoom(parsed);
      }
    }
  } catch {
    // Ignore localStorage access failures
  }
  return 1.0;
}

export function useUiZoom() {
  const [zoom, setZoomState] = useState<number>(getInitialZoom);
  const [showIndicator, setShowIndicator] = useState(false);

  const setZoom = useCallback((newVal: number | ((prev: number) => number)) => {
    setZoomState((prev) => {
      const target = typeof newVal === 'function' ? newVal(prev) : newVal;
      const clamped = clampZoom(target);
      try {
        localStorage.setItem(STORAGE_KEY, clamped.toString());
      } catch {
        // Ignore storage error
      }
      return clamped;
    });
    setShowIndicator(true);
  }, []);

  const zoomIn = useCallback(() => {
    setZoom((prev) => prev + STEP);
  }, [setZoom]);

  const zoomOut = useCallback(() => {
    setZoom((prev) => prev - STEP);
  }, [setZoom]);

  const resetZoom = useCallback(() => {
    setZoom(1.0);
  }, [setZoom]);

  // Hide toast indicator after 1.4s
  useEffect(() => {
    if (!showIndicator) return;
    const timer = setTimeout(() => setShowIndicator(false), 1400);
    return () => clearTimeout(timer);
  }, [showIndicator, zoom]);

  // Hotkeys & Ctrl+Wheel listeners
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (!e.ctrlKey && !e.metaKey) return;

      // Ctrl + / Ctrl = (Zoom In)
      if (e.key === '=' || e.key === '+' || e.code === 'NumpadAdd' || e.key === 'Add') {
        e.preventDefault();
        zoomIn();
        return;
      }

      // Ctrl - (Zoom Out)
      if (e.key === '-' || e.key === '_' || e.code === 'NumpadSubtract' || e.key === 'Subtract') {
        e.preventDefault();
        zoomOut();
        return;
      }

      // Ctrl 0 (Reset)
      if (e.key === '0' || e.code === 'Numpad0' || e.code === 'Digit0') {
        e.preventDefault();
        resetZoom();
        return;
      }
    };

    const handleWheel = (e: WheelEvent) => {
      if (e.ctrlKey || e.metaKey) {
        e.preventDefault();
        if (e.deltaY < 0) {
          zoomIn();
        } else if (e.deltaY > 0) {
          zoomOut();
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    window.addEventListener('wheel', handleWheel, { passive: false });

    return () => {
      window.removeEventListener('keydown', handleKeyDown);
      window.removeEventListener('wheel', handleWheel);
    };
  }, [zoomIn, zoomOut, resetZoom]);

  const isZoomSupported = typeof document !== 'undefined' && 'zoom' in document.documentElement.style;

  const rootStyle = useMemo<React.CSSProperties>(() => {
    if (isZoomSupported) {
      return {
        zoom,
        width: `${100 / zoom}vw`,
        height: `${100 / zoom}vh`,
      };
    }
    return {
      transform: `scale(${zoom})`,
      transformOrigin: 'top left',
      width: `${100 / zoom}vw`,
      height: `${100 / zoom}vh`,
    };
  }, [zoom, isZoomSupported]);

  return {
    zoom,
    zoomPercent: Math.round(zoom * 100),
    zoomIn,
    zoomOut,
    resetZoom,
    setZoom,
    rootStyle,
    showIndicator,
  };
}
