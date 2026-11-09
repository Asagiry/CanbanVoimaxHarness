import type { KanbanTask, PipelineStage } from '../../types';
import type { AgentPersonaInfo } from '../../stores/cortex/types';
import { catalogModelLabel, catalogModelProviderName, type Provider } from '../../stores/settings/providers';

export interface CardModelDisplay {
  label: string;
  providerName?: string;
}

function trimmed(value?: string | null): string | undefined {
  const v = value?.trim();
  return v ? v : undefined;
}

export function resolveCardModel(
  task: KanbanTask,
  column: PipelineStage | undefined,
  persona: AgentPersonaInfo | undefined,
  settingsModel?: string | null,
  providers?: Provider[],
  settingsProvider?: string | null,
): CardModelDisplay | undefined {
  const explicitProviderId = trimmed(task.providerId);
  const explicit = explicitProviderId && trimmed(task.model) ? trimmed(task.model) : undefined;
  const raw =
    explicit ??
    trimmed(persona?.model) ??
    trimmed(column?.modelOverride) ??
    trimmed(settingsModel);
  if (!raw) return undefined;
  const list = providers ?? [];
  const label = list.length > 0 ? catalogModelLabel(list, raw) : raw;
  if (list.length === 0) return { label };
  let providerName: string | undefined;
  if (explicit) {
    providerName = list.find((p) => p.id === explicitProviderId)?.name;
  }
  if (!providerName) {
    providerName = catalogModelProviderName(
      list,
      raw,
      explicit ? explicitProviderId : trimmed(persona?.providerId),
    );
  }
  if (!providerName) {
    providerName = trimmed(settingsProvider);
  }
  return { label, providerName };
}

export function formatCardModel(display: CardModelDisplay): string {
  return display.providerName ? `${display.providerName} / ${display.label}` : display.label;
}
