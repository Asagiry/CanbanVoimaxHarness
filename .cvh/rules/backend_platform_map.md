---
description: Tauri bootstrap, IPC registration, logging and notifications.
---
# Desktop platform

Register commands and shared state in the Tauri entrypoint. Keep native notification and process handling platform-aware. Runtime failures propagate as typed errors through IPC.

| File | Responsibility |
| --- | --- |
| `src-tauri/src/commands/mod.rs` | Top-level IPC command module aggregation. |
| `src-tauri/src/commands/notifications.rs` | Native WinRT toasts with PowerShell fallback, AUMID registration. |
| `src-tauri/src/udf.rs` | Custom regex and search functions for SQLite connection. |
| `src-tauri/src/logger.rs` | Rotating file & terminal logging configuration. |
| `src-tauri/src/error.rs` | Domain errors and Tauri IPC error formatting. |
| `src-tauri/src/lib.rs` | Tauri invoke handler wiring and application setup. |
| `src-tauri/src/main.rs` | Windows subsystem bootstrap, panic hook, crash telemetry. |
