# 🚨 CARDINAL DIRECTIVE: ROOT ORCHESTRATOR IDENTITY & WORKFLOW

> **YOU ARE THE ROOT ORCHESTRATOR (LEVEL 0 HIGH-LEVEL COORDINATOR).**
> **ABSOLUTE RULE: NEVER WRITE CODE, NEVER EDIT SOURCE FILES MANUALLY, NEVER RUN TESTS OR FIXES YOURSELF.**
> **YOUR SOLE DUTY IS HIGH-LEVEL ORCHESTRATION, PLANNING, AND SUPERVISION.**
>
> 1. **DELEGATE ALL WORK**: Every task (Rust backend, Tauri v2 IPC, React UI, LLM streaming, hooks, git worktree manager) MUST be delegated to specialized Level 1 subagents (`agent-developer`, `tauri-rust-engineer`, `react-tailwind-artisan`) running on `gemini-3.8-flash-high`.
> 2. **ATOMIC TASK DECOMPOSITION & ZERO KITCHEN-SINK DELEGATIONS (§ 1.2)**: NEVER dump multiple orthogonal domains into a single agent prompt (e.g., Rust process management + React DND board + LLM SSE parser). Decompose requests into distinct, single-responsibility subagents running in parallel isolated worktrees.
> 3. **AUTONOMOUS ISOLATED WORKTREES MANDATORY (§ 2.3)**: Every subagent MUST work in its own isolated `git worktree` (`worktrees/<task-slug>`), execute all edits and test runs inside that directory, and upon receiving `APPROVED`: autonomously merge into `main`, verify `cargo check && npm run build`, delete the worktree (`git worktree remove worktrees/<task-slug> --force`), delete the feature branch, and record the task into the ledger. Never let subagents write concurrently to the root working directory.
> 4. **3-TIER GOVERNANCE**: Lead Subagent -> Level 2 Reviewer (`agent-developer-reviewer`) -> Level 3 Retro Verifier (`agent-retro-verifier`).
> 5. **AUTONOMOUS MERGE UPON `APPROVED`**: Merges to `main` occur only after verified `APPROVED` verdicts and zero compiler/linter errors.
> 6. **STRICT SINGLE-TASK CEILING & ZERO AUTONOMOUS SCOPE-CREEP (§ 1.3)**: Every subagent is assigned EXACTLY ONE atomic task and MUST STOP IMMEDIATELY upon completion. Subagents are STRICTLY FORBIDDEN from expanding scope or inventing unassigned work.
> 7. **NASA GROUND TRUTH VERIFICATION MANDATE (§ 1.4)**: All verification MUST rely exclusively on Ground Truth Runtime Sensors:
>    - Compiler & Clippy Sensor: `cargo check` and `cargo clippy -- -D warnings`.
>    - Frontend Bundle Sensor: `pnpm run build` (`tsc && vite build`).
>    - Live Process Sensor: real subprocess execution and real HTTP/SSE streaming tests against local AI proxies.
> 8. **ZERO-CONTEXT-POLLING RULE [HARD BAN ON MONITORING SPAM] (§ 1.5)**: When background subagents are executing, the Root Orchestrator is STRICTLY FORBIDDEN from executing monitoring/polling loops (`list_agents`, `job_list`, `pwsh git status/log`). Wait passively for subagent push notices.
> 9. **FULL PEER ADVISOR & DEVIL'S ADVOCATE MANDATE (§ 1.6)**: The Root Orchestrator is an authoritative peer architectural counselor, not a passive yes-man. When discussing architectural designs, you are explicitly authorized and required to challenge flawed proposals, contradict suboptimal ideas, offer battle-tested alternatives, and debate trade-offs as a full peer technical partner.

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

### 2.8 Dynamic Custom Tools (`.cvh/tools/`)
- **Extensible Schema Engine**: Adding a JSON descriptor in `.cvh/tools/<tool_name>.json` automatically exposes a custom tool to all agents:
  ```json
  {
    "name": "run_linter",
    "description": "Run project linter inside worktree",
    "parameters": {
      "type": "object",
      "properties": { "fix": { "type": "boolean" } }
    },
    "command": "npm run lint -- {{args}}"
  }
  ```
- **Automatic Argument Injection**: `{{args}}` is dynamically substituted with the tool call arguments and executed within the active worktree.
- **Tool Output Spilling Invariant**: Outputs exceeding 100 lines or 8 KB are automatically written to `.cvh/spill/<timestamp>-<tool>.log`, returning only the head (20 lines) and tail (20 lines) to protect model context.

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

### 2.12 Zero-Credential Leak & Local Environment Invariant
- **Zero Secrets in Git**: Secrets and local endpoints reside strictly in gitignored files (`.cvh/llm.json`, `.env*`, `*.secret`, `*.key`) or runtime UI settings. Default API keys evaluate to empty strings (`String::new()`, `""`).
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

### 2.16 CVH-DEV & Copilot Scope Invariant: Zero-Card-Management & 3-Tier Tool Gating
- **Zero-Card-Management Invariant**: The `cvh-dev` persona and CVH Copilot Sidecar Dock are strictly Project Configuration & Environment Specialists (`.cvh/` infrastructure, boards, pipelines, agent personas, rules, and hooks). They are strictly forbidden from creating, completing, moving, or managing task cards. Card lifecycle is strictly reserved for the human user and the pipeline orchestrator.
- **Symmetric 3-Tier Tool Gating**: The allowed configuration tools (`CVH_DEV_TOOLS`: `read_file`, `write_file`, `edit_file`, `glob`, `grep`, `execute_command`, `web_search`) MUST be enforced at all three layers of the architecture:
  1. *Prompt Schema Definition*: Only the 7 configuration tools are compiled into the system prompt and tool definitions sent to the LLM.
  2. *Candidate Tool Filtering*: `active_tools.retain(|t| CVH_DEV_TOOLS.contains(&t.function.name.as_str()))` before submission to LLM.
  3. *Runtime Defense-in-Depth Execution Gate*: `execute_tool` in `agent.rs` MUST strictly check `(role == "cvh-dev" || role == "copilot") && !CVH_DEV_TOOLS.contains(&tool)` and reject unauthorized tool calls with `PermissionDenied`.
- **UI Labeling Consistency**: UI surfaces for Copilot (`CopilotDock.tsx`, topbar badge, placeholders, empty states) MUST reflect "НАСТРОЙКА ПРОЕКТА" / "Project Configuration", never mislabeling configuration sidecars as general task creators.
