#!/usr/bin/env bash
set -e

# ==============================================================================
# CanbanVoimaxHarness Hook: on_finish_done.sh
# Triggered when a task moves to done / ready for automated worktree & branch cleanup
# ==============================================================================

TASK_ID="${CVH_TASK_ID:-}"
BRANCH="${CVH_BRANCH:-}"
WORKTREE_PATH="${CVH_WORKTREE_PATH:-}"

echo "[CVH Done Hook] Cleaning up task: $TASK_ID | Branch: $BRANCH | Worktree: $WORKTREE_PATH"

TARGET_WT="$WORKTREE_PATH"
if [ -z "$TARGET_WT" ] && [ -n "$TASK_ID" ]; then
    if [ -d "worktrees/$TASK_ID" ]; then
        TARGET_WT="worktrees/$TASK_ID"
    fi
fi

if [ -n "$TARGET_WT" ] && [ -d "$TARGET_WT" ]; then
    echo "[CVH Done Hook] Tearing down worktree: $TARGET_WT"
    git worktree remove "$TARGET_WT" --force >/dev/null 2>&1 || true
    git worktree prune >/dev/null 2>&1 || true
    if [ -d "$TARGET_WT" ]; then
        rm -rf "$TARGET_WT" >/dev/null 2>&1 || true
    fi
fi

if [ -n "$BRANCH" ] && [ "$BRANCH" != "main" ] && [ "$BRANCH" != "master" ]; then
    git rev-parse --verify "$BRANCH" >/dev/null 2>&1 && git branch -D "$BRANCH" >/dev/null 2>&1 || true
fi

echo "[CVH Done Hook] Worktree cleanup completed for task: $TASK_ID"
exit 0
