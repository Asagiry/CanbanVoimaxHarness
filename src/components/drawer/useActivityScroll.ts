import { useRef, useState, useEffect } from 'react';
import type { DrawerActivityItem } from '../../stores/drawerStore';

export const OLD_ACTIVITY_COLLAPSE_THRESHOLD = 10;

export function isOldActivity(idx: number, total: number): boolean {
  return idx < total - OLD_ACTIVITY_COLLAPSE_THRESHOLD;
}

export function useActivityScroll(displayActivities: DrawerActivityItem[], taskId?: string) {
  const [collapsedOutputs, setCollapsedOutputs] = useState<Set<string>>(() => new Set());
  const [expandedOverrides, setExpandedOverrides] = useState<Set<string>>(() => new Set());
  const [showNewButton, setShowNewButton] = useState(false);
  const isFirstBatchRef = useRef(true);
  const nearBottomRef = useRef(true);
  const scrollRef = useRef<HTMLDivElement>(null);

  const toggleOutput = (id: string) => {
    setCollapsedOutputs((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  };

  const isCollapsed = (id: string, idx: number, total: number): boolean => {
    if (expandedOverrides.has(id)) return false;
    if (collapsedOutputs.has(id)) return true;
    return isOldActivity(idx, total);
  };

  const toggleCollapsed = (id: string, idx: number, total: number) => {
    const currentlyCollapsed = isCollapsed(id, idx, total);
    if (currentlyCollapsed) {
      setExpandedOverrides((prev) => new Set(prev).add(id));
      setCollapsedOutputs((prev) => {
        const next = new Set(prev);
        next.delete(id);
        return next;
      });
    } else {
      setExpandedOverrides((prev) => {
        const next = new Set(prev);
        next.delete(id);
        return next;
      });
      setCollapsedOutputs((prev) => new Set(prev).add(id));
    }
  };

  const scrollToBottom = () => {
    const el = scrollRef.current;
    if (el) {
      el.scrollTop = el.scrollHeight;
    }
    nearBottomRef.current = true;
    setShowNewButton(false);
  };

  const handleFeedScroll = () => {
    const el = scrollRef.current;
    if (!el) return;
    const nearBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 120;
    nearBottomRef.current = nearBottom;
    if (nearBottom) {
      setShowNewButton(false);
    }
  };

  // Reset scroll and collapsed state on task switch
  useEffect(() => {
    setCollapsedOutputs(new Set());
    setExpandedOverrides(new Set());
    setShowNewButton(false);
    isFirstBatchRef.current = true;
    nearBottomRef.current = true;
  }, [taskId]);

  // Smart auto-scroll: down only if user is already near bottom or initial batch
  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;
    if (isFirstBatchRef.current) {
      isFirstBatchRef.current = false;
      el.scrollTop = el.scrollHeight;
      nearBottomRef.current = true;
      setShowNewButton(false);
      return;
    }
    if (nearBottomRef.current) {
      el.scrollTop = el.scrollHeight;
      setShowNewButton(false);
    } else {
      setShowNewButton(true);
    }
  }, [displayActivities]);

  // Auto-scroll down during thinking/token streaming (with user scroll-up protection)
  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;
    const contentEl = el.firstElementChild as HTMLElement;
    if (!contentEl) return;
    const ro = new ResizeObserver(() => {
      if (nearBottomRef.current) {
        el.scrollTop = el.scrollHeight;
      }
    });
    ro.observe(contentEl);
    return () => ro.disconnect();
  }, []);

  return {
    scrollRef,
    collapsedOutputs,
    expandedOverrides,
    showNewButton,
    toggleOutput,
    isCollapsed,
    toggleCollapsed,
    scrollToBottom,
    handleFeedScroll,
  };
}
