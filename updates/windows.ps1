param([int]$PlayerId,[string]$CurrentVersion,[string]$PlayerPath,[switch]$Manual)
$ErrorActionPreference='Stop'
Add-Type -AssemblyName System.Windows.Forms

function Notice($message){[void][Windows.Forms.MessageBox]::Show($message,'RMP update','OK','Warning')}
$client=New-Object Net.WebClient
$client.Headers['User-Agent']='RMP-Updater'
$client.Headers['Accept']='application/vnd.github+json'
[Net.ServicePointManager]::SecurityProtocol=[Net.SecurityProtocolType]3072
$repo='https://github.com/KMThegamer2022/RMP/releases/download/'
try {
 $json=$client.DownloadString('https://api.github.com/repos/KMThegamer2022/RMP/releases?per_page=100')
 if(Get-Command ConvertFrom-Json -ErrorAction SilentlyContinue){$releases=ConvertFrom-Json -InputObject $json}
 else {Add-Type -AssemblyName System.Web.Extensions;$parser=New-Object Web.Script.Serialization.JavaScriptSerializer;$parser.MaxJsonLength=16777216;$releases=$parser.DeserializeObject($json)}
 $best=[Version]$CurrentVersion
 foreach($r in $releases){
  if($r.draft -or $r.prerelease -or $r.tag_name -notmatch '^v(\d+\.\d+\.\d+)(?:-windows)?$'){continue}
  $version=[Version](($r.tag_name -replace '^v','') -replace '-windows$','')
  if($version -le $best){continue}
  foreach($a in $r.assets){
   if($a.name -eq 'RMP-Setup-x64.exe' -and $a.browser_download_url -ceq ($repo+$r.tag_name+'/'+$a.name) -and $a.digest -match '^sha256:[0-9a-f]{64}$'){
    $best=$version;$release=$r;$asset=$a;break
   }
  }
 }
 if(!$asset){if($Manual){[void][Windows.Forms.MessageBox]::Show("RMP $CurrentVersion is up to date.",'RMP update','OK','Information')};return}
 $player=Get-Process -Id $PlayerId
 if($player.MainModule.FileName -ne $PlayerPath){throw 'The running player could not be identified.'}
} catch {if($Manual){Notice ("Could not check for updates. "+$_.Exception.Message)};return}
if([Windows.Forms.MessageBox]::Show(("RMP "+$best+" is available. Update now?"+[Environment]::NewLine+[Environment]::NewLine+$release.body),'RMP update','YesNo','Information') -ne 'Yes'){return}
$temp=Join-Path ([IO.Path]::GetTempPath()) ([Guid]::NewGuid().ToString())
$null=[IO.Directory]::CreateDirectory($temp)
$setup=Join-Path $temp 'RMP-Setup-x64.exe'
$form=New-Object Windows.Forms.Form
$form.Text='Updating RMP';$form.ClientSize=New-Object Drawing.Size(440,130);$form.FormBorderStyle='FixedDialog';$form.MaximizeBox=$false;$form.MinimizeBox=$false;$form.StartPosition='CenterScreen'
$label=New-Object Windows.Forms.Label;$label.Location=New-Object Drawing.Point(16,14);$label.Size=New-Object Drawing.Size(400,20);$label.Text="Downloading RMP $best..."
$bar=New-Object Windows.Forms.ProgressBar;$bar.Location=New-Object Drawing.Point(16,44);$bar.Size=New-Object Drawing.Size(400,22);$bar.Minimum=0;$bar.Maximum=100
$cancel=New-Object Windows.Forms.Button;$cancel.Text='Cancel';$cancel.Location=New-Object Drawing.Point(16,84);$cancel.Size=New-Object Drawing.Size(100,30)
$restart=New-Object Windows.Forms.Button;$restart.Text='Restart Now';$restart.Location=New-Object Drawing.Point(324,84);$restart.Size=New-Object Drawing.Size(100,30);$restart.Visible=$false
$form.Controls.Add($label);$form.Controls.Add($bar);$form.Controls.Add($cancel);$form.Controls.Add($restart)
$state=@{phase='download';error=$null;cancelled=$false;restart=$false;count=5}
$cancel.Add_Click({$state.cancelled=$true;if($state.phase -eq 'download'){$client.CancelAsync()}})
$restart.Add_Click({$state.restart=$true})
$client.add_DownloadProgressChanged({param($sender,$e)$bar.Value=[Math]::Min(100,[int]$e.ProgressPercentage);$label.Text="Downloading RMP $best... "+$e.ProgressPercentage+"%"})
$client.add_DownloadFileCompleted({param($sender,$e)if($e.Cancelled){$state.cancelled=$true}elseif($e.Error){$state.error=$e.Error.Message}else{$state.phase='ready'}})
$timer=New-Object Windows.Forms.Timer;$timer.Interval=1000
$timer.Add_Tick({
 if($state.restart){$state.count=0}else{$state.count--}
 if($state.count -le 0){$timer.Stop();$state.phase='install'}
 else{$label.Text="RMP $best is ready. Restarting in "+$state.count+" seconds to finish the update."}
})
try {
 $form.Show()
 $client.DownloadFileAsync($asset.browser_download_url,$setup)
 while($state.phase -eq 'download' -and -not $state.cancelled -and -not $state.error){[Windows.Forms.Application]::DoEvents();Start-Sleep -Milliseconds 30}
 if($state.error){throw $state.error}
 if($state.cancelled){return}
 $sha=[Security.Cryptography.SHA256]::Create();$stream=[IO.File]::OpenRead($setup)
 try {$digest='sha256:'+([BitConverter]::ToString($sha.ComputeHash($stream))).Replace('-','').ToLowerInvariant()}finally{$stream.Dispose();$sha.Dispose()}
 if($digest -cne $asset.digest){throw 'The download did not match the release checksum.'}
 $bar.Value=100;$restart.Visible=$true;$label.Text="RMP $best is ready. Restarting in 5 seconds to finish the update."
 $timer.Start()
 while($state.phase -eq 'ready' -and -not $state.cancelled){[Windows.Forms.Application]::DoEvents();Start-Sleep -Milliseconds 30}
 if($state.cancelled -or $state.phase -ne 'install'){return}
 $dir=[IO.Path]::GetDirectoryName($PlayerPath)
 if(!$player.HasExited){
  if(!$player.CloseMainWindow() -or !$player.WaitForExit(15000)){throw 'Close RMP and try again.'}
 }
 $form.Close()
 $p=Start-Process $setup -ArgumentList ("/S /D="+$dir) -PassThru -Wait
 if($p.ExitCode -ne 0){throw 'The installer could not update this folder.'}
 Start-Process $PlayerPath
} catch {if($form.Visible){$form.Close()};Notice $_.Exception.Message} finally {$timer.Dispose();$form.Dispose();if(Test-Path $setup){[IO.File]::Delete($setup)};[IO.Directory]::Delete($temp,$true)}
