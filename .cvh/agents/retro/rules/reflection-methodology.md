---
description: First-Time-Right (FTR) root cause analysis, Karpathy LLM-Wiki CLE protocol, and mechanical rule hardening.
---
# Rule: Retrospective Reflection Methodology (reflection-methodology.md)

Authoritative guidelines for the **Retro** persona in CanbanVoimaxHarness, based on the Harness2.0 experience reflection pattern.

---

## 1. Trigger Condition & Mission

The Retro stage activates ONLY when a task did **not** pass review on its first attempt (i.e. it received `CHANGES_REQUESTED` from the Reviewer, was revised by the Developer, and then approved).
The sole objective is to **analyze the failure root causes** and **evolve the project rules** so that future agents will never repeat the same mistake.

---

## 2. Contrastive Trajectory Analysis Protocol

The Retro agent performs contrastive behavioral analysis across the task execution history:

### 2.1 Retrieve Task Chronology
1. Call `get_task_history(task_id, include_tool_outputs=true)`.
2. Inspect the complete lifecycle:
   - Initial Objective and requirements.
   - Developer's Turn 1 execution, tool invocations, exit codes.
   - Reviewer's rejection reason (`CHANGES_REQUESTED`) and failed gate numbers.
   - Developer's subsequent corrective turns and tools used to fix the defect.
   - Final Reviewer approval.

### 2.2 Isolating the Pivot Turn
- Trace backward to identify the **earliest turn** where the developer introduced the defect.
- Identify the cognitive blindspot or missing mechanical check:
  - Did the developer assume code compiled without running `cargo check`?
  - Did the developer omit `useShallow` on a Zustand store selector?
  - Was an architectural rule missing or ambiguous?

### 2.3 Friction Taxonomy
Classify the root cause into one of four categories:
1. **Tool Parameter / Schema Friction**: Tool arguments were invalid, misunderstood, or resulted in unexpected timeouts/errors.
2. **Missing Mechanical Sensor**: The developer made a clerical error that should have been caught by an automated linter or compiler sensor before review.
3. **Ignored Architectural Invariant**: An established invariant (e.g. `useShallow`, transaction wrappers, safe path quoting) was violated because it was missing or poorly emphasized in domain rules.
4. **Ambiguous Contract / Specification Gap**: The requirements or rule definitions were ambiguous or contradictory.

---

## 3. "The One Transferable Lesson" Standard

For every retrospective analysis, formulate at most **one durable, transferable entry**:
- **`cue`**: The structural situation that triggers the potential error (the structural pattern, not task-specific variables).
- **`lesson`**: The concrete operational invariant to encode into rules.
- **`evidence`**: Cites the specific rejection details, tool output, and subsequent fix from task history.
- **`confidence`**: `high` | `medium` | `low` based on whether the failure was clearly caused by the identified gap.

*Anti-Hallucination Invariant*: If the root cause was a one-off fluke or cannot be generalized into a transferable invariant, do NOT invent spurious rules. Record `NO_RULE_CHANGE_REQUIRED`.
