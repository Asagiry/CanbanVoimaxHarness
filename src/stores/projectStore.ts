import { create } from 'zustand';
import { ProjectInfo } from '../types';
import { useBoardStore } from './boardStore';
import { useCortexStore } from './cortexStore';
import { useAgentSessionStore } from './agentSessionStore';
import { logEvent } from '../lib/logger';
import {
  DEFAULT_PROJECT,
  isInvalidProjectPath,
  loadStoredProjects,
  mapBackendProject,
  persistStoredProjects,
  resetWorkspaceStoresOnProjectChange,
  type RawBackendProjectDTO,
} from './project/projectHelpers';

export { isInvalidProjectPath } from './project/projectHelpers';

interface ProjectState {
  activeProject: ProjectInfo;
  availableProjects: ProjectInfo[];
  isTauriEnvironment: boolean;
  hasActiveProject: boolean;
  projectError: string | null;

  switchProject: (projectId: string) => Promise<void>;
  updateActiveProject: (partial: Partial<ProjectInfo>) => void;
  loadProjects: () => Promise<void>;
  selectProjectFolder: () => Promise<string | null>;
  openFolder: (path: string) => Promise<void>;
  setActiveProjectByPath: (path: string) => Promise<void>;
  removeProjectFromHistory: (projectId: string) => Promise<void>;
  clearProjectError: () => void;
}

const initialProjects = loadStoredProjects();
const initialCandidate = initialProjects.find((p) => p && !isInvalidProjectPath(p.rootPath));

export async function performProjectSwitch(targetProject: ProjectInfo): Promise<void> {
  const cleanPath = targetProject.rootPath?.trim();
  if (!cleanPath || isInvalidProjectPath(cleanPath)) return;

  const prevProject = useProjectStore.getState().activeProject;
  const prevHasActive = useProjectStore.getState().hasActiveProject;

  logEvent('INFO', 'PROJECT_STORE', `performProjectSwitch: switching to project "${targetProject.name}"`, cleanPath);

  resetWorkspaceStoresOnProjectChange(true);
  useProjectStore.setState({
    activeProject: targetProject,
    hasActiveProject: true,
    projectError: null,
  });

  try {
    const { invoke } = await import('@tauri-apps/api/core');
    try {
      await invoke('switch_project', { projectPath: cleanPath });
    } catch (switchErr) {
      try {
        await invoke('set_active_project', { path: cleanPath });
      } catch (fallbackErr) {
        const detail = fallbackErr instanceof Error ? fallbackErr.message : String(fallbackErr ?? switchErr);
        throw new Error(detail);
      }
    }
  } catch (err) {
    const errMsg = err instanceof Error ? err.message : String(err);
    console.error('Failed to switch project on backend:', err);
    logEvent('ERROR', 'PROJECT_STORE', `performProjectSwitch failed for "${cleanPath}"`, errMsg);
    useProjectStore.setState({
      activeProject: prevProject,
      hasActiveProject: prevHasActive,
      projectError: errMsg,
    });
    useBoardStore.setState({ isLoadingTasks: false, isLoadingBoards: false });
    return;
  }

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
    } catch (policyErr) {
      console.warn('Tool policy load during project switch failed:', policyErr);
    }

    await useAgentSessionStore.getState().syncActiveSessions();
  } catch (err) {
    const errMsg = err instanceof Error ? err.message : String(err);
    console.error('Error reloading project entities:', err);
    useProjectStore.setState({ projectError: errMsg });
  } finally {
    useBoardStore.setState({ isLoadingTasks: false, isLoadingBoards: false });
  }
}

export const useProjectStore = create<ProjectState>((set, get) => ({
  activeProject: initialCandidate || DEFAULT_PROJECT,
  availableProjects: initialProjects,
  isTauriEnvironment: true,
  hasActiveProject: false,
  projectError: null,

  clearProjectError: () => set({ projectError: null }),

  switchProject: async (projectId: string) => {
    const found = get().availableProjects.find(
      (p) => (p.id === projectId || p.rootPath.toLowerCase() === projectId.toLowerCase()) && !isInvalidProjectPath(p.rootPath),
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
    let loadError: string | null = null;

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      try {
        const activeRaw = await invoke<RawBackendProjectDTO | null>('get_active_project');
        if (activeRaw && activeRaw.hasProject !== false && (activeRaw.rootPath || activeRaw.path)) {
          backendActive = mapBackendProject(activeRaw, true);
        }
      } catch (err) {
        loadError = err instanceof Error ? err.message : String(err);
        console.warn('get_active_project IPC failed:', err);
      }

      try {
        const listRaw = await invoke<RawBackendProjectDTO[]>('list_projects');
        if (Array.isArray(listRaw)) {
          backendProjects = listRaw
            .map((p) => mapBackendProject(p))
            .filter((p): p is ProjectInfo => Boolean(p));
        }
      } catch (err) {
        loadError = loadError || (err instanceof Error ? err.message : String(err));
        console.warn('list_projects IPC failed:', err);
      }
    } catch (err) {
      loadError = err instanceof Error ? err.message : String(err);
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
      projectError: loadError,
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
      const errMsg = err instanceof Error ? err.message : String(err);
      console.error('Failed to select project folder:', err);
      set({ projectError: errMsg });
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

    const proj: ProjectInfo = {
      id,
      name,
      rootPath: cleanPath,
      activeBranch: 'main',
      worktreesCount: 0,
      hasProject: true,
    };

    const exists = state.availableProjects.some(
      (p) => p.id === id || p.rootPath.toLowerCase() === cleanPath.toLowerCase(),
    );
    const updatedProjects = exists
      ? state.availableProjects.map((p) =>
          p.id === id || p.rootPath.toLowerCase() === cleanPath.toLowerCase() ? proj : p,
        )
      : [...state.availableProjects.filter((p) => !isInvalidProjectPath(p.rootPath)), proj];

    set({ availableProjects: updatedProjects, projectError: null });
    persistStoredProjects(updatedProjects);

    // Switch backend project root BEFORE querying get_project_config_cmd for the new project.
    await performProjectSwitch(proj);
    if (get().projectError) return;

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const cfg = await invoke<{ name?: string; version?: string } | null>('get_project_config_cmd');
      if (cfg?.name && cfg.name.trim() && cfg.name !== proj.name) {
        const renamed: ProjectInfo = { ...proj, name: cfg.name.trim() };
        const renamedList = get().availableProjects.map((p) =>
          p.rootPath.toLowerCase() === cleanPath.toLowerCase() ? renamed : p,
        );
        set({ activeProject: renamed, availableProjects: renamedList });
        persistStoredProjects(renamedList);
      }
    } catch (err) {
      console.warn('get_project_config_cmd failed after switch:', err);
    }
  },

  removeProjectFromHistory: async (projectId: string) => {
    const state = get();
    const cleanId = projectId.trim();
    if (!cleanId) return;

    const targetProject = state.availableProjects.find(
      (p) => p.id === cleanId || p.rootPath.toLowerCase() === cleanId.toLowerCase(),
    );
    const updatedProjects = state.availableProjects.filter(
      (p) => p.id !== cleanId && p.rootPath.toLowerCase() !== cleanId.toLowerCase(),
    );
    persistStoredProjects(updatedProjects);

    const removedRootPath =
      targetProject?.rootPath?.trim() || (cleanId.includes('/') || cleanId.includes('\\') ? cleanId : '');
    if (removedRootPath && !isInvalidProjectPath(removedRootPath)) {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('remove_project', { path: removedRootPath });
      } catch (err) {
        const errMsg = err instanceof Error ? err.message : String(err);
        logEvent('WARN', 'PROJECT_STORE', `removeProjectFromHistory: backend remove_project failed: ${errMsg}`, removedRootPath);
        set({ projectError: errMsg });
      }
    }

    const isCurrentActive =
      state.activeProject.id === cleanId ||
      state.activeProject.rootPath.toLowerCase() === cleanId.toLowerCase() ||
      Boolean(targetProject && state.activeProject.rootPath.toLowerCase() === targetProject.rootPath.toLowerCase());

    if (isCurrentActive) {
      const nextProject = updatedProjects[0];
      if (nextProject) {
        logEvent('INFO', 'PROJECT_STORE', `removeProjectFromHistory: active project removed, switching to "${nextProject.name}"`);
        set({ availableProjects: updatedProjects });
        await performProjectSwitch(nextProject);
      } else {
        logEvent('INFO', 'PROJECT_STORE', 'removeProjectFromHistory: last active project removed, resetting workspace');
        resetWorkspaceStoresOnProjectChange(false);
        set({
          availableProjects: [],
          activeProject: DEFAULT_PROJECT,
          hasActiveProject: false,
        });
      }
    } else {
      set({ availableProjects: updatedProjects });
    }
  },
}));
