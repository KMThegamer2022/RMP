param([string]$Player, [string]$Fixture)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type -ReferencedAssemblies System.Drawing.Primitives @'
using System;
using System.Runtime.InteropServices;
public class ThemeTest {
 [DllImport("user32")] public static extern void mouse_event(uint flags,int dx,int dy,uint data,UIntPtr extra);
 public static void DoubleClick(int x,int y){
  SetCursorPos(x,y);
  for(int i=0;i<2;i++){
   mouse_event(2,0,0,0,UIntPtr.Zero);mouse_event(4,0,0,0,UIntPtr.Zero);
   System.Threading.Thread.Sleep(60);
  }
 }

 [DllImport("user32")] public static extern IntPtr GetDlgItem(IntPtr w,int id);
 [DllImport("user32",CharSet=CharSet.Unicode)] public static extern IntPtr FindWindow(string cls,string title);
 [DllImport("user32")] public static extern bool IsWindowVisible(IntPtr w);
 [DllImport("user32")] public static extern bool SetCursorPos(int x,int y);
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
 Check-Menu $w 315
 foreach ($id in 310..315) {
  [void][ThemeTest]::SendMessage($w,273,[IntPtr]$id,[IntPtr]::Zero)
  Check-Menu $w $id
  if ($id -eq 311 -or $id -eq 315) {
   [void][ThemeTest]::SetWindowPos($w,[IntPtr](-1),10,10,900,570,0x40)
   Start-Sleep -Milliseconds 700
   $rect=[Drawing.Rectangle]::Empty
   [void][ThemeTest]::GetWindowRect($w,[ref]$rect)
   $bmp=[Drawing.Bitmap]::new($rect.Width-$rect.X,$rect.Height-$rect.Y)
   $g=[Drawing.Graphics]::FromImage($bmp) 
   $g.CopyFromScreen($rect.X,$rect.Y,0,0,$bmp.Size)
   $file=if($id -eq 311){'RMP-Windows7-preview.png'}else{'RMP-VLC-preview.png'}
   $bmp.Save((Join-Path $pwd $file));$g.Dispose();$bmp.Dispose()
  }
  if ((Get-ItemPropertyValue 'HKCU:\Software\RMP' Theme) -ne ($id-310)){throw 'Preference was not saved'}
 }
 $button=[ThemeTest]::GetDlgItem($w,102)
 $oldFont=[ThemeTest]::SendMessage($button,49,[IntPtr]::Zero,[IntPtr]::Zero)
 [void][ThemeTest]::SetCursorPos(10,10)
 [ThemeTest]::DoubleClick(220,160);Start-Sleep -Milliseconds 350
 $deck=[ThemeTest]::FindWindow('RMPTransport','Playback controls')
 if(!$deck.ToInt64() -or ![ThemeTest]::IsWindowVisible($deck)){throw 'Fullscreen controller did not appear'}
 $font=[ThemeTest]::SendMessage($button,49,[IntPtr]::Zero,[IntPtr]::Zero)
 if($font -eq [IntPtr]::Zero -or $font -eq $oldFont){throw "Fullscreen control retained a discarded font"}
 Start-Sleep -Seconds 4
 if([ThemeTest]::IsWindowVisible($deck)){throw 'Fullscreen controller did not hide'}
 [void][ThemeTest]::SetCursorPos(55,55);Start-Sleep -Milliseconds 500
 if(![ThemeTest]::IsWindowVisible($deck)){throw 'Fullscreen controller did not reappear'}
 [ThemeTest]::DoubleClick(220,160);Start-Sleep -Milliseconds 350
 if([ThemeTest]::IsWindowVisible($deck)){throw "Video double-click did not exit fullscreen"}
} finally {$null=$p.CloseMainWindow();if (!$p.WaitForExit(5000)){$p.Kill()}}
$p=Start-Process $Player -PassThru
try {
 $w=Get-Window $p;Check-Menu $w 315
 [void][ThemeTest]::SetWindowPos($w,[IntPtr](-1),10,10,900,570,0x40)
 Start-Sleep -Milliseconds 700
 $rect=[Drawing.Rectangle]::Empty;[void][ThemeTest]::GetWindowRect($w,[ref]$rect)
 $bmp=[Drawing.Bitmap]::new($rect.Width-$rect.X,$rect.Height-$rect.Y)
 $g=[Drawing.Graphics]::FromImage($bmp);$g.CopyFromScreen($rect.X,$rect.Y,0,0,$bmp.Size)
 $bmp.Save((Join-Path $pwd 'RMP-VLC-preview.png'));$g.Dispose();$bmp.Dispose()
} finally {$null=$p.CloseMainWindow();if (!$p.WaitForExit(5000)){$p.Kill()}}
