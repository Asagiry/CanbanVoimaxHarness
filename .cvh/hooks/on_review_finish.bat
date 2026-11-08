@echo off
setlocal enabledelayedexpansion
REM ==============================================================================
REM CanbanVoimaxHarness Hook: on_review_finish.bat
REM Merges approved task feature branch into main and pushes to origin/main
REM Automates clean worktree teardown (§ 2.23)
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
    goto TEARDOWN
)

REM Check if branch is already merged into main
git merge-base --is-ancestor "%CVH_BRANCH%" main >nul 2>&1
if not errorlevel 1 (
    echo [CVH Merge Hook] Branch is already merged into main.
    echo [CVH Merge Hook] Pushing main to origin/main...
    git push origin main
    goto TEARDOWN
)

echo [CVH Merge Hook] Checking out main...
REM Preserve live .cvh/ (canonical source, uncommitted) across checkout/pull/merge
if exist ".cvh" (xcopy /E /I /Y ".cvh" "%TEMP%\cvh_backup_%CVH_TASK_ID%" >nul)
REM Discard uncommitted .cvh/ residue so checkout/merge cannot abort on local changes
git restore -- .cvh >nul 2>&1
git clean -fd -- .cvh >nul 2>&1

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
    echo [CVH Merge Hook] ERROR: Git merge conflict while merging into main. Aborting merge... >&2
    git merge --abort >nul 2>&1
    exit /b 1
)

REM Unstage any .cvh/ introduced by merge, then restore live backup over .cvh/
git restore --staged .cvh >nul 2>&1
if exist "%TEMP%\cvh_backup_%CVH_TASK_ID%" (xcopy /E /I /Y "%TEMP%\cvh_backup_%CVH_TASK_ID%" ".cvh" >nul & rmdir /S /Q "%TEMP%\cvh_backup_%CVH_TASK_ID%")

echo [CVH Merge Hook] Pushing to origin/main...
git push origin main
if errorlevel 1 (
    echo [CVH Merge Hook] WARNING: Failed to push main to origin/main (offline or rejected). Local merge preserved. >&2
)

echo [CVH Merge Hook] SUCCESS: Cleanly merged into main and pushed to origin/main!

:TEARDOWN
REM Worktree Teardown Protocol (§ 2.23)
set "TARGET_WT=%CVH_WORKTREE_PATH%"
if "%TARGET_WT%"=="" if not "%CVH_TASK_ID%"=="" (
    if exist "worktrees\%CVH_TASK_ID%" set "TARGET_WT=worktrees\%CVH_TASK_ID%"
)

if not "%TARGET_WT%"=="" if exist "%TARGET_WT%" (
    echo [CVH Merge Hook] Tearing down worktree: %TARGET_WT%
    git worktree remove "%TARGET_WT%" --force >nul 2>&1
    git worktree prune >nul 2>&1
    if exist "%TARGET_WT%" rmdir /s /q "%TARGET_WT%" >nul 2>&1
)

if not "%CVH_BRANCH%"=="" (
    git branch -D "%CVH_BRANCH%" >nul 2>&1
)

exit /b 0
