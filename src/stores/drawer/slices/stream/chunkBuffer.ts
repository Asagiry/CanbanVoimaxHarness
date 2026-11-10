import { DrawerState } from '../../types';
import { LiveStreamEntry } from '../../../../types';

export type StreamChunkType = 'thought' | 'output';

export interface BufferedStreamChunk {
  taskId: string;
  streamId: string;
  type: StreamChunkType;
  content: string;
}

const streamChunkBuffer = new Map<string, BufferedStreamChunk>();
let streamFlushHandle: number | ReturnType<typeof setTimeout> | null = null;
let streamFlushIsRaf = false;
let applyBufferedChunks: ((chunks: BufferedStreamChunk[]) => void) | null = null;

export function setBufferedChunkApplier(applier: (chunks: BufferedStreamChunk[]) => void): void {
  applyBufferedChunks = applier;
}

function streamChunkKey(taskId: string, streamId: string, type: StreamChunkType): string {
  return `${taskId}\u0000${streamId}\u0000${type}`;
}

function cancelScheduledFlush(): void {
  if (streamFlushHandle === null) return;
  const cancelRaf = (globalThis as { cancelAnimationFrame?: typeof cancelAnimationFrame }).cancelAnimationFrame;
  if (streamFlushIsRaf && typeof cancelRaf === 'function') {
    cancelRaf.call(globalThis, streamFlushHandle as number);
  } else {
    clearTimeout(streamFlushHandle as ReturnType<typeof setTimeout>);
  }
  streamFlushHandle = null;
}

function scheduleStreamFlush(): void {
  if (streamFlushHandle !== null) return;
  const raf = (globalThis as { requestAnimationFrame?: typeof requestAnimationFrame }).requestAnimationFrame;
  if (typeof raf === 'function') {
    streamFlushIsRaf = true;
    streamFlushHandle = raf.call(globalThis, () => {
      streamFlushHandle = null;
      flushStreamChunks();
    });
  } else {
    streamFlushIsRaf = false;
    streamFlushHandle = setTimeout(() => {
      streamFlushHandle = null;
      flushStreamChunks();
    }, 50);
  }
}

export function enqueueStreamChunk(taskId: string, streamId: string, type: StreamChunkType, content: string): void {
  if (!content) return;
  const key = streamChunkKey(taskId, streamId, type);
  const existing = streamChunkBuffer.get(key);
  if (existing) existing.content += content;
  else streamChunkBuffer.set(key, { taskId, streamId, type, content });
  scheduleStreamFlush();
}

export function dropBufferedStreamChunks(taskId: string, streamId: string): void {
  for (const [key, chunk] of streamChunkBuffer) {
    if (chunk.taskId === taskId && chunk.streamId === streamId) streamChunkBuffer.delete(key);
  }
  if (streamChunkBuffer.size === 0) cancelScheduledFlush();
}

export function flushStreamChunks(taskId?: string): void {
  const pending: BufferedStreamChunk[] = [];
  for (const [key, chunk] of streamChunkBuffer) {
    if (taskId === undefined || chunk.taskId === taskId) {
      pending.push(chunk);
      streamChunkBuffer.delete(key);
    }
  }
  if (pending.length === 0) return;
  if (streamChunkBuffer.size === 0) cancelScheduledFlush();
  else if (streamFlushHandle === null) scheduleStreamFlush();
  if (applyBufferedChunks) applyBufferedChunks(pending);
  else {
    for (const chunk of pending) streamChunkBuffer.set(streamChunkKey(chunk.taskId, chunk.streamId, chunk.type), chunk);
    scheduleStreamFlush();
  }
}

export function clearStreamChunkBuffer(): void {
  streamChunkBuffer.clear();
  cancelScheduledFlush();
}

export function appendStreamChunk(
  state: DrawerState,
  taskId: string,
  streamId: string,
  type: StreamChunkType,
  content: string,
): Partial<DrawerState> {
  // Ignore chunks queued before an attempt was discarded or superseded.
  if (taskId in state.activeStreamIds && state.activeStreamIds[taskId] !== streamId) {
    console.warn('[appendStreamChunk] dropping chunk streamId mismatch', { taskId, streamId, activeStreamId: state.activeStreamIds[taskId] });
    return state;
  }
  const id = `${streamId}:${type}`;
  const existing = state.streamEntries[taskId] || { thoughts: [], outputs: [] };
  const key = type === 'thought' ? 'thoughts' : 'outputs';
  const entries = existing[key];
  const feed = state.activityFeed[taskId] || [];
  const entryIndex = entries.findIndex((entry) => entry.id === id);
  const feedIndex = feed.findIndex((item) => item.id === id);
  const previous = feed[feedIndex] || entries[entryIndex];
  if (type === 'thought' && !previous && !content.trim()) {
    return state;
  }
  const now = Date.now();
  const createdAt = previous?.createdAt ?? now;
  const entry: LiveStreamEntry = {
    id,
    type,
    timestamp: previous?.timestamp || new Date(now).toLocaleTimeString('en-US', { hour12: false }),
    createdAt,
    content: (previous?.content || '') + content,
    ...(type === 'thought' ? { durationMs: Math.max(0, now - createdAt) } : {}),
  };
  return {
    connectionWait: { ...state.connectionWait, [taskId]: null },
    streamEntries: {
      ...state.streamEntries,
      [taskId]: {
        ...existing,
        [key]: entryIndex < 0 ? [...entries, entry] : entries.map((item, i) => i === entryIndex ? entry : item),
      },
    },
    activityFeed: {
      ...state.activityFeed,
      [taskId]: feedIndex < 0 ? [...feed, entry] : feed.map((item, i) => i === feedIndex ? { ...item, ...entry } : item),
    },
  };
}
