import { CortexAgentFile, CortexAgentStage, PipelineColumn } from '../../types';
import { CvhGitStatus, CortexState, PendingDelete } from './types';

type CortexSet = (
  partial: Partial<CortexState> | ((state: CortexState) => Partial<CortexState>),
) => void;
type CortexGet = () => CortexState;

export function normPath(p: string): string {
  return p.replace(/\\/g, '/').replace(/^\.\//, '');
}

export function isAgentDirty(
  stage: CortexAgentStage,
  gitStatus: CvhGitStatus | null,
  pendingDeletes: Record<string, PendingDelete>,
): boolean {
  if (stage.isDeleted) return true;
  if (stage.files.some((f) => f.isDirty || f.isDeleted)) return true;
  if (Object.values(pendingDeletes).some((d) => d.kind === 'agent' && d.role === stage.personaName)) return true;
  const prefix = `.cvh/agents/${stage.personaName}/`;
  return Boolean(gitStatus?.changedPaths.some((p) => normPath(p).startsWith(prefix)));
}

function personaAliasPaths(p: string): string[] {
  const n = normPath(p);
  const out = [n];
  if (n.endsWith('/agent.md')) out.push(n.replace(/\/agent\.md$/, '/persona.md'));
  if (n.endsWith('/persona.md')) out.push(n.replace(/\/persona\.md$/, '/agent.md'));
  if (n.endsWith('/setting')) out.push(`${n}.md`);
  if (n.endsWith('/setting.md')) out.push(n.replace(/\.md$/, ''));
  return out;
}

export function isPersonaFileDirty(
  file: Pick<CortexAgentFile, 'path' | 'isDirty' | 'isDeleted'>,
  gitStatus: CvhGitStatus | null,
): boolean {
  return isFileDirty(file, gitStatus);
}

export function isFileDirty(
  file: Pick<CortexAgentFile, 'path' | 'isDirty' | 'isDeleted'>,
  gitStatus: CvhGitStatus | null,
): boolean {
  if (file.isDirty || file.isDeleted) return true;
  if (!file.path || file.path.startsWith('builtin:')) return false;
  const targets = new Set(personaAliasPaths(file.path));
  const stripMd = (s: string) => s.replace(/\.md$/, '');
  const strippedTargets = new Set([...targets].map(stripMd));
  return Boolean(
    gitStatus?.changedPaths.some((p) => {
      const n = normPath(p);
      return targets.has(n) || strippedTargets.has(stripMd(n));
    }),
  );
}

export function isColumnDirty(
  stage: CortexAgentStage,
  column: PipelineColumn | undefined,
  gitStatus: CvhGitStatus | null,
  pendingColumnChanges: Record<string, true> = {},
): boolean {
  if (pendingColumnChanges[stage.stageId]) return true;
  if (stage.files.some((f) => f.isDirty || f.isDeleted)) return true;
  const paths = gitStatus?.changedPaths.map(normPath) ?? [];
  if (paths.length === 0) return false;
  const hookPaths = new Set(
    stage.files
      .filter((f) => f.type === 'hook' && f.path && !f.path.startsWith('builtin:'))
      .map((f) => normPath(f.path)),
  );
  if (paths.some((p) => hookPaths.has(p))) return true;
  const hookPrefix = `.cvh/hooks/${stage.stageId}-`;
  if (paths.some((p) => p.startsWith(hookPrefix))) return true;
  const persona = column?.persona?.trim() || stage.personaName;
  if (persona && persona !== 'система' && persona !== 'без агента') {
    const prefix = `.cvh/agents/${persona}/`;
    if (paths.some((p) => p.startsWith(prefix))) return true;
  }
  if (paths.some((p) => p.startsWith('.cvh/rules/'))) return true;
  if (paths.some((p) => p.startsWith('.cvh/agents/') && p.includes('/rules/'))) return true;
  return false;
}

export function isGlobalRulesDirty(
  gitStatus: CvhGitStatus | null,
  globalRules: CortexAgentFile[] = [],
): boolean {
  if (globalRules.some((f) => f.isDirty || f.isDeleted)) return true;
  return Boolean(gitStatus?.changedPaths.some((p) => normPath(p).startsWith('.cvh/rules/')));
}

export function hasAnyCvhChanges(state: Pick<CortexState, 'cvhGitStatus' | 'pendingDeletes' | 'stages' | 'agentStages' | 'globalRules' | 'activeFile' | 'pendingColumnChanges'>): boolean {
  if (state.cvhGitStatus?.hasChanges) return true;
  if (Object.keys(state.pendingDeletes).length > 0) return true;
  if (Object.keys(state.pendingColumnChanges ?? {}).length > 0) return true;
  if (state.activeFile?.isDirty) return true;
  const all = [...state.stages, ...state.agentStages];
  if (all.some((s) => s.isDeleted || s.files.some((f) => f.isDirty || f.isDeleted))) return true;
  return state.globalRules.some((f) => f.isDirty || f.isDeleted);
}

export async function refreshCvhGitStatusAction(set: CortexSet): Promise<void> {
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    const status = await invoke<CvhGitStatus>('cvh_git_status');
    set({ cvhGitStatus: status, cvhGitError: null });
  } catch (err) {
    const detail = err instanceof Error ? err.message : String(err);
    set({ cvhGitError: detail });
  }
}

export async function saveCvhToGitAction(set: CortexSet, get: CortexGet, message?: string): Promise<void> {
  const pending = get().pendingDeletes;
  set({ cvhGitSaving: true, cvhGitError: null });
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    const failedKeys: string[] = [];
    const deleteErrors: string[] = [];
    for (const [key, entry] of Object.entries(pending)) {
      try {
        if (entry.kind === 'agent' && entry.role) {
          await invoke('delete_agent_persona', { role: entry.role });
        } else if (entry.path) {
          await invoke('delete_cortex_file', { path: entry.path });
        }
      } catch (err) {
        failedKeys.push(key);
        deleteErrors.push(err instanceof Error ? err.message : String(err));
      }
    }
    if (failedKeys.length > 0) {
      const detail = `Не удалось удалить ${failedKeys.length} файл(ов): ${deleteErrors.join('; ')}`;
      set({ cvhGitSaving: false, cvhGitError: detail });
      throw new Error(detail);
    }
    const status = await invoke<CvhGitStatus>('cvh_git_save', { message: message ?? null });
    set((state) => ({
      cvhGitStatus: status,
      cvhGitSaving: false,
      pendingDeletes: {},
      pendingColumnChanges: {},
      stages: state.stages.map((s) => ({ ...s, isDeleted: false, files: s.files.filter((f) => !f.isDeleted).map((f) => ({ ...f, isDirty: false })) })),
      agentStages: state.agentStages.filter((s) => !s.isDeleted).map((s) => ({ ...s, files: s.files.filter((f) => !f.isDeleted).map((f) => ({ ...f, isDirty: false })) })),
      globalRules: state.globalRules.filter((f) => !f.isDeleted).map((f) => ({ ...f, isDirty: false })),
      activeFile: state.activeFile ? { ...state.activeFile, isDirty: false } : null,
    }));
  } catch (err) {
    const detail = err instanceof Error ? err.message : String(err);
    set({ cvhGitSaving: false, cvhGitError: detail });
    throw err instanceof Error ? err : new Error(detail);
  }
}

export function markPendingColumnChangeAction(set: CortexSet, stageId: string): void {
  set((state) => ({ pendingColumnChanges: { ...state.pendingColumnChanges, [stageId]: true } }));
}

export function markPendingDeleteAction(set: CortexSet, key: string, entry: PendingDelete): void {
  set((state) => ({ pendingDeletes: { ...state.pendingDeletes, [key]: entry } }));
}

export function restorePendingDeleteAction(set: CortexSet, key: string): void {
  set((state) => {
    const entry = state.pendingDeletes[key];
    if (!entry) return state;
    const next = { ...state.pendingDeletes };
    delete next[key];
    const clearFile = (f: CortexAgentFile) => (f.id === entry.fileId || f.path === entry.path ? { ...f, isDeleted: false } : f);
    return {
      pendingDeletes: next,
      globalRules: state.globalRules.map(clearFile),
      stages: state.stages.map((s) => ({
        ...s,
        isDeleted: entry.kind === 'agent' && s.personaName === entry.role ? false : s.isDeleted,
        files: s.files.map(clearFile),
      })),
      agentStages: state.agentStages.map((s) => ({
        ...s,
        isDeleted: entry.kind === 'agent' && s.personaName === entry.role ? false : s.isDeleted,
        files: s.files.map(clearFile),
      })),
    };
  });
}
