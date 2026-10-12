---
name: development
description: Authoritative software engineering skill for CanbanVoimaxHarness. Enforces isolated git worktrees, Ponytail minimalism, zero-TDD Ground Truth Sensor verification, and pre-commit verification.
---

# Development Skill for CanbanVoimaxHarness

### Core Directives:
1. **Isolated Worktrees**: Never author code directly in the root working directory. Work strictly inside `worktrees/<task-slug>`.
2. **Ponytail Minimalism**: Implement the simplest working architecture. Avoid overengineering, deep inheritance, or unneeded third-party crates/libraries.
3. **Zero-TDD Mandate**: Never author synthetic unit tests with mocked state. Use Ground Truth Sensors:
   - Rust: `cargo check` & `cargo clippy -- -D warnings`.
   - Frontend: `npm run build` (`tsc && vite build`).
4. **Pre-Commit Verification**: Run compiler and bundle sensors before spawning the reviewer subagent.
