#!/usr/bin/env bash
set -e
export PATH="$HOME/.cargo/bin:$PATH"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [ -z "$CARGO_TARGET_DIR" ]; then
    TARGET_DIR="$(cd "$SCRIPT_DIR/../../src-tauri/target" 2>/dev/null && pwd || echo "$SCRIPT_DIR/../../src-tauri/target")"
    if command -v cygpath >/dev/null 2>&1; then
        export CARGO_TARGET_DIR="$(cygpath -w "$TARGET_DIR" 2>/dev/null || echo "$TARGET_DIR")"
    else
        export CARGO_TARGET_DIR="$TARGET_DIR"
    fi
fi

if [ -n "$CVH_WORKTREE_PATH" ] && [ -d "$CVH_WORKTREE_PATH" ]; then
    cd "$CVH_WORKTREE_PATH"
fi

echo "[CVH Sensor] Checking git mergeability with main..."
if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    # Gate 6: Working tree must be clean and committed before handover (excluding gen artifacts, db, logs, node_modules, target)
    DIRTY_FILES="$(git status --porcelain -- ':!src-tauri/gen' ':!.cvh' ':!target' ':!**/target' ':!node_modules' ':!**/node_modules' ':!*.log' ':!*.db*' ':!*.sqlite*' 2>/dev/null || true)"
    if [ -n "$DIRTY_FILES" ]; then
        echo "[CVH Sensor] ERROR: Uncommitted changes detected in working tree! Commit all files before handoff (Gate 6)." >&2
        git status --short -- ':!src-tauri/gen' ':!.cvh' ':!target' ':!**/target' ':!node_modules' ':!**/node_modules' ':!*.log' ':!*.db*' ':!*.sqlite*' >&2
        exit 1
    fi
    if git rev-parse --verify main >/dev/null 2>&1; then
        if ! git merge-tree --write-tree main HEAD >/dev/null 2>&1; then
            echo "[CVH Sensor] ERROR: Merge conflict detected against 'main'! Merge or rebase latest main before handoff." >&2
            git merge --abort >/dev/null 2>&1 || true
            exit 1
        fi
    fi
fi

DIFF_AVAILABLE=0
HAS_BACKEND_CHANGES=0
HAS_FRONTEND_CHANGES=0

if git rev-parse --verify main >/dev/null 2>&1 && git rev-parse --verify HEAD >/dev/null 2>&1; then
    DIFF_AVAILABLE=1
    DIFF_FILES="$(git diff --name-only main HEAD 2>/dev/null || true)"
    if echo "$DIFF_FILES" | grep -qiE '^(src-tauri/|Cargo\.toml|Cargo\.lock)'; then
        HAS_BACKEND_CHANGES=1
    fi
    if echo "$DIFF_FILES" | grep -qiE '^(src/|index\.html|vite\.config\.|tsconfig|package\.json|pnpm-lock\.yaml)'; then
        HAS_FRONTEND_CHANGES=1
    fi
fi

RUN_BACKEND=1
RUN_FRONTEND=1
if [ "$DIFF_AVAILABLE" -eq 1 ]; then
    if [ "$HAS_BACKEND_CHANGES" -eq 0 ]; then
        RUN_BACKEND=0
    fi
    if [ "$HAS_FRONTEND_CHANGES" -eq 0 ]; then
        RUN_FRONTEND=0
    fi
fi

if [ "$RUN_BACKEND" -eq 0 ]; then
    echo "[CVH Sensor] Backend unchanged, skipping backend check."
elif [ -f "src-tauri/Cargo.toml" ]; then
    echo "[CVH Sensor] Running Rust backend check [mbx/cargo]..."
    if [ ! -d "dist" ]; then
        mkdir -p "dist"
    fi
    if command -v mbx >/dev/null 2>&1; then
        mbx check --manifest-path "src-tauri/Cargo.toml"
    else
        cargo check --manifest-path "src-tauri/Cargo.toml"
    fi
fi

if [ "$RUN_FRONTEND" -eq 0 ]; then
    echo "[CVH Sensor] Frontend unchanged, skipping frontend checks."
elif [ -f "package.json" ]; then
    echo "[CVH Sensor] Running frontend tests (Vitest)..."
    if command -v pnpm >/dev/null 2>&1; then
        pnpm test
        echo "[CVH Sensor] Running TypeScript frontend typecheck..."
        pnpm exec tsc --noEmit
        echo "[CVH Sensor] Running Oxlint linter..."
        pnpm exec oxlint --deny-warnings
    else
        npm test
        echo "[CVH Sensor] Running TypeScript frontend typecheck..."
        npx tsc --noEmit
        echo "[CVH Sensor] Running Oxlint linter..."
        npx oxlint --deny-warnings
    fi
fi

echo "[CVH Sensor] All gatekeeper sensors PASSED successfully."
exit 0
