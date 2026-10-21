---
name: development
description: Authoritative software engineering skill for CanbanVoimaxHarness. Enforces isolated git worktrees, Ponytail minimalism, Turbo Ground Truth Sensors (<10s), zero-TDD, and ban on premature compilation.
---

# Development Skill for CanbanVoimaxHarness

### Core Directives:
1. **Isolated Worktrees**: Never author code directly in the root working directory. Work strictly inside `worktrees/<task-slug>`.
2. **Ponytail Minimalism**: Implement the simplest working architecture. Avoid overengineering, deep inheritance, or unneeded third-party crates/libraries.
3. **Zero-TDD Mandate & Ban on `cargo test`**: NEVER author or execute synthetic unit tests (`cargo test`). Test builds allocate gigabytes of RAM, waste 5-8 minutes compiling test binaries, and cause catastrophic system lag.
4. **NASA Turbo Ground Truth Sensors (<10s Execution)**:
   Verification MUST be lightning fast and executed strictly ONCE at the end of implementation:
   - **Frontend**: `pnpm exec tsc --noEmit` (verifies 100% of TypeScript types in <3s). NEVER run full `pnpm run build` (`vite build`) in subagent worktrees.
   - **Backend**: `cargo check --manifest-path src-tauri/Cargo.toml` (<5s). NEVER run redundant sequential `cargo clippy`.
   - **Single Parallel Command**:
     `pwsh -Command "pnpm exec tsc --noEmit; cargo check --manifest-path src-tauri/Cargo.toml"`
5. **Strict Ban on Premature Cargo Checks**: NEVER run compilation commands during research, file reading, or mid-edit.
6. **Reviewer Ban on Re-Compilation**: If the developer reports 0 errors on Turbo Sensors, the reviewer audits the `git diff` only and MUST NOT re-run compiler passes.
7. **Zero-Retro Mandate**: Level 3 Retro Verifier is abolished. Do NOT spawn or wait for retro agents.
