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
        git checkout main
        git pull --rebase origin main || true
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
