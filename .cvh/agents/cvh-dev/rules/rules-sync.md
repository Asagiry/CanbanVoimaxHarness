# Rule: Architectural Rule Synchronization & Freshness Invariant

Authoritative protocol for keeping `.cvh/rules/` and `/docs` continuously synchronized with codebase changes.

---

## 1. The Core Invariant
**The codebase and its architectural rules MUST NEVER DIVERGE.**
In CanbanVoimaxHarness, agents read their instructions, tool catalogs, schemas, and design constraints directly from `.cvh/rules/` and `/docs`.
If an engineer modifies code architecture without updating rules, downstream agents will operate on stale assumptions, causing regression cascades.

---

## 2. Developer Directive (Mandatory Rule Updates)

Whenever a developer introduces, modifies, or deprecates:
1. **Backend Commands & APIs**: New Tauri commands, changed signatures, altered response types in `src-tauri/src/commands/`.
2. **Database Schemas & Persistence**: Changes to tables, indices, migrations, or transaction semantics in `src-tauri/src/db.rs` or SQLite WAL mode.
3. **Agent Runtime Loop & Tools**: Modifications to tool definitions, compaction logic, prompt priming, or tool-policy behavior in `src-tauri/src/agent.rs`.
4. **Git & Hooks Lifecycle**: Changes to worktree handling, branch naming, hook triggers, environment variables (`CVH_*`), or sensors in `src-tauri/src/git.rs` / `hooks.rs`.
5. **Frontend Stores & State Flows**: New Zustand stores, changed state structures, optimistic update logic, or event listeners in `src/stores/`.
6. **UI Design Systems & Layouts**: New CSS tokens, changes to Vercel/Paper or Obsidian Dark themes, zoom isolation rules, or flexbox layouts.

**The Developer MUST:**
- Simultaneously update the corresponding modular rule in `.cvh/rules/*.md` (and mirrored in `.cvh/agents/*/rules/*.md`).
- Update relevant project documentation in `/docs` (e.g. `docs/GUIDE.md`, `docs/board.md`, `docs/hooks.md`, `docs/agents.md`).
- Explicitly state in the handover summary (`advance_task(summary)`):
  - Exactly which architectural rules were modified.
  - Which files under `.cvh/rules/` and `/docs/` were updated.

---

## 3. Reviewer Directive (Mandatory Rule Freshness Audit)

The Reviewer operates under a **Zero-Tolerance Audit Policy**:
1. **Inspect Git Diff for Rule Updates**:
   Before reviewing code implementation, inspect the touched files in the diff.
2. **Trigger Condition**:
   If the task modified backend architecture, database schemas, IPC commands, Zustand stores, tool policies, or UI design tokens:
   - Check if `.cvh/rules/` or `/docs/` were updated in the same commit / branch.
   - If architectural changes exist **WITHOUT** corresponding rule and documentation updates, the Reviewer **MUST REJECT IMMEDIATELY**:
     ```
     reject_task(reason="Architectural changes detected in [files] without corresponding updates to .cvh/rules/ and /docs. Update the modular rules to reflect the new architecture before review approval.")
     ```
3. **Verify Accuracy**:
   The Reviewer must verify that the updated rules accurately reflect the implementation and adhere to the English-only schema token economy.

---

## 4. No Outdated Docs & No Hallucinated Rules
- Any rule file that references deleted code, deprecated tools, or non-existent files must be pruned or refactored.
- Rules must be verifiable against source code (testable via sensors like `cargo check` and `npm run build`).
