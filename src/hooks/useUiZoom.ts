import { useMemo } from 'react';
import { create } from 'zustand';

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

let indicatorTimer: ReturnType<typeof setTimeout> | null = null;

export interface UiZoomState {
  zoom: number;
  showIndicator: boolean;
  setZoom: (newVal: number | ((prev: number) => number)) => void;
  zoomIn: () => void;
  zoomOut: () => void;
  resetZoom: () => void;
}

export const useUiZoomStore = create<UiZoomState>((set, get) => ({
  zoom: getInitialZoom(),
  showIndicator: false,
  setZoom: (newVal) => {
    const current = get().zoom;
    const target = typeof newVal === 'function' ? newVal(current) : newVal;
    const clamped = clampZoom(target);
    try {
      localStorage.setItem(STORAGE_KEY, clamped.toString());
    } catch {
      // Ignore storage error
    }
    if (indicatorTimer) {
      clearTimeout(indicatorTimer);
    }
    indicatorTimer = setTimeout(() => {
      set({ showIndicator: false });
    }, 1400);

    set({ zoom: clamped, showIndicator: true });
  },
  zoomIn: () => {
    get().setZoom((prev) => prev + STEP);
  },
  zoomOut: () => {
    get().setZoom((prev) => prev - STEP);
  },
  resetZoom: () => {
    get().setZoom(1.0);
  },
}));

let isListenerAttached = false;

export function attachGlobalZoomListeners(): void {
  if (typeof window === 'undefined' || isListenerAttached) return;
  isListenerAttached = true;

  const handleKeyDown = (e: KeyboardEvent) => {
    if (!e.ctrlKey && !e.metaKey) return;

    // Ctrl + / Ctrl = (Zoom In)
    if (e.key === '=' || e.key === '+' || e.code === 'NumpadAdd' || e.key === 'Add') {
      e.preventDefault();
      useUiZoomStore.getState().zoomIn();
      return;
    }

    // Ctrl - (Zoom Out)
    if (e.key === '-' || e.key === '_' || e.code === 'NumpadSubtract' || e.key === 'Subtract') {
      e.preventDefault();
      useUiZoomStore.getState().zoomOut();
      return;
    }

    // Ctrl 0 (Reset)
    if (e.key === '0' || e.code === 'Numpad0' || e.code === 'Digit0') {
      e.preventDefault();
      useUiZoomStore.getState().resetZoom();
      return;
    }
  };

  const handleWheel = (e: WheelEvent) => {
    if (e.ctrlKey || e.metaKey) {
      e.preventDefault();
      if (e.deltaY < 0) {
        useUiZoomStore.getState().zoomIn();
      } else if (e.deltaY > 0) {
        useUiZoomStore.getState().zoomOut();
      }
    }
  };

  window.addEventListener('keydown', handleKeyDown);
  window.addEventListener('wheel', handleWheel, { passive: false });
}

// Auto-attach listeners in browser environment
if (typeof window !== 'undefined') {
  attachGlobalZoomListeners();
}

const isZoomSupported = typeof document !== 'undefined' && 'zoom' in document.documentElement.style;

export function getRootZoomStyle(zoom: number): React.CSSProperties {
  if (isZoomSupported) {
    return {
      zoom,
      width: `${100 / zoom}vw`,
      height: `${100 / zoom}vh`,
      maxWidth: 'none',
      maxHeight: 'none',
    };
  }
  return {
    transform: `scale(${zoom})`,
    transformOrigin: 'top left',
    width: `${100 / zoom}vw`,
    height: `${100 / zoom}vh`,
    maxWidth: 'none',
    maxHeight: 'none',
  };
}

export function getZoomOverlayStyle(zoom: number): React.CSSProperties | undefined {
  if (zoom === 1 || Math.abs(zoom - 1) < 0.001) {
    return undefined;
  }
  if (isZoomSupported) {
    return {
      zoom,
      width: `${100 / zoom}vw`,
      height: `${100 / zoom}vh`,
      maxWidth: 'none',
      maxHeight: 'none',
    };
  }
  return {
    transform: `scale(${zoom})`,
    transformOrigin: 'top left',
    width: `${100 / zoom}vw`,
    height: `${100 / zoom}vh`,
    maxWidth: 'none',
    maxHeight: 'none',
  };
}

export function adjustDndTransformForZoom(transform: string | undefined | null, zoom: number): string | undefined {
  if (!transform || zoom === 1 || Math.abs(zoom - 1) < 0.001) {
    return transform ?? undefined;
  }
  return transform.replace(/translate\(\s*(-?[\d.]+)px\s*,\s*(-?[\d.]+)px\s*\)/, (_match, x, y) => {
    const adjustedX = parseFloat(x) / zoom;
    const adjustedY = parseFloat(y) / zoom;
    return `translate(${adjustedX}px, ${adjustedY}px)`;
  });
}

export function useUiZoom() {
  attachGlobalZoomListeners();

  const zoom = useUiZoomStore((s) => s.zoom);
  const showIndicator = useUiZoomStore((s) => s.showIndicator);
  const setZoom = useUiZoomStore((s) => s.setZoom);
  const zoomIn = useUiZoomStore((s) => s.zoomIn);
  const zoomOut = useUiZoomStore((s) => s.zoomOut);
  const resetZoom = useUiZoomStore((s) => s.resetZoom);

  const rootStyle = useMemo<React.CSSProperties>(() => {
    return getRootZoomStyle(zoom);
  }, [zoom]);

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
