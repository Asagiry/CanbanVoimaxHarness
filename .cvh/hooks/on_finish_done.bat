@echo off
REM ==============================================================================
REM CanbanVoimaxHarness Transition Hook: on_finish_done.bat
REM Triggered when a card enters the "Done / Production" stage
REM Environment variables provided by CVH:
REM   %CVH_TASK_ID%        - Unique task identifier (e.g. CVH-001)
REM   %CVH_TASK_TITLE%     - Human readable task title
REM   %CVH_WORKTREE_PATH%  - Path to isolated worktree directory
REM   %CVH_BRANCH%         - Feature branch name
REM   %CVH_VERDICT%        - Final reviewer verdict (e.g. APPROVED)
REM ==============================================================================

echo [CVH Hook] Finishing task %CVH_TASK_ID% with verdict: %CVH_VERDICT%
if exist "worktrees\%CVH_TASK_ID%" (
    echo [CVH Hook] Cleaning up temporary worktree: worktrees\%CVH_TASK_ID%
    git worktree remove "worktrees\%CVH_TASK_ID%" --force >nul 2>&1
    git branch -D feat/%CVH_TASK_ID% >nul 2>&1
)
echo [CVH Hook] Task %CVH_TASK_ID% successfully promoted to Ready.
exit /b 0
