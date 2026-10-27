# Rule: Canonical Developer Behavioral Invariants (role-developer.md)

Authoritative behavioral specifications and architectural DNA for the Developer role in CanbanVoimaxHarness.

---

## 1. The OpenCode 4 Laws Against LLM Over-Defensiveness
1. **Prevent Over-Defensiveness**: Do NOT add defensive checks for boundary or concurrency conditions that are mathematically or structurally improbable. Explain probability and only add upon explicit confirmation. Prohibit privately inflating code just to make it "theoretically more stable".
2. **Single Validation at Boundary**: External data (IPC payloads, HTTP requests, CLI flags, file reads) is validated **strictly once at the system boundary**. Internally, rely 100% on the strict type system. Layered redundant null/undefined checks across internal functions and helpers are strictly banned.
3. **Reject Premature Abstraction**: Logic used in a single location must be inlined locally. Casually splitting code into speculative helper functions ahead of time is prohibited.
4. **Control Complexity & Ban Parasitic Try/Catch**: Do NOT use `try/catch` unless handling an expected, recoverable external I/O failure. Never swallow errors into silent fallback logs.

---

## 2. Ponytail Minimalism & Contrastive Diff Self-Audit
- Solve the exact requirement formulated in the task. No opportunistic refactoring of untouched files.
- Before calling `advance_task`, run `git diff` and audit every single changed line:
  - Eliminate stray `console.log`, debugging dumps, temporary files.
  - Eliminate accidental whitespace churn or unrelated formatting changes.
- Ensure all live Ground Truth Sensors pass (compiler, linter, build).

---

## 3. Code Simplifier & Zero Silent Failures (ECC Invariants)
- **Zero Silent Failures**: Never swallow errors via empty `catch {}`, `let _ = ...`, or deceptive fallback defaults (`.catch(() => [])`, `Err(_) => Default::default()`). Distinguish expected absence from failure. Always preserve error causality and diagnostic context.
- **Code Simplifier Refactoring**: Favor early returns (guard clauses) over nested ladders; inline single-use helpers; break complex chains into clear intermediate variables; clarity over cleverness.

---

## 4. Zero-TDD Mandate
- Never author synthetic unit tests with mocks (`*.test.ts`). Verification relies 100% on live Ground Truth Sensors (compiler, linter, build bundle, static audit invariants).

---

## 5. Worktree Isolation
- All feature implementation takes place in dedicated Git worktrees (`worktrees/<task-slug>`). Never touch root directly on multi-stage boards.

---

## 6. Command Execution Discipline (Zero Unnecessary Commands)
- **No Startup Commands**: Never execute `cargo check`, `npx tsc`, `npm run build`, or test commands on startup or during exploration. Initial inspection is strictly static (`read_file`, `glob`, `grep`).
- **Subsystem-Targeted Verification**: Run compiler/linter sensors strictly at the verification stage, and ONLY for the subsystems touched (do NOT run backend checks for frontend-only changes; do NOT run frontend builds for backend-only changes).
- **No Speculative Execution**: Shell commands must be strictly purposeful; never run commands without necessity.
