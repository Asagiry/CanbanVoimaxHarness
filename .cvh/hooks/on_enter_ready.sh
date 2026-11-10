#!/usr/bin/env bash
set -e
# ==============================================================================
# CanbanVoimaxHarness Hook: on_enter_ready.sh
# Triggered on ready:on_enter - merges feature branch into main, pushes to origin/main, removes worktree
# Terminal Exit Ban compliant: uses on_enter only, never on_leave
# ==============================================================================

TASK_ID="${CVH_TASK_ID:-unknown}"
TITLE="${CVH_TASK_TITLE:-task}"
BRANCH="${CVH_BRANCH:-}"
WORKTREE_PATH="${CVH_WORKTREE_PATH:-}"

echo "[CVH Ready Hook] Task: $TASK_ID | Branch: $BRANCH | Worktree: $WORKTREE_PATH"

backup_cvh() {
    src="$1"
    dst="$2"
    mkdir -p "$dst"
    if [ -d "$src" ]; then
        if command -v tar >/dev/null 2>&1; then
            (cd "$src" && tar \
                --exclude="*.db*" \
                --exclude="*.sqlite*" \
                --exclude="*.log*" \
                --exclude="target" \
                --exclude="node_modules" \
                --exclude="dialogs" \
                --exclude="logs" \
                --exclude="spill" \
                --exclude="context" \
                --exclude="worktrees" \
                -cf - .) | (cd "$dst" && tar -xf -) 2>/dev/null || cp -a "$src/." "$dst/"
        else
            cp -a "$src/." "$dst/"
        fi
    fi
}

restore_cvh() {
    src="$1"
    dst="$2"
    if [ -d "$src" ]; then
        mkdir -p "$dst"
        if command -v tar >/dev/null 2>&1; then
            (cd "$src" && tar \
                --exclude="*.db*" \
                --exclude="*.sqlite*" \
                --exclude="*.log*" \
                --exclude="target" \
                --exclude="node_modules" \
                --exclude="dialogs" \
                --exclude="logs" \
                --exclude="spill" \
                --exclude="context" \
                --exclude="worktrees" \
                -cf - .) | (cd "$dst" && tar -xf -) 2>/dev/null || cp -a "$src/." "$dst/"
        else
            cp -a "$src/." "$dst/"
        fi
    fi
}

if [ -z "$BRANCH" ]; then
    echo "[CVH Ready Hook] CVH_BRANCH not set. Pushing main..."
    git push origin main || true
else
    if ! git rev-parse --verify "$BRANCH" >/dev/null 2>&1; then
        echo "[CVH Ready Hook] Branch $BRANCH not found. Pushing main..."
        git push origin main || true
    elif git merge-base --is-ancestor "$BRANCH" main >/dev/null 2>&1; then
        echo "[CVH Ready Hook] Branch $BRANCH already merged into main. Pushing main..."
        git push origin main || true
    else
        echo "[CVH Ready Hook] Checking out main..."
        # Preserve live .cvh/ (canonical source, uncommitted) across checkout/pull/merge
        BACKUP="$(mktemp -d 2>/dev/null || echo "/tmp/cvh_backup_$TASK_ID")"
        backup_cvh ".cvh" "$BACKUP"

        # Discard uncommitted .cvh/ residue so checkout/merge cannot abort on local changes
        git restore -- .cvh >/dev/null 2>&1 || true
        git clean -fd -- .cvh >/dev/null 2>&1 || true

        if ! git checkout main; then
            echo "[CVH Ready Hook] ERROR: Failed to checkout main" >&2
            rm -rf "$BACKUP"
            exit 1
        fi

        echo "[CVH Ready Hook] Pulling latest main from origin..."
        git fetch origin || true
        git pull --rebase origin main || true

        echo "[CVH Ready Hook] Merging branch '$BRANCH' into main..."
        if ! git merge --no-ff "$BRANCH" -m "feat($TASK_ID): $TITLE [ready]"; then
            echo "[CVH Ready Hook] ERROR: Git merge conflict while merging into main. Aborting merge..." >&2
            git merge --abort >/dev/null 2>&1 || true
            restore_cvh "$BACKUP" ".cvh"
            rm -rf "$BACKUP"
            exit 1
        fi

        # Unstage any .cvh/ introduced by merge, then restore live backup over .cvh/
        git restore --staged .cvh >/dev/null 2>&1 || true
        restore_cvh "$BACKUP" ".cvh"
        rm -rf "$BACKUP"

        echo "[CVH Ready Hook] Pushing to origin/main..."
        if ! git push origin main; then
            echo "[CVH Ready Hook] WARNING: Failed to push main to origin/main -- offline or rejected. Local merge preserved." >&2
        fi
        echo "[CVH Ready Hook] SUCCESS: Merged into main and pushed!"
    fi
fi

# Worktree Teardown Protocol (§ 2.23)
TARGET_WT="$WORKTREE_PATH"
if [ -z "$TARGET_WT" ] && [ -n "$TASK_ID" ]; then
    if [ -d "worktrees/$TASK_ID" ]; then
        TARGET_WT="worktrees/$TASK_ID"
    fi
fi

if [ -n "$TARGET_WT" ] && [ -d "$TARGET_WT" ]; then
    echo "[CVH Ready Hook] Tearing down worktree: $TARGET_WT"
    git worktree remove "$TARGET_WT" --force >/dev/null 2>&1 || true
    git worktree prune >/dev/null 2>&1 || true
    if [ -d "$TARGET_WT" ]; then
        rm -rf "$TARGET_WT" >/dev/null 2>&1 || true
    fi
fi

if [ -n "$BRANCH" ]; then
    git rev-parse --verify "$BRANCH" >/dev/null 2>&1 && git branch -D "$BRANCH" >/dev/null 2>&1 || true
fi

exit 0
