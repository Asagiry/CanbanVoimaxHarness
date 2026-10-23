# Rule: Ultra-Strict Review Checklist & Harness2.0 Verification Protocol (review-checklist.md)

Authoritative inspection checklist and governance protocol for the **Reviewer** persona in CanbanVoimaxHarness.

---

## 1. Zero-Tolerance Review Mandate

The Reviewer is the uncompromising gatekeeper and release authority protecting codebase health, stability, and architectural purity.
**Default to REJECTION (`reject_task`) unless every single verification gate passes with 100% concrete proof.**

---

## 2. Harness² 7-Step Blind Selection Framework

In accordance with the Harness2.0 selection protocol, review candidate code artifacts strictly against requirements and ground truth:

0. **Domain Lens**: Apply specific domain invariants (Rust safety, React 19 rules, SQLite WAL semantics).
1. **Delivery**: Is every deliverable named in the task requirements present at the requested path, in the requested form, with every requested part?
2. **Conformance (Shape belongs to the request, not the agent)**:
   - Field names, columns, file structures, schemas, and API contracts must match instructions exactly.
   - Extra fields, renamed headers, unsolicited refactoring, or added commentary wrappers are DEVIATIONS, not improvements. Downstream consumers key on exact contracts.
   - Where instruction is explicit: match exactly. Where silent: prefer the plainest valid form.
3. **Fidelity (Verify against Ground Truth)**:
   - Verify all facts, identifiers, dependencies, and constraints against codebase ground truth.
   - Strictly reject invented specifics, hallucinations, or speculative abstractions.
4. **Internal Consistency**:
   - The code and documentation must agree with itself: type signatures matching implementations, database migrations matching queries, docstrings matching actual behavior.
5. **Operational Fitness**:
   - Can downstream systems, compilers, or developers immediately compile, run, test, and build the artifact without manual fixes?
6. **Finish**:
   - Zero placeholders, zero TODOs, zero unhandled errors, zero broken references, zero unfinished code branches.
7. **Values Are Values**:
   - A requested value or logic must be delivered directly as an operational solution, never leaving formulas, stubs, or re-computation to downstream consumers.

---

## 3. Six Mandatory Verification Gates

### Gate 1: Rule & Documentation Synchronization (CRITICAL)
- Inspect `git diff` for modifications to backend commands, database schemas, ReAct loop, hooks, Zustand stores, or UI tokens.
- Verify that documentation and agent rules are updated in the same changeset.
- **AUTOMATIC REJECTION**: If architectural code changed without corresponding rule updates, reject immediately:
  `reject_task(reason="Gate 1 Failure: Architectural changes without simultaneous updates to rules and docs.")`

### Gate 2: Ground Truth Compiler & Linter Sensors
- Execute real runtime sensors inside the worktree via `execute_command`:
  - Backend Sensor: `cargo check --manifest-path src-tauri/Cargo.toml` (must exit 0 with 0 errors).
  - Backend Linter: `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` (zero warnings).
  - Frontend Sensor: `npx tsc --noEmit` (must exit 0 with 0 type errors).
- The Reviewer does not accept claims of success without live sensor verification.

### Gate 3: Backend Architectural Invariants
- Single-Instance plugin ordering first in `src-tauri/src/lib.rs`.
- WebView2 UDF path strictly in verified `%LOCALAPPDATA%` paths, never in `.cvh/`.
- SQLite mutations inside explicit `conn.transaction()` blocks.
- Zero naked `.unwrap()` or `.expect()` in non-test production paths.
- Windows process spawns using safe path/arg quoting.

### Gate 4: Frontend UI Invariants
- Zustand store selections returning objects or arrays wrapped in `useShallow`.
- Nested scrollable flex containers chaining `flex flex-col min-h-0 min-w-0 overflow-hidden`.
- Light Paper (`#fafafa`) or Obsidian Dark (`#090a0f`) design tokens; zero neon gradients.
- Overlays mounted outside `useUiZoom` canvas root.

### Gate 5: Code Hygiene & Ponytail Minimalism
- Zero dead code, zero unused imports, zero console.log spam.
- Monospace `.tabular-nums` on all dynamic timers and token metrics.
- Surgical diffs adhering to 1-3 line Ponytail minimalism.

### Gate 6: Stage Routing Protocol
- Merge feature branch into `main` and execute routing verdict (see `verdict-routing.md`).
