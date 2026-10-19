# 🚀 ВЕЧЕРНИЙ ХЕНДОФФ: АРХИТЕКТУРНАЯ СИНХРОНИЗАЦИЯ ITCODASHBOARD & CANBANVOIMAXHARNESS

**Дата:** 19 октября 2026  
**Коммиты в main:** `f95606e`, `9257813`, `e4f5f50`, `b274c53`  
**Статус репозитория:** Чистый, синхронизирован с `origin/main` (`git push` выполнен)  
**Датчики Ground Truth:** 
- Frontend Bundle: `pnpm run build` (tsc && vite build) — **EXIT CODE 0** (1656 модулей)
- Rust Compiler: `cargo check --manifest-path src-tauri/Cargo.toml` — **EXIT CODE 0**
- Rust Linter: `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` — **EXIT CODE 0** (0 warnings, 0 errors)
- Unit Tests: `cargo test commands::tests` — **41 passed, 0 failed**

---

## 1. Точная первопричина найденных аномалий («Призрачные агенты», разрыв отчета, бешеный TPS)

### Аномалия 1: Два агента параллельно крутили одну задачу (CVH-003)
- **Механизм**: При создании или переносе карточки в колонку «Разработка» происходил двойной триггер:
  1. Фронтенд (`boardStore.ts:moveTask`) вызывал `await triggerRunAgent(taskId)`.
  2. Бэкенд (`tasks.rs:move_task_stage`) в ту же секунду генерировал событие `agent:autorun`, которое в `App.tsx` повторно дергало `runTaskAgent(taskId)`.
  3. В `src-tauri/src/commands/tasks.rs` в функции `run_agent` **не было проверки** `state.active_agents.contains_key(&task_id)`. Вместо отклонения дубля, бэкенд перезаписывал хэндл в `map.insert` и спавнил второй параллельный Tokio-поток (`tokio::spawn(run_agent_loop)`).
- **Следствия**:
  - Космический TPS и пулеметный вызов тулов (два агента Gemini параллельно поливали сокет).
  - Дублирование тулов (`glob: *` x2, `read_file` x3).
  - Разрыв финального отчета на куски: Поток 1 уже писал финальный отчет, а Поток 2 отставал и в этот момент слал команды `git mv`, `python` и `read_file`. Из-за промежуточных тулов Потока 2 фронтенд классифицировал куски отчета Потока 1 как промежуточные «Пояснения агента», а не как финальный «Ответ».
  - Поломка кнопки «Остановить»: нажатие «Стоп» глушило Поток 2 (чья ссылка лежала в словаре), а Поток 1 оставался зомби в Tokio-памяти и продолжал жить своей жизнью.

### Аномалия 2: Субагенты завершались за «0ms, 0 шагов, Готово»
- **Механизм**: В `src-tauri/src/agent.rs` стоял ошибочный костыль: если вызывался синхронный `subagent`, а в фоне уже крутился похожий агент, тул не ждал его, а мгновенно возвращал текстовую строку: `"[Субагент уже выполняется... используйте collect_subagent]"`.
- **Следствия**:
  - Фронтенд получал мгновенный ответ тула и рисовал в сайдбаре: «0ms, 0 шагов, Готово».
  - Модель читала этот текст, думала, что ей велели вызвать `collect_subagent`, вызывала его и вставала в зависание.

---

## 2. Что конкретно исправлено в коде (Все 7 дефектов пользователя)

### 1. Защита от призрачных агентов (Guard в `tasks.rs` & устранение дубля в `boardStore.ts`)
- В самом начале `src-tauri/src/commands/tasks.rs` в `run_agent`:
  ```rust
  let is_running = {
      let state = app_handle.state::<AppState>();
      state.active_agents.lock().map(|map| map.contains_key(&task_id)).unwrap_or(false)
  };
  if is_running {
      eprintln!("CVH [run_agent]: Task {} already has an active agent loop, rejecting duplicate run", task_id);
      return Ok(format!("Agent already running for task {}", task_id));
  }
  ```
  Повторный запуск моментально отклоняется без спавна фонового потока.
- В `src/stores/boardStore.ts` в `moveTask` удален лишний ручной вызов `triggerRunAgent`. Единственный источник правды — событие `agent:autorun` от бэкенда.
- Покрыто юнит-тестом: `commands::tests::test_active_agents_duplicate_run_check`.

### 2. Поддержка `skip_worktree` («Не создавать worktree»)
- В структуру `Task` (`agent.rs`) и `KanbanTask` (`types/index.ts`) добавлено поле `skip_worktree: Option<bool>`.
- В `create_task`: флаг сохраняется намертво.
- В `move_task_stage` и `advance_task`: если `skip_worktree == true`, вызовы создания ворктри и веток заблокированы.
- В `run_agent`: при `skip_worktree == true` агент работает напрямую в корне (`root.clone()`), без папок `worktrees/<slug>`.
- В `DrawerHeader.tsx`: плашки ворктри и ветки скрываются, если `skipWorktree === true`.
- Покрыто юнит-тестами: `test_skip_worktree_flag_serialization_and_defaults` и `test_skip_worktree_prevents_builtin_worktree_in_move_stage`.

### 3. Честное синхронное ожидание субагентов и честный таймер
- В `src-tauri/src/agent.rs`:
  - Вырезан возврат текстовой заглушки `[Субагент уже выполняется...]`. Синхронный `subagent` всегда запускает выполнение и блокируется до получения реального отчета.
  - В `run_inprocess_subagent` замеряется `std::time::Instant::now()`. При завершении честный `durationMs` передается в событие `subagent:finished` и сохраняется в сессии/БД.
- В `src/components/drawer/SubagentsSidebar.tsx`:
  - Исключен ошибочный захват `durationMs` из вызова `subagent_bg` (181мс).
  - Для работающих агентов считается живой elapsed от `createdAt` до `now`.
  - Для завершенных — реальное время работы.

### 4. Порядковые номера субагентов (`СУБАГЕНТ #1`, `#2`) и крупный бейдж
- В `SubagentsSidebar.tsx`, `ToolExecutionCard.tsx`, `DrawerHeader.tsx`, `DrawerSteerBar.tsx`:
  - Субагентам присваиваются порядковые номера: `#1`, `#2`...
  - Слово `СУБАГЕНТ` увеличено (`text-[11px] font-mono font-bold tracking-wider`).

### 5. Очистка шапки инпута субагента
- В `DrawerSteerBar.tsx`: удалена строка «Уточнение задачи субагента» над полем ввода.

### 6. Удаление сырых ISO-таймстампов & Длительность на каждом шаге
- В `DrawerActivityStream.tsx` и `ThinkingCard.tsx`:
  - Полностью удалены технические строки вида `2026-10-19T13:46:03.338969300+00:00`.
  - На **каждой** карточке инструмента, мыслей (`ThinkingCard`) и сообщений выводится прошедшее время в формате `tabular-nums` (например, `450ms`, `1.2s`, `35s`).

### 7. Логгер LLM-диалогов в реальном времени (`.cvh/llm_debug.log`)
- В `src-tauri/src/agent.rs` реализована функция `append_llm_debug_log`.
- На каждом ходе (как в основном агенте, так и в субагентах) в файл `.cvh/llm_debug.log` пишутся:
  - Точный запрос (системный промпт, сообщения, схемы инструментов).
  - Точный ответ модели (текст, рассуждения, вызовы тулов).
- Покрыто юнит-тестом: `test_append_llm_debug_log`.

---

## 3. Рекомендации для продолжения разработки дома

1. **Запуск**:
   - `git pull origin main`
   - `pnpm run dev` или запуск Tauri приложения через `cargo tauri dev`.
2. **Проверка**:
   - Перетащи задачу или запусти агента — обрати внимание, что в консоли и в `.cvh/llm_debug.log` четко пишется один поток.
   - Если нажать «Остановить» — процесс мгновенно гасится с фиксацией времени.
   - При создании задачи с галочкой «Не создавать worktree» — агент выполняется прямо в корне, а лишние плашки скрыты.
3. **Отладка модели**:
   - Если захочешь посмотреть, что именно модель получает и отдает — открой файл `.cvh/llm_debug.log`.
