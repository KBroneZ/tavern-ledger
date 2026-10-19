; Installer hooks (T-382). Tauri's uninstaller offers one checkbox,
; "Delete the application data", which on its own removes only the folders
; named after the app's identifier (the window's WebView2 cache). The label
; in English.nsh says "my games and settings", so the box must do that: it
; also removes the app's own data folder, %APPDATA%\TavernLedger (the game
; history, settings, upload settings, overlay layout, card cache and logs).
; Unticked, nothing of it is touched. An update (the in-app updater runs the
; old uninstaller in update mode) never deletes it.
;
; `RmDir /r` follows a junction or a symbolic link into its target. If the
; player made %APPDATA%\TavernLedger a link (to keep the history on another
; drive), only the link is removed: what it points at is theirs.
!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $DeleteAppDataCheckboxState = 1
  ${AndIf} $UpdateMode <> 1
    SetShellVarContext current
    ${If} $APPDATA != ""
      Push $0
      Push $1
      System::Call 'kernel32::GetFileAttributesW(w "$APPDATA\TavernLedger") i .r0'
      ; FILE_ATTRIBUTE_REPARSE_POINT = 0x400
      IntOp $1 $0 & 0x400
      ${If} $1 <> 0
        RmDir "$APPDATA\TavernLedger"
      ${Else}
        RmDir /r "$APPDATA\TavernLedger"
      ${EndIf}
      Pop $1
      Pop $0
    ${EndIf}
  ${EndIf}
!macroend
