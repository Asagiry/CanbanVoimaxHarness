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
4. **Tauri v2 Bundle & Resource Invariant**:
   - `tauri-build` in `build.rs` compiles Windows application resources (`.rc`). Icon assets defined in `tauri.conf.json` (`bundle.icon`) MUST exist on disk (`icons/32x32.png`, `icons/128x128.png`, `icons/icon.ico`). Never leave empty icon arrays or missing files during Tauri scaffolding.
5. **Windows Toolchain & Linker Invariant**:
   - When building on Windows, ensure the active toolchain (`x86_64-pc-windows-gnu` with LLVM-MinGW or `x86_64-pc-windows-msvc`) is configured with matching linkers. Ensure Cargo and toolchain bin directories are reachable in subshells.
6. **Reqwest Header Name Type Invariant**:
   - In `reqwest::header::HeaderMap::insert(K, V)`, the key `K` MUST implement `IntoHeaderName`. Use `HeaderName::from_static(...)` or `&'static str` literals. Never pass `HeaderValue` as the key.
7. **Closure Borrow & Move Invariant**:
   - When passing data collections (such as `HashSet` or `Vec`) to fallback closures (`unwrap_or_else`), avoid moving the binding if it is queried or reused in downstream logic. Explicitly clone (`.iter().cloned().collect()` or `.clone()`) to maintain zero-compiler-error hygiene.
