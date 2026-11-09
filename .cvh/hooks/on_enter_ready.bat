@echo off
setlocal enabledelayedexpansion
REM ==============================================================================
REM CanbanVoimaxHarness Hook: on_enter_ready.bat
REM Triggered on ready:on_enter - merges feature branch into main, pushes to origin/main, removes worktree
REM Terminal Exit Ban compliant: uses on_enter only, never on_leave
REM ==============================================================================

echo [CVH Ready Hook] Task: %CVH_TASK_ID% ^| Stage: %CVH_STAGE_ID% ^| Branch: [%CVH_BRANCH%] ^| Worktree: [%CVH_WORKTREE_PATH%]

if "%CVH_BRANCH%"=="" (
    echo [CVH Ready Hook] CVH_BRANCH not set. Pushing main...
    git push origin main
    goto TEARDOWN
)

REM Verify feature branch exists
git rev-parse --verify "%CVH_BRANCH%" >nul 2>&1
if errorlevel 1 (
    echo [CVH Ready Hook] Branch not found, already merged or cleaned. Pushing main...
    git push origin main
    goto TEARDOWN
)

REM Check if branch is already merged into main
git merge-base --is-ancestor "%CVH_BRANCH%" main >nul 2>&1
if not errorlevel 1 (
    echo [CVH Ready Hook] Branch already merged into main. Pushing main...
    git push origin main
    goto TEARDOWN
)

echo [CVH Ready Hook] Checking out main...
REM Preserve live .cvh/ config files across checkout/pull/merge (exclude databases and runtime logs/spill)
if exist ".cvh" (
    robocopy ".cvh" "%TEMP%\cvh_backup_%CVH_TASK_ID%" /E /XF *.db* *.sqlite* *.log* /XD dialogs context spill logs worktrees >nul 2>&1
)
REM Discard uncommitted tracked .cvh/ changes so checkout/merge cannot abort on local changes (never git clean DBs!)
git restore -- .cvh >nul 2>&1

git checkout main
if errorlevel 1 (
    echo [CVH Ready Hook] ERROR: Failed to checkout main >&2
    exit /b 1
)

echo [CVH Ready Hook] Pulling latest main from origin...
git fetch origin
git pull --rebase origin main >nul 2>&1

echo [CVH Ready Hook] Merging branch '%CVH_BRANCH%' into main...
git merge --no-ff "%CVH_BRANCH%" -m "feat(%CVH_TASK_ID%): %CVH_TASK_TITLE% [ready]"
if errorlevel 1 (
    echo [CVH Ready Hook] ERROR: Git merge conflict while merging into main. Aborting merge... >&2
    git merge --abort >nul 2>&1
    exit /b 1
)

REM Unstage any .cvh/ introduced by merge, then restore live backup over .cvh/ (excluding databases)
git restore --staged .cvh >nul 2>&1
if exist "%TEMP%\cvh_backup_%CVH_TASK_ID%" (
    robocopy "%TEMP%\cvh_backup_%CVH_TASK_ID%" ".cvh" /E /XF *.db* *.sqlite* *.log* /XD dialogs context spill logs worktrees >nul 2>&1
    rmdir /S /Q "%TEMP%\cvh_backup_%CVH_TASK_ID%" >nul 2>&1
)

echo [CVH Ready Hook] Pushing to origin/main...
git push origin main
if errorlevel 1 (
    echo [CVH Ready Hook] WARNING: Failed to push main to origin/main -- offline or rejected. Local merge preserved. >&2
)

echo [CVH Ready Hook] SUCCESS: Merged into main and pushed to origin/main!

:TEARDOWN
REM Worktree Teardown Protocol (§ 2.23)
set "TARGET_WT=%CVH_WORKTREE_PATH%"
if "%TARGET_WT%"=="" if not "%CVH_TASK_ID%"=="" (
    if exist "worktrees\%CVH_TASK_ID%" set "TARGET_WT=worktrees\%CVH_TASK_ID%"
)

if not "%TARGET_WT%"=="" if exist "%TARGET_WT%" (
    echo [CVH Ready Hook] Tearing down worktree: %TARGET_WT%
    git worktree remove "%TARGET_WT%" --force >nul 2>&1
    git worktree prune >nul 2>&1
    if exist "%TARGET_WT%" rmdir /s /q "%TARGET_WT%" >nul 2>&1
)

if not "%CVH_BRANCH%"=="" (
    git branch -D "%CVH_BRANCH%" >nul 2>&1 || (
        echo [CVH Ready Hook] Branch already deleted or merged, ignoring.
    )
)

exit /b 0
