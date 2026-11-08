---
description: Frontend entrypoints and topic-map router.
---
# Frontend map

React UI lives in `src/`. Start at the relevant route; load only the topic needed for the task. Known paths can be opened directly.

| Task | Rule to load |
| --- | --- |
| Boards, cards, stages, task actions | `load_rule("frontend_board_map")` |
| Conversation, tool cards, stream, effort selector, context/cost display | `load_rule("frontend_drawer_map")` |
| Providers, models, tool policy, personas and rules editor | `load_rule("frontend_settings_map")` |
| Cortex explorer, personas, rules and stage hooks | `load_rule("frontend_cortex_map")` |
| App shell, events, shared UI, translations and helpers | `load_rule("frontend_shared_map")` |

Exact file routes live only in the topic maps above — this router does not repeat them.

Data flow: UI → focused Zustand slices → Tauri IPC; typed backend events → `src/services/tauriEventGateway.ts` → stores → UI. Backend confirmation owns task lifecycle and IDs. Streaming and draft state live in drawer slices; reasoning effort (`ReasoningEffort` in `src/types/index.ts`) lives in board/cortex stores, not drawer slices.

For Rust behavior behind an IPC call, use `load_rule("backend_map")`. Keep exact file routes current in the relevant topic map when modules move. Verification policy belongs to the active role's `sensors` rule.
