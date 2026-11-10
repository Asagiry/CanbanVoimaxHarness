import type { ReasoningEffort } from '../../../types';
import { normalizeReasoningEffort } from '../../../types';
import type { TranslationKey } from '../../../i18n/types';
import {
  catalogModelOptions,
  type CatalogModelOption,
  type Provider,
} from '../../../stores/settings/providers';

export type { CatalogModelOption, Provider };

export const REASONING_EFFORT_LABELS: Record<ReasoningEffort, TranslationKey> = {
  off: 'ruleEditor.reasoningOff',
  low: 'ruleEditor.reasoningLow',
  medium: 'ruleEditor.reasoningMedium',
  high: 'ruleEditor.reasoningHigh',
};

export interface ParsedSetting {
  model: string | null;
  reasoningEffort: ReasoningEffort;
  contextWindow: number;
  maxTokens: number;
  hasAllowedTools: boolean;
  allowedToolsKey?: string;
  allowedTools?: unknown;
  icon?: string;
  color?: string;
  rawExtra: Record<string, unknown>;
}

export const CONTEXT_CHIPS = [
  { value: 128000, label: '128K' },
  { value: 256000, label: '256K' },
  { value: 384000, label: '384K' },
];

export const MAX_TOKENS_CHIPS = [
  { value: 4096, label: '4K' },
  { value: 8192, label: '8K' },
  { value: 16384, label: '16K' },
  { value: 65536, label: '64K' },
];

export function buildDynamicModelOptions(
  providers: Provider[],
  settingsModel?: string | null,
): CatalogModelOption[] {
  const catalog = catalogModelOptions(providers);
  const seen = new Set(catalog.map((o) => o.id));
  const extra: CatalogModelOption[] = [];
  if (settingsModel && typeof settingsModel === 'string') {
    const trimmed = settingsModel.trim();
    if (trimmed && trimmed !== 'inherit' && !seen.has(trimmed)) {
      extra.push({ id: trimmed, label: trimmed });
    }
  }
  const sorted = [...catalog].sort((a, b) => {
    const byProvider = (a.providerName ?? '').localeCompare(b.providerName ?? '', undefined, {
      sensitivity: 'base',
    });
    if (byProvider !== 0) return byProvider;
    return a.label.localeCompare(b.label, undefined, { sensitivity: 'base' });
  });
  return [{ id: 'inherit', label: 'inherit' }, ...sorted, ...extra];
}

export function parseSettingJson(raw: string): { parsed: ParsedSetting; error: string | null } {
  try {
    const trimmed = raw.trim();
    const obj = trimmed ? JSON.parse(trimmed) : {};
    if (typeof obj !== 'object' || obj === null || Array.isArray(obj)) {
      throw new Error('Ожидался JSON объект {...}');
    }

    const rawModel = obj.model;
    const model =
      rawModel === undefined || rawModel === null || rawModel === '' || rawModel === 'inherit'
        ? 'inherit'
        : String(rawModel);

    const reasoningEffort: ReasoningEffort =
      normalizeReasoningEffort(obj.reasoningEffort ?? obj.reasoning_effort) ?? 'off';

    const rawCw = obj.contextWindow ?? obj.context_window;
    const contextWindow =
      typeof rawCw === 'number' && !isNaN(rawCw) && rawCw > 0 ? Math.round(rawCw) : 200000;

    const rawMt = obj.maxTokens ?? obj.max_tokens;
    const maxTokens =
      typeof rawMt === 'number' && !isNaN(rawMt) && rawMt > 0 ? Math.round(rawMt) : 8192;

    const hasAllowedTools = 'allowedTools' in obj || 'allowed_tools' in obj;
    const allowedToolsKey = 'allowedTools' in obj ? 'allowedTools' : ('allowed_tools' in obj ? 'allowed_tools' : undefined);
    const allowedTools = hasAllowedTools
      ? (obj.allowedTools !== undefined ? obj.allowedTools : obj.allowed_tools)
      : undefined;

    const icon = typeof obj.icon === 'string' && obj.icon.trim() ? obj.icon.trim() : undefined;
    const color = typeof obj.color === 'string' && obj.color.trim() ? obj.color.trim() : undefined;

    const rawExtra: Record<string, unknown> = {};
    for (const [key, val] of Object.entries(obj)) {
      if (
        ![
          'model',
          'temperature',
          'reasoningEffort',
          'reasoning_effort',
          'contextWindow',
          'context_window',
          'maxTokens',
          'max_tokens',
          'allowedTools',
          'allowed_tools',
          'icon',
          'color',
        ].includes(key)
      ) {
        rawExtra[key] = val;
      }
    }

    return {
      parsed: {
        model,
        reasoningEffort,
        contextWindow,
        maxTokens,
        hasAllowedTools,
        allowedToolsKey,
        allowedTools,
        icon,
        color,
        rawExtra,
      },
      error: null,
    };
  } catch (err) {
    return {
      parsed: {
        model: 'inherit',
        reasoningEffort: 'off',
        contextWindow: 200000,
        maxTokens: 8192,
        hasAllowedTools: false,
        allowedToolsKey: undefined,
        allowedTools: undefined,
        icon: undefined,
        color: undefined,
        rawExtra: {},
      },
      error: err instanceof Error ? err.message : String(err),
    };
  }
}

export function serializeSetting(parsed: ParsedSetting): string {
  const output: Record<string, unknown> = {};

  if (parsed.rawExtra._comment) {
    output._comment = parsed.rawExtra._comment;
  }

  output.model =
    !parsed.model || parsed.model === 'inherit' ? 'inherit' : parsed.model.trim();

  if (parsed.reasoningEffort && parsed.reasoningEffort !== 'off') {
    output.reasoningEffort = parsed.reasoningEffort;
  }
  output.contextWindow = parsed.contextWindow;
  output.maxTokens = parsed.maxTokens;

  if (parsed.hasAllowedTools) {
    output[parsed.allowedToolsKey || 'allowedTools'] = parsed.allowedTools;
  }

  if (parsed.icon) {
    output.icon = parsed.icon;
  }
  if (parsed.color) {
    output.color = parsed.color;
  }

  for (const [key, val] of Object.entries(parsed.rawExtra)) {
    if (key !== '_comment') {
      output[key] = val;
    }
  }

  return JSON.stringify(output, null, 2);
}
