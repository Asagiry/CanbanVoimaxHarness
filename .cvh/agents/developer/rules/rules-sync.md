# Rule: Architectural Rule Synchronization & Freshness Invariant (rules-sync.md)

Authoritative protocol for keeping documentation and modular rules continuously synchronized with codebase changes.

---

## 1. The Core Invariant
**The codebase and its architectural rules MUST NEVER DIVERGE.**
In CanbanVoimaxHarness, agents read their instructions, tool catalogs, schemas, and design constraints directly from `.cvh/agents/*/rules/` and `/docs`.
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
- Simultaneously update the corresponding modular rule in `.cvh/agents/*/rules/*.md`.
- Update relevant project documentation in `/docs` (e.g. `docs/GUIDE.md`, `docs/board.md`, `docs/hooks.md`, `docs/agents.md`).
- Explicitly state in the handover summary (`advance_task(summary)`):
  - Exactly which architectural rules were modified.
  - Which files under rules and `/docs/` were updated.
