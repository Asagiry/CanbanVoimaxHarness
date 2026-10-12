# 🚀 CanbanVoimaxHarness (CVH)

> **Next-Generation Lightweight Desktop Agentic IDE & Orchestrator**  
> Built with **Rust (Tauri v2)**, **React 19**, **TypeScript**, and **Tailwind CSS v4**.

---

## 🌟 Key Architecture & Differentiators

1. **📋 Kanban Board as the First-Class Control Plane**:
   - The board is not a plugin or a side view — it IS the primary user interface and execution orchestrator.
   - Tasks flow through customizable stage columns with real-time timers, returns tracking, and live agent consoles.

2. **🧠 Zero-Turn Memory Priming (Killer Feature)**:
   - Eliminates all token waste on instructing agents to "read rule files".
   - At Turn 0, Rust automatically pre-compiles `SystemMessage = Stage Persona + Project Rules (AGENTS.md) + Active Skills + Task Description`.
   - The agent starts Turn 1 with 100% domain knowledge already primed in its context memory.

3. **🔀 Dynamic, Arbitrary Graph Pipelines**:
   - Fully customizable columns defined in `.cvh/pipeline.json` (`A -> B -> C -> D -> E` or `Ideation -> Spec -> 3D -> QA -> Prod`).
   - Per-column context window caps (e.g. 400k for Dev, 200k for Review, 128k for Retro).
   - Per-column model overrides, reasoning efforts, and conditional routing on verdicts.

4. **🌲 Native Git Worktree Isolation**:
   - Managed natively by Rust via `std::process::Command` without shell injection or buffering deadlocks.
   - Every task executes inside an isolated `worktrees/<task_id>` directory.
   - Automated merge and sensor verification (`npm run build`, `cargo check`) upon `APPROVED`.

5. **⚙️ Custom Transition Hooks (.bat / .ps1 / .sh)**:
   - Configurable scripts that run on stage entry, stage exit, or task completion.
   - Injected with environment variables: `%CVH_TASK_ID%`, `%CVH_WORKTREE_PATH%`, `%CVH_BRANCH%`, `%CVH_VERDICT%`.

6. **⚡ Extreme Lightweight Performance**:
   - Compiles to a single native binary (~20MB).
   - Runs in ~40MB of RAM (vs 800MB in Electron).
   - Zero-lag SSE streaming and instant startup time.

---

## 📁 Repository Structure

```text
CanbanVoimaxHarness/
├── AGENTS.md                # 3-Tier Governance & Multi-Agent Engineering Manual
├── .cvh/                    # Project Dogfooding Pipeline & Persona Configuration
│   ├── config.json          # Project endpoints & model definitions
│   ├── pipeline.json        # Dynamic pipeline columns & routing graph
│   ├── agents/              # Persona system prompts (developer, reviewer, retro)
│   └── hooks/               # Transition lifecycle scripts (on_enter_dev, on_finish_done)
├── .agents/
│   ├── skills/              # Core Development Skills
│   │   ├── development/
│   │   ├── agent-developer/
│   │   ├── agent-developer-reviewer/
│   │   ├── agent-retro-verifier/
│   │   ├── tauri-rust-engineer/
│   │   ├── react-tailwind-artisan/
│   │   └── llm-streaming-expert/
│   ├── tasks-history.json   # Machine-readable task ledger
│   └── knowledge-graph.json # Architectural MCP knowledge graph
├── src-tauri/               # Rust Desktop Core (Tauri v2)
└── src/                     # React 19 Frontend (TypeScript, Tailwind v4, Lucide)
```
