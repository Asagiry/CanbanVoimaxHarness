import { create } from 'zustand';
import { ProjectInfo } from '../types';
import { useBoardStore } from './boardStore';
import { useCortexStore } from './cortexStore';

interface ProjectState {
  activeProject: ProjectInfo;
  availableProjects: ProjectInfo[];
  isTauriEnvironment: boolean;

  // Actions
  switchProject: (projectId: string) => Promise<void>;
  updateActiveProject: (partial: Partial<ProjectInfo>) => void;
  loadProjects: () => Promise<void>;
  selectProjectFolder: () => Promise<string | null>;
  openFolder: (path: string) => Promise<void>;
  setActiveProjectByPath: (path: string) => Promise<void>;
}

const OPENED_PROJECTS_KEY = 'cvh_opened_projects';

function loadStoredProjects(): ProjectInfo[] {
  try {
    if (typeof window !== 'undefined' && window.localStorage) {
      const raw = window.localStorage.getItem(OPENED_PROJECTS_KEY);
      if (raw) {
        const parsed = JSON.parse(raw);
        if (Array.isArray(parsed)) {
          return parsed;
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
      window.localStorage.setItem(OPENED_PROJECTS_KEY, JSON.stringify(projects));
    }
  } catch (err) {
    console.warn('Failed to persist stored projects to localStorage:', err);
  }
}

const initialProjects = loadStoredProjects();
const DEFAULT_PROJECT: ProjectInfo = initialProjects[0] || {
  id: 'default',
  name: 'CanbanVoimaxHarness',
  rootPath: '',
  activeBranch: 'main',
  worktreesCount: 0,
};

export const useProjectStore = create<ProjectState>((set, get) => ({
  activeProject: DEFAULT_PROJECT,
  availableProjects: initialProjects,
  isTauriEnvironment: true,

  switchProject: async (projectId: string) => {
    const state = get();
    const found = state.availableProjects.find(
      (p) => p.id === projectId || p.rootPath.toLowerCase() === projectId.toLowerCase()
    );
    if (!found) return;

    set({ activeProject: found });

    if (found.rootPath) {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('set_active_project', { path: found.rootPath });
      } catch (err) {
        console.warn('Failed to invoke set_active_project on switch:', err);
      }
    }

    // When switching projects, clear and reload board, tasks, and cortex agents
    const board = useBoardStore.getState();
    board.setTasks([]);
    await board.loadBoards();
    await board.loadTasks();
    await useCortexStore.getState().loadAgentPersonas();
    useCortexStore.getState().syncStagesFromColumns(useBoardStore.getState().columns);
  },

  updateActiveProject: (partial: Partial<ProjectInfo>) => {
    set((state) => ({
      activeProject: { ...state.activeProject, ...partial },
    }));
  },

  loadProjects: async () => {
    let backendActive: ProjectInfo | null = null;
    let backendProjects: ProjectInfo[] = [];

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const activeRaw = await invoke<any>('get_active_project').catch(() => null);
      if (activeRaw && (activeRaw.rootPath || activeRaw.path)) {
        const rootPath = String(activeRaw.rootPath || activeRaw.path || '').trim();
        backendActive = {
          id: activeRaw.id || rootPath.replace(/\\/g, '/').split('/').filter(Boolean).pop()?.toLowerCase() || 'default',
          name: activeRaw.name || rootPath.replace(/\\/g, '/').split('/').filter(Boolean).pop() || 'Project',
          rootPath,
          activeBranch: activeRaw.activeBranch || activeRaw.active_branch || 'main',
          worktreesCount:
            typeof activeRaw.worktreesCount === 'number'
              ? activeRaw.worktreesCount
              : activeRaw.worktrees_count || 0,
        };
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
          });
      }
    } catch (err) {
      console.warn('Backend project query not available or in web mode:', err);
    }

    const stored = loadStoredProjects();
    const mergedMap = new Map<string, ProjectInfo>();

    if (backendActive) {
      mergedMap.set(backendActive.rootPath.toLowerCase(), backendActive);
    }
    for (const p of backendProjects) {
      if (p.rootPath && !mergedMap.has(p.rootPath.toLowerCase())) {
        mergedMap.set(p.rootPath.toLowerCase(), p);
      }
    }
    for (const p of stored) {
      if (p.rootPath && !mergedMap.has(p.rootPath.toLowerCase())) {
        mergedMap.set(p.rootPath.toLowerCase(), p);
      }
    }

    const availableProjects = Array.from(mergedMap.values());
    const activeProject = backendActive || availableProjects[0] || DEFAULT_PROJECT;

    set({
      availableProjects,
      activeProject,
    });
    persistStoredProjects(availableProjects);
  },

  selectProjectFolder: async () => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const chosen = await invoke<string | null>('choose_project_folder');
      if (chosen) {
        await get().setActiveProjectByPath(chosen);
      }
      return chosen;
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
    if (!cleanPath) return;

    const name = cleanPath.replace(/\\/g, '/').split('/').filter(Boolean).pop() || 'Project';
    const id = name.toLowerCase().replace(/[^a-z0-9_-]/g, '-');

    let projName = name;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const cfg = await invoke<{ name: string; version: string }>('set_active_project', { path: cleanPath });
      if (cfg?.name) {
        projName = cfg.name;
      }
    } catch (err) {
      console.error('Failed to set active project by path in Tauri:', err);
    }

    const proj: ProjectInfo = {
      id,
      name: projName,
      rootPath: cleanPath,
      activeBranch: 'main',
      worktreesCount: 0,
    };

    const existing = state.availableProjects.filter(
      (p) => p.id !== id && p.rootPath.toLowerCase() !== cleanPath.toLowerCase()
    );
    const updatedProjects = [proj, ...existing];

    set({
      activeProject: proj,
      availableProjects: updatedProjects,
    });
    persistStoredProjects(updatedProjects);

    // When setting active project by path, reload board, tasks, and cortex agents
    const board = useBoardStore.getState();
    board.setTasks([]);
    await board.loadBoards();
    await board.loadTasks();
    await useCortexStore.getState().loadAgentPersonas();
    useCortexStore.getState().syncStagesFromColumns(useBoardStore.getState().columns);
  },
}));