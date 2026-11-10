---
description: Verification evidence policy, presumption of guilt, and line-by-line review standards.
---
# Rule: Ultra-Strict Review Checklist & Harness2.0 Verification Protocol (review-checklist.md)

Authoritative inspection checklist and governance protocol for the **Reviewer** persona in CanbanVoimaxHarness.

---

## 1. Zero-Tolerance Review Mandate

The Reviewer is the uncompromising gatekeeper and release authority protecting codebase health, stability, and architectural purity.
**Default to REJECTION (`reject_task`) unless every single verification gate passes with 100% concrete proof.**

---

## 2. Harness² 7-Step Blind Selection Framework

In accordance with the Harness2.0 selection protocol, review candidate code artifacts strictly against requirements and ground truth:

0. **Domain Lens**: Apply specific domain invariants (Rust safety, React 19 rules, SQLite WAL semantics, Uncle Bob Clean Code).
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
   - Can downstream systems, compilers, or developers immediately compile, test, and run without manual fixes?
6. **Finish**:
   - Zero placeholders, zero TODOs, zero unhandled errors, zero broken references, zero unfinished code branches.
7. **Values Are Values**:
   - A requested value or logic must be delivered directly as an operational solution, never leaving formulas, stubs, or re-computation to downstream consumers.

---

## 3. Six Mandatory Verification Gates

### Gate 1: Rule Hygiene, Anti-Changelog & Navigation Map Sync Gate
- Rule hygiene (timeless contracts, never changelogs, when to update rules) is owned by `rules-sync.md` §1–2 — apply it here by reference, do not restate it. Reject if developer pollutes rule files with PR changelogs or transient bugfix notes.
- **Navigation Map Synchronization**: If the commit adds, deletes, moves, or renames files or modules in `src-tauri/src/` or `src/`, verify the relevant topic map in `.cvh/rules/` was updated. Update `backend_map.md` or `frontend_map.md` only when their topic routes change. Reject missing or incorrect file routes, not the absence of an unrelated router edit.

### Gate 2: Verification Evidence for the Changed Scope
- Require successful verification for affected subsystems. Frontend checks are `pnpm test`, `pnpm exec tsc --noEmit`, and `pnpm exec oxlint --deny-warnings`; backend checks are `mbx check --manifest-path src-tauri/Cargo.toml` and `mbx clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`, with targeted tests when changed behavior needs them. Do not run checks for untouched subsystems without a concrete cross-system risk.
- Accept existing command logs and exit codes tied to the reviewed commit when relevant sources, dependencies, configuration, toolchain, and environment are unchanged. A bare claim that checks passed is not evidence.
- Run only checks whose evidence is missing, failed, stale, or unverifiable, or whose scope does not cover a concrete risk found in the diff. State why each additional check is needed. Entering review alone does not invalidate successful verification.
- For documentation and prompt-only changes, inspect the diff, references, and affected contracts; compiler/test runs are needed only if executable behavior or configuration is affected.
- **Windows Command Hygiene**: Owned by `sensors.md` §1.4 (developer rule) — reviewer applies it by reference when running checks; do not restate the banned pipeline list here.

### Gate 3: Backend Architectural Invariants & Uncle Bob Clean Architecture
- **Single Responsibility Principle (SRP)**: Code strictly partitioned into `db/` (repositories & pool), `config/` (I/O, models, scaffold, tool policy), `domain/` (Value Objects & state machines), and `agent/` (ReAct engine, compactor, tools). The remaining top-level modules are first-class partitions, not exceptions: `commands/` (Tauri IPC handlers), `git/` (git operations), `llm/` (provider client/cache/models), plus root modules `hooks.rs`, `udf.rs`, `logger.rs`, `error.rs`, `lib.rs`, `main.rs`.
- **Backend File Size Ceiling (<= 500 lines)**: All backend Rust modules must strictly remain <= 500 lines. All 21 historical backend monoliths (including `agent.rs`, `compactor.rs`, `transitions.rs`, `models.rs`, `runner.rs`, `lifecycle.rs`, `client.rs`, `boards.rs`, etc.) were decomposed into modular submodules in CVH-069. Files touched or modified in or after CVH-069 are NOT exempt from line ceilings under legacy debt immunity. Pre-existing immunity (§4.1) applies solely to historically untouched legacy files.
- **Elimination of Primitive Obsession**: Mandatory use of Domain Value Objects (`domain::ids::TaskId`, `StageId`, `BoardId`) instead of naked strings.
- **Make Illegal States Unrepresentable**: Mandatory use of Algebraic Data Types (`domain::state::TaskState`) for lifecycle tracking. No conflicting booleans.
- Single-Instance plugin ordering first in `src-tauri/src/lib.rs`.
- WebView2 UDF path strictly in verified `%LOCALAPPDATA%` paths, never in `.cvh/`.
- SQLite mutations inside explicit `conn.transaction()` blocks with parameterized bindings (`params![...]`).
- Zero naked `.unwrap()` or `.expect()` in non-test production paths.
- Zero silent failures: reject `let _ = ...` or silent `.ok()` conversions on fallible I/O or DB operations.
- Windows process spawns using safe path/arg quoting (`quote_windows_path()`, `quote_windows_arg()`).

### Gate 4: Frontend UI Invariants, Size & Testing Gates
- **Strict Ban on Direct `listen<any>`**: Direct `listen(...)` or unvalidated `any` event listening is strictly banned. ALL Tauri event subscriptions MUST route through `src/services/tauriEventGateway.ts`. Sole exception: `src/components/layout/WindowControls.tsx:50` `win.onResized` (Tauri window API, not an event-gateway subscription).
- **Component File Size Ceiling (<= 350 lines)**: React components and store slices must strictly remain <= 350 lines. All 12 historical frontend monoliths (including `tauriEventGateway.ts`, `tasksCrudSlice.ts`, `SubagentToolCard.tsx`, `AgentSettingVisualEditor.tsx`, `AgentConsoleDrawer.tsx`, `SubagentsSidebar.tsx`, `StageContextMenu.tsx`, `ImageDiffViewer.tsx`, `FileOpsToolCard.tsx`, `DrawerSteerBar.tsx`, `GitDiffSidebar.tsx`, `KanbanBoard.tsx`) were decomposed into modular subcomponents and slices in CVH-069. Files touched or modified in or after CVH-069 are NOT exempt from line ceilings under legacy debt immunity. Pre-existing immunity (§4.1) applies solely to historically untouched legacy files.
- **Vitest Unit Test Coverage**: Pure utilities in `src/lib/`, `src/types/`, and store slices in `src/stores/` must have matching Vitest unit tests in `__tests__/`.
- Zustand store selections returning objects or arrays wrapped in `useShallow`.
- Nested scrollable flex containers chaining `flex flex-col min-h-0 min-w-0 overflow-hidden`.
- Design tokens per `antislop.md` §1.1 (single owner of palette and legacy-token ban).
- Overlays mounted outside `useUiZoom` canvas root.
- Zero silent failures: reject empty `catch {}` blocks or dangerous fallbacks (`.catch(() => [])`, `.catch(() => null)`).
- Redundant copy elimination and Tri-partite experience synergy (see `frontend-invariants.md` §9–§10).

### Gate 5: Code Hygiene, Simplifier & Ponytail Minimalism
- Zero dead code, zero unused imports, zero console.log spam.
- Numeric formatting per `antislop.md` §1.4 (single owner of `tabular-nums`).
- Surgical diffs adhering to Ponytail minimalism.
- Code Simplifier: favor early returns over deep nested ladders; unwind single-use pass-through wrapper functions.

### Gate 6: Clean Merge, Origin Integration & Stage Routing Protocol
- **Branch Diff Inspection Mandate**: The Reviewer MUST inspect the branch diff using `git diff main...HEAD` (or `git log -p main..HEAD`). A bare `git diff` only inspects uncommitted working tree changes, which produces a deceptive empty diff when developer changes are committed.
- **Clean Mergeability Check (reviewer verify)**: Independently verify the developer's pre-flight evidence (`git merge-tree --write-tree main HEAD` exit 0, clean `git status --porcelain`) against the committed HEAD; reject with `CHANGES_REQUESTED` if conflicts exist or evidence is stale. Developer pre-flight procedure is owned by `sensors.md` §2.5 — this gate only verifies, it does not restate the procedure.
- **Merge & Origin Integration**: The actual merge into `main` and push to remote `origin` is executed automatically by the transition hook (`.cvh/hooks/on_review_finish.bat`) upon advancing the task. The Reviewer executes stage routing via `advance_task(stage_id="ready" | "retro")` (see `verdict-routing.md`).

---

## 4. Anthropic Claude Code Convergence Protocol & Reviewer Guardrails

To prevent infinite review cycles, nitpicking loops, and cognitive drift between agent author and agent reviewer:

### 4.1 Immunity to Pre-Existing Historical Bugs
- The Reviewer is **strictly forbidden from failing a task (`reject_task`) for bugs, stylistic choices, or architectural debt that existed prior to the current task's branch point** (specifically in untouched legacy code).
- **Scope of Immunity**: In CVH-069, all historical monoliths were refactored. Any file touched, refactored, or newly introduced in or after CVH-069 is subject to strict line ceilings (<=500 lines Rust, <=350 lines TS/React) and clean architecture standards without exception. Immunity applies exclusively to files untouched by the current task that existed prior to its branch point.
- If a pre-existing defect or improvement opportunity is discovered in untouched surrounding code:
  - Do NOT issue `CHANGES_REQUESTED`.
  - Record it as an informative non-blocking note or recommend creating a separate backlog card.
  - Never hold the current PR hostage for untouched legacy code.

### 4.2 Diff-Only Re-Review Protocol
- On follow-up review cycles (after a developer addresses prior rejection feedback):
  - The Reviewer audits **strictly the new diff** produced to fix the requested items.
  - Do NOT re-audit untouched portions of the codebase or raise brand-new unrelated objections on files approved in round 1.

### 4.3 Actionable Line-by-Line References (`REVIEW.md` Invariant)
- Every objection in a rejection (`reject_task`) MUST provide:
  1. Exact `file_path` and concrete line numbers (`path/to/file.ext:42-45`).
  2. The exact architectural invariant or failure mode violated.
  3. Concrete, unambiguous remediation steps.
- Abstract philosophical complaints (e.g. *"consider reconsidering the design pattern"*) or vague unanchored criticisms are strictly prohibited.

### 4.4 Anti-Nitpicking & Style Tolerance
- If code passes all mechanical linters and compilers (`cargo clippy`, `oxlint`, `tsc`), the Reviewer MUST NOT reject tasks based on subjective naming preferences, personal stylistic tastes, or micro-optimizations that do not impact runtime correctness or performance.
