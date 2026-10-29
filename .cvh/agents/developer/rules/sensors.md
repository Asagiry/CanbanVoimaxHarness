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
3. **Run Sensors at Verification Phase**: Execute sensors strictly after code implementation is complete, just before calling `advance_task`.

**Never rely on hallucinations or synthetic assumptions.**
If a sensor command fails, you must analyze the compiler or test diagnostics, fix the root cause, and re-run the sensor until it exits with code 0.

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
```bash
# 1. Compiler check:
cargo check --manifest-path src-tauri/Cargo.toml

# 2. Strict linter & clippy:
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
- **Acceptance Criteria**: Exit code `0` with zero compilation errors and zero compiler warnings.
- Optional when domain/db modules are refactored: `cargo test --manifest-path src-tauri/Cargo.toml`.

### 2.3 Full-Stack Turbo Sensors (<15s)
When both frontend and backend are touched, or for full-stack pre-handover validation:
```bash
pnpm test && pnpm exec tsc --noEmit && pnpm exec oxlint --deny-warnings && cargo check --manifest-path src-tauri/Cargo.toml
```
- **Acceptance Criteria**: All 4 sensors exit `0` sequentially.

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

---

## 3. Sensor Output in Handover Summary

When invoking `advance_task(summary)`, the Developer MUST include explicit sensor proof:
```text
- Sensors Verified:
  1. pnpm test: EXIT 0 (139/139 tests passed)
  2. pnpm exec tsc --noEmit: EXIT 0 (0 type errors)
  3. pnpm exec oxlint --deny-warnings: EXIT 0 (0 warnings)
  4. cargo check --manifest-path src-tauri/Cargo.toml: EXIT 0 (0 errors)
  5. git merge-tree --write-tree main HEAD: EXIT 0 (0 merge conflicts)
```
Failure to include concrete sensor results will cause the Reviewer to reject the task under Gate 2.
