# Builds RMP-egui-Setup-x64.exe from build/RMP using the shipping installer script with another output name.
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path build/RMP).Path
$entries = Get-ChildItem $root -Recurse | Sort-Object { $_.FullName.Length } -Descending | ForEach-Object {
  $verb = if ($_.PSIsContainer) { 'RMDir' } else { 'Delete' }
  '{0} "$INSTDIR\{1}"' -f $verb, $_.FullName.Substring($root.Length + 1)
}
$entries | Set-Content windows/uninstall-files.nsh
(Get-Content windows/setup.nsi -Raw).Replace('OutFile "../RMP-Setup-x64.exe"', 'OutFile "../RMP-egui-Setup-x64.exe"') | Set-Content windows/setup-egui.nsi
& "${env:ProgramFiles(x86)}\NSIS\makensis.exe" windows/setup-egui.nsi
if ($LASTEXITCODE) { throw 'Installer build failed' }
Remove-Item windows/setup-egui.nsi, windows/uninstall-files.nsh
