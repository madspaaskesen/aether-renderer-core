# Signs the .exe in the Windows zip of a GitHub release with the Certum certificate (SimplySign),
# re-zips it with the same layout and name and uploads it again (--clobber).
#
#   .\scripts\sign-windows.ps1 -Tag vX.Y.Z
#
# Before running: SimplySign Desktop connected (token from the mobile app), GitHub CLI logged in
# (gh auth login). Thumbprint: see flixlar-caption-editor/docs/codesigning (it changes on renewal).
param(
  [Parameter(Mandatory = $true)][string]$Tag,
  [string]$Thumbprint = $env:CERTUM_THUMBPRINT,
  [string]$Crate = "aether-renderer-core",
  [string]$Repo = "madspaaskesen/aether-renderer-core",
  [string]$SignTool = "C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64\signtool.exe"
)
$ErrorActionPreference = "Stop"
if (-not $Thumbprint) { throw "Pass -Thumbprint or set CERTUM_THUMBPRINT" }

# CI uploads files with the same names: wait until the release build is done.
$ciStatus = gh run list --repo $Repo --workflow release.yml --limit 1 --json status -q ".[0].status"
if ($ciStatus -ne "completed") { throw "The release workflow is '$ciStatus', wait until it is done" }

$dir = Join-Path $env:TEMP "$Crate-sign-$Tag"
Remove-Item -Recurse -Force $dir -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $dir | Out-Null
$zipName = "$Crate-$Tag-windows-x86_64.zip"
gh release download $Tag --repo $Repo --pattern $zipName --dir $dir --clobber
if ($LASTEXITCODE) { throw "gh release download failed" }

$zip = Join-Path $dir $zipName
$unpacked = Join-Path $dir "unpacked"
Expand-Archive -Path $zip -DestinationPath $unpacked
$exe = Get-ChildItem $unpacked -Recurse -Filter "$Crate.exe" | Select-Object -First 1
if (-not $exe) { throw "$Crate.exe not found in $zipName" }

Write-Host "🔏 Signing $($exe.FullName)"
& $SignTool sign /sha1 $Thumbprint /tr http://time.certum.pl /td sha256 /fd sha256 /v $exe.FullName
if ($LASTEXITCODE) { throw "signtool sign failed" }
& $SignTool verify /pa /all $exe.FullName
if ($LASTEXITCODE) { throw "signtool verify failed" }

# Same layout as CI (files at the top level of the zip).
Remove-Item $zip
Compress-Archive -Path (Join-Path $unpacked "*") -DestinationPath $zip
gh release upload $Tag $zip --repo $Repo --clobber
if ($LASTEXITCODE) { throw "gh release upload failed" }
Write-Host "✅ Signed and uploaded $zipName to $Tag"
