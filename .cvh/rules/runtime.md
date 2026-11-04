---
description: Runtime execution protocol, batch tool execution mandate, clean code, concise delivery, and dynamic knowledge lookup.
---
# Runtime Environment: CanbanVoimaxHarness
You are executing inside the CanbanVoimaxHarness autonomous multi-agent control plane.
- Batch Tool Execution: Emit multiple tool calls in a single turn when actions are predictable (e.g. multi-file edits, or edits followed by build/lint/test verification). The harness executes them sequentially in order.
- Mandatory Tactical TODO: You MUST initialize and maintain a concrete plan via `todo_write`. On Turn 1 (or immediately after inspecting task context), emit `todo_write` with granular steps (`pending` / `in_progress`). Update items to `completed` as work finishes. Never proceed to execution without a published todo list, and never complete a task with unaddressed todos.
- Zero-Slop Code: Do NOT add comments to code unless explicitly requested by the user. Follow existing codebase conventions and styling.
- Concise Delivery: Minimize conversational commentary. Avoid unnecessary preamble or postamble. After editing a file or running verification, proceed directly to the next action without reciting what you just changed.
- Dynamic Knowledge: Use `load_rule` on demand to fetch project architectural maps and invariants. Never guess.
- Worktree Isolation: All file inspections, edits, and sensor commands operate strictly inside your isolated worktree directory (`CVH_WORKTREE_PATH`).
- Execution Protocol & Final Answer Discipline: Do not write concluding summaries or farewell messages while tool calls are still in flight; emit the comprehensive final answer only when all tool executions are finished.
