---
description: Rule evolution lifecycle, Uncle Bob refactoring lessons, and component hierarchy maintenance.
---
# Rule: Rule Evolution, Uncle Bob Lessons & Component Hierarchy (rule-evolution.md)

Authoritative guidelines for encoding lessons, mechanical hardening, and rule-sprawl prevention for the **Retro** persona in CanbanVoimaxHarness.

---

## 1. Component Cost and Reach Hierarchy

When updating the harness to prevent future errors, choose the cheapest effective component:

| Component | Cost & Lifecycle | When to Choose |
|---|---|---|
| **Gatekeeper Sensors / Hooks** (`.cvh/hooks/`, compiler/linter commands) | Zero prompt token cost; deterministic mechanical enforcement | **First Priority**. Choose whenever a check can be statically automated (`cargo clippy`, `cargo test`, `pnpm test`, `tsc`, `oxlint`, exit code assertions). |
| **Modular Rule** (`.cvh/agents/<role>/rules/<topic>.md`) | Read on-demand via `read_file` (~70% token savings) | **Second Priority**. Home for architectural standards, domain contracts, and timeless invariants. |
| **Always-On System Prompt** (`persona.md`) | Always present in context (most expensive token cost) | Reserve ONLY for core role mandates and relay protocols. |

---

## 2. Uncle Bob Clean Code Lessons & Refactoring Invariants

From the major architectural refactoring of CanbanVoimaxHarness, extract and enforce these foundational principles:

### 2.1 Eliminating Primitive Obsession via Value Objects
- **Lesson**: Never represent core domain concepts using raw primitive types (`String`, `&str`, `number`). Naked strings lead to semantic inversion bugs (e.g. passing a `StageId` into a `TaskId` parameter).
- **Practice**: Always use nominal branded types and Rust newtype wrappers: `domain::ids::TaskId`, `StageId`, `BoardId`. These types enforce semantic validation at construction and compile-time type safety.

### 2.2 Make Illegal States Unrepresentable via Algebraic Data Types (ADT)
- **Lesson**: Never model domain state with fragmented, independent boolean flags (e.g. `is_running: bool`, `is_waiting: bool`, `is_error: bool`). Boolean flags produce 2^N states, the majority of which are mutually contradictory and illegal.
- **Practice**: Model system state machines strictly using Rust `enum` ADTs (`domain::state::TaskState`): `Idle`, `Running { model }`, `WaitingSubtask { subtask_id }`, `WaitingSubagent`, `Done { verdict }`. Enforce valid state transitions via explicit methods (`state.transition_to(...)`).

### 2.3 Automated Unit Testing Shield (Vitest & Cargo Test)
- **Lesson**: Fast, reliable unit tests provide fearless refactoring velocity and act as an impenetrable regression shield.
- **Practice**: Enforce 100% test pass rates across both frontend (Vitest: `pnpm test`) and backend (`mbx test` / `cargo test` across `domain`, `db`, `config`, `agent`, `commands`).
- **Anti-Mock Policy**: Tests must verify real transformations, parsers, and reducers. Never substitute live runtime sensor verification with synthetic mocks for external subprocesses or IPC.

### 2.4 Single Responsibility Principle (SRP) & Sizing Ceilings
- **Lesson**: Massive monolithic files accumulate accidental coupling, invite git merge conflicts, and exceed model context windows.
- **Practice**: Enforce strict file sizing ceilings: React components <= 350 lines (gold standard: `KanbanCard.tsx` at 266 lines); Rust backend files <= 500 lines. Break modules cleanly across domain boundaries (`db/`, `config/`, `domain/`, `agent/`).

### 2.5 Decoupled Event Gateways (No Direct `listen<any>`)
- **Lesson**: Scattering raw IPC event listeners across UI components introduces memory leaks, unclosed listeners, and untyped payload bugs.
- **Practice**: All Tauri event subscriptions MUST route through `src/services/tauriEventGateway.ts` with strongly typed payloads and micro-batching.

---

## 3. Rule-Sprawl Filter & Anti-Inflation Mandate

### 3.1 The Ban on Chronological Bug Diaries
- A rule file is a **living architectural standard**, NOT a chronological diary of historical bug reports.
- **PROHIBITED**: Creating a new `.md` rule file for a single-instance defect.
- **PROHIBITED**: Writing generic prose essays, stylistic opinions, or verbose storytelling ("In task CVH-42 we noticed...").

### 3.2 Mechanical Hardening First
- If a mistake can be detected by a compiler flag, a linter check, or a shell script, **author a mechanical sensor or test assertion**.
- **Never leave an invariant as mere prose if it can be verified mechanically.**

### 3.3 Strict CLE Invariant Standard (Max 3–4 Lines)
- If an architectural invariant cannot be mechanically verified and requires cognitive guidance, add it to an **existing** rule page using the **Cue — Lesson — Evidence (CLE)** triad (maximum 3–4 lines):
  - **`Cue` (Trigger Context)**: The concrete structural condition triggering the rule (*"When adding IPC Tauri commands..."*).
  - **`Lesson` (Prescriptive Invariant)**: The actionable operational constraint (*"Always wrap mutations in conn.transaction()..."*).
  - **`Evidence` (Ground Truth Trace)**: Exact task ID, rejection quote, and commit hash.

---

## 4. Karpathy's LLM-Wiki Knowledge Ingest

The Retro agent operates as an **LLM-Wiki Administrator**:
1. **Ingest (Accumulation)**: Distill lessons into existing concept pages using CLE format.
2. **Query (Zero-Turn Priming)**: Future agents inherit these modular rules via pre-compiled system prompts or `read_file` without searching raw history.
3. **Lint (Mechanical Hardening)**: Convert manual reviewer findings into automated mechanical sensor assertions.

---

## 5. Concrete CLE Examples from Uncle Bob Refactoring

### [CLE: Primitive Obsession]
- Cue: When passing task, stage, or board identifiers across domain, DB, or IPC layers.
- Lesson: Never use naked String or &str; use `domain::ids::{TaskId, StageId, BoardId}` branded value objects.
- Evidence: CVH-UB01 - Eliminating Primitive Obsession across backend repositories.

### [CLE: Illegal State Prevention]
- Cue: When managing agent task lifecycle states (`idle`, `running`, `waiting_subtask`, `waiting_subagent`, `done`).
- Lesson: Never store multiple boolean flags; use `domain::state::TaskState` ADT enum and explicit transitions.
- Evidence: CVH-UB02 - Make Illegal States Unrepresentable in task transitions.

### [CLE: Frontend Event Gateway]
- Cue: When subscribing to Tauri backend IPC events in React components or stores.
- Lesson: Ban direct `listen<any>()`; subscribe exclusively through `src/services/tauriEventGateway.ts`.
- Evidence: CVH-UB03 - Centralizing Tauri event streaming and micro-batching.

### [CLE: File Sizing Ceilings]
- Cue: When creating or refactoring React components or Rust backend modules.
- Lesson: Enforce React components <= 350 lines, Rust files <= 500 lines; decompose into submodules on breach.
- Evidence: CVH-UB04 - Decomposing monolithic `db.rs`, `config.rs`, and `KanbanCard`.

### [CLE: Windows Desktop Toast Notification Delivery]
- Cue: When implementing native desktop notifications or spawning toast processes on Windows.
- Lesson: Register AUMID in `HKCU\Software\Classes\AppUserModelId\<id>`, implement multi-AUMID cascade with NotifyIcon fallback, enforce `Start-Sleep -Milliseconds 1500` before process exit to protect Action Center RPC, and use `CREATE_NO_WINDOW = 0x08000000`.
- Evidence: CVH-017 (commit `76597d7`) - Windows desktop notifications failing silently due to missing registry AUMID and premature PowerShell process termination.

### [CLE: i18n Key Parity Merge Conflict Resolution]
- Cue: When integrating upstream `main` into a branch adding translation keys and encountering conflict in `src/i18n/__tests__/i18n.test.ts:31-32`.
- Lesson: Calculate `(keys in main) + (branch keys added)`, update both `enKeys` and `ruKeys` length assertions, and verify with `pnpm test` and `git merge-tree --write-tree main HEAD`.
- Evidence: CVH-017 (commit `a9286e1`) and CVH-019 (commit `76597d7`) - Gate 6 rejection from unmerged concurrent i18n additions.

### [CLE: Worktree Dual-Write Residue & Merge Hook Safety]
- Cue: When editing files under `.cvh/` (such as `frontend_map.md`, `backend_map.md`, or agent rules) within an isolated git worktree (`worktrees/<task-id>`).
- Lesson: The harness dual-writes `.cvh/` modifications to the root repository. Merge hooks (`on_enter_ready.{bat,sh}`) must defensively clean uncommitted dual-write residue (`git restore --staged .cvh && git restore -- .cvh`) before `checkout main` and `git merge --no-ff`, and developers must commit `.cvh` edits cleanly to the feature branch.
- Evidence: CVH-057 (commit `8ab5ce7a`, fixed in `09693c48` / `8d0bea10`) - Review hook failure: 'Your local changes to `.cvh/rules/frontend_map.md` would be overwritten by merge'.

### [CLE: Merge Hook Backup-Then-Discard Ordering]
- Cue: When a merge hook backs up live `.cvh/` to `%TEMP%` before `git checkout main` / `git merge --no-ff`, while the main worktree holds uncommitted tracked `.cvh/` residue.
- Lesson: Backup alone does not unblock checkout/merge; after backing up config files (excluding DBs/runtime dirs), run `git restore -- .cvh` (never `git clean` on `.cvh/`, which would delete SQLite DBs and untracked local configs!) before `checkout main` and `git merge --no-ff`, then `git restore --staged .cvh` and restore the backup after merge. Keep `.bat` and `.sh` in parity.
- Evidence: CVH-048 (commit `54e9bb5f`) & `on_enter_ready.bat:34-64` (`never git clean DBs!`) - Gate 6 rejection when uncommitted tracked `.cvh/` changes blocked merge.

### [CLE: UI Column Layout Symmetry & Visual Boundaries]
- Cue: When designing or styling sidebars, drawers, or floating cockpit columns (`SubagentsSidebar`, `GitDiffSidebar`, `AgentConsoleDrawer` cockpit, `CortexExplorer`).
- Lesson: Columns must own their full perimeter: apply symmetric vertical margins (`my-1.5`), precise height calculations matching vertical margins (`h-[calc(100%-0.75rem)]`), complete 4-corner radius (`rounded-xl`), closed border (`border`), and clipping (`overflow-hidden`) to prevent unstyled flat bottom collisions with screen edges.
- Evidence: CVH-057 (commit `8ab5ce7a`) - Defective asymmetric columns with `mt-1`, `border-t border-x`, `rounded-tl-xl rounded-tr-xl`, and `h-[calc(100%-0.25rem)]` running off the bottom edge of the window.

### [CLE: Antigravity Stealth Dark Palette Enforcement]
- Cue: When styling dark mode variants in frontend UI components (`src/components/`, `src/index.css`).
- Lesson: Enforce Antigravity stealth dark palette tokens (`#111114`, `#17171b`, `#232328`, `#212126`, `#2c2c33`, `#3a3a44`, `#f4f4f5`, `#a1a1aa`, `#71717a`, `#52525b`); ban legacy GitHub dark tokens (`#0d1117`, `#161b22`, `#21262d`, `#30363d`, `#f0f6fc`, `#8b949e`) to prevent styling drift and merge conflicts with `main`.
- Evidence: CVH-064 (commit `8e49b1c9`) - Gate 6 merge conflict in `DrawerHeader.tsx` after concurrent CVH-062 dark theme migration.

### [CLE: Pipeline Stage Stepper & Duration Traversal Separation]
- Cue: When rendering stage progress, timers, or execution indicators across parent tasks and child subtasks (`rootTask`, `subtasks`).
- Lesson: Decompose hierarchy traversal and duration resolution into pure, testable helper functions (`resolveStageTask`, `resolveStageDuration`, `resolveStageStatus`); never leak cumulative `totalElapsed` into individual stage pills, and preserve parent stage durations when switching views.
- Evidence: CVH-064 (commit `b99f10ed`) - Resolving disappearing planning duration, 02:04 cumulative bleed on developer stage, and inverted active dot indicators.

### [CLE: Handover Steer Single-Writer & Deterministic Idempotency]
- Cue: When a stage transition writes a handover/steer dialog item and also emits `agent:autorun` with the same prompt that `run_agent_loop` persists again via `persist_steer_message`.
- Lesson: Keep exactly one writer per handover (skip direct `append_task_dialog_activity` when target has autorun), use deterministic `handover:{task_id}:{hash}` `message_id` for `ON CONFLICT DO NOTHING` idempotency, and dedup only handover-marked steer/summary by `TRIM(content)` - never blanket-dedup user/steer repeats.
- Evidence: CVH-051 (commit `e8b124eb`) - Duplicate HANDOVER SUMMARY Agent messages from dual UUID writes; fixed with single-writer gate, deterministic id, and handover-only dedup in `dialog_repo`/`activitySlice`/`streamHelpers`.

### [CLE: Scoped Git Staging & Deferred-Delete Failure Preservation]
- Cue: When a save/commit flow stages a subtree (e.g. `.cvh/`) with an allowlist filter, or executes deferred tombstone deletes before clearing them.
- Lesson: Stage only allowlist-filtered paths explicitly (`git add -- <each filtered path>`) and unstage pre-staged non-allowlist paths (`git restore --staged`) before commit - never `git add <subtree>/` then filter only the commit decision; collect deferred-delete failures, surface via error state, and abort before clearing tombstones.
- Evidence: Scoped `.cvh/` git save & deferred-delete preservation (`cortexGitSave.ts`, `cvh_save.rs`).

---

## 6. Terminal Handover to Ready

Once the rule or sensor update is verified:
1. Cease further file edits.
2. Advance the task to `ready` via:
   `advance_task(summary="Retrospective complete: analyzed rejection causes and evolved [sensor/rules] with durable prevention invariant.")`
