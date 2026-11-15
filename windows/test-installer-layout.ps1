param([string]$Installer='./RMP-Setup-x64.exe')
$ErrorActionPreference='Stop'
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public static class InstallerUI {
 [StructLayout(LayoutKind.Sequential)] public struct Rect {
  public int Left,Top,Right,Bottom;
  public bool Inside(Rect other){return Left>=other.Left&&Right<=other.Right&&Top>=other.Top&&Bottom<=other.Bottom&&Right>Left&&Bottom>Top;}
  public bool Overlaps(Rect other){return Left<other.Right&&Right>other.Left&&Top<other.Bottom&&Bottom>other.Top;}
 }
 public delegate bool EnumCallback(IntPtr hwnd,IntPtr data);
 [DllImport("user32")] static extern bool EnumChildWindows(IntPtr parent,EnumCallback action,IntPtr data);
 [DllImport("user32",CharSet=CharSet.Unicode)] static extern IntPtr FindWindowEx(IntPtr parent,IntPtr after,string cls,string title);
 [DllImport("user32")] static extern uint GetWindowThreadProcessId(IntPtr h,out uint pid);
 [DllImport("user32")] public static extern bool IsWindowVisible(IntPtr h);
 [DllImport("user32")] static extern int GetWindowLongW(IntPtr h,int index);
 [DllImport("user32",CharSet=CharSet.Unicode)] static extern int GetClassName(IntPtr w,StringBuilder name,int length);
 [DllImport("user32",EntryPoint="SendMessageW",CharSet=CharSet.Unicode)] static extern IntPtr ReadText(IntPtr h,uint msg,IntPtr wp,StringBuilder text);
 [DllImport("user32")] static extern bool GetWindowRect(IntPtr h,out Rect rect);
 [DllImport("user32")] public static extern IntPtr GetParent(IntPtr h);
 [DllImport("user32")] public static extern IntPtr GetDlgItem(IntPtr h,int id);
 [DllImport("user32")] public static extern IntPtr SendMessageW(IntPtr h,uint msg,IntPtr wp,IntPtr lp);
 [DllImport("user32")] public static extern bool PostMessageW(IntPtr h,uint msg,IntPtr wp,IntPtr lp);
 [DllImport("user32")] public static extern bool SetWindowPos(IntPtr h,IntPtr after,int x,int y,int cx,int cy,uint flags);
 public static Rect Bounds(IntPtr h){Rect r;if(!GetWindowRect(h,out r))throw new InvalidOperationException("Could not measure control");return r;}
 public static string Text(IntPtr h){var s=new StringBuilder(512);ReadText(h,13,(IntPtr)512,s);return s.ToString();}
 public static string Class(IntPtr h){var s=new StringBuilder(64);GetClassName(h,s,64);return s.ToString();}
 public static bool IsRadio(IntPtr h){int kind=GetWindowLongW(h,-16)&15;return Class(h)=="Button"&&(kind==4||kind==9);}
 public static IntPtr[] Dialogs(int pid){
  var result=new List<IntPtr>();IntPtr w=IntPtr.Zero;
  while((w=FindWindowEx(IntPtr.Zero,w,"#32770",null))!=IntPtr.Zero){
   uint id;GetWindowThreadProcessId(w,out id);if(id==pid&&IsWindowVisible(w))result.Add(w);
  }
  return result.ToArray();
 }
 public static IntPtr[] Children(IntPtr parent){
  var result=new List<IntPtr>();
  EnumChildWindows(parent,(h,data)=>{if(IsWindowVisible(h))result.Add(h);return true;},IntPtr.Zero);
  return result.ToArray();
 }
}
'@
$expected=@('VLC','RMP Default','Windows 10/11','Windows 7','Windows XP','Mac','Linux')
$p=Start-Process -FilePath $Installer -PassThru
try {
 $timer=[Diagnostics.Stopwatch]::StartNew()
 do {
  $dialog=[IntPtr]::Zero
  foreach($candidate in [InstallerUI]::Dialogs($p.Id)){
   $children=[InstallerUI]::Children($candidate)
   $radios=@($children|Where-Object {[InstallerUI]::IsRadio($_)})
   if($radios.Count -eq 7){$dialog=$candidate;break}
  }
  if($p.HasExited -or $timer.ElapsedMilliseconds -gt 15000){throw 'The graphical installer did not display seven theme choices'}
  if(!$dialog.ToInt64()){Start-Sleep -Milliseconds 100}
 }until($dialog.ToInt64())
 $labels=@($children|Where-Object {[InstallerUI]::Class($_) -eq 'Static' -and [InstallerUI]::Text($_) -like '*Theme menu*'})
 if($labels.Count -ne 1){throw 'Theme instructions are missing'}
 $label=[InstallerUI]::Bounds($labels[0])
 $page=[InstallerUI]::GetParent($labels[0])
 $pageBounds=[InstallerUI]::Bounds($page)
 if(!$label.Inside($pageBounds)){throw 'Theme instructions are clipped by the installer page'}
 $names=@($radios|ForEach-Object {[InstallerUI]::Text($_)})
 if(Compare-Object $names $expected){throw "Unexpected theme choices: $names"}
 for($i=0;$i -lt $radios.Count;$i++){
  $r=[InstallerUI]::Bounds($radios[$i])
  if([InstallerUI]::GetParent($radios[$i]) -ne $page -or !$r.Inside($pageBounds)){throw "$($names[$i]) extends outside the visible page"}
  if($r.Overlaps($label)){throw "$($names[$i]) overlaps the instructions"}
  for($j=0;$j -lt $i;$j++){
   if($r.Overlaps([InstallerUI]::Bounds($radios[$j]))){throw "$($names[$i]) overlaps $($names[$j])"}
  }
 }
 # Capture the actual interactive installer before making a temporary selection.
 [void][InstallerUI]::SetWindowPos($dialog,[IntPtr](-1),0,0,0,0,0x43)
 Start-Sleep -Milliseconds 350
 $r=[InstallerUI]::Bounds($dialog)
 $img=[Drawing.Bitmap]::new($r.Right-$r.Left,$r.Bottom-$r.Top)
 $g=[Drawing.Graphics]::FromImage($img)
 try {$g.CopyFromScreen($r.Left,$r.Top,0,0,$img.Size);$img.Save((Join-Path $pwd 'RMP-Installer-preview.png'),[Drawing.Imaging.ImageFormat]::Png)}
 finally {$g.Dispose();$img.Dispose()}
 foreach($radio in $radios){
  [void][InstallerUI]::SendMessageW($radio,0xF5,[IntPtr]::Zero,[IntPtr]::Zero)
  $checked=@($radios|Where-Object {[InstallerUI]::SendMessageW($_,0xF0,[IntPtr]::Zero,[IntPtr]::Zero).ToInt32() -eq 1})
  if($checked.Count -ne 1 -or $checked[0] -ne $radio){throw "Theme choice did not select exclusively: $([InstallerUI]::Text($radio))"}
 }
 [void][InstallerUI]::PostMessageW($dialog,16,[IntPtr]::Zero,[IntPtr]::Zero)
 $timer.Restart()
 while(!$p.HasExited -and $timer.ElapsedMilliseconds -lt 10000){
  foreach($prompt in [InstallerUI]::Dialogs($p.Id)){
   $yes=[InstallerUI]::GetDlgItem($prompt,6)
   if($prompt -ne $dialog -and $yes.ToInt64()){[void][InstallerUI]::PostMessageW($yes,0xF5,[IntPtr]::Zero,[IntPtr]::Zero)}
  }
  Start-Sleep -Milliseconds 100
 }
 if(!$p.HasExited){throw 'Could not dismiss the installer without installing'}
 Write-Host 'Verified seven visible, selectable, nonoverlapping themes on the native installer page.'
}finally{if(!$p.HasExited){$p.Kill();$p.WaitForExit()}}
