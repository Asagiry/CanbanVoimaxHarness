# 🚀 CanbanVoimaxHarness (CVH)

> **Ultra-fast (~40MB RAM) Desktop Agentic IDE & Multi-Agent Orchestrator built with Rust, Tauri v2, and React 19.**  
> *Where the Kanban Board IS the Autonomous Control Plane — Bridging Single-Turn LLMs into Industrial-Grade Multi-Agent Software Factories.*

[![License MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square)](LICENSE)
[![Rust 2021](https://img.shields.io/badge/Rust-2021%20Edition-black?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tauri v2](https://img.shields.io/badge/Tauri-v2.0-24C8DB?style=flat-square&logo=tauri&logoColor=white)](https://v2.tauri.app/)
[![React 19](https://img.shields.io/badge/React-19.0-61DAFB?style=flat-square&logo=react&logoColor=black)](https://react.dev/)
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

---

## 🛡️ Zero-Waste Token Engineering & Security Matrix

### Two-Tier Zero-Trust Tool Policy Matrix
Security is enforced with ironclad **AND-semantics**:
```text
effective(tool, stage) = global_enabled[tool] && stage_override[stage][tool]
```
- **Level A (Global Switches)**: Defined in `.cvh/settings/tool-policy.json`.
- **Level B (Stage Overrides)**: Enforced per pipeline column (e.g. Code Review stage disables code-mutation tools).
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
| **Pipeline Navigation** | `advance_task` / `move_task` | Advances task to next stage with Relay Handoff summary. |
| | `reject_task` | Rejects task with audit critique, unlocking previous stage. |
| **Subagent Hierarchy** | `subagent` | Synchronous in-process child delegation returning structured report. |
| | `subagent_bg` | Asynchronous background delegation returning `subagent_id`. |
| | `collect_subagent` | Zero-polling collection of background subagent reports. |
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

## 🏛️ Project Directory Structure

```text
CanbanVoimaxHarness/
├── AGENTS.md                  # 2-Tier Multi-Agent Governance Manual & Invariants
├── README.md                  # Showcase Presentation & Architecture Guide
├── package.json               # Frontend dependencies (React 19, Tailwind v4, Vite)
├── pnpm-lock.yaml             # Deterministic pnpm dependency lockfile
├── vite.config.ts             # Vite bundling configuration
├── tsconfig.json              # TypeScript strict configuration
│
├── .cvh/                      # Project Harness Control Plane Configuration
│   ├── harness.db             # High-performance SQLite database (WAL mode)
│   ├── boards.json            # Multi-board definitions & active board
│   ├── pipeline.json          # Pipeline columns, routing, sensors, & hooks
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
├── src-tauri/                 # Rust Desktop Core (Tauri v2)
│   ├── src/
│   │   ├── main.rs            # Desktop entrypoint & crash handler
│   │   ├── lib.rs             # Tauri plugin & IPC handler registration
│   │   ├── db.rs              # SQLite connection pooling & WAL dialogue storage
│   │   ├── config.rs          # .cvh config parser & auto-gitignore policy
│   │   ├── git.rs             # Native worktree & branch manager
│   │   ├── hooks.rs           # Stage transition script runner (.bat/.ps1/.sh)
│   │   ├── agent.rs           # ReAct loop, zero-turn priming & tools engine
│   │   ├── agent/
│   │   │   └── compactor.rs   # State-Preserving Dynamic Context Compactor
│   │   ├── commands/          # Decoupled domain IPC submodules (tasks, boards, git, etc.)
│   │   └── llm/               # Universal SSE streaming client & schema models
│   ├── Cargo.toml             # Rust dependencies (Tauri, Tokio, Rusqlite, Serde)
│   └── tauri.conf.json        # Tauri v2 bundle and window configuration
│
└── src/                       # React 19 Tactical Frontend
    ├── components/
    │   ├── board/             # Kanban board, columns, and DND interactions
    │   ├── card/              # Tactile Kanban card & tabular stage timers
    │   ├── drawer/            # Card-as-a-Chat, live thinking, diff sidebar
    │   ├── layout/            # Topbar, status pills, and theme toggle
    │   └── settings/          # Tool policy matrix & model configuration modal
    ├── stores/                # Reactive Zustand state stores (board, drawer, llm)
    ├── types/                 # Shared TypeScript domain interfaces
    └── App.tsx                # Root layout, drag-and-drop context & overlays
```

---

## 📚 Architectural Documentation Index

- **[📘 docs/GUIDE.md](docs/GUIDE.md)**: Complete harness specification, IPC API catalog, and `.cvh` configuration manual.
- **[🤖 docs/agents.md](docs/agents.md)**: Agent ReAct loop mechanics, persona design, and tool policy enforcement.
- **[📋 docs/board.md](docs/board.md)**: Board Control Plane invariants, multi-board templates, and automatic verdict routing.
- **[⚡ docs/hooks.md](docs/hooks.md)**: Column transition scripts, gatekeeper sensors, and context environment variables.
- **[🏛️ docs/references/ecc-patterns.md](docs/references/ecc-patterns.md)**: Everything-Claw-Code (ECC) patterns, specialized roles, and Progressive Disclosure architecture.
- **[🚨 AGENTS.md](AGENTS.md)**: Developer & Agent Engineering Manual, invariants, and code hygiene standards.

---

## ⚖️ License & Authorship

CanbanVoimaxHarness is free and open-source software licensed under the **[MIT License](LICENSE)**.  
Engineered with architectural discipline by **Voimax** for high-velocity, deterministic multi-agent software engineering.
