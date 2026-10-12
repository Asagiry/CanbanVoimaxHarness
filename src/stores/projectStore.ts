import { create } from 'zustand';
import { ProjectInfo } from '../types';

interface ProjectState {
  activeProject: ProjectInfo;
  availableProjects: ProjectInfo[];
  isTauriEnvironment: boolean;

  // Actions
  switchProject: (projectId: string) => void;
  updateActiveProject: (partial: Partial<ProjectInfo>) => void;
  openFolder: (path: string) => Promise<void>;
}

const DEFAULT_PROJECT: ProjectInfo = {
  id: 'current-workspace',
  name: 'CanbanVoimaxHarness',
  rootPath: typeof window !== 'undefined' ? window.location.pathname : '.',
  activeBranch: 'main',
  worktreesCount: 0,
};

export const useProjectStore = create<ProjectState>((set) => ({
  activeProject: DEFAULT_PROJECT,
  availableProjects: [DEFAULT_PROJECT],
  isTauriEnvironment: typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window,

  switchProject: (projectId) => {
    set((state) => {
      const found = state.availableProjects.find((p) => p.id === projectId);
      if (found) {
        return { activeProject: found };
      }
      return state;
    });
  },

  updateActiveProject: (partial) => {
    set((state) => ({
      activeProject: { ...state.activeProject, ...partial },
    }));
  },

  openFolder: async (folderPath: string) => {
    if (!folderPath || !folderPath.trim()) return;
    const cleanPath = folderPath.trim();
    const folderName = cleanPath.split(/[\\/]/).filter(Boolean).pop() || 'Project';
    const slug = folderName.toLowerCase().replace(/[^a-z0-9]+/g, '-');

    const newProject: ProjectInfo = {
      id: `proj-${Date.now()}-${slug}`,
      name: folderName,
      rootPath: cleanPath,
      activeBranch: 'main',
      worktreesCount: 0,
    };

    // Update backend if in Tauri
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('set_project_root', { path: cleanPath });
    } catch {
      // Ignore if web environment
    }

    set((state) => {
      const exists = state.availableProjects.find(
        (p) => p.rootPath.toLowerCase() === cleanPath.toLowerCase()
      );
      if (exists) {
        return { activeProject: exists };
      }
      return {
        activeProject: newProject,
        availableProjects: [...state.availableProjects, newProject],
      };
    });
  },
}));
