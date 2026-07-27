; Persistent Windows shell discovery is installer-owned.
; A portable iTime.exe never executes these hooks.

!define ITIME_APP_PATHS_KEY "Software\Microsoft\Windows\CurrentVersion\App Paths\iTime.exe"
!define ITIME_APP_PATHS_OWNER "com.itime.desktop:nsis"

!macro NSIS_HOOK_POSTINSTALL
  ReadRegStr $0 HKCU "${ITIME_APP_PATHS_KEY}" "iTimeOwner"
  ReadRegStr $1 HKCU "${ITIME_APP_PATHS_KEY}" ""

  ; Refresh an entry already owned by this installer. Claim a new empty entry,
  ; but never overwrite an unknown owner or a different existing executable.
  ${If} $0 == "${ITIME_APP_PATHS_OWNER}"
    WriteRegStr HKCU "${ITIME_APP_PATHS_KEY}" "" "$INSTDIR\${MAINBINARYNAME}.exe"
    WriteRegStr HKCU "${ITIME_APP_PATHS_KEY}" "Path" "$INSTDIR"
    WriteRegStr HKCU "${ITIME_APP_PATHS_KEY}" "iTimeOwner" "${ITIME_APP_PATHS_OWNER}"
  ${ElseIf} $0 == ""
  ${AndIf} $1 == ""
    WriteRegStr HKCU "${ITIME_APP_PATHS_KEY}" "" "$INSTDIR\${MAINBINARYNAME}.exe"
    WriteRegStr HKCU "${ITIME_APP_PATHS_KEY}" "Path" "$INSTDIR"
    WriteRegStr HKCU "${ITIME_APP_PATHS_KEY}" "iTimeOwner" "${ITIME_APP_PATHS_OWNER}"
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ReadRegStr $0 HKCU "${ITIME_APP_PATHS_KEY}" "iTimeOwner"
  ReadRegStr $1 HKCU "${ITIME_APP_PATHS_KEY}" ""

  ; Delete only the exact entry created by this installer instance. An entry
  ; redirected to another copy is preserved even if an ownership marker remains.
  ${If} $0 == "${ITIME_APP_PATHS_OWNER}"
  ${AndIf} $1 == "$INSTDIR\${MAINBINARYNAME}.exe"
    DeleteRegKey HKCU "${ITIME_APP_PATHS_KEY}\"
  ${EndIf}
!macroend
