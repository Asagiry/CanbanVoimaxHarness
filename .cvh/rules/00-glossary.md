# CanbanVoimaxHarness — Ubiquitous Language Glossary (DDD)

> **Authoritative Ubiquitous Language Dictionary** for CanbanVoimaxHarness (CVH).  
> Structured according to Eric Evans' *Domain-Driven Design* (Ubiquitous Language) and Matt Pocock's *Domain Modeling* methodology.  
> **Single Source of Truth**: All domain entities, value objects, and invariants defined here govern type definitions across TypeScript (`src/types/domain.ts`) and Rust (`src-tauri/src/domain/`).

---

## 1. Architectural Philosophy & Ubiquitous Language

Domain-Driven Design (DDD) dictates that software models must reflect a shared, rigorous vocabulary mutually understood by domain experts, systems developers, and executing runtime engines.

### 1.1 Core Principles (Evans & Pocock)
1. **Zero Incidental Leakage**: Domain models capture business rules, invariants, and state transitions—never incidental UI framework bindings, CSS quirks, or transient database drivers.
2. **Domain Reality vs. Presentation Projections**: A sharp, non-negotiable boundary separates domain concepts from their visual UI representations. UI elements (columns, cards, badges) are ephemeral projections; domain models (stages, tasks, personas) represent authoritative ground truth.
3. **Make Illegal States Unrepresentable**: Invalid domain configurations (e.g. a finalized task holding an unresolved subtask blocker, or a task entering a stage with a failing compiler sensor) cannot be instantiated.
4. **Parse, Don't Validate at Boundaries**: Data crossing IPC, network, or disk boundaries is validated once and transformed into strictly branded nominal types (`TaskId`, `StageId`, `BoardId`) or discriminated algebraic unions (`TaskState`). Internal systems trust domain types unconditionally.

---

## 2. Ubiquitous Terms vs. Presentation Projections

A primary architectural anti-pattern in workflow tooling is conflating domain entities with graphical interface controls. In CanbanVoimaxHarness, the distinction is strictly enforced:

| Domain Term (Ubiquitous Language) | Presentation Projection (UI Layer) | Semantic Distinction & Boundary |
| :--- | :--- | :--- |
| **Stage** | **Column** | A **Stage** is an autonomous pipeline execution phase governing execution policies, sensors, and hooks. A **Column** is strictly the vertical UI container rendering that stage on a visual board. |
| **Task** | **Card** | A **Task** is an atomic unit of engineering work with an immutable identity, Git worktree, state machine, and dialogue ledger. A **Card** is strictly the rectangular tactile widget displayed to the user. |
| **Persona** | **Role Badge / Avatar** | A **Persona** is a declarative role definition, system prompt, and constraint hierarchy in `.cvh/agents/`. The avatar and colored badge are visual signifiers. |
| **Agent** | **Activity Indicator / Stream Drawer** | An **Agent** is an active, stateful runtime process executing a ReAct loop on a task. The spinner, live output drawer, and thinking view are real-time telemetry displays. |
| **Worktree** | **Diff Viewer / Workspace Tab** | A **Worktree** is an isolated physical Git directory on disk (`.cvh/worktrees/<task-slug>`). The diff viewer is a visual surface rendering modified files. |
| **Board** | **Kanban Board View** | A **Board** is an aggregate root grouping an ordered set of Stages and resident Tasks. The Kanban view is the canvas layout hosting Column projections. |

---

## 3. Authoritative Domain Dictionary

### Stage
**Domain Category:** Entity / Pipeline Phase  
**Identifier:** `StageId` (Branded nominal string, normalized lowercase, e.g. `backlog`, `dev`, `review`, `ready`, `done`)

A discrete, sequential phase of engineering execution within a pipeline. A Stage defines:
- The assigned default **Persona** responsible for processing work entering this phase.
- **Gatekeeper Sensors** that must pass with exit code `0` before any Task can cross into this Stage.
- **Lifecycle Hooks** executed automatically upon Stage entry (`on_enter`) or exit (`on_finish`).
- **Tool Policy Matrix** overrides governing tool capabilities for Agents operating in this phase.
- Behavioral classification:
  * **Intake Stage**: Ingress point of incoming work (e.g. `backlog`).
  * **Active Stage**: Interactive engineering and verification phases (e.g. `dev`, `review`).
  * **Terminal Stage**: Completion sinks where tasks settle (e.g. `ready`, `done`).

*Ubiquitous Invariant:* A "Column" is strictly the UI presentation of a Stage on a Kanban board. Business logic, sensors, and transitions operate exclusively on Stages.

---

### Persona
**Domain Category:** Specification / Declarative Role Template  
**Storage Path:** `.cvh/agents/<role>/`

The declarative role definition, system prompt, and parameters specifying a specialized AI collaborator. A Persona encompasses:
- `persona.md`: Primary identity, core objectives, tone, cognitive specialization, and operational boundaries.
- `rules/*.md`: Modular, domain-specific rule files loaded dynamically or statically to enforce invariants (e.g. `sensors.md`, `ponytail.md`, `backend.md`).
- Configuration: Recommended LLM models, inference temperature, reasoning effort, and baseline tool capabilities.

*Ubiquitous Invariant:* A Persona is a declarative template. When assigned to a Task and executed by the runtime engine, it is instantiated as an active **Agent**.

---

### Agent
**Domain Category:** Runtime Execution Instance

The active, stateful runtime instance of a **Persona** executing a ReAct loop on a specific **Task**. An Agent:
- Executes a reasoning-action-observation cycle (LLM thought generation $\rightarrow$ tool call invocation $\rightarrow$ tool result ingestion).
- Interacts with system tools strictly bounded by the active **Tool Policy Matrix**.
- Reads, modifies, and compiles code exclusively inside the Task's dedicated **Worktree**.
- Emits real-time telemetry (`<thinking>`, tool parameters, execution output, token counters).
- Maintains a 1:1 binding to a Task during execution. Only one active Agent process may run on a given Task at any moment.

---

### Task
**Domain Category:** Entity / Aggregate Root  
**Identifier:** `TaskId` (Branded nominal string, normalized uppercase, e.g. `CVH-042`, subtask `CVH-042.1`)

An atomic unit of engineering work with a persistent identity and lifecycle. A Task encapsulates:
- Unique, monotonic, normalized identifier (`TaskId`).
- Title, requirements description, and optional tags/attachments.
- Assigned **Stage** (`StageId`) representing its current pipeline location.
- Authoritative **Task State** (`Idle`, `Running`, `WaitingSubtask`, `Done`).
- Assigned **Worktree** path and dedicated feature branch (`feat/<task-slug>`).
- Dialogue history, streaming thought chunks, and token usage telemetry.

*Subtask:* A hierarchical child Task decomposed from a parent Task (e.g. `CVH-042.1`). Subtasks execute in their own isolated worktrees while the parent Task enters a `WaitingSubtask` state.  
*Ubiquitous Invariant:* A "Card" is strictly the visual representation of a Task on the Kanban board.

---

### Worktree
**Domain Category:** Isolation Boundary / Physical Workspace  
**Storage Path:** `.cvh/worktrees/<task-slug>`

An isolated Git working directory spawned specifically for a Task.
- **Main Tree Immunity**: Guarantees zero mutation of the main working branch while an Agent writes code, executes scripts, or triggers builds.
- **Hermetic Environment**: Compiler caches, temporary files, and intermediate test artifacts remain sandboxed within the worktree.
- **Lifecycle Coupling**: Created automatically upon task progression into an active stage; verified by sensors; and torn down or merged cleanly into the target branch upon task finalization.

---

### Relay Handoff
**Domain Category:** Value Object / Cross-Stage Memory

The structured architectural summary and findings passed forward from a completed Stage to the next Stage upon task promotion.
- Contains the executive summary of modifications made, files touched, architectural invariants verified, and specific instructions for the next Persona.
- Injects zero-amnesia context into the incoming Agent's prompt without polluting its context window with thousands of lines of raw terminal transcripts.
- Acts as the immutable knowledge bridge between stages (e.g., Lead Developer $\rightarrow$ Reviewer; Reviewer $\rightarrow$ Release Integrator).

---

### Gatekeeper Sensor
**Domain Category:** Invariant / Verification Gate

A blocking shell command (e.g. `cargo check`, `pnpm exec tsc --noEmit`) that MUST execute inside the Task's **Worktree** and exit with code `0` before a Task is permitted to enter a Stage.
- Enforces ground truth verification before stage transitions, preventing broken code, compilation failures, or type regressions from progressing downstream.
- If a Gatekeeper Sensor fails (non-zero exit code or timeout), the task transition is rejected immediately, and the task remains in its source Stage with diagnostic logs attached.

---

### Lifecycle Hook
**Domain Category:** Automated Pipeline Trigger

An automated shell script (`.bat`, `.ps1`, `.sh`) executed upon Stage boundary transitions.
- **Entry Hook (`on_enter`)**: Executes immediately after a Task enters a Stage and passes Gatekeeper Sensors. Prepares dependencies, runs seed scripts, or initializes tools.
- **Exit Hook (`on_finish`)**: Executes immediately when an Agent finishes work or requests stage advancement. Handles cleanup, reporting, or artifact generation.
- Operates under strict process boundary rules: failures block transition or trigger recorded rollbacks.

---

### Tool Policy Matrix
**Domain Category:** Security Policy / Capability Gate

A two-level hierarchical authorization matrix governing tool execution for an Agent.
- **Conjunctive Evaluation Rule**: Tool access requires both global authorization and stage authorization:
  $$\text{Allowed} = \text{globalEnabled} \land \text{stageOverrides}$$
- **Global Policy**: Repository-level enablement/disablement of tool classes (e.g., shell commands, filesystem writes, network calls).
- **Stage Policy**: Stage-specific overrides permitting or denying specific tools (e.g., enabling code edits in `dev`, but restricting `review` to read-only tools and verification handoffs).
- If either level evaluates to `false`, the tool execution is denied by the security matrix before any process invocation.

---

### Spill Log
**Domain Category:** Value Object / Secondary Storage  
**Storage Path:** `.cvh/spill/`

Secondary on-disk storage dedicated to tool outputs that exceed context window size thresholds.
- **Threshold**: Output exceeding 100 lines or 8 KB triggers automatic spill offloading.
- **Behavior**: The complete, uncorrupted output is saved to disk; the Agent receives a structured preview containing the head, tail, and absolute file path to the spill log.
- **Context Protection**: Prevents massive compiler dumps or search outputs from saturating the LLM attention window while preserving full audit trails.

---

### Cortex
**Domain Category:** Control Plane / Governance Subsystem

The mental inspection and orchestration control plane of CanbanVoimaxHarness.
- Exposes active Personas, behavioral rule sets, Stage configurations, and tool policies for real-time inspection.
- Serves as the authoritative administrative surface where meta-agents (e.g. `cvh-dev`) and human developers inspect and configure pipeline invariants.
- Maintains the synchronized, authoritative view of system capabilities, constraints, and workspace topology.

---

## 4. Supporting Domain Concepts

### Board
**Domain Category:** Aggregate Root  
**Identifier:** `BoardId` (Branded nominal string, normalized lowercase, e.g. `development`, `hotfix`)

The top-level orchestration container organizing an ordered collection of **Stages** and their resident **Tasks** into an end-to-end delivery pipeline.

---

### Task State (Lifecycle State)
**Domain Category:** Algebraic Data Type (ADT) / Discriminated Union

The authoritative execution status of a Task:
- `Idle`: Task is resting in a Stage; no active Agent process is running.
- `Running`: An Agent is actively executing a ReAct loop on this Task with a specified model.
- `WaitingSubtask`: Task is suspended awaiting the resolution of a designated child subtask (`waiting_for_subtask_id`).
- `Done`: Task has reached a terminal state in its pipeline, optionally bearing a verification verdict (`APPROVED`, `APPROVED_WITH_CAVEATS`, `CHANGES_REQUESTED`).

*Invariant:* Conflicting states (e.g., `Done` with an active subtask dependency) are unrepresentable by type design.

---

### Promotion & Demotion
**Domain Category:** Domain Operation / State Transition

- **Promotion**: Forward advancement of a Task to the next Stage, executed only after Gatekeeper Sensors pass, Lifecycle Hooks run, and a **Relay Handoff** is attached.
- **Demotion (Rejection)**: Backward transition of a Task to an earlier Stage when review or verification fails, returning the Task for remediation with structured critique.

---

## 5. Domain Invariants Summary

1. **Worktree Isolation Invariant**: An Agent MUST NEVER mutate the main repository tree directly. All changes, sensor runs, and tests MUST execute inside an isolated **Worktree**.
2. **Gatekeeper Invariant**: A Task CANNOT transition into a Stage unless all configured **Gatekeeper Sensors** for that Stage return exit code `0`.
3. **Tool Execution Invariant**: A tool invocation MUST be rejected if either `globalEnabled` OR `stageOverride` evaluates to `false` in the **Tool Policy Matrix**.
4. **Context Protection Invariant**: Tool execution outputs exceeding size limits (100 lines / 8 KB) MUST be persisted to a **Spill Log** to protect LLM context windows.
5. **State Consistency Invariant**: A Task in `Done` state MUST NOT hold an active child subtask reference or running agent process.
