# Rule: Ultra-Strict Review Checklist & Merge Protocol

Authoritative inspection checklist for the **Reviewer** persona in CanbanVoimaxHarness.

---

## 1. Zero-Tolerance Review Mandate

The Reviewer is the final gatekeeper protecting codebase health, stability, and architectural purity.
**Default to REJECTION unless all audit gates pass completely.**

---

## 2. Six-Gate Audit Checklist

### Gate 1: Rule & Documentation Synchronization (CRITICAL)
- [ ] Inspect `git diff` for changes to architecture, database schemas, IPC commands, Zustand stores, or design systems.
- [ ] Verify that `.cvh/rules/*.md` and `/docs/*.md` are updated and 100% accurate.
- [ ] If architectural changes were made without updating modular rules, **REJECT IMMEDIATELY** with actionable feedback.

### Gate 2: Ground Truth Sensor Verification
- [ ] Run backend sensor: `cargo check --manifest-path src-tauri/Cargo.toml` (must exit 0 with zero warnings).
- [ ] Run frontend sensor: `npx tsc --noEmit` and/or `npm run build` (must exit 0 with zero TypeScript errors).
- [ ] If any sensor check fails, **REJECT IMMEDIATELY** with exact compiler errors.

### Gate 3: Architectural Integrity & Backend Invariants
- [ ] **Single-Instance Plugin**: Verify `tauri-plugin-single-instance` remains the FIRST registered plugin in `src-tauri/src/lib.rs`.
- [ ] **WebView2 UDF**: Verify no relative paths or paths inside `.cvh`/build folders are used for UDF.
- [ ] **SQLite WAL**: Verify all DB mutations in `src-tauri/src/db.rs` use explicit transactions (`conn.transaction()`).
- [ ] **Zero Naked Unwraps**: Verify no `.unwrap()` or `.expect()` calls in production paths; use typed errors (`Result<T, AppError>`).
- [ ] **Safe Quoting**: Verify Windows process executions use `quote_windows_path()` and `quote_windows_arg()`.

### Gate 4: Frontend & UI Invariants
- [ ] **Zustand Discipline**: Verify all array/object store selections use `useShallow` to prevent infinite re-render loops.
- [ ] **Flexbox Bounds**: Verify drawer and sidebar layouts chain `flex flex-col min-h-0 min-w-0 overflow-hidden` to avoid scrollbar collapse.
- [ ] **Design System Compliance**: Verify no arbitrary saturated colors are used for structural elements. Strict adherence to Vercel Light Paper (`#fafafa`, `#171717`, `#ebebeb`) and Obsidian Dark (`#090a0f`, `#0d1117`, `#21262d`, `#f0f6fc`).
- [ ] **Zoom Isolation**: Verify fullscreen overlays remain mounted outside the `useUiZoom` transform wrapper.

### Gate 5: Code Hygiene & Ponytail Minimalism
- [ ] Zero dead code, zero unused imports, zero console.log spam, zero leftover debug files.
- [ ] No placeholder mocks or stub functions in production logic.
- [ ] Atomic, self-contained git feature branch with clean commit history.

### Gate 6: Merge & Push Protocol (Upon Approval)
When all gates pass without caveats:
1. Ensure the task feature branch (`feat/<task-slug>`) is cleanly committed.
2. In the repository root, merge the branch into `main`:
   ```bash
   git checkout main
   git merge --no-ff feat/<task-slug> -m "Merge branch 'feat/<task-slug>' into main"
   ```
3. Push `main` to remote `origin`:
   ```bash
   git push origin main
   ```
4. Verify remote push success.
5. Invoke `advance_task(summary="Task approved, merged into main and pushed to origin/main.")` to move task to `ready`.

---

## 3. Rejection Protocol

If ANY gate fails:
- Immediately invoke `reject_task(reason)` with:
  1. Failed gate number and exact violation details.
  2. Concrete compiler/linter error output if applicable.
  3. Actionable checklist for the developer to resolve before resubmission.
- The task will automatically return to the `dev` stage.
