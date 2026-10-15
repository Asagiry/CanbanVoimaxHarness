; NSIS Hooks for CanbanVoimaxHarness
; Ensures WebView2Loader.dll is deployed alongside the main executable on Windows (GNU toolchain)

!macro NSIS_HOOK_POSTINSTALL
  File "${MAINBINARYSRCPATH}\..\WebView2Loader.dll"
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  Delete "$INSTDIR\WebView2Loader.dll"
!macroend
