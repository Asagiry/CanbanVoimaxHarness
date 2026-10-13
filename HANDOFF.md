# CanbanVoimaxHarness — Передача смены (HANDOFF)
**Дата:** 13 октября 2026 г., вечер (сессия на Linux)
**Ветка:** `main`
**Последний коммит:** `2294135` (всё ниже — НЕ закоммичено, см. раздел 5)
**Репозиторий:** `https://github.com/Asagiry/CanbanVoimaxHarness`
**Машина сессии:** Ubuntu 24.04 (X11, DISPLAY=:0), проект `/home/vepishin/Projects/CanbanVoimaxHarness`

---

## 1. Что сделано за сессию (по порядку)

### 1.1. Окружение Linux с нуля
- Склонирован репозиторий по SSH (`git@github.com:Asagiry/CanbanVoimaxHarness.git`) — HTTPS с паролем GitHub больше не принимает, только SSH/токен.
- Установлены: `rustup stable 1.99.0`, системные либы Tauri v2 (`libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`), `npm install`.
- Проверки зелёные: `npm run build`, `cargo check`, `cargo test` (сейчас 21 passed).
- Приложение запускается нативно: `setsid env DISPLAY=:0 ./src-tauri/target/debug/canban-voimax-harness` из корня репо (ВАЖНО: workdir = корень, иначе скаффолдит мусорный `.cvh` в месте запуска — такой уже чистили в `src-tauri/.cvh`).
- ВАЖНО про `pkill -f canban-voimax-harness`: убивает собственный shell (паттерн совпадает). Гасить только `kill <PID>` (PID смотреть через `ps aux | grep -F target/debug/canban`).

### 1.2. Починен агент-луп («Task not found», не стартовал ни на винде, ни на линуксе)
Корневые причины (все исправлены):
1. **Фронт терял текст пользователя**: `handleSendPrompt` при idle вызывал `run_agent(taskId)` без текста. Теперь `run_agent(task_id, prompt?)` вмерживает prompt в description; фронт передаёт текст всегда.
2. **Serde-дубль ронял все сейвы**: фронт слал `stageId` + `columnId` одновременно → `duplicate field 'stageId'` → `save_tasks` падал всегда → `tasks.json` вечно `[]`. Доказано отдельным тестом. Теперь шлётся только `stageId`.
3. **Ворктри подозревали зря**: `git worktree add/remove` проверен вручную — работает. До создания ворктри код просто не доходил (п.1–2). Плюс `create_worktree` теперь проверяет base-ветку и падает на `HEAD`, а не только `main`.
4. Дубли `Task`-структур слиты в одну (`pub use crate::agent::Task`), алиасы унифицированы.
5. `steer_agent` теперь смотрит сначала в in-memory реестр, как `run_agent`.
6. `agent:finished` слался ДВАЖДЫ (в sink + после промоушена) → финал троился в ленте. Остался один emit; фронт дополнительно дедупит по `endsWith(response)`.
7. `vite.config.ts`: `optimizeDeps.entries=['index.html']` + игнор `src-tauri/**` в watch (esbuild падал с EPIPE на артефактах таргета).

### 1.3. Матрица тулов и политики (3 бага, все закрыты)
- Модалка существовала, но `openModal()` нигде не вызывался → добавлена кнопка-гаечный ключ в топбар слева от настроек LLM.
- Был хардлок «Блок» для review (edit/write) — убран, review как все.
- **Split-brain путей**: скаффолд писал `.cvh/settings/tool-policy.json`, чтение/запись шли в `.cvh/tool_policy.json` → унифицировано на canonical, legacy мигрируется при сейве.
- **Семантика AND**: было «оверрайд перебивает глобал» (дефолтные true гасили глобальное false) → стало `effective = global && stage` (нет записи = true). Пустой набор тулов валиден (шлём `tools: None`).
- Матрица показывает только колонки-агенты (`persona != null`); таб переименован «Матрица агентов».

### 1.4. Архитектура «колонки = агенты, всё кастомное»
- Автопродвижение в `commands.rs` переписано: вердикт-роуты (если заданы) → иначе строго колонка справа → справа пусто = stay + done. Статус: `terminal→done`, `persona→running`, иначе `idle`. Ноль литералов dev/review/retro/ready. `custom_flow` игнорируется (поле оставлено для совместимости).
- `PipelineColumn`: +`autoRun?`, +`terminal?` (Rust `Option<bool>`, TS optional; backward-compatible).
- Стор generic: `STAGE_ORDER`, `fastTrack`-ветки, `backlog`-спецкейсы удалены; статус/авторан/промоушен одной формулой; `loadColumns()` тянет `get_pipeline` на старте и смене проекта (раньше колонки вообще не грузились из бэкенда).
- Модалка создания: без чекбоксов Ревью/Ретро, без `customFlow`/`fastTrack`.
- Колонки: кнопка «+ Колонка» (название + агент + автоцвет, вставка перед terminal), drag-reorder колонок (горизонтальный DnD, backlog pinned first / ready pinned last — и на фронте, и нормализацией в `save_pipeline`), удаление через ПКМ-меню шапки (× убран).
- Backlog/ready: неудаляемые (guard + скрытие), без персоны, persona-less скрыты из матрицы.
- Создание задачи ТОЛЬКО общей кнопкой (плюсики с колонок убраны) → всегда в backlog.
- ПКМ-меню: на задаче (Открыть/Запустить/Переместить/Удалить) и на шапке колонки (Переименовать/Сменить агента/Удалить).
- Создание колонки: select существующих агентов (`list_agent_personas`) + «+ Новый агент» → `create_agent_persona` пишет плейсхолдер `.cvh/agents/<role>.md`.
- Проводник разума: стадии строятся из колонок (уникальные persona по порядку), .md грузится через `read_cortex_file`, отсутствие → плейсхолдер + «Создать шаблон» (`save_cortex_file`). Моки только как fallback.
- Слово `artisan` вычищено везде (дефолт персоны `developer`).

### 1.5. Лента чата и карточки
- «Ответы агента» скрываемы (свёрнуты по умолчанию, Markdown только при раскрытии).
- Умный автоскролл (только если внизу, иначе кнопка «↓ К новому»).
- Thinking: бейдж «размышление...» убран, живой таймер в шапке thinking-блока.
- TPS морозится на финише (`finalTps`), сброс на `turn_start`.
- «Кеш % · TPS» всегда видны строкой справа от кольца контекста (из ховера убраны).
- Markdown: GFM-таблицы (терпимые к битым строкам), `---`→`<hr>`, `####`.
- Карточка: футер в 2 строки (модель + ветка + время в колонке «в колонке Nм»), персоны нет; бейдж времени тикает каждую секунду; счётчик «Всего задач» в шапке починен по высоте.
- Кнопка «Запустить агента» убрана из шапки чата (остались инпут «Отправить», пустое состояние, «Повторить запуск»).
- Плейсхолдер «Агент приступил к работе» удалён.

### 1.6. Кеш (честный итог)
- Наша сторона приведена в порядок: детерминированный префикс (сортировка custom tools, frozen breakpoints на system+user вместо прыгающего ephemeral), compactor срабатывает только при `turn>3 && tokens>40k` (раньше каждый терн с 3-го ломал префикс мутацией), парсинг всех форм usage (`cached_tokens`, `cached_content_token_count` и др.), eprintln-логи `CVH [cache]` (prefix_hash до/после) и `CVH [llm_client] usage raw=` на каждый терн.
- Главная причина нулей — прокси `:8045` (Antigravity-архитектура OpenAI→Gemini): `cache_control` не транслируется, `cachedContentTokenCount` отбрасывается из usage. Если `prefix_hash` стабилен, а `cached_*` в `usage raw=` нет — чинить маппер ответа прокси (`usageMetadata.cachedContentTokenCount → usage.prompt_tokens_details.cached_tokens`). Gemini implicit caching при этом может реально работать (скидка), просто неинструментирован.
- UDF-грант из прошлой смены: `main.rs` теперь ещё и `icacls *S-1-15-2-1:(OI)(CI)F` на `%LOCALAPPDATA%\com.voimax.cvh` (Windows).

---

## 2. Текущее состояние архитектуры (что теперь правда)

- **Колонка = агент.** Всё кастомное: добавление/удаление/переименование/DnD-перестановка. `backlog` всегда первая, `ready` всегда последняя (особые: без персоны, неудаляемые, скрыты из матрицы). Нормализация порядка и на фронте, и в `save_pipeline`.
- **Продвижение:** вердикт-роуты → иначе сосед справа → справа пусто = stay+done. Статус из `terminal/persona`.
- **Персоны:** `.cvh/agents/<role>.md`, выбор из существующих или создание с плейсхолдером при создании/смене агента колонки. Проводник разума строится из колонок.
- **Политики тулов:** `effective = global && stage`, канон `.cvh/settings/tool-policy.json` (+миграция legacy `.cvh/tool_policy.json`). Отключённые тулы вырезаются из схемы до LLM + гейт на исполнении.
- **Системный промпт худой:** только persona + tool instructions (+custom tools из `.cvh/tools`). Корневой `AGENTS.md` и `.agents/skills` больше не шлются модели.
- **События:** ровно один `agent:finished` за прогон (+фронт-дедуп), `task:moved` отдельно.
- **Запуск:** приложение живое, PID 80080, лог `/tmp/cvh_app.log`. Последняя проверка: `npm run build` OK, `cargo test` 21/21 OK.

## 3. Как запускать

**Linux (эта машина):**
```bash
npm run build
export PATH="$HOME/.cargo/bin:$PATH" && cargo build   # в src-tauri/
setsid env DISPLAY=:0 ./src-tauri/target/debug/canban-voimax-harness > /tmp/cvh_app.log 2>&1 < /dev/null &
# workdir ОБЯЗАТЕЛЬНО корень репо! гасить только kill <PID>, НЕ pkill -f (убьёт свой shell)
```
**Windows (домашняя машина):** тулчейн `llvm-mingw + WinSDK` из прошлой смены (см. git-историю HANDOFF: `$llvmBin/clang`, `LIB`, `RUSTFLAGS=-C linker=rust-lld`, `npm run tauri build`). Артефакты: `src-tauri\target\release\canban-voimax-harness.exe`, NSIS-сетап в `bundle\nsis`. После пула с этой сессии — пересобрать release (в коде есть изменения Rust: политики, промоушен, персоны IPC).

## 4. Известно / TODO на дом

1. **Прокси `:8045` и кеш** — см. 1.6. Без фикса маппера телеметрия кеша всегда 0.
2. **Модель многословна**: каждый терн = блок «Ответ агента» (до 25). Повторы одинаковых саммари подряд не схлопываются — кандидат на display-дедуп идентичных соседних output-блоков.
3. **Остатки `fastTrack`/`customFlow`**: из роутинга выведены, но поля/типы ещё лежат (`types`, маппер прокидывает как данные). Можно удалить следующим заходом.
4. **`tasks.json`/`pipeline.json`/персоны в `.cvh/` изменены рантаймом** (задачи CVH-001..004, колонки, disables) — это данные, не код; перед коммитом решить что коммитить.
5. `mockData.ts` (INITIAL_COLUMNS/CORTEX) оставлен только как seed для web-preview.
6. Дефолтный `pipeline.json`/`default_pipeline_config` на английском vs русские названия в UI — после сброса конфига имена «переведутся»; seed стоит синхронизировать.
7. `discover_project_root` содержит захардкоженный Windows-путь (`config.rs`) — не трогали.

## 5. Git-статус (не закоммичено!)

- Последний коммит `2294135`; сверху **27 изменённых + 2 новых файла** (`ColumnContextMenu.tsx`, `TaskContextMenu.tsx`), дифф ~+2400/−700.
- Требуется решение: `git add` осознанно (код vs `.cvh/`-данные), коммит, пуш в `origin/main` — чтобы дома продолжить `git pull`.
- HANDOFF этот файл — тоже незакоммичен, коммитить вместе.