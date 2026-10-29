import { create } from 'zustand';
import {
  CortexState,
  createCortexFilesSlice,
  createPersonasSlice,
  createRulesSlice,
  registerCortexStore,
} from './cortex';

// Ub-mode Re-exports for zero breaking changes across codebase
export {
  DEFAULT_CORTEX_STAGES,
  SCRIPT_EXTENSIONS,
} from './cortex/cortexConstants';

export {
  cortexAgentDir,
  buildRuleTemplate,
  buildHookTemplate,
  buildSettingTemplate,
  tokenizeCommand,
  extractScriptPath,
  buildBuiltinHookDescription,
  buildCommandHookDescription,
  buildScriptTemplate,
  buildHookFileName,
  previewCortexFileName,
  normalizeCortexFileName,
  buildPersonaTemplate,
} from './cortex/cortexHelpers';

export {
  initCortexEventListeners,
  registerCortexStore,
} from './cortex/cortexListeners';

export type {
  CortexHookMoment,
  CortexHookExt,
  AgentPersonaInfo,
  RawAgentPersona,
  PersonasState,
  PersonasActions,
  PersonasSlice,
  RulesState,
  RulesActions,
  RulesSlice,
  CortexFilesState,
  CortexFilesActions,
  CortexFilesSlice,
  CortexState,
} from './cortex/types';

export const useCortexStore = create<CortexState>()((...a) => ({
  ...createCortexFilesSlice(...a),
  ...createPersonasSlice(...a),
  ...createRulesSlice(...a),
}));

registerCortexStore(useCortexStore);
