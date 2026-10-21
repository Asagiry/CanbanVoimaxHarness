# 🏛️ Everything-Claw-Code (ECC) Patterns & Progressive Disclosure Architecture

> **Authoritative Systems Reference & Role Catalog for CanbanVoimaxHarness (CVH)**  
> *Preserving battle-tested multi-agent roles, stack-specific rule modularity, and command workflows for the upcoming Progressive Disclosure rule engine.*

---

## 📌 Executive Summary & Architectural Context

### The Challenge: Context Bloat & Rule Saturation
Traditional AI coding agent configurations often suffer from a severe architectural pathology: **Rules Saturation & Context Bloat**. When a project configures 50+ pages of detailed engineering rules across multiple domains (Rust memory safety, React 19 memoization, SQLite WAL transaction semantics, OWASP security, Python scripting) and injects all of them simultaneously into the initial agent system prompt, several failure modes emerge:
1. **Instruction Dilution ("Lost in the Middle")**: LLMs exhibit declining instruction-following accuracy as system prompt token counts balloon past 15,000–30,000 tokens of static rules.
2. **Token Economy Degradation**: Every inference step pays a heavy prompt token penalty, increasing latency (TTFT) and compute costs.
3. **Domain Cross-Contamination**: A frontend styling subagent receives low-level Rust linker invariants; a database migration agent receives CSS View Transition guidelines.

### The Solution: Progressive Disclosure Architecture
In **CanbanVoimaxHarness (CVH)**, the current baseline prioritizes an agile, hyper-stable single-stage development loop (`cvh-dev` with a 200,000-token capacity and focused operational rules) to ensure bulletproof execution. 

This document archives the comprehensive engineering patterns from **Everything-Claw-Code (ECC)** as a permanent reference blueprint. These patterns will feed directly into CVH's upcoming **Progressive Disclosure Engine**, which dynamically loads specialized domain rules and roles on-demand based on:
- Active Kanban pipeline column and persona binding (`persona: "<role>"`).
- Worktree file mutations (e.g., selectively injecting Rust rules only when touching `*.rs` files).
- Explicit slash commands or task directives (`/review`, `/plan`, `/audit`).

---

## 🎭 Specialized Role Catalog (Adapted from ECC)

The following role specifications define specialized agent personas. In CVH's multi-agent hierarchy, these roles are mapped directly to Kanban columns or invoked via isolated child subagents (`subagent` / `subagent_bg`).

```text
                                +----------------------------------+
                                |      CVH KANBAN CONTROL PLANE    |
                                +-----------------+----------------+
                                                  │
                 ┌──────────────────┬─────────────┴──────┬──────────────────┐
                 ▼                  ▼                    ▼                  ▼
          +--------------+   +--------------+     +--------------+   +--------------+
          |  Architect   |   | Lead Dev     |     | Sec Reviewer |   | Perf Eng     |
          |  (Decompose) |   | (Implement)  |     | (Zero-Trust) |   | (Profiling)  |
          +--------------+   +--------------+     +--------------+   +--------------+
```

### 1. 📐 Systems Architect (`persona: architect`)
- **Primary Mission**: System decomposition, module boundary definition, data flow design, invariant enforcement, and Architecture Decision Record (ADR) authoring.
- **Operational Scope**:
  - Evaluates system-level impact before any operational code is written.
  - Decomposes complex user requests into atomic, single-responsibility subtasks.
  - Guards against premature optimization, redundant abstractions, and architectural anti-patterns.
- **Allowed Tools**: Read-only inspection tools (`read_file`, `glob`, `grep`), architectural documentation tools (`write_file` restricted to `docs/` and `ADR/`), task creation (`create_tasks`), and planning (`todo_write`).
- **Core Prompt Invariant**: *"Never touch production code. Author crisp interfaces, data schemas, and contract tests. Pass implementation to the Lead Developer."*

### 2. 🛡️ Security Reviewer (`persona: security-reviewer`)
- **Primary Mission**: Threat modeling, vulnerability identification, zero-trust validation, credential leak prevention, and AST taint analysis.
- **Operational Scope**:
  - Inspects Tauri IPC commands for command injection, path traversal, or unprivileged host capabilities.
  - Audits dependency trees and lockfiles (`pnpm-lock.yaml`, `Cargo.lock`) for supply-chain vulnerabilities.
  - Enforces secret isolation: ensures API keys, tokens, and local endpoints never leak into Git tracking or terminal logs.
  - Verifies input sanitization and deserialization boundaries (`serde`, `zod`, `JSON.parse`).
- **Allowed Tools**: Read-only tools (`read_file`, `glob`, `grep`), audit runners (`execute_command` restricted to `cargo audit`, `pnpm audit`), and verdict generation (`advance_task`, `reject_task`).
- **Verdict Standard**: Delivers formal `APPROVED`, `APPROVED_WITH_CAVEATS`, or `CHANGES_REQUESTED` with line-anchored vulnerability explanations.

### 3. 🧪 TDD Guide & Verification Engineer (`persona: tdd-guide`)
- **Primary Mission**: Ground truth sensor validation, test formulation, regression prevention, and edge-case boundary verification.
- **Operational Scope**:
  - Enforces NASA Ground Truth verification: tests must execute against real compilers (`cargo check`, `cargo test`, `tsc --noEmit`) and live runtime sensors.
  - Zero-Mock Mandate: Strictly prohibits synthetic mocking that hides integration failures.
  - Formulates reproduction test harnesses for reported bugs before fixes are applied.
- **Allowed Tools**: Full worktree tool suite (`read_file`, `write_file`, `edit_file`, `execute_command`, `todo_write`).
- **Core Invariant**: *"A feature is not complete when the LLM thinks it works; it is complete when the compiler and runtime sensors exit with status code 0."*

### 4. 💾 Database & Storage Optimizer (`persona: db-optimizer`)
- **Primary Mission**: SQLite storage optimization, schema index tuning, lock contention prevention, and data durability.
- **Operational Scope**:
  - Tunes SQLite Write-Ahead Logging (`PRAGMA journal_mode = WAL`) and synchronous flushing (`PRAGMA synchronous = NORMAL`).
  - Analyzes query execution plans (`EXPLAIN QUERY PLAN`) for table scans and unindexed lookups.
  - Guarantees monotonic sequence allocation and prevents ID recycling.
  - Enforces atomic upserts for streaming dialogue and activity feeds to prevent database lock timeouts.
- **Allowed Tools**: `read_file`, `edit_file`, `execute_command` (for running benchmark and migration scripts).

### 5. 🏎️ Performance & Systems Engineer (`persona: performance-engineer`)
- **Primary Mission**: Latency reduction, Time-to-First-Token (TTFT) and TPS throughput profiling, memory leak tracking, and async runtime optimization.
- **Operational Scope**:
  - Profiles Tokio async runtime loops: ensures CPU-intensive operations never block Tokio worker threads (mandating `spawn_blocking`).
  - Monitors and enforces CVH's signature **~40MB RAM desktop footprint** under active workload.
  - Verifies State-Preserving Context Compactor: ensures fast evaporation of outputs older than 2 turns exceeding 500 characters.
  - Inspects React 19 rendering cycles: audits Zustand store subscriptions, eliminates cascade repaints, and enforces `React.memo` with shallow comparators.
- **Allowed Tools**: `read_file`, `grep`, `execute_command` (profilers, benchmarks), `edit_file`.

---

## 🧱 Stack-Specific Modular Rules Architecture

Under Progressive Disclosure, rules are categorized by technology stack and loaded conditionally rather than dumped in a monolithic prompt.

```text
.cvh/agents/<role>/rules/
├── stack-typescript.md    # Injected when modifying *.ts, *.tsx, *.json
├── stack-rust.md          # Injected when modifying *.rs, Cargo.toml
├── stack-python.md        # Injected when modifying *.py
└── core-invariants.md     # Always present (<1,500 tokens)
```

### 1. TypeScript & React 19 Frontend Invariants (`stack-typescript.md`)
- **State & Reactivity**:
  - Use Zustand stores with fine-grained selectors and `useShallow` to prevent board-wide re-renders.
  - Wrap high-frequency interactive elements (`KanbanCard`, `KanbanColumn`, `StageTimer`) in `React.memo` with explicit custom shallow comparators.
- **UI & Accessibility**:
  - Enforce high-contrast Obsidian Dark theme (`#07090e`, `#0d1117`, `#161b22`, `#21262d`, `#f0f6fc`).
  - Never use native browser tooltips (`title="..."` ban on labeled controls) in WebView2/Tauri.
  - Use monospace tabular numerals (`tabular-nums`) for all live timers, counters, and metrics to prevent layout jitter.
  - CSS View Transitions API must always provide defensive fallback toggles when unsupported.
- **Sensor Gate**:
  - Run `pnpm exec tsc --noEmit` inside the isolated worktree before declaring work ready for review.

### 2. Rust 2021 & Tauri v2 Desktop Backend (`stack-rust.md`)
- **Tokio Async Safety**:
  - Never execute blocking I/O or sleep inside async tasks; use `tokio::task::spawn_blocking` for disk-heavy or compute-heavy operations.
  - Channels must be bounded (`mpsc::channel(buffer)`) or explicitly handled to prevent unbounded memory growth.
- **Error Handling**:
  - Zero naked `.unwrap()` or `.expect()` in production paths.
  - Use idiomatic `Result<T, AppError>` with typed error enums and `anyhow` for context enrichment.
- **Cross-Platform Toolchain Hygiene**:
  - Linker arguments in `build.rs` must check `CARGO_CFG_TARGET_ENV == "msvc"` specifically before passing MSVC-specific flags, avoiding breakage on MinGW/GCC.
  - Process management on Windows must cleanly terminate process trees using `taskkill /F /T /PID <pid>`.
- **Sensor Gate**:
  - Run `cargo check --manifest-path src-tauri/Cargo.toml` and `cargo clippy -- -D warnings`.

### 3. Python & AI Local Orchestration (`stack-python.md`)
- **Environment & Process Isolation**:
  - Use virtual environments (`.venv`) and pinned dependency manifests.
  - Process execution via subprocess must configure explicit timeouts and clean stderr capture.
- **Streaming & SSE**:
  - Resilient chunk decoders handling partial JSON frames and carriage-return line splits.
  - Strict type hints with Pydantic / dataclasses for all model interchange schemas.

---

## ⚡ Slash Commands & Action Triggers

In ECC, workflows are triggered via slash commands. In CVH, slash commands map directly to Kanban board operations and subagent invocations:

| Slash Command | ECC Original Meaning | CVH Native Control Plane Mapping |
| :--- | :--- | :--- |
| `/plan` | Decompose task, formulate roadmap | Runs Architect persona in `backlog` / `plan` column; creates live `todo_write` checklist. |
| `/review` | Code review, diff audit | Advances card to `review` column; spawns isolated clean-context child subagent (`agent-developer-reviewer`). |
| `/audit` | Security, dependency, and sensor scan | Triggers Gatekeeper Sensors (`cargo check`, `cargo audit`, `pnpm exec tsc`) and Security Reviewer persona. |
| `/compact` | Manually trigger context compaction | Invokes State-Preserving Dynamic Context Compactor; fast-evaporates stale logs (>500c) with zero token cost. |
| `/steer` | Mid-flight agent course correction | Appends steer directive to SQLite WAL `dialog_items` and updates active ReAct loop without wiping context. |

---

## 🚀 Progressive Disclosure Engine Roadmap for CVH

### Phase 1: Agile Stability (Current State)
- Single agile development stage with unified `cvh-dev` persona.
- 200,000-token capacity ceiling with State-Preserving Context Compactor.
- 2-Tier Tool Policy Matrix ensuring zero-trust schema stripping.
- Reliable, repeatable task completion across isolated native Git worktrees.

### Phase 2: Context-Aware Dynamic Rule Mounting
- Implementation of file watcher and stage hooks that inspect target worktree files.
- Dynamic injection of `stack-rust.md` or `stack-typescript.md` into the turn prompt only when corresponding files are opened or edited.
- Preservation of lean baseline prompts (<2,000 tokens) with **>95% KV prompt cache hit rates**.

### Phase 3: Multi-Persona Board Orchestration
- Seamless multi-column pipeline activation: `backlog` (Architect) → `dev` (Developer) → `review` (Security & Code Reviewer) → `ready` (Clean Merge & Teardown).
- Automatic subtask dispatch with specialized role prompts from this reference catalog.
- Zero manual prompt engineering required from the developer.

---

*Document Author: CanbanVoimaxHarness Systems Architecture Working Group*  
*Status: Authoritative Reference Blueprint for Future Progressive Disclosure Modules.*
