import { ProjectInfo } from '../../types';
import { useBoardStore } from '../boardStore';
import { useCortexStore } from '../cortexStore';
import { useDrawerStore } from '../drawerStore';
import { useAgentSessionStore } from '../agentSessionStore';

const OPENED_PROJECTS_KEY = 'cvh_opened_projects';

export interface RawBackendProjectDTO {
  id?: string;
  name?: string;
  rootPath?: string;
  path?: string;
  activeBranch?: string;
  active_branch?: string;
  worktreesCount?: number;
  worktrees_count?: number;
  hasProject?: boolean;
}

export const DEFAULT_PROJECT: ProjectInfo = {
  id: 'default',
  name: 'CanbanVoimaxHarness',
  rootPath: '',
  activeBranch: 'main',
  worktreesCount: 0,
};

export function isInvalidProjectPath(path?: string | null): boolean {
  if (!path) return true;
  const trimmed = path.trim().replace(/[\\/]+$/, '');
  if (!trimmed || trimmed === 'default') return true;
  const normalized = trimmed.replace(/\\/g, '/').toLowerCase();
  return (
    normalized.endsWith('/bin') ||
    normalized.endsWith('/release') ||
    normalized.endsWith('/debug') ||
    normalized.endsWith('/target') ||
    normalized.endsWith('/dist') ||
    normalized.endsWith('/node_modules') ||
    normalized.endsWith('/src-tauri') ||
    normalized === 'bin' ||
    normalized === 'release' ||
    normalized === 'debug'
  );
}

export function loadStoredProjects(): ProjectInfo[] {
  try {
    if (typeof window !== 'undefined' && window.localStorage) {
      const raw = window.localStorage.getItem(OPENED_PROJECTS_KEY);
      if (raw) {
        const parsed: unknown = JSON.parse(raw);
        if (Array.isArray(parsed)) {
          return parsed.filter((p): p is ProjectInfo => Boolean(p) && !isInvalidProjectPath((p as ProjectInfo).rootPath));
        }
      }
    }
  } catch (err) {
    console.warn('Failed to load stored projects from localStorage:', err);
  }
  return [];
}

export function persistStoredProjects(projects: ProjectInfo[]): void {
  try {
    if (typeof window !== 'undefined' && window.localStorage) {
      const valid = projects.filter((p) => p && !isInvalidProjectPath(p.rootPath));
      window.localStorage.setItem(OPENED_PROJECTS_KEY, JSON.stringify(valid));
    }
  } catch (err) {
    console.warn('Failed to persist stored projects to localStorage:', err);
  }
}

export function mapBackendProject(raw: RawBackendProjectDTO, includeHasProject = false): ProjectInfo | null {
  const rootPath = String(raw.rootPath || raw.path || '').trim();
  if (!rootPath || isInvalidProjectPath(rootPath)) return null;
  const parts = rootPath.replace(/\\/g, '/').split('/').filter(Boolean);
  const tail = parts[parts.length - 1];
  return {
    id: raw.id || tail?.toLowerCase() || 'project',
    name: raw.name || tail || 'Project',
    rootPath,
    activeBranch: raw.activeBranch || raw.active_branch || 'main',
    worktreesCount: typeof raw.worktreesCount === 'number' ? raw.worktreesCount : raw.worktrees_count || 0,
    ...(includeHasProject ? { hasProject: true } : {}),
  };
}

export function resetWorkspaceStoresOnProjectChange(keepLoadingLock: boolean): void {
  try {
    const drawer = useDrawerStore.getState();
    drawer.closeDrawer();
    drawer.resetProjectContext();
    useCortexStore.getState().closeEditor?.();
    useAgentSessionStore.getState().reset();
  } catch (err) {
    console.warn('Failed to reset drawer/cortex/session state on project change:', err);
  }
  useBoardStore.setState({
    tasks: [],
    stages: [],
    columns: [],
    boards: [],
    isLoadingTasks: keepLoadingLock,
    isLoadingBoards: keepLoadingLock,
    selectedTaskId: null,
    hoveredTaskId: null,
  });
}
