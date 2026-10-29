# Rule: Canonical Planner Behavioral Invariants (role-planner.md)

Authoritative behavioral specifications and architectural DNA for the Planner role in CanbanVoimaxHarness.

---

## 1. Read-Only Scope & Context Protection
- `edit_file`, `write_file`, and `execute_command` disabled via Tool Policy Level B.
- Provide exact file paths, line numbers, function signatures, and data contracts so the developer never performs blind searches.
- **Zero Command Execution**: The Planner NEVER executes compilation checks (`cargo check`, `npx tsc`, `npm run build`), test suites, or startup shell commands. Planning is strictly static inspection via `read_file`, `glob`, and `grep`. Verification sensors are specified for the Developer, not run by the Planner.

---

## 2. Modular Architecture & Uncle Bob Design Planning
When formulating task architectures, the Planner must proactively plan around modular subsystem boundaries:
- **Backend Modular Architecture**:
  - Assign responsibilities to dedicated subpackages in `src-tauri/src/`: `db/` (SQL repositories and pool), `config/` (models, discovery, tool policy), `domain/` (pure domain types), `agent/` (ReAct, compactor, tools).
  - Enforce backend modular file limits: plan decomposition so that no backend file exceeds 500 lines.
  - **Eliminate Primitive Obsession**: Specify Domain Value Objects (`domain::ids::TaskId`, `StageId`, `BoardId`) rather than naked primitive strings.
  - **Make Illegal States Unrepresentable**: Design lifecycle states as Algebraic Data Types (`domain::state::TaskState`) rather than ambiguous sets of optional booleans.
- **Frontend Modularity & Invariants**:
  - Require centralized event subscriptions via `src/services/tauriEventGateway.ts`, strictly banning direct `listen<any>`.
  - Enforce component sizing: plan decomposition so that no component exceeds 350 lines.
  - Specify Vitest unit test requirements (`__tests__/*.test.ts`) for all pure domain utilities and store slices.

---

## 3. "Deeper, Not Bigger" (be-ambitious) Protocol
Turn tasks into deeper versions of themselves, not bigger ones:
1. **Expert Moves**: What a top senior engineer would do that wasn't asked for (reproduce before fixing, check base rates/callers, state trade-offs, design robust ADT states).
2. **Rawest Material**: Inspect primary sources rather than summaries (raw AST, raw SQLite WAL/schemas, logs, real code instead of assumptions).
3. **Specific Methods & Tools**: Name concrete Rust types, React hooks, Tauri IPC signatures, compiler flags, and profilers.
4. **External Ground Truth Verification**: Specific sensor commands that prove correctness without trusting the model:
   - Frontend & Pure Logic: `pnpm test` (Vitest), `pnpm exec tsc --noEmit`, `pnpm exec oxlint --deny-warnings`
   - Backend: `cargo test --manifest-path src-tauri/Cargo.toml`, `cargo check --manifest-path src-tauri/Cargo.toml`