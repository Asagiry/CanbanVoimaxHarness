#!/usr/bin/env bash
set -e
export PATH="$HOME/.cargo/bin:$PATH"

echo "[CVH Sensor] Checking git mergeability with main..."
if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    if git rev-parse --verify main >/dev/null 2>&1; then
        if ! git merge-tree --write-tree main HEAD >/dev/null 2>&1; then
            echo "[CVH Sensor] ERROR: Merge conflict detected against 'main'! Merge or rebase latest main before handoff." >&2
            exit 1
        fi
    fi
fi

echo "[CVH Sensor] Running Rust backend check..."
if [ -f "src-tauri/Cargo.toml" ]; then
    cargo check --manifest-path "src-tauri/Cargo.toml"
fi

echo "[CVH Sensor] Running TypeScript frontend typecheck..."
if [ -f "package.json" ]; then
    npx tsc --noEmit
fi

echo "[CVH Sensor] All gatekeeper sensors PASSED successfully."
exit 0
