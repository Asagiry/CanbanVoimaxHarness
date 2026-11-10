import { StateCreator } from 'zustand';
import {
  StreamSlice,
  DrawerState,
  DrawerActivityItem,
  ActivityType,
} from '../../types';
import { LiveStreamEntry } from '../../../../types';
import { enqueueStreamChunk } from './chunkBuffer';

type DrawerSet = Parameters<StateCreator<DrawerState, [], [], StreamSlice>>[0];

export function createStreamAppendActions(
  set: DrawerSet,
): Pick<StreamSlice, 'appendThought' | 'appendOutput'> {
  return {
    appendThought: (taskId, content, streamId) => {
      if (streamId) {
        enqueueStreamChunk(taskId, streamId, 'thought', content);
        return;
      }
      const timeStr = new Date().toLocaleTimeString('en-US', { hour12: false });
      const now = Date.now();

      set((state) => {
        const existing = state.streamEntries[taskId] || { thoughts: [], outputs: [] };
        const last = existing.thoughts[existing.thoughts.length - 1];

        const existingFeed = state.activityFeed[taskId] || [];
        const lastFeedItem = existingFeed[existingFeed.length - 1];

        // Detect if this is a fresh thought block:
        // (1) No feed items yet, or
        // (2) Last feed item is not a thought (e.g. output or tool intervened), or
        // (3) Coalescing window has passed (> 3000ms idle silence since last thought chunk)
        const lastChunkTime = (lastFeedItem?.createdAt || now) + (lastFeedItem?.durationMs ?? 0);
        const isNewBlock =
          !lastFeedItem ||
          lastFeedItem.type !== 'thought' ||
          (now - lastChunkTime > 3000);

        const blockStart = isNewBlock
          ? now
          : (lastFeedItem.createdAt || state.thinkingStartTime[taskId] || now);

        const durationMs = Math.max(0, now - blockStart);

        // Stream coalescing for thoughts (< 3s window and last feed item is thought)
        if (!isNewBlock && lastFeedItem && lastFeedItem.type === 'thought') {
          const updatedThoughts = [...existing.thoughts];
          if (last) {
            updatedThoughts[updatedThoughts.length - 1] = {
              ...last,
              content: last.content + content,
              durationMs,
              createdAt: last.createdAt || blockStart,
            };
          } else {
            updatedThoughts.push({
              id: lastFeedItem.id,
              timestamp: lastFeedItem.timestamp,
              type: 'thought',
              content: (lastFeedItem.content || '') + content,
              durationMs,
              createdAt: blockStart,
            });
          }

          const updatedFeed = [...existingFeed];
          updatedFeed[updatedFeed.length - 1] = {
            ...lastFeedItem,
            content: (lastFeedItem.content || '') + content,
            durationMs,
            createdAt: lastFeedItem.createdAt || blockStart,
          };

          return {
            thinkingStartTime: {
              ...state.thinkingStartTime,
              [taskId]: blockStart,
            },
            streamEntries: {
              ...state.streamEntries,
              [taskId]: {
                ...existing,
                thoughts: updatedThoughts,
              },
            },
            activityFeed: {
              ...state.activityFeed,
              [taskId]: updatedFeed,
            },
          };
        }

        // Fresh thought block: ignore whitespace-only openings (e.g. <think>\n\n</think>)
        if (!content.trim()) {
          return {};
        }
        const newEntry: LiveStreamEntry = {
          id: `th-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`,
          timestamp: timeStr,
          type: 'thought',
          content,
          durationMs,
          createdAt: blockStart,
        };

        const newFeedItem: DrawerActivityItem = {
          id: newEntry.id,
          type: 'thought',
          timestamp: timeStr,
          content,
          durationMs,
          createdAt: blockStart,
        };

        return {
          thinkingStartTime: {
            ...state.thinkingStartTime,
            [taskId]: blockStart,
          },
          streamEntries: {
            ...state.streamEntries,
            [taskId]: {
              ...existing,
              thoughts: [...existing.thoughts, newEntry],
            },
          },
          activityFeed: {
            ...state.activityFeed,
            [taskId]: [...existingFeed, newFeedItem],
          },
        };
      });
    },

    appendOutput: (taskId, content, type = 'output', images, streamId) => {
      if (streamId && type === 'output') {
        enqueueStreamChunk(taskId, streamId, 'output', content);
        return;
      }
      const timeStr = new Date().toLocaleTimeString('en-US', { hour12: false });

      set((state) => {
        const existing = state.streamEntries[taskId] || { thoughts: [], outputs: [] };
        const last = existing.outputs[existing.outputs.length - 1];

        const existingFeed = state.activityFeed[taskId] || [];
        const lastFeedItem = existingFeed[existingFeed.length - 1];

        const activityType: ActivityType =
          type === 'steer' ? 'steer' : type === 'system' ? 'system' : type === 'error' ? 'error' : type === 'user' ? 'user' : 'output';

        // User submissions have their own identity, even when their text repeats.
        if (!images?.length && last && last.type === type && type === 'output') {
          const updated = [...existing.outputs];
          updated[updated.length - 1] = {
            ...last,
            content: last.content + content,
          };

          const updatedFeed = [...existingFeed];
          if (lastFeedItem && lastFeedItem.type === activityType) {
            updatedFeed[updatedFeed.length - 1] = {
              ...lastFeedItem,
              content: (lastFeedItem.content || '') + content,
            };
          } else {
            updatedFeed.push({
              id: `out-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`,
              type: activityType,
              timestamp: timeStr,
              content,
            });
          }

          return {
            streamEntries: {
              ...state.streamEntries,
              [taskId]: {
                ...existing,
                outputs: updated,
              },
            },
            activityFeed: {
              ...state.activityFeed,
              [taskId]: updatedFeed,
            },
          };
        }

        const newEntry: LiveStreamEntry = {
          id: `out-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`,
          timestamp: timeStr,
          type,
          content,
          images,
        };

        const newFeedItem: DrawerActivityItem = {
          id: newEntry.id,
          type: activityType,
          timestamp: timeStr,
          content,
          images,
        };

        return {
          streamEntries: {
            ...state.streamEntries,
            [taskId]: {
              ...existing,
              outputs: [...existing.outputs, newEntry],
            },
          },
          activityFeed: {
            ...state.activityFeed,
            [taskId]: [...existingFeed, newFeedItem],
          },
        };
      });
    },
  };
}
