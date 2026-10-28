#!/usr/bin/env bash
set -e

# ==============================================================================
# CanbanVoimaxHarness Hook: create_worktree.sh
# Triggered on backlog on_leave to initialize an isolated git worktree for the task
# ==============================================================================

if [ -z "$CVH_TASK_ID" ]; then
    echo "[CVH Worktree Hook] ERROR: CVH_TASK_ID environment variable is missing" >&2
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [ -z "$CARGO_TARGET_DIR" ]; then
    export CARGO_TARGET_DIR="$(cd "$SCRIPT_DIR/../../src-tauri/target" 2>/dev/null && pwd || echo "$SCRIPT_DIR/../../src-tauri/target")"
fi

if [ "$CVH_SKIP_WORKTREE" = "true" ]; then
    echo "[CVH Worktree Hook] CVH_SKIP_WORKTREE is true. Skipping worktree creation."
    exit 0
fi

SLUG="$CVH_TASK_ID"
WT_DIR="worktrees/${SLUG}"
BRANCH="feat/${SLUG}"

echo "[CVH Worktree Hook] Task: ${CVH_TASK_ID} | Worktree: ${WT_DIR} | Branch: ${BRANCH}"

mkdir -p "worktrees"

if [ -d "$WT_DIR" ]; then
    echo "[CVH Worktree Hook] Worktree directory '${WT_DIR}' already exists."
else
    if git rev-parse --verify "$BRANCH" >/dev/null 2>&1; then
        echo "[CVH Worktree Hook] Branch '${BRANCH}' already exists. Attaching worktree..."
        git worktree add "$WT_DIR" "$BRANCH"
    else
        echo "[CVH Worktree Hook] Creating new branch '${BRANCH}' and worktree '${WT_DIR}'..."
        git worktree add -b "$BRANCH" "$WT_DIR" HEAD
    fi
fi

# Standard CVH output protocol
echo "CVH_WORKTREE=$WT_DIR"
echo "CVH_BRANCH=$BRANCH"
echo "[CVH Worktree Hook] Worktree successfully initialized."
exit 0
