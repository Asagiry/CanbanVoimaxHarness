# 🚀 CanbanVoimaxHarness (CVH)

> **Next-Generation Ultra-Lightweight Desktop Agentic IDE & Autonomous Control Plane**  
> *Where the Kanban Board IS the Control Plane — Built for Mission-Critical Engineering at Scale.*

[![Rust 2021](https://img.shields.io/badge/Rust-2021%20Edition-black?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tauri v2](https://img.shields.io/badge/Tauri-v2.0-24C8DB?style=for-the-badge&logo=tauri&logoColor=white)](https://v2.tauri.app/)
[![React 19](https://img.shields.io/badge/React-19.0-61DAFB?style=for-the-badge&logo=react&logoColor=black)](https://react.dev/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.7-3178C6?style=for-the-badge&logo=typescript&logoColor=white)](https://www.typescriptlang.org/)
[![Tailwind CSS v4](https://img.shields.io/badge/Tailwind_CSS-v4.0-38B2AC?style=for-the-badge&logo=tailwind-css&logoColor=white)](https://tailwindcss.com/)
[![SQLite WAL](https://img.shields.io/badge/SQLite-WAL%20Mode-003B57?style=for-the-badge&logo=sqlite&logoColor=white)](https://sqlite.org/)
[![Memory Footprint](https://img.shields.io/badge/RAM_Footprint-~40MB-emerald?style=for-the-badge)](#technical-highlights)
[![License MIT](https://img.shields.io/badge/License-MIT-yellow?style=for-the-badge)](LICENSE)

---

## 🧭 The Manifesto: Why Chat-Based AI is Obsolete

Modern AI coding assistants have hit an architectural wall.

Whether working with **Claude Code**, **OpenAI Codex**, **Cursor**, **Antigravity**, or **DeepSeek Harness (DSH)**, developers are universally shoehorned into a legacy 1960s terminal interaction model: **the linear conversational chat**. We have traded writing code for an exhausting loop of prompt babysitting, tab sprawl, git status anxiety, and silent file overwrites.

**CanbanVoimaxHarness (CVH)** is engineered from direct operational experience with these bleeding-edge tools to establish a new fundamental truth:

> **Software engineering is a multi-dimensional, stateful, multi-agent pipeline. It cannot survive inside a 1D conversational chat thread.**

---

### 💥 1. The Collapse of the Linear Chat Paradigm

Traditional chat-driven assistants suffer from fatal structural flaws that compound as project complexity scales:

| Failure Mode | Conventional Chat Assistants (Claude, Codex, Cursor, DSH) | Real-World Engineering Reality |
| :--- | :--- | :--- |
| **The Linear Chat Trap** | Forcing all interactions into a single sequential timeline | Developers drown in dozens of disconnected chat tabs and ephemeral threads, losing track of what was tested or changed. |
| **Context Degradation & Hallucination Drift** | Unbounded token sprawl; blunt truncation or lossy auto-summarization | As dialogue history expands, critical architectural invariants are squeezed out. Models hallucinate outdated APIs and forget core constraints. |
| **Working Tree Anarchy** | Uncontrolled, in-place file modifications directly on your active working directory | AI edits clobber unstaged work, introduce breaking changes mid-stride, and pollute Git history with dirty unstaged diffs. |
| **The "Tired Chat Operator" Syndrome** | Human engineer forced to manually copy errors, logs, and files between tools | You spend your day as an exhausted relay clerk: copying compiler errors back into chat, begging the model not to touch unrelated files, and cherry-picking broken patches. |
| **Resource & Electron Bloat** | Bulky Chromium/Electron runtimes consuming 1.5 GB to 3.5 GB of RAM | Desktop performance grinds to a halt while running heavy IDE wrappers alongside local LLM proxies and native language servers. |

#### 🥀 The "Tired Chat Operator" Anti-Pattern
In conventional tools, the human engineer is forced to act as the fragile glue between disjointed systems:
1. You describe a feature in a chat window.
2. The agent outputs code that partially touches multiple files, clobbering your current uncommitted changes.
3. The build breaks. You copy the terminal error and paste it back into the chat.
4. The model "apologizes" and writes a half-baked fix that breaks 3 other unit tests.
5. Context limits approach: the model begins hallucinating nonexistent APIs, forgetting earlier constraints.
6. In desperation, you open a new chat tab, re-explain the entire repository architecture from scratch, and repeat the cycle.

This is not autonomy. **This is glorified copy-paste with cognitive burnout.**

---

### ⚡ 2. The Solution: The Kanban Board IS the Autonomous Control Plane

**CanbanVoimaxHarness** replaces the fragile conversational thread with an immutable, deterministic engineering state machine. 

In CVH, you dispatch work once onto a tactile Kanban board. **The board orchestrates the agents; the agents execute across isolated worktrees; the machine enforces ground truth.**

```text
               +-----------------------------------------------------------------------+
               |                  THE KANBAN AUTONOMOUS CONTROL PLANE                  |
               +-----------------------------------------------------------------------+
                                                  │
                 ┌────────────────────────────────┴────────────────────────────────┐
                 ▼                                                                 ▼
   +---------------------------+                                     +---------------------------+
   |   COLUMNS = AGENT ROLES   |                                     |    NATIVE GIT WORKTREES   |
   | Each column is a bound    |                                     | Every task executes in its|
   | autonomous agent persona  |                                     | own isolated directory    |
   | with strict tool policies |                                     | and dedicated branch      |
   +-------------┬-------------+                                     +-------------┬-------------+
                 │                                                                 │
                 └────────────────────────────────┬────────────────────────────────┘
                                                  ▼
                               +-------------------------------------+
                               |     LIFECYCLE HOOKS & SENSORS       |
                               | Transitions execute .bat/.ps1/.sh   |
                               | Gatekeeper sensors (cargo / npm)    |
                               | enforce zero-regression invariants  |
                               +------------------┬------------------+
                                                  │
                 ┌────────────────────────────────┴────────────────────────────────┐
                 ▼                                                                 ▼
   +---------------------------+                                     +---------------------------+
   |  STRICT 2-TIER TOOL MATRIX|                                     | ULTRA-LEAN RUST CORE      |
   | Strict AND-semantics with |                                     | Rust 2021 + Tauri v2      |
   | dynamic schema stripping  |                                     | ~40MB RAM footprint       |
   | (-2,400+ prompt tokens)   |                                     | Zero Electron bloat       |
   +---------------------------+                                     +---------------------------+
```

#### 🏛️ Core Philosophical Invariants of CVH:

1. **Dispatched Once, Governed Autonomously**: You define a task once on a board card. CVH provisions the environment, delegates implementation to specialized agent personas, runs compiler checks, requests adversarial code reviews, and executes merges upon verification—all without babysitting.
2. **Columns ARE Specialized Agent Personas (Columns = Agents)**: Columns are not passive status labels or static buckets. In CVH, each column represents a specialized, autonomous persona (`cvh-dev`, `architect`, `reviewer`, `verifier`). Moving a card into a stage primes that persona, mounts its unique rules, and activates its ReAct loop automatically.
3. **Deterministic Native Git Worktree Isolation**: Every single task card provisions an isolated native Git worktree at `worktrees/<task-slug>` on a dedicated feature branch (`feat/<task-slug>`). Multiple agents can code, compile, and run tests concurrently with zero risk of dirty working trees, broken uncommitted code, or merge conflicts.
4. **Lifecycle Hooks & Strict Gatekeeper Sensors**: Moving cards between columns triggers customizable lifecycle scripts (`.bat`, `.cmd`, `.ps1`, `.sh`) injected with full runtime context (`CVH_TASK_ID`, `CVH_WORKTREE_PATH`, `CVH_BRANCH`, `CVH_STAGE_ID`). Ground Truth Gatekeeper Sensors (`cargo check`, `cargo clippy`, `pnpm run build`, `npm test`) validate physical correctness before a transition is permitted. If compilation fails, the transition is blocked deterministically.
5. **Two-Tier Zero-Trust Tool Policy Matrix & Dynamic Schema Stripping**: Tool security is governed by strict `AND`-semantics: `effective = global_enabled && stage_override`. Disabled tools are completely purged from the JSON schema payload sent to the LLM—guaranteeing zero tool hallucinations and saving **2,400+ prompt tokens per inference step**.
6. **Zero-Overhead Custom CLI Tools via Declarative JSON**: Extend agent capabilities on the fly. Drop a JSON descriptor into `.cvh/tools/<tool_name>.json` with parameterized `{{args}}` interpolation, and the harness immediately exposes custom internal binaries, scripts, or linters to the agent loop.
7. **Ultra-Lightweight Native Core (Rust 2021 + Tauri v2)**: Built for developers who demand instant responsiveness. CVH runs on a native asynchronous Tokio engine in Rust paired with a Tauri v2 shell, consuming a razor-thin **~40MB RAM** instead of the 2GB+ memory footprint typical of Electron-based apps.

---

## 🎯 Executive Overview & Value Proposition

**CanbanVoimaxHarness (CVH)** bridges the gap between chaotic single-prompt chat windows and enterprise-grade multi-agent software engineering. 

- 🏎️ **Ultra-Lightweight Footprint (~40MB RAM)**: Native **Rust 2021 / Tauri v2** asynchronous core and a hyper-responsive **React 19** frontend. Instantaneous startup (<200ms) with zero Electron ballast.
- 🌲 **Deterministic Worktree Isolation**: Every task executes strictly inside its own isolated Git worktree (`worktrees/<task-slug>`) on a dedicated feature branch. Concurrent agents never touch or pollute your active working tree or main branch.
- 🗄️ **High-Performance SQLite Persistence in WAL Mode**: Every token, thought block, tool invocation, and duration metric streams directly into an ACID-compliant SQLite engine (`.cvh/harness.db`), guaranteeing atomic persistence with zero lock contention.
- 🛡️ **Two-Tier Zero-Trust Tool Policy Matrix**: Ironclad `AND`-semantics ensure tools disabled globally can never be enabled by stages, while disabled tool schemas are completely purged from LLM requests—eliminating hallucinations and saving **2,400+ prompt tokens per step**.
- 🗜️ **State-Preserving Context Compactor & Prefix Caching**: Automated log evaporation reclaims up to **80% of token context** without calling external LLM APIs, while deterministic alphabetical tool serialization preserves provider KV cache hit rates **above 90%**.
- 🏃 **Relay Handoff Memory Priming**: As tasks advance across pipeline stages, structured handoff summaries are injected into top-level prompt banners (`[RELAY HANDOFF]`), achieving seamless context continuity with zero memory loss.
- ⚡ **Autonomous Transition Hooks & Gatekeeper Sensors**: Customizable lifecycle automation runs scripts on stage entry/exit, while ground-truth compilers gate card progression.
- 🎨 **Obsidian Dark Aesthetic & Tactile Pop-Art Minimalism**: High-contrast obsidian dark palette (`#07090e`, `#0d1117`, `#161b22`), monospace tabular timers (`tabular-nums`), signature circular wave theme transitions, and zero AI-slop pastel gradients.

---

## 🏛️ System Architecture

CanbanVoimaxHarness decouples desktop OS interactions, asynchronous LLM streaming, and reactive state management into clean, unidirectional layers.

```text
+----------------------------------------------------------------------------------------+
|                                   CANBAN VOIMAX HARNESS                                |
|                                (Tauri v2 + React 19 Shell)                             |
+-------------------------------------------+--------------------------------------------+
|           REACT 19 TACTICAL UI            |               RUST 2021 BACKEND            |
|       (Zustand + Tailwind CSS v4)         |           (Tokio Async Runtime Engine)     |
|                                           |                                            |
|  +-------------------------------------+  |  +--------------------------------------+  |
|  |        KANBAN CONTROL PLANE         |  |  |         TAURI IPC ROUTER & CMDS      |  |
|  |  * Multi-Board Engine (.cvh/boards) |<--+--|  * commands::tasks, commands::boards |  |
|  |  * Reactive Columns (Columns=Agents)|  |  |  * commands::git, commands::cortex   |  |
|  |  * Monospace Tabular Stage Timers   |  |  +------------------+-------------------+  |
|  +------------------+------------------+  |                     |                      |
|                     | (Card Click)        |                     v                      |
|                     v                     |  +--------------------------------------+  |
|  +-------------------------------------+  |  |        AUTONOMOUS REACT LOOP         |  |
|  |          CARD-AS-A-CHAT             |  |  |  * Zero-Turn Memory Priming Engine   |  |
|  |  * Live <thinking> Introspection    |<--+--|  * Universal SSE Streaming Client    |  |
|  |  * Subagent Delegation Viewports    |  |  |  * Deterministic Prefix Caching      |  |
|  |  * Real-Time Diff Viewer & Steer Bar|  |  +------------------+-------------------+  |
|  +------------------+------------------+  |                     |                      |
|                     |                     |                     v                      |
|                     v                     |  +--------------------------------------+  |
|  +-------------------------------------+  |  |     TWO-TIER TOOL POLICY MATRIX      |  |
|  |          CORTEX EXPLORER            |  |  |  * Global Switches (Level A)         |  |
|  |  * Agent Persona & Rule Inspector   |<--+--|  * Stage Overrides (Level B, AND-Sem)|  |
|  |  * In-App Live Rule Editor (Ctrl+S) |  |  |  * Schema Stripping (-2,400+ tokens) |  |
|  +-------------------------------------+  |  +------------------+-------------------+  |
|                                           |                     |                      |
|                                           |                     v                      |
|  +-------------------------------------+  |  +--------------------------------------+  |
|  |          CONTEXT GAUGE UI           |  |  |     STATE-PRESERVING COMPACTOR       |  |
|  |  * Real-Time Token Ring Indicator   |<--+--|  * Zero-LLM Fast Evaporation (>500c) |  |
|  |  * 100% Context Ceiling Compaction  |  |  |  * Collapsible Paper Accordion Feed  |  |
|  +-------------------------------------+  |  +------------------+-------------------+  |
|                                           |                     |                      |
+-------------------------------------------+---------------------+----------------------+
|                             SUBSURFACE DATA LAYER               |                      |
|                                                                 v                      |
|  +---------------------------+  +----------------------------+  +-------------------+  |
|  |  SQLite WAL (.cvh/harness)|  |  Git Worktree Subsystem    |  |  Tool Output Spill|  |
|  |  * ACID dialogue logs     |  |  * Isolated task branches  |  |  * >500 lines /   |  |
|  |  * Sub-ms upsert metrics  |  |  * Autonomous merge gates  |  |    >64KB to disk  |  |
|  +---------------------------+  +----------------------------+  +-------------------+  |
+----------------------------------------------------------------------------------------+
```

---

## 🔄 Autonomous Pipeline & Relay Handoff Control Flow

In CanbanVoimaxHarness, **Columns Represent Autonomous Agent Personas**. When a card moves into a column, the agent loop executes with that column's primed persona, rules, and security boundaries.

```text
 +--------------+     +--------------+     +--------------+     +--------------+     +--------------+
 |   BACKLOG    |---->|     DEV      |---->|    REVIEW    |---->|    RETRO     |---->|    READY     |
 |  (Architect) |     | (Developer)  |     |  (Reviewer)  |     |  (Verifier)  |     |  (Terminal)  |
 +--------------+     +------+-------+     +------+-------+     +------+-------+     +--------------+
                             |                    |                    |                    ^
                             |                    |                    |                    |
              [Enter Stage]  v                    |                    |                    |
           Creates Isolated Git Worktree          |                    |                    |
           worktrees/<task-slug>                  |                    |                    |
           feat/<task-slug> branch                |                    |                    |
                             |                    |                    |                    |
                             v                    |                    |                    |
                   Executes Developer Agent       |                    |                    |
                   (Tools: edit, bash, subagent)  |                    |                    |
                             |                    |                    |                    |
                             v                    |                    |                    |
                 advance_task(summary)            |                    |                    |
                             |                    |                    |                    |
                             +-------------------->                    |                    |
                             | Relay Handoff Priming                   |                    |
                             v                                         |                    |
                 Runs Gatekeeper Sensor                                |                    |
                 (e.g., cargo check / tsc)                             |                    |
                             |                                         |                    |
                    +--------+--------+                                |                    |
              [Pass]|           [Fail]|                                |                    |
                    v                 v                                |                    |
         Spawns Isolated Child   Transition Blocked                    |                    |
         Review Subtask (<id>.1) Returns with error logs               |                    |
         (Tools: read-only)           |                                |                    |
                    |                 |                                |                    |
         Audits Diffs & Security      |                                |                    |
                    |                 |                                |                    |
          +---------+---------+       |                                |                    |
[APPROVED]| [CHANGES_REQUESTED|       |                                |                    |
          |                   v       |                                |                    |
          |         Unlocks Dev Card  |                                |                    |
          |         Status -> 'idle'  |                                |                    |
          |         Feedback Injected |                                |                    |
          v                   |       |                                |                    |
    advance_task(verdict)<----+-------+                                |                    |
          |                                                            |                    |
          +------------------------------------------------------------+                    |
                                     | Relay Handoff Priming                                |
                                     v                                                      |
                         Executes Retro Agent                                               |
                         * FTR Root-Cause Analysis                                          |
                         * Rules & Ledger Update                                            |
                                     |                                                      |
                                     v advance_task()                                       |
                         [Autonomous Worktree Cleanup] -------------------------------------+
                         * Verifies zero compiler errors
                         * Merges feat/<task-slug> into main
                         * Removes worktree directory
                         * Deletes feature branch
```

---

## ⚡ Technical Highlights & Subsystems

### 1. 🦀 Tauri v2 + Rust 2021 Tokio Async Engine (~40MB RAM Footprint)
The desktop runtime is engineered in Rust 2021 using the **Tokio** asynchronous runtime, completely eliminating GUI thread freezes. Tauri v2 provides zero-overhead native OS integration (native menus, file dialogs, process management, window vibrancy, and single-instance locks) with rock-solid security boundaries and standard memory isolation—sipping ~40MB of RAM instead of multi-gigabyte Electron bloat.

### 2. ⚛️ React 19 + Tailwind CSS v4 + Framer Motion
- **React 19 Reactivity**: Utilizes fine-grained Zustand store subscriptions with `useShallow` selectors and custom `React.memo` shallow comparators to eliminate unnecessary board repaints on timer ticks or card drags.
- **Tailwind CSS v4 Engine**: Built on the next-gen Tailwind v4 engine using native CSS variables and lightningcss compilation for instant styles.
- **Framer Motion Micro-Interactions**: Smooth drawer reveals, animated status pills, and the signature circular wave theme reveal powered by the modern CSS View Transitions API.

### 3. 🗄️ High-Performance SQLite Database in WAL Mode (`.cvh/harness.db`)
Task messages, streaming chunks, tool invocations, and thinking steps are persisted in SQLite configured with Write-Ahead Logging (`PRAGMA journal_mode = WAL`) and `PRAGMA synchronous = NORMAL`.
- **Indexed Schema**: `dialog_items` table indexed by `(task_id, created_at)`.
- **Zero Lock Contention**: Atomic sub-millisecond transactions prevent flat-file serialization corruption and allow simultaneous read/write access during heavy agent streaming.

### 4. 🌲 Deterministic Git Worktree Isolation Per Task
Never suffer from AI tools overwriting your unstaged changes or breaking your active build:
- Every task card automatically provisions an isolated worktree at `worktrees/<task-slug>` on branch `feat/<task-slug>`.
- All file reads, edits, shell commands, and tests execute inside that dedicated directory.
- Upon passing all review gates and receiving an `APPROVED` verdict, the branch merges cleanly into `main`, the worktree is safely unmounted and pruned, and temporary branches are reclaimed.

### 5. 🗜️ State-Preserving Dynamic Context Compactor & Prefix Caching
Long-running agent workflows inevitably encounter context window degradation and token exhaustion:
- **Zero-LLM Fast Evaporation**: Terminal outputs and file dumps older than the last 2 active turns exceeding 500 characters are automatically evaporated into lightweight metadata summaries without spending a single API token.
- **Deterministic Prefix Caching**: System messages and tool schemas are serialized in strict alphabetical order with zero dynamic timestamps or randomized session IDs in the prompt prefix, yielding **>90% provider cache hit rates**.
- **100% Context Window Compaction Ceiling**: Compaction triggers precisely when context reaches the configured stage context window limit, ensuring full buffer utilization without premature token loss.

### 6. 🛡️ 2-Tier Zero-Trust Tool Policy Matrix (Strict AND-Semantics & Schema Stripping)
Security is enforced as an ironclad barrier at both the configuration and runtime kernel levels:
- **Level A (Global Switches)**: Enforced globally across all pipelines (`.cvh/settings/tool-policy.json`).
- **Level B (Stage Overrides)**: Enforced per pipeline stage (e.g., Code Review stage disables `write_file` and `edit_file`).
- **Strict AND-Semantics**: `effective(tool, stage) = global_enabled[tool] && stage_override[stage][tool]`. A stage can never re-enable a globally disabled tool.
- **Zero-Trust Request Stripping**: Disabled tools are entirely pruned from the schema sent to the LLM, saving **~2,400+ prompt tokens per turn** and eliminating tool hallucination vectors.

### 7. ⚡ Autonomous Transition Hooks & Gatekeeper Sensors (`.cvh/hooks/`)
Stages execute custom transition scripts (`.bat`, `.cmd`, `.ps1`, `.sh`, binaries) upon entry and exit.
- **Injected Context Variables**: Scripts receive rich metadata via `CVH_TASK_ID`, `CVH_TASK_TITLE`, `CVH_WORKTREE_PATH`, `CVH_BRANCH`, and `CVH_STAGE_ID`.
- **Compiler Sensor Gates**: Stages enforce hard gatekeeper sensors (e.g. `cargo check`, `cargo clippy`, `pnpm run build`). If a sensor exits non-zero, stage transition is rejected, and the card remains pinned with full diagnostic logs.

### 8. 🔧 Dynamic Custom CLI Tools via Declarative JSON (`.cvh/tools/`)
Extend the agent's capabilities with zero Rust recompilation:
- Drop a descriptive manifest into `.cvh/tools/<tool_name>.json`.
- The engine dynamically registers the tool with parameter validation and automatic shell argument interpolation (`{{args}}`).
- Custom tools are immediately subject to the 2-Tier Tool Policy Matrix and permission governance.

### 9. 🏃 Relay Handoff Memory Priming
Context fragmentation across multi-stage engineering pipelines is eliminated:
- When a task transitions (`advance_task` or `reject_task`), the handover summary is recorded in `task.handover_summary`.
- The subsequent agent receives this context pre-compiled into a top-level prompt banner:
  ```text
  ### [ЭСТАФЕТА ПАЙПЛАЙНА / RELAY HANDOFF]
  Summary of previous stage execution, test results, and audit verdicts.
  ```
- The receiving agent starts Turn 1 with 100% memory primed—no wasted turns reading rule or log files.

### 10. 🧬 Native Subagent Delegation Hierarchy (Sync & Async)
CVH implements a first-class subagent delegation tree:
- **Sync Oneshot (`subagent`)**: Spawns an in-process child agent synchronously for focused subtasks, blocking until completion and returning a concise markdown report without moving cards or creating duplicate worktrees.
- **Async Background (`subagent_bg`)**: Dispatches long-running background tasks returning an immediate `subagent_id`.
- **Zero-Polling Collection (`collect_subagent`)**: Awaits background subagent completion via async notification channels without CPU-wasting polling loops.

### 11. 🌑 Obsidian Dark Theme & Tactile Pop-Art Minimalism
Designed for professional engineers who spend hours inside their IDE:
- Deep obsidian dark palette (`#07090e`, `#0d1117`, `#161b22`, `#21262d`, `#f0f6fc`).
- Monospace numerals with `tabular-nums` preventing layout shifts on live timers.
- Deep token propagation across all nested inputs, textareas, and modals.
- Crisp SVG iconography dynamically mapped to Lucide icons.
- Strict anti-slop guidelines: zero pastel gradient soup, zero generic cartoon avatars, zero distracting fluff.

---

## 🛠️ Industrial Tools Catalog

CVH equips agents with an industrial-grade tool suite matching the SWE-bench standard:

| Category | Tool | Description |
| :--- | :--- | :--- |
| **Pipeline Navigation** | `advance_task` / `move_task` | Moves task to next stage or specific `target_stage` with Relay Handoff summary. |
| | `reject_task` | Rejects task with audit critique, returning card to previous stage and unlocking parent. |
| **Subagent Hierarchy** | `subagent` | Synchronous in-process child task delegation with structured report return. |
| | `subagent_bg` | Asynchronous background delegation returning `subagent_id`. |
| | `collect_subagent` | Awaits completion of background subagents with zero polling overhead. |
| **Human-in-the-Loop** | `question` / `ask_user` | Suspends agent execution and prompts the user with interactive option choices. |
| | `todo_write` | Real-time tactical task checklist (`todos: [{content, status}]`). |
| **Filesystem & Shell** | `read_file` | Reads UTF-8 file contents with line offsets and limits. |
| | `write_file` | Writes full UTF-8 file contents to disk. |
| | `edit_file` (`str_replace`) | Precise text replacement with unique pattern matching. |
| | `glob` | Rapid pattern-based file discovery within the active worktree. |
| | `grep` | High-speed regex text search across worktree source files. |
| | `execute_command` (`bash`) | Runs shell commands with timeout enforcement and piped output. |
| **Research & Web** | `web_search` | DuckDuckGo search integration with zero API key requirement. |
| | `web_fetch` | Fetches webpage content converted into clean, readable Markdown. |
| **Custom Tools** | `.cvh/tools/*.json` | Dynamic CLI tool integration with automatic `{{args}}` parameter interpolation. |

*Note: Large outputs exceeding 500 lines or 64 KB for shell commands (`execute_command`), or 2,000 lines / 128 KB for `read_file`, automatically spill to disk at `.cvh/spill/<timestamp>-<tool>.log`, returning only the head (20 lines) and tail (20 lines) to preserve LLM context.*

---

## 🚀 Quickstart Guide

### Prerequisites
- **Node.js**: v20+ & **pnpm**: v10+
- **Rust Toolchain**: Rust 2021 Edition (`rustc` / `cargo` 1.80+)
- **Build Tools**:
  - **Windows**: Visual Studio C++ Build Tools (MSVC) or MinGW
  - **Linux**: `build-essential`, `libwebkit2gtk-4.1-dev`, `libssl-dev`
  - **macOS**: Xcode Command Line Tools

### 1. Repository Setup & Dependencies
```bash
# Clone the repository
git clone https://github.com/voimax/CanbanVoimaxHarness.git
cd CanbanVoimaxHarness

# Install frontend dependencies
pnpm install
```

### 2. Running in Web Development Mode
Test and develop the React 19 interface in your web browser with instantaneous Hot Module Reloading (HMR):
```bash
pnpm run dev
```
Open **[http://localhost:5173](http://localhost:5173)** in your browser.

### 3. Running as Native Desktop Application (Tauri v2 Dev)
Launch the desktop application with full native Rust IPC bindings, Git worktree management, and SQLite dialogue storage:
```bash
pnpm run tauri dev
```

### 4. Compiling Production Release Bundle
Build a self-contained, optimized native executable:
```bash
# Build desktop release package (.exe on Windows, AppImage/.deb on Linux, .app on macOS)
pnpm run tauri build
```
The optimized native binary is produced in:
📁 `src-tauri/target/release/canban-voimax-harness.exe`

---

## 🏛️ Project Directory Structure

```text
CanbanVoimaxHarness/
├── AGENTS.md                  # 3-Tier Multi-Agent Governance Manual & Invariants
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
│   ├── agents/                # Modular persona definitions (<role>/persona.md)
│   ├── settings/              # Tool Policy Matrix (tool-policy.json)
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
│   └── hooks.md               # Stage transition lifecycle scripts & hooks
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
│   │   ├── commands/          # Decoupled domain IPC submodules
│   │   │   ├── boards.rs      # Multi-board IPC commands
│   │   │   ├── tasks.rs       # Task lifecycle & optimistic state IPC
│   │   │   ├── dialogs.rs     # Dialogue persistence IPC
│   │   │   ├── git.rs         # Git worktree & diff IPC
│   │   │   ├── cortex.rs      # Cortex rule exploration IPC
│   │   │   ├── tools.rs       # Tool policy IPC
│   │   │   ├── llm.rs         # Model configuration IPC
│   │   │   └── state.rs       # Shared application state
│   │   └── llm/               # Universal SSE streaming client & schema models
│   ├── Cargo.toml             # Rust dependencies (Tauri, Tokio, Rusqlite, Serde)
│   └── tauri.conf.json        # Tauri v2 bundle and window configuration
│
└── src/                       # React 19 Tactical Frontend
    ├── components/
    │   ├── board/             # Kanban board, columns, and DND interactions
    │   ├── card/              # Tactile Kanban card & tabular stage timers
    │   ├── cortex/            # Cortex Explorer & in-app live rule editor
    │   ├── drawer/            # Card-as-a-Chat, live thinking, diff sidebar
    │   ├── layout/            # Topbar, status pills, and theme toggle
    │   └── settings/          # Tool policy matrix & model configuration modal
    ├── stores/                # Reactive Zustand state stores
    ├── types/                 # Shared TypeScript domain interfaces
    └── App.tsx                # Root layout, drag-and-drop context & overlays
```

---

## 📚 Architectural Documentation Index

For in-depth engineering documentation and guides:
- **[📘 docs/GUIDE.md](docs/GUIDE.md)**: Complete harness specification, IPC API catalog, and `.cvh` configuration manual.
- **[🤖 docs/agents.md](docs/agents.md)**: Agent ReAct loop mechanics, persona design, and tool policy enforcement.
- **[📋 docs/board.md](docs/board.md)**: Board Control Plane invariants, multi-board templates, and automatic verdict routing.
- **[⚡ docs/hooks.md](docs/hooks.md)**: Column transition scripts, gatekeeper sensors, and context environment variables.
- **[🚨 AGENTS.md](AGENTS.md)**: 3-Tier Multi-Agent Governance manual, architectural invariants, and code hygiene standards.

---

## ⚖️ License & Open Source

CanbanVoimaxHarness is licensed under the **[MIT License](LICENSE)**.  
Engineered with precision by the Voimax team for high-velocity autonomous software development.
