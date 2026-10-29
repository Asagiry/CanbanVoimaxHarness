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
- **Practice**: Model system state machines strictly using Rust `enum` ADTs (`domain::state::TaskState`): `Idle`, `Running { model }`, `WaitingSubtask { subtask_id }`, `Done { verdict }`. Enforce valid state transitions via explicit methods (`state.transition_to(...)`).

### 2.3 Automated Unit Testing Shield (Vitest & Cargo Test)
- **Lesson**: Fast, reliable unit tests provide fearless refactoring velocity and act as an impenetrable regression shield.
- **Practice**: Enforce 100% test pass rates across both frontend (Vitest: `pnpm test`, 139 tests across 8 suites) and backend (`cargo test`, 244 tests across domain, db, config, agent, commands). Total test shield: 383 automated tests.
- **Anti-Mock Policy**: Tests must verify real transformations, parsers, and reducers. Never substitute live runtime sensor verification with synthetic mocks for external subprocesses or IPC.

### 2.4 Single Responsibility Principle (SRP) & Sizing Ceilings
- **Lesson**: Massive monolithic files accumulate accidental coupling, invite git merge conflicts, and exceed model context windows.
- **Practice**: Enforce strict file sizing ceilings: React components <= 350 lines (gold standard: `KanbanCard.tsx` at ~239 lines); Rust backend files <= 500 lines. Break modules cleanly across domain boundaries (`db/`, `config/`, `domain/`, `agent/`).

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

```markdown
### [CLE: Primitive Obsession]
- Cue: When passing task, stage, or board identifiers across domain, DB, or IPC layers.
- Lesson: Never use naked String or &str; use domain::ids::{TaskId, StageId, BoardId} branded value objects.
- Evidence: CVH-UB01 - Eliminating Primitive Obsession across backend repositories.

### [CLE: Illegal State Prevention]
- Cue: When managing agent task lifecycle states (idle, running, waiting_subtask, done).
- Lesson: Never store multiple boolean flags; use domain::state::TaskState ADT enum and explicit transitions.
- Evidence: CVH-UB02 - Make Illegal States Unrepresentable in task transitions.

### [CLE: Frontend Event Gateway]
- Cue: When subscribing to Tauri backend IPC events in React components or stores.
- Lesson: Ban direct listen<any>(); subscribe exclusively through src/services/tauriEventGateway.ts.
- Evidence: CVH-UB03 - Centralizing Tauri event streaming and micro-batching.

### [CLE: File Sizing Ceilings]
- Cue: When creating or refactoring React components or Rust backend modules.
- Lesson: Enforce React components <= 350 lines, Rust files <= 500 lines; decompose into submodules on breach.
- Evidence: CVH-UB04 - Decomposing monolithic db.rs, config.rs, and KanbanCard.
```

---

## 6. Terminal Handover to Ready

Once the rule or sensor update is verified:
1. Cease further file edits.
2. Advance the task to `ready` via:
   `advance_task(summary="Retrospective complete: analyzed rejection causes and evolved [sensor/rules] with durable prevention invariant.")`