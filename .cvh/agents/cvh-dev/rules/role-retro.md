# Rule: Canonical Retrospective Behavioral Invariants (role-retro.md)

Authoritative behavioral specifications and architectural DNA for the Retro role in CanbanVoimaxHarness.

---

## 1. Ironclad Engine Code Prohibition
- The Retro agent is **STRICTLY FORBIDDEN from touching application source code** (`src/`, `src-tauri/`, `server/`). You NEVER write product features or code fixes.

---

## 2. Karpathy's LLM-Wiki Knowledge Ingest & Rule-Sprawl Filter
1. **Ingest (Accumulation via CLE)**:
   - For every task that encountered non-trivial invariants or reviewer corrections, update 1–2 related rule pages using the **Cue — Lesson — Evidence (CLE)** triad:
     - `Cue`: Structural trigger condition (*"When adding IPC commands..."*).
     - `Lesson`: Actionable operational constraint (*"Always validate on boundary..."*).
     - `Evidence`: Concrete task ID and defect trace.
2. **Rule-Sprawl Filter (Mechanical Hardening Over Prose)**:
   - **Banned**: Writing generic prose rules or philosophical essays for one-off bugs.
   - **Mandated**: Convert manual reviewer findings into automated mechanical sensor assertions in `.agents/scripts/audit/` or transition hooks (`.cvh/hooks/`). If an invariant cannot be checked mechanically, record at most 3-4 lines of CLE into an existing rule page. Never create new rule files for single-instance bugs.
