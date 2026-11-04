�ии Live Diff и Cortex Memory Ledger полностью реализованы в кодовой базе).

---

## 0. Как сверялся гайд (что прочитано кодом, а не угадано)

- `src-tauri/src/config/` (`models.rs`, `tool_policy.rs`, `discovery.rs`, `scaffold.rs`, `io.rs`) —
  `scaffold_cvh_project`, `minimal_pipeline_config`, `default_pipeline_config`,
  `get_canonical_board_columns`, `get_board_template_columns`, `ToolPolicyConfig` (Single Authoritative Layer),
  `AgentSetting`, `agent_persona_path` / `agent_rules_dir` / `agent_hooks_dir` / `agent_setting_path`,
  `resolve_hook_path`, `load_boards_config` / `save_boards_config`, `tool_policy_path`,
  `load_tool_policies` / `save_tool_policies`.
- `src-tauri/src/commands/` (`state.rs`, `tasks.rs`, `tasks/*`, `boards.rs`, `cortex.rs`, `dialogs.rs`, `git.rs`, `llm.rs`, `notifications.rs`, `projects.rs`, `tools.rs`) —
  `create_task`, `update_task`, `move_task_stage`, `run_agent`, `steer_agent`, `answer_agent_question`, `create_subtask`,
  `list_agent_personas_in` / `create_agent_persona` / `delete_agent_persona` / `rename_agent_persona`,
  `list_custom_tools_in` / `save_custom_tool_file` / `delete_custom_tool_file`,
  `get_tool_policies` / `save_tool_policies`, `compact_context`, `get_task_git_diff`, `get_task_image_diff`,
  `load_subagent_dialog`, `list_subagents`, `get_cortex_tree` / `read_cortex_file` / `save_cortex_file`.
- `src-tauri/src/db/` (`pool.rs`, `task_repo.rs`, `dialog_repo.rs`, `telemetry_repo.rs`) —
  R2D2 SQLite connection pool, WAL mode, ACID dialog item transactions, telemetry aggregation.
- `src-tauri/src/agent/` (`react_engine.rs`, `session_registry.rs`, `compactor.rs`, `sanitizer.rs`, `telemetry.rs`, `text_match.rs`, `tool_schemas.rs`, `tools/*`) —
  `compile_primed_system_prompt_with_tools`, `get_standard_tools`, `ReActEngine<Driver>`,
  `session_registry` (child subagent sessions & Tokio completion channels),
  `maybe_spill_output` (500 строк / 64 КБ; 2000 строк / 128 КБ для `read_file`),
  `execute_tool` (Single Authoritative Layer gate, custom CLI tools).
- `src-tauri/src/hooks.rs`, `git.rs`, `udf.rs`, `logger.rs`, `error.rs` —
  `execute_hook` (env `CVH_*`, интерпретаторы, таймауты, process tree kill на Windows),
  `execute_sensor_command`, `HookContext`, git worktrees & diffs, SQLite UDFs.
- `src-tauri/src/lib.rs` — полный список зарегистрированных Tauri-команд.
- `src/stores/` (`boardStore.ts`, `drawerStore.ts`, `cortexStore.ts`, `settingsStore.ts`, `themeStore.ts`, `toolPolicyStore.ts`, `agentSessionStore.ts`, `slices/*`) —
  реактивные сторы, координация колонок, Single Authoritative Layer в матрице тулов.
- Живой `.cvh/` этого репозитория (`pipeline.json`, `boards.json`, `config.json`, `harness.db`,
  `settings/tool-policy.json`, `agents/<role>/persona.md`, `rules/`, `spill/`, `worktrees/`)
  и `src/components/` (`board/`, `card/`, `drawer/`, `cortex/`, `settings/`, `layout/`).

---

## 1. Что такое харнесс и `.cvh` за 2 минуты

**CanbanVoimaxHarness (CVH)** — десктопный оркестратор ИИ-агентов (Rust + Tauri v2 + React 19).
Фундаментальный инвариант: **колонки Kanban-доски — это агенты**. У колонки есть поле `persona: "<role>"`;
перетаскивание карточки в такую колонку запускает `run_agent` с системным промптом из
`.cvh/agents/<role>/persona.md`. Подробнее философия расписана в `docs/board.md` и `.cvh/rules/` — здесь только механика.

**`.cvh/`** — вся конфигурация харнесса для одного проекта (лежит в корне репозитория проекта).
Открытие папки проекта в CVH (`select_project_folder` / `set_active_project`) вызывает
`scaffold_cvh_project` — **работает**: недостающие файлы/папки досоздаются, существующие не трогаются.

**Свежий старт (пустой проект):**

1. Скаффолд создаёт ровно две колонки — `backlog` (без персоны, первая) и `ready` (терминальная, последняя).
   Это `minimal_pipeline_config` — **работает** (тест `test_scaffold_minimal_backlog_ready_only`).
2. Папка `.cvh/agents/` — **намеренно пустая, ни одной персоны не сидируется**
   (тест `test_scaffold_cvh_project_pure_empty_structure`). **Первого агента создаёшь сам** —
   через ПКМ по заголовку колонки → смена персоны → «новый агент», либо командой `create_agent_persona`.
3. Если корень — git-репо (есть `.git`), `backlog` сразу получает
   `hooks: { "on_enter": "builtin:create_worktree" }`, иначе `hooks: {}` — **работает**.
4. Дальше: создаёшь задачу в `backlog`, добавляешь колонку с `persona`, перетаскиваешь карточку —
   агент запускается в изолированном git worktree.

---

## 2. Справочник `.cvh` пофайлово

### 2.1 Карта каталога

```text
.cvh/
├── harness.db             # высокопроизводительная SQLite база (WAL-режим): диалоги и стриминг
├── config.json            # проект: имя, модель/endpoint по умолчанию, providers
├── pipeline.json          # пайплайн: columns (+зеркало boards/activeBoardId) — см. §2.2
├── boards.json            # мульти-доски: { activeBoardId, boards[] } — см. §2.3
├── tasks.json             # задачи: массив Task (CVH-001, CVH-002, …) — см. §2.4
├── agents/
│   └── <role>/            # модульная папка роли (канонический путь):
│       ├── persona.md    # системный промпт роли с YAML frontmatter (icon, description)
│       ├── setting       # JSON: {model?, temperature?, context_window?, icon?}
│       ├── rules/*.md    # правила роли, компилируемые в системник по алфавиту
│       └── hooks/        # on_enter.* / on_finish.* — fallback для hooks колонок
├── settings/
│   └── tool-policy.json   # ЕДИНСТВЕННЫЙ канонический файл матрицы тулов
├── tools/                 # кастомные тулы: .cvh/tools/<name>.json
├── hooks/                 # общие скрипты переходов (.bat/.ps1/.sh)
├── spill/                # spill-логи больших выводов тулов (>500 строк / >64KB; >2000 строк / >128KB для read_file)
└── worktrees/            # git worktrees задач (в .gitignore)
```

> **Диалоги и активность задач**: хранятся строго в SQLite-базе `.cvh/harness.db`
> (таблица `dialog_items` с индексами и WAL-журналированием). Легаси-файлы `.cvh/dialogs/`
> и `.cvh/tasks/<id>/dialog.json` ликвидированы в пользу атомарных ACID-транзакций.

> **Скрипты переходов**: общие хуки лежат в `.cvh/hooks/` (`gatekeeper_sensor.bat`, `on_review_finish.bat` и др.).
> Если хук не настроен или файл отсутствует, бэкенд безопасно пропускает вызов (`if hook_path.exists()`).

### 2.2 `pipeline.json` — колонки, hooks, autoRun/terminal — **работает**

Полная схема колонки (`PipelineColumn`, `config.rs`):

| Поле | Тип | Назначение |
|---|---|---|
| `id` | string | уникальный id; `backlog` всегда первая, `ready` всегда последняя (см. нормализацию ниже) |
| `title` | string | заголовок колонки |
| `color` / `icon` | string? | цвет, иконка Lucide |
| `persona` | string? | роль агента; `null` = колонка-хранилище, агент не запускается |
| `onEnterScript` | string? | **отдельное** поле-скрипт (не `hooks`!): выполняется в worktree-or-root, best-effort (не блокирует переход) |
| `contextWindow` | u64? | лимит контекста стадии (информирует `ContextGauge`) |
| `modelOverride` | string? | модель стадии (приоритет №4, см. §5) |
| `reasoningEffort` | string? | пробрасывается в LLM-запрос как есть |
| `temperature` | f32? | температура стадии (приоритет №3, см. §5) |
| `gatekeeperSensor` | string? | команда-сенсор; **блокирует** вход при ненулевом exit code |
| `allowedTools` | string[]? | декларативный список (см. статус ниже) |
| `hooks` | `{ on_enter?, on_finish? }` | пути скриптов ИЛИ `"builtin:create_worktree"` |
| `autoRun` | bool? | автозапуск агента при входе; default = `persona != null` |
| `terminal` | bool? | терминальная колонка: задача переходит в `done` |

Стабильная нормализация порядка (`normalize_pipeline_columns`, `commands.rs`) — **работает**:
`backlog` — всегда первая (если есть), `ready` — всегда последняя (если есть),
остальные сохраняют относительный порядок; **дублирующийся `id` — ошибка** (переход/сохранение отклоняется).

Сколько «дефолтов» существует (все — **работает**, но это разные функции, не путайте):

- `minimal_pipeline_config()` — скаффолд свежего проекта: только `backlog` + `ready`.
- `default_pipeline_config()` — 5 стадий `backlog → dev → review → retro → ready`; используется как
  fallback при битом/отсутствующем `pipeline.json` и как база `default_boards_config()`.
- `get_canonical_board_columns()` — 3 колонки `backlog → dev → ready` (русские титулы, `ready.terminal=true`);
  это то, что получает новая доска по умолчанию (`create_board` с `template: "canonical"`).
- `get_board_template_columns("modeler"|"3d"|"3d-modeler")` — 4 стадии
  `concept → modeling → texturing → ready`. Любой другой `template` → canonical.
- Фронтенд-стор держит ещё два набора: `CANONICAL_BOARD_COLUMNS` (3 колонки с сенсорами/хуками)
  и `INITIAL_COLUMNS` в `mockData.ts` (5 колонок с `wipLimit`) — это fallback для web-режима,
  бэкенд их не читает.

> Расхождение с `docs/board.md` (§3.2): дока обещает шаблоны `development` (5 стадий),
> `kanban`/`simple` (`todo → in_progress → ready`) и `empty` (`backlog → ready`) —
> **частично**: в коде таких веток `template` нет, всё не-`modeler` падает в canonical из 3 колонок.
> `empty` фактически покрывается скаффолдом (минимальные 2 колонки), а не `create_board`.

### 2.3 `boards.json` — мульти-доски — **работает**

```json
{ "activeBoardId": "development", "boards": [{ "id": "development", "name": "Code-dev", "columns": [ ... ] }] }
```

- `list_boards` / `get_active_board_id` / `create_board(name, template?)` / `save_board` /
  `delete_board` / `set_active_board` — **работает**.
- `create_board`: id — слагифицированное имя (`slugify_board_name`, с транслитерацией кириллицы,
  суффикс `-2`, `-3` при коллизии) — **работает**.
- `delete_board`: единственную доску удалить нельзя; задачи удалённой доски переезжают на новую активную — **работает**.
- Синхронизация двусторонняя (`save_boards_config` ↔ `save_pipeline_config`): активная доска
  зеркалируется в `pipeline.json` (поля `boards`, `activeBoardId`, плюс `columns`/`title` = активная доска) — **работает**.
- Задачи привязаны к доске полем `board_id` (пустое нормализуется в `"development"`).

### 2.4 `config.json` — **работает**

`{ name, version, description?, activePipeline?, defaultModel?, apiBaseUrl?, toolPermissions?, providers }`.
`toolPermissions` (legacy-модель `ToolPermissions { globalAllowedTools, stageAllowedTools }`) парсится,
но в исполнении агента не участвует (см. §4, статус). Провайдеры (`antigravity`/`deepseek`/`anthropic`)
в этом репозитории — декларативные; реальный вызов идёт на `baseUrl` из LLM-настроек (см. §5).

### 2.5 `tasks.json` — **работает**

Массив `Task`: `id` (`CVH-003`, сабтаски `CVH-012.1`), `title`, `description`, `stageId`,
`boardId`, `worktreePath?`, `branch?`, `verdict?`, `customFlow?`, `status?`, `model?`,
`parentId?`, `subtaskIndex?`, `waitingForSubtaskId?`, `coverImage?`, `attachments?`,
`todos?` (`[{id, content, status}]`), `createdAt`/`updatedAt` (ms).
ID монотонные `max+1`, **никогда не переиспользуются** после удаления (чтобы не воскрешать
старые диалоги из `dialogs/`/`localStorage`) — **работает** (`next_task_num`; фронт дублирует
логику счётчиками в `localStorage` + `syncTaskSeqWithTasks`).

### 2.6 `settings/tool-policy.json` — **работает**

Единственный канонический путь: `.cvh/settings/tool-policy.json` (`tool_policy_path`, без legacy-фолбэков).
Формат — см. §4. Читают/пишут: агентный цикл (`load_tool_policies`), `get/save_tool_policies`,
модалка матрицы, скаффолд (сидит `ToolPolicyConfig::default()`).

### 2.7 `agents/<role>/` — персона, правила, хуки, setting — **работает**

Канонический модульный формат персоны, который читает бэкенд: **директория `.cvh/agents/<role>/`**:
- **`persona.md`**: Системный промпт роли с YAML frontmatter (`icon`, `color`, `description`, `scope`).
  Если `persona.md` отсутствует, проверяется fallback `agent.md`. Плоские файлы `.cvh/agents/<role>.md` полностью упразднены.
- **`rules/*.md`**: Модульные правила роли, компилируемые в системный промпт **по алфавиту имени файла** — **работает**.
- **`hooks/`**: Файлы `on_enter.*` / `on_finish.*`; используются как **fallback по basename**:
  `resolve_hook_path` берёт настроенный путь, а если файла нет — `.cvh/agents/<persona>/hooks/<basename>` — **работает**.
- **`setting`**: JSON **без расширения**: `{ model?, temperature?, context_window?|contextWindow, allowed_tools?|allowedTools, icon?, color? }`.
  Отсутствует/пуст/бит → дефолт без ошибки (с варном в лог) — **работает**.

Управление персонами через IPC-команды — **работает**:
- `create_agent_persona(role)`: валидация `^[a-z0-9][a-z0-9-]*$`, скаффолдит `<role>/persona.md`, `<role>/rules/`, `<role>/hooks/` и `<role>/setting`.
- `delete_agent_persona(role)`: удаляет директорию роли (защита системной роли `cvh-dev`).
- `rename_agent_persona(old_role, new_role)`: атомарно переименовывает папку агента.
- `set_agent_persona_color(role, color)` / `set_agent_persona_icon(role, icon)`: обновляет frontmatter и `setting`.
- `list_agent_personas`: сканирует поддиректории `.cvh/agents/<role>/`, читает метаданные и возвращает список доступных персон.

### 2.8 `tools/*.json` — формат кастомного тула — **работает**

```json
{
  "name": "bake_colliders",
  "description": "Сгенерировать упрощённые коллайдеры для воксельной сетки",
  "parameters": { "type": "object", "properties": { "mesh_path": { "type": "string" } }, "required": ["mesh_path"] },
  "command": "python scripts/bake_colliders.py {{args}}"
}
```

- Имя строго `^[a-z0-9_]+$`, `parameters` обязан быть JSON-объектом, иначе `save_custom_tool` вернёт ошибку — **работает**.
- CRUD-команды `list_custom_tools` / `save_custom_tool` / `delete_custom_tool` — **работает**;
  листинг сортирован по `name`, битые JSON молча пропускаются.
- Исполнение: `{{args}}` (двойные скобки — буквально так в коде:
  `ct.command.replace("{{args}}", &args.to_string())`) заменяется сериализованным JSON аргументов вызова,
  команда бежит в `worktree` с таймаутом 60с — **работает**.
- В матрице кастомные тулы участвуют наравне со стандартными (фильтр тот же), `todo_write`-исключений нет.

### 2.9 `harness.db` (SQLite), `spill/`, `worktrees/` — **работает**

- Диалоги и активность задач — SQLite база данных `.cvh/harness.db` с WAL-журналированием (таблица `dialog_items` с композитным индексом по `task_id, created_at`).
  Операции `save_task_activity` / `load_task_activity` / `clear_task_activity` (с обратно совместимыми алиасами `save_task_dialog` / `load_task_dialog` / `clear_task_dialog`) выполняются атомарно в транзакциях (`src-tauri/src/db.rs`); легаси-файлы `.cvh/dialogs/` и `.cvh/tasks/<taskId>/dialog.json` ликвидированы — **работает**.
- `spill/` — см. §5 (порог 100 строк / 8 КБ, `HEAD 20 + TAIL 20`) — **работает**.
- `worktrees/` — git worktrees задач; в `.gitignore` (скаффолд дописывает `.cvh/llm.json`, `llm.json`, `.cvh/worktrees/`) — **работает**.
  Удаление — `cleanup_task_worktree` (worktree + ветка `feat/<slug>`, best-effort) — **работает**;
  из UI вызывается при удалении топ-уровневой задачи (сабтаски делят worktree родителя и его не чистят).

---

## 3. Жизненный цикл задачи

### 3.1 Создание: `create_task(title, description, stage_id, board_id?, skip_worktree?)` — **работает**

1. `board_id` пустой → активная доска. ID — `CVH-NNN` (max+1).
2. Новая задача стартует **без** `worktreePath`/`branch`. Дальше — on_enter колонки `stage_id`
   (поиск колонки: сначала доска `board_id`, потом `pipeline.json`):
   - `trigger_column_on_enter_script` — поле `onEnterScript`, cwd = worktree-если-есть иначе корень, best-effort;
   - `hooks.on_enter`: `"builtin:create_worktree"` → только builtin ниже; иначе shell-хук
     (primary-путь, иначе fallback `.cvh/agents/<persona>/hooks/<basename>`; отсутствие файла — пропуск);
   - stdout хука парсится протоколом `CVH_WORKTREE=` / `CVH_BRANCH=` (см. §3.4);
   - `needs_builtin_worktree`: `skip_worktree=true` (чекбокс «без worktree» в модалке) отключает **только builtin**,
     уже назначенный worktree (хоть из stdout хука) тоже отключает builtin;
     условие builtin: явный маркер **ИЛИ** дефолт бэклога (`id == "backlog"` без явного `on_enter`).
   - builtin: `create_worktree(root, lowercase(task.id), "HEAD")` + ветка `feat/<slug>` если пуста; ошибки — варн.
3. Если колонка вообще не найдена, но `stage_id == "backlog"` и нет `skip` — builtin всё равно срабатывает.

### 3.2 Перемещение: `move_task_stage(task_id, target_stage_id, verdict?)` — **работает**

Строгий порядок (колонки ищутся с учётом `board_id` задачи):

1. **on_finish source-колонки** — только shell (`hooks.on_finish`), best-effort: неуспех **не блокирует**
   переход, stdout пишет `worktree/branch` через тот же протокол. Builtin здесь нет.
2. **Gatekeeper sensor** target-колонки (`gatekeeperSensor`, выполняется в worktree-или-корне) —
   неуспех **блокирует** переход с ошибкой. Таймаут сенсора 300с.
3. **on_enter target-колонки** — shell-хук; неуспех **блокирует** переход
   (`Stage transition hook rejected transition`). `builtin:create_worktree` — не shell, а builtin ниже.
   Shell-приоритет: путь из stdout отключает builtin. `skip` здесь всегда `false`.
4. **`onEnterScript`** target-колонки — best-effort, после хука.
5. Обновление `stage_id` (+ `verdict`, если передан), `updated_at`, persist.

Фронт (`boardStore.moveTask`): сначала пробует `move_task_stage` и маппит **возвращённую** задачу
(чтобы не потерять назначенные хуками `worktreePath`/`branch`), затем — если у целевой колонки
`autoRun ?? !!persona` — вызывает `run_agent`. Фолбэк для web-режима — локальное перемещение + `save_tasks`.

### 3.3 Агент в worktree → финиш → колонка справа: `run_agent(task_id, prompt?)` — **работает**

- Steer-логика: пустое `description` + непустой `prompt` → становится описанием;
  иначе описание не мутируется, а `prompt` уходит отдельным сообщением `[USER STEER]: ...`.
- Колонка ищется по доске задачи → `pipeline.json` → заглушка. Нет `persona` → `"developer"`.
- **Worktree fallback**: путь пуст или отсутствует на диске → агент работает **в корне репо**
  (создавать worktree здесь запрещено, в лог — WARN). Агент уведомляется маркером, дописанным к описанию:
  `\n\n[HOOK] worktree: <path> — твой изолированный workdir, работаешь тут.` — только если worktree есть.
- Модель: `task.model` → `llm_settings.model` → `agents/<role>/setting.model` → `col.modelOverride` →
  `project.defaultModel` → хардкод `gemini-3.8-flash-high`.
  Температура: `llm_settings` → `agent setting` → `column`. `baseUrl`: `llm_settings` → проект → `http://127.0.0.1:8045/v1`.
- Статус `running` пишется в `tasks.json` + событие `agent:status`.
- Финиш: парсинг вердикта **по вхождению верхнего регистра** в финальном ответе:
  `CHANGES_REQUESTED` / `APPROVED_WITH_CAVEATS` / `APPROVED`. Дальше — иерархия сабтасок:
  родитель остаётся на месте (`waiting_subtask`), в следующей колонке создаётся `CVH-012.N`
  (наследует worktree/branch; вердикты APPROVED, APPROVED_WITH_CAVEATS, CHANGES_REQUESTED
  имеют приоритет над «следующей колонкой по порядку»); `CHANGES_REQUESTED` разблокирует родителя в `idle`;
  терминальная колонка (`terminal: true`) собирает всё в `done` и двигает родителя в неё.

### 3.4 Промпт-маркер `[HOOK]` и stdout-протокол `CVH_WORKTREE`/`CVH_BRANCH` — **работает**

Два разных механизма, не путайте:

- `[HOOK] worktree: ...` — строка, которую `run_agent` дописывает к описанию задачи перед отправкой в LLM.
  Это подсказка модели, где её изолированный workdir.
- `CVH_WORKTREE=<path>` (+ опционально `CVH_BRANCH=<branch>`) — построчный stdout-протокол хуков
  (`apply_worktree_from_hook_stdout`, общий для on_enter и on_finish): пишет `task.worktree_path`
  (перезаписывает) и `task.branch` (только если пуст). Хук может вывести относительный или абсолютный путь.

Env хуков (`execute_hook`, `hooks.rs`) — **работает**: `CVH_TASK_ID`, `CVH_TASK_TITLE`,
`CVH_WORKTREE_PATH`, `CVH_BRANCH`, опционально `CVH_VERDICT`, `CVH_STAGE_ID`, `CVH_BOARD_ID` + `extra_env`.
Интерпретатор по расширению: `.bat`/`.cmd` → `cmd.exe /C`, `.ps1` → `powershell -NoProfile -ExecutionPolicy Bypass -File`,
`.py` → `python`, `.sh` → `sh`, прочее — прямой запуск. Таймаут хуков 180с, сенсоров 300с, зависшие деревья убиваются
(`taskkill /F /T` на Windows, `kill -9` иначе).

> Всегда дописывайте `echo CVH_WORKTREE=...` / `echo CVH_BRANCH=...` в скрипты переходов, создающие ворктри, чтобы харнесс подхватил изолированную директорию.

### 3.5 Каталог Tauri IPC-команд управления задачами и агентами — **работает**

- **`update_task(task)`**: Обновление полей задачи (title, description, tags, model_override).
- **`answer_agent_question(task_id, answer)`**: Ответ пользователя на интерактивный вопрос агента (`question` / `ask_user`), возобновляющий приостановленный цикл ReAct.
- **`get_task_image_diff(task_id, relative_path)`**: Расчет визуального диффа графических ассетов (side-by-side, swipe-slider, blend).
- **`load_subagent_dialog(task_id, subagent_id)`**: Загрузка изолированной истории диалога и мыслей дочернего субагента из SQLite.
- **`list_subagents(task_id)`**: Возврат списка всех запущенных и завершенных субагентов для задачи.
- **`delete_agent_persona(role)`**: Удаление папки персоны `.cvh/agents/<role>/` с защитой встроенного агента `cvh-dev`.
- **`rename_agent_persona(old_role, new_role)`**: Атомарное переименование роли агента.

---

## 4. Матрица тулов: Single Authoritative Layer, todo_write, кастомные

### 4.1 Single Authoritative Layer — **работает**

`effective(tool, stage) = stage_override[stage][tool] ?? global_enabled[tool] ?? true`.
- Если на этапе колонки явно задан оверрайд (`true` или `false`), это решение является **окончательным** (Single Authoritative Layer).
- Если оверрайд для данного тула отсутствует, в силу вступает значение из `global_enabled`.
- Для иерархии субагентов (`subagent`, `subagent_bg`, `collect_subagent`, `subagent_list`) действует мастер-свитч стадии `subagent`: если он `false`, всё семейство блокируется.
- Алиасы SWE-bench нормализуются до проверки: `bash → execute_command`, `str_replace → edit_file` (`normalize_tool_name`).

### 4.2 Zero-trust gate — **работает**

`execute_tool`: сначала `is_tool_allowed(policy, stage, tool)` → `PermissionDenied`,
затем пересечение с `agents/<role>/setting.allowed_tools` → `PermissionDenied`
(`None` = без ограничений, `Some([])` = запрещено всё; alias-aware).
Отключённые тулы **вырезаются из схемы** запроса к LLM (экономия токенов + нет галлюцинаций),
пустой набор → поле `tools` опускается (`None`, т.к. `"tools": []` режут некоторые прокси).
Определения тулов сортируются по имени для prefix caching — **работает**.

### 4.3 Стандартные тулы 1-в-1 (agent.rs / tool_schemas.rs) — **работает**

| Тул | Параметры | Описание и заметки кода |
|---|---|---|
| `read_file` | `file_path` (`path`), `offset` (def 1), `limit` (def 2000) | UTF-8, номера строк; вне worktree — отказ. Порог спиллинга: 2000 строк / 128 КБ. |
| `write_file` | `file_path` (`path`), `content` | Атомарная запись файла в изолированном worktree. |
| `edit_file` (алиас `str_replace`) | `file_path` (`path`), `old_string` (`old_str`), `new_string` (`new_str`), `replace_all` (def false) | 5-уровневый fuzzy-матчинг с адаптацией CRLF/пробелов. |
| `glob` | `pattern` (req), `path` (def `.`) | Поиск путей по шаблону; пропуск `.git`/`target`/`node_modules`. |
| `grep` | `pattern` (req), `path` (def `.`) | Regex-поиск содержимого файлов с ограничением совпадений. |
| `execute_command` (алиас `bash`) | `command` (`cmd`), `timeout_seconds` (def 60) | Запуск shell-команд. Порог спиллинга: 500 строк / 64 КБ. Убийство дерева процессов при таймауте. |
| `web_search` | `query` (req), `limit` (def 5) | DuckDuckGo search интеграция без внешних API-ключей. |
| `web_fetch` | `url` (req) | Чистка HTML в читаемый Markdown. |
| `subagent` | `title` (req), `prompt` (req), `persona`/`role`, `tools` | **Синхронная делегация**: запускает дочерний ReAct-цикл и блокирует родителя до получения финального отчета. |
| `subagent_bg` | `title` (req), `prompt` (req), `persona`/`role`, `tools` | **Асинхронная фоновая делегация**: возвращает `subagent_id` без блокировки родителя. |
| `collect_subagent` | `subagent_id` или `subagent_ids` | **Сборщик отчетов субагентов**: 3-уровневое разрешение (Tokio broadcast, in-memory кэш, SQLite). |
| `subagent_list` | `status_filter` (`all\|running\|completed\|aborted`) | Реестр активных и завершенных субагентов задачи. |
| `load_rule` | `rule` (req) | JIT-загрузка модульных системных правил из `.cvh/rules/` и роли. |
| `todo_write` | `todos: [{content, status}]` (req) | Тактический чеклист `pending\|in_progress\|completed`. Пишет в SQLite WAL + пушит события в UI. |
| `question` (алиас `ask_user`) | `text` (req), `options`, `multiple` | Интерактивный Human-in-the-Loop запрос, приостанавливающий ReAct-цикл. |
| `create_task` | `title` (req), `description`, `stage_id`, `board_id` | Создание единичной карточки в Kanban-пайплайне. |
| `create_tasks` | `tasks: [{title, description, stage_id}]` | Пакетное создание карточек задач. |
| `complete_task` | `task_id` (req), `summary` | Перевод задачи в терминальную стадию с записью результата. |
| `list_tasks` | `limit`, `query`, `stage_id` | Компактная инспекция карточек задач Kanban-доски. |
| `get_task_history` | `task_id` (req), `include_tool_outputs` | Хронология выполнения задачи со всеми вызовами инструментов и вердиктами. |

Безопасность путей: `resolve_safe_path` — нормализация `..`, запрет выхода за worktree, запрет абсолютных системных путей — **работает**.

> Расхождение с `README.md` (§7): «`web_search` … with Google/Bing fallback» — **частично**: в коде только
> два DuckDuckGo-источника, Google/Bing нет.

### 4.4 `todo_write` и кастомные тулы в матрице — **работает**

Оба типа проходят один и тот же фильтр (`is_tool_allowed` + `agent_allows_tool`) и один gate.
Кастомные тулы грузятся из `.cvh/tools/*.json` (`load_custom_tools`), сортируются по имени,
мерджатся к стандартным (`get_all_tools`). Неизвестный тул → `Unknown tool`.

---

## 5. Контекстное окно / телеметрия / компактор — что видит агент

### 5.1 Что реально попадает в системник — **работает**

`compile_primed_system_prompt(repo_root, role)` собирает строго статический промпт
(ноль динамических timestamp/UUID — ради кэша):

1. `# Role Persona: <role>` + flat `.cvh/agents/<role>.md` (нет файла → fallback
   `You are an expert AI agent in <role>.`);
2. `## Rule: <filename>` + содержимое каждого `.cvh/agents/<role>/rules/*.md` **по алфавиту**;
3. `# Scoped Tool Execution Mandate` (read/write/edit/glob/grep/execute + Ground Truth Sensors).

Динамика (id, title, description, worktree) — **только** в user-сообщении
(`# Objective: <title>\n\n<description>` + `[HOOK]`-маркер).

> **Важно, расхождение с историческими черновиками**:
> Код **намеренно НЕ инжектит** устаревшие корневые файлы правил или внешние `.agents/skills`
> (регрессионный тест `test_system_prompt_is_lean_cvh_only`). Источник правды — код.
> Модульные правила живут исключительно в `.cvh/rules/` и `.cvh/agents/<role>/rules/`.

### 5.2 Телеметрия — **работает**

События цикла: `Token/Output/Thought`, `ToolCall/ToolResult`, `TurnStart`, `Telemetry
{input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens, cache_hit_rate}`,
`Compacted`, `Finished/Aborted/Error`, `TaskUpdated`. Фронт слушает `agent:*` через Tauri events
и рисует ленту, TPS, `% cache hit`, кольцо контекста. Каждый терн логирует
`prefix_len`/`prefix_hash` для отладки кэша (только длины+хэш, не содержимое).

### 5.3 Компактор — **работает** (авто), ручное — **частично**

- Оценка: ~4 chars/token + 4/сообщение (`estimate_tokens`).
- Испарение (`evaporate_tool_logs`): tool-ответы `> 500 chars` и **старше 3 тернов**
  (`current_turn - turn > 3`) заменяются плейсхолдером
  `[Output evaporated: N chars truncated. Exit code: C. Tool: T]`. Якоря Turn 0 (system + исходный user) неприкосновенны.
- Авто: в `run_agent_loop` при `turn > 3 && estimate > 40_000` → событие `Compacted`.
- Ручное `compact_context(task_id)`: жмёт `agent_contexts[task_id]`… **частично**: `run_agent_loop`
  держит сообщения в локальном `Vec` и в `agent_contexts` их не кладёт, поэтому для живой сессии
  компактится синтетический 2-месседжный контекст с отчётом `CompactionReport`, а не реальная лента.
  `compact_task_context` — тем более демо (компактит искусственные sample-сообщения).
- `AgentSessionConfig.max_turns = 25` выставляется, но цикл его **не проверяет** (безлимит до терминального
  ответа/аборта/ошибки) — **частично** (мёртвое поле).
- `ContextGauge` (фронт) считает токены клиентской эвристикой по ленте + телеметрии и кнопкой
  `⚡ Компактировать` дергает `compact_context`; дополнительно фронт сам испаряет старые tool-логи
  (`evaporateClientToolLogs` в `drawerStore`) — это уже про рендер, не про LLM-контекст.

> Расхождения цифр в доках: `docs/agents.md` пишет «>500 символов старше 2 шагов» и «сортировка определений»,
> `README` — «старше 3 turns (>500 chars)», «Cache Hit Rate >= 90%», «4 Immutable Anchors».
> Код: порог именно `>500` и `>3`, автотриггер `>40_000` токенов; «90%» и «4 якоря» — цели/описания, не ассерты.

---

## 6. UI/UX тур

### 6.1 Доска (`board/KanbanBoard`, `KanbanColumn`, `card/KanbanCard`) — **работает**

DnD карточек → `boardStore.moveTask` (§3.2). Карточка: моноширинные таймеры (`tabular-nums`,
`activeStageDuration`/`stageDurations`/`totalElapsed`, тик `tickTimers`), счётчик возвратов `↩ Returns`,
бейджи вердиктов (`APPROVED` / `APPROVED_WITH_CAVEATS` / `CHANGES_REQUESTED`), `fastTrack`-метки
(фронт-only, бэкенд их игнорирует при `moveTask` — **частично**), таймер идёт пока `isTimerRunning`.
Мульти-доски: переключатель + `CreateBoardModal` (имя + template) — **работает**.

### 6.2 Чат задачи (`drawer/AgentConsoleDrawer`) — **работает**

Клик по карточке → двухколоночный drawer: слева лента (цель, `<thinking>`-блоки с таймером,
аккордеоны тулов с exit-кодами/таймингами, gutter-diff для `edit_file`, `MarkdownViewer` с копированием,
`TaskTodoWidget` плана из `todo_write`), справа `GitDiffSidebar`. Внизу steer-бар
(`steer_agent`: шлёт `agent:steer`, абортит текущий ран, перезапускает `run_agent` с `[USER STEER]`;
описание задачи при этом не пачкается) + строка телеметрии (кэш/TPS/контекст) + `ContextGauge`.
`log_ui_error` увозит JS-ошибки рендера в бэкенд-лог (против белых экранов).

### 6.3 Проводник разума (`cortex/CortexExplorer` + `RuleEditorModal`) — **работает, с оговоркой §2.7**

Дерево строится **из колонок доски** (`syncStagesFromColumns`): колонка с `persona` = стадия-агент
с узлами `agent.md` (шаблон персоны), `setting`, `rules/`, `hooks/`; `backlog` — системный узел
(показывает свои хуки); колонки без персоны (напр. `ready`) пропускаются; свежий проект
(backlog+ready без персон) даёт пустое состояние. Бэкенд-сторона: `get_cortex_tree` отдаёт
`Personas (.cvh/agents)` / `Global Rules (.cvh/rules)` / `Transition Hooks (.cvh/hooks)` / `Skills (.agents/skills)`,
`read/save_cortex_file` читают/пишут произвольный относительный путь (создавая каталоги).
Редактор: подсветка, счётчики, `Ctrl+S`. Создание из UI: правило (`rules/<имя>.md` + автосуффикс),
хук (`hooks/on_enter[-<имя>].<bat|ps1|py|sh>` с шаблонами под момент), персона/настройка — шаблоны.

### 6.4 Матрица (`settings/ToolPolicyModal`) — **работает, превью — частично (§4.2)**

Глобальные тоглы + per-stage сетка по всем стадиям доски (ключи стадий = id колонок, не только dev/review),
`calculateSavedTokens` (сумма `tokenWeight` отключённых; минимум-показ 2400), `resetDefaults`,
`loadFromTauri`/`saveToTauri` (`get/save_tool_policies`). Вкладка кастомных тулов: тот же CRUD
(`NAME_PATTERN /^[a-z0-9_]+$/` дублирует бэкенд-валидацию).

### 6.5 Git Diff (`drawer/GitDiffSidebar` + `get_task_git_diff`) — **работает**

Ветка задачи (текущая в worktree, иначе `task.branch`, иначе `main`), счётчики `+N -M`,
список файлов (`M`/`A`/`D` из `git status --porcelain -uall`), патчи (`git diff HEAD` +
`--no-index` для untracked + синтез патча если git не смог), сортировка по пути, копирование патча,
обновление. Пустой дифф — пустая панель, не ошибка.

### 6.6 ПКМ-меню — **работает**

- Задача (`card/TaskContextMenu`): Открыть / Запустить агента (`runTaskAgent`: есть persona —
  running+run, нет — перенос в первую колонку с персоной) / + Создать подзадачу / Переместить в:
  (все колонки) / Удалить (каскад + чистка worktree только для топ-уровня + чистка диалогов).
- Колонка (`board/ColumnContextMenu`): переименование, смена персоны (список из `list_agent_personas`
  + `__new__` с валидацией), скрипт перехода (`onEnterScript`), удаление — кроме защищённых
  `backlog`/`ready` (`PROTECTED_COLUMN_IDS`; бэкенд тоже держит их на краях).
- Топбар/с firm: проекты (`select/set_active_project`, реестр, `active_project.txt`),
  LLM-настройки (`SettingsModal`: provider/baseUrl/apiKey/model/temperature + `fetch_available_models`
  с перебором `/models`, `/v1/models`, `/api/tags`, OpenAI/Ollama/массив-форматы), сплэш.

---

## 7. Паттерн «доски как витки разработки»

Идея: вместо одного длинного пайплайна — несколько досок-витков под разные контуры работ.
Задачи живут в своей доске (`board_id`), колонки доски = агенты контура со своими хуками/персонами.
Механика витков — `boards.json` + `create_board(template)` + сабтаски (§2.3, §3.3).
Пример ниже — рабочий скелет (ключи и значения — из кода, §2.2/§4).

### 7.1 Доска фронта — `frontend` (свои хуки и агенты)

```json
{
  "id": "frontend",
  "name": "Фронтенд",
  "columns": [
    { "id": "backlog", "title": "Бэклог", "persona": null, "hooks": {},
      "contextWindow": 128000 },
    { "id": "markup", "title": "Вёрстка", "persona": "markup-dev",
      "temperature": 0.3, "contextWindow": 200000,
      "hooks": { "on_enter": "builtin:create_worktree" }, "autoRun": true },
    { "id": "ux-review", "title": "UX-ревью", "persona": "ux-reviewer",
      "temperature": 0.1, "contextWindow": 128000,
      "hooks": { "on_enter": ".cvh/hooks/on_enter_ux.bat" },
      "autoRun": true },
    { "id": "ready", "title": "Готово", "persona": null, "terminal": true,
      "hooks": { "on_enter": ".cvh/hooks/on_finish_done.bat" },
      "gatekeeperSensor": "npm run build", "contextWindow": 64000 }
  ]
}
```

Агенты: `.cvh/agents/markup-dev.md` (верстает в worktree), `.cvh/agents/ux-reviewer.md`
(вердикты `APPROVED`/`CHANGES_REQUESTED` — их парсит §3.3). В `tool-policy.json`:
`stageOverrides."ux-review"`: `edit_file: false, write_file: false`
(глобально выключать запись не надо — AND всё равно даст `false` только здесь).
Хук `.cvh/hooks/on_enter_ux.bat` обязан печатать `CVH_WORKTREE=`/`CVH_BRANCH=`, если сам назначает worktree (§3.4).

### 7.2 Доска бэкенда — `backend`

`backlog → dev (persona: backend-dev, sensor: cargo check) → review (persona: backend-reviewer,
→ retro (persona: retro) → ready (terminal)`.
`backend-reviewer` — readonly по матрице; `retro` — пишет леджер (см. оговорку ниже — **задумано**).

### 7.3 Доска девопса — `devops` (только developer, без ревью)

```json
{
  "id": "devops",
  "name": "Девопс",
  "columns": [
    { "id": "backlog", "title": "Бэклог", "persona": null, "hooks": {} },
    { "id": "ops", "title": "Инфра", "persona": "developer",
      "hooks": { "on_enter": "builtin:create_worktree" }, "autoRun": true,
      "gatekeeperSensor": "terraform validate" },
    { "id": "ready", "title": "Готово", "persona": null, "terminal": true }
  ]
}
```

Без колонки ревью нет сабтасок-ревью: финиш агента двигает задачу прямо в `ready`
(ветка «terminal or non-persona» в §3.3). Отдельная персона не нужна — переиспользуется `developer`;
если надо ужать права — `agents/developer/setting` с `allowed_tools` (пересечение, §4.2).

> Оговорка про `retro` — **задумано**: персона `retro.md` описывает запись в
> `.agents/tasks-history.json` и `.agents/knowledge-graph.json` и патчинг правил в `.cvh/rules/`,
> `docs/board.md` (§2 п.4) это повторяет — но `grep` по `src-tauri/src` не находит ни одной записи
> в эти файлы (`resolve_allowed_tools|tasks-history|knowledge-graph` — тишина, кроме комментариев).
> Файлы `.agents/tasks-history.json` / `.agents/knowledge-graph.json` в репозитории существуют,
> автопополнения из кода нет — ведите их вручную или своим хуком/тулом, пока не реализовано.
> Архитектурные концепции графа памяти / SQLite WAL ledger и двухколоночного Live Diff — **работает**:
> SQLite диалоги сохраняются в `.cvh/harness.db` с WAL-журналированием (`db.rs`), Cortex API активен,
> а двухколоночный Live Diff (`GitDiffSidebar`) полностью интегрирован в карточки и панель drawer.

---

## 8. Чеклист «настрой `.cvh` для любого проекта» по шагам + типовые ошибки

### Шаг 0. Открой проект в CVH

Открой корень репо → скаффолд (§1). Проверь: `.cvh/config.json` (имя/endpoint),
`.cvh/pipeline.json` (2 колонки), `.cvh/tasks.json` (`[]`),
`.cvh/settings/tool-policy.json`, `.gitignore` (есть `.cvh/worktrees/`).
Если открываешь не git-репо — у `backlog` не будет builtin-хука: добавь позже вручную (§3.1).

### Шаг 1. Доска(и): `boards.json` / `pipeline.json`

1. Создай доски под свои контуры (§7) — `create_board` или правкой JSON.
2. Соблюди инвариант: `backlog` первая, `ready` последняя, id уникальны (иначе ошибка сохранения).
3. Расставь `persona` только там, где реально нужен агент; `terminal: true` — только финалу.
5. `gatekeeperSensor` — команда, честно проверяющая стадию в worktree (`cargo check`, `npm run build`,
   `terraform validate`); помни про таймаут 300с.

### Шаг 2. Персоны: `.cvh/agents/<role>/persona.md` + `rules/` + `setting`

1. `create_agent_persona(<role>)` (создаёт директорию `.cvh/agents/<role>/` со скаффолдом `persona.md`, `setting`, `rules/`, `hooks/`).
2. **Заполни `persona.md`** — системный промпт роли с YAML frontmatter (`icon`, `color`, `description`, `scope`).
3. Разложи инварианты по `rules/*.md` (имена задают порядок — называй `00-*.md`, `10-*.md`…).
4. Прижми права/настройки в `<role>/setting` (`allowed_tools`, `model`, `temperature`, `context_window`, `icon`, `color`).
   Проверь JSON валидность — битый файл молча = дефолт.

### Шаг 3. Хуки: `hooks` + `.cvh/hooks/` + per-agent fallback

1. Положи скрипты (расширение = интерпретатор: `.bat`/`.ps1`/`.py`/`.sh`).
2. Если хук сам назначает worktree — печатай `CVH_WORKTREE=` / `CVH_BRANCH=` (§3.4), иначе тишина.
3. Проверь env: скрипт должен читать `CVH_TASK_ID`, `CVH_STAGE_ID`, `CVH_BOARD_ID` (не `CVH_STAGE`!).
4. Помни разницу cwd: `hooks.on_enter/on_finish` бегут от корня; `onEnterScript` — от worktree-или-корня.
5. Альтернатива общим хукам — per-agent `.cvh/agents/<role>/hooks/<basename>` (fallback по basename).

### Шаг 4. Матрица: `settings/tool-policy.json`

1. Single Authoritative Layer: явный stage override (`true` или `false`) имеет приоритет над глобальным дефолтом.
2. Ревью/аудит: `edit_file: false, write_file: false` на стадии, не глобально.
3. Кастомные тулы из `tools/` — те же тоглы (имена — `name` из JSON).
4. Мастер-свитч `subagent` на стадии управляет доступностью всего семейства субагентов.

### Шаг 5. Кастомные тулы: `.cvh/tools/*.json`

Имя `^[a-z0-9_]+$`, `parameters` — объект, команда с `{{args}}` (двойные скобки).
Проверь: тул появился в `list_custom_tools`, матрица его видит, spill-порог тебя устраивает.

### Шаг 6. Первая задача и проверка контура

1. Создай задачу в `backlog` (чекбокс worktree: вкл = builtin по конвенции; выкл = только хуки).
2. Перетащи в агентскую колонку → проверь: worktree создан, в ленте `[HOOK] worktree:`,
   телеметрия идёт, `ContextGauge` считает.
3. Доведи до вердикта → проверь порядок стадий, сабтаску `CVH-NNN.M`, финал в `ready/done`.
4. Удали тестовую задачу → worktree/ветка/диалоги чистятся.

### Шаг 7. Канонический контур CanbanVoimaxHarness: `developer -> review`

Для разработки самого CanbanVoimaxHarness настроена оптимизированная доска:
1. **Колонки пайплайна**:
   - `backlog` (интейк с хуком `builtin:create_worktree`).
   - `dev` (роль `developer`, сенсор `gatekeeper_sensor.bat`, полный доступ к инструментам разработки).
   - `review` (роль `reviewer`, ультра-строгий аудит 6 врат, запрет на редактирование файлов).
   - `ready` (терминальная колонка сдачи, `terminal: true`).
2. **Инвариант синхронизации правил (`rules-sync`)**:
   - Разработчик обязан синхронизировать `.cvh/rules/` и `/docs/` при любых изменениях архитектуры, API, таблиц SQLite WAL, Zustand-сторов или дизайн-токенов.
   - Ревьюер проверяет git diff и немедленно реджектит (`reject_task`), если код изменил архитектуру без актуализации правил.
3. **Автоматический Merge & Push в main**:
   - При выставлении вердикта `APPROVED` на этапе `review` срабатывает хук `.cvh/hooks/on_review_finish.bat` (и `.sh`).
   - Хук автоматически выполняет `git merge --no-ff %CVH_BRANCH%` в ветку `main` и пушит в `origin/main`.

### Типовые ошибки (все — из кода выше)

| # | Ошибка | Что происходит | Как чинить |
|---|---|---|---|
| 1 | Хук настроен, файла нет | молчаливый пропуск (`exists`-check), в логе только on_enter_script-варн | положи файл или убери ключ |
| 2 | Хук создаёт worktree, но не печатает `CVH_WORKTREE=` | задача остаётся без worktree → агент работает **в корне репо** | допечатай протокол §3.4 |
| 3 | Пустая `persona.md` | слабый дженерик-системник | заполни цель/стиль/запреты и frontmatter |
| 4 | Надежда на `column.allowedTools` как enforcement | gate его не смотрит | права — в `tool-policy.json` + `setting` |
| 5 | Имя тула `my-tool` / `MyTool` | `save_custom_tool` отклонит (только `a-z0-9_`) | переименуй |
| 6 | `{args}` одинарные в `command` | подстановки нет (код ищет `{{args}}`) | двойные скобки |
| 8 | Правка `pipeline.json` при активных досках | перезапишется зеркалом активной доски | правь `boards.json`/доску в UI |
| 9 | `wipLimit` как лимит | фронт-only поле, бэкенд не знает | контролируй руками |
| 10 | Ожидание автозаписи `tasks-history.json`/knowledge-graph | код не пишет (задумано) | веди вручную/хуком |
| 11 | Стадия без worktree у задачи | агент тихо работает в корне (WARN в лог) | проверяй worktree после создания |

---

## Приложение А. Статусная сводка проверки

**Покрыто (работает)** — скаффолд (состав, минимальные 2 колонки, пустые `agents/`, git-условный builtin-хук,
`.gitignore`); все дефолты колонок (minimal/default/canonical/modeler); `ToolPolicyConfig` Single Authoritative Layer + алиасы;
`create_task`, `update_task`, `delete_task` (все параметры, board-aware поиск колонки, shell→stdout→builtin цепочка, `skip_worktree`);
`move_task_stage` (on_finish best-effort → sensor блокирующий → on_enter блокирующий → builtin → onEnterScript);
`run_agent` (steer, fallback persona `developer`, приоритеты модели/температуры/baseUrl, worktree-fallback в корень,
`[HOOK]`-маркер, вердикты, сабтаски, terminal); `list/create/delete/rename_agent_persona` (канонический `.cvh/agents/<role>/persona.md`, валидация, шаблон);
субагенты (`subagent` синхронный, `subagent_bg` фоновый, `collect_subagent` сборщик с 3-уровневым разрешением, `subagent_list`);
custom tools CRUD + исполнение `{{args}}`; `execute_hook` env/интерпретаторы/таймауты; stdout-протокол;
системник (персона + rules по алфавиту + runtime mandate, без внешних skills/монолитов — намеренно); полный стек инструментов
с параметрами; `todo_write` (persist + событие); spill (>500 строк / >64KB; >2000 строк / >128KB для `read_file`);
ACID-транзакции диалогов в SQLite WAL `harness.db`; `normalize_pipeline_columns`;
boards CRUD + слаги + защита последней доски + реассайна задач; `boards.json` ↔ `pipeline.json` синк;
Cortex tree/read/save; Git Diff и Image Diff; drawer/телеметрия/steer; оба ПКМ-меню (+защита `backlog`/`ready`);
`ContextGauge`+телеметрия; `localStorage`-бэкап задач и монотонные ID.

**Частично** — `column.allowedTools`/`resolve_allowed_tools` (декларативно, gate не смотрит);
`compact_task_context` и `compact_context` для живых сессий; `max_turns` (мёртвое поле);
шаблоны досок из `docs/board.md` (встроенный `modeler` на 4 стадии, остальные дают 3 стадии);
формула системника из `docs/agents.md` (очищена от устаревших ссылок); Google/Bing fallback (только DDG);
`fastTrack`, `wipLimit` (фронт-only).

**Задумано** — автозапись `.agents/tasks-history.json` / `.agents/knowledge-graph.json` и патчинг правил в `.cvh/rules/`
ретро-агентом (управляется агентом-верификатором).
(Все основные вехи дорожной карты CVH-001—CVH-014 и архитектурные концепции Live Diff и SQLite WAL леджера памяти переведены в статус «работает»).
системник (персона + rules по алфавиту + mandate, без внешних skills/монолитов — намеренно); 10 стандартных тулов
с параметрами; `todo_write` (persist + событие); spill; 3-tier диалогов; `normalize_pipeline_columns`;
boards CRUD + слаги + защита последней доски + реассайна задач; `boards.json` ↔ `pipeline.json` синк;
Cortex tree/read/save; Git Diff; drawer/телеметрия/steer; оба ПКМ-меню (+защита `backlog`/`ready`);
`ContextGauge`+телеметрия; `localStorage`-бэкап задач и монотонные ID.

**Частично** — `subagent` (заглушка); превью `isToolAllowed` фронта (stage-wins vs AND);
`column.allowedTools`/`resolve_allowed_tools` (декларативно, gate не смотрит); `compact_task_context`
и `compact_context` для живых сессий (синтетика); `max_turns` (мёртвое поле); шаблоны досок из `docs/board.md`
(`development`/`kanban`/`simple`/`empty` — кода нет, есть canonical+modeler+скаффолд);
формула системника из `docs/agents.md` (очищена от устаревших ссылок); Google/Bing fallback (только DDG);
`retro`-оверрайд (есть в сторе/файле, нет в бэкенд-дефолте); `fastTrack`, `wipLimit`
(фронт-only); узел `agent.md` Проводника vs flat-персона; двухколоночный Live Diff (`GitDiffSidebar` — полностью интегрирован);
пример `tool-policy.json` в `docs/agents.md` (без `todo_write`); env-таблица `docs/hooks.md` (без `CVH_BOARD_ID`,
без `CVH_WORKTREE=` в примерах).

**Задумано** — автозапись `.agents/tasks-history.json` / `.agents/knowledge-graph.json` и патчинг правил в `.cvh/rules/`
ретро-агентом (управляется агентом-верификатором); удаление/переименование персон командой
(только ручное); трёхшаговые шаблоны `todo→in_progress→ready` как отдельные `template`-ветки.
(Все основные вехи дорожной карты CVH-001—CVH-014 и архитектурные концепции Live Diff и SQLite WAL леджера памяти переведены в статус «работает»).

**Объём файла**: см. отчёт проверки ниже (команда `wc` после записи).
