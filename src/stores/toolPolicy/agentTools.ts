import { ToolMeta } from './toolPolicyTypes';

export const AGENT_TOOLS: ToolMeta[] = [
  {
    id: 'subagent',
    name: 'subagent',
    label: 'Вызов субагентов',
    description: 'Delegate a focused, independent sub-task to a specialized subagent synchronously (Oneshot). Blocks until completion and returns the final report directly. Subagents run in-process inside the current directory without moving cards or creating worktrees.',
    category: 'agent',
    tokenWeight: 310,
    isSubagentMaster: true,
    parameters: {
      type: 'object',
      properties: {
        title: { type: 'string', description: "Short task description (e.g. 'Analyze project architecture', 'Find memory leak')." },
        role: { type: 'string', description: "Subagent role persona (e.g. 'general', 'developer', 'reviewer', 'explorer'). Default: 'general'." },
        prompt: { type: 'string', description: 'Detailed task instructions.' },
        tools: {
          type: 'array',
          items: { type: 'string' },
          description: 'Optional list of tools allowed for this subagent. If omitted, defaults to ALL tools available to the parent agent.',
        },
      },
      required: ['title', 'prompt'],
    },
  },
  {
    id: 'subagent_sync',
    name: 'subagent',
    label: 'Субагент (синхронный)',
    description: 'Delegate a focused sub-task to a subagent synchronously (Oneshot). Blocks until completion and returns the final report.',
    category: 'agent',
    tokenWeight: 290,
    parentTool: 'subagent',
    parameters: {
      type: 'object',
      properties: {
        title: { type: 'string', description: 'Short task description.' },
        role: { type: 'string', description: "Subagent role persona. Default: 'general'." },
        prompt: { type: 'string', description: 'Detailed task instructions.' },
        tools: {
          type: 'array',
          items: { type: 'string' },
          description: 'Optional list of tools allowed for this subagent.',
        },
      },
      required: ['title', 'prompt'],
    },
  },
  {
    id: 'subagent_bg',
    name: 'subagent_bg',
    label: 'Субагент (фоновый)',
    description: "Launch a specialized subagent in the background (asynchronous). Returns immediately with a subagent_id and status 'running'. Use collect_subagent with subagent_id to retrieve the report when needed.",
    category: 'agent',
    tokenWeight: 290,
    parentTool: 'subagent',
    parameters: {
      type: 'object',
      properties: {
        title: { type: 'string', description: "Short task description (e.g. 'Run long benchmark', 'Deep search repo')." },
        role: { type: 'string', description: "Subagent role persona (e.g. 'general', 'developer', 'reviewer', 'explorer'). Default: 'general'." },
        prompt: { type: 'string', description: 'Detailed task instructions.' },
        tools: {
          type: 'array',
          items: { type: 'string' },
          description: 'Optional list of tools allowed for this subagent. If omitted, defaults to ALL tools available to the parent agent.',
        },
      },
      required: ['title', 'prompt'],
    },
  },
  {
    id: 'collect_subagent',
    name: 'collect_subagent',
    label: 'Собрать субагентов',
    description: "Await the completion of one or multiple background subagents (launched via subagent_bg) and retrieve their final reports without polling. Accepts either 'subagent_id' (string) or 'subagent_ids' (array of strings).",
    category: 'agent',
    tokenWeight: 260,
    parentTool: 'subagent',
    parameters: {
      type: 'object',
      properties: {
        subagent_id: { type: 'string', description: 'Single subagent_id to await.' },
        subagent_ids: {
          type: 'array',
          items: { type: 'string' },
          description: 'List of subagent_ids to await.',
        },
      },
    },
  },
  {
    id: 'subagent_list',
    name: 'subagent_list',
    label: 'Список субагентов',
    description: 'List all background and synchronous subagents for the current task with their status (running, completed, aborted), role, title, steps, and subagent_id for collection.',
    category: 'agent',
    tokenWeight: 200,
    parentTool: 'subagent',
    aliases: ['list_subagents'],
    parameters: {
      type: 'object',
      properties: {
        status_filter: {
          type: 'string',
          enum: ['all', 'running', 'completed', 'aborted'],
          description: "Optional filter by status: 'all', 'running', 'completed', 'aborted'.",
        },
      },
    },
  },
  {
    id: 'todo_write',
    name: 'todo_write',
    label: 'План выполнения',
    description: 'Record and update task plan list to show progress (max 20 items). Add todos with status pending | in_progress | completed.',
    category: 'agent',
    tokenWeight: 280,
    parameters: {
      type: 'object',
      properties: {
        todos: {
          type: 'array',
          maxItems: 20,
          items: {
            type: 'object',
            properties: {
              content: { type: 'string' },
              status: { type: 'string', enum: ['pending', 'in_progress', 'completed'] },
            },
            required: ['content', 'status'],
          },
        },
      },
      required: ['todos'],
    },
  },
];
