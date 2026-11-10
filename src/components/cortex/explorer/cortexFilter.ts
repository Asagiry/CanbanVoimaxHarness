import type { CortexAgentFile, CortexAgentStage } from '../../../types';
import { isBacklogSystemStage } from '../types';

function matchesFile(file: CortexAgentFile, normalizedQuery: string): boolean {
  return (
    file.name.toLowerCase().includes(normalizedQuery) ||
    file.path.toLowerCase().includes(normalizedQuery)
  );
}

/**
 * Filter column stages by stage title, persona name, or matching hook/backlog setting files.
 */
export function filterColumnStages(
  stages: CortexAgentStage[],
  normalizedQuery: string,
): CortexAgentStage[] {
  if (!normalizedQuery) return stages;

  return stages
    .map((stage) => {
      const matchesStage =
        stage.stageTitle.toLowerCase().includes(normalizedQuery) ||
        stage.personaName.toLowerCase().includes(normalizedQuery);
      if (matchesStage) return { ...stage, isExpanded: true };

      const backlog = isBacklogSystemStage(stage);
      const matchingHooks = stage.files.filter(
        (f) => f.type === 'hook' && matchesFile(f, normalizedQuery),
      );
      const settingsFile = stage.files.find((f) => f.type === 'settings');
      const matchingSetting =
        backlog && settingsFile && matchesFile(settingsFile, normalizedQuery)
          ? settingsFile
          : null;

      if (matchingHooks.length === 0 && !matchingSetting) return null;

      const keepIds = new Set([
        ...matchingHooks.map((f) => f.id),
        ...(matchingSetting ? [matchingSetting.id] : []),
      ]);
      return {
        ...stage,
        isExpanded: true,
        files: stage.files.filter((f) => keepIds.has(f.id)),
      };
    })
    .filter((stage): stage is CortexAgentStage => stage !== null);
}

/**
 * Filter agent stages by role/title or matching files.
 */
export function filterAgentStages(
  agentStages: CortexAgentStage[],
  normalizedQuery: string,
): CortexAgentStage[] {
  if (!normalizedQuery) return agentStages;

  return agentStages
    .map((stage) => {
      const matchesAgent =
        stage.stageTitle.toLowerCase().includes(normalizedQuery) ||
        stage.personaName.toLowerCase().includes(normalizedQuery);
      if (matchesAgent) return { ...stage, isExpanded: true };

      const matchingFiles = stage.files.filter((f) => matchesFile(f, normalizedQuery));
      if (matchingFiles.length === 0) return null;

      return { ...stage, isExpanded: true, files: matchingFiles };
    })
    .filter((stage): stage is CortexAgentStage => stage !== null);
}

/**
 * Filter global rules by file name or path.
 */
export function filterGlobalRules(
  globalRules: CortexAgentFile[],
  normalizedQuery: string,
): CortexAgentFile[] {
  if (!normalizedQuery) return globalRules;
  return globalRules.filter((f) => matchesFile(f, normalizedQuery));
}
