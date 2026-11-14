param([string]$Player)
$ErrorActionPreference='Stop'
Add-Type @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public class UpdateCheck {
 [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr w,uint m,IntPtr a,IntPtr b);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern IntPtr FindWindow(string cls,string title);
 [DllImport("user32.dll")] public static extern IntPtr GetDlgItem(IntPtr w,int id);
 [DllImport("user32.dll",EntryPoint="SendMessageW",CharSet=CharSet.Unicode)] public static extern IntPtr ReadText(IntPtr w,uint msg,IntPtr capacity,StringBuilder text);
}
'@
$p=Start-Process $Player -PassThru
try {
 $p.WaitForInputIdle(10000) | Out-Null
 $p.Refresh()
 [void][UpdateCheck]::SendMessage($p.MainWindowHandle,273,[IntPtr]490,[IntPtr]::Zero)
 for($i=0;$i -lt 200;$i++){
  $dialog=[UpdateCheck]::FindWindow('#32770','RMP update')
  if($dialog -ne [IntPtr]::Zero){break}
  Start-Sleep -Milliseconds 100
 }
 if($dialog -eq [IntPtr]::Zero){throw 'Manual check did not display a result'}
 $s=[Text.StringBuilder]::new(2000)
 [void][UpdateCheck]::ReadText([UpdateCheck]::GetDlgItem($dialog,65535),13,[IntPtr]$s.Capacity,$s)
 if($s.ToString() -notmatch 'up to date|available'){throw "Unexpected result: $s"}
 Write-Host $s.ToString()
 [void][UpdateCheck]::SendMessage($dialog,16,[IntPtr]::Zero,[IntPtr]::Zero)
}finally{$null=$p.CloseMainWindow();if(!$p.WaitForExit(5000)){$p.Kill()}}
