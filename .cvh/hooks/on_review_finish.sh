#!/usr/bin/env bash
set -e
# CanbanVoimaxHarness Hook: on_review_finish.sh
TASK_ID="${CVH_TASK_ID:-unknown}"
TITLE="${CVH_TASK_TITLE:-task}"
BRANCH="${CVH_BRANCH:-}"
VERDICT="${CVH_VERDICT:-}"

echo "[CVH Merge Hook] Task: $TASK_ID | Verdict: $VERDICT | Branch: $BRANCH"

if [ "$VERDICT" != "APPROVED" ] && [ "$VERDICT" != "APPROVED_WITH_CAVEATS" ]; then
    echo "[CVH Merge Hook] Verdict is not APPROVED ($VERDICT). Skipping merge."
    exit 0
fi

if [ -z "$BRANCH" ]; then
    echo "[CVH Merge Hook] CVH_BRANCH not set. Skipping."
    exit 0
fi

if ! git rev-parse --verify "$BRANCH" >/dev/null 2>&1; then
    echo "[CVH Merge Hook] Branch $BRANCH not found. Skipping."
else
    if git merge-base --is-ancestor "$BRANCH" main >/dev/null 2>&1; then
        echo "[CVH Merge Hook] Branch $BRANCH already merged into main. Pushing..."
        git push origin main || true
    else
        echo "[CVH Merge Hook] Checking out main..."
        # Discard uncommitted dual-write residue before checkout/pull/merge (§ 2.23)
        git restore --staged .cvh >/dev/null 2>&1 || true
        git restore .cvh >/dev/null 2>&1 || true

        git checkout main
        if [ $? -ne 0 ]; then
            echo "[CVH Merge Hook] ERROR: Failed to checkout main" >&2
            exit 1
        fi

        # Discard uncommitted dual-write residue on main before merge
        git restore --staged .cvh >/dev/null 2>&1 || true
        git restore .cvh >/dev/null 2>&1 || true

        echo "[CVH Merge Hook] Pulling latest main from origin..."
        git pull --rebase origin main || true

        # Ensure clean state before merge
        git restore --staged .cvh >/dev/null 2>&1 || true
        git restore .cvh >/dev/null 2>&1 || true

        echo "[CVH Merge Hook] Merging branch '$BRANCH' into main..."
        git merge --no-ff "$BRANCH" -m "feat($TASK_ID): $TITLE [approved]"
        git push origin main
        echo "[CVH Merge Hook] SUCCESS: $BRANCH merged into main and pushed to origin/main!"
    fi
fi

# Worktree Teardown Protocol (§ 2.23)
TARGET_WT="${CVH_WORKTREE_PATH:-}"
if [ -z "$TARGET_WT" ] && [ -n "$TASK_ID" ]; then
    if [ -d "worktrees/$TASK_ID" ]; then
        TARGET_WT="worktrees/$TASK_ID"
    fi
fi

if [ -n "$TARGET_WT" ] && [ -d "$TARGET_WT" ]; then
    echo "[CVH Merge Hook] Tearing down worktree: $TARGET_WT"
    git worktree remove "$TARGET_WT" --force >/dev/null 2>&1 || true
    git worktree prune >/dev/null 2>&1 || true
fi

if [ -n "$BRANCH" ]; then
    git branch -D "$BRANCH" >/dev/null 2>&1 || true
fi

exit 0
