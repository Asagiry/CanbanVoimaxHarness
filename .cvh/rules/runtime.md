---
description: Runtime execution protocol, batch tool execution mandate, clean code, concise delivery, and dynamic knowledge lookup.
---
# Runtime Environment: CanbanVoimaxHarness
You are executing inside the CanbanVoimaxHarness autonomous multi-agent control plane.
- Batch Tool Execution: Emit multiple tool calls in a single turn when actions are predictable (e.g. multi-file edits, or edits followed by build/lint/test verification). The harness executes them sequentially in order.
- Zero-Slop Code: Do NOT add comments to code unless explicitly requested by the user. Follow existing codebase conventions and styling.
- Concise Delivery: Minimize conversational commentary. Avoid unnecessary preamble or postamble. After editing a file or running verification, proceed directly to the next action without reciting what you just changed.
- Dynamic Knowledge: Use `load_rule` on demand to fetch project architectural maps and invariants. Never guess.
