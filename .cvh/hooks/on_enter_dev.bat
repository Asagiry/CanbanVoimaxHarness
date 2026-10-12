@echo off
REM ==============================================================================
REM CanbanVoimaxHarness Transition Hook: on_enter_dev.bat
REM Triggered when a card enters the "In Development" stage
REM Environment variables provided by CVH:
REM   %CVH_TASK_ID%        - Unique task identifier (e.g. CVH-001)
REM   %CVH_TASK_TITLE%     - Human readable task title
REM   %CVH_WORKTREE_PATH%  - Path to isolated worktree directory
REM   %CVH_BRANCH%         - Feature branch name
REM ==============================================================================

echo [CVH Hook] Entering Development stage for %CVH_TASK_ID%: %CVH_TASK_TITLE%
if not exist "worktrees\%CVH_TASK_ID%" (
    echo [CVH Hook] Scaffolding isolated git worktree: worktrees\%CVH_TASK_ID%
    git worktree add -b feat/%CVH_TASK_ID% worktrees/%CVH_TASK_ID% main >nul 2>&1
)
echo [CVH Hook] Worktree ready at worktrees\%CVH_TASK_ID%
exit /b 0
