import { useDrawerStore } from '../../stores/drawerStore';
import { flushStreamChunks } from '../../stores/drawer/slices/streamSlice';
import type { TextBuffer } from './types';

/**
 * Micro-batching window for streaming SSE tokens.
 * Coalesces consecutive chunks of the same type within 150ms into a single Zustand store mutation.
 */
export const TEXT_FLUSH_MS = 150;

/** Keeps channel/attempt ordering while batching frequent stream updates. */
export class TextBufferManager {
  private readonly buffers = new Map<string, TextBuffer>();

  public getBuffer(taskId: string): TextBuffer {
    let buffer = this.buffers.get(taskId);
    if (!buffer) {
      buffer = { chunks: [] };
      this.buffers.set(taskId, buffer);
    }
    return buffer;
  }

  public flushTask(taskId: string): void {
    const buffer = this.buffers.get(taskId);
    if (!buffer) return;
    if (buffer.timer) clearTimeout(buffer.timer);
    this.buffers.delete(taskId);
    for (const chunk of buffer.chunks) {
      const state = useDrawerStore.getState();
      if (chunk.type === 'thought') state.appendThought(taskId, chunk.text, chunk.streamId);
      else state.appendOutput(taskId, chunk.text, 'output', undefined, chunk.streamId);
    }
    flushStreamChunks(taskId);
  }

  public flushAll(): void {
    for (const taskId of Array.from(this.buffers.keys())) this.flushTask(taskId);
  }

  private queue(taskId: string, type: 'thought' | 'output', text: string, streamId?: string): void {
    const state = useDrawerStore.getState();
    if (streamId && taskId in state.activeStreamIds && state.activeStreamIds[taskId] !== streamId) {
      console.warn('[TextBufferManager] dropping chunk streamId mismatch', { taskId, streamId, activeStreamId: state.activeStreamIds[taskId] });
      return;
    }
    state.setActiveAgentTask(taskId, true);
    if (state.generationStartTime[taskId] == null) state.setGenerationStartTime(taskId, Date.now());
    state.recordStreamedChars(taskId, text.length);
    const buffer = this.getBuffer(taskId);
    const last = buffer.chunks[buffer.chunks.length - 1];
    if (last && last.type === type && last.streamId === streamId) last.text += text;
    else buffer.chunks.push({ type, text, streamId });
    if (!buffer.timer) buffer.timer = setTimeout(() => this.flushTask(taskId), TEXT_FLUSH_MS);
  }

  public queueThought(taskId: string, text: string, streamId?: string): void {
    this.queue(taskId, 'thought', text, streamId);
  }

  public queueOutput(taskId: string, text: string, streamId?: string): void {
    this.queue(taskId, 'output', text, streamId);
  }

  public startStream(taskId: string, streamId: string): void {
    this.flushTask(taskId);
    flushStreamChunks(taskId);
    useDrawerStore.getState().startStream(taskId, streamId);
  }

  public retryStream(taskId: string, streamId: string, delaySecs: number): void {
    const buffer = this.buffers.get(taskId);
    if (buffer) {
      buffer.chunks = buffer.chunks.filter((chunk) => chunk.streamId !== streamId);
      if (!buffer.chunks.length) this.clearTask(taskId);
    }
    useDrawerStore.getState().retryStream(taskId, streamId, delaySecs);
  }

  public clearTask(taskId: string): void {
    const buffer = this.buffers.get(taskId);
    if (buffer?.timer) clearTimeout(buffer.timer);
    this.buffers.delete(taskId);
    flushStreamChunks(taskId);
  }

  public dispose(): void {
    this.flushAll();
  }
}
