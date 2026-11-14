param([string]$Player, [string]$Fixture)
$ErrorActionPreference = 'Stop'
Add-Type @'
using System;
using System.Runtime.InteropServices;
public class ThemeTest {
 [DllImport("user32")] public static extern IntPtr SendMessage(IntPtr w,uint msg,IntPtr wp,IntPtr lp);
 [DllImport("user32")] public static extern IntPtr GetMenu(IntPtr w);
 [DllImport("user32")] public static extern IntPtr GetSubMenu(IntPtr m,int index);
 [DllImport("user32")] public static extern int GetMenuItemCount(IntPtr m);
 [DllImport("user32")] public static extern uint GetMenuState(IntPtr m,uint item,uint flags);
}
'@
function Get-Window($p) {
 for ($i=0;$i -lt 150;$i++) { $p.Refresh();if ($p.MainWindowHandle.ToInt64() -ne 0){return $p.MainWindowHandle};Start-Sleep -Milliseconds 50 }
 throw 'Window did not open'
}
function Check-Menu($w,$id) {
 $bar=[ThemeTest]::GetMenu($w)
 $menu=[ThemeTest]::GetSubMenu($bar,([ThemeTest]::GetMenuItemCount($bar)-1))
 $state=[ThemeTest]::GetMenuState($menu,$id,0)
 if ($state -eq [uint32]::MaxValue -or !($state -band 8)){throw "Missing checkmark for theme $id"}
}
$p=Start-Process $Player -ArgumentList $Fixture -PassThru
try {
 $w=Get-Window $p
 foreach ($id in 310..313) {
  [void][ThemeTest]::SendMessage($w,273,[IntPtr]$id,[IntPtr]::Zero)
  Check-Menu $w $id
  if ((Get-ItemPropertyValue 'HKCU:\Software\RMP' Theme) -ne ($id-310)){throw 'Preference was not saved'}
 }
} finally {$null=$p.CloseMainWindow();if (!$p.WaitForExit(5000)){$p.Kill()}}
$p=Start-Process $Player -PassThru
try {Check-Menu (Get-Window $p) 313} finally {$null=$p.CloseMainWindow();if (!$p.WaitForExit(5000)){$p.Kill()}}
