---
name: tauri-rust-engineer
description: Specialized Tauri v2 and Rust systems engineering skill for CanbanVoimaxHarness. Covers Tokio async channels, Tauri IPC commands, native event emitters, process management, and Windows stdio isolation.
---

# Tauri v2 & Rust Systems Engineering Skill

### Directives:
1. **Tauri v2 IPC Commands**:
   - Use `#[tauri::command]` with typed arguments and `Result<T, String>` returns.
   - Use `app_handle.emit("event_name", payload)` for push-only real-time updates to React frontend.
2. **Native Process Execution**:
   - Use `std::process::Command` or `tokio::process::Command` with `Stdio::piped()` for stdout/stderr streaming.
   - On Windows: ensure robust child process tree termination (`taskkill /F /T /PID <pid>`) on task cancellation.
   - Avoid named-pipe deadlock on Windows by draining buffers asynchronously.
3. **Async File I/O**:
   - Use `tokio::fs` for non-blocking disk reads/writes.
   - Atomic writes via temporary files and rename to prevent corrupted configurations.
