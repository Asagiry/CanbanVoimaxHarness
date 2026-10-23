@echo off
setlocal enabledelayedexpansion

REM ==============================================================================
REM CanbanVoimaxHarness Hook: create_worktree.bat
REM Triggered on backlog on_leave to initialize an isolated git worktree for the task
REM ==============================================================================

if "%CVH_TASK_ID%"=="" (
    echo [CVH Worktree Hook] ERROR: CVH_TASK_ID environment variable is missing >&2
    exit /b 1
)

if /i "%CVH_SKIP_WORKTREE%"=="true" (
    echo [CVH Worktree Hook] CVH_SKIP_WORKTREE is true. Skipping worktree creation.
    exit /b 0
)

set "SLUG=%CVH_TASK_ID%"
set "WT_DIR=worktrees\%SLUG%"
set "BRANCH=feat/%SLUG%"

echo [CVH Worktree Hook] Task: %CVH_TASK_ID% ^| Worktree: %WT_DIR% ^| Branch: %BRANCH%

REM Ensure worktrees root folder exists
if not exist "worktrees" (
    mkdir "worktrees" >nul 2>&1
)

REM Check if worktree directory already exists
if exist "%WT_DIR%" (
    echo [CVH Worktree Hook] Worktree directory '%WT_DIR%' already exists.
) else (
    REM Check if git branch already exists
    git rev-parse --verify "%BRANCH%" >nul 2>&1
    if not errorlevel 1 (
        echo [CVH Worktree Hook] Branch '%BRANCH%' already exists. Attaching worktree...
        git worktree add "%WT_DIR%" "%BRANCH%"
        if errorlevel 1 (
            echo [CVH Worktree Hook] ERROR: Failed to add existing branch to worktree >&2
            exit /b 1
        )
    ) else (
        echo [CVH Worktree Hook] Creating new branch '%BRANCH%' and worktree '%WT_DIR%'...
        git worktree add -b "%BRANCH%" "%WT_DIR%" HEAD
        if errorlevel 1 (
            echo [CVH Worktree Hook] ERROR: Failed to create worktree '%WT_DIR%' with branch '%BRANCH%' >&2
            exit /b 1
        )
    )
)

REM Standard CVH output protocol: runtime captures these lines to assign task.worktree_path and task.branch
echo CVH_WORKTREE=%WT_DIR%
echo CVH_BRANCH=%BRANCH%
echo [CVH Worktree Hook] Worktree successfully initialized.
exit /b 0
