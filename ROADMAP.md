# 🗺️ CanbanVoimaxHarness (CVH) — Development Roadmap

> **Next-Generation Lightweight Desktop Agentic IDE & Orchestrator**  
> **Core Stack:** Rust 2021 + Tauri v2 + Tokio + React 19 + TypeScript + Tailwind CSS v4 + Framer Motion  
> **Mission:** Ultra-lightweight (~40MB RAM) desktop harness where the Kanban board is the primary execution control plane with native Git worktree isolation, dynamic graph pipelines, and Zero-Turn Memory Priming.

---

## 📅 Roadmap Overview & Execution Phases

```
┌────────────────────────────────────────────────────────────────────────┐
│ Phase 1: Environment & Toolchain Setup (Rust toolchain, Tauri v2 init) │
└───────────────────────────────────┬────────────────────────────────────┘
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│ Phase 2: React 19 + Tailwind v4 + Lucide Tactile Frontend Scaffolding   │
└───────────────────────────────────┬────────────────────────────────────┘
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│ Phase 3: Core Rust Systems Engine (`src-tauri/`)                       │
│  - Config & Pipeline Parser (.cvh/)                                    │
│  - Native Git Worktree Manager (`git.rs`)                              │
│  - Transition Hooks Executor (`hooks.rs`)                              │
│  - Universal SSE Streaming Client (`llm/`)                             │
│  - Zero-Turn Memory Priming & ReAct Loop (`agent.rs`)                  │
│  - Tauri v2 IPC Commands & Push Events (`commands.rs`)                 │
└───────────────────────────────────┬────────────────────────────────────┘
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│ Phase 4: First-Class Kanban Control Plane UI (`src/`)                  │
│  - Tactile Cyber-Slate / Obsidian Theme + Neon Accents                 │
│  - Dynamic Multi-Column DND Board with Column Routing                  │
│  - Cards with Monospace Timers, Fast-Track, Verdicts, Returns Counter  │
│  - Slide-Out Drawer: Agent Thinking Stream & Interactive Tool Cards    │
│  - Process Control: Pause, Force Kill, Promote to Ready, Retry         │
└───────────────────────────────────┬────────────────────────────────────┘
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│ Phase 5: End-to-End Integration & NASA Ground Truth Sensors            │
│  - `cargo check` & `cargo clippy -- -D warnings`                       │
│  - `npm run build` (tsc & vite build)                                  │
│  - Real subprocess & Antigravity AI proxy streaming verification       │
└───────────────────────────────────┬────────────────────────────────────┘
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│ Phase 6: Dogfooding Bootstrap                                          │
│  - Launch CanbanVoimaxHarness locally                                  │
│  - Execute subsequent feature iterations through CVH itself!           │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 🎯 Detailed Tasks Breakdown & Subagent Assignments

### 🔹 Milestone 1: Toolchain & Project Scaffolding
- **Task CVH-001**: Rust Toolchain & Tauri v2 Core Scaffolding
  - Install Rust/Cargo toolchain in user environment.
  - Initialize Tauri v2 desktop application structure in `src-tauri/` with `Cargo.toml`, dependencies (`tauri`, `tokio`, `serde`, `serde_json`, `reqwest`, `eventsource-stream`).
  - *Lead Role:* `tauri-rust-engineer` (Level 1)
  - *Verification:* `cargo check` in isolated worktree.

- **Task CVH-002**: React 19 + TypeScript + Vite + Tailwind v4 Scaffolding
  - Setup `package.json`, Vite configuration, TypeScript types, and Tailwind CSS v4 styling.
  - Install icons (`lucide-react`), drag-and-drop (`@hello-pangea/dnd`), state management (`zustand`), and animations (`framer-motion`).
  - *Lead Role:* `react-tailwind-artisan` (Level 1)
  - *Verification:* `npm run build` in isolated worktree.

---

### 🔹 Milestone 2: Rust Systems Engine (`src-tauri/`)
- **Task CVH-003**: Dynamic Pipeline & Project Configuration Parser
  - Parse `.cvh/config.json` and `.cvh/pipeline.json`.
  - Typed models for columns, personas, context windows, model overrides, reasoning effort, gatekeeper sensors, and routes.
  - Multi-project loader & switcher.
  - *Lead Role:* `tauri-rust-engineer` (Level 1)

- **Task CVH-004**: Native Git Worktree & Branch Manager (`git.rs`)
  - Subprocess execution of `git worktree add`, `git worktree remove`, branch creation, merge back into `main`.
  - Robust error handling, zero naked unwraps, clean directory isolation.
  - *Lead Role:* `tauri-rust-engineer` (Level 1)

- **Task CVH-005**: Lifecycle Transition Hooks Engine (`hooks.rs`)
  - Execute `.bat` / `.ps1` / `.sh` on `on_enter`, `on_leave`, `on_finish`.
  - Inject environment variables: `%CVH_TASK_ID%`, `%CVH_TASK_TITLE%`, `%CVH_WORKTREE_PATH%`, `%CVH_BRANCH%`, `%CVH_VERDICT%`.
  - Windows stdio drain and process tree kill on timeout/abort.
  - *Lead Role:* `tauri-rust-engineer` (Level 1)

- **Task CVH-006**: Universal LLM SSE Streaming Client (`llm/`)
  - OpenAI-compatible endpoint streaming (`POST /chat/completions` with SSE) for Local Antigravity AI Proxy (`http://127.0.0.1:port`), DeepSeek, Ollama.
  - Anthropic Claude API streaming support (`POST /messages`).
  - Native token emitter to Tauri webview via `app_handle.emit("agent:token", ...)`.
  - *Lead Role:* `llm-streaming-expert` (Level 1)

- **Task CVH-006B**: Deterministic Prefix Caching in LLM Client (`llm/caching.rs`)
  - Tool definitions sorted alphabetically by name (`tools.sort_by(...)`) before serialization.
  - Strictly static System Prompt without dynamic timestamps or floating random IDs.
  - Append-only conversation history guaranteeing immutable prefix blocks.
  - Automatic `cache_control: {"type": "ephemeral"}` insertion for Anthropic API (up to 4 cache breakpoints).
  - Target: >= 90% Cache Hit Rate with `cache_read_input_tokens` vs `cache_creation_input_tokens` telemetry.
  - *Lead Role:* `llm-streaming-expert` (Level 1)

- **Task CVH-007**: Zero-Turn Memory Priming & Native ReAct Agent Loop (`agent.rs`)
  - Turn 0 prompt compiler: `Persona + Project Guidelines (AGENTS.md) + Active Skills + Task Description`.
  - Tool execution dispatch with streaming feedback to UI.
  - *Lead Role:* `agent-developer` (Level 1)

- **Task CVH-007C**: State-Preserving Context Compactor (`agent/compactor.rs`)
  - **Zero-LLM Log Evaporation**: automatically evaporates heavy stdout logs and bulky file dumps (>500 chars) older than 3 turns without making LLM calls (saves up to 80% tokens).
  - **Immutable Anchors**: System Prompt, initial Task Specification (ТЗ), and the 2 most recent turns are strictly preserved.
  - **Structured State Snapshot**: compresses middle turns into a rigorous structured block (`Modified Files`, `Build Status`, `Exact Next Step`, `Active Verdict`) rather than vague conversational prose.
  - **UI Context Gauge & Trigger**: visual token progress bar in the Agent Drawer with a one-click manual `⚡ Compact Context` button.
  - *Lead Role:* `tauri-rust-engineer` (Level 1)

- **Task CVH-007B**: Industrial Tool Suite & Tool Policy Matrix (`tools/`)
  - Industry-standard SWE-bench / Claude Code signatures:
    - `execute_command` (with `bash` alias)
    - `read_file`
    - `edit_file` (`str_replace` with rich context diff on mismatch for rapid self-correction)
    - `write_file`
    - `glob`
    - `grep`
    - `subagent`
  - Tool Aliasing layer for zero friction across model formats.
  - **Tool Policy Matrix (Матрица управления инструментами)**:
    - Global toggles (Settings ➔ Tools): enable/disable any tool project-wide.
    - Per-stage permissions (Колонка РЕВЬЮ: `read_file`, `glob`, `grep`, read-only `bash` ENABLED; `edit_file`, `write_file` DISABLED DEAD. Колонка ДЕВ: `read_file`, `edit_file`, `write_file`, `bash` ENABLED; `web_search` DISABLED).
    - Excluded from LLM request schemas (saves 200–400 tokens per tool per turn; ~2000+ tokens per step).
    - Zero-Trust Iron Barrier in Rust: intercept disabled calls with `PermissionDenied`.
  - *Lead Role:* `tauri-rust-engineer` (Level 1)

- **Task CVH-008**: Tauri v2 IPC Command Layer & State Management (`commands.rs`)
  - Register IPC commands: `get_projects`, `get_pipeline`, `list_tasks`, `create_task`, `move_task_stage`, `run_agent`, `abort_agent`, `get_task_logs`.
  - Connect Tokio async channels to Tauri event emitters.
  - *Lead Role:* `tauri-rust-engineer` (Level 1)

---

### 🔹 Milestone 3: Tactile Control Plane UI (`src/`)
- **Task CVH-009**: Cyber-Slate Kanban Board & Column Graph View
  - Dynamic columns rendered from `.cvh/pipeline.json`.
  - Drag-and-drop reordering & stage transition triggers.
  - Fast-track indicators, column gatekeeper status, and stage WIP counters.
  - Clicking a column on the board focuses the corresponding folder in Cortex Explorer.
  - *Lead Role:* `react-tailwind-artisan` (Level 1)

- **Task CVH-009B**: Cortex Explorer («Проводник разума агентов») in Left Sidebar
  - Replaces traditional project file tree with an agent-centric mind explorer in VS Code style.
  - 1-to-1 color connection with Kanban columns (e.g. Blue column = Blue folder with agent rules).
  - Inside each agent folder: `persona.md`, modular rules `rules/*.md`, and stage settings.
  - Clicking a column focuses its Cortex folder with instant on-the-fly markdown editing.
  - *Lead Role:* `react-tailwind-artisan` (Level 1)

- **Task CVH-010**: Tactile Kanban Card & Status Badges
  - Card layout: ID, Title, Persona badge, Stage progress, Returns badge (`↩ Returns: Xx`).
  - Monospace tabular timers (`font-mono tabular-nums`): per-stage frozen timers, running stage timer, total elapsed time.
  - Fast-track tags (`⚡ Fast-Track: No Review`, `⚡ Fast-Track: No Retro`).
  - Official Verdict badges: `✔ APPROVED` (emerald), `✔ APPROVED WITH CAVEATS` (amber), `✖ CHANGES REQUESTED` (crimson).
  - *Lead Role:* `react-tailwind-artisan` (Level 1)

- **Task CVH-011**: Real-Time Slide-Out Drawer (Agent Execution Console)
  - Split view drawer: Live thinking stream (`<thinking>` tokens in dim slate) and final text stream.
  - Interactive tool execution accordions: input args, stdout/stderr logs, execution duration, and diff highlights.
  - Live control panel: Pause, Force Kill, Promote to Ready, Retry.
  - Audio feedback cues on transitions, approvals, and returns.
  - *Lead Role:* `react-tailwind-artisan` (Level 1)

- **Task CVH-012**: Project Switcher & Settings Drawer
  - Dropdown selector for active projects (`.cvh/` detector).
  - Pipeline configuration viewer with visual graph route arrows.
  - Persona prompt editor.
  - *Lead Role:* `react-tailwind-artisan` (Level 1)

---

### 🔹 Milestone 4: Integration, Verification & First Native Build
- **Task CVH-013**: Full System Integration & Sensor Testing
  - Run `cargo check` and `cargo clippy -- -D warnings`.
  - Run `npm run build`.
  - Test real task flow: Create card -> Dev stage spawns agent -> Tools executed in isolated worktree -> Reviewer verifies -> Approved -> Merged into main.
  - *Lead Role:* `agent-developer` (Level 1)

- **Task CVH-014**: Dogfooding Bootstrap Run
  - Build dev target and execute CanbanVoimaxHarness natively.
  - Load `C:\Users\Voimax\Projects\CanbanVoimaxHarness` as the active project.
  - Orchestrate all subsequent tasks through our own desktop harness UI!
