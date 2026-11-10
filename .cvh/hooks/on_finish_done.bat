@echo off
setlocal enabledelayedexpansion

REM ==============================================================================
REM CanbanVoimaxHarness Hook: on_finish_done.bat
REM Triggered when a task moves to done / ready for automated worktree & branch cleanup
REM ==============================================================================

echo [CVH Done Hook] Cleaning up task: %CVH_TASK_ID% ^| Branch: [%CVH_BRANCH%] ^| Worktree: [%CVH_WORKTREE_PATH%]

set "TARGET_WT=%CVH_WORKTREE_PATH%"
if "%TARGET_WT%"=="" if not "%CVH_TASK_ID%"=="" (
    if exist "worktrees\%CVH_TASK_ID%" set "TARGET_WT=worktrees\%CVH_TASK_ID%"
)

if not "%TARGET_WT%"=="" if exist "%TARGET_WT%" (
    echo [CVH Done Hook] Tearing down worktree: %TARGET_WT%
    git worktree remove "%TARGET_WT%" --force >nul 2>&1
    git worktree prune >nul 2>&1
    if exist "%TARGET_WT%" rmdir /s /q "%TARGET_WT%" >nul 2>&1
)

if not "%CVH_BRANCH%"=="" if /i not "%CVH_BRANCH%"=="main" if /i not "%CVH_BRANCH%"=="master" (
    git rev-parse --verify "%CVH_BRANCH%" >nul 2>&1
    if not errorlevel 1 (
        git branch -D "%CVH_BRANCH%" >nul 2>&1
    )
)

echo [CVH Done Hook] Worktree cleanup completed for task: %CVH_TASK_ID%
exit /b 0
