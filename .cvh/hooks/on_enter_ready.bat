@echo off
setlocal enabledelayedexpansion
REM ==============================================================================
REM CanbanVoimaxHarness Hook: on_enter_ready.bat
REM Safety hook ensuring approved task branch is merged and pushed to origin/main
REM ==============================================================================

echo [CVH Ready Hook] Task: %CVH_TASK_ID% ^| Verdict: [%CVH_VERDICT%] ^| Branch: [%CVH_BRANCH%]

if "%CVH_VERDICT%" NEQ "APPROVED" if "%CVH_VERDICT%" NEQ "APPROVED_WITH_CAVEATS" (
    echo [CVH Ready Hook] Verdict is not APPROVED. Skipping merge.
    exit /b 0
)

if "%CVH_BRANCH%"=="" (
    exit /b 0
)

REM Verify feature branch exists
git rev-parse --verify "%CVH_BRANCH%" >nul 2>&1
if errorlevel 1 (
    exit /b 0
)

REM Check if already merged
git merge-base --is-ancestor "%CVH_BRANCH%" main >nul 2>&1
if not errorlevel 1 (
    echo [CVH Ready Hook] Branch is already merged into main.
    git push origin main >nul 2>&1
    exit /b 0
)

git checkout main >nul 2>&1
git merge --no-ff "%CVH_BRANCH%" -m "feat(%CVH_TASK_ID%): %CVH_TASK_TITLE% [approved]" >nul 2>&1
git push origin main >nul 2>&1
echo [CVH Ready Hook] Successfully verified and pushed main.
exit /b 0
