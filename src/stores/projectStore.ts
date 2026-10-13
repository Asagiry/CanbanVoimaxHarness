import { create } from 'zustand';
import { ProjectInfo } from '../types';
import { useBoardStore } from './boardStore';

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
  isTauriEnvironment: typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window,

  switchProject: async (projectId: string) => {
    const state = get();
    const found = state.availableProjects.find((p) => p.id === projectId);
    if (!found) return;

    set({ activeProject: found });

    if (state.isTauriEnvironment && found.rootPath) {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('set_active_project', { path: found.rootPath });
      } catch (err) {
        console.warn('Failed to invoke set_active_project on switch:', err);
      }
    }

    // When switching projects, clear and reload board tasks
    const board = useBoardStore.getState();
    board.setTasks([]);
    await board.loadTasks();
  },

  updateActiveProject: (partial: Partial<ProjectInfo>) => {
    set((state) => ({
      activeProject: { ...state.activeProject, ...partial },
    }));
  },

  loadProjects: async () => {
    const stored = loadStoredProjects();
    if (stored.length > 0) {
      set({
        availableProjects: stored,
        activeProject: stored[0],
      });
    } else {
      set({
        availableProjects: [],
      });
    }
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

    const name = cleanPath.split(/[/\\]/).filter(Boolean).pop() || 'Project';
    const id = name.toLowerCase().replace(/[^a-z0-9_-]/g, '-');

    let projName = name;
    if (state.isTauriEnvironment) {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const cfg = await invoke<{ name: string; version: string }>('set_active_project', { path: cleanPath });
        if (cfg?.name) {
          projName = cfg.name;
        }
      } catch (err) {
        console.error('Failed to set active project by path in Tauri:', err);
      }
    }

    const proj: ProjectInfo = {
      id,
      name: projName,
      rootPath: cleanPath,
      activeBranch: 'main',
      worktreesCount: 0,
    };

    const existing = state.availableProjects.filter((p) => p.id !== id && p.rootPath !== cleanPath);
    const updatedProjects = [proj, ...existing];

    set({
      activeProject: proj,
      availableProjects: updatedProjects,
    });
    persistStoredProjects(updatedProjects);

    // Call useBoardStore.getState().loadTasks() to load that project's tasks
    const board = useBoardStore.getState();
    board.setTasks([]);
    await board.loadTasks();
  },
}));
