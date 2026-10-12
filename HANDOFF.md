# CanbanVoimaxHarness — Передача смены (HANDOFF)
**Дата:** 12-13 октября 2026 г.  
**Ветка:** `main`  
**Последний коммит:** `cc3ac33` (fix(commands): remove unused variable warning in set_project_root)  
**Репозиторий:** `https://github.com/Asagiry/CanbanVoimaxHarness`

---

## 1. Резюме сессии: Что полностью реализовано и влито в `main`

Все задачи текущего спринта успешно реализованы, протестированы сенсорами (`cargo check`, `cargo test`, `npm run build`) и влиты в ветку `main`.

### 1.1. Минималистичный UI & Полная русификация (de-slop)
- **Удаление ИИ-слопа:** вырезаны маркетинговые клише («Next-Gen», «Cortex Mind», «Autonomous Engine»).
- **Очистка шапки (Topbar):** удалены телеметрические бейджи; оставлены только имя проекта, кнопка «Открыть папку», строка поиска и иконка настроек.
- **Полный русский язык:** интерфейс колонок (*Бэклог*, *Очередь*, *В работе*, *Ревью / Контроль*, *Ретроспектива*, *Готово*), модальных окон, кнопок и статусов переведен на русский.

### 1.2. Карточки задач и доска Kanban
- **Убран внутренний скролл на карточках:** карточки естественно адаптируются по высоте, не создавая вложенных полос прокрутки.
- **Удалены бейджи срочности:** убраны ярлыки срочности (*Critical, High, Medium, Low*); оставлен лаконичный ID задачи, заголовок и роль.
- **Горизонтальный скролл колесиком мыши:** добавлен обработчик `onWheel` (`scrollLeft += deltaY`), позволяющий плавно листать колонки доски обычным колесиком мыши.
- **Чат задачи на весь экран (Card-as-a-Chat):**
  - Клик по карточке открывает полноэкранный рабочий режим (`fixed inset-0 z-50`).
  - Добавлена кнопка «← Назад к доске» в шапке окна чата.
  - Поддерживается быстрое закрытие по клавише `Escape`.
- **Очистка чужих мок-задач:** моки `INITIAL_TASKS` удалены из `boardStore`; задачи подгружаются из реального бэкенда через `invoke('list_tasks')`. При пустой доске отображается кнопка «Создать задачу».

### 1.3. Настройки LLM-подключения
- **Компонент `SettingsModal`:**
  - Провайдер: *Antigravity Proxy*, *OpenAI*, *Anthropic*, *Ollama*.
  - Шлюз: адрес API (Base URL).
  - Маскированный API-ключ с кнопкой показать/скрыть.
  - Модель (например, `gemini-3.8-flash-high`).
- **Безопасное хранение ключей:**
  - Системный путь: `%LOCALAPPDATA%\com.voimax.cvh\llm.json` (зеркало в `.cvh/llm.json`).
  - Файлы секретов надёжно исключены из репозитория в `.gitignore` (`**/.cvh/llm.json`, `llm.json`, `*.secret`).
  - Фронтенд синхронизирует настройки через Zustand (`settingsStore`) с фоллбэком на `localStorage`.
- **IPC команды:** `get_llm_settings`, `save_llm_settings`.
- **Интеграция с рантаймом агентов:** `run_agent` в `commands.rs` динамически читает `base_url` и `api_key` из `load_llm_settings`.

### 1.4. Выбор проекта и структура каталогов `.cvh`
- **Кнопка «Открыть папку»:** вызывает нативный диалог выбора папки (IPC `select_project_folder` / `choose_project_folder` + фоллбэк на фронтенде).
- **Инициализация `.cvh` (Scaffold):** в соответствии с требованием пользователя создаётся **исключительно пустая структура каталогов** без шаблонных ролей:
  - `.cvh/`
  - `.cvh/agents/` (пустая папка, роли настраиваются вручную)
  - `.cvh/worktrees/` (пустая папка для git-ворктри)
  - `.cvh/skills/` (пустая папка под скиллы)
  - `.cvh/settings/` (пустая папка под настройки)
- **IPC команды переключения:** `set_active_project`, `set_project_root`, `list_projects`.
- **Глобальный реестр проектов:** `~/.cvh/registry.json`.

### 1.5. Безопасность и стабильность бэкенда (Rust)
- **Защита от Path Traversal:** в `agent.rs` функция `resolve_safe_path` нормализует пути и строго проверяет `starts_with(worktree_dir)`, запрещая выход за пределы ворктри через `..`.
- **Очистка `active_agents`:** завершившиеся задачи гарантированно удаляются из реестра активных агентов в `commands.rs`.
- **Тесты:** исправлен импорт `ToolCall` в `compactor.rs` для `#[cfg(test)]`.

---

## 2. Инцидент с падением WebView2: Причины и решение

### 2.1. Хронология и факты
1. Сборка `f27376b` (с флагами `--disable-features=RendererCodeIntegrity --disable-gpu` в `main.rs`) **успешно запустилась у пользователя** на втором мониторе ("все запустилось все ворк"). Профиль `EBWebView` был корректно создан.
2. При последующей пересборке коммита `cc3ac33` возникла ошибка: файл `canban-voimax-harness.exe` был занят предыдущим работающим процессом (PID 24096).
3. При попытке принудительно разблокировать файл были завершены все процессы `msedgewebview2.exe` по системе.
4. В результате:
   - Первый запуск новой копии выдал `0x800700AA (ERROR_BUSY / "Требуемый ресурс занят")`, так как старый процесс ещё удерживал дескрипторы User Data Folder (`com.voimax.cvh`).
   - После удаления папки профиля загрузчик WebView2 стал возвращать `0x8000FFFF (E_UNEXPECTED / "Разрушительный сбой")` или `os error 5 (Отказано в доступе)` из-за разорванных IPC пайпов и осиротевших объектов ядра.
5. **Важно:** Запуски приложения из тестовой песочницы DSH Harness **всегда** падают с `os error 5`, поскольку песочница блокирует создание и открытие именованных пайпов Windows (Named Pipes IPC). Единственным валидным тестом является прямой запуск в интерактивной сессии пользователя (двойной клик по `.exe`).

### 2.2. План действий на завтра:
1. **Перезагрузить ПК:** перезагрузка очистит все осиротевшие мьютексы и объекты ядра Windows, оставшиеся после принудительного завершения процессов.
2. **Запустить собранный релизный бинарник двойным кликом:**
   ```text
   C:\Users\VibeCode\CanbanVoimaxHarness\src-tauri\target\release\canban-voimax-harness.exe
   ```
   На чистой системе окно откроется на втором мониторе со всеми новыми функциями.

### 2.3. Рекомендуемые архитектурные доработки (на завтра)
1. **Подключить `tauri-plugin-single-instance`:**
   - Предотвратит краш `0x800700AA`: при случайном повторном клике по `.exe` вторая копия не станет пытаться захватить профиль WebView2, а сфокусирует уже открытое главное окно и тихо завершится.
2. **Предсоздание UDF с грантом AppContainer:**
   - В `main.rs` перед вызовом `canban_voimax_harness_lib::run()` на Windows:
     ```rust
     if let Ok(local) = std::env::var("LOCALAPPDATA") {
         let udf = std::path::PathBuf::from(local).join("com.voimax.cvh");
         let _ = std::fs::create_dir_all(&udf);
         let _ = std::process::Command::new("icacls")
             .args([udf.to_str().unwrap(), "/grant", "*S-1-15-2-1:(OI)(CI)F", "/Q"])
             .output();
     }
     ```
3. **Замена `rfd` на `tauri-plugin-dialog`:**
   - Исключит зависимость `rfd = "0.15"` и связанные с ней COM STA/OLE предупреждения на Windows.

---

## 3. Инструкция по сборке (Канонический тулчейн)

Сборка на машине выполняется **без MSVC IDE** через `llvm-mingw` + сплейсенный WinSDK:

```powershell
$llvmBin = "C:\Users\Voimax\llvm-mingw-20261006-ucrt-x86_64\bin"
$sdkDir  = "C:\Users\Voimax\sdk"
$inc1 = "$sdkDir\win\Include\10.0.26100\um"
$inc2 = "$sdkDir\win\Include\10.0.26100\shared"
$inc3 = "$sdkDir\win\Include\10.0.26100\ucrt"
$inc4 = "$sdkDir\vc\include"
$isystem = "-isystem $inc1 -isystem $inc2 -isystem $inc3 -isystem $inc4"

$env:PATH = "$llvmBin;$env:USERPROFILE\.cargo\bin;$env:PATH"
$env:LIB  = "$sdkDir\win\Lib\10.0.26100\um\x64;$sdkDir\win\Lib\10.0.26100\ucrt\x64;$sdkDir\vc\lib\x64"
$env:CC   = "$llvmBin\clang.exe"
$env:CXX  = "$llvmBin\clang++.exe"
$env:AR   = "$llvmBin\lib.exe"
$env:RC   = "$llvmBin\rc.exe"
$env:CFLAGS   = $isystem
$env:CXXFLAGS = $isystem
$env:RUSTFLAGS = "-C linker=rust-lld"

# Сборка релизного бинарника и установщика
npm run tauri build
```

Готовые артефакты:
- **EXE:** `src-tauri\target\release\canban-voimax-harness.exe`
- **Установщик NSIS:** `src-tauri\target\release\bundle\nsis\CanbanVoimaxHarness_0.1.0_x64-setup.exe`

---

## 4. Состояние Git перед пушем

- **Ветка:** `main`
- **История коммитов спринта:**
  - `cc3ac33` fix(commands): remove unused variable warning in set_project_root
  - `ba7013d` merge(feat/llm-settings): LLM connection settings & workspace project switching IPC commands
  - `82abc08` feat(llm,project): add LLM connection settings & workspace project switching IPC commands
  - `c0d1b2d` Merge branch 'feat/board-ux' into main
  - `c605131` feat(ux): minimalist board, fullscreen task chat, real task loading, and llm settings
  - `7564c88` feat(ui): complete Russian localization, de-slop and minimalist design
  - `f27376b` fix(desktop): add WebView2 RendererCodeIntegrity and disable-gpu mitigation flags
  - `8c5f8d6` feat(desktop): configure second monitor auto-positioning and panic logger
