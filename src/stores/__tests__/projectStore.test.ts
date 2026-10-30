import { describe, it, expect, beforeEach, vi } from 'vitest';
import { useProjectStore } from '../projectStore';
import { useCortexStore } from '../cortexStore';
import { useBoardStore } from '../boardStore';
import { useDrawerStore } from '../drawerStore';

const mockInvoke = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

describe('projectStore - Global Rules Sync on Project Switching', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockResolvedValue(null);
  });

  it('switchProject invokes loadGlobalRules and closeEditor on useCortexStore', async () => {
    const loadGlobalRulesSpy = vi.spyOn(useCortexStore.getState(), 'loadGlobalRules').mockResolvedValue();
    const closeEditorSpy = vi.spyOn(useCortexStore.getState(), 'closeEditor');
    const closeDrawerSpy = vi.spyOn(useDrawerStore.getState(), 'closeDrawer');
    vi.spyOn(useBoardStore.getState(), 'loadBoards').mockResolvedValue();
    vi.spyOn(useBoardStore.getState(), 'loadTasks').mockResolvedValue();
    vi.spyOn(useCortexStore.getState(), 'loadAgentPersonas').mockResolvedValue();

    useProjectStore.setState({
      availableProjects: [
        {
          id: 'proj-1',
          name: 'Project 1',
          rootPath: '/path/to/proj1',
          activeBranch: 'main',
          worktreesCount: 0,
        },
        {
          id: 'proj-2',
          name: 'Project 2',
          rootPath: '/path/to/proj2',
          activeBranch: 'main',
          worktreesCount: 0,
        },
      ],
      activeProject: {
        id: 'proj-1',
        name: 'Project 1',
        rootPath: '/path/to/proj1',
        activeBranch: 'main',
        worktreesCount: 0,
      },
    });

    await useProjectStore.getState().switchProject('proj-2');

    expect(closeDrawerSpy).toHaveBeenCalled();
    expect(closeEditorSpy).toHaveBeenCalled();
    expect(loadGlobalRulesSpy).toHaveBeenCalled();
    expect(useProjectStore.getState().activeProject.id).toBe('proj-2');
  });

  it('setActiveProjectByPath invokes loadGlobalRules and closeEditor on useCortexStore', async () => {
    const loadGlobalRulesSpy = vi.spyOn(useCortexStore.getState(), 'loadGlobalRules').mockResolvedValue();
    const closeEditorSpy = vi.spyOn(useCortexStore.getState(), 'closeEditor');
    vi.spyOn(useBoardStore.getState(), 'loadBoards').mockResolvedValue();
    vi.spyOn(useBoardStore.getState(), 'loadTasks').mockResolvedValue();
    vi.spyOn(useCortexStore.getState(), 'loadAgentPersonas').mockResolvedValue();

    await useProjectStore.getState().setActiveProjectByPath('/path/to/carsmash');

    expect(closeEditorSpy).toHaveBeenCalled();
    expect(loadGlobalRulesSpy).toHaveBeenCalled();
    expect(useProjectStore.getState().activeProject.rootPath).toBe('/path/to/carsmash');
  });

  it('removeProjectFromHistory switches project and invokes loadGlobalRules and closeEditor', async () => {
    const loadGlobalRulesSpy = vi.spyOn(useCortexStore.getState(), 'loadGlobalRules').mockResolvedValue();
    const closeEditorSpy = vi.spyOn(useCortexStore.getState(), 'closeEditor');
    vi.spyOn(useBoardStore.getState(), 'loadBoards').mockResolvedValue();
    vi.spyOn(useBoardStore.getState(), 'loadTasks').mockResolvedValue();
    vi.spyOn(useCortexStore.getState(), 'loadAgentPersonas').mockResolvedValue();

    useProjectStore.setState({
      availableProjects: [
        {
          id: 'proj-active',
          name: 'Project Active',
          rootPath: '/path/to/active',
          activeBranch: 'main',
          worktreesCount: 0,
        },
        {
          id: 'proj-next',
          name: 'Project Next',
          rootPath: '/path/to/next',
          activeBranch: 'main',
          worktreesCount: 0,
        },
      ],
      activeProject: {
        id: 'proj-active',
        name: 'Project Active',
        rootPath: '/path/to/active',
        activeBranch: 'main',
        worktreesCount: 0,
      },
      hasActiveProject: true,
    });

    await useProjectStore.getState().removeProjectFromHistory('proj-active');

    expect(closeEditorSpy).toHaveBeenCalled();
    expect(loadGlobalRulesSpy).toHaveBeenCalled();
    expect(useProjectStore.getState().activeProject.id).toBe('proj-next');
  });
});
