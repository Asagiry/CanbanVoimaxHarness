import { create } from 'zustand';
import { ProjectInfo } from '../types';
import { useBoardStore } from './boardStore';
import { useCortexStore } from './cortexStore';
import { useDrawerStore } from './drawerStore';
import { logEvent } from '../lib/logger';

export function isInvalidProjectPath(path?: string | null): boolean {
  if (!path) return true;
  const trimmed = path.trim().replace(/[\\/]+$/, '');
  if (!trimmed || trimmed === 'default') return true;
  const normalized = trimmed.replace(/\\/g, '/').toLowerCase();
  if (
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
  ) {
    return true;
  }
  return false;
}

interface ProjectState {
  activeProject: ProjectInfo;
  availableProjects: ProjectInfo[];
  isTauriEnvironment: boolean;
  hasActiveProject: boolean;

  // Actions
  switchProject: (projectId: string) => Promise<void>;
  updateActiveProject: (partial: Partial<ProjectInfo>) => void;
  loadProjects: () => Promise<void>;
  selectProjectFolder: () => Promise<string | null>;
  openFolder: (path: string) => Promise<void>;
  setActiveProjectByPath: (path: string) => Promise<void>;
  removeProjectFromHistory: (projectId: string) => Promise<void>;
}

const OPENED_PROJECTS_KEY = 'cvh_opened_projects';

function loadStoredProjects(): ProjectInfo[] {
  try {
    if (typeof window !== 'undefined' && window.localStorage) {
      const raw = window.localStorage.getItem(OPENED_PROJECTS_KEY);
      if (raw) {
        const parsed = JSON.parse(raw);
        if (Array.isArray(parsed)) {
          return parsed.filter((p) => p && !isInvalidProjectPath(p.rootPath));
        }
      }
    }
  } catch (err) {
    console.warn('Failed to load stored projects from localStorage:', err);
  }
  return [];
}

function persistStoredProjects(projects: ProjectInfo[]): void {
  try {
    if (typeof window !== 'undefined' && window.localStorage) {
      const valid = projects.filter((p) => p && !isInvalidProjectPath(p.rootPath));
      window.localStorage.setItem(OPENED_PROJECTS_KEY, JSON.stringify(valid));
    }
  } catch (err) {
    console.warn('Failed to persist stored projects to localStorage:', err);
  }
}

const initialProjects = loadStoredProjects();
const initialCandidate = initialProjects.find((p) => p && !isInvalidProjectPath(p.rootPath));
const DEFAULT_PROJECT: ProjectInfo = {
  id: 'default',
  name: 'CanbanVoimaxHarness',
  rootPath: '',
  activeBranch: 'main',
  worktreesCount: 0,
};

export async function performProjectSwitch(targetProject: ProjectInfo): Promise<void> {
  const cleanPath = targetProject.rootPath?.trim();
  if (!cleanPath || isInvalidProjectPath(cleanPath)) return;

  logEvent('INFO', 'PROJECT_STORE', `performProjectSwitch: switching to project "${targetProject.name}"`, cleanPath);

  // 1. Close drawer and editor immediately to avoid stale task/file interaction
  try {
    const drawer = useDrawerStore.getState();
    drawer.closeDrawer();
    useCortexStore.getState().closeEditor?.();
  } catch (err) {
    console.warn('Failed to close drawer before project switch:', err);
  }

  // 2. Clear tasks/boards and lock board with isLoadingTasks: true to prevent auto-saving
  useBoardStore.setState({
    tasks: [],
    columns: [],
    boards: [],
    isLoadingTasks: true,
    isLoadingBoards: true,
    selectedTaskId: null,
    hoveredTaskId: null,
  });

  // 3. Update activeProject state in projectStore
  useProjectStore.setState({
    activeProject: targetProject,
    hasActiveProject: true,
  });

  // 4. Invoke backend to switch project root and SQLite connection pool
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    try {
      await invoke('switch_project', { projectPath: cleanPath });
    } catch {
      await invoke('set_active_project', { path: cleanPath });
    }
  } catch (err) {
    console.error('Failed to invoke switch_project/set_active_project on backend:', err);
  }

  // 5. Sequentially reload isolated entities for the new project
  try {
    const board = useBoardStore.getState();
    await board.loadBoards();
    await board.loadTasks();

    const cortex = useCortexStore.getState();
    await cortex.loadAgentPersonas?.();
    await cortex.loadGlobalRules?.();
    cortex.syncStagesFromColumns(useBoardStore.getState().columns);

    try {
      const { useToolPolicyStore } = await import('./toolPolicyStore');
      await useToolPolicyStore.getState().loadFromTauri?.();
    } catch {
      // tool policy store optional
    }
  } catch (err) {
    console.error('Error reloading project entities:', err);
  } finally {
    useBoardStore.setState({ isLoadingTasks: false, isLoadingBoards: false });
  }
}

export const useProjectStore = create<ProjectState>((set, get) => ({
  activeProject: initialCandidate || DEFAULT_PROJECT,
  availableProjects: initialProjects,
  isTauriEnvironment: true,
  hasActiveProject: false,

  switchProject: async (projectId: string) => {
    const state = get();
    const found = state.availableProjects.find(
      (p) => (p.id === projectId || p.rootPath.toLowerCase() === projectId.toLowerCase()) && !isInvalidProjectPath(p.rootPath)
    );
    if (!found) return;
    await performProjectSwitch(found);
  },

  updateActiveProject: (partial: Partial<ProjectInfo>) => {
    set((state) => {
      const updated = { ...state.activeProject, ...partial };
      return {
        activeProject: updated,
        hasActiveProject: !isInvalidProjectPath(updated.rootPath),
      };
    });
  },

  loadProjects: async () => {
    let backendActive: ProjectInfo | null = null;
    let backendProjects: ProjectInfo[] = [];

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const activeRaw = await invoke<any>('get_active_project').catch(() => null);
      if (
        activeRaw &&
        activeRaw.hasProject !== false &&
        (activeRaw.rootPath || activeRaw.path)
      ) {
        const rootPath = String(activeRaw.rootPath || activeRaw.path || '').trim();
        if (!isInvalidProjectPath(rootPath)) {
          backendActive = {
            id: activeRaw.id || rootPath.replace(/\\/g, '/').split('/').filter(Boolean).pop()?.toLowerCase() || 'default',
            name: activeRaw.name || rootPath.replace(/\\/g, '/').split('/').filter(Boolean).pop() || 'Project',
            rootPath,
            activeBranch: activeRaw.activeBranch || activeRaw.active_branch || 'main',
            worktreesCount:
              typeof activeRaw.worktreesCount === 'number'
                ? activeRaw.worktreesCount
                : activeRaw.worktrees_count || 0,
            hasProject: true,
          };
        }
      }

      const listRaw = await invoke<any[]>('list_projects').catch(() => []);
      if (Array.isArray(listRaw)) {
        backendProjects = listRaw
          .filter((p) => p && (p.rootPath || p.path))
          .map((p) => {
            const rootPath = String(p.rootPath || p.path || '').trim();
            return {
              id: p.id || rootPath.replace(/\\/g, '/').split('/').filter(Boolean).pop()?.toLowerCase() || 'project',
              name: p.name || rootPath.replace(/\\/g, '/').split('/').filter(Boolean).pop() || 'Project',
              rootPath,
              activeBranch: p.activeBranch || p.active_branch || 'main',
              worktreesCount:
                typeof p.worktreesCount === 'number'
                  ? p.worktreesCount
                  : p.worktrees_count || 0,
            };
          })
          .filter((p) => !isInvalidProjectPath(p.rootPath));
      }
    } catch (err) {
      console.warn('Backend project query not available or in web mode:', err);
    }

    const stored = loadStoredProjects().filter((p) => !isInvalidProjectPath(p.rootPath));
    const mergedMap = new Map<string, ProjectInfo>();

    for (const p of stored) {
      if (p.rootPath && !mergedMap.has(p.rootPath.toLowerCase())) {
        mergedMap.set(p.rootPath.toLowerCase(), p);
      }
    }
    if (backendActive && !isInvalidProjectPath(backendActive.rootPath)) {
      const existing = mergedMap.get(backendActive.rootPath.toLowerCase());
      mergedMap.set(backendActive.rootPath.toLowerCase(), { ...existing, ...backendActive });
    }
    for (const p of backendProjects) {
      if (p.rootPath && !mergedMap.has(p.rootPath.toLowerCase())) {
        mergedMap.set(p.rootPath.toLowerCase(), p);
      }
    }

    const availableProjects = Array.from(mergedMap.values());
    const hasValidBackend = Boolean(backendActive && !isInvalidProjectPath(backendActive.rootPath));
    const activeProject = hasValidBackend ? backendActive! : (availableProjects[0] || DEFAULT_PROJECT);

    set({
      availableProjects,
      activeProject,
      hasActiveProject: hasValidBackend,
    });
    persistStoredProjects(availableProjects);
  },

  selectProjectFolder: async () => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const chosen = await invoke<string | null>('choose_project_folder');
      if (chosen && !isInvalidProjectPath(chosen)) {
        await get().setActiveProjectByPath(chosen);
        return chosen;
      }
      return null;
    } catch (err) {
      console.error('Failed to select project folder:', err);
      return null;
    }
  },

  openFolder: async (folderPath: string) => {
    await get().setActiveProjectByPath(folderPath);
  },

  setActiveProjectByPath: async (path: string) => {
    const state = get();
    const cleanPath = path.trim();
    if (!cleanPath || isInvalidProjectPath(cleanPath)) return;

    logEvent('INFO', 'PROJECT_STORE', `setActiveProjectByPath: requested path "${cleanPath}"`);

    const name = cleanPath.replace(/\\/g, '/').split('/').filter(Boolean).pop() || 'Project';
    const id = name.toLowerCase().replace(/[^a-z0-9_-]/g, '-');

    let projName = name;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const cfg = await invoke<{ name: string; version: string }>('get_project_config_cmd').catch(() => null);
      if (cfg?.name) {
        projName = cfg.name;
      }
    } catch {
      // ignore
    }

    const proj: ProjectInfo = {
      id,
      name: projName,
      rootPath: cleanPath,
      activeBranch: 'main',
      worktreesCount: 0,
      hasProject: true,
    };

    const exists = state.availableProjects.some(
      (p) => p.id === id || p.rootPath.toLowerCase() === cleanPath.toLowerCase()
    );
    const updatedProjects = exists
      ? state.availableProjects.map((p) =>
          p.id === id || p.rootPath.toLowerCase() === cleanPath.toLowerCase() ? proj : p
        )
      : [...state.availableProjects.filter((p) => !isInvalidProjectPath(p.rootPath)), proj];

    set({
      availableProjects: updatedProjects,
    });
    persistStoredProjects(updatedProjects);

    await performProjectSwitch(proj);
  },

  removeProjectFromHistory: async (projectId: string) => {
    const state = get();
    const cleanId = projectId.trim();
    if (!cleanId) return;

    const targetProject = state.availableProjects.find(
      (p) => p.id === cleanId || p.rootPath.toLowerCase() === cleanId.toLowerCase()
    );

    const updatedProjects = state.availableProjects.filter(
      (p) => p.id !== cleanId && p.rootPath.toLowerCase() !== cleanId.toLowerCase()
    );

    persistStoredProjects(updatedProjects);

    const isCurrentActive =
      state.activeProject.id === cleanId ||
      state.activeProject.rootPath.toLowerCase() === cleanId.toLowerCase() ||
      Boolean(targetProject && state.activeProject.rootPath.toLowerCase() === targetProject.rootPath.toLowerCase());

    if (isCurrentActive) {
      const nextProject = updatedProjects[0] || DEFAULT_PROJECT;
      logEvent('INFO', 'PROJECT_STORE', `removeProjectFromHistory: active project removed, switching to "${nextProject.name}"`);

      set({
        availableProjects: updatedProjects,
      });

      await performProjectSwitch(nextProject);
    } else {
      set({ availableProjects: updatedProjects });
    }
  },
}));
