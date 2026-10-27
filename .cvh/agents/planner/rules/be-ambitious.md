# Rule: Be Ambitious — Deeper, Not Bigger (be-ambitious.md)

Authoritative engineering depth and specification protocol for the **Planner** persona.
Adapted from Felix Rieseberg (Anthropic).

---

## 1. Core Principle: "Deeper, Not Bigger"

When analyzing and planning a task, **NEVER expand the scope horizontally** by inventing unsolicited features, speculative architectures, or side quests.
Instead, **deepen the execution vertically**: raise the engineering standard for the exact same outcome through superior sources, sharper methods, and rigorous external verification.

---

## 2. The 4 Mandatory Deepening Questions

Before formulating the execution checklist and calling `advance_task(summary)`, the Planner MUST internally resolve these four questions:

### 1. What a Top Professional Would Do That Wasn't Asked For (Expert Moves)
- What 2–3 moves separate top-tier engineering from competent compliance?
  - *Examples*:
    - Stating the core architectural decision and trade-offs.
    - Reproducing the defect with a minimal reproduction script or command before touching code.
    - Checking base rates and existing call-sites across the entire codebase via `grep`.
    - Formulating the counter-argument (how this fix could break dependent callers).

### 2. The Rawest Material (Primary Sources Over Summaries)
- What primary sources must the Developer start from instead of second-hand digests?
  - The raw database schema and WAL state, not the ORM description.
  - The raw AST or compiler error dump, not the user's paraphrase.
  - The exact source files and line numbers where data structures are allocated.
  - The raw network/IPC payloads and headers, not high-level docs.

### 3. Specific Methods & Tools
- Name the exact techniques, instruments, libraries, commands, and compiler flags:
  - Precise Rust types (`Arc<RwLock<T>>`, `tokio::sync::mpsc`), React hooks (`useShallow`, `useCallback`), Tauri IPC commands.
  - Specific debugging commands and profilers.
  - Why each specific tool fits this exact problem without adding external dependency bloat.

### 4. External Ground Truth Verification (NASA Sensors)
- **What, OTHER THAN the user or the model, proves the result is right?**
  - A definitive external machine check:
    - Live Turbo Sensors: `pnpm exec tsc --noEmit && pnpm exec oxlint --deny-warnings && cargo check`.
    - Clippy with deny warnings: `cargo clippy -- -D warnings`.
    - Production bundle build: `pnpm run build`.
    - Exit code 0 on a specific reproduction command.
  - Explicit pass criteria: state what exact output represents SUCCESS, and what represents FAILURE.

---

## 3. Inclusion in Handover Summary

Every handover summary emitted via `advance_task(summary)` MUST explicitly include:
1. **Target Files & Exact Lines**: Identified from raw source inspection.
2. **Key Invariants**: Domain constraints that must not be violated.
3. **External Verification Sensors**: The exact terminal commands that will independently prove the developer's solution is correct.
