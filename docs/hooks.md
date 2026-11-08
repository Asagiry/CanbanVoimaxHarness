# ⚡ Автономные скрипты перехода на колонках (docs/hooks.md)

> **CanbanVoimaxHarness (CVH)** позволяет привязывать скрипты жизненного цикла к любой колонке Kanban-доски.
> Хуки выполняются бэкендом на Rust при перемещении карточек между стадиями и автоматизируют рутину: создание ворктри, запуск тестов, уведомления и слияние веток.

---

## 1. Как работают переходные скрипты

Когда задача перемещается в колонку (вручную через Drag-and-Drop или автоматически по вердикту агента):

1. **Жизненный цикл переходов карточки (`on_leave`/`on_finish` → `sensor` → `on_enter`)**:
   - При вызове `advance_task` или перемещении задачи выполняются:
     1. Хук выхода текущей колонки (`on_leave` / `on_finish` / `on_exit`): выполняется в режиме **best-effort** — сбой или ненулевой код возврата логируются (`WARN`), но **не блокируют** переход карточки.
     2. Сенсор целевой колонки (`gatekeeperSensor`): валидирует тесты, компиляцию и mergeability; ненулевой exit code **блокирует** переход.
     3. Хук входа целевой колонки (`on_enter`): выполняет скрипты валидации и подготовки (например, `gatekeeper_sensor.bat` или создание ворктри); сбой скрипта **блокирует** переход.
   - Все хуки полностью видимы в интерфейсе: отображаются интерактивными бейджами в шапке колонки Kanban-доски (`StageHookBadge`), посылают системные события `hook:start` / `hook:finish` и сохраняют результат выполнения в `task.hook_states`.
   - При сбое блокирующих проверок (`gatekeeperSensor` или `on_enter`) переход блокируется, карточка остается на месте со статусом `running` (при наличии autorun) и вердиктом `CHANGES_REQUESTED`, новые подзадачи **строго не создаются**, а в диалог карточки отправляется steer-сообщение с детальным логом ошибки для исправления.
2. **Схема объявления хуков**:
     - **Объектный формат с описанием (рекомендуемый)**:
       ```json
       "hooks": {
         "on_leave": {
           "script": ".cvh/hooks/on_review_finish.bat",
           "description": "Сливает утвержденную feature-ветку в main, отправляет в origin/main и удаляет изолированный worktree"
         }
       }
       ```
       Поле `description` отображается при наведении курсора на индикатор хука на карточке Kanban-доски, объясняя его назначение и производимые действия.
     - **Строковый формат (компактный)**:
       ```json
       "hooks": {
         "on_leave": ".cvh/hooks/create_worktree.bat"
       }
       ```
3. **Реиспользование подзадач при отклонении (Generic Rejection Handover)**:
   - При вызове `reject_task` подзадача ревьюера остается на этапе ревью со статусом `idle`, а предыдущая подзадача разработчика возобновляется в `dev` со статусом `running` и текстом замечаний.
   - При повторном вызове `advance_task` после правок CVH возобновляет существующую подзадачу ревьюера, передавая ей отчет об исправлениях, без дублирования карточек.
4. **Контроль таймаута и изоляция процессов**:
   - На выполнение хука выделяется таймаут (по умолчанию 180 секунд).
   - Если скрипт зависает, бэкенд принудительно убивает все дерево дочерних процессов (`taskkill /F /T /PID <pid>` на Windows) для предотвращения зомби-процессов.

---

## 2. Переменные окружения (Environment Variables)

При вызове любого хука Rust-бэкенд инжектирует в процесс исчерпывающий контекст задачи через системные переменные окружения:

| Переменная | Описание | Пример значения |
|---|---|---|
| `CVH_TASK_ID` | Уникальный буквенно-цифровой идентификатор задачи | `CVH-042` |
| `CVH_TASK_TITLE` | Человекочитаемое название карточки | `Оптимизация стриминга SSE` |
| `CVH_WORKTREE_PATH` | Абсолютный путь к рабочей директории ворктри | `C:\Repo\worktrees\CVH-042` |
| `CVH_BRANCH` | Название целевой feature-ветки в Git | `feat/CVH-042` |
| `CVH_STAGE_ID` | Идентификатор текущей целевой колонки | `dev`, `review`, `ready` |
| `CVH_BOARD_ID` | Идентификатор активной Kanban-доски | `development`, `modeler` |
| `CVH_VERDICT` | Вердикт аудита (если задача выходит из ревью) | `APPROVED`, `CHANGES_REQUESTED` |

---

## 3. Готовый пример: Автоматическое создание ворктри (`create_worktree`)

Этот скрипт автоматически создает изолированный `git worktree` и feature-ветку при выходе карточки из бэклога в разработку (`dev`).

### Вариант на Windows Batch (`.cvh/hooks/create_worktree.bat`):
```bat
@echo off
REM ==============================================================================
REM CanbanVoimaxHarness Hook: create_worktree.bat
REM Создание изолированного ворктри для безопасной разработки
REM ==============================================================================

echo [CVH Hook] Entering Development stage for %CVH_TASK_ID%: %CVH_TASK_TITLE%

REM Проверяем, существует ли уже ворктри
if not exist "worktrees\%CVH_TASK_ID%" (
    echo [CVH Hook] Scaffolding isolated git worktree: worktrees\%CVH_TASK_ID%
    git worktree add -b feat/%CVH_TASK_ID% worktrees/%CVH_TASK_ID% main >nul 2>&1
    if errorlevel 1 (
        REM Если ветка feat/<id> уже была создана ранее, цепляемся к ней
        git worktree add worktrees/%CVH_TASK_ID% feat/%CVH_TASK_ID% >nul 2>&1
    )
)

if exist "worktrees\%CVH_TASK_ID%" (
    echo [CVH Hook] Worktree successfully verified at worktrees\%CVH_TASK_ID%
    REM Протокол передачи ворктри в ядро харнесса (stdout):
    echo CVH_WORKTREE=worktrees\%CVH_TASK_ID%
    echo CVH_BRANCH=feat/%CVH_TASK_ID%
    exit /b 0
) else (
    echo [CVH Hook] ERROR: Failed to scaffold worktree for %CVH_TASK_ID% >&2
    exit /b 1
)
```

### Вариант на Bash (`.cvh/hooks/create_worktree.sh`):
```bash
#!/usr/bin/env bash
set -e

echo "[CVH Hook] Entering Development stage for $CVH_TASK_ID: $CVH_TASK_TITLE"

if [ ! -d "worktrees/$CVH_TASK_ID" ]; then
    echo "[CVH Hook] Scaffolding isolated git worktree: worktrees/$CVH_TASK_ID"
    git worktree add -b "feat/$CVH_TASK_ID" "worktrees/$CVH_TASK_ID" main 2>/dev/null || \
    git worktree add "worktrees/$CVH_TASK_ID" "feat/$CVH_TASK_ID" 2>/dev/null
fi

if [ -d "worktrees/$CVH_TASK_ID" ]; then
    echo "[CVH Hook] Worktree successfully verified at worktrees/$CVH_TASK_ID"
    echo "CVH_WORKTREE=worktrees/$CVH_TASK_ID"
    echo "CVH_BRANCH=feat/$CVH_TASK_ID"
    exit 0
else
    echo "[CVH Hook] ERROR: Failed to scaffold worktree for $CVH_TASK_ID" >&2
    exit 1
fi
```

---

## 4. Готовый пример: Слияние и очистка при завершении ревью (`on_review_finish`)

Когда задача получает вердикт `APPROVED` и переходит дальше, хук автоматически сливает ветку в `main`, пушит в удаленный репозиторий и удаляет временный ворктри.

### Скрипт слияния и очистки (`.cvh/hooks/on_review_finish.bat`):
```bat
@echo off
echo [CVH Hook] Finishing task %CVH_TASK_ID% with verdict: %CVH_VERDICT%

REM Проверяем, существует ли ворктри
if exist "worktrees\%CVH_TASK_ID%" (
    echo [CVH Hook] Merging feat/%CVH_TASK_ID% into main and pushing...
    git checkout main
    git merge "feat/%CVH_TASK_ID%" --no-edit
    git push origin main
    git worktree remove "worktrees\%CVH_TASK_ID%" --force >nul 2>&1
    git branch -d "feat/%CVH_TASK_ID%" >nul 2>&1
)

echo [CVH Hook] Worktree cleaned and merged. Task %CVH_TASK_ID% is complete.
exit /b 0
```

---

## 5. Подключение хуков в `pipeline.json` или `boards.json`

Чтобы связать хук со стадией, добавьте поле `hooks` в описание колонки:

```json
{
  "id": "backlog",
  "title": "Бэклог",
  "hooks": {
    "on_leave": {
      "script": ".cvh/hooks/create_worktree.bat",
      "description": "Создает изолированный git worktree и feature-ветку feat/<task-id>"
    }
  }
},
{
  "id": "review",
  "title": "Ревью",
  "hooks": {
    "on_enter": {
      "script": ".cvh/hooks/gatekeeper_sensor.bat",
      "description": "Запуск gatekeeper-сенсоров: проверка git mergeability, cargo check, Vitest, tsc и Oxlint"
    },
    "on_leave": {
      "script": ".cvh/hooks/on_review_finish.bat",
      "description": "Сливает утвержденную ветку в main и удаляет ворктри"
    }
  }
}
```

С этой конфигурацией вам достаточно перетащить карточку на доске — CVH сам подготовит ветку, создаст ворктри и запустит нужного агента!
