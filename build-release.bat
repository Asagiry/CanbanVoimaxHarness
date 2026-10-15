@echo off
rem ==========================================================================
rem CanbanVoimaxHarness - one-button local Release build (Windows, cmd.exe).
rem Single entry point for agents: run from the repository root, no questions.
rem
rem   build-release.bat [--skip-install] [--skip-checks] [--bundles=LIST] [--pause] [--help]
rem
rem Pipeline: [1/5] preflight, [2/5] pnpm install, [3/5] frontend build,
rem             [4/5] cargo check + clippy, [5/5] tauri build + collect.
rem Layout:   Release\Windows\bin\        portable exe
rem           Release\Windows\installer\   NSIS setup.exe, MSI
rem Flags:    --skip-install  reuse existing node_modules
rem           --skip-checks   skip cargo check + clippy
rem           --bundles=LIST  forward to "tauri build --bundles LIST" (e.g. nsis,msi)
rem           --pause         keep the window open at the end (interactive use only)
rem           --help, -h, /?  print this help and exit 0
rem All errors are printed with an ERROR: prefix and exit with a nonzero code.
rem No pause at the end unless --pause is given (agents must never block).
rem ==========================================================================
setlocal
set "SKIP_INSTALL=0"
set "SKIP_CHECKS=0"
set "BUNDLES="
set "DO_PAUSE=0"

:parse_args
if "%~1"=="" goto args_done
if "%~1"=="--skip-install" set "SKIP_INSTALL=1"
if "%~1"=="--skip-install" goto shift_arg
if "%~1"=="--skip-checks" set "SKIP_CHECKS=1"
if "%~1"=="--skip-checks" goto shift_arg
if "%~1"=="--pause" set "DO_PAUSE=1"
if "%~1"=="--pause" goto shift_arg
if "%~1"=="--help" goto usage
if "%~1"=="-h" goto usage
if "%~1"=="/?" goto usage
set "ARG=%~1"
if "%ARG:~0,10%"=="--bundles=" goto bundles_eq
if "%~1"=="--bundles" goto bundles_next
echo ERROR: unknown flag: %~1 1>&2
echo Run "build-release.bat --help" for usage. 1>&2
exit /b 2
:bundles_eq
set "BUNDLES=%ARG:~10%"
if "%BUNDLES%"=="" (
  echo ERROR: --bundles requires a non-empty value. 1>&2
  exit /b 2
)
goto shift_arg
:bundles_next
if "%~2"=="" (
  echo ERROR: --bundles requires a value. 1>&2
  exit /b 2
)
set "BUNDLES=%~2"
shift
goto shift_arg
:shift_arg
shift
goto parse_args
:args_done

if /I not "%OS%"=="Windows_NT" (
  echo ERROR: build-release.bat requires Windows cmd.exe. On Linux run ./build-release.sh --help 1>&2
  exit /b 1
)

set "BIN_DIR=Release\Windows\bin"
set "INST_DIR=Release\Windows\installer"
set "APP_EXE=src-tauri\target\release\canban-voimax-harness.exe"

echo CanbanVoimaxHarness local Release build
echo   root: %CD%
if "%BUNDLES%"=="" (
  echo   bundles: ^(tauri.conf.json defaults^)
) else (
  echo   bundles: %BUNDLES%
)
echo.

echo [1/5] Preflight: checking pnpm, node, cargo...
where pnpm >nul 2>nul
if errorlevel 1 (
  echo ERROR: pnpm not found in PATH. Install Node.js 20 + pnpm. 1>&2
  exit /b 1
)
where node >nul 2>nul
if errorlevel 1 (
  echo ERROR: node not found in PATH. Install Node.js 20. 1>&2
  exit /b 1
)
where cargo >nul 2>nul
if errorlevel 1 (
  echo ERROR: cargo not found in PATH. Install the Rust stable-msvc toolchain. 1>&2
  exit /b 1
)
echo   ok: pnpm, node, cargo found.
echo.

echo [2/5] Installing frontend dependencies...
if "%SKIP_INSTALL%"=="1" (
  echo   skipped (--skip-install).
) else if exist "node_modules" (
  echo   skipped (node_modules exists).
) else (
  call pnpm install --frozen-lockfile
  if errorlevel 1 (
    echo ERROR: pnpm install --frozen-lockfile failed. 1>&2
    exit /b 1
  )
)
echo.

echo [3/5] Building frontend (pnpm run build)...
call pnpm run build
if errorlevel 1 (
  echo ERROR: pnpm run build failed (tsc ^& vite build). 1>&2
  exit /b 1
)
echo.

echo [4/5] Rust sensors (cargo check + clippy)...
if "%SKIP_CHECKS%"=="1" (
  echo   skipped (--skip-checks).
) else (
  if not exist "src-tauri" (
    echo ERROR: src-tauri directory not found. Run from the repository root. 1>&2
    exit /b 1
  )
  pushd "src-tauri"
  cargo check
  if errorlevel 1 (
    echo ERROR: cargo check failed. 1>&2
    popd
    exit /b 1
  )
  cargo clippy -- -D warnings
  if errorlevel 1 (
    echo ERROR: cargo clippy -- -D warnings failed. 1>&2
    popd
    exit /b 1
  )
  popd
)
echo.

echo [5/5] Building Tauri release bundles...
if "%BUNDLES%"=="" (
  call pnpm tauri build
) else (
  call pnpm tauri build --bundles %BUNDLES%
)
if errorlevel 1 (
  echo ERROR: pnpm tauri build failed. 1>&2
  exit /b 1
)
echo.

echo Collecting artifacts...
if not exist "%BIN_DIR%" mkdir "%BIN_DIR%"
if not exist "%INST_DIR%" mkdir "%INST_DIR%"
if not exist "%APP_EXE%" (
  echo ERROR: portable exe not found: %APP_EXE% 1>&2
  exit /b 1
)
copy /Y "%APP_EXE%" "%BIN_DIR%\" >nul
if errorlevel 1 (
  echo ERROR: failed to copy %APP_EXE% to %BIN_DIR% 1>&2
  exit /b 1
)
set "COPIED_BIN=%BIN_DIR%\canban-voimax-harness.exe"
set "FOUND_INSTALLER=0"
for %%F in ("src-tauri\target\release\bundle\nsis\*.exe") do (
  if exist "%%F" (
    copy /Y "%%F" "%INST_DIR%\" >nul
    set "FOUND_INSTALLER=1"
  )
)
for %%F in ("src-tauri\target\release\bundle\msi\*.msi") do (
  if exist "%%F" (
    copy /Y "%%F" "%INST_DIR%\" >nul
    set "FOUND_INSTALLER=1"
  )
)
if "%FOUND_INSTALLER%"=="0" echo   NOTE: no NSIS/MSI installers found; portable exe only.
echo.

echo Release artifacts:
call :print_size "%COPIED_BIN%"
for %%F in ("%INST_DIR%\*") do call :print_size "%%F"
echo.
echo DONE: local Release build complete.
if "%DO_PAUSE%"=="1" pause
exit /b 0

:usage
echo Usage: build-release.bat [options]
echo.
echo   One-button local Release build for CanbanVoimaxHarness (Windows, cmd.exe).
echo   Run from the repository root. Deterministic output layout:
echo     Release\Windows\bin\        portable exe (canban-voimax-harness.exe)
echo     Release\Windows\installer\   NSIS setup.exe, MSI
echo.
echo   Options:
echo     --skip-install    reuse existing node_modules (default: skip if present)
echo     --skip-checks     skip cargo check + cargo clippy
echo     --bundles=LIST    forward to "tauri build --bundles LIST" (e.g. nsis,msi)
echo     --pause           keep the window open at the end (interactive use only)
echo     --help, -h, /?    print this help and exit 0
echo.
echo   Examples:
echo     build-release.bat
echo     build-release.bat --bundles=nsis,msi --skip-install
exit /b 0

:print_size
if not exist "%~1" exit /b 0
for %%S in ("%~1") do echo   %~1  [%%~zS bytes]
exit /b 0
