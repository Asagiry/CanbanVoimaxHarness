# ============================================================================
# CanbanVoimaxHarness - one-button local Release build (PowerShell 5.1+).
# Single entry point for agents: run from the repository root, no questions.
#
#   ./build-release.ps1 [-SkipInstall] [-SkipChecks] [-Bundles nsis,msi] [-Pause] [-Help]
#   Double-dash spellings (--skip-install, --bundles=nsis, --help) work too.
#
# Pipeline: [1/5] preflight, [2/5] pnpm install, [3/5] frontend build,
#             [4/5] cargo check + clippy, [5/5] tauri build + collect.
# Layout (Windows): Release/Windows/bin/        portable exe
#                   Release/Windows/installer/  NSIS setup.exe, MSI
# Layout (Linux):   Release/Linux/bin/           portable binary
#                   Release/Linux/installer/     AppImage, deb
# All errors are printed with an ERROR: prefix and exit with a nonzero code.
# No blocking prompt at the end unless -Pause is given (agents must never block).
# ============================================================================
$ErrorActionPreference = "Stop"

function Show-Usage {
  @"
Usage: build-release.ps1 [options]

  One-button local Release build for CanbanVoimaxHarness (PowerShell 5.1+).
  Run from the repository root. Deterministic output layout:
    Release/Windows/bin/        portable exe (canban-voimax-harness.exe)
    Release/Windows/installer/  NSIS setup.exe, MSI
    Release/Linux/bin/          portable binary (canban-voimax-harness)
    Release/Linux/installer/    AppImage, deb

  Options:
    -SkipInstall | --skip-install    reuse existing node_modules (default: skip if present)
    -SkipChecks  | --skip-checks     skip cargo check + cargo clippy
    -Bundles LIST | --bundles=LIST   forward to "tauri build --bundles LIST" (e.g. nsis,msi)
    -Pause       | --pause           wait for Enter at the end (interactive use only)
    -Help        | --help, -h, /?    print this help and exit 0

  Examples:
    ./build-release.ps1
    ./build-release.ps1 -Bundles nsis,msi -SkipInstall

  NOTE: if execution policy blocks the script, run:
    powershell -NoProfile -ExecutionPolicy Bypass -File ./build-release.ps1
"@
}

function Fail([string]$Message, [int]$Code = 1) {
  [Console]::Error.WriteLine("ERROR: $Message")
  exit $Code
}

$SkipInstall = $false
$SkipChecks = $false
$Bundles = ""
$DoPause = $false
$i = 0
while ($i -lt $args.Count) {
  $a = [string]$args[$i]
  if ($a -eq "-h" -or $a -eq "-Help" -or $a -eq "--help" -or $a -eq "/?") { Show-Usage; exit 0 }
  elseif ($a -eq "--skip-install" -or $a -eq "-SkipInstall") { $SkipInstall = $true }
  elseif ($a -eq "--skip-checks" -or $a -eq "-SkipChecks") { $SkipChecks = $true }
  elseif ($a -eq "--pause" -or $a -eq "-Pause") { $DoPause = $true }
  elseif ($a -like "--bundles=*") {
    $Bundles = $a.Substring(11)
    if ([string]::IsNullOrWhiteSpace($Bundles)) { Fail "--bundles requires a non-empty value." 2 }
  }
  elseif ($a -eq "--bundles" -or $a -eq "-Bundles") {
    $i++
    if ($i -ge $args.Count) { Fail "--bundles requires a value." 2 }
    $Bundles = [string]$args[$i]
    if ([string]::IsNullOrWhiteSpace($Bundles)) { Fail "--bundles requires a non-empty value." 2 }
  }
  else { Fail "unknown flag: $a. Run ./build-release.ps1 -Help for usage." 2 }
  $i++
}

$OnWindows = ($env:OS -eq "Windows_NT")
if ($OnWindows) {
  $BinDir = "Release/Windows/bin"
  $InstDir = "Release/Windows/installer"
  $AppBin = "src-tauri/target/release/canban-voimax-harness.exe"
} else {
  $BinDir = "Release/Linux/bin"
  $InstDir = "Release/Linux/installer"
  $AppBin = "src-tauri/target/release/canban-voimax-harness"
}

Write-Host "CanbanVoimaxHarness local Release build"
Write-Host "  root: $((Get-Location).Path)"
if ([string]::IsNullOrEmpty($Bundles)) { Write-Host "  bundles: (tauri.conf.json defaults)" } else { Write-Host "  bundles: $Bundles" }
Write-Host ""

Write-Host "[1/5] Preflight: checking pnpm, node, cargo..."
foreach ($tool in @("pnpm", "node", "cargo")) {
  if ($null -eq (Get-Command $tool -ErrorAction SilentlyContinue)) {
    Fail "$tool not found in PATH. Install Node.js 20 + pnpm and the Rust stable toolchain."
  }
}
Write-Host "  ok: pnpm, node, cargo found."
Write-Host ""

Write-Host "[2/5] Installing frontend dependencies..."
if ($SkipInstall) {
  Write-Host "  skipped (--skip-install)."
} elseif (Test-Path -LiteralPath "node_modules") {
  Write-Host "  skipped (node_modules exists)."
} else {
  pnpm install --frozen-lockfile
  if ($LASTEXITCODE -ne 0) { Fail "pnpm install --frozen-lockfile failed." }
}
Write-Host ""

Write-Host "[3/5] Building frontend (pnpm run build)..."
pnpm run build
if ($LASTEXITCODE -ne 0) { Fail "pnpm run build failed (tsc && vite build)." }
Write-Host ""

Write-Host "[4/5] Rust sensors (cargo check + clippy)..."
if ($SkipChecks) {
  Write-Host "  skipped (--skip-checks)."
} else {
  if (-not (Test-Path -LiteralPath "src-tauri")) { Fail "src-tauri directory not found. Run from the repository root." }
  Push-Location -LiteralPath "src-tauri"
  try {
    cargo check
    if ($LASTEXITCODE -ne 0) { Fail "cargo check failed." }
    cargo clippy -- -D warnings
    if ($LASTEXITCODE -ne 0) { Fail "cargo clippy -- -D warnings failed." }
  } finally {
    Pop-Location
  }
}
Write-Host ""

Write-Host "[5/5] Building Tauri release bundles..."
if ([string]::IsNullOrEmpty($Bundles)) {
  pnpm tauri build
} else {
  pnpm tauri build --bundles $Bundles
}
if ($LASTEXITCODE -ne 0) { Fail "pnpm tauri build failed." }
Write-Host ""

Write-Host "Collecting artifacts..."
New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
New-Item -ItemType Directory -Force -Path $InstDir | Out-Null
if (-not (Test-Path -LiteralPath $AppBin -PathType Leaf)) { Fail "portable binary not found: $AppBin" }
Copy-Item -Force -LiteralPath $AppBin -Destination $BinDir
$copied = Join-Path $BinDir (Split-Path -Leaf $AppBin)
$installerFiles = @()
if ($OnWindows) {
  $installerFiles += @(Get-ChildItem "src-tauri/target/release/bundle/nsis/*.exe" -ErrorAction SilentlyContinue)
  $installerFiles += @(Get-ChildItem "src-tauri/target/release/bundle/msi/*.msi" -ErrorAction SilentlyContinue)
} else {
  $installerFiles += @(Get-ChildItem "src-tauri/target/release/bundle/appimage/*.AppImage" -ErrorAction SilentlyContinue)
  $installerFiles += @(Get-ChildItem "src-tauri/target/release/bundle/deb/*.deb" -ErrorAction SilentlyContinue)
}
foreach ($f in $installerFiles) { Copy-Item -Force -LiteralPath $f.FullName -Destination $InstDir }
if ($installerFiles.Count -eq 0) { Write-Host "  NOTE: no installers found; portable binary only." }
Write-Host ""

Write-Host "Release artifacts:"
$binItem = Get-Item -LiteralPath $copied
Write-Host ("  {0}  [{1} bytes]" -f $copied, $binItem.Length)
foreach ($f in (Get-ChildItem -LiteralPath $InstDir -File)) {
  Write-Host ("  {0}  [{1} bytes]" -f $f.FullName, $f.Length)
}
Write-Host ""
Write-Host "DONE: local Release build complete."
if ($DoPause) {
  Write-Host "Press Enter to continue..."
  [void][Console]::ReadLine()
}
exit 0
