# 🚀 CanbanVoimaxHarness (CVH)

> **Ultra-fast (~40MB RAM) Desktop Agentic IDE & Multi-Agent Orchestrator built with Rust, Tauri v2, and React 19.**  
> *Where the Kanban Board IS the Autonomous Control Plane — Bridging Single-Turn LLMs into Industrial-Grade Multi-Agent Software Factories.*

[![License MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square)](LICENSE)
[![Rust 2021](https://img.shields.io/badge/Rust-2021%20Edition-black?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tauri v2](https://img.shields.io/badge/Tauri-v2.0-24C8DB?style=flat-square&logo=tauri&logoColor=white)](https://v2.tauri.app/)
[![React 19](https://img.shields.io/badge/React-19.0-61DAFB?style=flat-square&logo=react&logoColor=black)](https://react.dev/)
[![Clean Architecture](https://img.shields.io/badge/Clean%20Architecture-9.7%2F10-blueviolet?style=flat-square)](#6-clean-architecture-9710--type-safe-domain-layer)
[![NASA Test Shield](https://img.shields.io/badge/NASA%20Test%20Shield-383%20PASS%20(100%25)-brightgreen?style=flat-square)](#-nasa-ground-truth-verification-shield-383-automated-tests)
[![SQLite WAL](https://img.shields.io/badge/SQLite-WAL%20Mode-003B57?style=flat-square&logo=sqlite&logoColor=white)](https://sqlite.org/)
[![RAM Footprint](https://img.shields.io/badge/RAM_Footprint-~40MB-emerald?style=flat-square)](#-key-architectural-highlights)
[![Zero-Amnesia Memory](https://img.shields.io/badge/Memory-Zero--Amnesia%20Relay-orange?style=flat-square)](#2-relay-handoff-memory-zero-amnesia-priming)
[![OpenAI Codex for OSS](https://img.shields.io/badge/OpenAI%20Codex-OSS%20Showcase-purple?style=flat-square&logo=openai&logoColor=white)](https://openai.com/)

---

## 🧭 The Paradigm Shift: Why Traditional Chat Interfaces Fail Software Engineering

Modern AI coding assistants (Claude Code, OpenAI Codex CLI, Cursor, Copilot) are marvels of synthesis, yet developers are universally trapped in a legacy 1960s terminal interaction pattern: **the linear conversational chat**. 

In real-world development, linear chat threads inevitably collapse under their own weight:
- **Chat Scroll Fatigue**: Multi-step architectures, checklists, and test outputs are quickly drowned in endless chat streams.
- **Context Amnesia**: Dialogue transcripts balloon, pushing earlier system invariants and interface constraints out of the attention window.
- **Dirty Tree Mutations**: Agents edit code directly in your active working tree, colliding with unstaged edits and breaking builds mid-stride.
- **The "Relay Clerk" Anti-Pattern**: The human engineer becomes an exhausted middleman, copying terminal logs back into chat, begging the model not to touch unrelated files, and cherry-picking broken patches.

**CanbanVoimaxHarness (CVH)** replaces this fragile conversational loop with an **immutable, stateful, visual multi-agent control plane**.

---

## ⚖️ Engineering Contrast: Why CanbanVoimaxHarness?

| Dimension | Without CVH (Traditional Chat & CLI) | With CanbanVoimaxHarness (CVH Control Plane) |
| :--- | :--- | :--- |
| **Task Planning** | Plans vanish into endless chat scrolls | Plans become tactile Kanban task cards & live `todo_write` checklists |
| **Code Verification** | Same context writes and reviews its own code | 2-Tier Governance: Lead Developer -> Clean-Context Reviewer |
| **Long-Term Memory** | Context amnesia: massive transcripts degrade | Zero-Turn Memory Priming + Relay Handoff + State-Preserving Compaction |
| **Filesystem Safety** | Code mutations happen in dirty main worktree | Isolated native `git worktree` per task with automated teardown |
| **Quality Gates** | Quality checks rely on LLM reminders | Ground Truth Sensors: native compiler gates (`cargo check`, `tsc`) |
| **Resource Footprint** | Bloated Electron apps (600MB+ RAM) | Ultra-lightweight Rust desktop core (~40MB RAM, instant launch) |

---

## 🏛️ System Architecture

CanbanVoimaxHarness decouples desktop operating system primitives, high-throughput asynchronous LLM streaming, and reactive tactile state into clean, unidirectional layers:

```text
 +--------------------------------------------------------------------------------------------------+
 |                                  CANBAN VOIMAX HARNESS (CVH)                                     |
 |                          Tactile Desktop Agentic IDE & Control Plane                             |
 +--------------------------------------------------------------------------------------------------+
                                                   │
                                                   ▼
 +--------------------------------------------------------------------------------------------------+
 |                                  REACT 19 TACTICAL KANBAN UI                                     |
 |  * Tactile Drag-and-Drop Board             * Live <thinking> & Activity Streams                  |
 |  * Real-Time Diff Viewer & Steer Bar       * Monospace Tabular Stage Timers                      |
 +-------------------------------------------------+------------------------------------------------+
                                                   │
                                                   │ Tauri v2 IPC Router (Zero-Copy Serialization)
                                                   ▼
 +--------------------------------------------------------------------------------------------------+
 |                                  RUST 2021 TOKIO ASYNC CORE                                      |
 |                                                                                                  |
 |  +-------------------------------------+      +-----------------------------------------------+  |
 |  |      TOKIO ASYNC REACT LOOP         |      |          TWO-TIER TOOL POLICY MATRIX          |  |
 |  |  * Non-blocking SSE Event Streaming |      |  * Strict AND-Semantics (Global && Stage)     |  |
 |  |  * Subagent Delegation Tree         |      |  * Zero-Trust Request Schema Stripping        |  |
 |  |  * Deterministic Prefix Caching     |      |    (-2,400+ prompt tokens per step)           |  |
 |  +------------------+------------------+      +-----------------------+-----------------------+  |
 |                     │                                                 │                          |
 +---------------------┼─────────────────────────────────────────────────┼────────────────----------+
                       │                                                 │
        ┌──────────────┴───────────────┐                  ┌──────────────┴───────────────┐
        ▼                              ▼                  ▼                              ▼
+-----------------------+   +----------------------+   +-------------------+   +--------------------+
|  SQLITE WAL DATABASE  |   | ISOLATED WORKTREES   |   | COMPILER SENSORS  |   | LOCAL & CLOUD LLMS |
|  * .cvh/harness.db    |   | * worktrees/<task>   |   | * cargo check     |   | * OpenAI Codex     |
|  * ACID dialogue logs |   | * feat/<task> branch |   | * cargo clippy    |   | * Antigravity Proxy|
|  * Sub-ms upserts     |   | * Auto merge & prune |   | * tsc --noEmit    |   | * Anthropic Claude |
+-----------------------+   +----------------------+   +-------------------+   +--------------------+
```

---

## ⚡ Key Architectural Highlights

### 1. 🎯 Kanban-As-Control-Plane (Columns = Agents Invariant)
In CVH, columns on the Kanban board are not passive status labels; **each column represents a specialized, autonomous agent role**.
- When a column configures `persona: "<role>"`, moving a task card into that column automatically initializes the agent loop with that persona's system prompt (`.cvh/agents/<role>/persona.md`), scoped rules (`.cvh/agents/<role>/rules/*.md`), and security permissions.
- Entering an active stage emits `agent:autorun` and executes the agent loop immediately.
- Cards feature interactive stage steppers with one-click transitions, active execution pulses, and passed stage checkmarks.

### 2. 🏃 Relay Handoff Memory (Zero-Amnesia Priming)
Traditional pipelines suffer severe context decay when passing tasks across stages. CVH eliminates context loss through **Relay Handoff Memory Priming**:
- When an agent completes its work and calls `advance_task(summary)` or `reject_task(reason)`, the structured handover findings are recorded in `task.handover_summary`.
- The receiving agent in the subsequent stage receives this context pre-compiled into a top-level prompt banner:
  ```markdown
  ### [ЭСТАФЕТА ПАЙПЛАЙНА / RELAY HANDOFF]
  Transitioned from: <previous_stage>
  Handoff Summary: <structured_findings_and_test_verdicts>
  ```
- The agent starts Turn 1 with **100% memory primed**—zero turns wasted re-reading history or rule files.

### 3. 🗄️ High-Performance SQLite Persistence in WAL Mode (`.cvh/harness.db`)
Every thought chunk, streaming token, tool call, and duration metric is persisted in an ACID-compliant SQLite engine located at `<repo_root>/.cvh/harness.db`:
- Configured with Write-Ahead Logging (`PRAGMA journal_mode = WAL`) and synchronous writes (`PRAGMA synchronous = NORMAL`).
- Monotonically indexed schema `dialog_items(task_id, created_at)` enables sub-millisecond atomic transactions.
- Zero file corruption, zero lock contention, and instant dialogue restoration on startup or card inspection.

### 4. 🔀 Multi-Board Topologies & Dynamic Routing
CVH adapts to project scope through first-class multi-board topologies defined in `.cvh/boards.json`:
- **Lean Agile Kanban (3 Stages)**: Quick fixes and rapid prototyping (`backlog` → `dev` → `ready`).
- **Canonical Engineering Pipeline (5 Stages)**: Mission-critical development (`backlog` → `dev` → `review` → `retro` → `ready`).
- **Domain Pipelines**: Specialized boards for 3D modeling, asset creation, or security auditing.
- Automatic routing parses structured verdicts (`APPROVED`, `APPROVED_WITH_CAVEATS`, `CHANGES_REQUESTED`), advancing verified tasks or returning rejected cards to development with injected audit feedback.

### 5. 📊 Live Telemetry & Honest Diagnostics
CVH delivers deep observability into agent execution velocity and token economics:
- **Active Generation TPS (Tokens Per Second)**: Measured strictly during active token generation, completely decoupled from pre-generation network setup or Time-To-First-Token (TTFT).
- **Non-Decaying TPS Invariant**: Generation speed freezes instantaneously upon turn completion—metrics never "melt" towards zero.
- **Safe Prompt Cache Hit Rate**: Division-by-zero guarded calculation tracking provider prompt cache efficiency (**consistently >90%** via deterministic alphabetical tool serialization).
- **RPM (Cadence Rate)**: Real-time tracking of agent steps-per-minute with monospace numerals (`tabular-nums`).
- **Dynamic Context Gauge**: Multi-segment visual ring breakdown (System Prompt, Tools, Dialogue Messages) with a strict **100% Compaction Ceiling**.

### 6. 🏛️ Clean Architecture 9.7/10 & Type-Safe Domain Layer
Engineered to strict Clean Architecture and Single-Responsibility principles (Uncle Bob `ub-mode`):
- **Type-Safe Nominal Branded Types**: `TaskId`, `StageId`, `BoardId`, and `TaskState` prevent primitive obsession across Rust core (`src-tauri/src/domain/`) and React frontend (`src/types/domain.ts`).
- **R2D2 SQLite WAL Connection Pooling**: Multi-threaded connection pooling with WAL mode and specialized repository pattern (`pool.rs`, `task_repo.rs`, `dialog_repo.rs`, `telemetry_repo.rs`), eliminating database lock contention.
- **Decoupled ReActEngine<Driver>**: Generic agent execution engine (`react_engine.rs`) parameterized by `AgentSessionDriver`, sharing SSE streaming and telemetry logic between root tasks and child subagents with zero duplication.
- **Type-Safe Tauri Event Gateway**: All IPC event communication routes through `tauriEventGateway.ts`, eliminating unverified `listen<any>` calls.
- **Modular Frontend Slices**: Monolithic stores and components decomposed into single-responsibility slices (`stores/board/`, `stores/drawer/`, `stores/cortex/`, `KanbanCard.tsx` reduced to 239 lines).

---

## 🛡️ Zero-Waste Token Engineering & Security Matrix

### Two-Tier Zero-Trust Tool Policy Matrix
Security is enforced with ironclad **AND-semantics**:
```text
effective(tool, stage) = global_enabled[tool] && stage_override[stage][tool]
```
- **Level A (Global Switches)**: Defined in `.cvh/settings/tool-policy.json`.
- **Level B (Stage Overrides)**: Enforced per pipeline column (e.g. Code Review stage disables code-mutation tools).
- **Direct JSON Schema Editor (`toolCustomSchemas`)**: The Global Registry in `.cvh/settings/tool-policy.json` supports direct customization of tool schemas (`toolCustomSchemas`). Architects and developers can tailor parameter descriptions, add domain-specific constraints, or specialize instructions without modifying core Rust engine code.
- **English-Only Schemas & BPE Token Compression**: All tool names, descriptions, and parameter definitions sent to LLMs are authoritatively maintained in English. Byte-Pair Encoding (BPE) tokenizers (used across Claude, Gemini, DeepSeek, and OpenAI) process English tokens with **~70% denser compression** than Cyrillic UTF-8 multi-byte sequences, saving thousands of prompt tokens per step and driving prompt cache hit rates above 90%.
- **Zero-Trust Request Schema Stripping**: Disabled tools are completely purged from the JSON tool schema sent to the LLM. This saves **~2,400+ prompt tokens per inference step** and completely eliminates tool hallucination vectors.

### State-Preserving Dynamic Context Compactor
Long-running sessions naturally accumulate stale terminal logs and bulky file dumps:
- **Zero-LLM Fast Evaporation**: Tool outputs older than the last 2 turns exceeding 500 characters are automatically evaporated into concise head/tail summaries without spending a single API token.
- Reclaims up to **80% of token context** while preserving critical architectural decisions.

---

## 🛠️ Industrial Tools Catalog

CVH equips agents with an industrial-grade tool suite compliant with SWE-bench standards:

| Category | Tool | Description |
| :--- | :--- | :--- |
| **Pipeline Navigation** | `advance_task` / `move_task` | Advances task to next stage with Relay Handoff summary (self-healing with available stages list on error). |
| | `reject_task` | Rejects task with audit critique, unlocking previous stage. |
| **Subagent Hierarchy** | `subagent` | Synchronous in-process child delegation returning structured report. |
| | `subagent_bg` | Asynchronous background delegation returning `subagent_id`. |
| | `collect_subagent` | Reliable zero-polling collector with 3-tier report resolution (⚡ СИНХР / 🗂️ ФОН). |
| **Knowledge & Dynamic Rules** | `load_rule` | JIT on-demand loading of modular architectural standards and guidelines by topic. |
| **Task Registry & Chronology** | `list_tasks` | Compact registry inspection of Kanban board tasks filtered by stage or query from SQLite WAL. |
| | `get_task_history` | Step-by-step task execution chronology (requirements, tool calls, exit codes, verdicts) for audits. |
| **Human-in-the-Loop** | `question` / `ask_user` | Interactive query prompt with option choices, suspending execution. |
| | `todo_write` | Live tactical checklist (`todos: [{content, status}]`). |
| **Filesystem & Shell** | `read_file` | UTF-8 file inspection with line offsets and limits. |
| | `write_file` | Full UTF-8 file creation and replacement. |
| | `edit_file` (`str_replace`) | 5-tier progressive matching with whitespace-relaxed fallback. |
| | `glob` | Rapid pattern-based file discovery in active worktree. |
| | `grep` | High-speed regex text search across worktree files. |
| | `execute_command` (`bash`) | Shell execution with timeout enforcement and stdio capture. |
| **Research & Web** | `web_search` | DuckDuckGo search integration with zero API key requirement. |
| | `web_fetch` | Clean Markdown webpage content extraction. |
| **Dynamic CLI Tools** | `.cvh/tools/*.json` | Declarative custom CLI tools with `{{args}}` interpolation. |

### ⚡ Reliable Subagent Collector (`collect_subagent`)
Background child agents (`subagent_bg`) execute independently without blocking the parent ReAct loop. The `collect_subagent` tool retrieves subagent findings with ironclad reliability:
- **3-Tier Cascade Resolution**: Resolves reports across (1) active Tokio async broadcast channels, (2) in-memory completed session cache, and (3) persistent SQLite `dialog_items` in `.cvh/harness.db`. Collecting completed or synchronous subagents never fails with missing registry errors.
- **Distinct Visual Execution Badges**: The UI and drawer streams clearly distinguish synchronous oneshot subagents (**⚡ СИНХР**) from asynchronous background delegations (**🗂️ ФОН**).
- **Spilling Bypass**: Subagent terminal reports are exempt from tool output truncation or disk spilling, guaranteeing 100% of synthesized findings reach parent models without diagnostic data loss.

*Note: Large outputs exceeding 500 lines or 64 KB for shell commands (`execute_command`), or 2,000 lines / 128 KB for `read_file`, automatically spill to disk at `.cvh/spill/<timestamp>-<tool>.log`, returning only the head (20 lines) and tail (20 lines) to preserve LLM context.*

---

## 🚀 Quick Start Guide

Get up and running with CanbanVoimaxHarness in seconds:

### Prerequisites
- **Node.js**: v20+ & **pnpm**: v10+
- **Rust Toolchain**: Rust 2021 Edition (`rustc` / `cargo` 1.80+)
- **Git**: 2.30+ (with native worktree support)

### Option A: From Source (3 Commands)
```bash
# 1. Clone repository
git clone https://github.com/Asagiry/CanbanVoimaxHarness.git
cd CanbanVoimaxHarness

# 2. Install dependencies
pnpm install

# 3. Launch Desktop Agentic IDE
pnpm tauri dev
```

### Option B: Pre-Compiled Desktop Installer
Download the latest verified Windows release installer directly:
- **[Download Latest Windows Installer (Release/Windows/installer)](Release/Windows/installer)**
- Instant launch, native OS integration, pre-bundled Rust core (~40MB RAM).

### Option C: Web Development Sandbox
Test and develop the React 19 UI with instantaneous Vite Hot Module Replacement (HMR):
```bash
pnpm run dev
# Open http://localhost:5173
```

---

## 🛡️ NASA Ground Truth Verification Shield (383 Automated Tests, 100% PASS)

CanbanVoimaxHarness enforces zero-regression engineering through an unyielding 383-test automated verification shield:

- **Frontend Test Shield (Vitest v5.0.5)**: **139 automated tests** across 8 test suites running in ~2.7s with 100% PASS (0 failures, 0 flaky):
  - Branded nominal types & domain invariants (`src/types/domain.test.ts`)
  - Board store reactive slices, transitions, and auto-unlock (`src/stores/board/boardStore.test.ts`)
  - Drawer streaming slices, thought parsers, and telemetry aggregation (`src/stores/drawer/drawerStore.test.ts`)
  - Cortex persona and rules management slices (`src/stores/cortex/cortexStore.test.ts`)
  - Type-Safe Tauri Event Gateway payload verification (`src/services/tauriEventGateway.test.ts`)
  - Markdown AST extraction, token counters, and code blocks (`src/lib/markdown/parser.test.ts`)
  - KanbanCard memoization, comparators, and tactile badges (`src/components/card/KanbanCard.test.tsx`)
  - Localization dictionary completeness and fallbacks (`src/i18n/i18n.test.ts`)
- **Backend Test Shield (Cargo Test)**: **244 automated tests** across all subsystems running in ~4.8s with 100% PASS (0 failures, 0 flaky):
  - Domain nominal branded IDs & state machine (`domain/ids.rs`, `domain/state.rs`)
  - R2D2 SQLite connection pooling, migrations, and repositories (`db/pool.rs`, `db/task_repo.rs`, `db/dialog_repo.rs`, `db/telemetry_repo.rs`)
  - Config discovery, models, tool policy, and auto-gitignore scaffold (`config/`)
  - Generic ReAct Engine, stream chunk consumer, compactor, and fuzzy text match (`agent/`)
  - Task transition pipelines, healing, and lifecycle commands (`commands/tasks/`)
- **NASA Ground Truth Turbo Sensors (<10s)**:
  ```bash
  # Execute full parallel ground-truth sensors:
  pnpm exec tsc --noEmit && pnpm exec oxlint --deny-warnings && cargo check
  
  # Run complete 383-test shield:
  pnpm test      # 139 Vitest tests (~2.7s)
  cargo test     # 244 Rust core tests (~4.8s)
  ```

---

## 🏛️ Project Directory Structure (9.7/10 Clean Code Modular Architecture)

```text
CanbanVoimaxHarness/
├── README.md                  # Showcase Presentation & Architecture Guide
├── package.json               # Frontend dependencies (React 19, Tailwind v4, Vite, Zustand)
├── pnpm-lock.yaml             # Deterministic pnpm dependency lockfile
├── vite.config.ts             # Vite bundling configuration
├── tsconfig.json              # TypeScript strict configuration
│
├── .cvh/                      # Project Harness Control Plane Configuration
│   ├── harness.db             # High-performance SQLite database (R2D2 WAL mode)
│   ├── boards.json            # Multi-board definitions & active board
│   ├── pipeline.json          # Pipeline columns, routing, sensors, & hooks
│   ├── rules/                 # Global architectural rules & navigation maps
│   ├── agents/                # Modular persona definitions (<role>/persona.md, setting, rules/)
│   ├── settings/              # Tool Policy Matrix (tool-policy.json) & sequence allocators
│   ├── tools/                 # Dynamic custom CLI tools (<tool>.json)
│   ├── hooks/                 # Transition scripts (.bat, .ps1, .sh)
│   └── spill/                 # Spilled tool output logs (>500 lines / >64KB)
│
├── .agents/                   # Agent Skills Catalog & Task History
│   ├── skills/                # Modular skills catalog
│   ├── tasks-history.json     # Machine-readable task ledger
│   └── knowledge-graph.json   # Architectural MCP knowledge graph
│
├── docs/                      # Authoritative Technical Manuals
│   ├── GUIDE.md               # Complete harness configuration & IPC manual
│   ├── agents.md              # ReAct loop, Personas, and Zero-Turn Priming
│   ├── board.md               # Board Control Plane, Invariants & Multi-Boards
│   ├── hooks.md               # Stage transition lifecycle scripts & hooks
│   └── references/            # Architectural References & Blueprints
│       └── ecc-patterns.md    # Everything-Claw-Code Patterns & Progressive Disclosure
│
├── src-tauri/                 # Rust Desktop Core (Tauri v2 Clean Modular Architecture)
│   ├── src/
│   │   ├── main.rs            # Desktop entrypoint & crash handler
│   │   ├── lib.rs             # Tauri plugin & IPC handler registration
│   │   ├── domain/            # Nominal Branded Types & Core Business Invariants
│   │   │   ├── ids.rs         # TaskId, StageId, BoardId branded types
│   │   │   └── state.rs       # TaskState state-machine & valid transitions
│   │   ├── db/                # R2D2 SQLite Persistence Layer (.cvh/harness.db)
│   │   │   ├── pool.rs        # R2D2 pool factory (WAL mode, busy_timeout=5s)
│   │   │   ├── task_repo.rs   # Task CRUD & query repository
│   │   │   ├── dialog_repo.rs # Dialogue upsert & streaming chunks repository
│   │   │   └── telemetry_repo.rs # Token usage & performance metrics repository
│   │   ├── config/            # Harness Configuration & Scaffold Subsystem
│   │   │   ├── models.rs      # Data structures (BoardDef, PipelineConfig, etc.)
│   │   │   ├── tool_policy.rs # Two-level tool policy matrix enforcement
│   │   │   ├── discovery.rs   # Multi-tier workspace root discovery
│   │   │   ├── scaffold.rs    # Fresh-project scaffolding & auto-gitignore
│   │   │   └── io.rs          # Atomic disk reads/writes
│   │   ├── git.rs             # Native worktree & branch manager
│   │   ├── hooks.rs           # Stage transition script runner (.bat/.ps1/.sh)
│   │   ├── agent.rs           # Agent lifecycle, priming & custom tool executor
│   │   ├── agent/             # Modular Agent Execution Subsystems
│   │   │   ├── react_engine.rs  # Generic ReActEngine<Driver> loop & stream consumer
│   │   │   ├── compactor.rs     # Typed ToolCallSummaryExtractor (CC=1)
│   │   │   ├── sanitizer.rs     # Unclosed tool call sanitization
│   │   │   ├── telemetry.rs     # Non-decaying TPS/RPM telemetry
│   │   │   ├── text_match.rs    # Resilient 5-tier fuzzy edit matching
│   │   │   ├── tool_schemas.rs  # LLM tool definitions & JSON schemas
│   │   │   └── tools/           # Modular tool execution implementations
│   │   ├── commands/          # Tauri IPC Command Handlers
│   │   │   ├── tasks.rs       # Task command routing
│   │   │   ├── tasks/         # Modular task pipelines (transitions, lifecycle, healing)
│   │   │   ├── boards.rs      # Board management commands
│   │   │   ├── dialogs.rs     # Dialogue history & streaming commands
│   │   │   ├── cortex.rs      # Persona & rules editor commands
│   │   │   ├── git.rs         # Worktree & diff commands
│   │   │   └── llm.rs         # LLM configuration commands
│   │   └── llm/               # Universal SSE streaming client & model registry
│   ├── Cargo.toml             # Rust dependencies (Tauri v2, Tokio, Rusqlite, R2D2)
│   └── tauri.conf.json        # Tauri v2 bundle and window configuration
│
└── src/                       # React 19 Tactical Frontend (Modular Architecture)
    ├── components/
    │   ├── board/             # Kanban board, BoardTabsHeader & column interactions
    │   ├── card/              # Modular KanbanCard (239 lines), badges & comparators
    │   ├── drawer/            # AgentConsoleDrawer, diff/ (ImageDiffViewer), stream/
    │   ├── cortex/            # Cortex Explorer, Persona & Rule visual editors
    │   └── settings/          # Pipeline, tool policy & LLM config modals
    ├── stores/                # Zustand reactive stores decomposed into modular slices
    │   ├── board/             # Board store slices, normalizers & helpers
    │   ├── drawer/            # Drawer store slices (stream, activity, subagents)
    │   └── cortex/            # Cortex store slices (personas, rules, files)
    ├── services/              # Pure decoupled services
    │   └── tauriEventGateway.ts # Strongly typed IPC event gateway (zero listen<any>)
    ├── types/
    │   └── domain.ts          # Nominal branded types (TaskId, StageId, BoardId)
    ├── lib/
    │   └── markdown/          # Extracted markdown AST parser & token helpers
    ├── i18n/                  # Modular localization (ru.ts, en.ts, types.ts)
    └── App.tsx                # Root layout, drag-and-drop context & overlays
```

---

## 📚 Architectural Documentation Index

- **[📘 docs/GUIDE.md](docs/GUIDE.md)**: Complete harness specification, IPC API catalog, and `.cvh` configuration manual.
- **[🤖 docs/agents.md](docs/agents.md)**: Agent ReAct loop mechanics, persona design, and tool policy enforcement.
- **[📋 docs/board.md](docs/board.md)**: Board Control Plane invariants, multi-board templates, and automatic verdict routing.
- **[⚡ docs/hooks.md](docs/hooks.md)**: Column transition scripts, gatekeeper sensors, and context environment variables.
- **[🏛️ docs/references/ecc-patterns.md](docs/references/ecc-patterns.md)**: Everything-Claw-Code (ECC) patterns, specialized roles, and Progressive Disclosure architecture.
- **[🧠 .cvh/agents/ & .cvh/rules/](.cvh/)**: Modular persona definitions, role-specific rules, and global architectural invariants.

---

## ⚖️ License & Authorship

CanbanVoimaxHarness is free and open-source software licensed under the **[MIT License](LICENSE)**.  
Engineered with architectural discipline by **Voimax** for high-velocity, deterministic multi-agent software engineering.
