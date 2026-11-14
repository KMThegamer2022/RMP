param([string]$Player, [string]$Fixture)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type -ReferencedAssemblies System.Drawing.Primitives @'
using System;
using System.Runtime.InteropServices;
public class ThemeTest {
 [DllImport("user32")] public static extern bool SetWindowPos(IntPtr w,IntPtr after,int x,int y,int cx,int cy,uint flags);
 [DllImport("user32")] public static extern bool PrintWindow(IntPtr w,IntPtr dc,uint flags);
 [DllImport("user32")] public static extern bool GetWindowRect(IntPtr w,out System.Drawing.Rectangle rect);
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
  if($id -ne 311){Check-Menu $w $id}
  if ($id -eq 311) {
   [void][ThemeTest]::SetWindowPos($w,[IntPtr](-1),10,10,900,570,0x40)
   Start-Sleep -Milliseconds 700
   $rect=[Drawing.Rectangle]::Empty
   [void][ThemeTest]::GetWindowRect($w,[ref]$rect)
   $bmp=[Drawing.Bitmap]::new($rect.Width-$rect.X,$rect.Height-$rect.Y)
   $g=[Drawing.Graphics]::FromImage($bmp) 
   $g.CopyFromScreen($rect.X,$rect.Y,0,0,$bmp.Size)
   $bmp.Save((Join-Path $pwd 'RMP-Windows7-preview.png'));$g.Dispose();$bmp.Dispose()
  }
  if ((Get-ItemPropertyValue 'HKCU:\Software\RMP' Theme) -ne ($id-310)){throw 'Preference was not saved'}
 }
} finally {$null=$p.CloseMainWindow();if (!$p.WaitForExit(5000)){$p.Kill()}}
$p=Start-Process $Player -PassThru
try {Check-Menu (Get-Window $p) 313} finally {$null=$p.CloseMainWindow();if (!$p.WaitForExit(5000)){$p.Kill()}}
