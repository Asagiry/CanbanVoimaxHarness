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
  id: 'current-workspace',
  name: 'CanbanVoimaxHarness',
  rootPath: typeof window !== 'undefined' ? window.location.pathname : '.',
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

    // Immediately reload tasks for the newly selected project
    await useBoardStore.getState().loadTasks();
  },

  updateActiveProject: (partial: Partial<ProjectInfo>) => {
    set((state) => ({
      activeProject: { ...state.activeProject, ...partial },
    }));
  },

  loadProjects: async () => {
    const state = get();
    if (state.isTauriEnvironment) {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        // 1. Try to load registered projects list from backend
        const list = await invoke<Array<{ id: string; name: string; path: string; rootPath?: string; activeBranch?: string; worktreesCount?: number }>>('list_projects');
        if (Array.isArray(list) && list.length > 0) {
          const mapped: ProjectInfo[] = list.map((p) => ({
            id: p.id,
            name: p.name,
            rootPath: p.rootPath || p.path,
            activeBranch: p.activeBranch || 'main',
            worktreesCount: p.worktreesCount || 0,
          }));

          const stored = loadStoredProjects();
          const combined = [...mapped];
          for (const s of stored) {
            if (!combined.some((c) => c.id === s.id || c.rootPath === s.rootPath)) {
              combined.push(s);
            }
          }

          set({ availableProjects: combined, activeProject: combined[0] });
          persistStoredProjects(combined);
          return;
        }

        // 2. Fallback: query active project config from backend
        const cfg = await invoke<{ name: string; version: string }>('get_project_config_cmd');
        if (cfg && cfg.name) {
          const name = cfg.name;
          const id = name.toLowerCase().replace(/\s+/g, '-');
          const proj: ProjectInfo = {
            id,
            name,
            rootPath: '.',
            activeBranch: 'main',
            worktreesCount: 0,
          };
          const stored = loadStoredProjects();
          const combined = [proj, ...stored.filter((p) => p.id !== id)];
          set({ activeProject: proj, availableProjects: combined });
          persistStoredProjects(combined);
          return;
        }
      } catch (err) {
        console.warn('Failed to load projects from Tauri:', err);
      }
    }

    // Web fallback or Tauri without backend projects
    const stored = loadStoredProjects();
    if (stored.length > 0) {
      set({ availableProjects: stored, activeProject: stored[0] });
    }
  },

  selectProjectFolder: async () => {
    const state = get();
    if (!state.isTauriEnvironment) return null;

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
    const name = cleanPath.split(/[/\\]/).filter(Boolean).pop() || 'Project';
    const id = name.toLowerCase().replace(/\s+/g, '-');

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

    // Immediately reload tasks for the new project
    await useBoardStore.getState().loadTasks();
  },
}));
