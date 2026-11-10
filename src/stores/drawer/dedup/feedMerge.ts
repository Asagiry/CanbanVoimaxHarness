import { DrawerActivityItem } from '../types';
import { capActivityFeed } from './windowing';

/**
 * Merges or appends the final assistant response into `activityFeed` without duplicating streamed output.
 */
export function mergeFinalResponseIntoFeed(
  feed: DrawerActivityItem[],
  response?: string,
  timeStr = new Date().toLocaleTimeString('en-US', { hour12: false })
): DrawerActivityItem[] {
  const updatedFeed = [...feed];

  let lastOutputIdx = -1;
  for (let i = updatedFeed.length - 1; i >= 0; i--) {
    if (updatedFeed[i].type === 'output' || (updatedFeed[i].type as string) === 'summary') {
      lastOutputIdx = i;
      break;
    }
  }

  const hasInterveningTools =
    lastOutputIdx !== -1 &&
    updatedFeed
      .slice(lastOutputIdx + 1)
      .some((it) => it.type === 'tool' || Boolean((it as { tool?: unknown }).tool));

  const resp = (response || '').trim();

  if (lastOutputIdx !== -1 && !hasInterveningTools) {
    const lastOutput = updatedFeed[lastOutputIdx];
    const content = (lastOutput.content || '').trim();
    if (resp && !content) {
      updatedFeed[lastOutputIdx] = {
        ...lastOutput,
        content: response!,
        isFinal: true,
      };
    } else {
      updatedFeed[lastOutputIdx] = {
        ...lastOutput,
        isFinal: true,
      };
    }
  } else if (resp.length > 0) {
    const alreadyExists = updatedFeed.some(
      (it) =>
        (it.type === 'output' || (it.type as string) === 'summary') &&
        (it.content || '').trim() === resp
    );
    if (!alreadyExists) {
      updatedFeed.push({
        id: `out-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`,
        type: 'output',
        timestamp: timeStr,
        content: response!,
        isFinal: true,
      });
    }
  }

  return capActivityFeed(updatedFeed);
}

/**
 * Appends or merges an activity item into `activityFeed` with duplicate suppression and rolling window cap.
 */
export function mergeActivityItemIntoFeed(
  feed: DrawerActivityItem[],
  item: DrawerActivityItem,
  isStreamedItem = false
): DrawerActivityItem[] {
  const idx = feed.findIndex((f) => f.id === item.id);
  let updatedFeed: DrawerActivityItem[];
  if (idx !== -1) {
    updatedFeed = [...feed];
    updatedFeed[idx] = { ...feed[idx], ...item };
  } else {
    const isHandoverSteer =
      item.type === 'steer' && item.content?.includes('[ОТЧЕТ ОБ ИСПРАВЛЕНИЯХ / HANDOVER SUMMARY]');
    let recentUserSteerIdx = -1;
    if ((item.type === 'user' || item.type === 'steer') && item.content?.trim()) {
      const targetText = item.content.trim();
      const start = Math.max(0, feed.length - 2);
      for (let i = feed.length - 1; i >= start; i--) {
        const candidate = feed[i];
        if (
          (candidate.type === 'user' || candidate.type === 'steer') &&
          candidate.content?.trim() === targetText &&
          candidate.id.startsWith('out-') !== item.id.startsWith('out-')
        ) {
          const sameTimestamp = Boolean(
            candidate.timestamp && item.timestamp && candidate.timestamp === item.timestamp
          );
          const timeDelta =
            candidate.createdAt && item.createdAt
              ? Math.abs(item.createdAt - candidate.createdAt)
              : candidate.createdAt
                ? Math.abs(Date.now() - candidate.createdAt)
                : 0;
          if (sameTimestamp || timeDelta <= 5000) {
            recentUserSteerIdx = i;
            break;
          }
        }
      }
    }
    const duplicateIdx =
      recentUserSteerIdx !== -1
        ? recentUserSteerIdx
        : isStreamedItem || item.type === 'user' || (item.type === 'steer' && !isHandoverSteer)
          ? -1
          : feed.findIndex(
              (f) =>
                !/:(thought|output)$/.test(f.id) &&
                f.type === item.type &&
                f.content &&
                item.content &&
                f.content.trim() === item.content.trim()
            );
    if (duplicateIdx !== -1) {
      updatedFeed = [...feed];
      updatedFeed[duplicateIdx] = { ...feed[duplicateIdx], ...item };
    } else {
      updatedFeed = [...feed, item];
    }
  }
  return capActivityFeed(updatedFeed);
}
