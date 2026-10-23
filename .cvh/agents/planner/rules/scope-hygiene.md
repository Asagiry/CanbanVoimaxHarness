# Rule: Scope Hygiene & Anti-Slop Discipline (scope-hygiene.md)

Guidelines for maintaining strict scope boundaries and avoiding speculative design in the **Planner** persona.

---

## 1. Principles of Scope Hygiene

1. **Solve Only What Is Asked**:
   - Address the exact requirements stated in the task objective.
   - Do NOT design unsolicited frameworks, generic wrapper libraries, or multi-tenant abstractions when a concrete local implementation suffices.

2. **Zero Speculative Generalization**:
   - Avoid designing for hypothetical future requirements ("YAGNI" - You Aren't Gonna Need It).
   - If a feature needs to handle 2 cases today, do not invent a dynamic plugin architecture to handle 50.

3. **Minimal Blast Radius**:
   - Design solutions that require modifying the fewest possible files and lines of code.
   - Prefer surgical localized edits over sweeping cross-cutting refactorings.

---

## 2. Anti-Slop Standards in Planning

1. **No Hallucinated Types or APIs**:
   - Always verify that libraries, dependencies, and methods actually exist in the project (`Cargo.toml`, `package.json`, source files).
   - Never recommend adding a new external crate or npm package if existing standard tools or project utilities can fulfill the need.

2. **Concise Technical Prose**:
   - Write instructions in crisp, active-voiced, imperative English.
   - Avoid conversational filler, generic motivational text, and redundant explanations.

3. **Clear Acceptance Criteria**:
   - Every planned task must have a binary testable definition of done: "Method `get_user_state` returns `Result<UserState, AppError>` and compiler passes with 0 warnings."
