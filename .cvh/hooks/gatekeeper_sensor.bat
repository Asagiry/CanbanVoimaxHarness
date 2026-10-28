@echo off
setlocal enabledelayedexpansion
set "PATH=%USERPROFILE%\.cargo\bin;C:\Users\Voimax\.cargo\bin;%PATH%"

REM Ensure shared Cargo build cache across all worktrees to prevent cold compilations
if "%CARGO_TARGET_DIR%"=="" set "CARGO_TARGET_DIR=%~dp0..\..\src-tauri\target"

echo [CVH Sensor] Checking git mergeability with main...
git rev-parse --is-inside-work-tree >nul 2>&1
if not errorlevel 1 (
    REM Gate 6: Working tree must be clean and committed before handover (excluding gen artifacts)
    set "HAS_DIRTY="
    for /f "tokens=*" %%s in ('git status --porcelain -- ":!src-tauri/gen" 2^>nul') do (
        set "HAS_DIRTY=1"
    )
    if defined HAS_DIRTY (
        echo [CVH Sensor] ERROR: Uncommitted changes detected in working tree. >&2
        echo [CVH Sensor] ERROR: All files must be committed to the feature branch before handover [Gate 6]. >&2
        git status --short -- ":!src-tauri/gen" >&2
        exit /b 1
    )
    git rev-parse --verify main >nul 2>&1
    if not errorlevel 1 (
        git merge-tree --write-tree main HEAD >nul 2>&1
        if errorlevel 1 (
            echo [CVH Sensor] ERROR: Merge conflict detected against 'main'! Merge or rebase latest main before handoff. >&2
            exit /b 1
        )
    )
)

set "DIFF_AVAILABLE="
set "HAS_BACKEND_CHANGES="
set "HAS_FRONTEND_CHANGES="

git rev-parse --verify main >nul 2>&1
if not errorlevel 1 (
    git rev-parse --verify HEAD >nul 2>&1
    if not errorlevel 1 (
        set "DIFF_AVAILABLE=1"
        git diff --name-only main HEAD 2>nul | findstr /i "src-tauri Cargo.toml Cargo.lock" >nul 2>&1
        if not errorlevel 1 set "HAS_BACKEND_CHANGES=1"

        git diff --name-only main HEAD 2>nul | findstr /i "src/ index.html vite.config tsconfig package.json" >nul 2>&1
        if not errorlevel 1 set "HAS_FRONTEND_CHANGES=1"
    )
)

set "RUN_BACKEND=1"
set "RUN_FRONTEND=1"
if defined DIFF_AVAILABLE (
    if not defined HAS_BACKEND_CHANGES (
        set "RUN_BACKEND=0"
    )
    if not defined HAS_FRONTEND_CHANGES (
        set "RUN_FRONTEND=0"
    )
)

if "!RUN_BACKEND!"=="0" (
    echo [CVH Sensor] Backend unchanged, skipping cargo check.
) else (
    echo [CVH Sensor] Running Rust backend check...
    if exist "src-tauri\Cargo.toml" (
        cargo check --manifest-path "src-tauri\Cargo.toml"
        if errorlevel 1 (
            echo [CVH Sensor] ERROR: Backend cargo check failed! >&2
            exit /b 1
        )
    )
)

if "!RUN_FRONTEND!"=="0" (
    echo [CVH Sensor] Frontend unchanged, skipping TypeScript check.
) else (
    echo [CVH Sensor] Running TypeScript frontend typecheck...
    if exist "package.json" (
        call npx tsc --noEmit
        if errorlevel 1 (
            echo [CVH Sensor] ERROR: Frontend TypeScript check failed! >&2
            exit /b 1
        )
    )
)

echo [CVH Sensor] All gatekeeper sensors PASSED successfully.
exit /b 0
