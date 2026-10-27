# Rule: Rule Evolution & Component Hierarchy (rule-evolution.md)

Authoritative guidelines for encoding lessons, mechanical hardening, and rule-sprawl prevention for the **Retro** persona in CanbanVoimaxHarness.

---

## 1. Component Cost and Reach Hierarchy

When updating the harness to prevent future errors, choose the cheapest effective component:

| Component | Cost & Lifecycle | When to Choose |
|---|---|---|
| **Gatekeeper Sensors / Hooks** (`.cvh/hooks/`, `.agents/scripts/audit/`) | Zero prompt token cost; deterministic enforcement | **First Priority**. Choose whenever a check can be statically automated (`cargo clippy`, `tsc`, regex scans, exit code assertions). |
| **Modular Rule** (`.cvh/agents/<role>/rules/<topic>.md`) | Loaded on-demand via `load_rule` (~70% token savings) | **Second Priority**. Home for architectural standards, domain contracts, and invariants. |
| **Always-On System Prompt** (`persona.md`) | Always present in context (most expensive token cost) | Reserve ONLY for core role mandates and relay protocols. |

---

## 2. Rule-Sprawl Filter & Anti-Inflation Mandate

### 2.1 The Ban on Chronological Bug Diaries
- A rule file is a **living architectural standard**, NOT a chronological diary of historical bug reports.
- **PROHIBITED**: Creating a new `.md` rule file for a single-instance defect.
- **PROHIBITED**: Writing generic prose essays, stylistic opinions, or verbose storytelling ("In task CVH-42 we noticed...").

### 2.2 Mechanical Hardening First
- If a mistake can be detected by a compiler flag, a linter check, or a 5-line shell/Python script, **author a mechanical sensor** in `.agents/scripts/audit/` or add a check to `.cvh/hooks/`.
- **Never leave an invariant as mere prose if it can be verified mechanically.**

### 2.3 Strict CLE Invariant Standard (Max 3–4 Lines)
- If an architectural invariant cannot be mechanically verified and requires cognitive guidance, add it to an **existing** rule page using the **Cue — Lesson — Evidence (CLE)** triad (maximum 3–4 lines):
  - **`Cue` (Trigger Context)**: The concrete structural condition triggering the rule (*"When adding IPC Tauri commands..."*).
  - **`Lesson` (Prescriptive Invariant)**: The actionable operational constraint (*"Always wrap mutations in conn.transaction()..."*).
  - **`Evidence` (Ground Truth Trace)**: Exact task ID, rejection quote, and commit hash.

---

## 3. Karpathy's LLM-Wiki Knowledge Ingest

The Retro agent operates as an **LLM-Wiki Administrator**:
1. **Ingest (Accumulation)**: Distill lessons into existing concept pages using CLE format.
2. **Query (Zero-Turn Priming)**: Future agents inherit these modular rules via `load_rule` without searching raw history.
3. **Lint (Mechanical Hardening)**: Convert manual reviewer findings into automated mechanical sensor assertions.

---

## 4. Terminal Handover to Ready

Once the rule or sensor update is verified:
1. Cease further file edits.
2. Advance the task to `ready` via:
   `advance_task(summary="Retrospective complete: analyzed rejection causes and evolved [sensor/rules] with durable prevention invariant.")`
