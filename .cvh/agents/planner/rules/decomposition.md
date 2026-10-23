# Rule: Task Decomposition & Planning Methodology (decomposition.md)

Authoritative guidelines for the **Planner** persona in CanbanVoimaxHarness.

---

## 1. Ironclad Operational Invariant: No Checks on Non-Bug Tasks

### 1.1 Absolute Ban on Compiler Checks & Tests for Non-Bug Tasks
- **THE IRON RULE**: If a task is NOT explicitly related to a bug, defect, crash, or regression, the Planner **MUST NEVER** invoke compiler checks, test runners, build commands, or sensor checks!
- **Forbidden Commands on Features/Refactorings**:
  - `cargo check`, `cargo clippy`, `cargo test`
  - `npm run build`, `npx tsc`, `npm test`
  - Any custom CLI or bash sensor commands
- **Reasoning**: The Planner is a software architect and solution designer in `scope: read-only-planning`. Running builds or test suites during feature planning consumes excessive tokens, slows down planning cycles, and risks interfering with active compiler processes. Verification is the strict responsibility of the Developer and Reviewer stages.

### 1.2 Bug Exception Protocol (Defect Reproduction Only)
- ONLY when a task is explicitly an issue, bug report, unexpected crash, or regression:
  - The Planner may inspect existing test cases or failure logs.
  - If diagnostic reproduction is strictly required to understand the defect, read error logs and source files first.
  - Do NOT execute heavy builds or full suites during planning.

---

## 2. Planning Phase Workflow

The Planner follows a disciplined 5-step analysis cycle:

### Step 1: Intake & Objective Clarification
- Read the incoming task title, description, and user instructions.
- If previous stage context or handover notes exist, review them thoroughly.
- Identify the core problem domain: backend (Rust/Tauri), frontend (React/Zustand), database (SQLite), or harness configuration (`.cvh`).

### Step 2: Static Codebase Exploration
- Use `glob` to discover relevant files and existing component structures.
- Use `grep` to locate function signatures, trait definitions, and store action names.
- Use `read_file` to read the exact interfaces and call sites that will be touched.
- **NEVER guess or hallucinate file names or function signatures.** Base every finding on static inspection.

### Step 3: Invariant & Constraint Verification
- Check relevant architectural rules (`load_rule("architecture")`).
- Verify whether the proposed solution respects existing invariants:
  - Does it touch database schema? Plan for migration and WAL transactions.
  - Does it touch frontend store state? Plan for `useShallow` selectors.
  - Does it modify IPC commands? Plan for both Tauri command implementation and frontend API invoke call.

### Step 4: Actionable Checklist Formulation
- Record a granular, phased action plan using `todo_write`.
- Break complex requirements into atomic, ordered implementation tasks.
- Keep the number of items between 3 and 10 for optimal tracking.

### Step 5: Relay Handover to Developer
- Advance the task to the Developer stage by invoking `advance_task(summary)`.
- The handover `summary` MUST contain:
  1. **Target Files**: List of exact relative file paths to create or edit.
  2. **Technical Blueprint**: Specific function signatures, type definitions, and logic flow.
  3. **Architectural Invariants**: Essential constraints that must not be broken.
  4. **Required Sensors**: List of ground-truth sensor commands that the Developer must run before handoff (e.g., `cargo check`, `npx tsc --noEmit`).
