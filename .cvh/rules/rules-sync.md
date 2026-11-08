---
description: Architectural rule hygiene, timeless contracts invariant, and ban on commit-log rule pollution.
---
# Rule: Architectural Rule Hygiene & Timeless Contracts (rules-sync.md)

Authoritative protocol for maintaining clean, timeless architectural rules in `.cvh/`.

---

## 1. The Core Invariant: Rules are Timeless Architectural Contracts
Rules in `.cvh/` define permanent system standards, runtime invariants, and component boundaries.
**They are NEVER task changelogs, release notes, or bugfix diaries.**

Code is authoritatively described by its types, tests, and interfaces. Prose rules exist solely to enforce universal invariants and system-wide constraints that compilers and linters cannot express alone.

---

## 2. When to Update Rules
Update rules **ONLY** when:
1. **Universal Architecture Shift**: A fundamental architectural redesign of a core subsystem (e.g. replacing SQLite with an alternative engine, changing desktop windowing architectures, rewriting the agent ReAct loop).
2. **Public Contract Invariants**: Adding or breaking global architectural contracts that all agents across the system must strictly obey.
3. **Architectural Map & Subsystem Synchronization**: Whenever adding, removing, renaming, or relocating modules or files in `src-tauri/src/` or `src/`, update the relevant topic map in `.cvh/rules/` in the same diff/commit. Update `backend_map.md` or `frontend_map.md` only when their topic routes change. Navigation maps must remain faithful to the filesystem.

Do **NOT** update rules for:
- Routine feature implementations, normal UI screens, or standard bugfixes.
- Minor command additions or refactorings that stay within existing architectural patterns.
- Task changelogs, commit messages, or PR summaries.

---

## 3. Anti-Changelog Discipline
- Never append PR summaries or task-specific change histories to `.cvh/agents/*/rules/*.md`.
- Keep rules concise, prescriptive, and timeless.
- Code describes itself through strict types and comprehensive tests, not transient prose in rule files.

## 4. Boundary with Retro Rule Evolution
- `retro/rules/rule-evolution.md` is the sole exception to the ban above: it may record task IDs, commit hashes, and rejection quotes **only** as `Evidence` lines inside CLE entries (Cue — Lesson — Evidence, max 3–4 lines per entry), and only when the lesson is a durable, timeless invariant.
- This exception does not authorize appending task histories, PR summaries, or bug diaries to any other rule file, nor to `rule-evolution.md` outside CLE format. If an entry cannot be phrased as a timeless invariant with a concrete trigger, it does not belong in a rule file at all.
