# ============================================================================
# CanbanVoimaxHarness - one-button local Release build (PowerShell 5.1+).
# Single entry point for agents: run from the repository root, no questions.
#
#   ./build-release.ps1 [-SkipInstall] [-SkipChecks] [-Bundles nsis] [-Target x86_64-pc-windows-gnu] [-NoPause] [-Pause] [-Help]
#   Double-dash spellings (--skip-install, --bundles=nsis, --target=..., --no-pause, --pause, --help) work too.
#
# Pipeline: [1/5] preflight, [2/5] pnpm install, [3/5] frontend build,
#             [4/5] cargo check + clippy, [5/5] tauri build + collect.
# Layout (Windows): Release/Windows/bin/        portable exe + WebView2Loader.dll
#                   Release/Windows/installer/  NSIS setup.exe, MSI
# Layout (Linux):   Release/Linux/bin/           portable binary
#                   Release/Linux/installer/     AppImage, deb
# All errors are printed with an ERROR: prefix and exit with a nonzero code.
# Window stays open on completion or error unless -NoPause / --no-pause is passed.
# ============================================================================
$ErrorActionPreference = "Stop"

function Show-Usage {
  @"
Usage: build-release.ps1 [options]

  One-button local Release build for CanbanVoimaxHarness (PowerShell 5.1+).
  Run from the repository root. Deterministic output layout:
    Release/Windows/bin/        portable exe (canban-voimax-harness.exe) + WebView2Loader.dll
    Release/Windows/installer/  NSIS setup.exe, MSI
    Release/Linux/bin/          portable binary (canban-voimax-harness)
    Release/Linux/installer/    AppImage, deb

  Options:
    -SkipInstall | --skip-install    reuse existing node_modules (default: skip if present)
    -SkipChecks  | --skip-checks     skip cargo check + cargo clippy
    -Bundles LIST | --bundles=LIST   forward to "tauri build --bundles LIST" (e.g. nsis)
    -Target TRIPLE | --target=TRIPLE target triple (e.g. x86_64-pc-windows-gnu)
    -NoPause     | --no-pause        do not pause before exit (agents and automated scripts)
    -Pause       | --pause           wait for Enter at the end (default behavior)
    -Help        | --help, -h, /?    print this help and exit 0

  Examples:
    ./build-release.ps1
    ./build-release.ps1 -Target x86_64-pc-windows-gnu -Bundles nsis -SkipInstall

  NOTE: if execution policy blocks the script, run:
    powershell -NoProfile -ExecutionPolicy Bypass -File ./build-release.ps1
"@
}

$NoPause = $false

function Fail([string]$Message, [int]$Code = 1) {
  [Console]::Error.WriteLine("ERROR: $Message")
  if (-not $script:NoPause) {
    Write-Host "Press Enter to exit..."
    if (-not [Console]::IsInputRedirected) {
      try { [void][Console]::ReadLine() } catch {}
    }
  }
  exit $Code
}

$SkipInstall = $false
$SkipChecks = $false
$Bundles = ""
$Target = ""
$i = 0
while ($i -lt $args.Count) {
  $a = [string]$args[$i]
  if ($a -eq "-h" -or $a -eq "-Help" -or $a -eq "--help" -or $a -eq "/?") { Show-Usage; exit 0 }
  elseif ($a -eq "--skip-install" -or $a -eq "-SkipInstall") { $SkipInstall = $true }
  elseif ($a -eq "--skip-checks" -or $a -eq "-SkipChecks") { $SkipChecks = $true }
  elseif ($a -eq "--no-pause" -or $a -eq "-NoPause") { $NoPause = $true }
  elseif ($a -eq "--pause" -or $a -eq "-Pause") { $NoPause = $false }
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
  elseif ($a -like "--target=*") {
    $Target = $a.Substring(9)
    if ([string]::IsNullOrWhiteSpace($Target)) { Fail "--target requires a non-empty value." 2 }
  }
  elseif ($a -eq "--target" -or $a -eq "-Target") {
    $i++
    if ($i -ge $args.Count) { Fail "--target requires a value." 2 }
    $Target = [string]$args[$i]
    if ([string]::IsNullOrWhiteSpace($Target)) { Fail "--target requires a non-empty value." 2 }
  }
  else { Fail "unknown flag: $a. Run ./build-release.ps1 -Help for usage." 2 }
  $i++
}

$OnWindows = ($env:OS -eq "Windows_NT")
if ($OnWindows) {
  $BinDir = "Release/Windows/bin"
  $InstDir = "Release/Windows/installer"
} else {
  $BinDir = "Release/Linux/bin"
  $InstDir = "Release/Linux/installer"
}

Write-Host "CanbanVoimaxHarness local Release build"
Write-Host "  root: $((Get-Location).Path)"

# Local short temporary directory on Windows to prevent path length/permission issues
if ($OnWindows) {
  $localTmp = Join-Path (Get-Location).Path "src-tauri/target/tmp"
  New-Item -ItemType Directory -Force -Path $localTmp | Out-Null
  $env:TMP = $localTmp
  $env:TEMP = $localTmp
}

Write-Host "[1/5] Preflight: checking compiler, cargo, node, pnpm..."

# Auto-add llvm-mingw to PATH on Windows if found
if ($OnWindows) {
  $mingwCandidates = @(
    "C:/Users/Voimax/llvm-mingw-20261006-ucrt-x86_64/bin",
    "C:/llvm-mingw/bin"
  )
  foreach ($mc in $mingwCandidates) {
    if (Test-Path -LiteralPath (Join-Path $mc "gcc.exe") -PathType Leaf) {
      if ($env:PATH -notlike "*$mc*") {
        $env:PATH = "$mc;$env:PATH"
        Write-Host "  notice: added $mc to PATH"
      }
      break
    }
  }
}

# Auto-add cargo to PATH if missing
if ($null -eq (Get-Command cargo -ErrorAction SilentlyContinue)) {
  $cargoCandidates = @(
    (Join-Path $env:USERPROFILE ".cargo/bin/cargo.exe"),
    (Join-Path $HOME ".cargo/bin/cargo"),
    "C:/Users/Voimax/.cargo/bin/cargo.exe"
  )
  foreach ($c in $cargoCandidates) {
    if (Test-Path -LiteralPath $c -PathType Leaf) {
      $cargoDir = Split-Path -Parent $c
      $env:PATH = "$cargoDir;$env:PATH"
      Write-Host "  notice: added $cargoDir to PATH"
      break
    }
  }
}

# Auto-add node to PATH if missing
if ($null -eq (Get-Command node -ErrorAction SilentlyContinue)) {
  $nodeCandidates = @(
    (Join-Path $env:LOCALAPPDATA "Programs/nodejs/node.exe"),
    (Join-Path $env:ProgramFiles "nodejs/node.exe"),
    "C:/Program Files/nodejs/node.exe",
    "C:/Users/Voimax/.dsh/dsh-runtimes/dsh-primary-runtime/dependencies/node/bin/node.exe"
  )
  foreach ($cand in $nodeCandidates) {
    if (Test-Path -LiteralPath $cand -PathType Leaf) {
      $nodeDir = Split-Path -Parent $cand
      $env:PATH = "$nodeDir;$env:PATH"
      Write-Host "  notice: added $nodeDir to PATH"
      break
    }
  }
}

# Auto-add pnpm to PATH or create shim
if ($null -eq (Get-Command pnpm -ErrorAction SilentlyContinue)) {
  $pnpmCandidates = @(
    (Join-Path $env:APPDATA "npm/pnpm.cmd"),
    (Join-Path $env:LOCALAPPDATA "Programs/nodejs/pnpm.cmd"),
    (Join-Path $env:LOCALAPPDATA "pnpm/pnpm.exe"),
    (Join-Path $env:ProgramFiles "nodejs/node_modules/corepack/shims/pnpm.cmd"),
    "C:/Program Files/nodejs/node_modules/corepack/shims/pnpm.cmd"
  )
  foreach ($cand in $pnpmCandidates) {
    if (Test-Path -LiteralPath $cand -PathType Leaf) {
      $pnpmDir = Split-Path -Parent $cand
      $env:PATH = "$pnpmDir;$env:PATH"
      Write-Host "  notice: added $pnpmDir to PATH"
      break
    }
  }
}

# If pnpm still not in PATH, create shim from bundled pnpm.mjs if available
if ($null -eq (Get-Command pnpm -ErrorAction SilentlyContinue)) {
  $bundledPnpm = "C:/Users/Voimax/.dsh/dsh-runtimes/dsh-primary-runtime/dependencies/pnpm/bin/pnpm.mjs"
  if (Test-Path -LiteralPath $bundledPnpm -PathType Leaf) {
    $shimDir = Join-Path (Get-Location).Path "src-tauri/target/bin"
    New-Item -ItemType Directory -Force -Path $shimDir | Out-Null
    $shimPath = Join-Path $shimDir "pnpm.cmd"
    Set-Content -Path $shimPath -Value "@`"node`" `"$bundledPnpm`" %*" -Encoding Ascii
    $env:PATH = "$shimDir;$env:PATH"
    Write-Host "  notice: created pnpm shim at $shimPath"
  }
}

foreach ($tool in @("cargo", "node", "pnpm")) {
  if ($null -eq (Get-Command $tool -ErrorAction SilentlyContinue)) {
    Fail "$tool not found in PATH. Install Node.js 20 + pnpm and the Rust toolchain."
  }
}
Write-Host "  ok: cargo, node, pnpm found."

# Target resolution
if ($OnWindows) {
  if ([string]::IsNullOrEmpty($Target)) {
    $hasMsvc = $null -ne (Get-Command link.exe -ErrorAction SilentlyContinue)
    $hasGnuGcc = $null -ne (Get-Command gcc.exe -ErrorAction SilentlyContinue)
    $hasGnuTarget = @(rustup target list --installed 2>$null) -contains "x86_64-pc-windows-gnu"
    if (-not $hasMsvc -and $hasGnuGcc -and $hasGnuTarget) {
      $Target = "x86_64-pc-windows-gnu"
      Write-Host "  notice: MSVC link.exe not found and GNU target installed; defaulting target to $Target"
    }
  }
  if ([string]::IsNullOrEmpty($Bundles)) {
    $Bundles = "nsis"
  }
}

if ([string]::IsNullOrEmpty($Target)) { Write-Host "  target: host default" } else { Write-Host "  target: $Target" }
if ([string]::IsNullOrEmpty($Bundles)) { Write-Host "  bundles: (tauri.conf.json defaults)" } else { Write-Host "  bundles: $Bundles" }
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
    if (-not [string]::IsNullOrEmpty($Target)) {
      cargo check --target $Target
      if ($LASTEXITCODE -ne 0) { Fail "cargo check --target $Target failed." }
      cargo clippy --target $Target -- -D warnings
      if ($LASTEXITCODE -ne 0) { Fail "cargo clippy --target $Target -- -D warnings failed." }
    } else {
      cargo check
      if ($LASTEXITCODE -ne 0) { Fail "cargo check failed." }
      cargo clippy -- -D warnings
      if ($LASTEXITCODE -ne 0) { Fail "cargo clippy -- -D warnings failed." }
    }
  } finally {
    Pop-Location
  }
}
Write-Host ""

Write-Host "[5/5] Building Tauri release bundles..."
# Ensure WebView2Loader.dll is present in target release directory for NSIS bundling on Windows
if ($OnWindows) {
  $targetReleaseDir = if (-not [string]::IsNullOrEmpty($Target)) { "src-tauri/target/$Target/release" } else { "src-tauri/target/release" }
  $targetDll = Join-Path $targetReleaseDir "WebView2Loader.dll"
  if (-not (Test-Path -LiteralPath $targetDll -PathType Leaf)) {
    $foundBuildDll = Get-ChildItem "$targetReleaseDir/build" -Recurse -Filter "WebView2Loader.dll" -ErrorAction SilentlyContinue |
      Where-Object { $_.FullName -like "*x64*" } | Select-Object -First 1
    if ($foundBuildDll) {
      Copy-Item -Force -LiteralPath $foundBuildDll.FullName -Destination $targetReleaseDir
      Write-Host "  notice: pre-copied WebView2Loader.dll to $targetReleaseDir"
    }
  }
}
$tauriArgs = @("tauri", "build")
if (-not [string]::IsNullOrEmpty($Target)) {
  $tauriArgs += @("--target", $Target)
}
if (-not [string]::IsNullOrEmpty($Bundles)) {
  $tauriArgs += @("--bundles", $Bundles)
}
pnpm @tauriArgs
if ($LASTEXITCODE -ne 0) { Fail "pnpm tauri build failed." }
Write-Host ""

Write-Host "Collecting artifacts..."
New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
New-Item -ItemType Directory -Force -Path $InstDir | Out-Null

$targetDirs = @()
if (-not [string]::IsNullOrEmpty($env:CARGO_TARGET_DIR)) {
  $targetDirs += $env:CARGO_TARGET_DIR
}
$targetDirs += "src-tauri/target"

# Resolve binary source paths (newest first)
$binCandidates = @()
foreach ($td in $targetDirs) {
  if (-not [string]::IsNullOrEmpty($Target)) {
    if ($OnWindows) {
      $binCandidates += "$td/$Target/release/canban-voimax-harness.exe"
    } else {
      $binCandidates += "$td/$Target/release/canban-voimax-harness"
    }
  }
  if ($OnWindows) {
    $binCandidates += "$td/release/canban-voimax-harness.exe"
  } else {
    $binCandidates += "$td/release/canban-voimax-harness"
  }
}

$foundBinItems = @()
foreach ($cand in $binCandidates) {
  if (Test-Path -LiteralPath $cand -PathType Leaf) {
    $foundBinItems += (Get-Item -LiteralPath $cand)
  }
}
if ($foundBinItems.Count -eq 0) { Fail "portable binary not found in: $($binCandidates -join ', ')" }
$newestBin = ($foundBinItems | Sort-Object LastWriteTime -Descending)[0]

Copy-Item -Force -LiteralPath $newestBin.FullName -Destination $BinDir
$copiedBin = Join-Path $BinDir (Split-Path -Leaf $newestBin.FullName)

# On Windows (GNU), copy WebView2Loader.dll alongside the portable exe
if ($OnWindows) {
  $dllCandidates = @()
  foreach ($td in $targetDirs) {
    if (-not [string]::IsNullOrEmpty($Target)) {
      $dllCandidates += "$td/$Target/release/WebView2Loader.dll"
    }
    $dllCandidates += "$td/release/WebView2Loader.dll"
  }

  $foundDllItems = @()
  foreach ($cand in $dllCandidates) {
    if (Test-Path -LiteralPath $cand -PathType Leaf) {
      $foundDllItems += (Get-Item -LiteralPath $cand)
    }
  }
  if ($foundDllItems.Count -gt 0) {
    $newestDll = ($foundDllItems | Sort-Object LastWriteTime -Descending)[0]
    Copy-Item -Force -LiteralPath $newestDll.FullName -Destination $BinDir
    Write-Host "  ok: copied WebView2Loader.dll to $BinDir"
  } else {
    Write-Host "  notice: WebView2Loader.dll not present in release output (statically linked or not required)."
  }
}

# Resolve installer files (prefer targeted, newest by leaf name)
$installerCandidates = @()
foreach ($td in $targetDirs) {
  if ($OnWindows) {
    if (-not [string]::IsNullOrEmpty($Target)) {
      $installerCandidates += @(Get-ChildItem "$td/$Target/release/bundle/nsis/*.exe" -ErrorAction SilentlyContinue)
      $installerCandidates += @(Get-ChildItem "$td/$Target/release/bundle/msi/*.msi" -ErrorAction SilentlyContinue)
    }
    if ($installerCandidates.Count -eq 0) {
      $installerCandidates += @(Get-ChildItem "$td/release/bundle/nsis/*.exe" -ErrorAction SilentlyContinue)
      $installerCandidates += @(Get-ChildItem "$td/release/bundle/msi/*.msi" -ErrorAction SilentlyContinue)
    }
  } else {
    if (-not [string]::IsNullOrEmpty($Target)) {
      $installerCandidates += @(Get-ChildItem "$td/$Target/release/bundle/appimage/*.AppImage" -ErrorAction SilentlyContinue)
      $installerCandidates += @(Get-ChildItem "$td/$Target/release/bundle/deb/*.deb" -ErrorAction SilentlyContinue)
    }
    if ($installerCandidates.Count -eq 0) {
      $installerCandidates += @(Get-ChildItem "$td/release/bundle/appimage/*.AppImage" -ErrorAction SilentlyContinue)
      $installerCandidates += @(Get-ChildItem "$td/release/bundle/deb/*.deb" -ErrorAction SilentlyContinue)
    }
  }
}

$seenByLeaf = @{}
foreach ($f in ($installerCandidates | Sort-Object LastWriteTime -Descending)) {
  $leaf = Split-Path -Leaf $f.FullName
  if (-not $seenByLeaf.ContainsKey($leaf)) {
    $seenByLeaf[$leaf] = $true
    Copy-Item -Force -LiteralPath $f.FullName -Destination $InstDir
  }
}
if ($seenByLeaf.Count -eq 0) { Write-Host "  NOTE: no installers found; portable binary only." }
Write-Host ""

Write-Host "Release artifacts:"
$binItem = Get-Item -LiteralPath $copiedBin
Write-Host ("  {0}  [{1} bytes]" -f $copiedBin, $binItem.Length)
foreach ($f in (Get-ChildItem -LiteralPath $BinDir -File)) {
  if ($f.FullName -ne $binItem.FullName) {
    Write-Host ("  {0}  [{1} bytes]" -f $f.FullName, $f.Length)
  }
}
foreach ($f in (Get-ChildItem -LiteralPath $InstDir -File)) {
  Write-Host ("  {0}  [{1} bytes]" -f $f.FullName, $f.Length)
}
Write-Host ""
Write-Host "DONE: local Release build complete."
if (-not $NoPause) {
  Write-Host "Press Enter to continue..."
  if (-not [Console]::IsInputRedirected) {
    try { [void][Console]::ReadLine() } catch {}
  }
}
exit 0
