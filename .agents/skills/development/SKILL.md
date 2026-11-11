---
name: development
description: Authoritative software engineering skill for CanbanVoimaxHarness. Enforces isolated git worktrees, Ponytail minimalism, subsystem-targeted verification sensors (Vitest, oxlint, tsc, mbx), and active retro stage alignment.
---

# Development Skill for CanbanVoimaxHarness

### Core Directives:
1. **Isolated Worktrees**: Never author code directly in the root working directory. Work strictly inside `worktrees/<task-slug>`.
2. **Ponytail Minimalism**: Implement the simplest working architecture. Avoid overengineering, deep inheritance, or unneeded third-party crates/libraries.
3. **Automated Test & Linter Shield**: Author deterministic Vitest tests (`pnpm test`) for pure utilities, domain types, and store slices, and targeted `mbx test` / `cargo test` for Rust domain/db/agent changes. Never rely on unverified assumptions.
4. **Subsystem-Targeted Sensors**:
   Run verification for the touched subsystems once implementation is complete:
   - **Frontend**: `pnpm test`, `pnpm exec tsc --noEmit`, `pnpm exec oxlint --deny-warnings`
   - **Backend**: `mbx check --manifest-path src-tauri/Cargo.toml`, `mbx clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`, and targeted `mbx test --manifest-path src-tauri/Cargo.toml` when changed behavior needs it (fall back to `cargo` only if `mbx` is absent from PATH).
5. **No Startup Commands**: Do NOT run compilation or test commands during initial static exploration (`read_file`, `glob`, `grep`).
6. **Reviewer Verification Policy**: Follow `.cvh/rules/review-checklist.md` Gate 2 — reuse valid sensor logs tied to the reviewed commit when relevant sources and environment are unchanged; run only missing, failed, stale, or risk-justified checks.
7. **Active Retro Stage**: `retro` is an active stage in `.cvh/boards.json` (`.cvh/agents/retro/`) for post-revision CLE rule evolution and mechanical sensor hardening; tasks approved after `CHANGES_REQUESTED` route through `retro` before `ready`.
