# Drives the egui RMP through its script hook and real mouse/keyboard input, then checks the state log.
param([string]$Exe, [string]$Fixture, [string]$OutDir)
$ErrorActionPreference = 'Stop'
$Exe = (Resolve-Path $Exe).Path
$null = New-Item -ItemType Directory -Force $OutDir
$OutDir = (Resolve-Path $OutDir).Path
Add-Type -Language CSharp @'
using System; using System.Runtime.InteropServices;
public class In {
 [DllImport("user32")] public static extern void mouse_event(uint f,int dx,int dy,uint d,UIntPtr e);
 [DllImport("user32")] public static extern void keybd_event(byte vk,byte scan,uint f,UIntPtr e);
 [DllImport("user32")] public static extern bool SetCursorPos(int x,int y);
 [DllImport("user32")] public static extern bool SetForegroundWindow(IntPtr w);
 [DllImport("user32")] public static extern bool GetWindowRect(IntPtr w,out RECT r);
 [DllImport("user32",CharSet=CharSet.Unicode)] public static extern IntPtr FindWindow(string cls,string title);
 [DllImport("user32")] public static extern IntPtr SendMessage(IntPtr w,uint m,IntPtr a,IntPtr b);
 [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L,T,R,B; }
 [StructLayout(LayoutKind.Explicit,Size=40)] public struct INPUT { [FieldOffset(0)] public uint type; [FieldOffset(8)] public ushort vk; [FieldOffset(10)] public ushort scan; [FieldOffset(12)] public uint flags; [FieldOffset(16)] public uint time; [FieldOffset(24)] public IntPtr extra; }
 [DllImport("user32",SetLastError=true)] public static extern uint SendInput(uint n,ref INPUT i,int size);
 public static void Key(byte vk){
  INPUT i = new INPUT(); i.type=1; i.vk=vk; i.scan=(ushort)MapVirtualKey(vk,0);
  SendInput(1,ref i,40); System.Threading.Thread.Sleep(60);
  i.flags=2; SendInput(1,ref i,40); }
 [DllImport("user32")] public static extern uint MapVirtualKey(uint code,uint type);
 public static void Click(int x,int y,int n){ SetCursorPos(x,y); for(int i=0;i<n;i++){ mouse_event(2,0,0,0,UIntPtr.Zero); mouse_event(4,0,0,0,UIntPtr.Zero); System.Threading.Thread.Sleep(60);} }
}
'@
function Wait-Label($log, $label, $seconds = 40) {
  $limit = (Get-Date).AddSeconds($seconds)
  while ((Get-Date) -lt $limit) {
    if ((Test-Path $log) -and (Select-String -Path $log -Pattern "^$label " -Quiet)) { return }
    Start-Sleep -Milliseconds 200
  }
  throw "Timed out waiting for '$label' in the state log"
}
function Field($log, $label, $field) {
  $line = (Select-String -Path $log -Pattern "^$label " | Select-Object -First 1).Line
  if ($line -match "$field=(\S+)") { return $Matches[1] }
  throw "No $field in '$label'"
}
function Expect($actual, $expected, $what) {
  if ($actual -ne $expected) { throw "$what`: expected $expected, got $actual" }
  Write-Host "OK  $what = $actual"
}

# --- Playback, keyboard and mouse ---------------------------------------------------------------
$log = Join-Path $OutDir 'state.log'
Remove-Item $log -ErrorAction SilentlyContinue
$script = Join-Path $OutDir 'drive.txt'
@"
wait 3
dump a
wait 5
dump b
wait 5
dump c
wait 5
dump d
shotall $(Join-Path $OutDir 'screen.png')
quit
"@ | Set-Content $script
$env:RMP_SPIKE_SCRIPT = $script; $env:RMP_SPIKE_LOG = $log; $env:RMP_SPIKE_NO_UPDATE_CHECK = '1'; $env:RMP_SPIKE_THEME = 'vlc'
$p = Start-Process $Exe -ArgumentList "`"$Fixture`"" -PassThru
$window = [IntPtr]::Zero
for ($i = 0; $i -lt 200 -and $window -eq [IntPtr]::Zero; $i++) { $p.Refresh(); $window = $p.MainWindowHandle; Start-Sleep -Milliseconds 100 }
if ($window -eq [IntPtr]::Zero) { throw 'RMP did not open a window' }
Wait-Label $log 'a'
Expect (Field $log 'a' 'state') '3' 'playing after launch'
Expect (Field $log 'a' 'video') 'true' 'video output present'
Expect (Field $log 'a' 'full') 'false' 'windowed at start'
# Double-click on the screen enters fullscreen.
# Click the middle of the video rectangle the app reports (screen coordinates).
$line = (Select-String -Path $log -Pattern '^a ' | Select-Object -First 1).Line
if ($line -notmatch 'rect=(-?\d+),(-?\d+),(-?\d+),(-?\d+)') { throw 'No video rectangle in the state dump' }
$cx = [int](([int]$Matches[1] + [int]$Matches[3]) / 2); $cy = [int](([int]$Matches[2] + [int]$Matches[4]) / 2)
Write-Host "Double-clicking the video at $cx,$cy"
[In]::Click($cx, $cy, 2)
Wait-Label $log 'b'
Write-Host ((Select-String -Path $log -Pattern '^b ' | Select-Object -First 1).Line -replace '^.*input=','input=')
$soft = @()
try { Expect (Field $log 'b' 'full') 'true' 'double-click enters fullscreen' } catch { $soft += $_.Exception.Message; Write-Host "FAIL $($_.Exception.Message)" }
# Escape leaves fullscreen.
[void][In]::SetForegroundWindow($window)
[In]::Key(0x1B)
Wait-Label $log 'c'
Write-Host ((Select-String -Path $log -Pattern '^c ' | Select-Object -First 1).Line -replace '^.*input=','input=')
try { Expect (Field $log 'c' 'full') 'false' 'Escape leaves fullscreen' } catch { $soft += $_.Exception.Message; Write-Host "FAIL $($_.Exception.Message)" }
# Space pauses.
[void][In]::SetForegroundWindow($window)
[In]::Key(0x20)
Wait-Label $log 'd'
Write-Host ((Select-String -Path $log -Pattern '^d ' | Select-Object -First 1).Line -replace '^.*input=','input=')
try { Expect (Field $log 'd' 'state') '4' 'Space pauses' } catch { $soft += $_.Exception.Message; Write-Host "FAIL $($_.Exception.Message)" }
if ($soft) { $script:softFailures = $soft }
if (!$p.WaitForExit(20000)) { $p.Kill(); throw 'RMP did not quit from the script' }
Copy-Item $log (Join-Path $OutDir 'state-final.log')

# --- Every skin, with a screenshot of each ---------------------------------------------------
foreach ($skin in 'vlc', 'rmp-default', 'win10', 'win7', 'xp', 'mac', 'linux') {
  $log = Join-Path $OutDir "skin-$skin.log"
  Remove-Item $log -ErrorAction SilentlyContinue
  $env:RMP_SPIKE_LOG = $log; $env:RMP_SPIKE_THEME = $skin
  @"
wait 5
dump skin
shot $(Join-Path $OutDir "skin-$skin.png")
quit
"@ | Set-Content $script
  $p = Start-Process $Exe -ArgumentList "`"$Fixture`"" -PassThru
  Wait-Label $log 'skin' 30
  Expect (Field $log 'skin' 'skin') $skin "skin $skin applied"
  Expect (Field $log 'skin' 'state') '3' "skin $skin playing"
  if (!$p.WaitForExit(20000)) { $p.Kill() }
}
# --- Check for Updates (update.ps1 shows its own dialog) ----------------------------------------
$env:RMP_SPIKE_LOG = Join-Path $OutDir 'update.log'; $env:RMP_SPIKE_THEME = 'vlc'
@"
wait 3
act updates
wait 25
quit
"@ | Set-Content $script
$p = Start-Process $Exe -ArgumentList "`"$Fixture`"" -PassThru
$dialog = [IntPtr]::Zero
for ($i = 0; $i -lt 300 -and $dialog -eq [IntPtr]::Zero; $i++) { $dialog = [In]::FindWindow('#32770', 'RMP update'); Start-Sleep -Milliseconds 100 }
if ($dialog -eq [IntPtr]::Zero) { throw 'Check for Updates did not show a result dialog' }
Write-Host 'OK  Check for Updates shows the update dialog'
[void][In]::SendMessage($dialog, 16, [IntPtr]::Zero, [IntPtr]::Zero)
if (!$p.WaitForExit(40000)) { $p.Kill() }
if ($script:softFailures) { throw ('Failed: ' + ($script:softFailures -join '; ')) }
Write-Host 'All egui checks passed' 
