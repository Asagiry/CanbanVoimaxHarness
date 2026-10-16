@echo off
rem ==========================================================================
rem CanbanVoimaxHarness - one-button local Release build (Windows, cmd.exe).
rem Single entry point for agents: run from the repository root, no questions.
rem
rem   build-release.bat [--skip-install] [--skip-checks] [--bundles=LIST] [--target=TRIPLE] [--no-pause] [--pause] [--help]
rem
rem Pipeline: [1/5] preflight, [2/5] pnpm install, [3/5] frontend build,
rem             [4/5] cargo check + clippy, [5/5] tauri build + collect.
rem Layout:   Release\Windows\bin\        portable exe + WebView2Loader.dll
rem           Release\Windows\installer\  NSIS setup.exe, MSI
rem Flags:    --skip-install    reuse existing node_modules
rem           --skip-checks     skip cargo check + clippy
rem           --bundles=LIST    forward to "tauri build --bundles LIST" (e.g. nsis)
rem           --target=TRIPLE   target triple (e.g. x86_64-pc-windows-gnu)
rem           --no-pause        do not pause before exit (agents and automated scripts)
rem           --pause           pause before exit (default behavior)
rem           --help, -h, /?    print this help and exit 0
rem All errors are printed with an ERROR: prefix and exit with a nonzero code.
rem Window stays open on completion or error unless --no-pause is passed.
rem ==========================================================================
setlocal EnableExtensions
set "SKIP_INSTALL=0"
set "SKIP_CHECKS=0"
set "BUNDLES="
set "TARGET="
set "NO_PAUSE=0"
set "EXIT_CODE=0"

:parse_args
if "%~1"=="" goto args_done
if "%~1"=="--no-pause" (
  set "NO_PAUSE=1"
  shift
  goto parse_args
)
if "%~1"=="-no-pause" (
  set "NO_PAUSE=1"
  shift
  goto parse_args
)
if "%~1"=="--pause" (
  set "NO_PAUSE=0"
  shift
  goto parse_args
)
if "%~1"=="-pause" (
  set "NO_PAUSE=0"
  shift
  goto parse_args
)
if "%~1"=="--skip-install" (
  set "SKIP_INSTALL=1"
  shift
  goto parse_args
)
if "%~1"=="--skip-checks" (
  set "SKIP_CHECKS=1"
  shift
  goto parse_args
)
if "%~1"=="--help" goto usage
if "%~1"=="-h" goto usage
if "%~1"=="/?" goto usage
set "ARG=%~1"
if "%ARG:~0,10%"=="--bundles=" (
  set "BUNDLES=%ARG:~10%"
  if "%BUNDLES%"=="" (
    echo ERROR: --bundles requires a non-empty value. 1>&2
    goto error_exit_2
  )
  shift
  goto parse_args
)
if "%~1"=="--bundles" (
  if "%~2"=="" (
    echo ERROR: --bundles requires a value. 1>&2
    goto error_exit_2
  )
  set "BUNDLES=%~2"
  shift
  shift
  goto parse_args
)
if "%ARG:~0,9%"=="--target=" (
  set "TARGET=%ARG:~9%"
  if "%TARGET%"=="" (
    echo ERROR: --target requires a non-empty value. 1>&2
    goto error_exit_2
  )
  shift
  goto parse_args
)
if "%~1"=="--target" (
  if "%~2"=="" (
    echo ERROR: --target requires a value. 1>&2
    goto error_exit_2
  )
  set "TARGET=%~2"
  shift
  shift
  goto parse_args
)
echo ERROR: unknown flag: %~1 1>&2
echo Run "build-release.bat --help" for usage. 1>&2
goto error_exit_2

:args_done
if /I not "%OS%"=="Windows_NT" (
  echo ERROR: build-release.bat requires Windows cmd.exe. On Linux run ./build-release.sh --help 1>&2
  goto error_exit
)

set "BIN_DIR=Release\Windows\bin"
set "INST_DIR=Release\Windows\installer"

echo CanbanVoimaxHarness local Release build
echo   root: %CD%

rem Set local short temporary directory to avoid path length and permission issues
if not exist "src-tauri\target\tmp" mkdir "src-tauri\target\tmp"
set "TMP=%CD%\src-tauri\target\tmp"
set "TEMP=%CD%\src-tauri\target\tmp"

echo [1/5] Preflight: checking compiler, cargo, node, pnpm...

rem Auto-detect llvm-mingw
where gcc >nul 2>nul
if errorlevel 1 (
  if exist "C:\Users\Voimax\llvm-mingw-20261006-ucrt-x86_64\bin\gcc.exe" (
    set "PATH=C:\Users\Voimax\llvm-mingw-20261006-ucrt-x86_64\bin;%PATH%"
    echo   notice: added llvm-mingw to PATH
  )
)

rem Auto-detect cargo in %USERPROFILE%\.cargo\bin or C:\Users\Voimax\.cargo\bin
where cargo >nul 2>nul
if errorlevel 1 (
  if exist "%USERPROFILE%\.cargo\bin\cargo.exe" (
    set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
    echo   notice: added "%USERPROFILE%\.cargo\bin" to PATH
  ) else if exist "C:\Users\Voimax\.cargo\bin\cargo.exe" (
    set "PATH=C:\Users\Voimax\.cargo\bin;%PATH%"
    echo   notice: added "C:\Users\Voimax\.cargo\bin" to PATH
  )
)

rem Auto-detect node
where node >nul 2>nul
if errorlevel 1 (
  if exist "%LOCALAPPDATA%\Programs\nodejs\node.exe" (
    set "PATH=%LOCALAPPDATA%\Programs\nodejs;%PATH%"
    echo   notice: added "%LOCALAPPDATA%\Programs\nodejs" to PATH
  ) else if exist "%ProgramFiles%\nodejs\node.exe" (
    set "PATH=%ProgramFiles%\nodejs;%PATH%"
    echo   notice: added "%ProgramFiles%\nodejs" to PATH
  ) else if exist "C:\Program Files\nodejs\node.exe" (
    set "PATH=C:\Program Files\nodejs;%PATH%"
    echo   notice: added "C:\Program Files\nodejs" to PATH
  )
)

rem Auto-detect pnpm
where pnpm >nul 2>nul
if errorlevel 1 (
  if exist "%APPDATA%\npm\pnpm.cmd" (
    set "PATH=%APPDATA%\npm;%PATH%"
    echo   notice: added "%APPDATA%\npm" to PATH
  ) else if exist "%LOCALAPPDATA%\Programs\nodejs\pnpm.cmd" (
    set "PATH=%LOCALAPPDATA%\Programs\nodejs;%PATH%"
    echo   notice: added "%LOCALAPPDATA%\Programs\nodejs" to PATH
  ) else if exist "%ProgramFiles%\nodejs\node_modules\corepack\shims\pnpm.cmd" (
    set "PATH=%ProgramFiles%\nodejs\node_modules\corepack\shims;%PATH%"
    echo   notice: added corepack shims to PATH
  ) else if exist "C:\Program Files\nodejs\node_modules\corepack\shims\pnpm.cmd" (
    set "PATH=C:\Program Files\nodejs\node_modules\corepack\shims;%PATH%"
    echo   notice: added corepack shims to PATH
  )
)

rem If pnpm still not in PATH, create shim from bundled pnpm.mjs if available
where pnpm >nul 2>nul
if errorlevel 1 (
  if exist "C:\Users\Voimax\.dsh\dsh-runtimes\dsh-primary-runtime\dependencies\pnpm\bin\pnpm.mjs" (
    if not exist "src-tauri\target\bin" mkdir "src-tauri\target\bin"
    echo @"node" "C:\Users\Voimax\.dsh\dsh-runtimes\dsh-primary-runtime\dependencies\pnpm\bin\pnpm.mjs" %%* > "src-tauri\target\bin\pnpm.cmd"
    set "PATH=%CD%\src-tauri\target\bin;%PATH%"
    echo   notice: created pnpm shim in src-tauri\target\bin
  )
)

rem Validate required tools
where cargo >nul 2>nul
if errorlevel 1 (
  echo ERROR: cargo not found in PATH or in .cargo\bin. Install the Rust toolchain. 1>&2
  goto error_exit
)
where node >nul 2>nul
if errorlevel 1 (
  echo ERROR: node not found in PATH. Install Node.js 20. 1>&2
  goto error_exit
)
where pnpm >nul 2>nul
if errorlevel 1 (
  echo ERROR: pnpm not found in PATH. Install pnpm. 1>&2
  goto error_exit
)
echo   ok: cargo, node, pnpm found.

rem Target and bundle defaults
if "%TARGET%"=="" (
  where link.exe >nul 2>nul
  if errorlevel 1 (
    where gcc.exe >nul 2>nul
    if not errorlevel 1 (
      rustup target list --installed 2>nul | findstr /i "x86_64-pc-windows-gnu" >nul
      if not errorlevel 1 (
        set "TARGET=x86_64-pc-windows-gnu"
        echo   notice: MSVC link.exe not found and GNU target installed; defaulting target to x86_64-pc-windows-gnu
      )
    )
  )
)
if "%BUNDLES%"=="" set "BUNDLES=nsis"

if "%TARGET%"=="" (
  echo   target: host default
) else (
  echo   target: %TARGET%
)
echo   bundles: %BUNDLES%
echo.

echo [2/5] Installing frontend dependencies...
if "%SKIP_INSTALL%"=="1" goto skip_install_flag
if exist "node_modules" goto skip_install_exists
goto do_install

:skip_install_flag
echo   skipped ^(--skip-install^).
goto install_done

:skip_install_exists
echo   skipped ^(node_modules exists^).
goto install_done

:do_install
call pnpm install --frozen-lockfile
if errorlevel 1 (
  echo ERROR: pnpm install --frozen-lockfile failed. 1>&2
  goto error_exit
)

:install_done
echo.

echo [3/5] Building frontend (pnpm run build)...
call pnpm run build
if errorlevel 1 (
  echo ERROR: pnpm run build failed ^(tsc ^& vite build^). 1>&2
  goto error_exit
)
echo.

echo [4/5] Rust sensors (cargo check + clippy)...
if "%SKIP_CHECKS%"=="1" goto skip_checks
if not exist "src-tauri" (
  echo ERROR: src-tauri directory not found. Run from the repository root. 1>&2
  goto error_exit
)
pushd "src-tauri"
if not "%TARGET%"=="" (
  cargo check --target %TARGET%
  if errorlevel 1 (
    echo ERROR: cargo check --target %TARGET% failed. 1>&2
    popd
    goto error_exit
  )
  cargo clippy --target %TARGET% -- -D warnings
  if errorlevel 1 (
    echo ERROR: cargo clippy --target %TARGET% -- -D warnings failed. 1>&2
    popd
    goto error_exit
  )
) else (
  cargo check
  if errorlevel 1 (
    echo ERROR: cargo check failed. 1>&2
    popd
    goto error_exit
  )
  cargo clippy -- -D warnings
  if errorlevel 1 (
    echo ERROR: cargo clippy -- -D warnings failed. 1>&2
    popd
    goto error_exit
  )
)
popd
goto checks_done

:skip_checks
echo   skipped ^(--skip-checks^).

:checks_done
echo.

echo [5/5] Building Tauri release bundles...
rem Ensure WebView2Loader.dll is present in target release directory for NSIS bundling
if not exist "src-tauri\target\release\WebView2Loader.dll" (
  for /r "src-tauri\target\release\build" %%D in (WebView2Loader.dll) do (
    if exist "%%D" copy /Y "%%D" "src-tauri\target\release\" >nul 2>nul
  )
)
if not "%TARGET%"=="" (
  if not exist "src-tauri\target\%TARGET%\release\WebView2Loader.dll" (
    for /r "src-tauri\target\%TARGET%\release\build" %%D in (WebView2Loader.dll) do (
      if exist "%%D" copy /Y "%%D" "src-tauri\target\%TARGET%\release\" >nul 2>nul
    )
  )
)
set "BUILD_CMD=pnpm tauri build"
if not "%TARGET%"=="" set "BUILD_CMD=%BUILD_CMD% --target %TARGET%"
if not "%BUNDLES%"=="" set "BUILD_CMD=%BUILD_CMD% --bundles %BUNDLES%"

call %BUILD_CMD%
if errorlevel 1 (
  echo ERROR: tauri build failed. 1>&2
  goto error_exit
)
echo.

echo Collecting artifacts...
if not exist "%BIN_DIR%" mkdir "%BIN_DIR%"
if not exist "%INST_DIR%" mkdir "%INST_DIR%"

set "APP_EXE="
if not "%CARGO_TARGET_DIR%"=="" (
  if not "%TARGET%"=="" (
    if exist "%CARGO_TARGET_DIR%\%TARGET%\release\canban-voimax-harness.exe" (
      set "APP_EXE=%CARGO_TARGET_DIR%\%TARGET%\release\canban-voimax-harness.exe"
    )
  )
  if "%APP_EXE%"=="" (
    if exist "%CARGO_TARGET_DIR%\release\canban-voimax-harness.exe" (
      set "APP_EXE=%CARGO_TARGET_DIR%\release\canban-voimax-harness.exe"
    )
  )
)
if "%APP_EXE%"=="" (
  if not "%TARGET%"=="" (
    if exist "src-tauri\target\%TARGET%\release\canban-voimax-harness.exe" (
      set "APP_EXE=src-tauri\target\%TARGET%\release\canban-voimax-harness.exe"
    )
  )
)
if "%APP_EXE%"=="" (
  if exist "src-tauri\target\release\canban-voimax-harness.exe" (
    set "APP_EXE=src-tauri\target\release\canban-voimax-harness.exe"
  )
)

if "%APP_EXE%"=="" (
  echo ERROR: portable exe not found. 1>&2
  goto error_exit
)

copy /Y "%APP_EXE%" "%BIN_DIR%\" >nul
if errorlevel 1 (
  echo ERROR: failed to copy %APP_EXE% to %BIN_DIR% 1>&2
  goto error_exit
)
set "COPIED_BIN=%BIN_DIR%\canban-voimax-harness.exe"

rem Copy WebView2Loader.dll if present
set "DLL_FOUND=0"
if not "%CARGO_TARGET_DIR%"=="" (
  if not "%TARGET%"=="" (
    if exist "%CARGO_TARGET_DIR%\%TARGET%\release\WebView2Loader.dll" (
      copy /Y "%CARGO_TARGET_DIR%\%TARGET%\release\WebView2Loader.dll" "%BIN_DIR%\" >nul
      set "DLL_FOUND=1"
    )
  )
  if "%DLL_FOUND%"=="0" (
    if exist "%CARGO_TARGET_DIR%\release\WebView2Loader.dll" (
      copy /Y "%CARGO_TARGET_DIR%\release\WebView2Loader.dll" "%BIN_DIR%\" >nul
      set "DLL_FOUND=1"
    )
  )
)
if "%DLL_FOUND%"=="0" (
  if not "%TARGET%"=="" (
    if exist "src-tauri\target\%TARGET%\release\WebView2Loader.dll" (
      copy /Y "src-tauri\target\%TARGET%\release\WebView2Loader.dll" "%BIN_DIR%\" >nul
      set "DLL_FOUND=1"
    )
  )
)
if "%DLL_FOUND%"=="0" (
  if exist "src-tauri\target\release\WebView2Loader.dll" (
    copy /Y "src-tauri\target\release\WebView2Loader.dll" "%BIN_DIR%\" >nul
    set "DLL_FOUND=1"
  )
)
if "%DLL_FOUND%"=="1" (
  echo   ok: copied WebView2Loader.dll to %BIN_DIR%
)

set "FOUND_INSTALLER=0"
if not "%CARGO_TARGET_DIR%"=="" (
  if not "%TARGET%"=="" (
    for %%F in ("%CARGO_TARGET_DIR%\%TARGET%\release\bundle\nsis\*.exe") do (
      if exist "%%F" (
        copy /Y "%%F" "%INST_DIR%\" >nul
        set "FOUND_INSTALLER=1"
      )
    )
    for %%F in ("%CARGO_TARGET_DIR%\%TARGET%\release\bundle\msi\*.msi") do (
      if exist "%%F" (
        copy /Y "%%F" "%INST_DIR%\" >nul
        set "FOUND_INSTALLER=1"
      )
    )
  )
)
if not "%TARGET%"=="" (
  for %%F in ("src-tauri\target\%TARGET%\release\bundle\nsis\*.exe") do (
    if exist "%%F" (
      copy /Y "%%F" "%INST_DIR%\" >nul
      set "FOUND_INSTALLER=1"
    )
  )
  for %%F in ("src-tauri\target\%TARGET%\release\bundle\msi\*.msi") do (
    if exist "%%F" (
      copy /Y "%%F" "%INST_DIR%\" >nul
      set "FOUND_INSTALLER=1"
    )
  )
)
if "%FOUND_INSTALLER%"=="0" (
  if not "%CARGO_TARGET_DIR%"=="" (
    for %%F in ("%CARGO_TARGET_DIR%\release\bundle\nsis\*.exe") do (
      if exist "%%F" (
        copy /Y "%%F" "%INST_DIR%\" >nul
        set "FOUND_INSTALLER=1"
      )
    )
    for %%F in ("%CARGO_TARGET_DIR%\release\bundle\msi\*.msi") do (
      if exist "%%F" (
        copy /Y "%%F" "%INST_DIR%\" >nul
        set "FOUND_INSTALLER=1"
      )
    )
  )
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
)

if "%FOUND_INSTALLER%"=="0" (
  echo   NOTE: no NSIS/MSI installers found; portable exe only.
)
echo.

echo Release artifacts:
call :print_size "%COPIED_BIN%"
if exist "%BIN_DIR%\WebView2Loader.dll" call :print_size "%BIN_DIR%\WebView2Loader.dll"
for %%F in ("%INST_DIR%\*") do (
  if exist "%%F" call :print_size "%%F"
)
echo.
echo DONE: local Release build complete.
if not "%NO_PAUSE%"=="1" pause
exit /b 0

:error_exit_2
set "EXIT_CODE=2"
goto error_exit

:error_exit
if "%EXIT_CODE%"=="" set "EXIT_CODE=1"
if "%EXIT_CODE%"=="0" set "EXIT_CODE=1"
echo.
echo ==============================================================================
echo [ERROR] Build script failed with exit code %EXIT_CODE%.
echo ==============================================================================
if not "%NO_PAUSE%"=="1" pause
exit /b %EXIT_CODE%

:usage
echo Usage: build-release.bat [options]
echo.
echo   One-button local Release build for CanbanVoimaxHarness (Windows, cmd.exe).
echo   Run from the repository root. Deterministic output layout:
echo     Release\Windows\bin\        portable exe + WebView2Loader.dll
echo     Release\Windows\installer\  NSIS setup.exe, MSI
echo.
echo   Options:
echo     --skip-install    reuse existing node_modules (default: skip if present)
echo     --skip-checks     skip cargo check + cargo clippy
echo     --bundles=LIST    forward to "tauri build --bundles LIST" (e.g. nsis)
echo     --target=TRIPLE   target triple (e.g. x86_64-pc-windows-gnu)
echo     --no-pause        do not pause before exit (agents and automated scripts)
echo     --pause           pause before exit (default behavior)
echo     --help, -h, /?    print this help and exit 0
echo.
echo   Examples:
echo     build-release.bat
echo     build-release.bat --target=x86_64-pc-windows-gnu --bundles=nsis --skip-install
exit /b 0

:print_size
if not exist "%~1" exit /b 0
for %%S in ("%~1") do echo   %~1  [%%~zS bytes]
exit /b 0
