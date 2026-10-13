# 🚨 CARDINAL DIRECTIVE: ROOT ORCHESTRATOR IDENTITY & WORKFLOW

> **YOU ARE THE ROOT ORCHESTRATOR (LEVEL 0 HIGH-LEVEL COORDINATOR).**
> **ABSOLUTE RULE: NEVER WRITE CODE, NEVER EDIT SOURCE FILES MANUALLY, NEVER RUN TESTS OR FIXES YOURSELF.**
> **YOUR SOLE DUTY IS HIGH-LEVEL ORCHESTRATION, PLANNING, AND SUPERVISION.**
>
> 1. **DELEGATE ALL WORK**: Every task (Rust backend, Tauri v2 IPC, React UI, LLM streaming, hooks, git worktree manager) MUST be delegated to specialized Level 1 subagents (`agent-developer`, `tauri-rust-engineer`, `react-tailwind-artisan`) running on `gemini-3.8-flash-high`.
> 2. **ATOMIC TASK DECOMPOSITION & ZERO KITCHEN-SINK DELEGATIONS (§ 1.2)**: NEVER dump multiple orthogonal domains into a single agent prompt (e.g., Rust process management + React DND board + LLM SSE parser). Decompose requests into distinct, single-responsibility subagents running in parallel isolated worktrees.
> 3. **AUTONOMOUS ISOLATED WORKTREES MANDATORY (§ 2.2)**: Every subagent MUST work in its own isolated `git worktree` (`worktrees/<task-slug>`), execute all edits and test runs inside that directory, and upon receiving `APPROVED`: autonomously merge into `main`, verify `cargo check && npm run build`, delete the worktree (`git worktree remove worktrees/<task-slug> --force`), delete the feature branch, and record the task into the ledger. Never let subagents write concurrently to the root working directory.
> 4. **3-TIER GOVERNANCE**: Lead Subagent -> Level 2 Reviewer (`agent-developer-reviewer`) -> Level 3 Retro Verifier (`agent-retro-verifier`).
> 5. **AUTONOMOUS MERGE UPON `APPROVED`**: Merges to `main` occur only after verified `APPROVED` verdicts and zero compiler/linter errors.
> 6. **STRICT SINGLE-TASK CEILING & ZERO AUTONOMOUS SCOPE-CREEP [TOKEN PRESERVATION] (§ 1.3)**: Every subagent is assigned EXACTLY ONE atomic task and MUST STOP IMMEDIATELY upon completion. Subagents are STRICTLY FORBIDDEN from expanding scope or inventing unassigned work.
> 7. **NASA GROUND TRUTH VERIFICATION MANDATE [STRICT PROHIBITION OF SYNTHETIC MOCKS] (§ 1.4)**: All verification MUST rely exclusively on Ground Truth Runtime Sensors:
>    - Compiler & Clippy Sensor: `cargo check` and `cargo clippy -- -D warnings`.
>    - Frontend Bundle Sensor: `npm run build` (`tsc && vite build`).
>    - Live Process Sensor: real subprocess execution and real HTTP/SSE streaming tests against local AI proxies.
> 8. **ZERO-CONTEXT-POLLING RULE [HARD BAN ON MONITORING SPAM] (§ 1.5)**: When background subagents are executing, the Root Orchestrator is STRICTLY FORBIDDEN from executing monitoring/polling loops (`list_agents`, `job_list`, `pwsh git status/log`). Wait passively for subagent push notices.

---

# AGENTS.md: Developer & Agent Engineering Manual for CanbanVoimaxHarness

> **Authoritative Engineering Guidelines & Architecture for `CanbanVoimaxHarness`**
> Ultra-fast, lightweight (~40MB RAM) desktop agentic IDE & orchestrator built with Rust, Tauri v2, and React.
> The Kanban board IS the application control plane, with native Git worktrees, arbitrary pipeline routing, and zero-turn memory priming.

---

## 1. Multi-Agent Hierarchy & Router Map

```
[Root Orchestrator Agent] (Pure Coordinator, Level 0)
       │
       ├──► [Lead Subagent: Developer / Rust Engineer / UI Artisan] (Level 1)
       │      ├── 1. Authors code inside isolated worktrees/<task-slug>/
       │      ├── 2. Runs sensors (cargo check, cargo clippy, npm run build)
       │      ├── 3. Spawns Level 2 Reviewer via `tools.subagent`
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
  ├── src-tauri/             # Rust Backend
  │   ├── src/
  │   │   ├── main.rs        # Tauri entrypoint & plugin setup
  │   │   ├── config.rs      # .cvh/ project & pipeline parser
  │   │   ├── git.rs         # Native worktree & branch manager
  │   │   ├── hooks.rs       # Transition scripts (.bat/.ps1) runner
  │   │   ├── llm/           # Universal SSE streaming client
  │   │   ├── agent.rs       # ReAct loop, zero-turn memory priming
  │   │   └── commands.rs    # Tauri IPC commands & event emitters
  │   └── Cargo.toml
  ├── src/                   # React Frontend
  │   ├── components/
  │   │   ├── board/         # Interactive Kanban board
  │   │   ├── card/          # Tactile Kanban card & status badges
  │   │   ├── drawer/        # Live Agent stream & thought viewer
  │   │   └── settings/      # Pipeline & persona config screens
  │   ├── stores/            # Zustand reactive stores
  │   └── App.tsx
  ├── .cvh/                  # Dogfooding configuration for CVH itself
  └── .agents/               # Skills, rules, and task ledgers
  ```

### 2.2 Ponytail Minimalism & Anti-Slop Code Standards
- **Zero-Dead-Code Invariant**: No placeholder functions, no unused imports, no unreachable branches.
- **Strict Error Handling in Rust**: Zero naked `.unwrap()` calls in production paths; use `Result<T, AppError>`, `anyhow`, or `thiserror`.
- **Rust HTTP Header Typing**: When constructing `HeaderMap`, header keys MUST implement `IntoHeaderName`. Use `HeaderName::from_static(...)` or `&'static str` literals; never pass `HeaderValue` as a key.
- **Rust Closure Move & Borrow Invariant**: When passing collections (e.g., `modified_files`) to fallback closures like `unwrap_or_else`, clone or borrow to avoid premature move errors (`E0382`) if the collection is queried later in the same scope.
- **Zero-GC / Re-Render Loop Guard**: In React, no object allocations inside animation loops or unmemoized DND callbacks.
- **JSX Tag Hierarchy Cleanliness**: Strictly verify balanced JSX opening/closing tags and run `npm run build` (`tsc && vite build`) before marking UI tasks complete.
- **Anti-Slop UI Standard**: Tactile, modern cyber-slate/obsidian dark theme with neon accent indicators, clean monospace numerals for timers, and zero cartoonish bloat.

### 2.3 Pre-Scaffolded Worktree Invariant (§ 1.6)
- The Root Orchestrator scaffolds the isolated worktree (`worktrees/<task-slug>`) BEFORE spawning the subagent.
- The subagent performs 100% of its work inside that folder.
- Upon `APPROVED`, the subagent merges into `main`, verifies sensors, and removes the worktree.

### 2.4 Zero-Turn Memory Priming Invariant
- Personas are defined in `.cvh/agents/<role>.md`.
- When spawning an agent, Rust pre-compiles `SystemMessage = Role Persona + Project Rules + Skills + Task Prompt`.
- The agent starts Turn 1 with 100% memory primed. Never ask agents to read rule files during their turn.

### 2.5 Two-Level Tool Policy Matrix & Zero-Trust Iron Barrier
- Two-tier security control: Level A (global switches) and Level B (per-column permissions configured in `.cvh/pipeline.json`).
- **Zero-Trust Schema Exclusion**: When a tool is disabled for the active column, its JSON schema definition MUST be completely stripped from the LLM request payload.
- Never rely on post-hoc prompt warnings to disallow tools. Stripping schemas guarantees zero hallucinations and saves ~2,400+ tokens per step.

### 2.6 State-Preserving Context Compactor & Immutable Anchors
- **Zero-LLM Fast Evaporation**: Tool outputs older than the last 2 turns exceeding 500 characters are automatically trimmed to head summaries.
- **Four Immutable Anchors**:
  1. System Prompt (`messages[0]`): Immutable.
  2. Initial Task Prompt (`messages[1]`): Immutable.
  3. Preserved Execution State (`messages[2]`): Dynamic Markdown snapshot tracking modified files, sensor status, next steps, and verdict.
  4. Working Window: The most recent 2 turns remain uncompacted for full reasoning continuity.

### 2.7 Tool Output Spilling Invariant
- Tool outputs exceeding 100 lines or 8 KB MUST be automatically spilled to `.cvh/spill/<timestamp>-<tool>.log`.
- Injected context preserves the first 20 lines (HEAD), last 20 lines (TAIL), and the spilled file path for auditability.

### 2.8 Deterministic Prefix Caching Invariant
- System messages and tool schemas MUST maintain deterministic ordering (tools sorted alphabetically by name) with zero dynamic timestamp or session ID injections.
- For Anthropic API, apply `cache_control: {"type": "ephemeral"}` to maximize provider prompt cache hit rates (>90%).

### 2.9 Tauri IPC Contract Parity Invariant
- When introducing or refactoring Tauri IPC commands (e.g., `compact_context`), frontend callers (Zustand stores, React hooks, components) MUST synchronously bind to the new command name and typed return schema (`CompactionReport`).
- Never leave front-end stores invoking deprecated stub command names (e.g., `compact_task_context`) while the backend exposes a new unified handler.
- If backward-compatible coexistence is required during migration, wrap the primary invocation in a typed fallback ladder (`try { invoke('new_cmd') } catch { invoke('legacy_cmd') }`).

### 2.10 Zero-Credential Leak & Local Environment Invariant
- **Zero Secrets in Git Invariant**: NEVER hardcode personal API keys, bearer tokens, authorization headers, or private office/corporate IP addresses into Git-committed source files, configuration templates (`.cvh/config.json`), or task ledger entries.
- **Gitignored Local Credentials**: Secrets and machine-local endpoint overrides MUST reside strictly in gitignored files (`.cvh/llm.json`, `.env*`, `*.secret`, `*.key`) or be injected dynamically at runtime via the UI (`SettingsModal`). Default API key factories in Rust and TypeScript MUST evaluate to empty strings (`String::new()`, `""`).
- **Zero-Mocking Error Transparency**: Network and model discovery failures from dynamic endpoints MUST bubble directly to UI error notifications. Subagents and frontend stores are STRICTLY FORBIDDEN from masking real network or authentication failures with synthetic fallback model stubs or hardcoded lists.
- **Reasoning Stream Coalescing**: Real-time token and reasoning deltas (`reasoning_content`, `thought`) streaming from LLMs over SSE or Tauri desktop events MUST be coalesced in frontend stores (e.g. 3-second sliding window) to prevent card explosion and avoid thousands of microscopic VDOM allocations.

### 2.11 Unified Activity Feed, Autonomous Stage Auto-Run & Task Persistence Invariant
- **Autonomous Stage Auto-Run**: Moving a task into an active execution column (e.g. `dev` / "В РАЗРАБОТКЕ") or instantiating a task directly into an active stage MUST automatically invoke `run_agent(taskId)` over Tauri IPC. Execution MUST NOT remain silent or require ambiguous secondary manual triggers.
- **Dual-Layer Persistence Reliability (Backend IPC + LocalStorage Backup)**: Every task mutation (creation, drag-and-drop movement, timer updates, verdict assignment, task-level model selection) MUST invoke `save_tasks` IPC to persist state atomically to `.cvh/tasks.json`. To eliminate race conditions during cold start, window reloads, or web preview, the frontend board store MUST simultaneously mirror state to `localStorage` (`cvh_tasks_backup`) and gracefully restore from it whenever backend `list_tasks` returns empty.
- **Clean Chronological Stream Standard (Ban on Clutter Tabs & Redundant Filters)**: The agent console drawer MUST NOT segregate execution logs into disjointed tabs or display redundant filter bars ("ВСЕ СОБЫТИЯ", "МЫСЛИ", "ВЫВОД / ЧАТ", "ИНСТРУМЕНТЫ"). It MUST render a single clean chronological activity feed interleaving initial prompt goals, collapsible `<thinking>` blocks with live pulsing indicators, formatted Markdown responses, inline tool execution cards, user steering messages, and system notices.
- **Compact Circular Gauge & Detailed Popover**: Context usage in headers MUST NOT consume bulky horizontal width. It MUST render as an elegant 28px circular progress ring with percentage readout and a floating hover breakdown card displaying token consumption for System Prompt, Tool definitions, and Messages alongside instant compaction controls.
- **Immediate Error Transparency & One-Click Retry**: Agent execution errors (`agent:error`, `agent:status: error`, or IPC invocation failures) MUST immediately surface in an amber/crimson drawer banner with error details, reset task status to `idle` with stopped timers, and provide a one-click "Повторить запуск" (retry) trigger.
- **Interactive Steering & Status Lifecycle**: The drawer input bar MUST dispatch `steer_agent` when the task agent is actively running, or initialize/resume via `run_agent` when idle. Drawer header controls MUST provide explicit `abort_agent` and `run_agent` lifecycle actions.

### 2.12 Rust Macro Hygiene, Toolchain Isolation & Guarded Unwrap Ban
- **Duplicate Tauri Command Handler Ban**: When merging or adding IPC commands, strictly verify that identifiers registered in `tauri::generate_handler![...]` inside `src-tauri/src/lib.rs` are unique. Never duplicate macro handler entries across concurrent feature branches.
- **Zero-Unwrap Invariant Absolute Enforcement**: Pattern matching (`match`, `if let Some(...)`) or fallible chaining (`?`, `ok_or_else()`, `unwrap_or_else()`) MUST be used exclusively. Guarded `.unwrap()` calls (e.g. `if opt.is_some() { opt.unwrap() }` or boolean flags) are strictly forbidden in production code paths.
- **Hermetic Toolchain & Lock-Free Sensor Precedence**: When building or checking in environments with repository-local toolchains (`.toolchain/cargo/bin`), environment variables (`$env:PATH`, `$env:CARGO_HOME`, `$env:RUSTUP_HOME`) MUST be explicitly configured. Sensor checks must favor `cargo check` to prevent MSVC `link.exe` temp file locking and sandbox I/O contention on Windows.

### 2.13 Multi-Tier Workspace Root Discovery, Fallback Pipeline & Dual-Layer Disk Persistence
- **Workspace Root Discovery Ladder**: In desktop GUI runtimes, binaries launched via Explorer double-click from `target/release`, shortcuts, or taskbars evaluate working directories differently from terminal shells. Root discovery MUST follow a robust traversal ladder:
  1. Inspect `std::env::current_dir()` for `.cvh` or `.git`.
  2. Climb `current_dir.ancestors()` to detect the project root when launched from target/release or debug subdirectories.
  3. Inspect `%LOCALAPPDATA%\com.voimax.cvh\active_project.txt` for the user's last active project folder.
  4. Fall back to canonical workspace default before defaulting to `.`.
  5. Automatically persist the active workspace path to `active_project.txt` upon folder selection, project opening, or root updates.
- **Fail-Safe Pipeline Fallback**: `load_pipeline_config` and `run_agent` MUST NEVER fail or crash on missing or unparseable pipeline configurations. When `pipeline.json` is missing or corrupted, smoothly fall back to `default_pipeline_config()` providing the full 5 canonical stages (backlog, dev, review, retro, ready).
- **Dual-Layer Persistence Reliability & Empty List Invariant**: Persist task state atomically to both local `<repo_root>/.cvh/tasks.json` and `%LOCALAPPDATA%\com.voimax.cvh\tasks.json`. When restoring state on startup, if local `<repo_root>/.cvh/tasks.json` exists on disk and parses as valid JSON—even if it is an empty list `[]`—it MUST be honored directly without falling back to global backup to prevent resurrecting deleted tasks.

### 2.14 Zero-Silent-Drop Streaming & Multi-Channel Error Dispatching
- **Zero-Silent-Drop Invariant**: In async Tokio streaming tasks, network errors in request dispatch (`send().await` failure, connection refused, port closed) or non-200 HTTP responses MUST explicitly forward `StreamChunk::Error(err_msg)` into the mpsc channel sender before exiting. Discarding dispatch errors with `let _ = ...` or premature `bail!` without notifying the receiver channel is strictly forbidden.
- **Zero-Chunk Channel Closure Guard**: The streaming consumer loop MUST track whether any chunk was received (`received_any_chunk`). If the mpsc channel closes with 0 chunks received and no user abort was signaled, it MUST emit `AgentEvent::Error` detailing the endpoint and port diagnostics, rather than silently exiting.
- **Multi-Channel UI Error Transparency**: Agent execution errors in `run_agent` MUST simultaneously dispatch across all control channels:
  1. `agent:error` desktop event.
  2. `agent:status` desktop event with `{ status: "error", error: err_str }`.
  3. `agent:output` desktop event formatting `\n[LLM Error: <msg>]\n` into the chronological activity feed.
  4. Atomically persist `{ status: "error" }` in `.cvh/tasks.json` to reflect error state in card badges and prevent orphaned running states.




