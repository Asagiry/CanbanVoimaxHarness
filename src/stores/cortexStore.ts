import { create } from 'zustand';
import { CortexAgentFile, CortexAgentStage } from '../types';
import { INITIAL_CORTEX_STAGES } from './mockData';

interface CortexState {
  stages: CortexAgentStage[];
  activeFile: CortexAgentFile | null;
  focusedStageId: string | null;
  isEditorOpen: boolean;
  isSaving: boolean;
  saveFeedback: string | null;

  // Actions
  toggleStageExpand: (stageId: string) => void;
  focusStage: (stageId: string) => void;
  openFile: (fileId: string) => void;
  updateActiveFileContent: (content: string) => void;
  saveRules: () => Promise<void>;
  closeEditor: () => void;
}

export const useCortexStore = create<CortexState>((set, get) => ({
  stages: INITIAL_CORTEX_STAGES,
  activeFile: null,
  focusedStageId: null,
  isEditorOpen: false,
  isSaving: false,
  saveFeedback: null,

  toggleStageExpand: (stageId) => {
    set((state) => ({
      stages: state.stages.map((st) =>
        st.stageId === stageId ? { ...st, isExpanded: !st.isExpanded } : st
      ),
    }));
  },

  focusStage: (stageId) => {
    set((state) => ({
      focusedStageId: stageId,
      stages: state.stages.map((st) =>
        st.stageId === stageId ? { ...st, isExpanded: true } : st
      ),
    }));

    // Reset focused highlight after 2.5s
    setTimeout(() => {
      if (get().focusedStageId === stageId) {
        set({ focusedStageId: null });
      }
    }, 2500);
  },

  openFile: (fileId) => {
    const { stages } = get();
    for (const stage of stages) {
      const file = stage.files.find((f) => f.id === fileId);
      if (file) {
        set({
          activeFile: { ...file },
          isEditorOpen: true,
          saveFeedback: null,
        });
        return;
      }
    }
  },

  updateActiveFileContent: (content) => {
    set((state) => {
      if (!state.activeFile) return state;
      return {
        activeFile: {
          ...state.activeFile,
          content,
          isDirty: true,
        },
      };
    });
  },

  saveRules: async () => {
    const { activeFile, stages } = get();
    if (!activeFile) return;

    set({ isSaving: true });

    // Emulate / perform disk save
    await new Promise((resolve) => setTimeout(resolve, 350));

    // Update in memory stages
    const updatedStages = stages.map((stage) => {
      if (stage.stageId !== activeFile.stageId) return stage;
      return {
        ...stage,
        files: stage.files.map((file) =>
          file.id === activeFile.id
            ? { ...file, content: activeFile.content, isDirty: false }
            : file
        ),
      };
    });

    set({
      stages: updatedStages,
      activeFile: { ...activeFile, isDirty: false },
      isSaving: false,
      saveFeedback: `Saved ${activeFile.name} successfully`,
    });

    setTimeout(() => {
      set({ saveFeedback: null });
    }, 2800);
  },

  closeEditor: () => {
    set({
      isEditorOpen: false,
      activeFile: null,
      saveFeedback: null,
    });
  },
}));
