import { CortexHookExt, CortexHookMoment } from './types';
import { SCRIPT_EXTENSIONS } from './cortexConstants';
import {
  getDeterministicAgentColor,
  getDeterministicAgentIcon,
} from '../../lib/agentPalette';

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
export function buildRuleTemplate(baseName: string, stageTitle: string): string {
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

// Concise English template for global shared rules in `.cvh/rules/`
export function buildGlobalRuleTemplate(baseName: string): string {
  return (
    `# Rule: ${baseName}\n\n` +
    `## Purpose\n` +
    `Define the system standard or architectural invariant.\n\n` +
    `## Invariants & Guidelines\n` +
    `- Adhere strictly to project conventions.\n` +
    `- Verify results with ground truth sensors.\n`
  );
}

// Минимальный шаблон хука: момент в имени файла, расширение задаёт интерпретатор.
export function buildHookTemplate(
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
export function buildSettingTemplate(role: string, stageTitle: string): string {
  const icon = getDeterministicAgentIcon(role);
  const color = getDeterministicAgentColor(role);
  return (
    `{\n` +
    `  "_comment": "Настройки агента ${role} (${stageTitle})",\n` +
    `  "model": null,\n` +
    `  "temperature": 0.2,\n` +
    `  "context_window": 128000,\n` +
    `  "icon": "${icon}",\n` +
    `  "color": "${color}"\n` +
    `}\n`
  );
}

// Токенизация shell-команд с учётом кавычек
export function tokenizeCommand(cmd: string): string[] {
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

// Извлечение пути к файлу скрипта из команды хука (напр. "python .cvh/hooks/gatekeeper.py", ".bat", ".ps1")
export function extractScriptPath(cmd: string): string | null {
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
export function buildBuiltinHookDescription(path: string, moment: string, stageTitle: string): string {
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
export function buildCommandHookDescription(command: string, moment: string, stageTitle: string): string {
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
export function buildScriptTemplate(scriptPath: string, moment: string, stageId: string): string {
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
export function buildHookFileName(
  rawName: string,
  moment: CortexHookMoment,
  ext: CortexHookExt,
): string {
  const typed = rawName.trim().replace(/\.[A-Za-z0-9]+$/, '').trim();
  return typed ? `${moment}-${typed}.${ext}` : `${moment}.${ext}`;
}

// Нормализация имени: trim + автосуффикс .md. Возвращает { base, fileName }.
export function normalizeCortexFileName(rawName: string): { base: string; fileName: string } {
  const trimmed = rawName.trim();
  const withoutExt = /\.md$/i.test(trimmed) ? trimmed.slice(0, -3) : trimmed;
  const base = withoutExt.trim();
  return { base, fileName: `${base}.md` };
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

// English persona template for missing .cvh/agents/<persona>.md (English-Only Invariant)
export function buildPersonaTemplate(personaName: string, stageTitle: string): string {
  return (
    `# Role Persona: ${personaName} (${stageTitle})\n` +
    `\n` +
    `## Identity & Scope\n` +
    `- Core Mission: <describe the primary responsibility and objectives of this role>\n` +
    `- Permitted Operations: <list allowed tools, inspection, and verification scopes>\n` +
    `- Prohibitions: <list strict prohibitions, e.g. never write code without tests, never commit broken builds>\n` +
    `\n` +
    `## Guidelines\n` +
    `- Work strictly inside the isolated git worktree.\n` +
    `- State only what runtime sensors prove.\n` +
    `\n` +
    `## Sensors\n` +
    `- pnpm run build\n`
  );
}
