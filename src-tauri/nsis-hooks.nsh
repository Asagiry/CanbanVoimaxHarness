; NSIS Hooks for CanbanVoimaxHarness
; Ensures WebView2Loader.dll is deployed alongside the main executable when present (e.g. GNU toolchain)

!macro NSIS_HOOK_POSTINSTALL
  !if /FileExists "${MAINBINARYSRCPATH}\..\WebView2Loader.dll"
    File "${MAINBINARYSRCPATH}\..\WebView2Loader.dll"
  !endif
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  Delete "$INSTDIR\WebView2Loader.dll"
!macroend
