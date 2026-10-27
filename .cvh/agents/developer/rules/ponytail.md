# JIT Rule: Minimal Surgical Diff and Over-Engineering Prevention (ponytail.md)

Ponytail engineering discipline for the **Developer** persona: achieving requirements via the shortest reliable path, eliminating architectural baggage, and preserving system robustness.

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
- **Rust Backend**: Strongly typed error handling via `Result<T, AppError>`. Zero `.unwrap()` or `.expect()` in production paths. SQLite WAL transaction boundaries with atomic transactions.
- **React Frontend**: Mandatory `useShallow` on Zustand store selectors returning arrays or objects. Clean timer and subscription teardowns in `useEffect`. Monospace `.tabular-nums` on dynamic telemetry and timers.


---

## 5. OpenCode 4 Anti-Overengineering Laws (Curing LLM Over-Defensiveness)
Language models suffer from pathological over-defensiveness: stuffing defensive null-checks everywhere, nesting parasitic `try/catch` blocks, and authoring premature abstractions. The developer persona enforces 4 non-negotiable laws:

1. **Law 1: Prevent Over-Defensiveness**:
   - For boundary or concurrency issues that are mathematically or structurally improbable, explain the likelihood and only add checks after explicit user confirmation.
   - Strictly prohibit privately inflating code just to make it "theoretically more stable".
2. **Law 2: Single Validation at Boundary (Entry-Point Guard)**:
   - External data (IPC payloads, HTTP requests, CLI flags, file reads) is validated **strictly once at the system boundary**.
   - Internally, rely 100% on the strict type system (TypeScript / Rust). Layered redundant null/undefined checks across internal functions and helpers are strictly banned.
3. **Law 3: Reject Premature Abstraction**:
   - For logic used only in a single location, inline it directly by the call site.
   - Casually splitting code into speculative helper functions or micro-utilities ahead of time is strictly prohibited.
4. **Law 4: Control Complexity & Ban Parasitic Try/Catch**:
   - Do not wrap code in `try/catch` unless handling an anticipated, recoverable external I/O error.
   - Never swallow errors into silent fallback logs. If logic can be handled clearly in a single function, do not forcibly decompose it into multiple fragmented layers.

---

## 6. Contrastive Diff Self-Audit
Before calling `advance_task(summary)` or declaring work complete:
- Execute `git diff` (and `git status`) inside the worktree.
- Scrutinize every line of changed code against the task requirements.
- Eradicate stray `console.log` statements, debugging dumps, temporary scratch files, and accidental formatting/whitespace churn.
- Ensure the diff contains ONLY the surgical solution to the requested task.
