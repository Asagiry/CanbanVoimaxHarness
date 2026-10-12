import { create } from 'zustand';
import { ProjectInfo } from '../types';

interface ProjectState {
  activeProject: ProjectInfo;
  availableProjects: ProjectInfo[];
  isTauriEnvironment: boolean;

  // Actions
  switchProject: (projectId: string) => Promise<void>;
  updateActiveProject: (partial: Partial<ProjectInfo>) => void;
  loadProjects: () => Promise<void>;
  selectProjectFolder: () => Promise<string | null>;
  setActiveProjectByPath: (path: string) => Promise<void>;
}

const DEFAULT_PROJECT: ProjectInfo = {
  id: 'cvh-dogfooding',
  name: 'CanbanVoimaxHarness',
  rootPath: 'C:\\Users\\Voimax\\Projects\\CanbanVoimaxHarness',
  activeBranch: 'feat/cvh-001-frontend',
  worktreesCount: 2,
};

const SAMPLE_PROJECTS: ProjectInfo[] = [
  DEFAULT_PROJECT,
  {
    id: 'carsmash-sim',
    name: 'CarSmash Destruction Engine',
    rootPath: 'C:\\Users\\Voimax\\Projects\\CarSmash',
    activeBranch: 'main',
    worktreesCount: 1,
  },
];

export const useProjectStore = create<ProjectState>((set, get) => ({
  activeProject: DEFAULT_PROJECT,
  availableProjects: SAMPLE_PROJECTS,
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
  },

  updateActiveProject: (partial: Partial<ProjectInfo>) => {
    set((state) => ({
      activeProject: { ...state.activeProject, ...partial },
    }));
  },

  loadProjects: async () => {
    const state = get();
    if (!state.isTauriEnvironment) return;

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const list = await invoke<Array<{ id: string; name: string; path: string; rootPath?: string; activeBranch?: string; worktreesCount?: number }>>('list_projects');
      if (Array.isArray(list) && list.length > 0) {
        const mapped: ProjectInfo[] = list.map((p) => ({
          id: p.id,
          name: p.name,
          rootPath: p.rootPath || p.path,
          activeBranch: p.activeBranch || 'main',
          worktreesCount: p.worktreesCount || 0,
        }));
        set({ availableProjects: mapped });
        if (mapped.length > 0) {
          set({ activeProject: mapped[0] });
        }
      }
    } catch (err) {
      console.warn('Failed to load projects from Tauri:', err);
    }
  },

  selectProjectFolder: async () => {
    const state = get();
    if (!state.isTauriEnvironment) return null;

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const chosen = await invoke<string | null>('select_project_folder');
      if (chosen) {
        await get().setActiveProjectByPath(chosen);
      }
      return chosen;
    } catch (err) {
      console.error('Failed to select project folder:', err);
      return null;
    }
  },

  setActiveProjectByPath: async (path: string) => {
    const state = get();
    if (!state.isTauriEnvironment) {
      const name = path.split(/[/\\]/).filter(Boolean).pop() || 'Project';
      const id = name.toLowerCase().replace(/\s+/g, '-');
      const proj: ProjectInfo = {
        id,
        name,
        rootPath: path,
        activeBranch: 'main',
        worktreesCount: 0,
      };
      set((s) => ({
        activeProject: proj,
        availableProjects: [proj, ...s.availableProjects.filter((p) => p.id !== id)],
      }));
      return;
    }

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const cfg = await invoke<{ name: string; version: string }>('set_active_project', { path });
      const name = cfg.name || path.split(/[/\\]/).filter(Boolean).pop() || 'Project';
      const id = name.toLowerCase().replace(/\s+/g, '-');
      const proj: ProjectInfo = {
        id,
        name,
        rootPath: path,
        activeBranch: 'main',
        worktreesCount: 0,
      };
      set((s) => ({
        activeProject: proj,
        availableProjects: [proj, ...s.availableProjects.filter((p) => p.id !== id)],
      }));
    } catch (err) {
      console.error('Failed to set active project by path:', err);
    }
  },
}));
