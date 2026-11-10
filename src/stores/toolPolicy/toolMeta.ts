import { ToolMeta } from './toolPolicyTypes';
import { BUILTIN_TOOLS } from './builtinTools';
import { AGENT_TOOLS } from './agentTools';

export type { ToolMeta };

export function getToolTranslationKey(toolId: string, field: 'label' | 'description'): string {
  return `tools.${toolId}.${field}`;
}

export const AVAILABLE_TOOLS: ToolMeta[] = [
  ...BUILTIN_TOOLS,
  ...AGENT_TOOLS,
];
