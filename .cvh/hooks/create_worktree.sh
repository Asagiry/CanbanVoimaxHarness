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
    TARGET_DIR="$(cd "$SCRIPT_DIR/../../src-tauri/target" 2>/dev/null && pwd || echo "$SCRIPT_DIR/../../src-tauri/target")"
    if command -v cygpath >/dev/null 2>&1; then
        export CARGO_TARGET_DIR="$(cygpath -w "$TARGET_DIR" 2>/dev/null || echo "$TARGET_DIR")"
    else
        export CARGO_TARGET_DIR="$TARGET_DIR"
    fi
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
        if ! git worktree add "$WT_DIR" "$BRANCH"; then
            echo "[CVH Worktree Hook] ERROR: Failed to add existing branch '$BRANCH' to worktree" >&2
            exit 1
        fi
    else
        echo "[CVH Worktree Hook] Creating new branch '${BRANCH}' and worktree '${WT_DIR}'..."
        if ! git worktree add -b "$BRANCH" "$WT_DIR" HEAD; then
            echo "[CVH Worktree Hook] ERROR: Failed to create worktree '$WT_DIR' with branch '$BRANCH'" >&2
            exit 1
        fi
    fi
fi

# Sync live .cvh/ protected paths from root (canonical source) into worktree mirror
# Exclude target/, .log, .db, node_modules from worktree synchronization
mkdir -p "$WT_DIR/.cvh"

sync_cvh_dir() {
    sub="$1"
    if [ -d ".cvh/$sub" ]; then
        mkdir -p "$WT_DIR/.cvh/$sub"
        if command -v tar >/dev/null 2>&1; then
            (cd ".cvh/$sub" && tar \
                --exclude="*.db*" \
                --exclude="*.sqlite*" \
                --exclude="*.log*" \
                --exclude="target" \
                --exclude="node_modules" \
                --exclude="logs" \
                --exclude="dialogs" \
                --exclude="spill" \
                --exclude="context" \
                -cf - .) | (cd "$WT_DIR/.cvh/$sub" && tar -xf -) 2>/dev/null || cp -a ".cvh/$sub/." "$WT_DIR/.cvh/$sub/"
        else
            cp -a ".cvh/$sub/." "$WT_DIR/.cvh/$sub/"
        fi
    fi
}

sync_cvh_dir "agents"
sync_cvh_dir "rules"
sync_cvh_dir "hooks"
sync_cvh_dir "settings"

if [ -f ".cvh/boards.json" ]; then
    cp -a ".cvh/boards.json" "$WT_DIR/.cvh/boards.json"
fi

# Ensure dist directory exists for tauri generate_context!() proc macro
mkdir -p "$WT_DIR/dist"

# Link root node_modules if present to avoid cold npm installs
if [ -d "node_modules" ] && [ ! -e "$WT_DIR/node_modules" ]; then
    if command -v cygpath >/dev/null 2>&1 && command -v cmd.exe >/dev/null 2>&1; then
        WIN_WT="$(cygpath -w "$WT_DIR/node_modules" 2>/dev/null || true)"
        WIN_NM="$(cygpath -w "$(pwd)/node_modules" 2>/dev/null || true)"
        if [ -n "$WIN_WT" ] && [ -n "$WIN_NM" ]; then
            cmd.exe //c "mklink /J \"$WIN_WT\" \"$WIN_NM\"" >/dev/null 2>&1 || ln -s "$(pwd)/node_modules" "$WT_DIR/node_modules" 2>/dev/null || true
        else
            ln -s "$(pwd)/node_modules" "$WT_DIR/node_modules" 2>/dev/null || true
        fi
    else
        ln -s "$(pwd)/node_modules" "$WT_DIR/node_modules" 2>/dev/null || true
    fi
fi

# Standard CVH output protocol
echo "CVH_WORKTREE=$WT_DIR"
echo "CVH_BRANCH=$BRANCH"
echo "[CVH Worktree Hook] Worktree successfully initialized."
exit 0
