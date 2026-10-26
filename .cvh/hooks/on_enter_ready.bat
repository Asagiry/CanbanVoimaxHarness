@echo off
setlocal enabledelayedexpansion
REM ==============================================================================
REM CanbanVoimaxHarness Hook: on_enter_ready.bat
REM Safety hook ensuring approved task branch is merged and pushed to origin/main
REM Automates clean worktree teardown (§ 2.23)
REM ==============================================================================

echo [CVH Ready Hook] Task: %CVH_TASK_ID% ^| Verdict: [%CVH_VERDICT%] ^| Branch: [%CVH_BRANCH%]

if "%CVH_VERDICT%" NEQ "APPROVED" if "%CVH_VERDICT%" NEQ "APPROVED_WITH_CAVEATS" (
    echo [CVH Ready Hook] Verdict is not APPROVED. Skipping merge.
    exit /b 0
)

if "%CVH_BRANCH%"=="" (
    goto TEARDOWN
)

REM Verify feature branch exists
git rev-parse --verify "%CVH_BRANCH%" >nul 2>&1
if errorlevel 1 (
    goto TEARDOWN
)

REM Check if already merged
git merge-base --is-ancestor "%CVH_BRANCH%" main >nul 2>&1
if not errorlevel 1 (
    echo [CVH Ready Hook] Branch is already merged into main.
    git push origin main >nul 2>&1
) else (
    git checkout main >nul 2>&1
    git merge --no-ff "%CVH_BRANCH%" -m "feat(%CVH_TASK_ID%): %CVH_TASK_TITLE% [approved]" >nul 2>&1
    git push origin main >nul 2>&1
    echo [CVH Ready Hook] Successfully verified and pushed main.
)

:TEARDOWN
REM Worktree Teardown Protocol (§ 2.23)
set "TARGET_WT=%CVH_WORKTREE_PATH%"
if "%TARGET_WT%"=="" if not "%CVH_TASK_ID%"=="" (
    if exist "worktrees%CVH_TASK_ID%" set "TARGET_WT=worktrees%CVH_TASK_ID%"
)

if not "%TARGET_WT%"=="" if exist "%TARGET_WT%" (
    echo [CVH Ready Hook] Tearing down worktree: %TARGET_WT%
    git worktree remove "%TARGET_WT%" --force >nul 2>&1
    git worktree prune >nul 2>&1
)

if not "%CVH_BRANCH%"=="" (
    git branch -D "%CVH_BRANCH%" >nul 2>&1
)

exit /b 0
