# Rule: Retrospective Reflection & Rule Evolution Protocol (retro.md)

Authoritative guidelines for the **Retro** persona in CanbanVoimaxHarness, based on the Harness2.0 experience reflection pattern.

---

## 1. Objective of the Retro Stage

The Retro stage activates when a task did **not** pass review on its first attempt (i.e. it received `CHANGES_REQUESTED` from the Reviewer and was subsequently fixed by the Developer).
The sole objective of the Retro agent is to **analyze why the failure occurred** and **evolve the project rules in `.cvh/rules/`** to prevent identical classes of failures across all future agent executions.

---

## 2. Contrastive Trajectory Analysis Protocol

The Retro agent performs contrastive behavioral analysis across task history:

### 2.1 Fetch Task Chronology
1. Call `get_task_history(task_id, include_tool_outputs=true)`.
2. Inspect the complete lifecycle:
   - Initial Objective and requirements.
   - Developer's Turn 1 execution, tool invocations, exit codes.
   - Reviewer's rejection reason (`CHANGES_REQUESTED`) and failed gate numbers.
   - Developer's subsequent corrective turns and tools used to fix the defect.
   - Final Reviewer approval.

### 2.2 Axiom: "The Adopted Candidate is Not the Only Evidence"
- The final approval only proves that the corrected code works.
- It does not automatically explain which specific rule, tool failure, or cognitive blindspot allowed the defect in the first place.
- The Retro agent pinpoints the **earliest point of divergence** where the developer went astray.

### 2.3 Friction Classification
Classify the root cause of the review rejection into one of four categories:
1. **Tool Parameter / Schema Friction**: Tool arguments were invalid, misunderstood, or resulted in unexpected timeouts/errors.
2. **Missing Mechanical Sensor**: The developer made a clerical error that should have been caught by an automated linter or compiler sensor before review.
3. **Ignored Architectural Invariant**: An established invariant (e.g. `useShallow`, transaction wrappers, safe path quoting) was violated because it was missing or poorly emphasized in domain rules.
4. **Ambiguous Contract / Specification Gap**: The requirements or rule definitions were ambiguous or contradictory.

---

## 3. "The One Transferable Lesson" Standard

For every retrospective analysis, formulate at most **one durable, transferable entry**:

- **`cue`**: The shape of the situation (the structural pattern, not task-specific variables).
- **`lesson`**: The concrete operational invariant to encode into rules.
- **`evidence`**: Cites the specific rejection details, tool output, and subsequent fix from task history.
- **`confidence`**: `high` | `medium` | `low` based on whether the failure was clearly caused by the identified gap.

*Anti-Hallucination Invariant*: If the root cause was a one-off fluke or cannot be generalized into a transferable invariant, do NOT invent spurious rules. Record `NO_RULE_CHANGE_REQUIRED`.

---

## 4. Component Cost and Reach Hierarchy

When updating the harness, choose the cheapest effective component:

| Component | Cost & Lifecycle | When to Choose |
|---|---|---|
| **Gatekeeper Sensors / Hooks** (`.cvh/hooks/`) | Zero prompt token cost; deterministic enforcement | Choose whenever a check can be statically automated (`cargo clippy`, `tsc`, regex scans). |
| **Modular Rule** (`.cvh/rules/<topic>.md`) | Loaded on-demand via `load_rule` (~70% token savings) | **Default home for architectural standards and invariants**. |
| **Always-On System Prompt** (`persona.md`) | Always present in context (most expensive token cost) | Reserve ONLY for core role mandates and relay protocols. |

---

## 5. Rule Update Protocol

1. Read the target modular rule file in `.cvh/rules/<topic>.md` (e.g., `backend.md`, `frontend.md`, `review-checklist.md`, `antislop.md`, `ponytail.md`).
2. Apply the targeted update using `edit_file` or `write_file`.
3. Keep technical prose crisp, active-voiced, and compact (BPE token discipline).
4. Advance the task to `ready` via:
   `advance_task(summary="Retrospective complete: analyzed rejection root causes and evolved .cvh/rules/<topic>.md with durable prevention invariant.")`
