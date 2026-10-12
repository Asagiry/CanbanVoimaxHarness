import { create } from 'zustand';
import { ProjectInfo } from '../types';

interface ProjectState {
  activeProject: ProjectInfo;
  availableProjects: ProjectInfo[];
  isTauriEnvironment: boolean;

  // Actions
  switchProject: (projectId: string) => void;
  updateActiveProject: (partial: Partial<ProjectInfo>) => void;
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

export const useProjectStore = create<ProjectState>((set) => ({
  activeProject: DEFAULT_PROJECT,
  availableProjects: SAMPLE_PROJECTS,
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
}));
