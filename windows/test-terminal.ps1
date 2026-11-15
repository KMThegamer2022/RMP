param([string]$Player,[string]$Fixture)
$ErrorActionPreference='Stop'
Add-Type @'
using System;
using System.Runtime.InteropServices;
using System.Text;
public class TerminalTest {
 [DllImport("user32",CharSet=CharSet.Unicode)] public static extern IntPtr FindWindowEx(IntPtr p,IntPtr after,string cls,string title);
 [DllImport("user32")] public static extern uint GetWindowThreadProcessId(IntPtr h,out uint pid);
 [DllImport("user32")] public static extern IntPtr GetDlgItem(IntPtr h,int id);
 [DllImport("user32")] public static extern bool IsWindowVisible(IntPtr h);
 [DllImport("user32")] public static extern IntPtr GetMenu(IntPtr h);
 [DllImport("user32",EntryPoint="SendMessageW")] public static extern IntPtr Send(IntPtr h,uint m,IntPtr w,IntPtr l);
 [DllImport("user32",EntryPoint="SendMessageW",CharSet=CharSet.Unicode)] static extern IntPtr Put(IntPtr h,uint m,IntPtr w,string t);
 [DllImport("user32",EntryPoint="SendMessageW",CharSet=CharSet.Unicode)] static extern IntPtr Read(IntPtr h,uint m,IntPtr w,StringBuilder t);
 public static IntPtr Window(string cls,int pid){
  IntPtr h=IntPtr.Zero;
  while((h=FindWindowEx(IntPtr.Zero,h,cls,null))!=IntPtr.Zero){
   uint id;GetWindowThreadProcessId(h,out id);if(id==pid)return h;
  }
  return IntPtr.Zero;
 }
 public static string Text(IntPtr h){
  int n=Send(h,14,IntPtr.Zero,IntPtr.Zero).ToInt32();var s=new StringBuilder(n+1);
  Read(h,13,(IntPtr)(n+1),s);return s.ToString();
 }
 public static void Set(IntPtr h,string text){Put(h,12,IntPtr.Zero,text);}
 public static void Key(IntPtr h,int key){Send(h,256,(IntPtr)key,IntPtr.Zero);}
 public static void Command(IntPtr h,string line){Set(h,line);Key(h,13);}
 public static void Menu(IntPtr h,int id){Send(h,273,(IntPtr)id,IntPtr.Zero);}
}
'@
function Wait-Until([scriptblock]$test,[string]$reason){
 $timer=[Diagnostics.Stopwatch]::StartNew()
 while(!(& $test)){
  if($timer.ElapsedMilliseconds -gt 15000){throw $reason}
  Start-Sleep -Milliseconds 60
 }
}
$settings='HKCU:\Software\RMP'
$previous=Get-ItemPropertyValue $settings Theme -ErrorAction SilentlyContinue
$fixturePath=(Resolve-Path $Fixture).Path
$p=$null
try {
 $p=Start-Process $Player -PassThru
 Wait-Until {([TerminalTest]::Window('RMPPlayer',$p.Id)).ToInt64()} 'Main window missing'
 $w=[TerminalTest]::Window('RMPPlayer',$p.Id)
 [TerminalTest]::Menu($w,316)
 Wait-Until {[TerminalTest]::IsWindowVisible([TerminalTest]::Window('RMPTerminal',$p.Id))} 'Terminal did not open'
 $term=[TerminalTest]::Window('RMPTerminal',$p.Id)
 $output=[TerminalTest]::GetDlgItem($term,621)
 $prompt=[TerminalTest]::GetDlgItem($term,622)
 $edit=[TerminalTest]::GetDlgItem($term,623)
 if([TerminalTest]::Text($output) -notmatch 'Type help' -or [TerminalTest]::Text($prompt) -notmatch '^rmp:.*\$$'){throw 'Terminal prompt or help hint missing'}
 $seek=[TerminalTest]::GetDlgItem($w,107)
 if([TerminalTest]::IsWindowVisible($seek)){throw 'Transport controls visible over Linux video'}
 $surface=[TerminalTest]::FindWindowEx($w,[IntPtr]::Zero,'Static',$null)
 # The video surface is the static with a nested logo child; labels are siblings.
 while($surface.ToInt64()){
  $logo=[TerminalTest]::FindWindowEx($surface,[IntPtr]::Zero,'Static',$null)
  if($logo.ToInt64()){break}
  $surface=[TerminalTest]::FindWindowEx($w,$surface,'Static',$null)
 }
 if(!$surface.ToInt64() -or ![TerminalTest]::IsWindowVisible($surface) -or [TerminalTest]::IsWindowVisible($logo)){throw 'Idle video surface is not blank'}
 [TerminalTest]::Set($edit,'vol');[TerminalTest]::Key($edit,9)
 if([TerminalTest]::Text($edit) -ne 'volume '){throw 'Tab completion failed'}
 [TerminalTest]::Command($edit,'volume 42');[TerminalTest]::Key($edit,38)
 if([TerminalTest]::Text($edit) -ne 'volume 42' -or [TerminalTest]::Text($output) -notmatch 'Volume 42%'){throw 'Volume or command history failed'}
 $level=[TerminalTest]::GetDlgItem($w,108)
 if([TerminalTest]::Send($level,0x400,[IntPtr]::Zero,[IntPtr]::Zero).ToInt32() -ne 42){throw 'Terminal volume did not reach the player'}
 [TerminalTest]::Key($edit,40)
 if([TerminalTest]::Text($edit)){throw 'Down history did not return to draft'}
 [TerminalTest]::Command($edit,'help')
 if([TerminalTest]::Text($output) -notmatch 'seek'){throw 'Missing playback help'}
 $sentinel=Join-Path $env:TEMP ('RMP-terminal-test-'+[guid]::NewGuid()+'.txt')
 [TerminalTest]::Command($edit,('echo unsafe > "'+$sentinel+'"'))
 if((Test-Path $sentinel) -or [TerminalTest]::Text($output) -notmatch 'Unknown command'){throw 'Unknown command was not safely rejected'}
 [TerminalTest]::Command($edit,('play "'+$fixturePath+'"'))
 Wait-Until {[TerminalTest]::Text($w) -eq [IO.Path]::GetFileName($fixturePath) -and [TerminalTest]::Send($seek,0x400,[IntPtr]::Zero,[IntPtr]::Zero).ToInt32() -gt 0} 'Terminal play did not advance video'
 [TerminalTest]::Command($edit,'fullscreen')
 Wait-Until {![TerminalTest]::GetMenu($w).ToInt64()} 'Fullscreen did not enter'
 [TerminalTest]::Command($edit,'fullscreen')
 Wait-Until {[TerminalTest]::GetMenu($w).ToInt64()} 'Fullscreen did not exit'
 [void][TerminalTest]::Send($term,16,[IntPtr]::Zero,[IntPtr]::Zero)
 if([TerminalTest]::IsWindowVisible($term) -or $p.HasExited){throw 'Closing the terminal closed the player'}
 [TerminalTest]::Menu($w,620)
 Wait-Until {[TerminalTest]::IsWindowVisible($term)} 'View menu did not reopen terminal'
 [TerminalTest]::Command($edit,'clear')
 if([TerminalTest]::Text($output)){throw 'Clear did not empty transcript'}
 [TerminalTest]::Menu($w,315)
 if([TerminalTest]::IsWindowVisible($term) -or ![TerminalTest]::IsWindowVisible($seek)){throw 'VLC theme did not restore its controls'}
 [TerminalTest]::Menu($w,316)
 [void][TerminalTest]::Send($w,16,[IntPtr]::Zero,[IntPtr]::Zero)
 if(!$p.WaitForExit(5000)){throw 'Closing the video window did not quit both windows'}
 $p=Start-Process $Player -PassThru
 Wait-Until {[TerminalTest]::IsWindowVisible([TerminalTest]::Window('RMPTerminal',$p.Id))} 'Saved Linux theme did not reopen the terminal'
 Write-Host 'Verified Linux terminal, command completion and history, isolated input, real video playback, fullscreen and window restoration.'
}finally{
 if($p -and !$p.HasExited){$w=[TerminalTest]::Window('RMPPlayer',$p.Id);[void][TerminalTest]::Send($w,16,[IntPtr]::Zero,[IntPtr]::Zero);if(!$p.WaitForExit(5000)){$p.Kill()}}
 if($null -eq $previous){Remove-ItemProperty $settings Theme -ErrorAction SilentlyContinue}else{Set-ItemProperty $settings Theme $previous}
}
