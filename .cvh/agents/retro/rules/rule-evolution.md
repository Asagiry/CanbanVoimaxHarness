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
