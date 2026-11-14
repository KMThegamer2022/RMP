param([string]$Player,[string]$Fixture)
$ErrorActionPreference='Stop'
Add-Type @'
using System;
using System.Runtime.InteropServices;
using System.Text;
public class FileMenuTest {
 [DllImport("user32")] public static extern bool PostMessage(IntPtr h,uint msg,IntPtr wp,IntPtr lp);
 [DllImport("user32",CharSet=CharSet.Unicode)] public static extern IntPtr FindWindow(string cls,string name);
 [DllImport("user32",CharSet=CharSet.Unicode)] public static extern IntPtr FindWindowEx(IntPtr parent,IntPtr after,string cls,string name);
 [DllImport("user32")] public static extern IntPtr GetDlgItem(IntPtr h,int id);
 [DllImport("user32")] public static extern IntPtr SendMessage(IntPtr h,uint msg,IntPtr w,IntPtr l);
 [DllImport("user32",CharSet=CharSet.Unicode)] public static extern IntPtr SendMessage(IntPtr h,uint msg,IntPtr w,string text);
 [DllImport("user32")] public static extern uint GetWindowThreadProcessId(IntPtr h,out uint pid);
 public static IntPtr Dialog(int pid) {
  IntPtr h=IntPtr.Zero;
  while((h=FindWindowEx(IntPtr.Zero,h,"#32770",null))!=IntPtr.Zero){
   uint id;GetWindowThreadProcessId(h,out id);if(id==pid)return h;
  }return IntPtr.Zero;
 }
}
'@
$p=Start-Process $Player -PassThru
try {
 $clock=[Diagnostics.Stopwatch]::StartNew()
 do {$p.Refresh();$w=$p.MainWindowHandle;if($clock.ElapsedMilliseconds -gt 15000){throw 'No player window'};Start-Sleep -Milliseconds 25}until($w.ToInt64())
 [void][FileMenuTest]::PostMessage($w,273,[IntPtr]101,[IntPtr]::Zero)
 do {$dlg=[FileMenuTest]::Dialog($p.Id);if($clock.ElapsedMilliseconds -gt 20000){throw 'File picker did not open'};Start-Sleep -Milliseconds 25}until($dlg.ToInt64())
 $edit=[FileMenuTest]::GetDlgItem($dlg,0x480)
 if(!$edit.ToInt64()){
  $combo=[FileMenuTest]::GetDlgItem($dlg,0x47c)
  $inner=[FileMenuTest]::FindWindowEx($combo,[IntPtr]::Zero,'ComboBox',$null)
  if($inner.ToInt64()){$combo=$inner}
  $edit=[FileMenuTest]::FindWindowEx($combo,[IntPtr]::Zero,'Edit',$null)
 }
 if(!$edit.ToInt64()){throw 'Missing file name edit'}
 [void][FileMenuTest]::SendMessage($edit,12,[IntPtr]::Zero,(Resolve-Path $Fixture).Path)
 Start-Sleep -Milliseconds 800
 $clock.Restart()
 [void][FileMenuTest]::PostMessage($dlg,273,[IntPtr]1,[FileMenuTest]::GetDlgItem($dlg,1))
 $seek=[FileMenuTest]::GetDlgItem($w,107)
 do {
  $pos=[FileMenuTest]::SendMessage($seek,0x400,[IntPtr]::Zero,[IntPtr]::Zero)
  if($clock.ElapsedMilliseconds -gt 15000){throw 'Selected video did not advance'}
  if(!$pos.ToInt64()){Start-Sleep -Milliseconds 10}
 }until($pos.ToInt64())
 Write-Host "FILE_PICKER_TO_PLAYBACK_MS $Player $($clock.ElapsedMilliseconds)"
}finally{if(!$p.HasExited){[void]$p.CloseMainWindow();if(!$p.WaitForExit(5000)){$p.Kill()}}}
