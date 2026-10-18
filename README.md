# 🚀 CanbanVoimaxHarness (CVH)

> **The Next-Generation Lightweight Desktop Agentic IDE & Orchestrator**  
> Built with **Rust 2021 (Tauri v2 Core)**, **Tokio**, **React 19**, **TypeScript**, and **Tailwind CSS v4**.

---

## 📚 Архитектурные руководства и документация (Documentation)

Полная документация по устройству и кастомизации CanbanVoimaxHarness находится в директории `docs/` (все этапы дорожной карты CVH-001—CVH-014 и архитектурные концепции Live Diff и Cortex Memory Ledger полностью реализованы в кодовой базе):

- **[📘 docs/GUIDE.md](docs/GUIDE.md)** — **Полный гайд по харнессу и `.cvh`**:
  - Полная сверка всех подсистем, инвариантов и IPC-команд с кодом бэкенда на Rust и фронтенда.
  - Пошаговые чеклисты настройки досок, колонок, персон, правил и хуков перехода.

- **[🤖 docs/agents.md](docs/agents.md)** — **Руководство по архитектуре и персонам агентов**:
  - Как устроен нативный ReAct-цикл в Rust и Zero-Turn Memory Priming.
  - Описание персон в `.cvh/agents/<role>.md`.
  - Двухуровневая матрица `.cvh/settings/tool-policy.json` с жесткой логикой `AND` и Zero-Trust вырезанием схем.
  - Кастомные инструменты в `.cvh/tools/<tool>.json` с Output Spilling.
  - Пошаговое руководство по переносу скиллов и ролей из сторонних проектов (например, carSmash).

- **[📋 docs/board.md](docs/board.md)** — **Архитектура досок и колонок (Колонки = Агенты)**:
  - Панель управления (Control Plane): почему колонки представляют активных агентов.
  - Фундаментальный инвариант: задачи ВСЕГДА двигаются из `Backlog` в `Ready`.
  - Мульти-доски в `.cvh/boards.json` и готовые шаблоны (`development`, `modeler`, `kanban`, `empty`).
  - Автоматический роутинг между стадиями по вердиктам (`APPROVED`, `CHANGES_REQUESTED`) и иерархия подзадач.

- **[⚡ docs/hooks.md](docs/hooks.md)** — **Автономные скрипты перехода на колонках**:
  - Механизм хуков жизненного цикла (`.bat`, `.ps1`, `.sh`, `.py`).
  - Контекстные переменные окружения (`CVH_TASK_ID`, `CVH_WORKTREE_PATH`, `CVH_BRANCH`, `CVH_STAGE_ID`, `CVH_VERDICT`).
  - Сенсоры-привратники (`gatekeeperSensor`) для защиты от сломанных сборок.
  - Готовые примеры: автоматическое создание Git worktree при взятии в работу и очистка при завершении.

---

## ⚡ Манифест быстрого онбординга новых проектов (Fast Onboarding Manifest)

CanbanVoimaxHarness разработан для мгновенного развертывания поверх **любого существующего репозитория** без навязывания чужих зависимостей.

```text
[Ваш репозиторий] + [.cvh/ конфигурация] = Автономная агентная IDE за 2 минуты
```

### 5 шагов для подключения проекта:
1. **Откройте папку проекта в CVH**:
   - При первом открытии бэкенд на Rust автоматически создаст легковесную структуру `.cvh/` (конфиг, пайплайн, папки для агентов, хуков и настроек) и добавит локальные секреты в `.gitignore`.
2. **Выберите или настройте доску (`.cvh/boards.json`)**:
   - Выберите готовый шаблон: классический 5-стадийный инженерный цикл (`development`), 3D-пайплайн (`modeler`) или минималистичный (`kanban`).
3. **Определите персоны агентов (`.cvh/agents/<role>.md`)**:
   - Добавьте файлы ролей под специфику вашего стека (например, `rust-engineer.md`, `react-artisan.md`, `security-auditor.md`).
4. **Задайте границы безопасности (`.cvh/settings/tool-policy.json`)**:
   - Настройте матрицу инструментов: разрешите запись только разработчикам, запретите модификацию файлов аудиторам, подключите кастомные скрипты сборки в `.cvh/tools/`.
5. **Создайте первую задачу на доске**:
   - Перетащите карточку из `Backlog` в `In Dev` — CVH мгновенно создаст изолированный Git worktree, скомпилирует контекст памяти и начнет решение задачи.

---

## 🌟 Mission & Architectural Pillars

**CanbanVoimaxHarness (CVH)** is the world's first lightweight (~38MB RAM) desktop AI orchestrator where the **Kanban board is the First-Class Control Plane**, not an afterthought or side-panel. Tasks flow autonomously through customizable agent pipelines with native Git Worktree isolation, deterministic prefix caching, zero-turn memory priming, and a two-tier zero-trust tool policy matrix.

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                                   CANBAN VOIMAX HARNESS                                │
│                                                                                        │
│   ┌─────────────────────┐  ┌──────────────────────────────────────────────────────┐  │
│   │   CORTEX EXPLORER   │  │                 KANBAN CONTROL PLANE                 │  │
│   │  (Mind Explorer)    │  │                                                      │  │
│   │                     │  │  [Backlog]  ➔  [In Dev]  ➔  [Review]  ➔  [Done]       │  │
│   │  🔵 backlog/        │  │     │             │            │           │         │  │
│   │  🟡 dev/            │  │     ▼             ▼            ▼           ▼         │  │
│   │     persona.md      │  │  Card #1       Card #2      Card #3     Card #4      │  │
│   │     rules/*.md      │  │  "Ideation"    "Backend"    "Audit"     "Shipped"    │  │
│   │  🟣 review/         │  └──────────────────────┬───────────────────────────────┘  │
│   │  🟢 ready/          │                         │ (Click Card)                     │
│   └─────────────────────┘                         ▼                                  │
│                                      ┌─────────────────────────┐                     │
│                                      │     CARD-AS-A-CHAT      │                     │
│                                      │  - Live Thinking Stream │                     │
│                                      │  - Rich Markdown Viewer │                     │
│                                      │  - Tool Diff Inspector  │                     │
│                                      │  - Steer Agent Prompt   │                     │
│                                      └─────────────────────────┘                     │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## ✨ Killer Features

### 1. 📋 Kanban Board as the First-Class Control Plane
- **Zero Chat Friction**: Rather than forcing work into a singular chaotic chat log, every unit of work lives in a Kanban card with strict stage transitions.
- **Dynamic Pipelines & Multi-Boards (`.cvh/boards.json`)**: Arbitrary multi-column routing graph (`A -> B -> C -> D -> E` or `Ideation -> Spec -> Dev -> QA -> Prod`) with per-stage context window caps (`400k`, `200k`, `128k`) and model overrides.
- **Monospace Tabular Timers (`font-mono tabular-nums`)**: Real-time stage timers (`Dev: 14m 23s`), frozen stage history timers, total elapsed duration, and glowing rework counters (`↩ Returns: 2x`).
- **Fast-Track & Verdict Badges**: `⚡ Fast-Track: No Review`, `⚡ Fast-Track: No Retro`, and official verdicts (`✔ APPROVED`, `✔ APPROVED WITH CAVEATS`, `✖ CHANGES REQUESTED`).

### 2. 💬 Card-as-a-Chat (Isolated Task Execution Streams)
- **Individual Conversation Streams**: Clicking any task card on the board slides open the task conversation drawer.
- **Live `<thinking>` Introspection**: Dedicated thinking block with dim monospace typography, glowing amber border, live reasoning indicator, and elapsed duration counters.
- **Rich Markdown Viewer (`MarkdownViewer.tsx`)**: Syntax-highlighted code snippets, language headers, and one-click copy buttons.
- **Interactive Tool Accordions & Visual Diff Viewer**:
  - Direct exit codes, command traces, and execution timings.
  - Dedicated **Line Gutter Diff Viewer** for `edit_file` / `str_replace` with red removals (`-`) and green additions (`+`).
- **Interactive Steer Bar**: Bottom input bar allows injecting instructions or teaching the agent mid-flight (`steer_agent` IPC) with quick-action suggestion chips (`[▶ Продолжай]`, `[🔍 Тесты]`, `[🛑 Откати]`, `[⚡ Сжать]`).

### 3. 🧠 Cortex Explorer («Проводник разума агентов»)
- **VS Code-Style Mind Explorer**: Left sidebar visualizes agent minds instead of traditional filesystems.
- **1-to-1 Column Color Sync**: Blue column = Blue folder with agent rules. Clicking any column header on the board automatically focuses and smoothly scrolls to the corresponding folder in Cortex Explorer.
- **In-App Rule Editor (`RuleEditorModal.tsx`)**: Live syntax editing for `persona.md`, modular rules `rules/*.md`, and `settings.json` with line gutters, word counts, and `Ctrl+S` tactile persistence.

### 4. 🎛️ Tool Policy Matrix (Two-Level Zero-Trust Iron Barrier)
- **Level A (Global Switches: Settings ➔ Tools)**: Master toggles for `execute_command` (bash), `read_file`,`edit_file`, `write_file`, `glob`, `grep`, `web_search`, `web_fetch`, `subagent`.
- **Level B (Per-Stage Permissions)**: Column-level boundaries (e.g. in *Review Gate*, `edit_file` and `write_file` are locked off completely—the reviewer cannot mutate code).
- **Strict AND-Semantics**: `effective = global_enabled && stage_override`. Stages cannot override a global disable.
- **Zero-Trust Schema Exclusion**: Disabled tools are **completely stripped from the LLM request schemas**, saving ~2,400+ prompt tokens per step and eliminating hallucinations.
- **Rust Iron Barrier**: Kernel-level interception in Rust returns `PermissionDenied` if a model attempts to invoke an unauthorized tool.

### 5. 🗜️ State-Preserving Context Compactor & Prefix Caching
- **Zero-LLM Log Evaporation**: Heavy terminal outputs and file dumps older than 3 turns (>500 chars) are automatically evaporated into lightweight summary tags (`[Output evaporated: 4,820 bytes. Exit: 0]`) without making LLM API calls, reclaiming up to 80% context tokens instantly.
- **Deterministic Prefix Caching (Cache Hit Rate >= 90%)**: Tool definitions are deterministically sorted by name (`tools.sort_by(|a, b| a.name.cmp(&b.name))`) before serialization. Zero dynamic timestamps or random UUIDs in the System prompt prefix.
- **Four Immutable Anchors**: System Prompt, initial Task Prompt (ТЗ), and the 2 most recent turns are strictly preserved to maintain KV prefix cache validity.
- **`ContextGauge` UI**: Real-time token consumption ring gauge with one-click `⚡ Compact Context` trigger.

### 6. 🌲 Native Git Worktree Isolation & Hooks
- Managed natively by Rust via `tokio::process::Command` without shell injection or deadlocks.
- Every task executes inside an isolated `worktrees/<task-slug>` directory on branch `feat/<task-slug>`.
- **Custom Lifecycle Hooks**: Runs `.bat`, `.ps1`, or `.sh` on stage entry, exit, or completion with environment variables `%CVH_TASK_ID%`, `%CVH_WORKTREE_PATH%`, `%CVH_BRANCH%`, `%CVH_STAGE_ID%`, `%CVH_VERDICT%`.

### 7. 🔍 Industrial SWE-bench Tools, Spilling & Custom Extensions
- Industrial signatures: `execute_command` (alias `bash`), `read_file`, `edit_file` (`str_replace`), `write_file`, `glob`, `grep`, `subagent`.
- **Dynamic Custom Tools**: Adding JSON schemas into `.cvh/tools/<tool>.json` auto-exposes custom commands with `{{args}}` templating.
- **Tool Output Spilling**: Outputs >100 lines or >8 KB are saved to `.cvh/spill/<timestamp>-<tool>.log`, passing HEAD 20 + TAIL 20 lines to LLM context.
- **`web_search`**: DuckDuckGo search (zero API key required) with Google/Bing fallback.
- **`web_fetch`**: HTTP GET converting HTML into clean Markdown.

### 8. 🗄️ High-Performance SQLite Storage, SVG Icons & Relay Handoff
- **SQLite Engine with WAL Mode (`.cvh/harness.db`)**: Chat messages and streaming deltas persist into an ACID SQLite database (`dialog_items` with index on `task_id, created_at`), replacing legacy flat JSON files.
- **Relay Handoff Memory Priming**: Handover summaries from `advance_task` and `reject_task` are injected as top-level banners (`### [ЭСТАФЕТА ПАЙПЛАЙНА / RELAY HANDOFF]`), providing zero-degradation context across pipeline transitions.
- **Backlog-to-Ready Invariant**: Inviolable board boundaries with immutable `backlog` (pos 0) and `ready` (terminal, last position) columns.
- **SVG Persona Icons**: Full support for canonical SVG persona icons (`Bot`, `Brain`, `Cpu`, `Code2`, `ShieldCheck`, `Sparkles`, `Terminal`, `Zap`, `Flame`, `Radar`, etc.) persisted in frontmatter or JSON settings.

---

## 🚀 Quickstart & Development

### Prerequisites
- **Node.js**: v20+ & pnpm v10+
- **Rust Toolchain**: Rust 2021 (Cargo 1.80+)
- **Windows Build Tools**: MSVC Build Tools for linking

### 1. Run Web UI Control Plane (Dev Mode)
```bash
# Start Vite development server with Hot Module Reloading
pnpm run dev
```
Open **[http://localhost:5173](http://localhost:5173)** in your browser.

### 2. Run Native Desktop Application (Tauri v2)
```bash
# Launch native desktop application with full Rust IPC bindings
pnpm run tauri dev
```

### 3. Build Native Desktop Executable (`.exe`)
```bash
# Compiles the single self-contained binary (~22MB)
pnpm run tauri build
```
Output executable:
📁 `src-tauri/target/release/CanbanVoimaxHarness.exe`

---

## 🏛️ Project Directory Structure

```text
CanbanVoimaxHarness/
├── AGENTS.md                # 3-Tier Governance & Multi-Agent Engineering Manual
├── docs/                    # Architectural Guides & Manuals
│   ├── GUIDE.md             # Complete harness & .cvh configuration manual
│   ├── agents.md            # Agent ReAct loop, Personas, Tool Policy, and Migration
│   ├── board.md             # Board Control Plane, Invariants, Multi-Boards, Auto-Routing
│   └── hooks.md             # Stage transition scripts, env vars, and worktree scaffolding
├── .cvh/                    # Project configuration for CVH
│   ├── harness.db           # SQLite database with WAL mode (dialogue & stream storage)
│   ├── boards.json          # Multi-board definitions & active board
│   ├── pipeline.json        # Dynamic pipeline columns & routing graph
│   ├── agents/              # Modular persona folders (<role>/persona.md, setting, rules/)
│   ├── settings/            # Tool Policy Matrix (tool-policy.json)
│   ├── tools/               # Dynamic custom tools (*.json)
│   ├── hooks/               # Transition lifecycle scripts (.bat, .ps1)
│   └── spill/               # Spilled tool execution logs (>100 lines / >8KB)
├── .agents/
│   ├── skills/              # Specialized skills catalog
│   ├── tasks-history.json   # Machine-readable task ledger
│   └── knowledge-graph.json # Architectural MCP knowledge graph
├── src-tauri/               # Rust Desktop Core (Tauri v2)
│   ├── src/
│   │   ├── main.rs          # App entrypoint
│   │   ├── lib.rs           # Tauri builder & plugin wiring
│   │   ├── db.rs            # SQLite connection pooling & dialogue persistence
│   │   ├── config.rs        # Dynamic .cvh parser & atomic disk writes
│   │   ├── git.rs           # Native worktree & branch manager
│   │   ├── hooks.rs         # Transition scripts (.bat/.ps1) runner
│   │   ├── commands.rs      # Tauri IPC command layer
│   │   ├── agent.rs         # ReAct loop, zero-turn priming, tool executor
│   │   ├── agent/
│   │   │   └── compactor.rs # State-Preserving Context Compactor
│   │   └── llm/
│   │       ├── client.rs    # SSE client with deterministic prefix caching
│   │       └── models.rs    # OpenAI & Anthropic request/response schemas
│   ├── Cargo.toml
│   └── tauri.conf.json
└── src/                     # React 19 Frontend (TypeScript, Tailwind v4, Lucide)
    ├── components/
    │   ├── board/           # Dynamic Kanban Board & Columns
    │   ├── card/            # Tactile Kanban Card & Monospace Timers
    │   ├── cortex/          # Cortex Explorer («Проводник разума агентов»)
    │   ├── drawer/          # Card-as-a-Chat, Live Thoughts, Diff Viewer, ContextGauge
    │   ├── layout/          # Topbar & telemetry indicators
    │   └── settings/        # Tool Policy Matrix Modal
    ├── stores/              # Zustand reactive stores
    └── App.tsx
```

---

## ⚖️ License & Credits

Built with pride by the Voimax team. Designed for high-velocity autonomous engineering.
