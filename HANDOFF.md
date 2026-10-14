# CanbanVoimaxHarness — Передача смены (HANDOFF)
**Дата:** 14 октября 2026 г., день (сессия на Linux)
**Ветка:** `main` → `c57c3ae` (синхронно с `origin/main`, дерево чистое)
**Репозиторий:** `https://github.com/Asagiry/CanbanVoimaxHarness`
**Машина:** Ubuntu 24.04 (X11), `/home/vepishin/Projects/CanbanVoimaxHarness`

---

## 1. Что сделано за сессию

### 1.1. Агент-луп и политики тулов
- `run_agent(task_id, prompt?)` вмерживает текст в description; убран serde-дубль `stageId`+`columnId` (ронял все `save_tasks`); единый `Task`; `steer_agent` смотрит in-memory.
- Ровно один `agent:finished` + фронт-дедуп; **безлимитный ReAct-цикл** (выход: терминальный текст/abort/ошибка); +терминальный терн удалён за ненадобностью.
- Политики: канон `.cvh/settings/tool-policy.json`, семантика `global && stage`, алиасы, `tools: None` при пустом наборе; `todo_write` в дефолтах; CRUD кастомных тулов (`list/save/delete_custom_tool`); кастомные видны в матрице.
- Худой системник: только persona + `.cvh` (без корневого `AGENTS.md`/`.agents/skills`).
- Кеш: детерминированный префикс, frozen breakpoints, парсинг всех форм usage, `CVH [cache]`-логи. Нули в основном от прокси `:8045` (отбрасывает счётчики).

### 1.2. Ворктри только хуком (финальный флоу)
- Автосоздание вырезано из `run_agent`/`create_task`. Нет пути → fallback корень + warn.
- `builtin:create_worktree` на **старте**: маркер `on_enter` в колонке ИЛИ дефолт бэклога без явного хука; shell-хуки первее (`CVH_WORKTREE=`/`CVH_BRANCH=` из stdout скипают builtin).
- Скаффолд ставит маркер бэклогу только если в корне есть `.git`.
- Создание задачи идёт через бэкенд `create_task(title, description, stage_id, board_id?, skip_worktree?)`; в модалке чекбокс «Создать worktree при старте» (дефолт ВКЛ). Подзадачи наследуют worktree родителя.
- `moveTask` фронта идёт через `move_task_stage` IPC (хуки/сенсоры срабатывают), fallback на `save_tasks`.
- В свежий ворктри синкается живой `.cvh` (`sync_cvh_runtime_state`, без worktrees/dialogs/spill/секретов).
- В промпт агента добавляется `[HOOK] worktree: <path> — твой изолированный workdir, работаешь тут.`

### 1.3. Колонки = агенты, всё кастомное
- Промоушен: вердикт-роуты → сосед справа → stay+done; статус `terminal→done`, `persona→running`. `STAGE_ORDER`/`fastTrack`/`customFlow` выведены из роутинга; `PipelineColumn`: `autoRun?`, `terminal?`.
- Backlog всегда первая, ready всегда последняя (pinned, неудаляемые, нормализация и на фронте, и в `save_pipeline`); создание только в backlog; DnD-перестановка колонок; ПКМ-меню задачи и шапки колонки (переименовать/сменить агента/удалить/скрипт).
- Персоны: `list/create_agent_persona` (flat `.cvh/agents/<role>.md`), имена авто-слагифицируются (кириллица → латиница, превью «Роль:»), `src/lib/slug.ts`.
- Легаси вырезано: dual-path политик/персон, migrate-*, их тесты. Serde-алиасы оставлены (безвредны).
- ID монотонные (`cvh_task_seq_*`, бэкенд max+1); удаление чистит ворктри+ветку+диалоги+каскад подзадач.

### 1.4. Проводник разума (две секции)
- Секция «Колонки»: только хуки + поле АГЕНТ (select/свободный ввод); backlog и ready/terminal — системные узлы «система» (хуки видны, rules/setting убраны) — на ready вешается on_finish вроде коммит+пуш.
- Секция «Агенты»: agent.md, rules/, hooks/ (бейджи on_enter/on_finish/встроенный), setting; создание файлов с формами; отсутствующие — плейсхолдеры.
- Поиск один на обе секции.

### 1.5. Чат и доска
- Шапка чата = шапка приложения (лого+Назад, wrench/настройки, оконные кнопки); ворктри-полоса ниже шапки.
- To-do стрип вплотную над отправкой, свёрнут по умолчанию, read-only (заполняет агент через `todo_write`), скрыт при выкл. туле; `todos` едут в sync-пейлоаде (больше не затираются).
- Ответы скрываемы, умный автоскролл + «К новому», thinking-таймер, фриз TPS, `Кеш·TPS` у кольца, микробатчинг токенов ~150мс, GFM-таблицы, `log_ui_error` + window hooks в лог, ErrorBoundary везде.
- Карточки: футер (модель/ветка/время/`созд.`/`идёт`), обложки вырезаны полностью, ПКМ-меню; шапки колонок 13px; топбар без дублей (поиск в ряду вкладок, подпись папки); общий `WindowControls`; светлый сплэш-минимал + drag; kill-ai-slop; `.desktop` с иконкой.
- Матрица только по агентам (persona != null); кнопка-гаечный ключ в топбаре.

## 2. Как запускать (важно: фронт вшит в бинарь!)

```bash
npm run build                                   # фронт
export PATH="$HOME/.cargo/bin:$PATH"
cargo clean -p canban-voimax-harness && cargo build   # в src-tauri/ (clean обязателен — иначе вшьётся старый фронт!)
# сверка: strings target/debug/canban-voimax-harness | grep -o 'index-.*\.js' == dist/assets/index-*.js
ps aux | grep -F target/debug/canban | grep -v grep | awk '{print $2}' | xargs -r kill
setsid env DISPLAY=:0 ./src-tauri/target/debug/canban-voimax-harness > /tmp/cvh_app.log 2>&1 < /dev/null &
# workdir = корень репо; гасить только kill <PID>, НЕ pkill -f
```
Проверки: `npm run build`, `cargo check`, `cargo test` (сейчас 52 passed). Живое: PID 35537, лог `/tmp/cvh_app.log`.

## 3. Открытые пункты / TODO

1. **Прокси `:8045` и кеш** — если `prefix_hash` стабилен, а `cached_*` в `usage raw=` нет: чинить маппер ответа прокси.
2. **Модель болтлива** — каждый терн = блок «Ответ агента»; схлоп идентичных соседних блоков не сделан.
3. **Остатки `fastTrack`/`customFlow`** — выведены из роутинга, поля/типы лежат; удалить следующим заходом.
4. **`ToolPermissions`/allowlist-модель и `column.allowedTools`** — по GUIDE помечены «частично», gate их не смотрит; унифицировать с политиками.
5. **Дефолтный pipeline на английском** vs русские названия; seed синхронизировать.
6. **`discover_project_root`** — захардкоженный Windows-путь; `max_turns` — мёртвое поле; `retro`-дефолта нет в бэкенде.
7. **Редактор проводника vs бэкенд**: убедиться что openFile персон правит тот файл что читает рантайм (см. GUIDE, 14 «частично»).
8. GUIDE: `docs/GUIDE.md` (676 строк) — сверить после этой сессии (проводник/backlog-ready/worktree-старт), там же 4 «задумано» (SQLite, граф памяти, удаление персон командой).

## 4. Справочник `.cvh` (что где лежит)

- `pipeline.json` — колонки/поток (порядок, persona, hooks, routes, autoRun/terminal)
- `boards.json` — доски (вкладки, у каждой свои колонки)
- `config.json` — проект (имя/версия/activePipeline)
- `llm.json` — секреты подключения (baseUrl/ключ, **gitignored**, в ворктри не копируется)
- `tasks.json` — задачи всех досок (id/стадии/статусы/todos)
- `tasks/<id>/` + `dialogs/` — история чатов по задачам (два пути: основной `dialogs/<id>.json`, запасной `tasks/<id>/dialog.json` — кандидаты на слияние)
- `agents/<role>.md` — персоны (flat; per-agent dirs не внедрены)
- `settings/tool-policy.json` — матрица (global && stage)
- `tools/*.json` — кастомные shell-тулы (`{name,description,parameters,command}` с `{args}`)
- `hooks/` — скрипты переходов (on_enter/on_finish, `CVH_*` env + `CVH_WORKTREE=` из stdout)
- `skills/` — скиллы агента; `spill/` — длинные выводы тулов (>100 строк/8КБ, head/tail в контекст); `worktrees/` — git-ворктри задач (gitignored-через `.gitignore`: `.cvh/worktrees/`)
