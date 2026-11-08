import { useState, useLayoutEffect, useEffect, type RefObject } from 'react';

export interface UseContextMenuPositionOptions {
  x: number;
  y: number;
  menuRef: RefObject<HTMLElement | null>;
  onClose: () => void;
  defaultWidth?: number;
  viewportMargin?: number;
}

export function useContextMenuPosition({
  x,
  y,
  menuRef,
  onClose,
  defaultWidth = 220,
  viewportMargin = 8,
}: UseContextMenuPositionOptions) {
  const [pos, setPos] = useState({ left: x, top: y });

  useLayoutEffect(() => {
    const el = menuRef.current;
    if (!el) {
      setPos({
        left: Math.min(x, Math.max(viewportMargin, window.innerWidth - defaultWidth - viewportMargin)),
        top: Math.min(y, Math.max(viewportMargin, window.innerHeight - viewportMargin)),
      });
      return;
    }
    const rect = el.getBoundingClientRect();
    let left = x;
    let top = y;
    if (left + rect.width > window.innerWidth - viewportMargin) {
      left = Math.max(viewportMargin, window.innerWidth - rect.width - viewportMargin);
    }
    if (top + rect.height > window.innerHeight - viewportMargin) {
      top = Math.max(viewportMargin, window.innerHeight - rect.height - viewportMargin);
    }
    if (left !== x || top !== y) {
      setPos({ left, top });
    }
  }, [x, y, defaultWidth, viewportMargin, menuRef]);

  useEffect(() => {
    const handlePointerDown = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose();
      }
    };
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      }
    };
    const handleScroll = () => onClose();
    const handleResize = () => onClose();
    const handleContextMenu = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose();
      }
    };

    document.addEventListener('mousedown', handlePointerDown);
    document.addEventListener('keydown', handleKeyDown);
    document.addEventListener('contextmenu', handleContextMenu);
    window.addEventListener('scroll', handleScroll, true);
    window.addEventListener('resize', handleResize);

    return () => {
      document.removeEventListener('mousedown', handlePointerDown);
      document.removeEventListener('keydown', handleKeyDown);
      document.removeEventListener('contextmenu', handleContextMenu);
      window.removeEventListener('scroll', handleScroll, true);
      window.removeEventListener('resize', handleResize);
    };
  }, [onClose, menuRef]);

  return pos;
}
