# Rule: Ground Truth Compiler Sensors & Verification (sensors.md)

Authoritative guidelines for compiler verification for the **Developer** persona in CanbanVoimaxHarness.

---

## 1. Zero-Mock Ground Truth Mandate

In CanbanVoimaxHarness, tasks are never completed or handed off on assumptions.
Every code modification must be verified against real compiler and linter sensors inside the active worktree directory (`CVH_WORKTREE_PATH`).

**Never rely on hallucinations or synthetic assumptions.**
If a sensor command fails, you must analyze the compiler diagnostics, fix the root cause, and re-run the sensor until it exits with code 0.

---

## 2. Mandatory Verification Sensor Commands

Before calling `advance_task`, the Developer MUST execute:

### 2.1 Backend Compiler Sensor (Rust / Tauri)
```bash
cargo check --manifest-path src-tauri/Cargo.toml
```
- **Acceptance Criterion**: Exit code `0` with zero compilation errors.

### 2.2 Backend Linter Sensor (Clippy)
```bash
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
- **Acceptance Criterion**: Exit code `0` with zero compiler warnings.

### 2.3 Frontend Typecheck Sensor (TypeScript)
```bash
npx tsc --noEmit
```
- **Acceptance Criterion**: Exit code `0` with zero TypeScript type errors.

### 2.4 Frontend Build Sensor (Vite / Production Bundle)
```bash
npm run build
```
- **Acceptance Criterion**: Exit code `0` producing clean production assets in `dist/`.

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
