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
try {
 $setup=Join-Path $temp 'RMP-Setup-x64.exe'
 $client.DownloadFile($asset.browser_download_url,$setup)
 $sha=[Security.Cryptography.SHA256]::Create();$stream=[IO.File]::OpenRead($setup)
 try {$digest='sha256:'+([BitConverter]::ToString($sha.ComputeHash($stream))).Replace('-','').ToLowerInvariant()}finally{$stream.Dispose();$sha.Dispose()}
 if($digest -cne $asset.digest){throw 'The download did not match the release checksum.'}
 $dir=[IO.Path]::GetDirectoryName($PlayerPath)
 if(!$player.HasExited){
  if(!$player.CloseMainWindow() -or !$player.WaitForExit(15000)){throw 'Close RMP and try again.'}
 }
 $p=Start-Process $setup -ArgumentList ("/S /D="+$dir) -PassThru -Wait
 if($p.ExitCode -ne 0){throw 'The installer could not update this folder.'}
 Start-Process $PlayerPath
} catch {Notice $_.Exception.Message} finally {if(Test-Path $setup){[IO.File]::Delete($setup)};[IO.Directory]::Delete($temp)}
