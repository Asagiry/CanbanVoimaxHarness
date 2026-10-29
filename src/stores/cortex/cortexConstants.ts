import { CortexAgentStage } from '../../types';

export const DEFAULT_CORTEX_STAGES: CortexAgentStage[] = [
  {
    stageId: 'backlog',
    stageTitle: 'Backlog',
    color: '#64748b',
    icon: 'Inbox',
    personaName: 'система',
    isExpanded: false,
    files: [],
  },
  {
    stageId: 'dev',
    stageTitle: 'CVH Development',
    color: '#6366f1',
    icon: 'Bot',
    personaName: 'cvh-dev',
    isExpanded: true,
    files: [
      {
        id: 'dev-persona',
        name: 'persona.md',
        stageId: 'dev',
        type: 'persona',
        path: '.cvh/agents/cvh-dev/persona.md',
        content: '# Role: CVH-DEV (Project Configuration & Environment Specialist)\n\nНастройка проекта, правил и архитектуры CanbanVoimaxHarness (.cvh/).',
      },
      {
        id: 'dev-settings',
        name: 'setting',
        stageId: 'dev',
        type: 'settings',
        path: '.cvh/agents/cvh-dev/setting',
        content: '{\n  "model": "gemini-3.8-flash-high",\n  "context_window": 200000,\n  "temperature": 0.2,\n  "max_tokens": 8192\n}',
      },
      {
        id: 'dev-rule-tools',
        name: 'tools.md',
        stageId: 'dev',
        type: 'rule',
        path: '.cvh/agents/cvh-dev/rules/tools.md',
        content: '# Guide: Tool Policies\n\nДвухуровневая матрица политик инструментов (global + stageOverrides).',
      },
      {
        id: 'dev-rule-boards',
        name: 'boards.md',
        stageId: 'dev',
        type: 'rule',
        path: '.cvh/agents/cvh-dev/rules/boards.md',
        content: '# Guide: Boards & Pipelines\n\nИнварианты структуры досок и автономных агентов.',
      },
    ],
  },
  {
    stageId: 'ready',
    stageTitle: 'Ready',
    color: '#10b981',
    icon: 'CheckCircle2',
    personaName: 'система',
    isExpanded: false,
    files: [
      {
        id: 'ready-hook-on_finish',
        name: 'on_finish',
        stageId: 'ready',
        type: 'hook',
        path: '.cvh/hooks/on_finish_done.bat',
        content: '@echo off\necho Stage transition to ready completed.\n',
      },
    ],
  },
];

export const SCRIPT_EXTENSIONS = new Set([
  'bat',
  'cmd',
  'ps1',
  'sh',
  'bash',
  'py',
  'js',
  'ts',
  'mjs',
  'cjs',
  'rb',
  'pl',
]);
