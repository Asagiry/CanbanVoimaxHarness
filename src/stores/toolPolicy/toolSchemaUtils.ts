import { ToolMeta } from './toolPolicyTypes';

export type ExecuteShell = 'bash' | 'pwsh' | 'cmd';

const SHELL_SCHEMA_META: Record<
  ExecuteShell,
  { aliases: string[]; description: string; commandDescription: string }
> = {
  bash: {
    aliases: ['bash'],
    description:
      'Execute a shell command inside the isolated worktree via Git Bash with piped stdout/stderr.',
    commandDescription: "Command string to run in Git Bash (e.g. 'cargo check', 'git status').",
  },
  pwsh: {
    aliases: ['pwsh', 'powershell'],
    description:
      'Execute a shell command inside the isolated worktree via PowerShell with piped stdout/stderr.',
    commandDescription:
      "Command string to run in PowerShell (e.g. 'cargo check', 'Get-ChildItem').",
  },
  cmd: {
    aliases: ['cmd'],
    description:
      'Execute a shell command inside the isolated worktree via Command Prompt (cmd.exe) with piped stdout/stderr.',
    commandDescription:
      "Command string to run in Command Prompt (e.g. 'cargo check', 'dir').",
  },
};

export function applyShellToExecuteCommandSchema(
  schema: Record<string, unknown>,
  shell: ExecuteShell,
): Record<string, unknown> {
  const fn = schema.function as Record<string, unknown> | undefined;
  if (!fn || typeof fn !== 'object') return schema;
  const meta = SHELL_SCHEMA_META[shell];
  const params = (fn.parameters as Record<string, unknown> | undefined) ?? {};
  const props = (params.properties as Record<string, unknown> | undefined) ?? {};
  const commandProp = (props.command as Record<string, unknown> | undefined) ?? { type: 'string' };

  return {
    ...schema,
    function: {
      ...fn,
      aliases: meta.aliases,
      description: meta.description,
      parameters: {
        ...params,
        properties: {
          ...props,
          command: {
            ...commandProp,
            description: meta.commandDescription,
          },
        },
      },
    },
  };
}

export function buildDefaultToolSchema(
  tool: ToolMeta,
  shell: ExecuteShell = 'bash',
): Record<string, unknown> {
  const base: Record<string, unknown> = {
    type: 'function',
    function: {
      name: tool.name,
      description: tool.description,
      ...(tool.aliases && tool.aliases.length > 0 ? { aliases: tool.aliases } : {}),
      parameters: tool.parameters || { type: 'object', properties: {} },
    },
  };
  return tool.id === 'execute_command' ? applyShellToExecuteCommandSchema(base, shell) : base;
}

export function normalizeStoredToolSchema(
  stored: unknown,
  defaultSchema: Record<string, unknown>,
): unknown {
  try {
    const parsed =
      typeof stored === 'string'
        ? JSON.parse(stored)
        : stored && typeof stored === 'object' && !Array.isArray(stored)
          ? { ...(stored as Record<string, unknown>) }
          : stored;
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) return parsed;

    const record = parsed as Record<string, unknown>;
    const fn = record.function as Record<string, unknown> | undefined;
    const defFn = defaultSchema.function as Record<string, unknown> | undefined;
    const defAliases = defFn?.aliases as string[] | undefined;
    if (fn && typeof fn === 'object' && !fn.aliases && defAliases?.length) {
      return {
        ...record,
        function: {
          ...fn,
          aliases: defAliases,
        },
      };
    }
    return record;
  } catch {
    return stored;
  }
}

export function stableSchemaEqual(a: unknown, b: unknown): boolean {
  if (Object.is(a, b)) return true;
  if (typeof a !== typeof b || a === null || b === null) return false;
  if (Array.isArray(a) || Array.isArray(b)) {
    if (!Array.isArray(a) || !Array.isArray(b) || a.length !== b.length) return false;
    return a.every((item, i) => stableSchemaEqual(item, b[i]));
  }
  if (typeof a === 'object' && typeof b === 'object') {
    const recA = a as Record<string, unknown>;
    const recB = b as Record<string, unknown>;
    const keysA = Object.keys(recA).filter((k) => recA[k] !== undefined);
    const keysB = Object.keys(recB).filter((k) => recB[k] !== undefined);
    if (keysA.length !== keysB.length) return false;
    return keysA.every(
      (k) => Object.prototype.hasOwnProperty.call(recB, k) && stableSchemaEqual(recA[k], recB[k]),
    );
  }
  return false;
}

export function isToolSchemaCustomized(
  tool: ToolMeta,
  storedSchema: unknown,
  shell: ExecuteShell = 'bash',
): boolean {
  if (storedSchema === undefined) return false;
  const defaultSchema = buildDefaultToolSchema(tool, shell);
  const normalized = normalizeStoredToolSchema(storedSchema, defaultSchema);
  return !stableSchemaEqual(normalized, defaultSchema);
}
