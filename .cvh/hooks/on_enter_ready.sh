#!/usr/bin/env bash
set -e
# CanbanVoimaxHarness Hook: on_enter_ready.sh
# Safety hook ensuring approved task branch is merged, pushed, and worktree is torn down

TASK_ID="${CVH_TASK_ID:-unknown}"
TITLE="${CVH_TASK_TITLE:-task}"
BRANCH="${CVH_BRANCH:-}"
VERDICT="${CVH_VERDICT:-}"

echo "[CVH Ready Hook] Task: $TASK_ID | Verdict: $VERDICT | Branch: $BRANCH"

if [ "$VERDICT" != "APPROVED" ] && [ "$VERDICT" != "APPROVED_WITH_CAVEATS" ]; then
    echo "[CVH Ready Hook] Verdict is not APPROVED ($VERDICT). Skipping merge."
    exit 0
fi

if [ -n "$BRANCH" ] && git rev-parse --verify "$BRANCH" >/dev/null 2>&1; then
    if git merge-base --is-ancestor "$BRANCH" main >/dev/null 2>&1; then
        echo "[CVH Ready Hook] Branch $BRANCH already merged into main. Pushing..."
        git push origin main || true
    else
        git checkout main
        git pull --rebase origin main || true
        git merge --no-ff "$BRANCH" -m "feat($TASK_ID): $TITLE [approved]"
        git push origin main
        echo "[CVH Ready Hook] Successfully merged $BRANCH and pushed main."
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
    echo "[CVH Ready Hook] Tearing down worktree: $TARGET_WT"
    git worktree remove "$TARGET_WT" --force >/dev/null 2>&1 || true
    git worktree prune >/dev/null 2>&1 || true
fi

if [ -n "$BRANCH" ]; then
    git branch -D "$BRANCH" >/dev/null 2>&1 || true
fi

exit 0
