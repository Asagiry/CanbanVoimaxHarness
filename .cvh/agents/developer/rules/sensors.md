# Rule: Ground Truth Compiler Sensors & Verification (sensors.md)

Authoritative guidelines for compiler verification for the **Developer** persona in CanbanVoimaxHarness.

---

## 1. Zero-Mock Ground Truth Mandate & Command Discipline

In CanbanVoimaxHarness, tasks are never completed or handed off on assumptions.
Every code modification must be verified against real compiler and linter sensors inside the active worktree directory (`CVH_WORKTREE_PATH`).

### Command Execution Discipline (Zero Unnecessary Commands)
1. **Never Run Commands on Startup**: Do NOT run `cargo check`, `npx tsc`, `npm run build`, or any other commands at the start of a task or before code edits are made. Initial exploration is strictly static (`read_file`, `glob`, `grep`).
2. **Subsystem-Targeted Sensors**: Run ONLY sensors corresponding to the files you modified. Never run backend checks for frontend-only tasks, and never run frontend builds for backend-only tasks.
3. **Run Sensors at Verification Phase**: Execute sensors strictly after code implementation is complete, just before calling `advance_task`.

**Never rely on hallucinations or synthetic assumptions.**
If a sensor command fails, you must analyze the compiler diagnostics, fix the root cause, and re-run the sensor until it exits with code 0.

---

## 2. Subsystem-Targeted Verification Sensors

Before calling `advance_task`, execute the sensors corresponding to the touched subsystems:

### 2.1 Backend Sensors (Run ONLY if `src-tauri/` or `.rs` files were modified)
```bash
cargo check --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
- **Acceptance Criteria**: Exit code `0` with zero compilation errors and zero compiler warnings.

### 2.2 Frontend Sensors (Run ONLY if `src/` or frontend assets/config were modified)
```bash
npx tsc --noEmit
npm run build
```
- **Acceptance Criteria**: Exit code `0` with zero TypeScript errors and clean Vite production build.

### 2.3 Non-Code / Prompt / Rule Modifications
- If only documentation, rule markdown files, or configuration files were changed without code edits, no heavy compilation commands are required. Verify file formatting and valid syntax.

---

## 3. Sensor Output in Handover Summary

When invoking `advance_task(summary)`, the Developer MUST include explicit sensor proof:
```text
- Sensors Verified:
  1. cargo check --manifest-path src-tauri/Cargo.toml: EXIT 0 (0 errors)
  2. cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings: EXIT 0 (0 warnings)
  3. npx tsc --noEmit: EXIT 0 (0 type errors)
```
Failure to include concrete sensor results will cause the Reviewer to reject the task under Gate 2.
