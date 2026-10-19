; Installer hooks (T-382). Tauri's uninstaller offers one checkbox,
; "Delete the application data", which on its own removes only the folders
; named after the app's identifier (the window's WebView2 cache). The label
; in English.nsh says "my games and settings", so the box must do that: it
; also removes the app's own data folder, %APPDATA%\TavernLedger (the game
; history, settings, upload settings, overlay layout, card cache and logs).
; Unticked, nothing of it is touched. An update (the in-app updater runs the
; old uninstaller in update mode) never deletes it.
!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $DeleteAppDataCheckboxState = 1
  ${AndIf} $UpdateMode <> 1
    SetShellVarContext current
    RmDir /r "$APPDATA\TavernLedger"
  ${EndIf}
!macroend
