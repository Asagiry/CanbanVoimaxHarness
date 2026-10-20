import { create } from 'zustand';
import { CortexAgentFile, CortexAgentStage, PipelineColumn } from '../types';
import { useBoardStore } from './boardStore';
import { getDefaultColumnIcon } from '../lib/columnIcons';
import {
  BACKLOG_COLOR,
  READY_COLOR,
  getDeterministicAgentColor,
  isAllowedAgentColor,
} from '../lib/agentPalette';

const DEFAULT_CORTEX_STAGES: CortexAgentStage[] = [
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

interface CortexState {
  stages: CortexAgentStage[];
  // Секция «Агенты»: стадии всех доступных персон (`list_agent_personas`),
  // независимые от колонок доски. Файлы правил/хуков здесь — только
  // созданные через createCortexFile в текущей сессии (листинга директорий нет).
  agentStages: CortexAgentStage[];
  agentPersonas: AgentPersonaInfo[];
  activeFile: CortexAgentFile | null;
  focusedStageId: string | null;
  isEditorOpen: boolean;
  isSaving: boolean;
  saveFeedback: string | null;
  // Файлы, отсутствующие на диске (read_cortex_file завершился ошибкой), по id файла.
  // Покрывает одиночные узлы новой схемы: `agent.md` и `setting`.
  // Списки rules/ и hooks/ без листинга директорий неизвестны и сюда не попадают:
  // стор показывает только файлы, созданные через createCortexFile в текущей сессии.
  fileMissing: Record<string, boolean>;

  // Actions
  toggleStageExpand: (stageId: string) => void;
  focusStage: (stageId: string) => void;
  // Перестроить стадии из колонок доски: колонка с persona = агент
  syncStagesFromColumns: (columns: PipelineColumn[]) => void;
  // Загрузить доступных агентов (`list_agent_personas`) и перестроить секцию «Агенты».
  // Ошибка IPC → пустой список, секция показывает подсказку. Колонки не трогает.
  loadAgentPersonas: () => Promise<void>;
  renameAgentPersona: (oldRole: string, newRole: string) => Promise<void>;
  deleteAgentPersona: (role: string) => Promise<void>;
  setAgentPersonaColor: (role: string, color: string) => Promise<void>;
  setAgentPersonaIcon: (role: string, icon: string) => Promise<void>;
  openFile: (fileId: string) => Promise<void>;
  createPersonaTemplate: (fileId: string) => Promise<void>;
  createSettingTemplate: (fileId: string) => Promise<void>;
  // Создать новый файл в стадии по НОВОЙ схеме `.cvh/agents/<role>/...`:
  // правило -> `rules/<имя>.md`, хук -> `hooks/<момент>[-<имя>].<расширение>`.
  // <role> = persona колонки (fallback — stageId). Родительские каталоги
  // создаёт бэкенд (save_cortex_file делает create_dir_all).
  // При ошибке IPC бросает Error с русским текстом (форма показывает его красным).
  createCortexFile: (
    stageId: string,
    rawName: string,
    kind: 'rule' | 'hook',
    opts?: { hookMoment?: CortexHookMoment; hookExt?: CortexHookExt },
  ) => Promise<void>;
  updateActiveFileContent: (content: string) => void;
  saveRules: () => Promise<void>;
  closeEditor: () => void;
}

export type CortexHookMoment = 'on_enter' | 'on_finish';
export type CortexHookExt = 'bat' | 'ps1' | 'py' | 'sh';

// Персона из бэкенда (`list_agent_personas` → `[{role, hasPrompt}]`,
// сериализация Rust — camelCase). Сырой ответ принимаем терпимо:
// учитываем и `has_prompt`, и отсутствие флага.
export interface AgentPersonaInfo {
  role: string;
  hasPrompt: boolean;
  isBuiltin?: boolean;
  personaFile?: string;
  rules?: string[];
  hooks?: string[];
  icon?: string;
  color?: string;
  contextWindow?: number;
}

interface RawAgentPersona {
  role: string;
  hasPrompt?: boolean;
  has_prompt?: boolean;
  isBuiltin?: boolean;
  is_builtin?: boolean;
  personaFile?: string;
  persona_file?: string;
  rules?: string[];
  hooks?: string[];
  icon?: string;
  color?: string;
  contextWindow?: number;
  context_window?: number;
}

// Каталог агента по новой схеме. Роль = persona колонки; фолбэк — stageId,
// чтобы стадии без персоны тоже получали детерминированный путь.
// Системный backlog-узел (personaName «система»/«system») — не роль:
// тоже фолбэк на stageId, иначе файлы ушли бы в `.cvh/agents/система`.
export function cortexAgentDir(personaName: string, stageId: string): string {
  const trimmed = (personaName ?? '').trim();
  const role =
    !trimmed ||
    trimmed === 'система' ||
    trimmed.toLowerCase() === 'system' ||
    trimmed === 'без агента' ||
    trimmed === 'колонка'
      ? stageId
      : trimmed;
  return `.cvh/agents/${role}`;
}

// Минимальный русский шаблон правила для `.cvh/agents/<role>/rules/`
function buildRuleTemplate(baseName: string, stageTitle: string): string {
  return (
    `# Правило: ${baseName} (${stageTitle})\n` +
    `\n` +
    `## Назначение\n` +
    `Опишите назначение правила.\n` +
    `\n` +
    `## Проверки\n` +
    `- Проверять результат реальными сенсорами перед сдачей.\n`
  );
}

// Минимальный шаблон хука: момент в имени файла, расширение задаёт интерпретатор.
function buildHookTemplate(
  moment: CortexHookMoment,
  ext: CortexHookExt,
  label: string,
  role: string,
  stageTitle: string,
): string {
  const head =
    moment === 'on_enter'
      ? `Хук входа задачи агента ${role} (${stageTitle})`
      : `Хук завершения задачи агента ${role} (${stageTitle})`;
  switch (ext) {
    case 'bat':
      return `@echo off\nREM ${head}: ${label}\nREM Добавьте команды проверки и завершения.\n`;
    case 'ps1':
      return `# ${head}: ${label}\n# Добавьте команды проверки и завершения.\n`;
    case 'py':
      return `#!/usr/bin/env python3\n# ${head}: ${label}\n# Добавьте проверки и завершите с кодом 0 при успехе.\n`;
    case 'sh':
      return `#!/bin/sh\n# ${head}: ${label}\n# Добавьте проверки и завершите с кодом 0 при успехе.\n`;
  }
}

// Минимальный русский шаблон настроек для `.cvh/agents/<role>/setting` (JSON без расширения)
function buildSettingTemplate(role: string, stageTitle: string): string {
  return (
    `{\n` +
    `  "_comment": "Настройки агента ${role} (${stageTitle})",\n` +
    `  "model": null,\n` +
    `  "temperature": 0.2,\n` +
    `  "context_window": 128000\n` +
    `}\n`
  );
}

// Токенизация shell-команд с учётом кавычек
function tokenizeCommand(cmd: string): string[] {
  const tokens: string[] = [];
  const regex = /[^\s"']+|"([^"]*)"|'([^']*)'/g;
  let match: RegExpExecArray | null;
  while ((match = regex.exec(cmd)) !== null) {
    if (match[1] !== undefined) tokens.push(match[1]);
    else if (match[2] !== undefined) tokens.push(match[2]);
    else tokens.push(match[0]);
  }
  return tokens;
}

const SCRIPT_EXTENSIONS = new Set(['bat', 'cmd', 'ps1', 'sh', 'bash', 'py', 'js', 'ts', 'mjs', 'cjs', 'rb', 'pl']);

// Извлечение пути к файлу скрипта из команды хука (напр. "python .cvh/hooks/gatekeeper.py", ".bat", ".ps1")
function extractScriptPath(cmd: string): string | null {
  if (!cmd || cmd.startsWith('builtin:')) return null;
  const trimmed = cmd.trim();
  const tokens = tokenizeCommand(trimmed);
  if (tokens.length === 0) return null;

  // 1. Проверяем токен с распознанным расширением скрипта
  for (const token of tokens) {
    const extMatch = token.match(/\.([a-zA-Z0-9_-]+)$/);
    if (extMatch && SCRIPT_EXTENSIONS.has(extMatch[1].toLowerCase())) {
      return token;
    }
  }

  // 2. Проверяем токен с разделителями каталогов и расширением
  for (const token of tokens) {
    if (!token.startsWith('-') && (token.includes('/') || token.includes('\\'))) {
      const extMatch = token.match(/\.([a-zA-Z0-9_-]+)$/);
      if (extMatch && extMatch[1].length > 0) {
        return token;
      }
    }
  }

  return null;
}

// Генерация подробного описания и псевдокода для встроенного хука
function buildBuiltinHookDescription(path: string, moment: string, stageTitle: string): string {
  if (path === 'builtin:create_worktree' || path.startsWith('builtin:create_worktree')) {
    return `# Встроенный хук: create_worktree (${moment})

Выполняется автоматически при перемещении задачи в колонку ${stageTitle || 'Backlog'}.

## Логика работы:
1. Создает изолированный git worktree по пути worktrees/<task-id>.
2. Создает и переключает ветку feat/<task-id> от main.
3. Синхронизирует актуальное состояние проекта и .cvh/ в изолированный каталог.
4. Изолирует правки агента от основной ветки.

## Переменные окружения:
- CVH_TASK_ID, CVH_TASK_TITLE, CVH_WORKTREE_PATH, CVH_BRANCH, CVH_STAGE_ID.

## Псевдокод выполнения:
\`\`\`bash
# 1. Определение целевого пути и имени ветки
WORKTREE_DIR="worktrees/\${CVH_TASK_ID}"
BRANCH_NAME="feat/\${CVH_TASK_ID}"

# 2. Создание изолированной ветки и worktree от main
git branch "\${BRANCH_NAME}" main
git worktree add "\${WORKTREE_DIR}" "\${BRANCH_NAME}"

# 3. Синхронизация актуальной конфигурации harness
cp -r .cvh/ "\${WORKTREE_DIR}/.cvh/"
\`\`\``;
  }

  const hookName = path.replace(/^builtin:/, '');
  return `# Встроенный хук: ${hookName} (${moment})

Выполняется автоматически ядром CanbanVoimaxHarness.

## Логика работы:
1. Вызывается ядром Tauri v2 при наступлении события ${moment} стадии "${stageTitle}".
2. Выполняет встроенные операции жизненного цикла задачи.

## Переменные окружения:
- CVH_TASK_ID, CVH_TASK_TITLE, CVH_WORKTREE_PATH, CVH_BRANCH, CVH_STAGE_ID.`;
}

// Генерация описания для прямой команды (напр. "cargo check")
function buildCommandHookDescription(command: string, moment: string, stageTitle: string): string {
  const tokens = tokenizeCommand(command.trim());
  const binary = tokens[0] || command;
  const args = tokens.slice(1);

  return `# Команда хука: ${moment}

Выполняется как системная команда в рабочем окружении (worktree) задачи.

## Команда:
\`\`\`bash
${command}
\`\`\`

## Детали запуска:
- **Утилита / Бинарник**: \`${binary}\`
- **Аргументы**: ${args.length > 0 ? `\`${args.join(' ')}\`` : '*(без аргументов)*'}
- **Рабочий каталог**: \`CVH_WORKTREE_PATH\`

## Логика работы:
1. Запускается ядром CanbanVoimaxHarness при наступлении события **${moment}** колонки "${stageTitle}".
2. Выполняет команду внутри изолированного каталога задачи.
3. При ненулевом коде возврата (exit code != 0) переход стадии отклоняется.

## Переменные окружения:
- CVH_TASK_ID, CVH_TASK_TITLE, CVH_WORKTREE_PATH, CVH_BRANCH, CVH_STAGE_ID, CVH_VERDICT.`;
}

// Шаблон скрипта при отсутствии файла на диске
function buildScriptTemplate(scriptPath: string, moment: string, stageId: string): string {
  const ext = scriptPath.split('.').pop()?.toLowerCase() ?? '';
  if (ext === 'bat' || ext === 'cmd') {
    return `@echo off
rem Скрипт хука: ${moment} (${stageId})
rem Файл: ${scriptPath}
rem Рабочий каталог: %CVH_WORKTREE_PATH%

echo [CVH Hook: %CVH_STAGE_ID%] Running ${moment} for %CVH_TASK_ID%...
exit /b 0
`;
  }
  if (ext === 'ps1') {
    return `# Скрипт хука: ${moment} (${stageId})
# Файл: ${scriptPath}
# Рабочий каталог: $env:CVH_WORKTREE_PATH

Write-Output "[CVH Hook: $env:CVH_STAGE_ID] Running ${moment} for $env:CVH_TASK_ID..."
exit 0
`;
  }
  if (ext === 'py') {
    return `import os
import sys

# Скрипт хука: ${moment} (${stageId})
# Файл: ${scriptPath}

task_id = os.environ.get("CVH_TASK_ID", "")
stage_id = os.environ.get("CVH_STAGE_ID", "")
print(f"[CVH Hook: {stage_id}] Running ${moment} for {task_id}...")
sys.exit(0)
`;
  }
  return `#!/usr/bin/env bash
# Скрипт хука: ${moment} (${stageId})
# Файл: ${scriptPath}

echo "[CVH Hook: \${CVH_STAGE_ID}] Running ${moment} for \${CVH_TASK_ID}..."
exit 0
`;
}


// Имя файла хука по моменту и расширению. Базу из ввода очищаем от
// вручную вписанного расширения, чтобы не получить `setup.bat.bat`.
function buildHookFileName(
  rawName: string,
  moment: CortexHookMoment,
  ext: CortexHookExt,
): string {
  const typed = rawName.trim().replace(/\.[A-Za-z0-9]+$/, '').trim();
  return typed ? `${moment}-${typed}.${ext}` : `${moment}.${ext}`;
}

// Превью имени создаваемого файла для формы (без записи на диск).
export function previewCortexFileName(
  rawName: string,
  kind: 'rule' | 'hook',
  opts?: { hookMoment?: CortexHookMoment; hookExt?: CortexHookExt },
): string {
  if (kind === 'rule') return normalizeCortexFileName(rawName).fileName;
  return buildHookFileName(rawName, opts?.hookMoment ?? 'on_enter', opts?.hookExt ?? 'bat');
}

// Нормализация имени: trim + автосуффикс .md. Возвращает { base, fileName }.
function normalizeCortexFileName(rawName: string): { base: string; fileName: string } {
  const trimmed = rawName.trim();
  const withoutExt = /\.md$/i.test(trimmed) ? trimmed.slice(0, -3) : trimmed;
  const base = withoutExt.trim();
  return { base, fileName: `${base}.md` };
}

// Шаблон персоны на русском для отсутствующего .cvh/agents/<persona>.md
function buildPersonaTemplate(personaName: string, stageTitle: string): string {
  return (
    `# Роль: ${personaName} (${stageTitle})\n` +
    `\n` +
    `## Назначение\n` +
    `Опишите здесь обязанности и границы ответственности агента.\n` +
    `\n` +
    `## Инварианты\n` +
    `- Работать только в рамках своей колонки пайплайна.\n` +
    `- Не использовать заглушки и синтетические моки без необходимости.\n` +
    `- Проверять результат реальными сенсорами перед сдачей.\n` +
    `\n` +
    `## Сенсоры\n` +
    `- npm run build\n`
  );
}

export const useCortexStore = create<CortexState>((set, get) => ({
  stages: DEFAULT_CORTEX_STAGES,
  agentStages: [],
  agentPersonas: [],
  activeFile: null,
  focusedStageId: null,
  isEditorOpen: false,
  isSaving: false,
  saveFeedback: null,
  fileMissing: {},

  toggleStageExpand: (stageId) => {
    // Один флоу на обе секции: id колонок и агентов (`agent:<role>`) не пересекаются.
    set((state) => ({
      stages: state.stages.map((st) =>
        st.stageId === stageId ? { ...st, isExpanded: !st.isExpanded } : st
      ),
      agentStages: state.agentStages.map((st) =>
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
      agentStages: state.agentStages.map((st) =>
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

  syncStagesFromColumns: (columns) => {
    const prevStages = get().stages;
    const prevStageById = new Map(prevStages.map((st) => [st.stageId, st]));
    const prevFileById = new Map(prevStages.flatMap((st) => st.files.map((f) => [f.id, f])));
    // Ранее созданные файлы (правила/персоны) по stageId — пережить пересборку стадий.
    const prevExtrasByStage = new Map<string, CortexAgentFile[]>();
    for (const st of prevStages) {
      for (const f of st.files) {
        const arr = prevExtrasByStage.get(f.stageId) ?? [];
        arr.push(f);
        prevExtrasByStage.set(f.stageId, arr);
      }
    }

    // Каждая колонка — стадия в проводнике разума. Все колонки из columns
    // остаются в boardStages проводника разума (дедупа по persona здесь нет:
    // дедуп живёт только в секции агентов).
    // Системные узлы (backlog + ready/terminal без персоны) показывают свои hooks
    // из колонки и personaName: 'система'.
    // Колонки без выбранного агента отображаются со статусом personaName: 'без агента',
    // сохраняя файлы хуков/правил стадии.
    const boardStages: CortexAgentStage[] = [];
    for (const col of columns) {
      const persona = col.persona?.trim() ?? '';
      const isSystemNode =
        persona === '' &&
        (col.id === 'backlog' || col.id === 'ready' || col.terminal === true);
      const dir = cortexAgentDir(col.persona ?? '', col.id);
      if (isSystemNode) {
        // Хуки колонки (on_enter/on_finish) как узлы типа hook.
        // Маркер `builtin:create_worktree` хранится в path — проводник
        // покажет бейдж «встроенный» и не будет открывать файл.
        const hookMoments = Object.entries(col.hooks ?? {})
          .filter(([moment]) => moment === 'on_enter' || moment === 'on_finish')
          .sort(([a], [b]) => a.localeCompare(b));
        const hookFiles = hookMoments.map(([moment, target]) => {
          const hookId = `${col.id}-hook-${moment}`;
          const prev = prevFileById.get(hookId);
          return {
            id: hookId,
            name: moment,
            stageId: col.id,
            type: 'hook' as const,
            path: target,
            content: prev?.content ?? '',
            isDirty: false,
          };
        });
        const extras = (prevExtrasByStage.get(col.id) ?? [])
          .filter(
            (f) =>
              !hookFiles.some((h) => h.id === f.id) &&
              (f.type === 'rule' || f.type === 'hook') &&
              f.path.startsWith(`${dir}/`),
          )
          .sort((a, b) => {
            const order = (t: string) => (t === 'rule' ? 0 : 1);
            return order(a.type) - order(b.type) || a.name.localeCompare(b.name);
          });
        boardStages.push({
          stageId: col.id,
          stageTitle: col.title,
          color: col.color,
          icon: col.icon,
          personaName: 'система',
          isExpanded: prevStageById.get(col.id)?.isExpanded ?? true,
          files: [...hookFiles, ...extras],
        });
        continue;
      }
      const personaId = `${col.id}-persona`;
      const settingsId = `${col.id}-settings`;
      const prevPersona = prevFileById.get(personaId);
      const prevSettings = prevFileById.get(settingsId);
      // Хуки колонки (on_enter/on_finish) как узлы типа hook
      const hookMoments = Object.entries(col.hooks ?? {})
        .filter(([moment]) => moment === 'on_enter' || moment === 'on_finish')
        .sort(([a], [b]) => a.localeCompare(b));
      const hookFiles = hookMoments.map(([moment, target]) => {
        const hookId = `${col.id}-hook-${moment}`;
        const prev = prevFileById.get(hookId);
        return {
          id: hookId,
          name: moment,
          stageId: col.id,
          type: 'hook' as const,
          path: target,
          content: prev?.content ?? '',
          isDirty: false,
        };
      });
      // Ранее созданные файлы стадии (правила/хуки новой схемы) — пережить пересборку.
      // Фильтр по префиксу каталога агента: файлы чужой роли и старой схемы
      // (`.cvh/rules/...`, `.cvh/agents/<role>.md`) в дерево НЕ возвращаются.
      // Если персона снята, сохраняем хуки и правила этой стадии.
      const extras = (prevExtrasByStage.get(col.id) ?? [])
        .filter(
          (f) =>
            f.id !== personaId &&
            f.id !== settingsId &&
            !hookFiles.some((h) => h.id === f.id) &&
            (f.type === 'rule' || f.type === 'hook') &&
            (f.path.startsWith(`${dir}/`) || f.type === 'hook' || !persona),
        )
        .sort((a, b) => {
          const order = (t: string) => (t === 'rule' ? 0 : 1);
          return order(a.type) - order(b.type) || a.name.localeCompare(b.name);
        });
      const personaInfo = get().agentPersonas.find((ap) => ap.role === col.persona?.trim());
      const personaFileName = personaInfo?.personaFile ?? 'persona.md';

      const diskRules = (personaInfo?.rules ?? []).map((ruleName) => {
        const ruleId = `${col.id}-rule-${ruleName}`;
        const prevRule = prevFileById.get(ruleId);
        return {
          id: ruleId,
          name: ruleName,
          stageId: col.id,
          type: 'rule' as const,
          path: `${dir}/rules/${ruleName}`,
          content: prevRule?.content ?? '',
          isDirty: prevRule?.isDirty ?? false,
        };
      });

      const inMemoryExtras = extras.filter(
        (f) => f.type !== 'rule' || !diskRules.some((dr) => dr.name === f.name),
      );

      const allExtras = [...diskRules, ...inMemoryExtras].sort((a, b) => {
        const order = (t: string) => (t === 'rule' ? 0 : 1);
        return order(a.type) - order(b.type) || a.name.localeCompare(b.name);
      });

      // Четкая стилизация Backlog (#64748b) и Ready (#10b981) + реактивная иконка и цвет персоны
      let stageColor = col.color;
      let stageIcon = col.icon;
      if (col.id === 'backlog') {
        stageColor = BACKLOG_COLOR;
        stageIcon = col.icon || getDefaultColumnIcon('backlog');
      } else if (col.id === 'ready') {
        stageColor = READY_COLOR;
        stageIcon = col.icon || getDefaultColumnIcon('ready');
      } else {
        if (col.persona) {
          if (personaInfo?.icon) {
            stageIcon = personaInfo.icon;
          } else if (!stageIcon) {
            stageIcon = getDefaultColumnIcon(col.id);
          }
          if (personaInfo?.color && isAllowedAgentColor(personaInfo.color)) {
            stageColor = personaInfo.color;
          } else if (col.color && isAllowedAgentColor(col.color)) {
            stageColor = col.color;
          } else {
            stageColor = getDeterministicAgentColor(col.persona);
          }
        } else {
          stageIcon = col.icon || getDefaultColumnIcon(col.id);
          stageColor = isAllowedAgentColor(col.color) ? col.color : '#171717';
        }
      }

      boardStages.push({
        stageId: col.id,
        stageTitle: col.title,
        color: stageColor,
        icon: stageIcon,
        personaName: col.persona?.trim() || 'без агента',
        isExpanded: prevStageById.get(col.id)?.isExpanded ?? true,
        files: [
          {
            id: personaId,
            name: personaFileName,
            stageId: col.id,
            type: 'persona',
            path: `${dir}/${personaFileName}`,
            content: prevPersona?.content ?? '',
            isDirty: prevPersona?.isDirty ?? false,
          },
          ...hookFiles,
          ...allExtras,
          {
            id: settingsId,
            name: 'setting',
            stageId: col.id,
            type: 'settings',
            path: `${dir}/setting`,
            content: prevSettings?.content ?? '',
            isDirty: prevSettings?.isDirty ?? false,
          },
        ],
      });
    }

    if (boardStages.length === 0) {
      if (columns.length === 0) {
        // Fallback для web-preview без бэкенда: мок-стадии.
        // Секцию агентов не трогаем: agentStages живут отдельно.
        set({ stages: DEFAULT_CORTEX_STAGES });
        return;
      }
      // Свежий проект без агентов (только backlog+ready без persona):
      // пустое состояние секции колонок. Пометы agentStages не сбрасываем.
      set((state) => {
        const aliveAgentIds = new Set(
          state.agentStages.flatMap((s) => s.files.map((f) => f.id)),
        );
        const nextMissing: Record<string, boolean> = {};
        for (const [id, value] of Object.entries(state.fileMissing)) {
          if (aliveAgentIds.has(id)) nextMissing[id] = value;
        }
        return { stages: [], fileMissing: nextMissing };
      });
      return;
    }

    // Сохранить пометки об отсутствующих файлах для переживших пересборку узлов.
    // Встроенный маркер `builtin:create_worktree` — не файл: пометку не переносим.
    // Пометы секции агентов переживают пересборку колонок без изменений.
    const prevMissing = get().fileMissing;
    const prevAgentStages = get().agentStages;
    const nextMissing: Record<string, boolean> = {};
    for (const st of boardStages) {
      for (const f of st.files) {
        if (f.path === 'builtin:create_worktree' || f.path.startsWith('builtin:')) continue;
        if (prevMissing[f.id]) nextMissing[f.id] = true;
      }
    }
    for (const st of prevAgentStages) {
      for (const f of st.files) {
        if (prevMissing[f.id]) nextMissing[f.id] = true;
      }
    }
    set({ stages: boardStages, fileMissing: nextMissing });
  },

  loadAgentPersonas: async () => {
    let raw: RawAgentPersona[] = [];
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const list = await invoke<RawAgentPersona[]>('list_agent_personas');
      if (Array.isArray(list)) raw = list;
    } catch {
      // IPC недоступен — секция агентов пуста, колонки не страдают.
      raw = [];
    }
    const seen = new Set<string>();
    const personas: AgentPersonaInfo[] = [];
    for (const item of raw) {
      if (!item || typeof item.role !== 'string') continue;
      const role = item.role.trim();
      if (!role || seen.has(role)) continue;
      seen.add(role);
      const roleColor = isAllowedAgentColor(item.color)
        ? item.color!
        : getDeterministicAgentColor(role);
      personas.push({
        role,
        hasPrompt: item.hasPrompt ?? item.has_prompt ?? true,
        isBuiltin: item.isBuiltin ?? item.is_builtin ?? (role === 'cvh-dev'),
        personaFile: item.personaFile ?? item.persona_file ?? 'persona.md',
        rules: Array.isArray(item.rules) ? item.rules : [],
        hooks: Array.isArray(item.hooks) ? item.hooks : [],
        icon: item.icon,
        color: roleColor,
        contextWindow: item.contextWindow ?? item.context_window,
      });
    }
    personas.sort((a, b) => a.role.localeCompare(b.role));

    const prev = get().agentStages;
    const prevByRole = new Map(prev.map((st) => [st.personaName, st]));
    const nextStages: CortexAgentStage[] = personas.map((p) => {
      const dir = cortexAgentDir(p.role, p.role);
      const stageId = `agent:${p.role}`;
      const prevStage = prevByRole.get(p.role);
      const prevFiles = prevStage?.files ?? [];
      const prevById = new Map(prevFiles.map((f) => [f.id, f]));
      const personaFileName = p.personaFile ?? 'persona.md';
      const personaId = `${stageId}-persona`;
      const settingsId = `${stageId}-settings`;
      const prevPersona = prevById.get(personaId);
      const prevSettings = prevById.get(settingsId);

      // Real rules from disk!
      const diskRules = (p.rules ?? []).map((ruleName) => {
        const ruleId = `${stageId}-rule-${ruleName}`;
        const prevRule = prevById.get(ruleId);
        return {
          id: ruleId,
          name: ruleName,
          stageId,
          type: 'rule' as const,
          path: `${dir}/rules/${ruleName}`,
          content: prevRule?.content ?? '',
          isDirty: prevRule?.isDirty ?? false,
        };
      });

      // Extra in-memory rules not yet on disk or newly added
      const inMemoryExtras = prevFiles.filter(
        (f) =>
          f.id !== personaId &&
          f.id !== settingsId &&
          f.type === 'rule' &&
          !diskRules.some((dr) => dr.name === f.name),
      );

      const allRules = [...diskRules, ...inMemoryExtras].sort((a, b) =>
        a.name.localeCompare(b.name),
      );

      const isBuiltin = p.role === 'cvh-dev' || Boolean(p.isBuiltin);
      return {
        stageId,
        stageTitle: p.role,
        color: p.color || getDeterministicAgentColor(p.role),
        icon: p.icon || (isBuiltin ? 'Cpu' : 'Bot'),
        personaName: p.role,
        isExpanded: prevStage?.isExpanded ?? false,
        isBuiltin,
        files: [
          {
            id: personaId,
            name: personaFileName,
            stageId,
            type: 'persona',
            path: `${dir}/${personaFileName}`,
            content: prevPersona?.content ?? '',
            isDirty: prevPersona?.isDirty ?? false,
          },
          ...allRules,
          {
            id: settingsId,
            name: 'setting',
            stageId,
            type: 'settings',
            path: `${dir}/setting`,
            content: prevSettings?.content ?? '',
            isDirty: prevSettings?.isDirty ?? false,
          },
        ],
      };
    });

    // Снять пометы об отсутствующих файлах для агентов, которых больше нет.
    const alive = new Set(nextStages.flatMap((s) => s.files.map((f) => f.id)));
    for (const st of get().stages) {
      for (const f of st.files) alive.add(f.id);
    }
    const prunedMissing: Record<string, boolean> = {};
    for (const [id, value] of Object.entries(get().fileMissing)) {
      if (alive.has(id)) prunedMissing[id] = value;
    }
    set({ agentPersonas: personas, agentStages: nextStages, fileMissing: prunedMissing });

    // Реактивно синхронизировать колонки сразу при загрузке агентов (иконка и цвет персоны)
    get().syncStagesFromColumns(useBoardStore.getState().columns);
  },

  setAgentPersonaColor: async (role: string, color: string) => {
    if (!isAllowedAgentColor(color)) {
      throw new Error("Серый и зеленый цвета зарезервированы за Backlog и Ready и запрещены для агентов");
    }
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('set_agent_persona_color', { role, color });
    } catch (err) {
      console.info('set_agent_persona_color IPC not available or failed:', err);
    }
    set((state) => ({
      agentPersonas: state.agentPersonas.map((p) =>
        p.role === role ? { ...p, color } : p
      ),
      agentStages: state.agentStages.map((st) =>
        st.personaName === role ? { ...st, color } : st
      ),
    }));

    // Автоматическое наследование цвета колонками, на которые назначен этот агент (если не backlog и не ready)
    const boardStore = useBoardStore.getState();
    const prevCols = boardStore.columns;
    let changed = false;
    const nextCols = prevCols.map((c) => {
      if (c.persona === role && c.id !== 'backlog' && c.id !== 'ready') {
        changed = true;
        return { ...c, color };
      }
      return c;
    });
    if (changed) {
      boardStore.setColumns(nextCols);
      get().syncStagesFromColumns(nextCols);
      void boardStore.saveCurrentBoard(nextCols);
    } else {
      get().syncStagesFromColumns(prevCols);
    }
  },

  setAgentPersonaIcon: async (role: string, icon: string) => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('set_agent_persona_icon', { role, icon });
    } catch (err) {
      console.info('set_agent_persona_icon IPC not available or failed:', err);
    }
    set((state) => ({
      agentPersonas: state.agentPersonas.map((p) =>
        p.role === role ? { ...p, icon } : p
      ),
      agentStages: state.agentStages.map((st) =>
        st.personaName === role ? { ...st, icon } : st
      ),
    }));

    // Автоматическое наследование иконки колонками, на которые назначен этот агент
    const boardStore = useBoardStore.getState();
    const prevCols = boardStore.columns;
    let changed = false;
    const nextCols = prevCols.map((c) => {
      if (c.persona === role) {
        changed = true;
        return { ...c, icon };
      }
      return c;
    });
    if (changed) {
      boardStore.setColumns(nextCols);
      get().syncStagesFromColumns(nextCols);
      void boardStore.saveCurrentBoard(nextCols);
    } else {
      get().syncStagesFromColumns(prevCols);
    }
  },

  renameAgentPersona: async (oldRole: string, newRole: string) => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('rename_agent_persona', { oldRole, newRole });
    } catch (err) {
      console.info('rename_agent_persona IPC not available or failed:', err);
      throw err;
    }
    await get().loadAgentPersonas();
    try {
      await useBoardStore.getState().loadBoards();
    } catch (err) {
      console.info('Failed to reload boards after agent rename:', err);
    }
  },

  deleteAgentPersona: async (role: string) => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('delete_agent_persona', { role });
    } catch (err) {
      console.info('delete_agent_persona IPC not available or failed:', err);
      throw err;
    }
    await get().loadAgentPersonas();
    try {
      await useBoardStore.getState().loadBoards();
    } catch (err) {
      console.info('Failed to reload boards after agent delete:', err);
    }
  },

  openFile: async (fileId) => {
    // Один флоу открытия на обе секции: ищем узел и в колонках, и в агентах.
    const { stages, agentStages } = get();
    const allStages = [...stages, ...agentStages];
    for (const stage of allStages) {
      const file = stage.files.find((f) => f.id === fileId);
      if (!file) continue;

      // 1. Встроенные хуки (builtin:create_worktree и др.)
      if (file.path.startsWith('builtin:') || file.path === 'builtin:create_worktree') {
        const content =
          file.content && file.content.trim().length > 0
            ? file.content
            : buildBuiltinHookDescription(file.path, file.name, stage.stageTitle);
        const loaded: CortexAgentFile = { ...file, content, isDirty: false };
        set((state) => ({
          stages: state.stages.map((st) =>
            st.stageId === stage.stageId
              ? { ...st, files: st.files.map((f) => (f.id === fileId ? loaded : f)) }
              : st
          ),
          agentStages: state.agentStages.map((st) =>
            st.stageId === stage.stageId
              ? { ...st, files: st.files.map((f) => (f.id === fileId ? loaded : f)) }
              : st
          ),
          activeFile: loaded,
          isEditorOpen: true,
          saveFeedback: null,
        }));
        return;
      }

      // 2. Хуки стадий: скрипты или системные команды
      if (file.type === 'hook') {
        const scriptPath = extractScriptPath(file.path);
        if (scriptPath) {
          // Хук является скриптом: пробуем прочитать его реальный исходный код с диска
          try {
            const { invoke } = await import('@tauri-apps/api/core');
            const content = await invoke<string>('read_cortex_file', { path: scriptPath });
            const loaded: CortexAgentFile = { ...file, content, isDirty: false };
            set((state) => ({
              stages: state.stages.map((st) =>
                st.stageId === stage.stageId
                  ? { ...st, files: st.files.map((f) => (f.id === fileId ? loaded : f)) }
                  : st
              ),
              agentStages: state.agentStages.map((st) =>
                st.stageId === stage.stageId
                  ? { ...st, files: st.files.map((f) => (f.id === fileId ? loaded : f)) }
                  : st
              ),
              activeFile: loaded,
              isEditorOpen: true,
              saveFeedback: null,
            }));
            // Файл найден: снимаем пометку отсутствия
            set((state) => {
              if (!state.fileMissing[fileId]) return state;
              const next = { ...state.fileMissing };
              delete next[fileId];
              return { fileMissing: next };
            });
            return;
          } catch (err) {
            console.info('read_cortex_file for hook script failed, using template/memory:', err);
            const content =
              file.content && file.content.trim().length > 0
                ? file.content
                : buildScriptTemplate(scriptPath, file.name, stage.stageId);
            const loaded: CortexAgentFile = { ...file, content, isDirty: false };
            set((state) => ({
              stages: state.stages.map((st) =>
                st.stageId === stage.stageId
                  ? { ...st, files: st.files.map((f) => (f.id === fileId ? loaded : f)) }
                  : st
              ),
              agentStages: state.agentStages.map((st) =>
                st.stageId === stage.stageId
                  ? { ...st, files: st.files.map((f) => (f.id === fileId ? loaded : f)) }
                  : st
              ),
              activeFile: loaded,
              isEditorOpen: true,
              saveFeedback: null,
            }));
            return;
          }
        } else {
          // Прямая команда (напр. "cargo check", "npm run build") — формируем структурированное описание
          const content =
            file.content && file.content.trim().length > 0
              ? file.content
              : buildCommandHookDescription(file.path, file.name, stage.stageTitle);
          const loaded: CortexAgentFile = { ...file, content, isDirty: false };
          set((state) => ({
            stages: state.stages.map((st) =>
              st.stageId === stage.stageId
                ? { ...st, files: st.files.map((f) => (f.id === fileId ? loaded : f)) }
                : st
            ),
            agentStages: state.agentStages.map((st) =>
              st.stageId === stage.stageId
                ? { ...st, files: st.files.map((f) => (f.id === fileId ? loaded : f)) }
                : st
            ),
            activeFile: loaded,
            isEditorOpen: true,
            saveFeedback: null,
          }));
          return;
        }
      }

      // 3. Обычные текстовые файлы (persona, rule, settings)
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const content = await invoke<string>('read_cortex_file', { path: file.path });
        const loaded: CortexAgentFile = { ...file, content, isDirty: false };
        set((state) => ({
          stages: state.stages.map((st) =>
            st.stageId === stage.stageId
              ? {
                  ...st,
                  files: st.files.map((f) =>
                    f.id === fileId ? { ...f, content, isDirty: false } : f
                  ),
                }
              : st
          ),
          agentStages: state.agentStages.map((st) =>
            st.stageId === stage.stageId
              ? {
                  ...st,
                  files: st.files.map((f) =>
                    f.id === fileId ? { ...f, content, isDirty: false } : f
                  ),
                }
              : st
          ),
          activeFile: loaded,
          isEditorOpen: true,
          saveFeedback: null,
        }));
        // Файл на месте: снять пометку об отсутствии, если была
        set((state) => {
          if (!state.fileMissing[fileId]) return state;
          const next = { ...state.fileMissing };
          delete next[fileId];
          return { fileMissing: next };
        });
      } catch (err) {
        console.info('read_cortex_file not available or file is missing:', err);
        if (file.content) {
          // Fallback для web-preview без бэкенда: открыть содержимое из памяти
          set({ activeFile: { ...file }, isEditorOpen: true, saveFeedback: null });
          return;
        }
        // Файла нет на диске: показать в дереве плейсхолдер «нет файла»
        // с кнопкой создания по существующему флоу шаблонов
        set((state) => ({
          fileMissing: { ...state.fileMissing, [fileId]: true },
        }));
      }
      return;
    }
  },

  createPersonaTemplate: async (fileId) => {
    const { stages, agentStages } = get();
    const allStages = [...stages, ...agentStages];
    for (const stage of allStages) {
      const file = stage.files.find((f) => f.id === fileId);
      if (!file) continue;

      const template = buildPersonaTemplate(stage.personaName, stage.stageTitle);
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('save_cortex_file', { path: file.path, content: template });
      } catch (err) {
        console.info('save_cortex_file not available or in web mode:', err);
      }
      // Перезагрузить узел: обновить содержимое, снять пометку, открыть в редакторе
      const loaded: CortexAgentFile = { ...file, content: template, isDirty: false };
      set((state) => {
        const nextMissing = { ...state.fileMissing };
        delete nextMissing[fileId];
        return {
          stages: state.stages.map((st) =>
            st.stageId === stage.stageId
              ? {
                  ...st,
                  files: st.files.map((f) =>
                    f.id === fileId ? { ...f, content: template, isDirty: false } : f
                  ),
                }
              : st
          ),
          agentStages: state.agentStages.map((st) =>
            st.stageId === stage.stageId
              ? {
                  ...st,
                  files: st.files.map((f) =>
                    f.id === fileId ? { ...f, content: template, isDirty: false } : f
                  ),
                }
              : st
          ),
          activeFile: loaded,
          isEditorOpen: true,
          saveFeedback: null,
          fileMissing: nextMissing,
        };
      });
      await get().loadAgentPersonas();
      return;
    }
  },

  createSettingTemplate: async (fileId) => {
    const { stages, agentStages } = get();
    const allStages = [...stages, ...agentStages];
    for (const stage of allStages) {
      const file = stage.files.find((f) => f.id === fileId);
      if (!file) continue;

      const role = (stage.personaName ?? '').trim() || stage.stageId;
      const template = buildSettingTemplate(role, stage.stageTitle);
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('save_cortex_file', { path: file.path, content: template });
      } catch (err) {
        console.info('save_cortex_file not available or in web mode:', err);
      }
      // Перезагрузить узел: обновить содержимое, снять пометку, открыть в редакторе
      const loaded: CortexAgentFile = { ...file, content: template, isDirty: false };
      set((state) => {
        const nextMissing = { ...state.fileMissing };
        delete nextMissing[fileId];
        return {
          stages: state.stages.map((st) =>
            st.stageId === stage.stageId
              ? {
                  ...st,
                  files: st.files.map((f) =>
                    f.id === fileId ? { ...f, content: template, isDirty: false } : f
                  ),
                }
              : st
          ),
          agentStages: state.agentStages.map((st) =>
            st.stageId === stage.stageId
              ? {
                  ...st,
                  files: st.files.map((f) =>
                    f.id === fileId ? { ...f, content: template, isDirty: false } : f
                  ),
                }
              : st
          ),
          activeFile: loaded,
          isEditorOpen: true,
          saveFeedback: null,
          fileMissing: nextMissing,
        };
      });
      return;
    }
  },

  createCortexFile: async (stageId, rawName, kind, opts) => {
    const { stages, agentStages } = get();
    // Создание работает и из секции колонок, и из секции агентов.
    const stage =
      stages.find((st) => st.stageId === stageId) ??
      agentStages.find((st) => st.stageId === stageId);
    if (!stage) throw new Error('Стадия не найдена');

    const role = (stage.personaName ?? '').trim() || stageId;
    const dir = cortexAgentDir(stage.personaName, stageId);

    let targetPath: string;
    let content: string;
    let fileName: string;
    let fileType: CortexAgentFile['type'];

    if (kind === 'rule') {
      const { base, fileName: ruleName } = normalizeCortexFileName(rawName);
      if (!base) throw new Error('Введите имя файла');
      if (!/^[A-Za-z0-9-_]+$/.test(base)) {
        throw new Error('Только латиница, цифры, - и _');
      }
      fileName = ruleName;
      fileType = 'rule';
      targetPath = `${dir}/rules/${fileName}`;
      content = buildRuleTemplate(base, stage.stageTitle);
    } else {
      // Хук: имя файла кодирует момент срабатывания.
      const moment: CortexHookMoment = opts?.hookMoment ?? 'on_enter';
      const ext: CortexHookExt = opts?.hookExt ?? 'bat';
      if (moment !== 'on_enter' && moment !== 'on_finish') {
        throw new Error('Момент хука: on_enter или on_finish');
      }
      if (ext !== 'bat' && ext !== 'ps1' && ext !== 'py' && ext !== 'sh') {
        throw new Error('Расширение хука: .bat, .ps1, .py или .sh');
      }
      // Пользователь мог вписать расширение вручную — срезать, чтобы не дублировалось.
      const typed = rawName.trim().replace(/\.[A-Za-z0-9]+$/, '').trim();
      if (typed && !/^[A-Za-z0-9-_]+$/.test(typed)) {
        throw new Error('Только латиница, цифры, - и _');
      }
      fileName = typed ? `${moment}-${typed}.${ext}` : `${moment}.${ext}`;
      fileType = 'hook';
      targetPath = `${dir}/hooks/${fileName}`;
      content = buildHookTemplate(moment, ext, typed || moment, role, stage.stageTitle);
    }

    const duplicate = stage.files.some(
      (f) => f.name.toLowerCase() === fileName.toLowerCase() || f.path.toLowerCase() === targetPath.toLowerCase()
    );
    if (duplicate) throw new Error(`Файл ${fileName} уже есть в стадии`);

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('save_cortex_file', { path: targetPath, content });
    } catch (err) {
      const detail = err instanceof Error ? err.message : String(err);
      throw new Error(`Не удалось сохранить файл: ${detail}`);
    }

    // Уникальный id узла в пределах стадии
    const idBase = `${stageId}-${kind}-${fileName}`;
    let fileId = idBase;
    let suffix = 2;
    const existingIds = new Set(stage.files.map((f) => f.id));
    while (existingIds.has(fileId)) {
      fileId = `${idBase}-${suffix}`;
      suffix += 1;
    }

    const created: CortexAgentFile = {
      id: fileId,
      name: fileName,
      stageId,
      type: fileType,
      path: targetPath,
      content,
      isDirty: false,
    };

    // Узел появляется в дереве стадии и открывается в редакторе (существующий флоу)
    set((state) => ({
      stages: state.stages.map((st) =>
        st.stageId === stageId ? { ...st, files: [...st.files, created] } : st
      ),
      agentStages: state.agentStages.map((st) =>
        st.stageId === stageId ? { ...st, files: [...st.files, created] } : st
      ),
      activeFile: created,
      isEditorOpen: true,
      saveFeedback: null,
    }));
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
    const { activeFile, stages, agentStages } = get();
    if (!activeFile) return;

    set({ isSaving: true });

    const isBuiltin = activeFile.path.startsWith('builtin:');
    const scriptPath = extractScriptPath(activeFile.path);
    const isDirectCommand = activeFile.type === 'hook' && !isBuiltin && !scriptPath;

    if (!isBuiltin && !isDirectCommand) {
      const targetPath = scriptPath ?? activeFile.path;
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('save_cortex_file', { path: targetPath, content: activeFile.content });
      } catch (err) {
        console.info('save_cortex_file not available or in web mode:', err);
      }
    }

    // Update in memory stages (обе секции: файл мог быть открыт из любой)
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
    const updatedAgentStages = agentStages.map((stage) => {
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

    const feedbackMsg = isBuiltin
      ? `Встроенный хук ${activeFile.name} обновлен`
      : isDirectCommand
      ? `Команда ${activeFile.name} обновлена`
      : `Файл ${activeFile.name} сохранен`;

    set({
      stages: updatedStages,
      agentStages: updatedAgentStages,
      activeFile: { ...activeFile, isDirty: false },
      isSaving: false,
      saveFeedback: feedbackMsg,
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

let cortexListenersInitialized = false;
export async function initCortexEventListeners(): Promise<() => void> {
  if (cortexListenersInitialized) return () => {};
  try {
    const { listen } = await import('@tauri-apps/api/event');
    const uPersonas = await listen<any>('agent:personas_updated', async () => {
      await useCortexStore.getState().loadAgentPersonas();
      useCortexStore.getState().syncStagesFromColumns(useBoardStore.getState().columns);
    });
    const uBoards = await listen<any>('boards:updated', async () => {
      useCortexStore.getState().syncStagesFromColumns(useBoardStore.getState().columns);
    });
    const uPipeline = await listen<any>('pipeline:updated', async () => {
      useCortexStore.getState().syncStagesFromColumns(useBoardStore.getState().columns);
    });
    cortexListenersInitialized = true;
    return () => {
      uPersonas();
      uBoards();
      uPipeline();
      cortexListenersInitialized = false;
    };
  } catch (e) {
    console.info('initCortexEventListeners not active in web mode:', e);
    return () => {};
  }
}
