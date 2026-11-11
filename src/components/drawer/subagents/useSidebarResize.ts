import { useState, useRef, useCallback, useEffect } from 'react';

export const DEFAULT_SIDEBAR_WIDTH = 320;
export const MIN_SIDEBAR_WIDTH = 240;
export const MAX_SIDEBAR_WIDTH = 600;

export function useSidebarResize(storageKey = 'cvh_subagents_sidebar_width') {
  const [sidebarWidth, setSidebarWidth] = useState<number>(() => {
    try {
      const saved = localStorage.getItem(storageKey);
      if (saved) {
        const parsed = parseInt(saved, 10);
        if (!isNaN(parsed) && parsed >= MIN_SIDEBAR_WIDTH && parsed <= MAX_SIDEBAR_WIDTH) {
          return parsed;
        }
      }
    } catch (err) {
      console.debug('[useSidebarResize] Failed to read persisted width:', err);
    }
    return DEFAULT_SIDEBAR_WIDTH;
  });

  const [isResizing, setIsResizing] = useState(false);
  const isDraggingRef = useRef(false);
  const startXRef = useRef(0);
  const startWidthRef = useRef(sidebarWidth);
  const activeMouseMoveRef = useRef<((e: MouseEvent) => void) | null>(null);
  const activeMouseUpRef = useRef<(() => void) | null>(null);

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    isDraggingRef.current = true;
    startXRef.current = e.clientX;
    startWidthRef.current = sidebarWidth;
    setIsResizing(true);
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';

    const handleMouseMove = (moveEvent: MouseEvent) => {
      if (!isDraggingRef.current) return;
      const delta = moveEvent.clientX - startXRef.current;
      const maxAllowed = Math.min(MAX_SIDEBAR_WIDTH, window.innerWidth * 0.5);
      const newWidth = Math.min(Math.max(MIN_SIDEBAR_WIDTH, startWidthRef.current + delta), maxAllowed);
      setSidebarWidth(newWidth);
    };

    const handleMouseUp = () => {
      if (isDraggingRef.current) {
        isDraggingRef.current = false;
        setIsResizing(false);
        document.body.style.cursor = '';
        document.body.style.userSelect = '';
        setSidebarWidth((finalWidth) => {
          try {
            localStorage.setItem(storageKey, String(Math.round(finalWidth)));
          } catch (err) {
      console.debug('[useSidebarResize] Failed to read persisted width:', err);
    }
          return finalWidth;
        });
      }
      activeMouseMoveRef.current = null;
      activeMouseUpRef.current = null;
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };

    activeMouseMoveRef.current = handleMouseMove;
    activeMouseUpRef.current = handleMouseUp;
    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
  }, [sidebarWidth, storageKey]);

  useEffect(() => {
    return () => {
      if (activeMouseMoveRef.current) {
        window.removeEventListener('mousemove', activeMouseMoveRef.current);
      }
      if (activeMouseUpRef.current) {
        window.removeEventListener('mouseup', activeMouseUpRef.current);
      }
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    };
  }, []);

  return { sidebarWidth, isResizing, handleMouseDown };
}
