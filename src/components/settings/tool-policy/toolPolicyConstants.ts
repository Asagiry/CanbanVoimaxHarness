import type { TranslationKey } from '../../../stores/languageStore';

export interface CustomTool {
  name: string;
  description: string;
  parameters: Record<string, unknown>;
  command: string;
}

export interface AgentMatrixItem {
  role: string;
  icon?: string;
  color?: string;
  isBuiltin: boolean;
}

export interface StageColumnRef {
  id: string;
  persona?: string | null;
  icon?: string | null;
  color?: string | null;
}

export const NAME_PATTERN = /^[a-z0-9_]+$/;

export const CATEGORY_ORDER: Array<
  'orchestration' | 'interactive' | 'workflow' | 'system' | 'fs' | 'web' | 'agent'
> = ['orchestration', 'interactive', 'workflow', 'system', 'fs', 'web', 'agent'];

export const getCategoryLabel = (catKey: string, t: (key: TranslationKey) => string) => {
  const catTransKey = `toolPolicy.category.${catKey}` as TranslationKey;
  const label = t(catTransKey);
  return label !== catTransKey ? label : catKey;
};

export function buildAgentMatrix(
  agentPersonas: ReadonlyArray<{
    role?: string | null;
    icon?: string | null;
    color?: string | null;
    isBuiltin?: boolean | null;
  }>,
  columns: ReadonlyArray<StageColumnRef>,
): AgentMatrixItem[] {
  const map = new Map<string, AgentMatrixItem>();

  for (const p of agentPersonas) {
    if (p.role) {
      map.set(p.role, {
        role: p.role,
        icon: p.icon ?? undefined,
        color: p.color ?? undefined,
        isBuiltin: p.role === 'cvh-dev' || Boolean(p.isBuiltin),
      });
    }
  }

  for (const col of columns) {
    if (col.persona && !map.has(col.persona)) {
      map.set(col.persona, {
        role: col.persona,
        icon: col.icon ?? undefined,
        color: col.color ?? undefined,
        isBuiltin: col.persona === 'cvh-dev',
      });
    }
  }

  if (map.size === 0) {
    map.set('cvh-dev', { role: 'cvh-dev', icon: 'Cpu', isBuiltin: true });
  }

  return Array.from(map.values()).sort((a, b) => {
    if (a.isBuiltin && !b.isBuiltin) return -1;
    if (!a.isBuiltin && b.isBuiltin) return 1;
    return a.role.localeCompare(b.role);
  });
}

export function applyAgentToolToggle(
  agentRole: string,
  toolId: string,
  enabled: boolean,
  columns: ReadonlyArray<StageColumnRef>,
  setStageTool: (stage: string, toolId: string, enabled: boolean) => void,
) {
  setStageTool(agentRole, toolId, enabled);
  for (const col of columns) {
    if (col.persona === agentRole && col.id !== agentRole) {
      setStageTool(col.id, toolId, enabled);
    }
  }
}

export async function fetchCustomToolsIpc(): Promise<CustomTool[]> {
  const { invoke } = await import('@tauri-apps/api/core');
  const list = await invoke<CustomTool[]>('list_custom_tools');
  return Array.isArray(list) ? list : [];
}

export async function saveCustomToolIpc(payload: CustomTool): Promise<void> {
  const { invoke } = await import('@tauri-apps/api/core');
  await invoke<string>('save_custom_tool', { ...payload });
}

export async function deleteCustomToolIpc(name: string): Promise<void> {
  const { invoke } = await import('@tauri-apps/api/core');
  await invoke('delete_custom_tool', { name });
}
