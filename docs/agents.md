# 🤖 Руководство по архитектуре и настройке агентов (docs/agents.md)

> **CanbanVoimaxHarness (CVH)** — легковесная (~40MB RAM) агентная среда и оркестратор на Rust (Tauri v2) и React 19.
> В CVH агенты не живут в изоляции от интерфейса: **колонки Kanban-доски и есть агенты**.

---

## 1. Как работают агенты в CanbanVoimaxHarness

Агентный цикл в CVH построен на принципах максимальной производительности, детерминизма и защиты от галлюцинаций:

1. **ReAct-цикл нативно в Rust (`src-tauri/src/agent.rs`)**:
   - Асинхронное исполнение на Tokio runtime.
   - Стриминг Server-Sent Events (SSE) в реальном времени через Tokio MPSC-каналы.
   - Разделение и коалесценция потоков: рассуждения (`thought` / `reasoning_content`) и контент ответов отображаются в единой хронологической ленте без лагов интерфейса.
2. **Zero-Turn Memory Priming (Инвариант нулевого хода)**:
   - Агент **никогда не тратит первый ход** на команды вроде `read AGENTS.md` или чтение системных инструкций.
   - Перед первым вызовом LLM бэкенд на Rust компилирует единый системный контекст:
     ```text
     SystemMessage = Persona (.cvh/agents/<role>.md) 
                   + Архитектурные правила проекта (AGENTS.md) 
                   + Активные скиллы (.agents/skills/*) 
                   + Контекст задачи (Task Prompt)
     ```
   - С первого же токена агент знает свою роль, правила безопасности, доступные сенсоры и задачу.
3. **Изолированные Git Worktrees (`worktrees/<task-slug>`)**:
   - Агенты этапа разработки работают исключительно внутри своей директории ворктри.
   - Защита от разрушения основного дерева: любые правки, тесты и сборки происходят в ветке задачи (`feat/<task-id>`).
4. **State-Preserving Compactor & Deterministic Prefix Caching**:
   - Длинные выводы тулов (>500 символов старше 2 шагов) сжимаются, экономя контекст.
   - Заголовки, системные промпты и определения тулов сортируются строго по алфавиту для достижения >90% попаданий в Prompt Cache (Anthropic / DeepSeek / Gemini).

---

## 2. Персоны агентов (`.cvh/agents/<role>.md`)

Каждая активная колонка доски связывается с файлом персоны через свойство `persona: "<role>"` в конфигурации пайплайна или доски.

### 2.1 Расположение и структура файлов
Файлы персон хранятся в папке:
```text
.cvh/agents/
├── developer.md   # Ведущий разработчик (agent-developer)
├── reviewer.md    # Критический ревьюер (agent-developer-reviewer)
├── retro.md       # Ретроспективный аудитор (agent-retro-verifier)
└── <custom>.md    # Любая кастомная роль (например, modeler.md, copywriter.md)
```

### 2.2 Пример персоны: Разработчик (`developer.md`)
```markdown
# 💻 Persona: Lead Developer (agent-developer)

You are the Lead Developer and Systems Engineer for **CanbanVoimaxHarness (CVH)**.
Your mission is to implement robust features across the Rust/Tauri v2 desktop backend and React 19 frontend.

### Engineering Responsibilities:
- Author systems code inside `src-tauri/` adhering to strict memory safety and Tokio async conventions.
- Author modern, high-density React 19 / TypeScript UI components inside `src/`.
- Run Ground Truth Runtime Sensors before submitting changes (`cargo check`, `pnpm run build`).
- Ensure all execution happens inside isolated git worktrees (`worktrees/<task-slug>`).

### Architectural Invariants:
1. **Rust Safety**: Zero naked `.unwrap()` in production paths. Always use `Result<T, AppError>`.
2. **React 19 Standards**: Monospace tabular numerals (`tabular-nums`), cyber-slate theme, zero AI-slop.
3. **Ponytail Minimalism**: Zero dead code, no placeholder functions.
```

### 2.3 Пример персоны: Ревьюер (`reviewer.md`)
Ревьюер работает в режиме «Красной команды» (Devil's Advocate):
- **Zero Code Mutation Invariant**: Ревьюер не имеет инструментов модификации кода (`edit_file` и `write_file` отключены на уровне Tool Policy).
- Обязан проверить изменения через сенсоры и вынести один из строгих вердиктов:
  - `APPROVED`: Изменения безупречны, готовы к слиянию.
  - `APPROVED_WITH_CAVEATS`: Мелкие некритичные замечания.
  - `CHANGES_REQUESTED`: Найдены баги, нарушения инвариантов или регрессии.

---

## 3. Политика инструментов: Tool Policy Matrix

Безопасность и контроль доступа к системным вызовам обеспечиваются двухслойной матрицей с **логикой AND**.

### 3.1 Единый источник правды: `.cvh/settings/tool-policy.json`
```json
{
  "globalEnabled": {
    "execute_command": true,
    "read_file": true,
    "edit_file": true,
    "write_file": true,
    "glob": true,
    "grep": true,
    "web_search": true,
    "web_fetch": true,
    "subagent": true
  },
  "stageOverrides": {
    "dev": {
      "execute_command": true,
      "read_file": true,
      "edit_file": true,
      "write_file": true,
      "glob": true,
      "grep": true,
      "web_search": false,
      "web_fetch": true,
      "subagent": true
    },
    "review": {
      "execute_command": true,
      "read_file": true,
      "edit_file": false,
      "write_file": false,
      "glob": true,
      "grep": true,
      "web_search": true,
      "web_fetch": true,
      "subagent": true
    },
    "retro": {
      "execute_command": true,
      "read_file": true,
      "edit_file": false,
      "write_file": true,
      "glob": true,
      "grep": true,
      "web_search": false,
      "web_fetch": false,
      "subagent": false
    }
  }
}
```

### 3.2 Принцип работы логики AND
Решение о доступности инструмента принимается функцией `is_tool_allowed`:
```rust
effective = global_enabled[tool] && stage_overrides[stage][tool]
```
- Если инструмент выключен в `globalEnabled`, **ни одна колонка не сможет включить его обратно**.
- Если записи для стадии нет, по умолчанию берется значение глобального переключателя.
- Алиасы автоматически нормализуются: `bash` → `execute_command`, `str_replace` → `edit_file`.

### 3.3 Zero-Trust Schema Exclusion (Защита от галлюцинаций)
Если инструмент отключен для текущей стадии задачи:
- Его определение схемы JSON **полностью вырезается** из массива `tools` запроса к LLM.
- Это экономит до 2,500 токенов на каждом запросе.
- Модель физически не знает о существовании запрещенного инструмента и не может галлюцинировать попыткой его вызвать.

---

## 4. Кастомные инструменты (`.cvh/tools/`)

Систему можно расширять кастомными тулами без перекомпиляции Rust-бэкенда.

### 4.1 Создание кастомного тула
Любой `.json` файл в каталоге `.cvh/tools/` автоматически регистрируется как доступный инструмент:

```json
// .cvh/tools/run_tests.json
{
  "name": "run_tests",
  "description": "Запуск тестов в изолированном ворктри с фильтром по модулю",
  "parameters": {
    "type": "object",
    "properties": {
      "module": {
        "type": "string",
        "description": "Название тестового модуля или пакета (опционально)"
      }
    }
  },
  "command": "cargo test {{args}}"
}
```

### 4.2 Шаблонизация и исполнение
- Подстрока `{{args}}` заменяется сериализованным JSON аргументов или распакованными флагами.
- Команда выполняется внутри директории активного ворктри с таймаутом и контролем дочерних процессов.
- Если вывод команды превышает 100 строк или 8 КБ, срабатывает **Output Spilling**:
  - Полный лог пишется в `.cvh/spill/<timestamp>-<tool>.log`.
  - В контекст возвращаются первые 20 строк (HEAD), последние 20 строк (TAIL) и путь к файлу.

---

## 5. Пошаговая инструкция: перенос скиллов и ролей из других проектов (например, carSmash)

Если у вас есть наработки из другого проекта (роли, скиллы, архитектурные промпты), следуйте этому алгоритму для их интеграции в CVH.

### Шаг 1. Анализ ролей и обязанностей
Определите, какие роли выполняли задачи в исходном проекте. Например, в carSmash:
- `VehiclePhysicsEngineer` (настройка физики колес и столкновений).
- `VoxelAssetArtist` (генерация и оптимизация воксельных сеток).
- `GameplayAuditor` (ревью баланса и стабильности FPS).

### Шаг 2. Создание файлов персон в `.cvh/agents/`
Создайте markdown-файлы для каждой роли:
```text
.cvh/agents/
├── physics-engineer.md
├── voxel-artist.md
└── gameplay-auditor.md
```
В каждом файле зафиксируйте:
- Имя персоны и миссию.
- Инварианты предметной области (например: «Никогда не использовать сжатие с потерями для воксельных палитр»).
- Ожидаемый формат вердикта (для аудиторов: `APPROVED`, `CHANGES_REQUESTED`).

### Шаг 3. Перенос скиллов в `.agents/skills/`
Скиллы проекта оформляются в виде модулей с файлом `SKILL.md`:
```text
.agents/skills/
├── voxel-art/
│   └── SKILL.md
└── vehicle-physics/
    └── SKILL.md
```
В начале файла укажите YAML-шапку:
```markdown
---
name: vehicle-physics
description: Экспертиза физики подвески, сил трения и деформации кузова.
---

# Руководство по физике
...
```
Rust-бэкенд CVH автоматически компилирует подходящие скиллы в системный промпт при запуске задачи.

### Шаг 4. Настройка политик доступа (`.cvh/settings/tool-policy.json`)
Определите границы прав для каждой стадии:
- Разработчикам физики разрешите `execute_command`, `edit_file`, `write_file`.
- Воксельным художникам разрешите кастомные тулы генерации и `read_file`.
- Аудиторам запретите мутацию кода (`edit_file: false`, `write_file: false`).

### Шаг 5. Оформление специфичных тулов в `.cvh/tools/`
Если в carSmash использовались скрипты (например, `generate_vox.py` или `bake_colliders.exe`), оберните их в `.cvh/tools/`:
```json
// .cvh/tools/bake_colliders.json
{
  "name": "bake_colliders",
  "description": "Сгенерировать упрощенные коллайдеры для воксельной сетки",
  "parameters": {
    "type": "object",
    "properties": {
      "mesh_path": { "type": "string" }
    },
    "required": ["mesh_path"]
  },
  "command": "python scripts/bake_colliders.py {{args}}"
}
```

### Шаг 6. Привязка к колонкам доски (`.cvh/boards.json`)
Включите созданные персоны в целевую доску:
```json
{
  "id": "physics",
  "title": "Физика",
  "persona": "physics-engineer",
  "allowedTools": ["execute_command", "read_file", "edit_file", "write_file"]
}
```

Теперь перетаскивание карточки задачи в колонку «Физика» автоматически запускает агента с нужной персоной, памятью и набором инструментов!
