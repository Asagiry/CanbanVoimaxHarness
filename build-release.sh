#!/usr/bin/env bash
# ==========================================================================
# CanbanVoimaxHarness - one-button local Release build (Linux / bash).
# Single entry point for agents: run from the repository root, no questions.
#
#   ./build-release.sh [--skip-install] [--skip-checks] [--bundles=LIST] [--help]
#
# Pipeline: [1/5] preflight, [2/5] pnpm install, [3/5] frontend build,
#             [4/5] cargo check + clippy, [5/5] tauri build + collect.
# Layout (Linux):   Release/Linux/bin/        portable binary
#                   Release/Linux/installer/  AppImage, deb
# Layout (Git Bash): Release/Windows/bin/      portable exe
#                    Release/Windows/installer/ NSIS setup.exe, MSI
# All errors are printed with an ERROR: prefix and exit with a nonzero code.
# ==========================================================================
set -euo pipefail

SKIP_INSTALL=0
SKIP_CHECKS=0
BUNDLES=""
DO_PAUSE=0

usage() {
cat <<'EOF'
Usage: ./build-release.sh [options]

  One-button local Release build for CanbanVoimaxHarness (Linux / bash).
  Run from the repository root. Deterministic output layout:
    Release/Linux/bin/        portable binary (canban-voimax-harness)
    Release/Linux/installer/  AppImage, deb
    (on Git Bash: Release/Windows/bin + installer, same as build-release.bat)

  Options:
    --skip-install    reuse existing node_modules (default: skip if present)
    --skip-checks     skip cargo check + cargo clippy
    --bundles=LIST    forward to "tauri build --bundles LIST" (e.g. appimage,deb)
    --pause           accepted for parity, no-op on Linux (never blocks)
    -h, --help        print this help and exit 0

  Examples:
    ./build-release.sh
    ./build-release.sh --bundles=appimage,deb --skip-install
EOF
}

fail() {
  echo "ERROR: $1" >&2
  exit "${2:-1}"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-install) SKIP_INSTALL=1; shift ;;
    --skip-checks) SKIP_CHECKS=1; shift ;;
    --pause) DO_PAUSE=1; echo "NOTE: --pause is a no-op on Linux (never blocks)."; shift ;;
    --bundles=?*) BUNDLES="${1#--bundles=}"; shift ;;
    --bundles)
      BUNDLES="${2:-}"
      if [[ -z "$BUNDLES" ]]; then fail "--bundles requires a value." 2; fi
      shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) fail "unknown flag: $1. Run ./build-release.sh --help for usage." 2 ;;
  esac
done

case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*) EXE=".exe"; BIN_DIR="Release/Windows/bin"; INST_DIR="Release/Windows/installer" ;;
  *)                    EXE="";     BIN_DIR="Release/Linux/bin";     INST_DIR="Release/Linux/installer" ;;
esac
APP_BIN="src-tauri/target/release/canban-voimax-harness${EXE}"

echo "CanbanVoimaxHarness local Release build"
echo "  root: $PWD"
if [[ -z "$BUNDLES" ]]; then echo "  bundles: (tauri.conf.json defaults)"; else echo "  bundles: $BUNDLES"; fi
echo ""

echo "[1/5] Preflight: checking pnpm, node, cargo..."
command -v pnpm >/dev/null 2>&1 || fail "pnpm not found in PATH. Install Node.js 20 + pnpm."
command -v node >/dev/null 2>&1 || fail "node not found in PATH. Install Node.js 20."
command -v cargo >/dev/null 2>&1 || fail "cargo not found in PATH. Install the Rust stable toolchain."
echo "  ok: pnpm, node, cargo found."
echo ""

echo "[2/5] Installing frontend dependencies..."
if [[ "$SKIP_INSTALL" == "1" ]]; then
  echo "  skipped (--skip-install)."
elif [[ -d "node_modules" ]]; then
  echo "  skipped (node_modules exists)."
else
  pnpm install --frozen-lockfile || fail "pnpm install --frozen-lockfile failed."
fi
echo ""

echo "[3/5] Building frontend (pnpm run build)..."
pnpm run build || fail "pnpm run build failed (tsc && vite build)."
echo ""

echo "[4/5] Rust sensors (cargo check + clippy)..."
if [[ "$SKIP_CHECKS" == "1" ]]; then
  echo "  skipped (--skip-checks)."
else
  [[ -d "src-tauri" ]] || fail "src-tauri directory not found. Run from the repository root."
  pushd "src-tauri" >/dev/null || fail "cannot enter src-tauri directory."
  cargo check || { popd >/dev/null; fail "cargo check failed."; }
  cargo clippy -- -D warnings || { popd >/dev/null; fail "cargo clippy -- -D warnings failed."; }
  popd >/dev/null || fail "cannot leave src-tauri directory."
fi
echo ""

echo "[5/5] Building Tauri release bundles..."
if [[ -z "$BUNDLES" ]]; then
  pnpm tauri build || fail "pnpm tauri build failed."
else
  pnpm tauri build --bundles "$BUNDLES" || fail "pnpm tauri build failed."
fi
echo ""

echo "Collecting artifacts..."
mkdir -p "$BIN_DIR" "$INST_DIR"
[[ -f "$APP_BIN" ]] || fail "portable binary not found: $APP_BIN"
cp -f "$APP_BIN" "$BIN_DIR/" || fail "failed to copy $APP_BIN to $BIN_DIR"
shopt -s nullglob
installer_files=()
if [[ -n "$EXE" ]]; then
  installer_files+=(src-tauri/target/release/bundle/nsis/*.exe src-tauri/target/release/bundle/msi/*.msi)
else
  installer_files+=(src-tauri/target/release/bundle/appimage/*.AppImage src-tauri/target/release/bundle/deb/*.deb)
fi
for f in "${installer_files[@]}"; do cp -f "$f" "$INST_DIR/"; done
if [[ "${#installer_files[@]}" -eq 0 ]]; then echo "  NOTE: no installers found; portable binary only."; fi
echo ""

print_size() {
  local f="$1" s
  s=$(stat -c%s "$f" 2>/dev/null || stat -f%z "$f" 2>/dev/null || wc -c <"$f")
  s="$(echo "$s" | tr -d '[:space:]')"
  echo "  $f  [${s} bytes]"
}

echo "Release artifacts:"
print_size "$BIN_DIR/$(basename "$APP_BIN")"
for f in "$INST_DIR"/*; do [[ -e "$f" ]] || continue; print_size "$f"; done
echo ""
echo "DONE: local Release build complete."
exit 0
