import type { KanbanTask, PipelineStage } from '../../types';
import type { AgentPersonaInfo } from '../../stores/cortex/types';
import { catalogModelLabel, type Provider } from '../../stores/settings/providers';

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
): string | undefined {
  const explicit = task.providerId?.trim() && task.model?.trim() ? task.model.trim() : undefined;
  const raw =
    explicit ??
    trimmed(persona?.model) ??
    trimmed(column?.modelOverride) ??
    trimmed(settingsModel);
  if (!raw) return undefined;
  return providers && providers.length > 0 ? catalogModelLabel(providers, raw) : raw;
}
