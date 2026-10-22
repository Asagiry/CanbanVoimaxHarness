@echo off
setlocal enabledelayedexpansion
REM ==============================================================================
REM CanbanVoimaxHarness Hook: on_review_finish.bat
REM Merges approved task feature branch into main and pushes to origin/main
REM ==============================================================================

echo [CVH Merge Hook] Task: %CVH_TASK_ID% ^| Stage: %CVH_STAGE_ID% ^| Verdict: [%CVH_VERDICT%] ^| Branch: [%CVH_BRANCH%]

if "%CVH_VERDICT%" NEQ "APPROVED" if "%CVH_VERDICT%" NEQ "APPROVED_WITH_CAVEATS" (
    echo [CVH Merge Hook] Verdict is not APPROVED. Skipping merge and push.
    exit /b 0
)

if "%CVH_BRANCH%"=="" (
    echo [CVH Merge Hook] Notice: CVH_BRANCH is not set. Skipping git merge.
    exit /b 0
)

REM Verify feature branch exists
git rev-parse --verify "%CVH_BRANCH%" >nul 2>&1
if errorlevel 1 (
    echo [CVH Merge Hook] Notice: Branch is not found or was already merged. Skipping.
    exit /b 0
)

REM Check if branch is already merged into main
git merge-base --is-ancestor "%CVH_BRANCH%" main >nul 2>&1
if not errorlevel 1 (
    echo [CVH Merge Hook] Branch is already merged into main.
    echo [CVH Merge Hook] Pushing main to origin/main...
    git push origin main
    exit /b 0
)

echo [CVH Merge Hook] Checking out main...
git checkout main
if errorlevel 1 (
    echo [CVH Merge Hook] ERROR: Failed to checkout main >&2
    exit /b 1
)

echo [CVH Merge Hook] Pulling latest main from origin...
git pull --rebase origin main >nul 2>&1

echo [CVH Merge Hook] Merging branch '%CVH_BRANCH%' into main...
git merge --no-ff "%CVH_BRANCH%" -m "feat(%CVH_TASK_ID%): %CVH_TASK_TITLE% [approved]"
if errorlevel 1 (
    echo [CVH Merge Hook] ERROR: Git merge conflict while merging into main >&2
    exit /b 1
)

echo [CVH Merge Hook] Pushing to origin/main...
git push origin main
if errorlevel 1 (
    echo [CVH Merge Hook] ERROR: Failed to push main to origin/main >&2
    exit /b 1
)

echo [CVH Merge Hook] SUCCESS: Cleanly merged into main and pushed to origin/main!
exit /b 0
