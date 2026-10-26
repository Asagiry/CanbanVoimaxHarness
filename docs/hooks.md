# ⚡ Автономные скрипты перехода на колонках (docs/hooks.md)

> **CanbanVoimaxHarness (CVH)** позволяет привязывать скрипты жизненного цикла к любой колонке Kanban-доски.
> Хуки выполняются бэкендом на Rust при перемещении карточек между стадиями и автоматизируют рутину: создание ворктри, запуск тестов, уведомления и слияние веток.

---

## 1. Как работают переходные скрипты

Когда задача перемещается в колонку (вручную через Drag-and-Drop или автоматически по вердикту агента):

1. **Сенсор-привратник (`gatekeeperSensor`)**:
   - Если на колонке задан сенсор (например, `cargo check` или `pnpm test`), он выполняется **до** входа в колонку внутри каталога задачи.
   - Если сенсор завершился с кодом ошибки, переход немедленно блокируется, карточка остается на месте со статусом `running` (при наличии autorun) и вердиктом `CHANGES_REQUESTED`, а в диалог задачи отправляется steer-сообщение с детальным логом ошибки.
2. **Хук входа (`hooks.on_enter`) и хук выхода (`hooks.on_leave`)**:
   - При вызове `advance_task` или перемещении задачи выполняются:
     1. Хук выхода текущей колонки (`on_leave` / `on_exit`).
     2. Gatekeeper-сенсор целевой колонки.
     3. Хук входа целевой колонки (`on_enter`).
   - При сбое любого хука переход блокируется, новые подзадачи **строго не создаются**, а исполнителю передается ошибка для автономного исправления.
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
| `CVH_VERDICT` | Вердикт аудита (если задача выходит из ревью) | `APPROVED`, `CHANGES_REQUESTED` |

---

## 3. Готовый пример: Автоматическое создание ворктри (`on_enter_dev`)

Этот скрипт автоматически создает изолированный `git worktree` и feature-ветку при перемещении карточки из бэклога в разработку (`dev`).

### Вариант на Windows Batch (`.cvh/hooks/on_enter_dev.bat`):
```bat
@echo off
REM ==============================================================================
REM CanbanVoimaxHarness Hook: on_enter_dev.bat
REM Создание изолированного ворктри при входе в стадию разработки
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
    exit /b 0
) else (
    echo [CVH Hook] ERROR: Failed to scaffold worktree for %CVH_TASK_ID% >&2
    exit /b 1
)
```

### Вариант на PowerShell (`.cvh/hooks/on_enter_dev.ps1`):
```powershell
# ==============================================================================
# CanbanVoimaxHarness Hook: on_enter_dev.ps1
# ==============================================================================
$taskId = $env:CVH_TASK_ID
$title  = $env:CVH_TASK_TITLE
$wtPath = "worktrees/$taskId"
$branch = "feat/$taskId"

Write-Host "[CVH Hook] Preparing isolated worktree for $taskId ($title)..."

if (-not (Test-Path $wtPath)) {
    Write-Host "[CVH Hook] Creating git branch '$branch' and worktree at '$wtPath'..."
    git worktree add -b $branch $wtPath main 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) {
        # Повторная попытка для существующей ветки
        git worktree add $wtPath $branch 2>&1 | Out-Null
    }
}

if (Test-Path $wtPath) {
    Write-Host "[CVH Hook] Worktree ready at $wtPath."
    exit 0
} else {
    Write-Error "[CVH Hook] Failed to create worktree at $wtPath!"
    exit 1
}
```

---

## 4. Готовый пример: Очистка ворктри при сдаче (`on_finish_done`)

Когда задача получает вердикт `APPROVED` и переходит в колонку `ready` (Готово), хук автоматически закрывает ворктри и удаляет временную ветку.

### Скрипт очистки (`.cvh/hooks/on_finish_done.bat`):
```bat
@echo off
echo [CVH Hook] Finishing task %CVH_TASK_ID% with verdict: %CVH_VERDICT%

if exist "worktrees\%CVH_TASK_ID%" (
    echo [CVH Hook] Cleaning up temporary worktree: worktrees\%CVH_TASK_ID%
    git worktree remove "worktrees\%CVH_TASK_ID%" --force >nul 2>&1
    git branch -D feat/%CVH_TASK_ID% >nul 2>&1
)

echo [CVH Hook] Worktree cleaned. Task %CVH_TASK_ID% is ready.
exit /b 0
```

---

## 5. Подключение хуков в `pipeline.json` или `boards.json`

Чтобы связать хук со стадией, добавьте поле `hooks` в описание колонки:

```json
{
  "id": "dev",
  "title": "В разработке",
  "hooks": {
    "on_enter": ".cvh/hooks/on_enter_dev.bat"
  }
},
{
  "id": "ready",
  "title": "Готово",
  "gatekeeperSensor": "cargo check",
  "hooks": {
    "on_enter": ".cvh/hooks/on_finish_done.bat"
  }
}
```

С этой конфигурацией вам достаточно перетащить карточку на доске — CVH сам подготовит ветку, создаст ворктри и запустит нужного агента!
