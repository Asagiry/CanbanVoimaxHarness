# 🚀 CanbanVoimaxHarness (CVH)

> **The Next-Generation Lightweight Desktop Agentic IDE & Orchestrator**  
> Built with **Rust 2021 (Tauri v2 Core)**, **Tokio**, **React 19**, **TypeScript**, and **Tailwind CSS v4**.

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
- **Dynamic Pipelines (`.cvh/pipeline.json`)**: Arbitrary multi-column routing graph (`A -> B -> C -> D -> E` or `Ideation -> Spec -> Dev -> QA -> Prod`) with per-stage context window caps (`400k`, `200k`, `128k`) and model overrides.
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
- **Level A (Global Switches: Settings ➔ Tools)**: Master toggles for `execute_command` (bash), `read_file`, `edit_file`, `write_file`, `glob`, `grep`, `web_search`, `web_fetch`, `subagent`.
- **Level B (Per-Stage Permissions)**: Column-level boundaries (e.g. in *Review Gate*, `edit_file` and `write_file` are locked off completely—the reviewer cannot mutate code).
- **Zero-Trust Schema Exclusion**: Disabled tools are **completely stripped from the LLM request schemas**, saving ~2,400+ prompt tokens per step and eliminating hallucinations.
- **Rust Iron Barrier**: Kernel-level interception in Rust returns `PermissionDenied` if a model attempts to invoke an unauthorized tool.

### 5. 🗜️ State-Preserving Context Compactor
- **Zero-LLM Log Evaporation**: Heavy terminal outputs and file dumps older than 3 turns (>500 chars) are automatically evaporated into lightweight summary tags (`[Output evaporated: 4,820 bytes. Exit: 0]`) without making LLM API calls, reclaiming up to 80% context tokens instantly.
- **Four Immutable Anchors**: System Prompt, initial Task Prompt (ТЗ), and the 2 most recent turns are strictly preserved to maintain KV prefix cache validity.
- **Structured State Snapshot**: Compresses middle turns into a structured Markdown block (`## 📌 Preserved Execution State`: Modified Files, Sensor Status, Next Step, Verdict).
- **`ContextGauge` UI**: Real-time token consumption progress bar with one-click `⚡ Compact Context` trigger.

### 6. ⚡ Deterministic Prefix Caching (Cache Hit Rate >= 90%)
- **Alphabetical Tool Sorting**: Tool definitions are deterministically sorted by name (`tools.sort_by(|a, b| a.name.cmp(&b.name))`) before serialization.
- **Static Primed Memory**: Zero dynamic timestamps or random UUIDs in the System prompt.
- **Anthropic Ephemeral Breakpoints**: Injects `"cache_control": {"type": "ephemeral"}` onto System prompts and stable user/assistant anchors (up to 4 cache breakpoints per Anthropic spec).

### 7. 🌲 Native Git Worktree Isolation & Hooks
- Managed natively by Rust via `tokio::process::Command` without shell injection or deadlocks.
- Every task executes inside an isolated `worktrees/<task-slug>` directory on branch `feat/<task-slug>`.
- **Custom Lifecycle Hooks**: Runs `.bat`, `.ps1`, or `.sh` on stage entry, exit, or completion with environment variables `%CVH_TASK_ID%`, `%CVH_WORKTREE_PATH%`, `%CVH_BRANCH%`, `%CVH_VERDICT%`.

### 8. 🔍 Industrial SWE-bench Tools, Spilling & Web Search
- Industrial signatures: `execute_command` (alias `bash`), `read_file`, `edit_file` (`str_replace`), `write_file`, `glob`, `grep`, `subagent`.
- **Tool Output Spilling**: Outputs >100 lines or >8 KB are saved to `.cvh/spill/<timestamp>-<tool>.log`, passing HEAD 20 + TAIL 20 lines to LLM context.
- **`web_search`**: Free DuckDuckGo HTML/API search (zero API key required) with Google/Bing fallback.
- **`web_fetch`**: HTTP GET converting HTML into clean Markdown.

---

## 🚀 Quickstart & Development

### Prerequisites
- **Node.js**: v20+ & npm 10+
- **Rust Toolchain**: Rust 2021 (Cargo 1.80+)
- **Windows Build Tools / MinGW / Zig**: Clang/Zig for linking

### 1. Run Web UI Control Plane (Dev Mode)
```bash
# Start Vite development server with Hot Module Reloading
npm run dev
```
Open **[http://localhost:5173](http://localhost:5173)** in your browser.

### 2. Run Native Desktop Application (Tauri v2)
```bash
# Launch native desktop application with full Rust IPC bindings
npm run tauri dev
```

### 3. Build Native Desktop Executable (`.exe`)
```bash
# Compiles the single self-contained binary (~22MB)
npm run tauri build
```
Output executable:
📁 `src-tauri/target/release/CanbanVoimaxHarness.exe`

---

## 🏛️ Project Directory Structure

```text
CanbanVoimaxHarness/
├── AGENTS.md                # 3-Tier Governance & Multi-Agent Engineering Manual
├── ROADMAP.md               # Architecture roadmap and milestones
├── .cvh/                    # Dogfooding configuration for CVH itself
│   ├── config.json          # Project endpoints & model definitions
│   ├── pipeline.json        # Dynamic pipeline columns & routing graph
│   ├── agents/              # Persona system prompts (developer, reviewer, retro)
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
