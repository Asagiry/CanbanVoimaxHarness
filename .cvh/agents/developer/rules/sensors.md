---
description: Ground truth compiler, linter, test suite, and Turbo sensor execution protocols for pre-review verification.
---
# Rule: Ground Truth Compiler & Test Sensors (sensors.md)

Authoritative guidelines for compiler, test, and runtime verification for the **Developer** persona in CanbanVoimaxHarness.

---

## 1. Zero-Mock Ground Truth Mandate & Command Discipline

In CanbanVoimaxHarness, tasks are never completed or handed off on assumptions.
Every code modification must be verified against real compiler, linter, and automated test sensors inside the active worktree directory (`CVH_WORKTREE_PATH`).

### Command Execution Discipline (Zero Unnecessary Commands)
1. **Never Run Commands on Startup**: Do NOT run `cargo check`, `pnpm test`, `pnpm exec tsc`, `npm run build`, or test suites at the start of a task before code edits are made. Initial exploration is strictly static (`read_file`, `glob`, `grep`).
2. **Subsystem-Targeted Sensors**: Run ONLY sensors corresponding to the files you modified. Never run backend checks for frontend-only tasks, and never run frontend builds for backend-only tasks.
3. **Run Sensors at Verification Phase**: Execute applicable sensors once after implementation. Repeat only when relevant changes invalidate prior results, a check failed, or a concrete uncovered risk needs verification.
4. **Strict Prohibition of Unix Pipelines on Windows**: In `execute_command`, NEVER execute Unix shell pipelines (`| wc -l`, `| grep`, `| head`, `| tail`, `export VAR=...`) on Windows. Use dedicated built-in tools (`grep`, `glob`, `read_file`) or platform-safe scripts.

**Never rely on hallucinations or synthetic assumptions.**
If a sensor fails, inspect diagnostics and fix the cause before rerunning the affected check. If an external blocker prevents verification, report it explicitly; do not repeat an unchanged failing command or claim success.

---

## 2. Subsystem-Targeted Verification Sensors

Before calling `advance_task`, execute the sensors corresponding to the touched subsystems:

### 2.1 Frontend Sensors (Run if `src/` or frontend assets/config were modified)
```bash
# 1. Run automated Vitest test shield (MANDATORY):
pnpm test

# 2. Run TypeScript typecheck:
pnpm exec tsc --noEmit

# 3. Run high-performance linter:
pnpm exec oxlint --deny-warnings
```
- **Acceptance Criteria**:
  - `pnpm test`: Exit code `0`, 100% test suites passing (0 failures).
  - `pnpm exec tsc --noEmit`: Exit code `0` with zero TypeScript errors.
  - `pnpm exec oxlint --deny-warnings`: Exit code `0` with zero warnings or errors.

### 2.2 Backend Sensors (Run if `src-tauri/` or `.rs` files were modified)
For accelerated verification in CanbanVoimaxHarness, use `mbx` (configured via `.mbx.toml` with cache in `.toolchain/mbx-cache`).
**MANDATORY**: Always use `mbx` for backend compilation and tests. Direct invocation of `cargo` is prohibited because `mbx` provides zero-latency compiler cache hits (<0.6s vs 15s cold). Use `cargo` only if `mbx` is physically not installed in PATH.
```bash
# 1. Accelerated compiler check (MANDATORY):
mbx check --manifest-path src-tauri/Cargo.toml

# 2. Accelerated test suite (for domain/db refactors & test updates):
mbx test --manifest-path src-tauri/Cargo.toml

# 3. Strict linter & clippy:
mbx clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
- **Acceptance Criteria**: Exit code `0` with zero compilation errors and zero compiler warnings.
- `mbx check`, `mbx test`, and `mbx clippy` use `.toolchain/mbx-cache` and `.mbx.toml`.

### 2.3 Full-Stack Turbo Sensors (<10s)
When both frontend and backend are touched, or for full-stack pre-handover validation:
```bash
git add -A && git commit -m "feat(<task-id>): <summary>" && git merge-tree --write-tree main HEAD && pnpm test && pnpm exec tsc --noEmit && pnpm exec oxlint --deny-warnings && mbx check --manifest-path src-tauri/Cargo.toml
```
- **Acceptance Criteria**: All sensors exit `0` sequentially with zero merge conflicts.

### 2.4 Non-Code / Prompt / Rule Modifications
- If only documentation, rule markdown files, or configuration files were changed without code edits, no heavy compilation commands are required. Verify file formatting and valid syntax.

### 2.5 Branch Integration & Zero-Regression Pre-Flight Sensor
```bash
# 1. Commit all working tree changes to the branch first:
git add -A && git commit -m "feat(<task-id>): <concise summary>"
# 2. Verify clean mergeability against main:
git merge-tree --write-tree main HEAD
```
- **Acceptance Criteria**: Working tree is clean (`git status --porcelain` empty) and `git merge-tree` returns exit code `0` with zero merge conflicts.
- **Concurrent Main Invariant (CLE)**:
  - **Cue**: When completing a task in an isolated worktree or touching shared components (`KanbanCard.tsx`, stores, migrations) while `main` advances with concurrent PRs.
  - **Lesson**: Always sync with `main` (`git merge origin/main`) and verify clean mergeability (`git merge-tree --write-tree main HEAD` exit 0) before handover; diff against `origin/main` to ensure concurrently merged features are never overwritten.
- **Committed HEAD Pre-Flight Invariant (CLE)**:
  - **Cue**: When preparing to invoke `advance_task` or running mergeability pre-flight checks.
  - **Lesson**: Commit all modified files to the feature branch before handover; running `git merge-tree --write-tree main HEAD` on a dirty working tree tests stale base commits and triggers automatic gatekeeper rejection (`git status --porcelain` check). Never hand off uncommitted changes, as teardown hooks (`on_review_finish.bat`) will discard them upon merge.
- **Worktree Dual-Write Residue Invariant (CLE)**:
  - **Cue**: When modifying files under `.cvh/` (such as `frontend_map.md`, `backend_map.md`, or agent rules) within an isolated git worktree (`worktrees/<task-id>`).
  - **Lesson**: The harness automatically dual-writes `.cvh/` modifications to the root repository. Feature branches must commit `.cvh` changes cleanly, and merge hooks (`on_review_finish.{bat,sh}`) must defensively clean uncommitted dual-write residue (`git restore --staged .cvh && git restore .cvh`) before `checkout main` and `git merge --no-ff` to prevent merge abortion from dirty working copies.
  - **Evidence**: CVH-057 (commit `8ab5ce7a`, fixed in `09693c48` / `8d0bea10`) - Review `on_leave` hook failed due to uncommitted `.cvh/rules/frontend_map.md` in root repo.
- **i18n Parity Assertion Merge Invariant (CLE)**:
  - **Cue**: When integrating upstream `main` into a branch that adds new translation keys in `src/i18n/{en,ru}.ts` and encountering a conflict in `src/i18n/__tests__/i18n.test.ts:31-32`.
  - **Lesson**: Never guess or leave stale branch key counts; calculate `(keys in main) + (keys introduced by feature branch)`. Update both `enKeys.length` and `ruKeys.length` assertions to the sum, verify via `pnpm test`, and confirm clean merge with `git merge-tree --write-tree main HEAD`.
  - **Evidence**: CVH-017 (commit `a9286e1`) & CVH-019 (commit `76597d7`) - Gate 6 rejection due to unmerged upstream i18n key count additions.

---

## 3. Sensor Output in Handover Summary

When invoking `advance_task(summary)`, include commands, exit codes, and log references for the applicable checks, plus the delivered commit and relevant environment. This allows the Reviewer to reuse evidence when sources, dependencies, configuration, and toolchain are unchanged. The following full-stack example is not a requirement to run checks for untouched subsystems:
```text
- Sensors Verified:
  1. pnpm test: EXIT 0 (100% Vitest suites passing, 0 failures)
  2. pnpm exec tsc --noEmit: EXIT 0 (0 type errors)
  3. pnpm exec oxlint --deny-warnings: EXIT 0 (0 warnings)
  4. mbx check / mbx test: EXIT 0 (0 compiler errors, 100% tests passing)
  5. git merge-tree --write-tree main HEAD: EXIT 0 (0 merge conflicts)
```
Failure to include concrete sensor results will cause the Reviewer to reject the task under Gate 2.
