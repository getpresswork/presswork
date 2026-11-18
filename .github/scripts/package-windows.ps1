param(
    [Parameter(Mandatory)] [string] $Target,
    [Parameter(Mandatory)] [string] $Label,
    [Parameter(Mandatory)] [string] $Version,
    [Parameter(Mandatory)] [string] $Tools
)

$ErrorActionPreference = 'Stop'

$arch = switch ($Target) {
    'x86_64-pc-windows-msvc' { 'x64' }
    'aarch64-pc-windows-msvc' { 'arm64' }
    default { throw "There is no MSI file for $Target." }
}
$name = "presswork-$($Label -replace '/', '-')-$Target"
$Tools = (Resolve-Path $Tools).Path

$config = Join-Path $Tools 'nuget.config'
Set-Content -Path $config -Encoding ascii -Value @'
<configuration>
  <packageSources>
    <clear />
    <add key="checked" value="." />
  </packageSources>
</configuration>
'@
$wix = Join-Path $Tools 'bin\wix.exe'
if (-not (Test-Path $wix)) {
    dotnet tool install wix --version 6.0.2 --tool-path (Join-Path $Tools 'bin') --configfile $config
    if ($LASTEXITCODE -ne 0) { throw 'dotnet did not install WiX.' }
}

$ui = Join-Path $Tools 'ui'
if (-not (Test-Path $ui)) {
    $archive = Join-Path $Tools 'ui.zip'
    Copy-Item (Join-Path $Tools 'wixtoolset.ui.wixext.6.0.2.nupkg') $archive
    Expand-Archive $archive $ui
}
$extension = Get-ChildItem $ui -Recurse -Filter 'WixToolset.UI.wixext.dll' | Select-Object -First 1
if (-not $extension) { throw 'The WiX UI extension has no DLL file.' }

$text = (Get-Content LICENSE -Raw) -replace '\\', '\\' -replace '\{', '\{' -replace '\}', '\}'
$text = $text -replace "`r?`n", "\par`n"
$rtf = Join-Path $Tools 'License.rtf'
Set-Content -Path $rtf -Encoding ascii -Value "{\rtf1\ansi\deff0{\fonttbl{\f0\fmodern Consolas;}}\f0\fs16`n$text}"

New-Item -ItemType Directory -Force dist | Out-Null
& $wix build packaging\windows\presswork.wxs `
    -arch $arch `
    -ext $extension.FullName `
    -pdbtype none `
    -d "Version=$Version" `
    -d "Binary=target\$Target\release\presswork.exe" `
    -d 'Icon=crates\presswork\icons\presswork.ico' `
    -d 'License=LICENSE' `
    -d 'Notice=NOTICE' `
    -d "LicenseRtf=$rtf" `
    -o "dist\$name.msi"
if ($LASTEXITCODE -ne 0) { throw 'WiX did not make the MSI file.' }
