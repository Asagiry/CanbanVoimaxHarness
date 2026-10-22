#!/usr/bin/env bash
set -e
export PATH="$HOME/.cargo/bin:$PATH"

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
