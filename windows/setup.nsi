Unicode true
!include MUI2.nsh
!include x64.nsh
!include WinVer.nsh
!include nsDialogs.nsh
Var ThemeChoice
Var Theme0
Var Theme1
Var Theme2
Var Theme3
Name "RMP · Rust Media Player"
OutFile "../RMP-Setup-x64.exe"
InstallDir "$LOCALAPPDATA\Programs\RMP"
RequestExecutionLevel user
Icon "../assets/RMP.ico"
!define MUI_ICON "../assets/RMP.ico"
Page custom ThemePage ThemeLeave
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"
Function ThemePage
 !insertmacro MUI_HEADER_TEXT "Choose a player theme" "Choose how RMP looks when it opens."
 nsDialogs::Create 1018
 Pop $0
 ${NSD_CreateLabel} 0 4u 100% 28u "Select one theme. You can change it later using the Theme button in the player or the Theme menu."
 Pop $0
 ${NSD_CreateRadioButton} 8u 36u 90% 14u "Windows 10/11"
 Pop $Theme0
 ${If} $ThemeChoice == 0
  ${NSD_Check} $Theme0
 ${EndIf}
 ${NSD_CreateRadioButton} 8u 56u 90% 14u "Windows 7"
 Pop $Theme1
 ${If} $ThemeChoice == 1
  ${NSD_Check} $Theme1
 ${EndIf}
 ${NSD_CreateRadioButton} 8u 76u 90% 14u "Windows XP"
 Pop $Theme2
 ${If} $ThemeChoice == 2
  ${NSD_Check} $Theme2
 ${EndIf}
 ${NSD_CreateRadioButton} 8u 96u 90% 14u "Mac"
 Pop $Theme3
 ${If} $ThemeChoice == 3
  ${NSD_Check} $Theme3
 ${EndIf}
 nsDialogs::Show
FunctionEnd
Function ThemeLeave
 ${NSD_GetState} $Theme0 $0
 ${If} $0 == ${BST_CHECKED}
  StrCpy $ThemeChoice 0
 ${EndIf}
 ${NSD_GetState} $Theme1 $0
 ${If} $0 == ${BST_CHECKED}
  StrCpy $ThemeChoice 1
 ${EndIf}
 ${NSD_GetState} $Theme2 $0
 ${If} $0 == ${BST_CHECKED}
  StrCpy $ThemeChoice 2
 ${EndIf}
 ${NSD_GetState} $Theme3 $0
 ${If} $0 == ${BST_CHECKED}
  StrCpy $ThemeChoice 3
 ${EndIf}
FunctionEnd
Function .onInit
 ReadRegDWORD $ThemeChoice HKCU "Software\RMP" "Theme"
 ${If} ${Errors}
  StrCpy $ThemeChoice 0
 ${EndIf}
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
 WriteRegDWORD HKCU "Software\RMP" "Theme" $ThemeChoice
 SetOutPath "$INSTDIR"
 File /r "..\build\RMP\*.*"
 WriteUninstaller "$INSTDIR\Uninstall.exe"
 CreateShortcut "$SMPROGRAMS\RMP.lnk" "$INSTDIR\RMP.exe"
 WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\RMP" "DisplayName" "RMP · Rust Media Player"
 WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\RMP" "DisplayVersion" "2.3.2"
 WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\RMP" "UninstallString" '"$INSTDIR\Uninstall.exe"'
SectionEnd
Section "Uninstall"
 !include "uninstall-files.nsh"
 Delete "$INSTDIR\Uninstall.exe"
 RMDir "$INSTDIR"
 Delete "$SMPROGRAMS\RMP.lnk"
 DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\RMP"
SectionEnd
