@echo off
setlocal
set "PATH=%USERPROFILE%\.cargo\bin;C:\Users\Voimax\.cargo\bin;%PATH%"

echo [CVH Sensor] Running Rust backend check...
if exist "src-tauri\Cargo.toml" (
    cargo check --manifest-path "src-tauri\Cargo.toml"
    if errorlevel 1 (
        echo [CVH Sensor] ERROR: Backend cargo check failed! >&2
        exit /b 1
    )
)

echo [CVH Sensor] Running TypeScript frontend typecheck...
if exist "package.json" (
    call pnpm exec tsc --noEmit
    if errorlevel 1 (
        echo [CVH Sensor] ERROR: Frontend TypeScript check failed! >&2
        exit /b 1
    )
)

echo [CVH Sensor] All gatekeeper sensors PASSED successfully.
exit /b 0
