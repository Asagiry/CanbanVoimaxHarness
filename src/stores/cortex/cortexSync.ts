import {
  AgentPersonaInfo,
  CortexHookMoment,
  CortexState,
  RawAgentPersona,
} from './types';
import { CortexAgentFile, CortexAgentStage, PipelineColumn } from '../../types';
import { DEFAULT_CORTEX_STAGES } from './cortexConstants';
import { cortexAgentDir } from './cortexHelpers';
import {
  BACKLOG_COLOR,
  READY_COLOR,
  getDeterministicAgentColor,
  isAllowedAgentColor,
} from '../../lib/agentPalette';
import { getDefaultColumnIcon } from '../../lib/columnIcons';
import { useBoardStore } from '../boardStore';

type CortexSet = (
  partial: Partial<CortexState> | ((state: CortexState) => Partial<CortexState>),
) => void;
type CortexGet = () => CortexState;

function buildColumnHookFiles(
  col: PipelineColumn,
  allowedMoments: string[],
  prevFileById: Map<string, CortexAgentFile>,
  prevStages: CortexAgentStage[],
): CortexAgentFile[] {
  return Object.entries(col.hooks ?? {})
    .filter(([moment]) => allowedMoments.includes(moment))
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([moment, target]) => {
      const hookId = `${col.id}-hook-${moment}`;
      const targetPath =
        typeof target === 'string'
          ? target
          : target?.script || (target as { path?: string })?.path || '';
      const scriptName = targetPath.startsWith('builtin:')
        ? moment
        : targetPath.split(/[/\\]/).pop() || moment;
      const prev =
        prevFileById.get(hookId) ??
        prevStages.flatMap((s) => s.files).find((f) => f.path === targetPath || f.id === hookId);
      return {
        id: hookId,
        name: scriptName,
        stageId: col.id,
        type: 'hook' as const,
        hookMoment: (moment === 'on_exit' ? 'on_leave' : moment) as CortexHookMoment,
        path: targetPath,
        content: prev?.content ?? '',
        isDirty: prev?.isDirty ?? false,
      };
    });
}

function sortRulesBeforeHooks(a: CortexAgentFile, b: CortexAgentFile): number {
  const order = (t: string) => (t === 'rule' ? 0 : 1);
  return order(a.type) - order(b.type) || a.name.localeCompare(b.name);
}

function buildDiskRules(
  ruleNames: string[] | undefined,
  stageId: string,
  dir: string,
  prevFileById: Map<string, CortexAgentFile>,
): CortexAgentFile[] {
  return (ruleNames ?? []).map((ruleName) => {
    const id = `${stageId}-rule-${ruleName}`;
    const prev = prevFileById.get(id);
    return {
      id,
      name: ruleName,
      stageId,
      type: 'rule' as const,
      path: `${dir}/rules/${ruleName}`,
      content: prev?.content ?? '',
      isDirty: prev?.isDirty ?? false,
    };
  });
}

function buildPersonaAndSettingFiles(
  stageId: string,
  dir: string,
  personaFileName: string,
  middleFiles: CortexAgentFile[],
  prevById: Map<string, CortexAgentFile>,
): CortexAgentFile[] {
  const personaId = `${stageId}-persona`;
  const settingsId = `${stageId}-settings`;
  const prevPersona = prevById.get(personaId);
  const prevSettings = prevById.get(settingsId);
  return [
    { id: personaId, name: personaFileName, stageId, type: 'persona', path: `${dir}/${personaFileName}`, content: prevPersona?.content ?? '', isDirty: prevPersona?.isDirty ?? false },
    ...middleFiles,
    { id: settingsId, name: 'setting', stageId, type: 'settings', path: `${dir}/setting`, content: prevSettings?.content ?? '', isDirty: prevSettings?.isDirty ?? false },
  ];
}

function resolveColumnStyle(
  col: PipelineColumn,
  personaInfo?: AgentPersonaInfo,
): { stageColor: string; stageIcon: string } {
  if (col.isIntake || col.id === 'backlog') {
    return { stageColor: BACKLOG_COLOR, stageIcon: col.icon || getDefaultColumnIcon('backlog') };
  }
  if (col.terminal || col.id === 'ready') {
    return { stageColor: READY_COLOR, stageIcon: col.icon || getDefaultColumnIcon('ready') };
  }
  if (col.persona) {
    const stageIcon = personaInfo?.icon || col.icon || getDefaultColumnIcon(col.id);
    const stageColor =
      personaInfo?.color && isAllowedAgentColor(personaInfo.color)
        ? personaInfo.color
        : col.color && isAllowedAgentColor(col.color)
          ? col.color
          : getDeterministicAgentColor(col.persona);
    return { stageColor, stageIcon };
  }
  return {
    stageIcon: col.icon || getDefaultColumnIcon(col.id),
    stageColor: isAllowedAgentColor(col.color) ? col.color : '#171717',
  };
}

function filterStageExtras(
  extras: CortexAgentFile[],
  colId: string,
  dir: string,
  hookFiles: CortexAgentFile[],
  allowAgentHooks: boolean,
): CortexAgentFile[] {
  return extras
    .filter(
      (f) =>
        f.id !== `${colId}-persona` &&
        f.id !== `${colId}-settings` &&
        !hookFiles.some(
          (h) => h.id === f.id || (h.path && f.path && h.path.toLowerCase() === f.path.toLowerCase()),
        ) &&
        (f.type === 'rule' || f.type === 'hook') &&
        (f.path.startsWith(`${dir}/`) ||
          (f.type === 'hook' && f.path.startsWith('.cvh/hooks/')) ||
          (allowAgentHooks && f.type === 'hook')),
    )
    .sort(sortRulesBeforeHooks);
}

export function syncStagesFromColumnsAction(
  set: CortexSet,
  get: CortexGet,
  columns: PipelineColumn[],
): void {
  const prevStages = get().stages;
  const prevStageById = new Map(prevStages.map((st) => [st.stageId, st]));
  const prevFileById = new Map(prevStages.flatMap((st) => st.files.map((f) => [f.id, f])));
  const prevExtrasByStage = new Map<string, CortexAgentFile[]>();
  for (const st of prevStages) {
    for (const f of st.files) {
      const arr = prevExtrasByStage.get(f.stageId) ?? [];
      arr.push(f);
      prevExtrasByStage.set(f.stageId, arr);
    }
  }

  const boardStages: CortexAgentStage[] = [];
  for (const col of columns) {
    const persona = col.persona?.trim() ?? '';
    const isSystemNode =
      persona === '' &&
      (col.isIntake === true || col.terminal === true || col.id === 'backlog' || col.id === 'ready');
    const dir = cortexAgentDir(col.persona ?? '', col.id);

    if (isSystemNode) {
      const hookFiles = buildColumnHookFiles(col, ['on_enter', 'on_leave'], prevFileById, prevStages);
      const extras = filterStageExtras(prevExtrasByStage.get(col.id) ?? [], col.id, dir, hookFiles, false);
      boardStages.push({
        stageId: col.id,
        stageTitle: col.title,
        color: col.color,
        icon: col.icon,
        personaName: 'система',
        isExpanded: prevStageById.get(col.id)?.isExpanded ?? true,
        files: [...hookFiles, ...extras],
      });
      continue;
    }

    const hookFiles = buildColumnHookFiles(
      col,
      ['on_enter', 'on_leave', 'on_finish', 'on_exit'],
      prevFileById,
      prevStages,
    );
    const extras = filterStageExtras(prevExtrasByStage.get(col.id) ?? [], col.id, dir, hookFiles, true);
    const personaInfo = get().agentPersonas.find((ap) => ap.role === col.persona?.trim());
    const diskRules = buildDiskRules(personaInfo?.rules, col.id, dir, prevFileById);
    const inMemoryExtras = extras.filter(
      (f) => f.type !== 'rule' || !diskRules.some((dr) => dr.name === f.name),
    );
    const allExtras = [...diskRules, ...inMemoryExtras].sort(sortRulesBeforeHooks);
    const { stageColor, stageIcon } = resolveColumnStyle(col, personaInfo);

    boardStages.push({
      stageId: col.id,
      stageTitle: col.title,
      color: stageColor,
      icon: stageIcon,
      personaName: col.persona?.trim() || 'без агента',
      isExpanded: prevStageById.get(col.id)?.isExpanded ?? true,
      files: buildPersonaAndSettingFiles(
        col.id,
        dir,
        personaInfo?.personaFile ?? 'persona.md',
        [...hookFiles, ...allExtras],
        prevFileById,
      ),
    });
  }

  if (boardStages.length === 0) {
    if (columns.length === 0) {
      set({ stages: DEFAULT_CORTEX_STAGES });
      return;
    }
    set((state) => {
      const aliveAgentIds = new Set(state.agentStages.flatMap((s) => s.files.map((f) => f.id)));
      const nextMissing: Record<string, boolean> = {};
      for (const [id, value] of Object.entries(state.fileMissing)) {
        if (aliveAgentIds.has(id)) nextMissing[id] = value;
      }
      return { stages: [], fileMissing: nextMissing };
    });
    return;
  }

  const prevMissing = get().fileMissing;
  const nextMissing: Record<string, boolean> = {};
  for (const st of [...boardStages, ...get().agentStages]) {
    for (const f of st.files) {
      if (!f.path.startsWith('builtin:') && prevMissing[f.id]) nextMissing[f.id] = true;
    }
  }
  set({ stages: boardStages, fileMissing: nextMissing });
}

export async function loadAgentPersonasAction(set: CortexSet, get: CortexGet): Promise<void> {
  let raw: RawAgentPersona[] = [];
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    const list = await invoke<RawAgentPersona[]>('list_agent_personas');
    if (Array.isArray(list)) raw = list;
  } catch {
    raw = [];
  }
  const seen = new Set<string>();
  const personas: AgentPersonaInfo[] = [];
  for (const item of raw) {
    const candidate = item?.persona ?? item?.role;
    if (typeof candidate !== 'string') continue;
    const role = candidate.trim();
    if (!role || seen.has(role)) continue;
    seen.add(role);
    personas.push({
      role,
      persona: role,
      hasPrompt: item.hasPrompt ?? item.has_prompt ?? true,
      isBuiltin: item.isBuiltin ?? item.is_builtin ?? role === 'cvh-dev',
      personaFile: item.personaFile ?? item.persona_file ?? 'persona.md',
      rules: Array.isArray(item.rules) ? item.rules : [],
      hooks: Array.isArray(item.hooks) ? item.hooks : [],
      icon: item.icon,
      color: isAllowedAgentColor(item.color) ? item.color! : getDeterministicAgentColor(role),
      contextWindow: item.contextWindow ?? item.context_window,
      reasoningEffort: item.reasoningEffort ?? item.reasoning_effort ?? null,
      model: item.model ?? null,
      providerId: item.providerId ?? item.provider_id ?? null,
    });
  }
  personas.sort((a, b) => a.role.localeCompare(b.role));

  const prevByRole = new Map(get().agentStages.map((st) => [st.personaName, st]));
  const nextStages: CortexAgentStage[] = personas.map((p) => {
    const dir = cortexAgentDir(p.role, p.role);
    const stageId = `agent:${p.role}`;
    const prevStage = prevByRole.get(p.role);
    const prevFiles = prevStage?.files ?? [];
    const prevById = new Map(prevFiles.map((f) => [f.id, f]));
    const diskRules = buildDiskRules(p.rules, stageId, dir, prevById);
    const inMemoryExtras = prevFiles.filter(
      (f) =>
        f.id !== `${stageId}-persona` &&
        f.id !== `${stageId}-settings` &&
        f.type === 'rule' &&
        !diskRules.some((dr) => dr.name === f.name),
    );
    const allRules = [...diskRules, ...inMemoryExtras].sort((a, b) => a.name.localeCompare(b.name));
    const isBuiltin = p.role === 'cvh-dev' || Boolean(p.isBuiltin);

    return {
      stageId,
      stageTitle: p.role,
      color: p.color || getDeterministicAgentColor(p.role),
      icon: p.icon || (isBuiltin ? 'Cpu' : 'Bot'),
      personaName: p.role,
      isExpanded: prevStage?.isExpanded ?? false,
      isBuiltin,
      files: buildPersonaAndSettingFiles(stageId, dir, p.personaFile ?? 'persona.md', allRules, prevById),
    };
  });

  const alive = new Set(nextStages.flatMap((s) => s.files.map((f) => f.id)));
  for (const st of get().stages) {
    for (const f of st.files) alive.add(f.id);
  }
  const prunedMissing: Record<string, boolean> = {};
  for (const [id, value] of Object.entries(get().fileMissing)) {
    if (alive.has(id)) prunedMissing[id] = value;
  }
  set({ agentPersonas: personas, agentStages: nextStages, fileMissing: prunedMissing });

  const boardState = useBoardStore.getState();
  let colsChanged = false;
  const nextCols = boardState.columns.map((col) => {
    if (!col.persona) return col;
    const matched = personas.find((p) => p.role === col.persona?.trim());
    if (!matched) return col;
    const newIcon = matched.icon || col.icon;
    const newColor = matched.color || col.color;
    if (col.icon !== newIcon || col.color !== newColor) {
      colsChanged = true;
      return { ...col, icon: newIcon, color: newColor };
    }
    return col;
  });

  if (colsChanged) {
    boardState.setColumns(nextCols);
    void boardState.saveCurrentBoard(nextCols);
  }

  get().syncStagesFromColumns(nextCols);
}
