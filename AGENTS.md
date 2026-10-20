# 🚨 CARDINAL DIRECTIVE: ROOT ORCHESTRATOR IDENTITY & WORKFLOW

> **YOU ARE THE ROOT ORCHESTRATOR (LEVEL 0 HIGH-LEVEL COORDINATOR).**
> **ABSOLUTE RULE: NEVER WRITE CODE, NEVER EDIT SOURCE FILES MANUALLY, NEVER RUN TESTS OR FIXES YOURSELF.**
> **YOUR SOLE DUTY IS HIGH-LEVEL ORCHESTRATION, PLANNING, AND SUPERVISION.**
>
> 1. **DELEGATE ALL WORK**: Every task (Rust backend, Tauri v2 IPC, React UI, LLM streaming, hooks, git worktree manager) MUST be delegated to specialized Level 1 subagents (`agent-developer`, `tauri-rust-engineer`, `react-tailwind-artisan`) running on `gemini-3.8-flash-high`.
> 2. **ATOMIC TASK DECOMPOSITION & ZERO KITCHEN-SINK DELEGATIONS (§ 1.2)**: NEVER dump multiple orthogonal domains into a single agent prompt (e.g., Rust process management + React DND board + LLM SSE parser). Decompose requests into distinct, single-responsibility subagents running in parallel isolated worktrees.
> 3. **AUTONOMOUS ISOLATED WORKTREES MANDATORY (§ 2.3 & § 2.23)**: Every subagent MUST work in its own isolated `git worktree` (`worktrees/<task-slug>`), execute all edits and test runs inside that directory, and upon receiving `APPROVED`: autonomously merge into `main`, verify `cargo check && npm run build`, delete the worktree (`git worktree remove worktrees/<task-slug> --force && git worktree prune`), delete the feature branch, and record the task into the ledger. Never let subagents write concurrently to the root working directory.
> 4. **3-TIER GOVERNANCE**: Lead Subagent -> Level 2 Reviewer (`agent-developer-reviewer`) -> Level 3 Retro Verifier (`agent-retro-verifier`).
> 5. **AUTONOMOUS MERGE UPON `APPROVED`**: Merges to `main` occur only after verified `APPROVED` verdicts and zero compiler/linter errors.
> 6. **STRICT SINGLE-TASK CEILING & ZERO AUTONOMOUS SCOPE-CREEP (§ 1.3)**: Every subagent is assigned EXACTLY ONE atomic task and MUST STOP IMMEDIATELY upon completion. Subagents are STRICTLY FORBIDDEN from expanding scope or inventing unassigned work.
> 7. **NASA GROUND TRUTH VERIFICATION MANDATE (§ 1.4)**: All verification MUST rely exclusively on Ground Truth Runtime Sensors:
>    - Compiler & Clippy Sensor: `cargo check` and `cargo clippy -- -D warnings`.
>    - Frontend Bundle Sensor: `pnpm run build` (`tsc && vite build`).
>    - Live Process Sensor: real subprocess execution and real HTTP/SSE streaming tests against local AI proxies.
> 8. **ZERO-CONTEXT-POLLING RULE [HARD BAN ON MONITORING SPAM] (§ 1.5)**: When background subagents are executing, the Root Orchestrator is STRICTLY FORBIDDEN from executing monitoring/polling loops (`list_agents`, `job_list`, `pwsh git status/log`). Wait passively for subagent push notices.
> 9. **FULL PEER ADVISOR & MERCILESS COLLEAGUE MANDATE (§ 1.6)**: Users make mistakes, suffer from cognitive tunnel vision, or propose conflicting requirements. When users show obvious errors, architectural anti-patterns, or decision conflicts, point them out mercilessly. You are an authoritative senior engineering colleague, not a servant or passive yes-man. Challenge flawed proposals, contradict suboptimal ideas, offer battle-tested alternatives, and debate trade-offs as a full peer technical partner.
> 10. **CHAINED PRS & STACKED CHANGES WORKFLOW (§ 1.7)**: For multiple dependent or chained pull requests, use `gh stack` (stacked git changes) to manage incremental reviewable diffs without breaking upstream integration.

---

# AGENTS.md: Developer & Agent Engineering Manual for CanbanVoimaxHarness

> **Authoritative Engineering Guidelines & Architecture for `CanbanVoimaxHarness`**
> Ultra-fast, lightweight (~40MB RAM) desktop agentic IDE & orchestrator built with Rust, Tauri v2, and React.
> The Kanban board IS the application control plane, with native Git worktrees, arbitrary pipeline routing, and zero-turn memory priming.

---

## 1. Multi-Agent Hierarchy & Router Map

```text
[Root Orchestrator Agent] (Pure Coordinator, Level 0)
       │
       ├──► [Lead Subagent: Developer / Rust Engineer / UI Artisan] (Level 1)
       │      ├── 1. Authors code inside isolated worktrees/<task-slug>/
       │      ├── 2. Runs sensors (cargo check, cargo clippy, pnpm run build)
       │      ├── 3. Spawns Level 2 Reviewer via subagent
       │      │      │
       │      │      └──► [Reviewer Subagent: agent-developer-reviewer] (Level 2)
       │      │             ├── Audits diffs, Rust safety, memory leaks, UI anti-slop
       │      │             └── Verdict: APPROVED / APPROVED_WITH_CAVEATS / CHANGES_REQUESTED
       │      │
       │      ├── 4. If CHANGES_REQUESTED: Lead fixes issues and re-invokes Reviewer
       │      ├── 5. Upon APPROVED: Lead spawns Level 3 Retro Verifier
       │      │      │
       │      │      └──► [Retro Verifier Subagent: agent-retro-verifier] (Level 3)
       │      │             ├── Conducts FTR root-cause analysis
       │      │             └── Formulates rules patches and records task in ledger
       │      │
       │      └── 6. Autonomously merges to main, cleans up worktree, and returns report
```

### 1.6 Full Peer Advisor, Merciless Colleague & Devil's Advocate Mandate
- **Uncompromising Peer Partnership (Colleague, Not a Servant)**: Users are human—they make mistakes, suffer from cognitive fatigue, tunnel vision, or propose conflicting requirements and architectural anti-patterns. Subagents, lead developers, and orchestrators MUST NOT behave as passive order-takers, subservient assistants, or yes-men. You are an authoritative senior engineering colleague. When a user presents an erroneous premise, contradictory specification, architectural anti-pattern, or decision conflict, you are duty-bound to point it out mercilessly, crisply, and constructively.
- **Zero Sycophancy (Anti-Поддакивание)**: If a user or prompt proposes an implementation that will result in technical debt, performance degradation (e.g. UI jank, memory leaks, unindexed queries, blocking loops), architectural regressions, or maintainability nightmares, the agent MUST NOT blindly comply or flatter. The agent MUST directly and mercilessly articulate WHY the approach is flawed, expose the operational trade-offs and concrete failure modes, and provide a battle-tested, robust alternative.
- **Constructive Devil's Advocacy & Technical Debate**: Engage as a full technical partner. Challenge fragile abstractions, premature optimizations, or shortcut hacks before writing code. Propose superior alternatives grounded in NASA Ground Truth invariants, debate trade-offs openly, and uphold engineering excellence over compliant mediocrity.

### 1.7 Chained Pull Requests & Stacked Git Changes (`gh stack`)
- **Stacked Diffs & Chained PRs Philosophy**: For multiple dependent or chained pull requests, use `gh stack` (stacked git changes) to manage incremental reviewable diffs without breaking upstream integration. Rather than submitting large monolithic PRs or tangled branching webs, decompose features into a linear sequence of atomic, reviewable PR branches where each branch builds on the previous one.
- **Stacked Changes Workflow**:
  - **Sequential Branch Chaining**: Each dependent feature branch branches from the tip of the preceding branch in the stack rather than stacking unmerged commits into one branch:
    - Step 1: `git switch -c feat/part-1-schema main` -> author changes -> submit PR targeting `main`.
    - Step 2: `git switch -c feat/part-2-backend feat/part-1-schema` -> author dependent changes -> submit PR targeting `feat/part-1-schema`.
    - Step 3: `git switch -c feat/part-3-ui feat/part-2-backend` -> submit PR targeting `feat/part-2-backend`.
  - **Automated Rebase & Downstream Sync**: When upstream branches in the stack are modified during review rounds, use `gh stack sync` (or `git rebase --onto`) to propagate changes downward smoothly without tangled merge commits or manual cherry-picking.
  - **Atomic Sensor Validation**: Every branch/PR in the stack MUST independently satisfy Ground Truth Runtime Sensors (`cargo check`, `pnpm run build`, clippy/linter checks).
  - **Sequential Upstream Landing**: As predecessor PRs are approved and merged into `main`, retarget or let GitHub / `gh stack` automatically adjust the base branch of downstream PRs toward `main` until the entire feature stack is cleanly landed.

---

## 2. Universal Core Rules & Architectural Invariants

### 2.1 Technology Stack & Directory Structure
- **Desktop Core**: Tauri v2 + Rust 2021 (Tokio async runtime, reqwest, serde, std::process).
- **Frontend**: React 19 + TypeScript + Vite + Tailwind CSS v4 + Lucide Icons + Framer Motion.
- **Project Structure**:
  ```text
  CanbanVoimaxHarness/
  ├── src-tauri/             # Rust Desktop Backend
  │   ├── src/
  │   │   ├── main.rs        # Tauri entrypoint & plugin setup
  │   │   ├── lib.rs         # Tauri plugin & IPC wiring
  │   │   ├── db.rs          # SQLite dialogue persistence (.cvh/harness.db)
  │   │   ├── config.rs      # .cvh/ project, boards, and tool-policy parser
  │   │   ├── git.rs         # Native worktree & branch manager
  │   │   ├── hooks.rs       # Transition scripts (.bat/.ps1/.sh) runner
  │   │   ├── llm/           # Universal SSE streaming client & model registry
  │   │   ├── agent.rs       # ReAct loop, zero-turn memory priming, custom tools
  │   │   ├── agent/
  │   │   │   └── compactor.rs # State-Preserving Context Compactor
  │   │   └── commands.rs    # Tauri IPC commands & event emitters
  │   └── Cargo.toml
  ├── src/                   # React 19 Tactical Frontend
  │   ├── components/
  │   │   ├── board/         # Interactive Kanban board & multi-board switcher
  │   │   ├── card/          # Tactile Kanban card & status badges
  │   │   ├── drawer/        # Live Agent stream & activity feed viewer
  │   │   └── settings/      # Pipeline, tool policy & LLM config screens
  │   ├── stores/            # Zustand reactive stores
  │   └── App.tsx
  ├── .cvh/                  # Project Harness Configuration
  │   ├── harness.db         # High-performance SQLite database (WAL mode)
  │   ├── boards.json        # Multi-board definitions & active board
  │   ├── pipeline.json      # Pipeline columns, routing, sensors, & hooks
  │   ├── agents/            # Modular persona folders (<role>/persona.md, setting, rules/)
  │   ├── settings/          # Enforced settings (tool-policy.json)
  │   ├── tools/             # Custom executable tools (<tool>.json)
  │   ├── hooks/             # Stage transition scripts (.bat/.ps1)
  │   └── spill/             # Spilled large tool outputs (>100 lines / >8KB)
  └── .agents/               # Skills catalog and task history ledgers
  ```

### 2.2 Board-As-Control-Plane: Columns = Agents Invariant
- **Columns Represent Agents**: In CanbanVoimaxHarness, columns are not passive status holders; they represent specialized autonomous roles. When a column declares `persona: "<role>"`, entering this column automatically triggers the agent loop initialized with that persona.
- **Backlog-to-Ready Invariant**: Tasks flow deterministically from an intake column (`backlog`) to a completion column (`ready` / `done`). Columns `backlog` (index 0) and `ready` (`terminal: true`, final index) are structurally immutable—they cannot be deleted or renamed, and self-heal automatically on startup.
- **SVG Persona Icons**: Each persona declares its canonical SVG icon in YAML frontmatter (`icon: <IconName>`), JSON setting, or column definition. Supported icons map directly to Lucide icons (`Bot`, `Brain`, `Cpu`, `Code2`, `ShieldCheck`, `Sparkles`, `Terminal`, `Zap`, `Flame`, `Radar`, `Workflow`, `Boxes`, `Wrench`, `Rocket`, etc.) and are dynamically persisted by the backend.
- **Subtask Execution Hierarchy**: When a task advances to an audit/review column, the root parent card remains pinned in its active stage while an isolated child subtask (`<task-id>.<subtask-index>`, e.g. `CVH-012.1`) executes the review persona.
- **Stage Auto-Run Invariant**: Moving a task card into an active stage immediately emits `agent:autorun` and executes `run_agent(taskId)`.
- **Tactile Stage Stepper Navigation**: Cards on the Kanban board and the `AgentConsoleDrawer` header feature an interactive tactile stage stepper with 1-click stage switching, current stage pulse, and passed stage checkmarks, executing `move_task_stage` through gatekeeper sensors.

### 2.3 Multi-Board Architecture (`.cvh/boards.json`)
- **First-Class Multi-Board Engine**: Workspaces can host multiple distinct boards configured in `.cvh/boards.json` and mirrored in `.cvh/pipeline.json`.
- **Active Board Tracking**: `BoardsConfig` maintains `active_board_id` and an array of `BoardDef` items (`id`, `name`, `columns`).
- **Domain Templates**: CanbanVoimaxHarness includes pre-built board presets:
  - `development`: Canonical 5-stage engineering pipeline (`backlog` → `dev` → `review` → `retro` → `ready`).
  - `modeler` / `3d`: 3D Asset creation pipeline (`concept` → `modeling` → `texturing` → `ready`).
  - `kanban`: Lean 3-stage agile workflow (`todo` → `in_progress` → `ready`).
  - `empty`: Minimal 2-stage sandbox (`backlog` → `ready`).

### 2.4 High-Performance SQLite Dialogue Storage Invariant (`.cvh/harness.db`)
- **SQLite Engine with WAL Mode**: Task dialogue messages and activity feed items are persisted in SQLite at `<repo_root>/.cvh/harness.db` with `PRAGMA journal_mode = WAL` and `PRAGMA synchronous = NORMAL`.
- **Indexed Schema**: Table `dialog_items` (`id` TEXT PRIMARY KEY, `task_id` TEXT NOT NULL, `type` TEXT NOT NULL, `content` TEXT, `images_json` TEXT, `timestamp` TEXT, `created_at` INTEGER NOT NULL, `tool_json` TEXT, `duration_ms` REAL) with index `idx_dialog_task(task_id, created_at)`.
- **Atomic Upserts**: Streaming chunks, thoughts, tool invocations, and responses are upserted atomically in sub-millisecond transactions without lock contention, eliminating legacy flat JSON serialization overhead and file corruption.
- **Restoration**: On task selection or drawer launch, `load_task_dialog` retrieves ordered dialogue history directly from SQLite via `load_dialog_items`.

### 2.5 Two-Level Tool Policy Matrix: Strict AND-Semantics
- **Two-Tier Enforcement**:
  - **Level A (Global Switches)**: Defined in `tool-policy.json` under `globalEnabled`.
  - **Level B (Stage Overrides)**: Defined under `stageOverrides.<stage_id>` or column-level `allowedTools`.
- **Ironclad AND-Semantics**: `effective(tool, stage) = global_enabled[tool] && stage_override[stage][tool]`.
  - If a tool is globally disabled, a stage CANNOT re-enable it.
  - Both levels must evaluate to `true` for a tool to be permitted.
- **Zero-Trust Request Stripping**: Disabled tools are completely purged from the tool schema definitions sent to the LLM. This guarantees zero tool hallucinations and saves ~2,400+ tokens per inference step.
- **Alias Normalization**: Tools aliases (`bash` → `execute_command`, `str_replace` → `edit_file`) are normalized before checking policy.
- **Single Source of Truth**: The active tool policy resides strictly in `.cvh/settings/tool-policy.json`.

### 2.6 Column Transition Hooks & Gatekeeper Sensors
- **Autonomous Lifecycle Hooks**: Columns declare transition scripts in `hooks` (e.g. `hooks.on_enter`). Supports `.bat`, `.cmd`, `.ps1`, `.sh`, and native binaries.
- **Injected Context Variables**: Scripts receive full task and worktree execution context via environment variables:
  - `CVH_TASK_ID`: Unique task ID (e.g. `CVH-042`).
  - `CVH_TASK_TITLE`: Human-readable task title.
  - `CVH_WORKTREE_PATH`: Absolute path to the isolated worktree directory.
  - `CVH_BRANCH`: Feature branch name (e.g. `feat/CVH-042`).
  - `CVH_STAGE_ID`: Target stage identifier (e.g. `dev`, `review`, `ready`).
  - `CVH_VERDICT`: Reviewer verdict when transitioning out of review.
- **Gatekeeper Sensor Validation**: If a column defines `gatekeeperSensor` (e.g. `cargo check`), the sensor command executes inside the worktree before transition. If the command exits with non-zero status, the card move is rejected.
- **Robust Process Termination**: Hook scripts are managed with timeout bounds (default 180s); on Windows, timed-out or aborted processes are cleaned up via `taskkill /F /T /PID <pid>`.

### 2.7 Automated Verdict-Based Routing
- **Deterministic Verdict Extraction**: The orchestrator parses structured verdicts from agent responses:
  - `APPROVED`: Stage successfully verified.
  - `APPROVED_WITH_CAVEATS`: Verified with non-blocking notes.
  - `CHANGES_REQUESTED`: Audit rejected; modifications required.
- **Routing Decision Tree**:
  - When `CHANGES_REQUESTED` is emitted by a reviewer subtask: the parent task in the development stage is unlocked, and its status resets to `idle` for the developer to address feedback.
  - When `APPROVED` is emitted: the task advances according to `routes.on_approved` or moves to the next sequential column in the pipeline.
  - Upon reaching a column with `terminal: true` (or the final `ready` stage), the task is marked as `done`.

### 2.8 Authoritative Standard Tools Catalog & Dynamic Custom Tools
- **Complete Standard Tools Catalog**: Defined in Rust engine (`src-tauri/src/agent.rs`) and governed by the two-level tool policy matrix:
  - **Pipeline Navigation**:
    - `advance_task` / `move_task`: Transitions task to next sequential stage or specific `stage_id` / `target_stage`. Accepts `summary` parameter injected via Relay Handoff. Emits `agent:autorun` on stage entry.
    - `reject_task`: Rejects task with `reason` parameter, returning card to prior pipeline stage and unlocking it.
  - **Subagent Delegation Hierarchy**:
    - `subagent` (Sync Oneshot): Executes focused child task synchronously in-process without moving cards or creating worktrees; blocks until completion and returns report.
    - `subagent_bg` (Async Background): Spawns asynchronous child task returning immediately with `subagent_id`.
    - `collect_subagent`: Awaits completion of one or more background subagents without busy-polling.
  - **Human-in-the-Loop & Planning**:
    - `question` / `ask_user`: Prompts user with interactive query (`text`, `options`, `multiple`), suspending execution until answered.
    - `todo_write`: Records and updates tactical execution checklist (`todos: [{content, status}]`).
  - **Worktree File & Shell Operations**:
    - `read_file`: Reads UTF-8 text file with line numbers (`file_path`, `offset`, `limit`).
    - `write_file`: Writes complete UTF-8 file content (`file_path`, `content`).
    - `edit_file` (alias: `str_replace`): Precise text replacement (`file_path`, `old_string`, `new_string`, `replace_all`).
    - `glob`: Finds files by pattern within active worktree (`pattern`, `path`).
    - `grep`: Searches file contents by regex within active worktree (`pattern`, `path`).
    - `execute_command` (alias: `bash`): Runs shell command with piped stdio (`command`, `timeout_seconds`).
  - **External Research**:
    - `web_search`: DuckDuckGo web search for APIs, documentation, and error solutions (`query`, `limit`).
    - `web_fetch`: Fetches URL content converted to clean Markdown (`url`).
- **Strict Role Boundary on Task Card Operations**:
  - `create_task`, `create_tasks`, and `complete_task` are **strictly reserved for Root Orchestrators (Level 0), human users via UI, and Sidecar Copilot**.
  - **Development Subagents and cvh-dev are STRICTLY FORBIDDEN from calling `create_task`, `create_tasks`, or `complete_task`**. Subagents work inside assigned tasks and advance stages via `advance_task` / `move_task`.
- **Dynamic Custom Tools (`.cvh/tools/`)**:
  - Extensible JSON descriptors in `.cvh/tools/<tool_name>.json` automatically expose custom CLI tools with argument interpolation (`{{args}}`).
- **Tool Output Spilling Invariant**:
  - Outputs exceeding 100 lines or 8 KB are automatically written to `.cvh/spill/<timestamp>-<tool>.log`, returning only head (20 lines) and tail (20 lines) to preserve model context.

### 2.9 Zero-Turn Memory Priming & Relay Handoff Invariant
- Personas are defined in `.cvh/agents/<role>.md` (or `.cvh/agents/<role>/persona.md`).
- When spawning an agent, Rust pre-compiles:
  `SystemMessage = Role Persona + Project Rules + Skills + Task Prompt`
- The agent starts Turn 1 with 100% memory primed. Never ask agents to read rule files during their turn.
- **Relay Handoff Memory Priming**: When tasks transition between pipeline stages via `advance_task(summary)` or `reject_task(reason)`, the handover summary is recorded in `task.handover_summary` and injected as a top-level prompt banner (`### [ЭСТАФЕТА ПАЙПЛАЙНА / RELAY HANDOFF]`) in `initial_user_prompt`. Subsequent agents inherit exact context and findings of earlier stages with zero context degradation.

### 2.10 State-Preserving Context Compactor & Immutable Anchors
- **Zero-LLM Fast Evaporation**: Tool outputs older than the last 2 turns exceeding 500 characters are automatically trimmed to head summaries.
- **Deterministic Prefix Caching**: System messages and tool schemas maintain deterministic alphabetical ordering with zero dynamic timestamps or session IDs in the prompt prefix to maximize provider prompt cache hit rates (>90%).

### 2.11 Ponytail Minimalism & Anti-Slop Code Standards
- **Zero-Dead-Code Invariant**: No placeholder functions, no unused imports, no unreachable branches.
- **Strict Error Handling in Rust**: Zero naked `.unwrap()` calls in production paths; use `Result<T, AppError>`, `anyhow`, or `thiserror`.
- **Rust HTTP Header Typing**: Header keys in `HeaderMap` MUST implement `IntoHeaderName`. Use `HeaderName::from_static(...)` or `&'static str`; never pass `HeaderValue` as a key.
- **Anti-Slop UI Standard**: Tactile obsidian dark theme (`#07090e`, `#0d1117`, `#161b22`), monospace numerals (`tabular-nums`) for timers/metrics, and zero pastel gradients or generic illustrations.

### 2.12 Zero-Credential Leak, Auto-Gitignore & Local Environment Invariant
- **Zero Secrets in Git**: Secrets and local endpoints reside strictly in gitignored files (`.cvh/llm.json`, `.env*`, `*.secret`, `*.key`) or runtime UI settings. Default API keys evaluate to empty strings (`String::new()`, `""`).
- **Auto-Gitignore Runtime Policy**: Implemented in Rust backend (`ensure_cvh_scaffold` in `src-tauri/src/config.rs`). On startup and workspace initialization, the backend checks root `.gitignore` and automatically appends `.cvh/llm.json`, `llm.json`, and `.cvh/worktrees/` if missing, preventing accidental credential commits or tracking of transient task worktrees.
- **Multi-Tier Workspace Root Discovery**: GUI launches locate the project root via:
  1. Current directory inspection for `.cvh` or `.git`.
  2. Ancestor directory traversal.
  3. `%LOCALAPPDATA%/com.voimax.cvh/active_project.txt`.
  4. Automatic persistence upon opening new workspaces.

### 2.13 Viewport Overlay & Flex Boundary Invariants
- **Root-Level Overlay Hoisting**: All full-screen overlays, drawers (`AgentConsoleDrawer`), modals, and splash screens (`SplashScreen`) using `position: fixed` MUST be rendered at the root React fragment level outside any zoomed (`transform`, `zoom`) or flex-row layout containers. Anchoring them inside transformed containers breaks viewport clipping and containment.
- **Defensive Flex Bounding (`min-h-0` / `min-w-0`)**: Every flex-col or flex-row container hosting scrollable streams, activity feeds, or diff sidebars MUST enforce `min-h-0`, `min-w-0`, and `overflow-hidden` at all intermediate layers. Without this, dynamic child content expands default `auto` dimensions, pushing headers and steer bars off-screen.
- **Zero-Unilateral-Feature-Deletion**: When resolving UI regressions or restyling themes, subagents MUST NOT delete functional components (e.g. bootloader sequence, gauges). Mismatched themes must be restyled, not pruned.
- **Staggered Effect Timer Cleanup**: All nested asynchronous timers (`setTimeout` within callbacks) MUST have their handles hoisted and explicitly cancelled in the `useEffect` cleanup return to prevent unmounted memory leaks.

### 2.14 Multi-Tag Reasoning Stream & Transient UI State Invariants
- **Multi-Block Reasoning Tag Parsing**: When parsing reasoning tags (`<thinking>`, `<reasoning>`) from LLM streaming outputs, parsers MUST use global iterative extraction (`/<thinking>([\s\S]*?)<\/thinking>/gi` with `while (exec)`) rather than single-match (`.match()`). Advanced reasoning models frequently generate multiple fragmented or alternating thought blocks.
- **Transient UI Confirmation State Lifecycle**: Any transient user confirmation timer (e.g. `setConfirm(true) -> setTimeout(false, 3000)`) MUST store its handle in a `useRef` and enforce defensive double-cleanup: explicitly cancel any pending timer before arming a new one, and cancel during component unmount in `useEffect` cleanup to prevent unmounted state updates.
- **Model Display Name Regex Hygiene**: Model name stripping regexes (such as 8-digit date suffixes like `-20250219`) must strictly use escaped digit classes `-\d{8}$` rather than literal `-d{8}$`.

### 2.15 Cross-Toolchain Windows Linker Flag Invariant (`build.rs`)
- **Target Environment Discrimination**: In `build.rs`, platform-specific linker arguments (such as `/MANIFESTDEPENDENCY`, `/ENTRY`, `/SUBSYSTEM`) MUST inspect `CARGO_CFG_TARGET_ENV == "msvc"`, not merely `cfg(windows)`. The GNU toolchain (`windows-gnu` / MinGW GCC `ld`) treats MSVC flags as unrecognized options, causing immediate build failures. Always use:
  ```rust
  let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
  let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
  if target_os == "windows" && target_env == "msvc" {
      println!("cargo:rustc-link-arg=/MANIFESTDEPENDENCY:...");
  }
  ```

### 2.16 CVH-DEV & Copilot Scope Invariant: Zero-Card-Management & Authoritative Tool Policy Matrix
- **Zero-Card-Management Invariant**: The `cvh-dev` persona and CVH Copilot Sidecar Dock are strictly Project Configuration & Environment Specialists (`.cvh/` infrastructure, boards, pipelines, agent personas, rules, and hooks). They are strictly forbidden from creating, completing, or managing task cards (§ 2.8). Card lifecycle is strictly reserved for the human user and the pipeline orchestrator.
- **Authoritative Tool Policy Matrix Decider**: Tool permissions for all agents (including `cvh-dev`, `developer`, `reviewer`) are governed authoritatively by the Two-Level Tool Policy Matrix (`tool-policy.json`) and per-agent setting (`setting.allowed_tools`). If a tool is enabled in `tool-policy.json`), the agent is permitted to use it. Hardcoded role-based tool whitelists are prohibited.
- **Strict Role Boundary on Task Card Operations (§ 2.8)**: Task card manipulation tools (`create_task`, `create_tasks`, `complete_task`) are strictly reserved for Root Orchestrators (Level 0), human users via UI, and Sidecar Copilot. Development subagents, reviewer subagents, and `cvh-dev` are strictly forbidden from calling `create_task` or `complete_task`.
- **UI Labeling Consistency**: UI surfaces for Copilot (`CopilotDock.tsx`, topbar badge, placeholders, empty states) MUST reflect "НАСТРОЙКА ПРОЕКТА" / "Project Configuration", never mislabeling configuration sidecars as general task creators.

### 2.17 Sidecar Copilot Dock Architecture
- **Persistent Companion Control Plane**: The Sidecar Copilot Dock (`CopilotDock.tsx`, `copilotStore.ts`) resides as a collapsible, resizable panel docked to the right edge of the screen, providing real-time architectural peer consultation, codebase exploration, and multi-board supervision alongside the Kanban board.
- **White Paper / Vercel Tactical Aesthetic**: Features a high-contrast White Paper design system (`bg-white`, left border 1px `#ebebeb`, graphite ink typography `#171717`, zero dark slop) intentionally differentiating the meta-architectural companion from the obsidian dark Kanban console.
- **SQLite Dialogue Persistence Invariant**: All Copilot messages, user prompts, thinking blocks, and tool executions persist in SQLite at `<repo_root>/.cvh/harness.db` under dedicated ID `task_id = "copilot:meta-chat"` using WAL mode, loaded with `load_copilot_history` and cleared with `clear_copilot_history`.
- **Independent ReAct Loop & Event Streaming**: Operates its own autonomous ReAct loop (`src-tauri/src/commands/copilot.rs`) emitting `copilot:stream` events with token streaming, multi-block thinking cards, tool call visualization, and abort capability (`abort_copilot`).
- **Dedicated Permitted Tool Suite**: Pre-compiled with `get_copilot_tool_suite()` containing permitted configuration tools:
  - Configuration & inspection tools: `read_file`, `write_file`, `edit_file`, `glob`, `grep`, `execute_command`, `web_search`.

### 2.18 Board Self-Healing Runtime Subsystem
- **Autonomous Task Healing Invariant**: The Rust desktop backend automatically heals tasks against board configurations (`heal_task_board_and_column` and `heal_tasks` in `src-tauri/src/commands/tasks.rs`).
- **Orphaned Board Resolution**: If a task's `board_id` is missing, empty, or references a deleted board, the harness dynamically assigns it to `active_board_id` (or the first existing board, or `"development"`).
- **Orphaned Column Resolution**: If a task's `stage_id` does not exist in the target board's columns, it is dynamically reassigned to that board's first column (typically `"backlog"`).
- **Healing Lifecycle Execution**: Self-healing runs autonomously during task loading (`load_tasks`), board deletion, board switching, and task imports, preventing broken references, orphan cards, or frontend crashes.

### 2.19 Dark Theme & Deep Token Propagation Invariant
- **Deep Control Styling Invariant**: Any UI component supporting dark theme MUST NOT stop at top-level container backgrounds. All nested form controls (`<input>`, `<select>`, `<textarea>`, `<option>`), context submenus, and dialog panels MUST declare paired dark tokens (`dark:bg-[#0d1117]`, `dark:bg-[#161b22]`, `dark:border-[#30363d]`, `dark:text-[#f0f6fc]`). Hardcoded light tokens (`#ffffff`, `#fafafa`, `#ebebeb`) without dark overrides are strictly forbidden.
- **Defensive View Transitions API Fallback**: Signature animations utilizing the CSS View Transitions API (`document.startViewTransition`) MUST include graceful fallbacks that immediately toggle state when the API is unsupported (e.g. non-Chromium or older webviews) or when the user prefers reduced motion (`prefers-reduced-motion: reduce`).
- **Pre-Review TypeScript Sensor Gate**: Before requesting Level 2 Review on multi-file UI refactors, the lead developer MUST execute `tsc --noEmit` inside the isolated worktree to catch unbalanced JSX tags or missing props early, preserving high First-Time-Right (FTR) execution.

### 2.20 Optimistic In-Chat Feedback, Synchronized Tool ID & Native Tooltip Ban Invariants
- **Immediate Optimistic In-Chat Feedback Invariant**: User-initiated operations that trigger background processing or asynchronous IPC (such as context compaction, manual steering, or custom tool runs) MUST immediately (<16ms) inject an optimistic execution item with status `'running'` into both `activityFeed` and `dialogItems` before awaiting backend completion. Users must never be left staring at an unresponsive button.
- **Deterministic Tool ID Synchronization Invariant**: When an optimistic tool card is generated on the client, the client MUST generate a unique, deterministic tool identifier (e.g., `compact_context-${taskId}-${Date.now()}`) and pass it across IPC (`tool_id: Option<String>`). The backend MUST adopt this ID for all lifecycle events (`agent:tool_call`, `agent:tool_result`) and persist it into SQLite `dialog_items`. When the frontend subsequently reloads dialogue history (`load_task_dialog`), the record reconciles under the exact same key with zero card duplication, zero ID mismatches, and zero visual flickering.
- **Store-Level In-Place Tool Deduplication**: Store insertion methods (`addToolExecution`, `injectOptimisticCompactTool`) MUST verify if an item with the matching tool ID already exists in `toolExecutions` or `activityFeed` and update it in-place rather than naively appending or prepending duplicate items.
- **Native Browser Tooltip Ban on Labeled Controls**: Native HTML `title="..."` attributes are strictly prohibited on interactive buttons and controls that already display text labels or self-explanatory iconography. In desktop webviews (WebView2/Tauri), native tooltips render unstyled, delay-lagging OS popups that linger over click targets, obstruct adjacent controls, and break the obsidian dark aesthetic.
- **Unified Action Trigger Delegation**: Secondary trigger points for primary actions (such as triggering compaction from `ContextGauge`) MUST delegate through the canonical store action (`compactTaskContext`) rather than issuing direct backend IPC calls, ensuring that optimistic UI feedback, duration measurement, and error handling remain uniformly applied.

### 2.21 Task Model Override Priority, 100% Compaction Ceiling & Persona Policy Sync
- **Strict Model Resolution Hierarchy**: When determining the active model for task execution (`resolve_task_active_agent_model`), priority is ironclad:
  1. `task.model` (explicit user or agent task override) strictly takes precedence.
  2. `column.model_override` (stage persona configuration) is used if `task.model` is empty or None.
  3. `llm_settings.model` (global configuration) is the ultimate fallback.
  Subtasks inherit the parent task's explicit model override before falling back to target stage column overrides (`parent_model.or(target_col.model_override)`).
- **Atomic Task Updates Across Lifecycles**: Updates to task model overrides via UI or agent operations MUST be committed immediately via dedicated `update_task` Tauri IPC and written directly to `.cvh/tasks.json` with event emission (`task:updated`, `tasks:updated`). In `run_agent`, tasks are loaded from disk first to ensure the latest model overrides and steer instructions are applied.
- **100% Context Window Compaction Ceiling**: Automatic state-preserving context compaction MUST trigger strictly when current estimated tokens reach or exceed 100% of the active agent persona's `context_window` (`current_tokens >= compaction_threshold`). Premature, aggressive thresholds (such as 70% or arbitrary >20,000 token turn triggers) are strictly prohibited, ensuring models utilize their full available context buffer without premature token degradation.
- **Bi-Directional Persona Frontmatter Tool Policy Synchronization**: Whenever tool policies are updated (`save_tool_policies`), the backend MUST autonomously synchronize the effective allowed tools into the YAML frontmatter (`tools: [...]`) of `.cvh/agents/<role>/persona.md`. This guarantees prompt instructions, runtime schema definitions, and system prompts remain 100% consistent, eliminating tool hallucinations and cognitive mismatch.

### 2.22 Optimistic Kanban Card Lifecycle & Symmetric State Invariants
- **Instant Optimistic Card Lifecycle (<1ms)**: `addTask`, `createSubtask`, and `deleteTask` in `boardStore.ts` MUST update Zustand state immediately (<1ms) so cards appear or vanish instantaneously on the Kanban board without blocking on backend IPC or disk I/O. Background IPC operations (`create_task`, `create_subtask`, `delete_task`), worktree initialization/cleanup, SQLite dialogue clearing, and disk persistence run asynchronously and silently reconcile state in-place without wholesale board reloads (`loadTasks`).
- **Symmetric Parent-Subtask Lifecycle & Auto-Unlock Invariant**: Any operation modifying or deleting a subtask MUST maintain relational symmetry with its parent task. When a subtask is deleted (in UI or backend), any parent task waiting on that subtask (`waitingForSubtaskId == deletedId` or `status == 'waiting_subtask'`) MUST automatically clear `waitingForSubtaskId` and reset its status to `'idle'`. If a subtask ID drifts during backend reconciliation, the parent's `waitingForSubtaskId` MUST be updated atomically in the same reconciliation pass.
- **Fine-Grained Board Subscription & React.memo Optimization**: Kanban cards (`KanbanCard.tsx`) and columns (`KanbanColumn.tsx`) MUST NOT subscribe wholesale to `useBoardStore()`. Interactive cards and columns MUST use selective Zustand selectors (`useShallow`, atomic boolean/property getters) and be wrapped in `React.memo` with custom shallow comparators (`areCardPropsEqual`, `areColumnPropsEqual`) to prevent cascading board re-renders on hover or timer updates. Callback handlers (`handleDeleteColumn`, `savePipelineConfig`) MUST maintain stable references via `useCallback` and `useBoardStore.getState()`.

### 2.23 Git Worktree Lifecycle, Pruning & Clean Teardown Invariants
- **Symmetric Git Worktree Deregistration Invariant**: Worktrees MUST NEVER be removed solely by physical directory deletion (e.g. `rmdir`, `rm -rf`, or `cargo clean` inside `target/`). Physical directory removal without git unregistration leaves dangling administrative references in `.git/worktrees/<slug>`, corrupting `git worktree list` with `prunable` warnings.
- **Two-Step Worktree Teardown Protocol**: When retiring a completed task or tearing down test worktrees, agents and test harnesses MUST execute:
  1. `git worktree remove <path> --force` (deregisters the worktree and cleans the physical directory).
  2. `git worktree prune` (defensively purges any stale or dangling gitdir pointers from `.git/worktrees/`).
  3. `git branch -D <branch>` (purges merged feature branch).
- **Test Worktree Isolation Policy**: Any automated test suite or script creating transient worktrees (such as `src-tauri/target/.../.cvh/worktrees/`) MUST register an explicit `Drop` or teardown hook that runs `git worktree prune` and deletes transient branches, guaranteeing zero metadata pollution in repository git status.

### 2.24 Telemetry Accuracy & Non-Decaying TPS/RPM Metric Invariant
- **Active Generation Time Bounding (TTFT Separation)**: Token generation speed (TPS / tok/s) MUST be measured strictly across active token generation (from `first_token_instant` to `last_token_instant`), completely excluding pre-generation reasoning setup, network latency, or Time-To-First-Token (TTFT) from output throughput calculations.
- **Ironclad Non-Decaying TPS Invariant**: Generation speed (TPS) MUST freeze instantaneously upon generation finish (`AgentEvent::Telemetry`, tool call interception, or agent finish) and persist in reactive store state (`finalTps`). The UI is STRICTLY FORBIDDEN from calculating live TPS using continuously advancing `Date.now() - startTime` once generation ceases, preventing metrics from "melting" or decaying towards zero.
- **Stream Options & Resilient Fallback Telemetry**: OpenAI-compatible streaming requests MUST include `stream_options: { include_usage: true }`. If a provider or local proxy fails to return an explicit usage chunk, the backend agent loop MUST autonomously calculate heuristic fallback telemetry (`estimate_text_tokens` + duration) and emit `AgentEvent::Telemetry`, guaranteeing the UI never displays dead dashes (`—`).
- **Safe Prompt Cache Hit Rate Computation**: Cache hit percentages MUST be calculated defensively with division-by-zero guards: `safeCacheHitRate = total_input > 0 ? (cache_read / total_input) * 100.0 : 0.0`. NaN, infinite, or negative values are strictly prohibited.
- **RPM (Requests Per Minute / Step Rate) Cadence Invariant**: Task execution cadence is tracked as `RPM = (effectiveTurns / (totalTaskSeconds / 60))` using Russian pluralization ("шаг", "шага", "шагов") and monospace numerals (`tabular-nums`), giving operators instantaneous insight into agent step velocity without UI jitter.
- **Hierarchical Context Gauge Token Priority**: Token estimation in `ContextGauge` MUST strictly obey the resolution cascade:
  1. Explicit `currentEstimatedTokens` prop.
  2. In-memory compaction override (`tokenOverrides`).
  3. Compacted tokens from persisted dialogue feed.
  4. Verified server telemetry `inputTokens` + streamed turn output tokens (true model prompt context).
  5. Heuristic character estimation fallback.

### 2.25 Drawer State Hygiene, Context Resolution & Popover Contrast Invariants
- **Unconditional Drawer Memory Eviction on Task Deletion**: When deleting a task (`deleteTask` in `boardStore.ts`), `clearTaskLogs(id)` MUST be invoked unconditionally for every deleted task and cascaded subtask ID prior to backend IPC calls. Placing log clearance inside fallback or error catch blocks is strictly prohibited, preventing silent accumulation of orphaned dialogue history, activity feeds, and token telemetry in Zustand store memory.
- **Defensive Zombie Drawer Auto-Close Invariant**: If the active task currently open in `AgentConsoleDrawer` (`drawerStore.activeTaskId`) is among the deleted tasks or subtasks, `useDrawerStore.getState().closeDrawer()` MUST execute immediately to avoid rendering stale, orphaned, or unmounted state in the console overlay.
- **Empty Dialogue Load State Reset Invariant**: In `drawerStore.ts` (`loadTaskDialog`), if retrieved dialogue history is empty (`items.length === 0`), the store MUST explicitly reset all transient task state (`activityFeed = []`, `streamEntries = { thoughts: [], outputs: [] }`, `toolExecutions = []`, `tokenOverrides = undefined`, `cumulativeOutputTokens = 0`, `cumulativeGenerationSec = 0`). Failure to reset creates ghost streams and inaccurate context measurements when inspecting fresh tasks.
- **Hierarchical Context Window Resolution Invariant**: `ContextGauge` limit resolution MUST prioritize:
  1. Explicit `contextWindow` prop passed from parent (e.g. `DrawerSteerBar`).
  2. Active agent persona context window resolved strictly from agent persona setting (`.cvh/agents/<role>/setting`, e.g. via `get_task_context_window` or persona setting).
  3. Safe fallback `stageLimit` (default 128,000).
- **Decoupled Context Window & Persona Single-Source-of-Truth Invariant**: Boards and pipeline columns MUST NOT define, store, or serialize `contextWindow`. The context window is an intrinsic invariant strictly belonging to the agent persona (`.cvh/agents/<role>/setting`). Resolving the context window for task cards, subtasks, or the drawer MUST query `get_task_context_window(task_id, persona)` or persona settings metadata, dynamically adapting when subagents execute without coupling to board columns.
- **Proportional Context Breakdown & Message Preservation**: When computing token segment allocations (System prompt, Tool definitions, Messages) in `ContextGauge`, if dialogue messages exist, tokens MUST be distributed proportionally across all three segments when total tokens are below base threshold, never squashing `messageTokens` to zero.
- **Smooth Multi-Turn Cumulative TPS/RPM Averaging**: In multi-turn tasks, tokens-per-second (tok/s) MUST track both the latest turn's frozen TPS and the weighted cumulative average across all turns (`cumulativeOutputTokens / cumulativeGenerationSec`), eliminating erratic metric spikes on micro-turns (e.g. 1-token responses) and preserving honest execution cadence.
- **Dedicated Context Popover & Deep Dark Mode Contrast**: The context gauge hover card MUST remain focused exclusively on context breakdown and compaction actions, avoiding redundant 2x2 telemetry duplicates. All popover text, rings, and badges MUST enforce paired dark mode classes (`dark:text-[#f0f6fc]`, `dark:text-[#8b949e]`, `dark:stroke-[#21262d]`, `dark:text-[#f59e0b]`) with zero unpainted light tokens.

### 2.26 Persistent Monotonic Task ID & Platform-Agnostic CRLF Normalization Invariants
- **Persistent Monotonic Task ID Invariant**: Task numeric sequences MUST be persisted to disk (`.cvh/settings/sequence.json` and mirrored in `.cvh/task-sequence.json`) and incremented monotonically (`allocate_next_task_num`). Task ID allocation MUST NEVER derive solely from in-memory or on-disk array maximums (`max_num_on_disk + 1`), guaranteeing that deleted tasks never have their numerical IDs re-issued or recycled under any circumstances.
- **Symmetric Deletion Broadcasting & Worktree Pruning Invariant**: The `delete_task` Tauri IPC command MUST accept `app_handle: AppHandle` and broadcast `tasks:updated` across the application upon task removal. Additionally, task deletion MUST execute full cleanup: removing worktrees (`remove_worktree`), pruning dangling gitdir pointers (`prune_worktrees`), deleting feature branches, and clearing associated SQLite dialogue items.
- **4-Tier Progressive CRLF/LF Normalization Invariant**: The `edit_file` (alias: `str_replace`) tool MUST implement progressive line ending normalization (`replace_text_normalized`) accommodating Windows CRLF (`\r\n`) vs Unix LF (`\n`) discrepancies. When LLMs supply search or replacement strings containing standard Unix LF, the engine MUST adapt line endings to match the target file without failing with false `NOT_FOUND` errors, while strictly preserving the file's original line ending convention upon saving.

### 2.27 Stage Transition Natural Agent Loop Termination Invariant
- **Natural Termination on Stage Transition**: When any executed tool call is `advance_task`, `move_task`, or `reject_task` (or aliases `move_task_stage`, `advance_stage`, `next_stage`), the agent has handed over or finalized the task. In `run_agent_loop`, the engine MUST:
  1. Record `ChatMessage::tool_response` into the conversation history.
  2. Synchronize messages to in-memory context state in `AppState` (if present).
  3. Emit `AgentEvent::Finished { final_response: result_str.clone() }`.
  4. Break the ReAct loop and return `Ok(result_str)` naturally and cleanly.
  5. Under NO circumstances send the result back to the LLM for an extra turn.

### 2.28 Resilient Line Matching & Honest Edit Diagnostics Invariants
- **5-Tier Progressive Text Replacement Hierarchy**: The `edit_file` (alias: `str_replace`) tool MUST execute progressive matching in strict order:
  1. Exact byte matching (`current.contains(old_string)`).
  2. CRLF adaptation (file has CRLF, LLM supplies LF).
  3. Reverse CRLF adaptation (file has LF, LLM supplies CRLF).
  4. Normalized line endings fallback (normalizing both to LF and restoring file's original convention upon write).
  5. Whitespace-relaxed line matching (`replace_whitespace_relaxed`): trimming leading and trailing whitespace on each line, adapting indentation differences proportionally, and preserving file newline endings.
- **Greedy Non-Overlapping Replacement Invariant**: In `replace_whitespace_relaxed`, when `replace_all: true`, match advancement MUST step forward by the full matched block length (`idx += m`) rather than single lines (`idx += 1`), preventing corrupt overlapping replacements or duplicate substitutions.
- **Ambiguous Match Rejection**: If `replace_all: false` and multiple blocks match under relaxed whitespace comparison, the tool MUST reject the edit with an informative `Ambiguous match` error requiring `replace_all: true` or wider surrounding context.
- **Honest Mismatch Diagnostics & Zero-Fake-100% Invariant**: In `build_edit_mismatch_error`, similarity MUST NEVER be calculated from a single line when `old_string` spans multiple lines:
  1. Candidate matching MUST use sliding window multi-line block similarity averaged across all search lines.
  2. If any line or token in the candidate block differs from `old_string`, the reported similarity MUST NEVER reach or exceed 100% (`best_score.clamp(0.0, 99.0)`).
  3. Diagnostics MUST output a structured line-by-line diff classifying lines as `EXACT MATCH`, `WHITESPACE MISMATCH`, or `MISMATCH`.
  4. Mismatched lines MUST include token-level LCS diff diagnostics (`diff_line_tokens`) pinpointing expected vs actual tokens, missing tokens, or unexpected tokens.
  5. The error message MUST output the candidate file block with `>>` margin markers and line numbers, followed by pre-computed `read_file(offset, limit)` recommendations to facilitate immediate LLM self-correction in Turn N+1.

### 2.29 Reactive Agent Running State & Honest Terminal Response Invariants
- **Multi-Source Reactive Liveness Invariant**: The UI running state (`isRunning` in `AgentConsoleDrawer` and action controls) MUST NOT rely exclusively on `task.status === 'running'`. It MUST reactively evaluate multi-source liveness: `task.status === 'running' || hasRunningTool || isAgentActive`, where `hasRunningTool` tracks active tool executions across both parent task and active subagent, and `isAgentActive` tracks reactive store flags (`activeAgentTaskIds`). This guarantees the Stop button remains active and responsive whenever the backend is performing work.
- **Symmetric In-Flight Tool Abort Teardown Invariant**: When an agent task or subagent is aborted by the user (`handleAbortAgent` / `abort_agent`), the client MUST immediately sweep all in-flight tools (`status === 'running'`) across both the parent task and active subagent, marking them as `failed` with diagnostic reason `"Прервано пользователем"` to eliminate dangling spinners and misleading UI indicators.
- **Terminal Response vs Intermediate Commentary Differentiation Invariant**: In streaming activity feeds (`DrawerActivityStream`), assistant output items MUST NOT be unconditionally rendered as the final collapsible response card (`"✨ Ответ агента"`). Output preceding or intervening between tool calls represents intermediate intent or step explanation and MUST be rendered unboxed as natural step commentary (`"Пояснение агента"`). Only the true terminal output (explicitly flagged `isFinal: true` or terminating without subsequent tools/turns) may be rendered as the prominent terminal response card.
- **Intervening Tool Output Preservation**: In `markFinalResponse`, if tool executions intervened after the last streamed output chunk, the store MUST NOT mutate or overwrite the prior commentary; it MUST append a new terminal response item preserving chronological fidelity.

### 2.30 Initial Fresh-Project 3-Column Board, cvh-dev Persona & 200k Context Capacity Invariants
- **Lean 3-Column Default Architecture Invariant**: In fresh projects and default configurations (`scaffold_cvh_project`, `.cvh/boards.json`, `.cvh/pipeline.json`, `DEFAULT_BOARDS_JSON`, `DEFAULT_PIPELINE_JSON`), the primary development board MUST default to an agile 3-column topology:
  1. `backlog` (Intake stage, index 0, persona: none / system intake).
  2. `dev` (Active development & environment configuration stage, persona: `"cvh-dev"`, stage name: `"Разработка"` / `"Development"`).
  3. `ready` (Completion stage, final index, `terminal: true`, stage name: `"Готово"` / `"Ready"`).
  The structurally immutable columns (`backlog` and `ready`) anchor intake and delivery, while `dev` serves as the single active autonomous execution stage for fresh projects.
- **Unified cvh-dev Development Persona Invariant**: The default development column (`dev`) MUST bind to the `cvh-dev` persona (`persona: "cvh-dev"`), ensuring immediate alignment with project configuration, codebase modification, tactical planning (`todo_write`), subagent delegation, and environment inspection capabilities.
- **200,000 Token Persona Context Window Invariant**: The default context window capacity for `cvh-dev` MUST be configured to 200,000 tokens (`context_window: 200000` in `.cvh/agents/cvh-dev/setting`). On project scaffold or initialization (`scaffold_cvh_project`), if `context_window` is absent or below 200,000 tokens (`val.get("context_window").and_then(|v| v.as_u64()).is_none_or(|w| w < 200000)`), the backend MUST automatically ensure and seed 200,000 tokens, providing ample context headroom for complex multi-turn development sessions.
### 2.31 Steer Context Preservation, SQLite Persistence & Tool Call Sanitization Invariants
- **Multi-Turn In-Memory Context Continuity**: When steering an active or idle agent via `steer_agent` or `run_agent(task_id, Some(steer_prompt))`, the ReAct loop in `run_agent_loop` MUST NOT wipe existing conversation messages with a fresh 2-message array (`system + user`). The engine MUST first check `AppState::agent_contexts` for existing messages (>2); if absent (e.g. after desktop app restart), it MUST reconstruct the full transcript from SQLite `dialog_items` (`reconstruct_messages_from_dialog`).
- **Idempotent In-Memory Context State Retention**: In `run_agent`, initializing `state.agent_contexts` MUST use `ctx_map.entry(task_id).or_insert_with(...)` rather than unconditional `insert(..., AgentContextState::default())`, guaranteeing that active or pre-existing conversation context is never wiped during steer re-entry.
- **Symmetric SQLite Steer Persistence**: Whenever a user steer is submitted via `steer_agent` or injected via `run_agent_loop`, the steer directive MUST be persisted to SQLite `.cvh/harness.db` with item type `"steer"` via `append_task_dialog_activity`. This guarantees steer history survives page reloads, drawer closes, and application restarts.
- **Strict LLM Tool Call Schema Sanitization Invariant**: When an agent turn is aborted mid-execution (via Stop button or user steer), any assistant message with unclosed `tool_calls` lacking matching `role: "tool"` responses MUST be sanitized via `sanitize_unclosed_tool_calls` prior to submitting subsequent requests to LLM APIs. The sanitizer appends synthetic tool responses indicating the interruption (`"[Инструмент прерван указанием пользователя / Tool execution interrupted by user steer]"`), preventing provider HTTP 400 Bad Request schema validation failures.
- **Real-Time Context Synchronization on Abort**: On any user abort or cancellation checkpoint during generation or tool execution, the engine MUST immediately call `sync_context_state` and `sanitize_unclosed_tool_calls` to flush the latest turn count and messages to `AppState::agent_contexts`, ensuring Turn N+1 inherits accurate state.

### 2.32 Subagent Natural Lifecycle, Autonomous Report Synthesis & Symmetric Steer Deduplication Invariants
- **Natural Subagent Lifecycle & Hard-Limit Ban**: Subagents executing via `run_inprocess_subagent` MUST run their natural ReAct loop until task completion (`turn_tool_calls.is_empty()`) or explicit user cancellation (`ctx.abort_handle`). Artificial step limits (such as `sub_turn > 15`) are strictly prohibited, ensuring deep exploratory subtasks can complete naturally without premature severance.
- **Autonomous Subagent Report Synthesis**: If a subagent completes its tool executions without emitting a textual terminal response (`final_sub_response.trim().is_empty()`), the engine MUST autonomously execute a final synthesis turn with `tools: None` to force the LLM to generate a comprehensive findings report. If synthesis fails, the engine MUST collect the most recent tool and assistant outputs from conversation history. Under NO circumstances may a subagent return an empty placeholder (`"Work complete"` without findings).
- **3-Tier Subagent Collection Resolution**: The `collect_subagent` tool MUST resolve subagent reports across three cascading tiers: (1) `SUBAGENT_RECEIVERS` for active background subagents, (2) `SUBAGENT_SESSIONS` (`info.report`) for completed synchronous or background sessions, and (3) SQLite `dialog_items` for persisted subagent outputs via `find_subagent_report_from_db`. Collecting synchronous or previously completed subagents MUST NEVER crash or fail with "not found in background registry".
- **Subagent Todo Persistence**: When `todo_write` executes inside a subagent (`task_id` contains `".sub-"`), the checklist MUST be persisted into SQLite `dialog_items` under `subagent_dialog_id` (`{parent_id}:sub:{subagent_id}`) with item type `"todos"` and broadcast to the UI via `subagent:activity`, rather than dropped due to absence from `tasks.json`.
- **Symmetric Autorun Prompt Decoupling & Steer Deduplication**: Card movements (`moveTask`, `createSubtask`, `addTask`) and stage autoruns MUST NOT inject `task.description` into `triggerRunAgent`, ensuring `run_agent` receives `prompt = undefined` (`None` in Rust). If `prompt` matches `task.description`, the backend MUST treat it as `None` (not a steer). User steers in the frontend and backend enforce deduplication against SQLite `dialog_items` within the last 3 items, preventing duplicate steer cards in chat history.

### 2.33 Subagent Report Spilling Bypass, Unconditional Abort & Turn Deduplication Invariants
- **Subagent Report Spilling Bypass Invariant**: Subagent output reports generated by `subagent`, `subagent_bg`, and `collect_subagent` MUST NEVER be truncated, clipped, or spilled to disk in `maybe_spill_output`. The parent agent requires 100% of the synthesized findings to make reasoned decisions in Turn N+1 without losing critical diagnostic data.
- **Modernized Spilling Ceilings**: For generic shell commands and tools (`execute_command`, etc.), the spilling threshold is raised from 100 lines / 8 KB to 500 lines / 64 KB. For `read_file`, outputs do not truncate or spill unless exceeding 2,000 lines or 128 KB, preventing premature file truncation during routine codebase inspection.
- **Unconditional Abort & Child Subagent Sweep Invariant**: `abort_agent` and `internal_abort_agent` MUST stop both the parent task and all active child subagents registered under that task across `state.active_agents`, `SUBAGENT_SESSIONS`, and `SUBAGENT_RECEIVERS`. Moving a task card to `backlog` (or any stage without autorun/persona) MUST automatically trigger `internal_abort_agent` to terminate orphaned background Tokio loops immediately. `run_agent_loop` must check `abort_flag` before starting each turn and before executing every tool call, terminating immediately upon signal.
- **Single-Turn Subagent Deduplication Invariant**: If an LLM emits multiple `subagent` or `subagent_bg` tool calls with identical or virtually identical prompt and role within a single turn, the ReAct loop MUST deduplicate them via `are_subagent_prompts_duplicate`, returning the result of the already started agent without spawning duplicate concurrent tasks or redundant child processes.
- **Authoritative Tool Policy Matrix Invariant (§ 2.16 & § 2.21)**: The Tool Policy Matrix (`.cvh/settings/tool-policy.json`) is the single authoritative source of truth for tool permissions. Persona YAML frontmatter (`tools: [...]`) and `setting.allowed_tools` cannot silently drop tools enabled in the active matrix. Persona frontmatters are synchronized dynamically upon startup and whenever policies are saved. The `cvh-dev` persona MUST have `advance_task` enabled and follow an ironclad completion directive: after completing planned items, output a terminal report and cease tool calls or call `advance_task`.
### 2.34 Subagent UI Polish, Role Iconography, Granular Abort & Anti-Slop Presentation Invariants
- **Dynamic Role Iconography vs Glyph Ban (Anti-Square Invariant)**: In all UI headers, drawer activity streams, breadcrumbs, sidebar items, and tool execution cards, literal unicode placeholder glyphs (`■`, `●`, `▲`, `[]`) are strictly prohibited. Subagent roles and agent personas MUST resolve to canonical Lucide SVG components via dynamic resolver `getRoleIcon(role)` (`Code2` for developer, `ShieldCheck` for reviewer/audit, `Compass` for explorer/research, `Bot` as fallback), maintaining deep visual consistency across light and dark themes.
- **Technical Sub-Call ID Sanitization & Humanized Display Titles**: Internal IPC call markers, temporary execution identifiers, and machine sub-call slugs (`(?:[A-Za-z0-9_-]+\.)?sub-call_\d+`) MUST NEVER leak into user-facing headers, subagent sidebar lists, breadcrumbs, or tool execution cards. All subagent titles and tool parameters MUST pass through `formatSubagentDisplayTitle`, stripping raw call suffixes and humanizing them into clear contextual representations (e.g. `"субагент"`). Redundant raw technical ID badges (such as monospace `{subagentId}` pills alongside task titles) are strictly banned from tool execution cards.
- **Structured Todo Parsing & Proximity Suppression (Anti-JSON-Slop)**: Raw JSON payloads (such as raw arrays `[{...}]` or markdown code fences ```json ... ``` representing task lists) emitted in assistant thoughts or intermediate commentary MUST NEVER be displayed as raw unformatted JSON strings in `AgentExplanationCard`. The activity stream MUST parse candidate payloads via `tryExtractTodos`. If a `todo_write` tool execution is nearby (within a ±3 activity window) and the explanation contains solely the redundant JSON checklist, the explanation card MUST be suppressed entirely (`return null`). When no nearby `todo_write` exists, the parsed items MUST be rendered as an interactive, tactile action checklist (`"План действий"`) featuring `ListTodo` iconography, status checkmarks (`CheckCircle2`, `Clock`, `Circle`), and localized status badges (`"Готово"`, `"В работе"`, `"Ожидает"`).
- **Granular Hierarchical Subagent Abort Invariant**: The drawer and subagent sidebar MUST provide granular, isolated abort capabilities. When an operator triggers an abort while viewing an active subagent (`activeSubagentId`), or clicks the dedicated `"Остановить"` control on an active subagent in `SubagentsSidebar`, the frontend MUST target ONLY that specific subagent via `abort_agent({ taskId: resolvedSubagentId })` (resolving through `toolIdToSubagentId` when applicable). The parent task's ReAct loop, context window, and sibling subagents MUST remain active and unperturbed. Only running tools belonging to the targeted subagent are swept and marked as `failed` (`"Прервано пользователем"`).
- **Anti-Slop Metric Hygiene & Ban on Unformatted ISO Timestamps**: Tool execution cards, headers, and stream item footers MUST NOT display unformatted ISO 8601 timestamp strings (`2026-10-19T...`). Card metrics are strictly limited to actionable execution indicators (`duration_ms` in `tabular-nums`, exit codes, and relative status). All interactive controls, including subagent stop buttons, MUST strictly obey § 2.20 with zero native `title="..."` attributes.

### 2.35 Granular Hierarchical Subagent Abort, Deduplication & Mid-Flight Tool Teardown Invariant
- **Granular Isolated Subagent Abort**: When an abort signal is triggered for a subagent (`is_subagent` evaluated via delimiter presence `.sub-` / `:sub:`, `SUBAGENT_SESSIONS` membership, or `state.active_agents`), the engine MUST terminate ONLY that specific subagent's join handle and abort flag (`internal_abort_agent`). The parent task's ReAct loop, context window, active Tokio loop, and sibling subagents MUST remain running and undisturbed. Root-level task status updates (`task:status` -> `idle`) and cascaded aborts are strictly reserved for parent tasks.
- **Strict Delimiter Boundary in Subagent Key Matching**: Suffix and prefix key resolution in `is_matching_subagent_key` MUST enforce delimiter boundaries (`.sub-{suffix}`, `:sub:{suffix}`) rather than loose substring `ends_with`, preventing false positive matches across subagent IDs sharing numerical or lexical suffixes.
- **Exploratory Persona Deduplication & Report Reuse**: Subagent personas classified as exploratory (`is_exploratory_role`: `"explorer"`, `"architect"`, `"researcher"`, `"scout"`, `"code_explorer"`, `"system_architect"`) are subject to cross-call and single-turn deduplication:
  1. If an exploratory subagent has already completed with a valid report for the same parent task, subsequent requests in `execute_tool` (`subagent` and `subagent_bg`) MUST immediately reuse and return the existing report without launching a redundant agent.
  2. If an exploratory subagent is currently running, synchronous calls return an informative in-flight notice instructing the model to await completion or collect via `collect_subagent`, preventing explosive token expenditure and duplicate investigation paths.
  3. In single-turn ReAct loops, multiple exploratory tool calls are deduplicated in-place, echoing the prior tool result.
- **Mid-Flight Tool Abort Teardown & SQLite Persistence**: When an agent or subagent is aborted, the backend MUST synchronously sweep all mid-flight tool executions (`tool_json` with `status: "running"`) via `mark_running_tools_as_aborted` in SQLite `dialog_items`, atomically updating them to `status: "failed"` with `stderr: "Прервано пользователем"`, and emit `agent:tool_result` events. In `run_inprocess_subagent`, if `abort_handle` trips during or immediately following tool execution, an aborted tool execution card MUST be written to dialogue history before exiting. Dangling tool spinners across restarts or drawer re-renders are strictly prohibited.
- **Graceful Subagent Abort Channel Absorption**: When a child subagent is cancelled mid-flight, its channel receiver error in the parent ReAct loop MUST be absorbed gracefully (`"[Субагент прерван пользователем / Subagent aborted by user]"`) without bailing the parent loop or throwing unhandled errors, allowing the parent orchestrator to synthesize the partial findings or continue execution naturally.

### 2.36 Streaming Auto-Scroll, Honest Tool Failure State & Anti-Duplication Scaffolding Purge Invariants
- **Streaming ResizeObserver Auto-Scroll Invariant**: In streaming activity views (`DrawerActivityStream`), height expansion during intra-item token/thinking generation MUST be tracked with a `ResizeObserver` observing the scroll container's content element (`scrollRef.current.firstElementChild`). When `nearBottomRef.current` is true, the scroll container automatically scrolls to `scrollHeight`. When the user has deliberately scrolled up to inspect earlier logs or code (`nearBottomRef.current` is false), automatic scrolling MUST NOT hijack user viewport position.
- **Honest Tool Card Failure Status Invariant**: Tool execution cards (`ToolExecutionCard`) MUST NEVER render a green success checkmark (`Check`) or declare `status: "success"` if `isFailed` is true (`tool.status === 'failed' || (tool.exitCode !== undefined && tool.exitCode !== 0)`). Cards MUST evaluate `!isFailed && tool.status === 'success'` before showing success state, and format status in card footers as `failed (код ${exitCode})` when failed, preventing misleading green indicators on runtime errors.
- **Anti-Duplication UI Scaffolding Purge**: Legacy, redundant sub-components (such as static child task cards rendered inside the main activity feed when dedicated subagent navigation sidebars and inline tool execution cards exist) MUST be purged, along with unused props in component interfaces, preventing UI clutter and layout bloat.
