# JIT Rule: Minimal Surgical Diff and Over-Engineering Prevention (ponytail.md)

This rule establishes the Ponytail engineering discipline for CanbanVoimaxHarness: achieving requirements via the shortest reliable path, eliminating architectural baggage, and preserving system robustness.

---

## 1. Minimal Diff Invariant (1–3 Lines)
- If a bug or feature can be resolved by editing 1–3 lines in an existing file, implement the solution strictly in place.
- Do not create new classes, contexts, custom hooks, helper modules, or service layers if the logic naturally belongs inside an existing component.
- Creating a new file requires explicit justification that the responsibility exceeds the existing file's boundary.

---

## 2. Ban on Unwarranted Dependencies
- Do not add external crates to `src-tauri/Cargo.toml` or npm packages to `package.json` without explicit approval.
- Standard capabilities must rely on:
  - Rust standard library, Tokio, Tauri API, rusqlite, serde.
  - React 19, Zustand, Tailwind CSS v4, Lucide Icons.
- If a utility can be implemented in 10–20 lines of clean code, do not install a third-party dependency for a single function call.

---

## 3. Ban on Speculative Abstractions (Zero Future-Proofing)
- Author code strictly for current task requirements.
- Strictly prohibited:
  - Abstract factories, single-implementation traits, and pass-through interfaces.
  - Unused function arguments added "for future flexibility".
  - Speculative configuration flags, switches, and branching scenarios for unassigned requirements.
  - Wrapping concrete types in complex multi-layer generics without immediate necessity.

---

## 4. Preservation of System Robustness
Ponytail simplification governs solution structure, never compromising runtime reliability:
- **Rust Backend**:
  - Strongly typed error handling via `Result<T, AppError>`. Zero `.unwrap()` or `.expect()` in production paths.
  - SQLite WAL transaction boundaries with atomic transactions.
  - Supervised child process management with clean Tokio resource cleanup.
- **React Frontend**:
  - Mandatory `useShallow` on Zustand store selectors returning arrays or objects.
  - Clean timer and subscription teardowns in `useEffect`.
  - Monospace `.tabular-nums` on dynamic telemetry and timers.

---

## 5. Pre-Commit Verification Checklist
Before submitting code to the Review stage:
- [ ] Has `git diff` been inspected for accidental formatting changes or unintended edits?
- [ ] Can the count of modified files be minimized without architectural harm?
- [ ] Have any unwarranted dependencies been introduced into manifests?
- [ ] Does every newly added type, function, or field have active call sites?
