---
description: Frontend entrypoints and topic-map router.
---
# Frontend map

React UI lives in `src/`. Start at the relevant route; load only the topic needed for the task. Known paths can be opened directly.

| Task | Rule to load | Start here |
| --- | --- | --- |
| Boards, cards, stages, task actions | `load_rule("frontend_board_map")` | `src/components/board/KanbanBoard.tsx` |
| Conversation, tool cards, stream, context/cost display | `load_rule("frontend_drawer_map")` | `src/components/drawer/DrawerActivityStream.tsx` |
| Providers, models, tool policy, personas and rules editor | `load_rule("frontend_settings_map")` | `src/components/settings/SettingsModal.tsx` |
| App shell, events, shared UI, translations and helpers | `load_rule("frontend_shared_map")` | `src/App.tsx` |

Data flow: UI → focused Zustand slices → Tauri IPC; typed backend events → `src/services/tauriEventGateway.ts` → stores → UI. Backend confirmation owns task lifecycle and IDs. Streaming and draft state live in drawer slices.

For Rust behavior behind an IPC call, use `load_rule("backend_map")`. Keep exact file routes current in the relevant topic map when modules move. Verification policy belongs to the active role's `sensors` rule.
