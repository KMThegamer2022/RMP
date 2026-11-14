Unicode true
!include MUI2.nsh
!include x64.nsh
!include WinVer.nsh
Name "RMP · Rust Media Player"
OutFile "../RMP-Setup-x64.exe"
InstallDir "$LOCALAPPDATA\Programs\RMP"
RequestExecutionLevel user
Icon "../assets/RMP.ico"
!define MUI_ICON "../assets/RMP.ico"
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"
Function .onInit
 ${IfNot} ${RunningX64}
  MessageBox MB_OK "RMP requires 64-bit Windows."
  Abort
 ${EndIf}
 ${IfNot} ${AtLeastWin7}
  MessageBox MB_OK "RMP requires Windows 7 or newer."
  Abort
 ${EndIf}
FunctionEnd
Section
 SetOutPath "$INSTDIR"
 File /r "..\build\RMP\*.*"
 WriteUninstaller "$INSTDIR\Uninstall.exe"
 CreateShortcut "$SMPROGRAMS\RMP.lnk" "$INSTDIR\RMP.exe"
 WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\RMP" "DisplayName" "RMP · Rust Media Player"
 WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\RMP" "DisplayVersion" "2.0.0"
 WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\RMP" "UninstallString" '"$INSTDIR\Uninstall.exe"'
SectionEnd
Section "Uninstall"
 !include "uninstall-files.nsh"
 Delete "$INSTDIR\Uninstall.exe"
 RMDir "$INSTDIR"
 Delete "$SMPROGRAMS\RMP.lnk"
 DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\RMP"
SectionEnd
