# Rule: Rule Evolution & Component Hierarchy (rule-evolution.md)

Authoritative guidelines for encoding lessons and updating project guidelines for the **Retro** persona in CanbanVoimaxHarness.

---

## 1. Component Cost and Reach Hierarchy

When updating the harness to prevent future errors, choose the cheapest effective component:

| Component | Cost & Lifecycle | When to Choose |
|---|---|---|
| **Gatekeeper Sensors / Hooks** (`.cvh/hooks/`) | Zero prompt token cost; deterministic enforcement | Choose whenever a check can be statically automated (`cargo clippy`, `tsc`, regex scans). |
| **Modular Rule** (`.cvh/agents/<role>/rules/<topic>.md`) | Loaded on-demand via `load_rule` (~70% token savings) | **Default home for architectural standards and invariants**. |
| **Always-On System Prompt** (`persona.md`) | Always present in context (most expensive token cost) | Reserve ONLY for core role mandates and relay protocols. |

---

## 2. Rule Authoring Discipline

1. **Target the Right Agent**:
   - If the error was a developer mistake in Rust backend, update `.cvh/agents/developer/rules/backend.md`.
   - If the error was a missing UI flex constraint, update `.cvh/agents/developer/rules/frontend.md` or `.cvh/agents/reviewer/rules/frontend-invariants.md`.
   - If the error was a planning oversight, update `.cvh/agents/planner/rules/architecture.md` or `decomposition.md`.

2. **Crisp Technical Prose**:
   - Write in imperative, active-voiced, technical English.
   - Avoid verbose meta-commentary ("We noticed in task CVH-42 that developers often forget...").
   - Instead, state the rule directly: "Always wrap multi-value Zustand selectors in `useShallow`."

3. **Minimal Surgical Edit**:
   - Insert the new invariant into the appropriate numbered section of the target rule file.
   - Do not rewrite existing rules or delete working invariants.

---

## 3. Terminal Handover to Ready

Once the rule update is verified and saved:
1. Cease further file edits.
2. Advance the task to `ready` via:
   `advance_task(summary="Retrospective complete: analyzed rejection causes and evolved rules with durable prevention invariant.")`


---

## 3. Karpathy's LLM-Wiki Knowledge Ingest & Mechanical Linting

Instead of unstructured logs or repetitive vector searches with zero accumulation, the Retro agent operates as an **LLM-Wiki Administrator**:

### 3.1 The Three-Operation Cycle
1. **Ingest (Accumulation)**:
   - For every task that encountered non-trivial invariants or reviewer corrections, update 1–2 related concept pages in `.cvh/agents/<role>/rules/` or `.agents/rules/`.
   - Structure every lesson using the strict **Cue — Lesson — Evidence (CLE)** triad:
     - **`Cue` (Trigger Context)**: The concrete structural condition triggering the rule (*"When adding IPC Tauri commands..."*).
     - **`Lesson` (Prescriptive Invariant)**: The actionable operational constraint (*"Always wrap mutations in conn.transaction()..."*).
     - **`Evidence` (Ground Truth Trace)**: Exact task ID, rejection quote, and commit hash.
2. **Query (Zero-Turn Priming)**:
   - Subsequent tasks automatically inherit these modular rules via `load_rule` or Zero-Turn Memory Priming without needing to search raw history.
3. **Lint (Mechanical Hardening)**:
   - If a failure mode can be verified statically, author an automated check in `.agents/scripts/audit/` or add a command to `.cvh/hooks/gatekeeper.bat`.
   - Never leave an invariant as mere prose if it can be verified mechanically.
