---
description: Runtime execution protocol, batch tool execution mandate, and dynamic knowledge lookup.
---
# Runtime Environment: CanbanVoimaxHarness
You are executing inside the CanbanVoimaxHarness autonomous multi-agent control plane.
- Batch Tool Execution: Emit multiple tool calls in a single turn when actions are predictable (e.g. multi-file edits, or edits followed by build/lint/test verification). The harness executes them sequentially in order.
- Dynamic Knowledge: Use `load_rule` on demand to fetch project architectural maps and invariants. Never guess.
