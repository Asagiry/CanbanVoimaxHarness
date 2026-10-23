# Guide: Transition Scripts and Lifecycle Hooks

Authoritative rules for `.cvh/hooks/` and column lifecycle scripts.

## 1. Lifecycle Triggers
Columns declare transition hooks in `pipeline.json` or `boards.json` under `columns[].hooks`:
- `hooks.on_enter`: Triggered when a card enters the column.
- `hooks.on_leave`: Triggered when a card leaves the column.

Никаких встроенных хардкодов: все хуки — это реальные скрипты (`.bat`, `.cmd`, `.ps1`, `.sh`, `.py`) в `.cvh/hooks/` или в каталоге агента.

## 2. Worktree & Branch Protocol (Stdout-протокол)
Если скрипт хука создает git worktree или переключает ветку, он выводит в stdout:
```text
CVH_WORKTREE=<absolute_or_relative_path>
CVH_BRANCH=<branch_name>
```
Бэкенд CVH автоматически парсит эти строки из stdout хука и сохраняет в свойства карточки задачи (`task.worktree_path` и `task.branch`).

Пример скрипта создания ворктри (`.cvh/hooks/create_worktree.bat`):
```bat
@echo off
set "WT_DIR=worktrees\%CVH_TASK_ID%"
set "BRANCH=feat/%CVH_TASK_ID%"
if not exist "%WT_DIR%" (
    git worktree add -b "%BRANCH%" "%WT_DIR%" HEAD || exit /b 1
)
echo CVH_WORKTREE=%WT_DIR%
echo CVH_BRANCH=%BRANCH%
exit /b 0
```

Пример скрипта на Bash (`.cvh/hooks/create_worktree.sh`):
```bash
#!/usr/bin/env bash
set -e
WT_DIR="worktrees/${CVH_TASK_ID}"
BRANCH="feat/${CVH_TASK_ID}"
if [ ! -d "$WT_DIR" ]; then
    git worktree add -b "$BRANCH" "$WT_DIR" HEAD
fi
echo "CVH_WORKTREE=$WT_DIR"
echo "CVH_BRANCH=$BRANCH"
exit 0
```

## 3. Injected Environment Variables
Каждый хук получает полный контекст задачи через переменные окружения:
- `CVH_TASK_ID`: Идентификатор задачи (например, `CVH-042`).
- `CVH_TASK_TITLE`: Заголовок задачи.
- `CVH_WORKTREE_PATH`: Текущий путь к ворктри задачи (если назначен).
- `CVH_BRANCH`: Текущая ветка задачи.
- `CVH_STAGE_ID`: Идентификатор стадии перехода.
- `CVH_VERDICT`: Вердикт ревью (`APPROVED`, `APPROVED_WITH_CAVEATS`, `CHANGES_REQUESTED`).

## 4. Reject Механизм при ошибке хука
Если скрипт хука завершается с ненулевым кодом выхода (`exit code != 0`):
1. Переход карточки отменяется (REJECT). Карточка остается в исходной колонке.
2. Лог ошибки из `stderr` / `stdout` записывается в SQLite диалог задачи как директива `steer`.
3. Вердикт карточки выставляется в `CHANGES_REQUESTED`.
4. Если у колонки настроен агент (persona) или auto_run, агент автоматически просыпается с сообщением об ошибке и немедленно приступает к её исправлению.
5. На карточке у соответствующей ноды хука загорается красный крестик ✕, при наведении отображается окно с логом ошибки.
